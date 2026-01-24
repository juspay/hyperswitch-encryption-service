pub mod create;
mod list;
#[cfg(feature = "aws")]
mod reencrypt;
mod rotate;
mod transfer;

use axum::Json;
use opentelemetry::KeyValue;

#[cfg(feature = "aws")]
use self::reencrypt::*;
use self::{create::*, list::*, rotate::*};
use crate::{
    core::custodian::Custodian,
    env::observability as logger,
    errors::{self, ToContainerError},
    metrics,
    multitenancy::TenantState,
    types::{
        requests::{
            CreateDataKeyRequest, ListKeysRequest, ReEncryptDataKeysRequest, RotateDataKeyRequest,
            TransferKeyRequest,
        },
        response::{DataKeyCreateResponse, ListKeysResponse, ReEncryptDataKeysResponse},
    },
};

pub async fn create_data_key(
    state: TenantState,
    custodian: Custodian,
    Json(req): Json<CreateDataKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    let identifier = req.identifier.clone();

    generate_and_create_data_key(state, custodian, req)
        .await
        .map(Json)
        .map_err(|err| {
            logger::error!(key_create_failure=?err);

            let (data_identifier, key_identifier) = identifier.get_identifier();
            metrics::KEY_CREATE_FAILURE.add(
                1,
                &[
                    KeyValue::new("key_identifier", key_identifier),
                    KeyValue::new("data_identifier", data_identifier),
                ],
            );
            err
        })
        .to_container_error()
}

pub async fn rotate_data_key(
    state: TenantState,
    custodian: Custodian,
    Json(req): Json<RotateDataKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    let identifier = req.identifier.clone();

    generate_and_rotate_data_key(state, custodian, req)
        .await
        .map(Json)
        .map_err(|err| {
            logger::error!(key_create_failure=?err);

            let (data_identifier, key_identifier) = identifier.get_identifier();
            metrics::KEY_ROTATE_FAILURE.add(
                1,
                &[
                    KeyValue::new("key_identifier", key_identifier),
                    KeyValue::new("data_identifier", data_identifier),
                ],
            );
            err
        })
        .to_container_error()
}

pub async fn transfer_data_key(
    state: TenantState,
    custodian: Custodian,
    Json(req): Json<TransferKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    transfer::transfer_data_key(state, custodian, req)
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
