mod create;
mod rotate;
mod transfer;

use std::sync::Arc;

use crate::{
    app::AppState,
    core::custodian::Custodian,
    env::observability as logger,
    errors::{self, ToContainerError},
    metrics,
    types::{
        requests::{CreateDataKeyRequest, RotateDataKeyRequest, TransferKeyRequest},
        response::DataKeyCreateResponse,
    },
};
use axum::{extract::State, Json};
use create::*;
use opentelemetry::KeyValue;
use rotate::*;

#[axum::debug_handler]
pub async fn create_data_key(
    State(state): State<Arc<AppState>>,
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

#[axum::debug_handler]
pub async fn rotate_data_key(
    State(state): State<Arc<AppState>>,
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

#[axum::debug_handler]
pub async fn transfer_data_key(
    State(state): State<Arc<AppState>>,
    custodian: Custodian,
    Json(req): Json<TransferKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    transfer::transfer_data_key(state, custodian, req)
        .await
        .map(Json)
        .to_container_error()
}
