use axum::{routing::get, Router};
use std::sync::Arc;

use crate::{app::AppState, core};
pub struct Health;

impl Health {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(core::heath_check))
            .with_state(state)
    }
}
