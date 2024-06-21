use std::sync::Arc;

use error_stack::ResultExt;

use crate::{
    app::AppState,
    consts::base64::BASE64_ENGINE,
    core::crypto::KeyEncrypt,
    crypto::Source,
    errors::{self, SwitchError},
    storage::dek::DataKeyStorageInterface,
    types::{key::Version, requests::TransferKeyRequest, response::DataKeyCreateResponse, Key},
};
use base64::Engine;

pub async fn transfer_data_key(
    state: Arc<AppState>,
    req: TransferKeyRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::ApplicationErrorResponse> {
    let db = &state.db_pool;
    let key = BASE64_ENGINE
        .decode(req.key)
        .change_context(errors::ApplicationErrorResponse::InternalServerError)?;
    let key = <[u8; 32]>::try_from(key)
        .map_err(|_| error_stack::report!(errors::ApplicationErrorResponse::InternalServerError))?;
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
        router_env::logger::error!(?err);
        err
    })?;

    let data_key = db.insert_data_key(key).await.switch()?;

    Ok(DataKeyCreateResponse {
        identifier: req.identifier,
        key_version: data_key.version,
    })
}
