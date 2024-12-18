use std::sync::Arc;

use crate::{
    app::AppState,
    core::{crypto::KeyEncrypter, custodian::Custodian},
    env::observability as logger,
    errors::{self, SwitchError},
    storage::dek::DataKeyStorageInterface,
    types::{key::Version, requests::CreateDataKeyRequest, response::DataKeyCreateResponse, Key},
};

pub async fn generate_and_create_data_key(
    state: Arc<AppState>,
    custodian: Custodian,
    req: CreateDataKeyRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::ApplicationErrorResponse> {
    let db = &state.db_pool;
    let version = Version::get_latest(&req.identifier, &state).await;

    let (source, aes_key) = state.keymanager_client.generate_key().await.switch()?;

    let key = Key {
        version,
        identifier: req.identifier.clone(),
        key: aes_key,
        source,
        token: custodian.into_access_token(state.as_ref()),
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
        key_version: data_key.version,
        identifier: req.identifier,
    })
}
