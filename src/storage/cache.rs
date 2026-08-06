mod statics;

use moka::future::Cache as MokaCache;

pub use self::statics::{KEY_CACHE, VERSION_CACHE};
use crate::{env::metrics, errors, multitenancy::TenantState};

#[derive(PartialEq, Eq, Hash)]
pub struct Key {
    prefix: String,
    key: String,
}

impl Key {
    // Taking TenantState instead of cache_prefix here because both of them are String type and
    // it's easy to interchange these accidentally
    pub fn from_state(tenant: &TenantState, key: String) -> Self {
        Self {
            prefix: tenant.cache_prefix.clone(),
            key,
        }
    }
}

pub struct Cache<V: Send + Sync + Clone>
where
    V: Send + Sync + Clone,
{
    inner: MokaCache<Key, V>,
    name: &'static str,
}

impl<V> Cache<V>
where
    V: Send + Sync + Clone + 'static,
{
    fn new(
        name: &'static str,
        time_to_live: u64,
        time_to_idle: u64,
        max_capacity: Option<u64>,
    ) -> Self {
        let mut cache_builder = MokaCache::builder()
            .time_to_live(std::time::Duration::from_secs(time_to_live))
            .time_to_idle(std::time::Duration::from_secs(time_to_idle))
            .eviction_listener({
                move |_key, _value, removal_cause| {
                    cache_eviction_listener(name, removal_cause);
                }
            });
        if let Some(capacity) = max_capacity {
            cache_builder = cache_builder.max_capacity(capacity * 1024 * 1024);
        }

        Self {
            inner: cache_builder.build(),
            name,
        }
    }

    pub async fn push(&self, key: Key, val: V) {
        self.inner.insert(key, val).await;
        metrics::CACHE_INSERT_COUNT.add(1, metrics_utils::metric_attributes!(("cache", self.name)));
    }

    pub async fn get(&self, key: &Key) -> Option<V> {
        let value = self.inner.get(key).await;

        metrics::CACHE_LOOKUP_COUNT.add(
            1,
            metrics_utils::metric_attributes!(
                ("cache", self.name),
                ("outcome", if value.is_some() { "hit" } else { "miss" })
            ),
        );

        value
    }

    pub async fn record_entry_count_metric(&self) {
        self.inner.run_pending_tasks().await;

        metrics::CACHE_ENTRY_COUNT.record(
            self.inner.entry_count(),
            metrics_utils::metric_attributes!(("cache", self.name),),
        );
    }
}

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

fn cache_eviction_listener(
    cache_name: &'static str,
    removal_cause: moka::notification::RemovalCause,
) {
    use moka::notification::RemovalCause;

    let removal_cause_label = match removal_cause {
        RemovalCause::Expired => "expired",
        RemovalCause::Explicit => "explicit",
        RemovalCause::Replaced => "replaced",
        RemovalCause::Size => "size",
    };

    metrics::CACHE_REMOVAL_COUNT.add(
        1,
        metrics_utils::metric_attributes!(
            ("cache", cache_name),
            ("removal_cause", removal_cause_label)
        ),
    );
}
