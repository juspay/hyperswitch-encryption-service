use crate::env::{metrics, observability as logger};

pub(crate) async fn heath_check() -> (hyper::StatusCode, &'static str) {
    logger::info!("Health was called");
    metrics::HEALTH_METRIC.add(1, &[]);
    (hyper::StatusCode::OK, "Health is good")
}
