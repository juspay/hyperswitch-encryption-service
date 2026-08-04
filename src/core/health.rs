use crate::env::observability as logger;

#[tracing::instrument(skip_all)]
pub(crate) async fn health_check() -> (hyper::StatusCode, &'static str) {
    logger::info!("Health was called");
    (hyper::StatusCode::OK, "Health is good")
}
