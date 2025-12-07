mod crux;
pub mod custodian;
mod decryption;
mod encryption;

use axum::extract::Json;
pub use crux::*;
use opentelemetry::KeyValue;

use self::custodian::Custodian;
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
    custodian: Custodian,
    Json(req): Json<EncryptDataRequest>,
) -> errors::ApiResponseResult<Json<EncryptionResponse>> {
    let (data_identifier, key_identifier) = req.identifier.get_identifier();

    utils::record_api_operation(
        encryption::encryption(state, custodian, req),
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
    custodian: Custodian,
    Json(req): Json<DecryptionRequest>,
) -> errors::ApiResponseResult<Json<DecryptionResponse>> {
    let (data_identifier, key_identifier) = req.identifier.get_identifier();

    utils::record_api_operation(
        decryption::decryption(state, custodian, req),
        &metrics::DECRYPTION_API_LATENCY,
        &[
            KeyValue::new("data_identifier", data_identifier),
            KeyValue::new("key_identifier", key_identifier),
        ],
    )
    .await
}
