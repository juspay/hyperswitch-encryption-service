use axum::{routing::get, Router};

use crate::{app::AppState, core};
pub struct Health;

impl Health {
    pub fn server(state: AppState) -> Router<AppState> {
        Router::new()
            .route("/", get(core::heath_check))
            .with_state(state)
    }
}
