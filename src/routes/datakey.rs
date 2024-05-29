use axum::{routing::post, Router};

use crate::{app::AppState, core};
pub struct DataKey;

impl DataKey {
    pub fn server(state: AppState) -> Router<AppState> {
        Router::new()
            .route("/create", post(core::create_data_key))
            .route("/rotate", post(core::rotate_data_key))
            .with_state(state)
    }
}
