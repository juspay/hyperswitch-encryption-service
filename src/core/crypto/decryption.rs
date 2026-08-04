use crate::{
    env::{metrics, observability as logger},
    errors::{self, SwitchError},
    multitenancy::TenantState,
    types::{requests::DecryptionRequest, response::DecryptionResponse},
};

pub(super) async fn decryption(
    state: TenantState,
    req: DecryptionRequest,
) -> errors::CustomResult<DecryptionResponse, errors::ApplicationErrorResponse> {
    let identifier = req.identifier.clone();
    let decrypted_data = req
        .data
        .decrypt(&state, &identifier)
        .await
        .map_err(|err| {
            logger::error!(encryption_error=?err);

            let (data_identifier, key_identifier) = identifier.get_identifier();
            metrics::DECRYPTION_FAILURE.add(
                1,
                metrics_utils::metric_attributes!(
                    ("key_identifier", key_identifier),
                    ("data_identifier", data_identifier)
                ),
            );
            err
        })
        .switch()?;

    Ok(DecryptionResponse {
        data: decrypted_data,
    })
}
