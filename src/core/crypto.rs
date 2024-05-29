mod crux;
mod decryption;
mod encryption;

pub use crux::*;

use crate::{
    app::AppState,
    errors,
    types::{
        requests::{DecryptionRequest, EncryptDataRequest},
        response::{DecryptionResponse, EncryptionResponse},
    },
};
use axum::extract::{Json, State};

pub async fn encrypt_data(
    State(state): State<AppState>,
    Json(req): Json<EncryptDataRequest>,
) -> Result<Json<EncryptionResponse>, errors::ApplicationErrorResponse> {
    encryption::encryption(state, req)
        .await
        .map(Json)
        .map_err(|err| errors::ApplicationErrorResponse::Other(err.to_string()))
}

pub async fn decrypt_data(
    State(state): State<AppState>,
    Json(req): Json<DecryptionRequest>,
) -> Result<Json<DecryptionResponse>, errors::ApplicationErrorResponse> {
    decryption::decryption(state, req)
        .await
        .map(Json)
        .map_err(|err| errors::ApplicationErrorResponse::Other(err.to_string()))
}
