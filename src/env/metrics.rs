mod middleware;

use std::time::Duration;

use metrics_utils::{
    counter_metric, f64_histogram_buckets, global_meter, histogram_metric_f64,
    up_down_counter_metric,
};

pub use self::middleware::HttpRequestMetricsLayer;
use crate::config::MetricsConfig;

#[derive(Debug)]
pub enum MetricsHandle {
    Disabled,
    Otlp { inner: metrics_utils::MetricsHandle },
    Prometheus { inner: metrics_utils::MetricsHandle },
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

        MetricsConfig::Prometheus => {
            let metrics_config = metrics_utils::MetricsConfig {
                service_name: String::from(service_name),
                resource_attributes: Vec::new(),
                otlp_config: None,
                enable_prometheus: true,
            };

            match metrics_utils::init_metrics(&metrics_config) {
                Ok(inner) => {
                    inner.register_as_global();
                    MetricsHandle::Prometheus { inner }
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
