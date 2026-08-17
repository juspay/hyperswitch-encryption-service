#[cfg(feature = "mtls")]
pub mod tls;

use std::sync::Arc;

#[cfg(not(feature = "cassandra"))]
use diesel_async::{AsyncPgConnection, pooled_connection::bb8::Pool};
use rayon::{ThreadPool, ThreadPoolBuilder};
use rustc_hash::FxHashMap;

use crate::{
    config::{Config, TenantConfig},
    crypto::KeyManagerClient,
    multitenancy::{MultiTenant, TenantId, TenantState},
    storage::{DbState, adapter, cache::Caches},
};

#[cfg(not(feature = "cassandra"))]
pub(crate) type StorageState = DbState<Pool<AsyncPgConnection>, adapter::PostgreSQL>;

#[cfg(feature = "cassandra")]
pub(crate) type StorageState =
    DbState<scylla::client::caching_session::CachingSession, adapter::Cassandra>;

pub struct AppState {
    pub conf: Config,
    pub tenant_states: MultiTenant<TenantState>,
}

impl AppState {
    pub async fn from_config(config: Config) -> Self {
        let mut tenants = FxHashMap::default();

        for (tenant_id, tenant) in &config.multitenancy.tenants.0 {
            let tenant_id = TenantId::new(tenant_id.clone());
            tenants.insert(
                tenant_id.clone(),
                TenantState::new(Arc::new(
                    SessionState::from_config(&config, &tenant_id, tenant).await,
                )),
            );
        }

        Self {
            conf: config,
            tenant_states: tenants,
        }
    }
}

pub struct SessionState {
    pub caches: Caches,
    pub thread_pool: ThreadPool,
    pub keymanager_client: KeyManagerClient,
    db_pool: StorageState,
}

impl SessionState {
    /// # Panics
    ///
    /// Panics if failed to build thread pool
    #[allow(clippy::expect_used)]
    pub async fn from_config(
        config: &Config,
        tenant_id: &TenantId,
        tenant_config: &TenantConfig,
    ) -> Self {
        let secrets = config.secrets.clone();
        let db_pool = StorageState::from_config(config, tenant_id, &tenant_config.schema).await;
        let num_threads = config.pool_config.pool;

        Self {
            caches: Caches::from_config(&config.cache, tenant_id),
            keymanager_client: secrets.create_keymanager_client().await,
            db_pool,
            thread_pool: ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .expect("Failed to create a threadpool"),
        }
    }

    pub fn db_pool(&self) -> &StorageState {
        &self.db_pool
    }
}
