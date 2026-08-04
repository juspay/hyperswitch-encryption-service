use crate::{
    env::{metrics, observability as logger},
    errors::{self, SwitchError},
    multitenancy::TenantState,
    types::{requests::EncryptDataRequest, response::EncryptionResponse},
};

pub(super) async fn encryption(
    state: TenantState,
    req: EncryptDataRequest,
) -> errors::CustomResult<EncryptionResponse, errors::ApplicationErrorResponse> {
    let identifier = req.identifier.clone();
    let encrypted_data = req
        .data
        .encrypt(&state, &identifier)
        .await
        .map_err(|err| {
            logger::error!(encryption_error=?err);

            let (data_identifier, key_identifier) = identifier.get_identifier();
            metrics::ENCRYPTION_FAILURE.add(
                1,
                metrics_utils::metric_attributes!(
                    ("key_identifier", key_identifier),
                    ("data_identifier", data_identifier)
                ),
            );
            err
        })
        .switch()?;
    Ok(EncryptionResponse {
        data: encrypted_data,
    })
}
