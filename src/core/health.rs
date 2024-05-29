use crate::app::AppState;
use axum::extract::State;

pub(crate) async fn heath_check(State(_): State<AppState>) -> (hyper::StatusCode, &'static str) {
    (hyper::StatusCode::OK, "Health is good")
}
