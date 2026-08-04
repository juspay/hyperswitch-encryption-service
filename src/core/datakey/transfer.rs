use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use hyperswitch_masking::PeekInterface;

use crate::{
    consts::base64::BASE64_ENGINE,
    core::crypto::KeyEncrypter,
    crypto::Source,
    env::observability as logger,
    errors::{self, SwitchError},
    multitenancy::TenantState,
    storage::dek::DataKeyStorageInterface,
    types::{Key, key::Version, requests::TransferKeyRequest, response::DataKeyCreateResponse},
};

pub async fn transfer_data_key(
    state: TenantState,
    req: TransferKeyRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::ApplicationErrorResponse> {
    let db = state.get_db_pool();
    let key = BASE64_ENGINE.decode(req.key.peek()).change_context(
        errors::ApplicationErrorResponse::InternalServerError("Failed to decode the base64 key"),
    )?;
    let key = <[u8; 32]>::try_from(key).map_err(|_| {
        errors::ApplicationErrorResponse::InternalServerError("Invalid key found").into_report()
    })?;
    let key = Key {
        version: Version::default(),
        identifier: req.identifier.clone(),
        key: key.into(),
        source: Source::KMS,
    }
    .encrypt(&state)
    .await
    .switch()
    .map_err(|err| {
        logger::error!(?err);
        err
    })?;

    let data_key = db.get_or_insert_data_key(key).await.switch()?;

    Ok(DataKeyCreateResponse {
        identifier: req.identifier,
        key_version: data_key.version,
    })
}
