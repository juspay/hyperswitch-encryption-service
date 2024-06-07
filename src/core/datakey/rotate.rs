use crate::{
    app::AppState,
    core::crypto::KeyEncrypt,
    errors::{self, SwitchError},
    storage::dek::DataKeyStorageInterface,
    types::{key::Version, requests::RotateDataKeyRequest, response::DataKeyCreateResponse, Key},
};

pub async fn generate_and_rotate_data_key(
    state: AppState,
    req: RotateDataKeyRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::ApplicationErrorResponse> {
    let db = &state.db_pool;
    let version = Version::from(db.get_latest_version(&req.identifier).await.switch()?)
        .increment()
        .switch()?;

    let aes_key = state.encryption_client.generate_key().await.switch()?;

    let key = Key {
        version,
        identifier: req.identifier.clone(),
        key: aes_key,
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
