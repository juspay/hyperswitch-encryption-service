pub mod create;
mod list;
#[cfg(feature = "aws")]
mod reencrypt;
mod rotate;
mod transfer;

use axum::Json;

#[cfg(feature = "aws")]
use self::reencrypt::*;
use self::{create::*, list::*, rotate::*};
use crate::{
    env::{metrics, observability as logger},
    errors::{self, ToContainerError},
    multitenancy::TenantState,
    types::{
        requests::{
            CreateDataKeyRequest, ListKeysRequest, ReEncryptDataKeysRequest, RotateDataKeyRequest,
            TransferKeyRequest,
        },
        response::{DataKeyCreateResponse, ListKeysResponse, ReEncryptDataKeysResponse},
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

pub async fn list_data_keys_handler(
    state: TenantState,
    Json(req): Json<ListKeysRequest>,
) -> errors::ApiResponseResult<Json<ListKeysResponse>> {
    list_data_keys(state, req)
        .await
        .map(Json)
        .map_err(|err| {
            logger::error!(key_list_failure=?err);
            err
        })
        .to_container_error()
}

// TODO: refactor
pub async fn reencrypt_data_keys_handler(
    state: TenantState,
    Json(req): Json<ReEncryptDataKeysRequest>,
) -> errors::ApiResponseResult<Json<ReEncryptDataKeysResponse>> {
    #[cfg(feature = "aws")]
    {
        reencrypt_data_keys(state, req)
            .await
            .map(Json)
            .map_err(|err| {
                logger::error!(reencrypt_failure=?err);
                err
            })
            .to_container_error()
    }

    #[cfg(not(feature = "aws"))]
    {
        let _ = (state, req);
        Err(error_stack::report!(
            errors::ApplicationErrorResponse::InternalServerError(
                "Re-encryption is only available when using AWS KMS"
            )
        ))
        .to_container_error()
    }
}
