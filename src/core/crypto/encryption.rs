use super::custodian::Custodian;
use crate::{
    env::observability as logger,
    errors::{self, SwitchError},
    metrics,
    multitenancy::TenantState,
    types::{requests::EncryptDataRequest, response::EncryptionResponse},
};
use opentelemetry::KeyValue;

pub(super) async fn encryption(
    state: TenantState,
    custodian: Custodian,
    req: EncryptDataRequest,
) -> errors::CustomResult<EncryptionResponse, errors::ApplicationErrorResponse> {
    let identifier = req.identifier.clone();
    let encrypted_data = req
        .data
        .encrypt(&state, &identifier, custodian)
        .await
        .map_err(|err| {
            logger::error!(encryption_error=?err);

            let (data_identifier, key_identifier) = identifier.get_identifier();
            metrics::ENCRYPTION_FAILURE.add(
                1,
                &[
                    KeyValue::new("key_identifier", key_identifier),
                    KeyValue::new("data_identifier", data_identifier),
                ],
            );
            err
        })
        .switch()?;
    Ok(EncryptionResponse {
        data: encrypted_data,
    })
}
