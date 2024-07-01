use crate::{app::AppState, env::observability as logger, metrics};
use axum::extract::State;
use std::sync::Arc;

pub(crate) async fn heath_check(
    State(_): State<Arc<AppState>>,
) -> (hyper::StatusCode, &'static str) {
    logger::info!("Health was called");
    metrics::HEALTH_METRIC.add(1, &[]);
    (hyper::StatusCode::OK, "Health is good")
}
