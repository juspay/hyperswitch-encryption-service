mod middleware;

use std::{sync::Arc, time::Duration};

use axum::Router;
use metrics_utils::{
    counter_metric, f64_histogram_buckets, global_meter, histogram_metric_f64,
    up_down_counter_metric,
};

pub use self::middleware::HttpRequestMetricsLayer;
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

        MetricsConfig::Prometheus { host, port, .. } => {
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

global_meter!(pub(crate) CRIPTA_METER, "encryption_service");

// HTTP server
counter_metric!(
    pub(crate) HTTP_SERVER_REQUEST_COUNT, CRIPTA_METER,
    name: "http.server.request.count",
    description: "Number of HTTP server requests received",
    unit: "{request}",
);
histogram_metric_f64!(
    pub(crate) HTTP_SERVER_REQUEST_DURATION, CRIPTA_METER,
    name: "http.server.request.duration",
    description: "Duration of HTTP server requests",
    unit: "s",
    buckets: f64_histogram_buckets().to_vec(),
);
up_down_counter_metric!(
    pub(crate) HTTP_SERVER_ACTIVE_REQUESTS, CRIPTA_METER,
    name: "http.server.active_requests",
    description: "Number of HTTP server requests currently in flight",
    unit: "{request}",
);

// Database
counter_metric!(
    pub(crate) DATABASE_QUERY_COUNT, CRIPTA_METER,
    name: "database.query.count",
    description: "Number of database query attempts",
    unit: "{query}",
);
histogram_metric_f64!(
    pub(crate) DATABASE_QUERY_DURATION, CRIPTA_METER,
    name: "database.query.duration",
    description: "Duration of completed database queries",
    unit: "s",
    buckets: f64_histogram_buckets().to_vec(),
);
histogram_metric_f64!(
    pub(crate) DATABASE_CONNECTION_ACQUIRE_DURATION, CRIPTA_METER,
    name: "database.connection.acquire.duration",
    description: "Duration of database connection acquisition attempts",
    unit: "s",
    buckets: f64_histogram_buckets().to_vec(),
);

// Cache
counter_metric!(
    pub(crate) CACHE_LOOKUP_COUNT, CRIPTA_METER,
    name: "cache.lookup.count",
    description: "Number of cache lookup attempts",
    unit: "{lookup}",
);
counter_metric!(
    pub(crate) CACHE_INSERT_COUNT, CRIPTA_METER,
    name: "cache.insert.count",
    description: "Number of cache insert attempts",
    unit: "{insert}",
);
counter_metric!(
    pub(crate) CACHE_REMOVAL_COUNT, CRIPTA_METER,
    name: "cache.removal.count",
    description: "Number of cache removal events",
    unit: "{event}",
);

// Key manager
histogram_metric_f64!(
    pub(crate) KEY_MANAGER_CALL_DURATION, CRIPTA_METER,
    name: "key_manager.call.duration",
    description: "Duration of completed key-manager call attempts",
    unit: "s",
    buckets: f64_histogram_buckets().to_vec(),
);

// Data key storage
counter_metric!(
    pub(crate) DATA_KEY_OPERATION_COUNT, CRIPTA_METER,
    name: "data_key.operation.count",
    description: "Number of data key storage operation attempts",
    unit: "{operation}",
);

// Domain operations
counter_metric!(
    pub(crate) DOMAIN_OPERATION_COUNT, CRIPTA_METER,
    name: "domain.operation.count",
    description: "Number of domain operation attempts",
    unit: "{operation}",
);
histogram_metric_f64!(
    pub(crate) DOMAIN_OPERATION_DURATION, CRIPTA_METER,
    name: "domain.operation.duration",
    description: "Duration of completed domain operations",
    unit: "s",
    buckets: f64_histogram_buckets().to_vec(),
);

// Data encryption/decryption
histogram_metric_f64!(
    pub(crate) CRYPTO_OPERATION_DURATION, CRIPTA_METER,
    name: "crypto.operation.duration",
    description: "Duration of completed data encryption/decryption operations",
    unit: "s",
    buckets: f64_histogram_buckets().to_vec(),
);

#[macro_export]
macro_rules! impl_metric_value_from {
        ($($ty:ty),+ $(,)?) => {
            $(
                impl From<$ty> for metrics_utils::opentelemetry::Value {
                    fn from(v: $ty) -> Self {
                        Self::from(<&'static str>::from(v))
                    }
                }
            )+
        };
    }

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum KeyManagerBackend {
    AwsKms,
    Vault,
    Aes256,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum KeyManagerOperation {
    GenerateKey,
    Encrypt,
    Decrypt,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum DomainOperation {
    Encrypt,
    Decrypt,
    KeyCreate,
    KeyRotate,
    KeyTransfer,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum DataCryptoOperation {
    Encrypt,
    Decrypt,
}

impl_metric_value_from!(
    KeyManagerBackend,
    KeyManagerOperation,
    DomainOperation,
    DataCryptoOperation,
);
