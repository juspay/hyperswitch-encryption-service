use crate::{errors, storage::cache::Cache};

pub async fn get_or_populate_cache<T, Fut>(
    key: String,
    cache: &Cache<T>,
    f: Fut,
) -> errors::CustomResult<T, errors::DatabaseError>
where
    T: Clone + Sync + Send + 'static,
    Fut: futures::Future<Output = errors::CustomResult<T, errors::DatabaseError>> + Send,
{
    if let Some(val) = cache.get(&key).await {
        Ok(val)
    } else {
        let val = f.await?;
        cache.push(key, val.clone()).await;
        Ok(val)
    }
}
