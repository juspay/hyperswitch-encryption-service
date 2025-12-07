use std::sync::Arc;

use axum::{Router, routing::get};

use crate::{app::AppState, core::gather};

pub struct Metrics;

impl Metrics {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new().route("/", get(gather)).with_state(state)
    }
}
