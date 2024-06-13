use std::sync::Arc;

use crate::{
    app::AppState,
    core::crypto::KeyEncrypt,
    errors::{self, SwitchError},
    storage::dek::DataKeyStorageInterface,
    types::{key::Version, requests::CreateDataKeyRequest, response::DataKeyCreateResponse, Key},
};

pub async fn generate_and_create_data_key(
    state: Arc<AppState>,
    req: CreateDataKeyRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::ApplicationErrorResponse> {
    let db = &state.db_pool;
    let version = Version::get_latest(&req.identifier, &state).await;

    let (source, aes_key) = state.encryption_client.generate_key().await.switch()?;

    let key = Key {
        version,
        identifier: req.identifier.clone(),
        key: aes_key,
        source,
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
        key_version: data_key.version,
        identifier: req.identifier,
    })
}
