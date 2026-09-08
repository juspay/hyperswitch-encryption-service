use moka::future::Cache as MokaCache;

use crate::{
    config::CacheConfig,
    env::metrics,
    errors,
    multitenancy::TenantId,
    types::{Key, key::Version},
};

#[derive(PartialEq, Eq, Hash)]
pub struct CacheKey {
    key: String,
}

impl CacheKey {
    fn new(key: String) -> Self {
        Self { key }
    }
}

pub struct Cache<V: Send + Sync + Clone>
where
    V: Send + Sync + Clone,
{
    inner: MokaCache<CacheKey, V>,
    name: &'static str,
    // The observable instruments aren't read directly, we just need to keep them alive for metrics
    // to be recorded.
    _entry_count: metrics_utils::opentelemetry::ObservableGauge<u64>,
}

impl<V> Cache<V>
where
    V: Send + Sync + Clone + 'static,
{
    fn new(
        name: &'static str,
        time_to_live: u64,
        time_to_idle: u64,
        max_capacity: u64,
        tenant_id: &TenantId,
    ) -> Self {
        let cache_builder = MokaCache::builder()
            .time_to_live(std::time::Duration::from_secs(time_to_live))
            .time_to_idle(std::time::Duration::from_secs(time_to_idle))
            .max_capacity(max_capacity)
            .eviction_listener({
                move |_key, _value, removal_cause| {
                    cache_eviction_listener(name, removal_cause);
                }
            });

        let inner = cache_builder.build();

        let _entry_count = {
            let inner_clone = inner.clone();
            let tenant_id_clone = tenant_id.to_owned();
            metrics::CRIPTA_METER
                .u64_observable_gauge("cache.entry.count")
                .with_description("Current number of cache entries")
                .with_unit("{entry}")
                .with_callback(move |observer| {
                    let tenant_id = tenant_id_clone.clone().into_inner();

                    observer.observe(
                        inner_clone.entry_count(),
                        metrics_utils::metric_attributes!(
                            ("cache", name),
                            ("tenant_id", tenant_id)
                        ),
                    );
                })
                .build()
        };

        Self {
            inner,
            name,
            _entry_count,
        }
    }

    pub async fn push(&self, key: CacheKey, val: V) {
        self.inner.insert(key, val).await;
        metrics::CACHE_INSERT_COUNT.add(1, metrics_utils::metric_attributes!(("cache", self.name)));
    }

    pub async fn get(&self, key: &CacheKey) -> Option<V> {
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
}

pub struct Caches {
    pub version: Cache<Version>,
    pub key: Cache<Key>,
}

impl Caches {
    pub fn from_config(config: &CacheConfig, tenant_id: &TenantId) -> Self {
        Self {
            version: Cache::new(
                "version",
                config.time_to_live_secs,
                config.time_to_idle_secs,
                config.max_capacity.get(),
                tenant_id,
            ),
            key: Cache::new(
                "key",
                config.time_to_live_secs,
                config.time_to_idle_secs,
                config.max_capacity.get(),
                tenant_id,
            ),
        }
    }
}

pub async fn get_or_populate_cache<T, Fut>(
    key: String,
    cache: &Cache<T>,
    f: Fut,
) -> errors::CustomResult<T, errors::DatabaseError>
where
    T: Clone + Sync + Send + 'static,
    Fut: futures::Future<Output = errors::CustomResult<T, errors::DatabaseError>> + Send,
{
    let key = CacheKey::new(key);

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
