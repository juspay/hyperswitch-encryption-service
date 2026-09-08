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

    let keys = db.get_keys_by_filter(req.key_source).await.switch()?;

    let total_keys = keys.len();

    let keys_info: Vec<i32> = keys.into_iter().map(|key| key.id).collect();

    let batch_size = req.batch_size.filter(|&size| size > 0);

    let batched_keys: Vec<Vec<i32>> = keys_info
        .chunks(batch_size.unwrap_or(keys_info.len()))
        .map(|chunk| chunk.to_vec())
        .collect();

    Ok(ListKeysResponse {
        total_keys,
        key_ids: batched_keys,
    })
}
