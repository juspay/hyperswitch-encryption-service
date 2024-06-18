use crate::{
    app::AppState,
    core::crypto::DataEncrypt,
    errors::{self, SwitchError},
    types::{requests::EncryptDataRequest, response::EncryptionResponse},
};
use std::sync::Arc;

pub(super) async fn encryption(
    state: Arc<AppState>,
    req: EncryptDataRequest,
) -> errors::CustomResult<EncryptionResponse, errors::ApplicationErrorResponse> {
    let identifier = req.identifier.clone();
    let encrypted_data = req.data.encrypt(&state, &identifier).await.switch()?;
    Ok(EncryptionResponse {
        data: encrypted_data,
    })
}
