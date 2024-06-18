mod create;
mod rotate;

use std::sync::Arc;

use crate::{
    app::AppState,
    errors::{self, ToContainerError},
    types::{
        requests::{CreateDataKeyRequest, RotateDataKeyRequest},
        response::DataKeyCreateResponse,
    },
};
use axum::{extract::State, Json};
use create::*;
use rotate::*;

#[axum::debug_handler]
pub async fn create_data_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDataKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    generate_and_create_data_key(state, req)
        .await
        .map(Json)
        .to_container_error()
}

#[axum::debug_handler]
pub async fn rotate_data_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RotateDataKeyRequest>,
) -> errors::ApiResponseResult<Json<DataKeyCreateResponse>> {
    generate_and_rotate_data_key(state, req)
        .await
        .map(Json)
        .to_container_error()
}
