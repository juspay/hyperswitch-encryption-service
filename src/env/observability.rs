pub use tracing::{debug, error, info, trace, warn};

pub use super::logger::{LogConfig, LogLevel, OnRequest, OnResponse};
use super::{
    logger::{self, LogGuard},
    metrics::{self, MetricsGuard},
};

pub struct Guards {
    _log_guard: LogGuard,
    _metrics_guard: MetricsGuard,
}

pub fn setup(
    log_config: &LogConfig,
    crates_to_filter: impl AsRef<[&'static str]>,
    service_name: &'static str,
) -> Guards {
    let log_guard = logger::setup_logging_pipeline(log_config, crates_to_filter);
    let metrics_guard = metrics::setup_metrics_pipeline(service_name);

    Guards {
        _log_guard: log_guard,
        _metrics_guard: metrics_guard,
    }
}
