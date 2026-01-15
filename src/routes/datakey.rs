use std::sync::Arc;

use axum::{Router, routing::post};

use crate::{app::AppState, core};
pub struct DataKey;

impl DataKey {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/create", post(core::create_data_key))
            .route("/rotate", post(core::rotate_data_key))
            .route("/transfer", post(core::transfer_data_key))
            .route("/reencrypt", post(core::reencrypt_data_keys_handler))
            .with_state(state)
    }
}
