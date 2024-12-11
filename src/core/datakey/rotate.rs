use std::sync::Arc;

use crate::{
    app::AppState,
    core::{crypto::KeyEncrypter, custodian::Custodian},
    env::observability as logger,
    errors::{self, SwitchError},
    storage::dek::DataKeyStorageInterface,
    types::{requests::RotateDataKeyRequest, response::DataKeyCreateResponse, Key},
};

pub async fn generate_and_rotate_data_key(
    state: Arc<AppState>,
    custodian: Custodian,
    req: RotateDataKeyRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::ApplicationErrorResponse> {
    let db = &state.db_pool;
    let version = db
        .get_latest_version(&req.identifier)
        .await
        .switch()?
        .increment()
        .switch()?;

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
