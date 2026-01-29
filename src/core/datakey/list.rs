use crate::{
    errors::{self, SwitchError},
    multitenancy::TenantState,
    storage::{dek::DataKeyStorageInterface, types::ListKeyInfo},
    types::{requests::ListKeysRequest, response::ListKeysResponse},
};

pub async fn list_data_keys(
    state: TenantState,
    req: ListKeysRequest,
) -> errors::CustomResult<ListKeysResponse, errors::ApplicationErrorResponse> {
    let db = state.get_db_pool();

    let keys = db.get_keys_by_filter(req.key_source).await.switch()?;

    let total_keys = keys.len();

    let keys_info: Vec<ListKeyInfo> = keys
        .into_iter()
        .map(|key| ListKeyInfo {
            version: key.version,
            key_identifier: key.key_identifier,
            data_identifier: key.data_identifier,
        })
        .collect();

    // If batch_size is specified and valid, chunk the keys into batches
    if let Some(batch_size) = req.batch_size.filter(|&size| size > 0) {
        let batched_keys: Vec<Vec<ListKeyInfo>> = keys_info
            .chunks(batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        Ok(ListKeysResponse {
            total_keys,
            keys: None,
            batched_keys: Some(batched_keys),
        })
    } else {
        Ok(ListKeysResponse {
            total_keys,
            keys: Some(keys_info),
            batched_keys: None,
        })
    }
}
