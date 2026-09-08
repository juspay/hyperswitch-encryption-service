pub use tracing::{debug, error, info, trace, warn};

use super::{
    logger::{self, LogGuard},
    metrics,
};
pub use super::{
    logger::{LogConfig, LogLevel},
    metrics::{HttpRequestMetricsLayer, MetricsHandle},
};
use crate::config::Config;

pub struct Guards {
    _log_guard: LogGuard,
    metrics_handle: MetricsHandle,
}

impl Guards {
    pub fn metrics_handle(&self) -> &MetricsHandle {
        &self.metrics_handle
    }
}

pub fn setup(
    config: &Config,
    crates_to_filter: impl AsRef<[&'static str]>,
    service_name: &'static str,
) -> Result<Guards, log_utils::LoggerError> {
    let log_guard = logger::setup(&config.log, service_name, crates_to_filter)?;
    let metrics_handle = metrics::init_metrics(&config.metrics, service_name);

    Ok(Guards {
        _log_guard: log_guard,
        metrics_handle,
    })
}
