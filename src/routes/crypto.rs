use std::sync::Arc;

use axum::{Router, routing::post};

use crate::{app::AppState, core};

pub struct Crypto;

impl Crypto {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/encrypt", post(core::encrypt_data))
            .route("/decrypt", post(core::decrypt_data))
            .with_state(state)
    }
}
