use crate::{app::AppState, core};
use axum::{routing::post, Router};
use std::sync::Arc;

pub struct Crypto;

impl Crypto {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/encrypt", post(core::encrypt_data))
            .route("/decrypt", post(core::decrypt_data))
            .with_state(state)
    }
}
