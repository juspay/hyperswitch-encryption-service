mod create;
mod rotate;

use std::sync::Arc;

use crate::{
    app::AppState,
    errors,
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
) -> Result<Json<DataKeyCreateResponse>, errors::ApplicationErrorResponse> {
    generate_and_create_data_key(state, req)
        .await
        .map(Json)
        .map_err(|err| errors::ApplicationErrorResponse::Other(err.to_string()))
}

#[axum::debug_handler]
pub async fn rotate_data_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RotateDataKeyRequest>,
) -> Result<Json<DataKeyCreateResponse>, errors::ApplicationErrorResponse> {
    generate_and_rotate_data_key(state, req)
        .await
        .map(Json)
        .map_err(|err| errors::ApplicationErrorResponse::Other(err.to_string()))
}
