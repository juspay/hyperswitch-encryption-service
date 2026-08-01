mod crux;
mod decryption;
mod encryption;

use axum::extract::Json;
pub use crux::*;

use crate::{
    env::metrics,
    errors::{self, ToContainerError},
    multitenancy::TenantState,
    types::{
        requests::{DecryptionRequest, EncryptDataRequest},
        response::{DecryptionResponse, EncryptionResponse},
    },
};

#[tracing::instrument(skip_all)]
pub async fn encrypt_data(
    state: TenantState,
    Json(req): Json<EncryptDataRequest>,
) -> errors::ApiResponseResult<Json<EncryptionResponse>> {
    let (data_identifier, _key_identifier) = req.identifier.get_identifier();

    super::record_domain_operation(
        encryption::encryption(state, req),
        metrics::DomainOperation::Encrypt,
        data_identifier,
    )
    .await
    .map(Json)
    .to_container_error()
}

#[tracing::instrument(skip_all)]
pub async fn decrypt_data(
    state: TenantState,
    Json(req): Json<DecryptionRequest>,
) -> errors::ApiResponseResult<Json<DecryptionResponse>> {
    let (data_identifier, _key_identifier) = req.identifier.get_identifier();

    super::record_domain_operation(
        decryption::decryption(state, req),
        metrics::DomainOperation::Decrypt,
        data_identifier,
    )
    .await
    .map(Json)
    .to_container_error()
}
