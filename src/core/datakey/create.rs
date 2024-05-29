use crate::{
    app::AppState,
    core::crypto::KeyEncrypt,
    errors::{self, SwitchError},
    storage::dek::DataKeyStorageInterface,
    types::{key::Version, requests::CreateDataKeyRequest, response::DataKeyCreateResponse, Key},
};

#[cfg(not(feature = "aws"))]
use crate::crypto::aes256::GcmAes256;

pub async fn generate_and_create_data_key(
    state: AppState,
    req: CreateDataKeyRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::ApplicationErrorResponse> {
    let db = &state.db_pool;
    let version = Version::get_latest(&req.identifier, &state).await;

    #[cfg(not(feature = "aws"))]
    let aes_key = GcmAes256::generate_key().switch()?;

    #[cfg(feature = "aws")]
    let aes_key = state.aws_client.generate_key().await.switch()?;

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
