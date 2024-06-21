use axum::{routing::post, Router};
use std::sync::Arc;

use crate::{app::AppState, core};
pub struct DataKey;

impl DataKey {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/create", post(core::create_data_key))
            .route("/rotate", post(core::rotate_data_key))
            .route("/transfer", post(core::transfer_data_key))
            .with_state(state)
    }
}
