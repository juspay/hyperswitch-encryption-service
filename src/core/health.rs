use crate::env::observability as logger;

pub(crate) async fn heath_check() -> (hyper::StatusCode, &'static str) {
    logger::info!("Health was called");
    (hyper::StatusCode::OK, "Health is good")
}
