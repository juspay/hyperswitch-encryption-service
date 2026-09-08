use crate::{
    env::observability as logger,
    errors::{self, SwitchError},
    multitenancy::TenantState,
    types::{requests::DecryptionRequest, response::DecryptionResponse},
};

pub(super) async fn decryption(
    state: TenantState,
    req: DecryptionRequest,
) -> errors::CustomResult<DecryptionResponse, errors::ApplicationErrorResponse> {
    let decrypted_data = req
        .data
        .decrypt(&state, &req.identifier)
        .await
        .inspect_err(|err| logger::error!(decryption_error = ?err))
        .switch()?;
    Ok(DecryptionResponse {
        data: decrypted_data,
    })
}
