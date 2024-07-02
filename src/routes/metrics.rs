use crate::{app::AppState, core::gather};
use axum::{routing::get, Router};
use std::sync::Arc;

pub struct Metrics;

impl Metrics {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new().route("/", get(gather)).with_state(state)
    }
}
