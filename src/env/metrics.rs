use std::{sync::Arc, time::Duration};

use axum::Router;
use metrics_utils::{counter_metric, f64_histogram_buckets, global_meter, histogram_metric_f64};

use crate::{app::AppState, config::MetricsConfig, errors, routes::Health};

#[derive(Debug)]
pub enum MetricsHandle {
    Disabled,
    Otlp {
        inner: metrics_utils::MetricsHandle,
    },
    Prometheus {
        inner: metrics_utils::MetricsHandle,
        host: String,
        port: u16,
    },
}

impl MetricsHandle {
    pub const fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

pub fn init_metrics(config: &MetricsConfig, service_name: &'static str) -> MetricsHandle {
    match config {
        MetricsConfig::Disabled => MetricsHandle::Disabled,

        MetricsConfig::Otlp {
            endpoint,
            endpoint_timeout_secs,
            metrics_export_interval_secs,
            ..
        } => {
            let metrics_config = metrics_utils::MetricsConfig {
                service_name: String::from(service_name),
                resource_attributes: Vec::new(),
                otlp_config: Some(metrics_utils::OtlpConfig {
                    endpoint: endpoint.clone(),
                    endpoint_timeout: Some(Duration::from_secs(*endpoint_timeout_secs)),
                    metrics_export_interval: Some(Duration::from_secs(
                        *metrics_export_interval_secs,
                    )),
                    compression: Some(metrics_utils::OtlpCompression::Zstd),
                    temporality: Some(metrics_utils::Temporality::Cumulative),
                }),
                enable_prometheus: false,
            };

            match metrics_utils::init_metrics(&metrics_config) {
                Ok(inner) => {
                    inner.register_as_global();
                    MetricsHandle::Otlp { inner }
                }
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        "Failed to initialize metrics pipeline; metrics disabled"
                    );
                    MetricsHandle::Disabled
                }
            }
        }

        MetricsConfig::Prometheus { host, port } => {
            let metrics_config = metrics_utils::MetricsConfig {
                service_name: String::from(service_name),
                resource_attributes: Vec::new(),
                otlp_config: None,
                enable_prometheus: true,
            };

            match metrics_utils::init_metrics(&metrics_config) {
                Ok(inner) => {
                    inner.register_as_global();
                    MetricsHandle::Prometheus {
                        inner,
                        host: host.clone(),
                        port: *port,
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        "Failed to initialize metrics pipeline; metrics disabled"
                    );
                    MetricsHandle::Disabled
                }
            }
        }
    }
}

pub fn spawn_prometheus_metrics_server(
    host: &str,
    port: u16,
    registry: metrics_utils::prometheus::Registry,
    state: Arc<AppState>,
) -> errors::CustomResult<(), errors::ParsingError> {
    use metrics_utils::prometheus::Encoder;

    let addr = match host.parse::<std::net::IpAddr>() {
        Ok(ip) => std::net::SocketAddr::new(ip, port),
        Err(_) => {
            return Err(error_stack::Report::new(
                errors::ParsingError::DecodingFailed(format!(
                    r#"metrics.host "{host}" is not a valid IP address"#
                )),
            ));
        }
    };

    let app = Router::new()
        .route(
            "/metrics",
            axum::routing::get(move || {
                let registry = registry.clone();
                async move {
                    let encoder = metrics_utils::prometheus::TextEncoder::new();
                    let mut buffer = Vec::new();

                    if let Err(error) = encoder.encode(&registry.gather(), &mut buffer) {
                        tracing::warn!(?error, "Failed to encode prometheus metrics");
                    }

                    (
                        axum::http::StatusCode::OK,
                        String::from_utf8(buffer).unwrap_or_default(),
                    )
                }
            }),
        )
        .nest("/health", Health::server(state.clone()))
        .with_state(state);

    tokio::spawn(async move {
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                tracing::info!("Starting Prometheus metrics server at `{addr}`");

                if let Err(error) = axum::serve(listener, app).await {
                    tracing::warn!(?error, "Prometheus metrics server failed");
                }
            }
            Err(error) => {
                tracing::error!(?error, "Failed to bind prometheus metrics server");
            }
        }
    });

    Ok(())
}

global_meter!(pub(crate) CRIPTA_METER, "cripta");

counter_metric!(
    pub(crate) HEALTH_METRIC, CRIPTA_METER,
    description: "Counts the number of times the health endpoint is called",
);

counter_metric!(
    pub(crate) ENCRYPTION_FAILURE, CRIPTA_METER,
    description: "Counts encryption failures",
);

counter_metric!(
    pub(crate) DECRYPTION_FAILURE, CRIPTA_METER,
    description: "Counts decryption failures",
);

counter_metric!(
    pub(crate) KEY_CREATE_FAILURE, CRIPTA_METER,
    description: "Counts data key creation failures",
);

counter_metric!(
    pub(crate) KEY_ROTATE_FAILURE, CRIPTA_METER,
    description: "Counts data key rotation failures",
);

histogram_metric_f64!(
    pub(crate) ENCRYPTION_API_LATENCY, CRIPTA_METER,
    description: "Encryption API latency in seconds",
    buckets: f64_histogram_buckets().to_vec(),
);

histogram_metric_f64!(
    pub(crate) DECRYPTION_API_LATENCY, CRIPTA_METER,
    description: "Decryption API latency in seconds",
    buckets: f64_histogram_buckets().to_vec(),
);
