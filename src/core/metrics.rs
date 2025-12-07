use std::{io::BufWriter, sync::Arc};

use axum::extract::State;
use error_stack::ResultExt;
use prometheus::{Encoder, TextEncoder, default_registry};

use crate::{
    app::AppState,
    errors::{self, ToContainerError},
};

pub async fn gather(State(_): State<Arc<AppState>>) -> errors::ApiResponseResult<Vec<u8>> {
    let registry = default_registry();
    let metrics_families = registry.gather();
    let encoder = TextEncoder::new();
    let mut buffer = BufWriter::new(Vec::new());

    encoder
        .encode(&metrics_families, &mut buffer)
        .change_context(errors::ApplicationErrorResponse::ParsingFailed(
            "Failed to flush the metrics buffer".to_string(),
        ))
        .to_container_error()?;

    buffer
        .into_inner()
        .change_context(errors::ApplicationErrorResponse::ParsingFailed(
            "Failed to flush the metrics buffer".to_string(),
        ))
        .to_container_error()
}
