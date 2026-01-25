use crate::{
    errors::{self, SwitchError},
    multitenancy::TenantState,
    storage::dek::DataKeyStorageInterface,
    types::{requests::ListKeysRequest, response::ListKeysResponse},
};

pub async fn list_data_keys(
    state: TenantState,
    req: ListKeysRequest,
) -> errors::CustomResult<ListKeysResponse, errors::ApplicationErrorResponse> {
    let db = state.get_db_pool();

    let keys = db
        .get_keys_by_filter(req.key_source.clone())
        .await
        .switch()?;

    let total_keys = keys.len();

    let k_ids: Vec<i32> = keys.into_iter().map(|key| key.id).collect();

    // If batch_size is specified and valid, chunk the keys into batches
    if let Some(batch_size) = req.batch_size.filter(|&size| size > 0) {
        let batched_keys: Vec<Vec<i32>> = k_ids
            .chunks(batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        Ok(ListKeysResponse {
            total_keys,
            key_ids: None,
            batched_key_ids: Some(batched_keys),
        })
    } else {
        Ok(ListKeysResponse {
            total_keys,
            key_ids: Some(k_ids),
            batched_key_ids: None,
        })
    }
}
