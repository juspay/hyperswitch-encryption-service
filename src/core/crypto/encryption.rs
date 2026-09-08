use crate::{
    env::observability as logger,
    errors::{self, SwitchError},
    multitenancy::TenantState,
    types::{requests::EncryptDataRequest, response::EncryptionResponse},
};

pub(super) async fn encryption(
    state: TenantState,
    req: EncryptDataRequest,
) -> errors::CustomResult<EncryptionResponse, errors::ApplicationErrorResponse> {
    let encrypted_data = req
        .data
        .encrypt(&state, &req.identifier)
        .await
        .inspect_err(|err| logger::error!(encryption_error = ?err))
        .switch()?;
    Ok(EncryptionResponse {
        data: encrypted_data,
    })
}
