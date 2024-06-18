use crate::app::AppState;
use axum::extract::State;
use std::sync::Arc;

pub(crate) async fn heath_check(
    State(_): State<Arc<AppState>>,
) -> (hyper::StatusCode, &'static str) {
    (hyper::StatusCode::OK, "Health is good")
}
