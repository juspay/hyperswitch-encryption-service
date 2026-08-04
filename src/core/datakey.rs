pub mod create;
mod rotate;
mod transfer;

use axum::Json;

use self::{create::*, rotate::*};
use crate::{
    env::{metrics, observability as logger},
    errors::{self, ToContainerError},
    multitenancy::TenantState,
    types::{
        requests::{CreateDataKeyRequest, RotateDataKeyRequest, TransferKeyRequest},
        response::DataKeyCreateResponse,
    },
};

#[tracing::instrument(skip_all)]
pub async fn create_data_key(
    state: TenantState,
    Json(req): Json<CreateDataKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    let (data_identifier, _) = req.identifier.get_identifier();

    super::record_domain_operation(
        generate_and_create_data_key(state, req),
        metrics::DomainOperation::KeyCreate,
        data_identifier,
    )
    .await
    .inspect_err(|error| logger::error!(?error, "Failed to create data key"))
    .map(Json)
    .to_container_error()
}

#[tracing::instrument(skip_all)]
pub async fn rotate_data_key(
    state: TenantState,
    Json(req): Json<RotateDataKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    let (data_identifier, _) = req.identifier.get_identifier();

    super::record_domain_operation(
        generate_and_rotate_data_key(state, req),
        metrics::DomainOperation::KeyRotate,
        data_identifier,
    )
    .await
    .inspect_err(|error| logger::error!(?error, "Failed to rotate data key"))
    .map(Json)
    .to_container_error()
}

#[tracing::instrument(skip_all)]
pub async fn transfer_data_key(
    state: TenantState,
    Json(req): Json<TransferKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    let (data_identifier, _) = req.identifier.get_identifier();

    super::record_domain_operation(
        transfer::transfer_data_key(state, req),
        metrics::DomainOperation::KeyTransfer,
        data_identifier,
    )
    .await
    .map(Json)
    .to_container_error()
}
