use std::sync::Arc;

use axum::{Router, routing::get};

use crate::{app::AppState, core};
pub struct Health;

impl Health {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(core::heath_check))
            .with_state(state)
    }
}
