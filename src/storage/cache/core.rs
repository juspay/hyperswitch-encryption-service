use crate::{
    errors,
    multitenancy::TenantState,
    storage::cache::{Cache, Key},
};

pub async fn get_or_populate_cache<T, Fut>(
    tenant: &TenantState,
    key: String,
    cache: &Cache<T>,
    f: Fut,
) -> errors::CustomResult<T, errors::DatabaseError>
where
    T: Clone + Sync + Send + 'static,
    Fut: futures::Future<Output = errors::CustomResult<T, errors::DatabaseError>> + Send,
{
    let key = Key::from_state(tenant, key);

    if let Some(val) = cache.get(&key).await {
        Ok(val)
    } else {
        let val = f.await?;
        cache.push(key, val.clone()).await;
        Ok(val)
    }
}
