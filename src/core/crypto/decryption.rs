use crate::{
    env::observability as logger,
    errors::{self, SwitchError},
    metrics,
    multitenancy::TenantState,
    types::{requests::DecryptionRequest, response::DecryptionResponse},
};
use opentelemetry::KeyValue;

use super::custodian::Custodian;

pub(super) async fn decryption(
    state: TenantState,
    custodian: Custodian,
    req: DecryptionRequest,
) -> errors::CustomResult<DecryptionResponse, errors::ApplicationErrorResponse> {
    let identifier = req.identifier.clone();
    let decrypted_data = req
        .data
        .decrypt(&state, &identifier, custodian)
        .await
        .map_err(|err| {
            logger::error!(encryption_error=?err);

            let (data_identifier, key_identifier) = identifier.get_identifier();
            metrics::DECRYPTION_FAILURE.add(
                1,
                &[
                    KeyValue::new("key_identifier", key_identifier),
                    KeyValue::new("data_identifier", data_identifier),
                ],
            );
            err
        })
        .switch()?;

    Ok(DecryptionResponse {
        data: decrypted_data,
    })
}
