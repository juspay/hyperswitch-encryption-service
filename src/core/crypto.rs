mod crux;
mod decryption;
mod encryption;

use axum::extract::Json;
pub use crux::*;
use opentelemetry::KeyValue;

use crate::{
    errors, metrics,
    multitenancy::TenantState,
    types::{
        requests::{DecryptionRequest, EncryptDataRequest},
        response::{DecryptionResponse, EncryptionResponse},
    },
    utils,
};

pub async fn encrypt_data(
    state: TenantState,
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
    state: TenantState,
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
