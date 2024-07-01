use crate::{
    app::AppState,
    core::crypto::DataEncrypt,
    env::observability as logger,
    errors::{self, SwitchError},
    metrics,
    types::{requests::EncryptDataRequest, response::EncryptionResponse},
};
use std::sync::Arc;

pub(super) async fn encryption(
    state: Arc<AppState>,
    req: EncryptDataRequest,
) -> errors::CustomResult<EncryptionResponse, errors::ApplicationErrorResponse> {
    let identifier = req.identifier.clone();
    let encrypted_data = req
        .data
        .encrypt(&state, &identifier)
        .await
        .map_err(|err| {
            logger::error!(encryption_error=?err);
            metrics::ENCRYPTION_FAILURE.add(1, &[]);
            err
        })
        .switch()?;
    Ok(EncryptionResponse {
        data: encrypted_data,
    })
}
