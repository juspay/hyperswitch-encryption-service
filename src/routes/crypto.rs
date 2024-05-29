use crate::{app::AppState, core};
use axum::{routing::post, Router};

pub struct Crypto;

impl Crypto {
    pub fn server(state: AppState) -> Router<AppState> {
        Router::new()
            .route("/encrypt", post(core::encrypt_data))
            .route("/decrypt", post(core::decrypt_data))
            .with_state(state)
    }
}
