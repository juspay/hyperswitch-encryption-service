mod crux;
mod decryption;
mod encryption;

pub use crux::*;

use crate::{
    app::AppState,
    errors::{self, ToContainerError},
    types::{
        requests::{DecryptionRequest, EncryptDataRequest},
        response::{DecryptionResponse, EncryptionResponse},
    },
};
use axum::extract::{Json, State};
use std::sync::Arc;

pub async fn encrypt_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EncryptDataRequest>,
) -> errors::ApiResponseResult<Json<EncryptionResponse>> {
    encryption::encryption(state, req)
        .await
        .map(Json)
        .to_container_error()
}

pub async fn decrypt_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DecryptionRequest>,
) -> errors::ApiResponseResult<Json<DecryptionResponse>> {
    decryption::decryption(state, req)
        .await
        .map(Json)
        .to_container_error()
}
