mod crux;
mod decryption;
mod encryption;

pub use crux::*;

use crate::{
    app::AppState,
    errors, metrics,
    types::{
        requests::{DecryptionRequest, EncryptDataRequest},
        response::{DecryptionResponse, EncryptionResponse},
    },
    utils,
};
use axum::extract::{Json, State};
use opentelemetry::KeyValue;
use std::sync::Arc;

pub async fn encrypt_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EncryptDataRequest>,
) -> errors::ApiResponseResult<Json<EncryptionResponse>> {
    let (data_identifier, key_identifier) = req.identifier.get_identifier();

    utils::record_api_operation(
        encryption::encryption(state, req),
        &metrics::ENCRYPTION_API_LATENCY,
        &[
            KeyValue::new("data_identifier", data_identifier),
            KeyValue::new("key_identifier", key_identifier),
        ],
    )
    .await
}

pub async fn decrypt_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DecryptionRequest>,
) -> errors::ApiResponseResult<Json<DecryptionResponse>> {
    let (data_identifier, key_identifier) = req.identifier.get_identifier();

    utils::record_api_operation(
        decryption::decryption(state, req),
        &metrics::DECRYPTION_API_LATENCY,
        &[
            KeyValue::new("data_identifier", data_identifier),
            KeyValue::new("key_identifier", key_identifier),
        ],
    )
    .await
}
