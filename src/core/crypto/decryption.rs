use std::sync::Arc;

use crate::{
    app::AppState,
    core::crypto::DataDecrypt,
    env::observability as logger,
    errors::{self, SwitchError},
    metrics,
    types::{requests::DecryptionRequest, response::DecryptionResponse},
};

pub(super) async fn decryption(
    state: Arc<AppState>,
    req: DecryptionRequest,
) -> errors::CustomResult<DecryptionResponse, errors::ApplicationErrorResponse> {
    let identifier = req.identifier.clone();
    let decrypted_data = req
        .data
        .decrypt(&state, &identifier)
        .await
        .map_err(|err| {
            logger::error!(encryption_error=?err);
            metrics::DECRYPTION_FAILURE.add(1, &[]);
            err
        })
        .switch()?;

    Ok(DecryptionResponse {
        data: decrypted_data,
    })
}
