use crate::{
    app::AppState,
    core::crypto::DataDecrypt,
    errors::{self, SwitchError},
    types::{requests::DecryptionRequest, response::DecryptionResponse},
};

pub(super) async fn decryption(
    state: AppState,
    req: DecryptionRequest,
) -> errors::CustomResult<DecryptionResponse, errors::ApplicationErrorResponse> {
    let identifier = req.identifier.clone();
    let decrypted_data = req.data.decrypt(&state, &identifier).await.switch()?;

    Ok(DecryptionResponse {
        data: decrypted_data,
    })
}
