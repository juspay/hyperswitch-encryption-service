#[cfg(feature = "mtls")]
pub mod tls;

use crate::storage::adapter;
use crate::{
    config::{Config, TenantConfig},
    crypto::blake3::Blake3,
    crypto::KeyManagerClient,
    multitenancy::{MultiTenant, TenantId, TenantState},
    storage::DbState,
};
use std::sync::Arc;

#[cfg(not(feature = "cassandra"))]
use diesel_async::pooled_connection::bb8::Pool;
#[cfg(not(feature = "cassandra"))]
use diesel_async::AsyncPgConnection;

use rayon::{ThreadPool, ThreadPoolBuilder};
use rustc_hash::FxHashMap;

#[cfg(not(feature = "cassandra"))]
pub(crate) type StorageState = DbState<Pool<AsyncPgConnection>, adapter::PostgreSQL>;

#[cfg(feature = "cassandra")]
pub(crate) type StorageState = DbState<scylla::CachingSession, adapter::Cassandra>;

pub struct AppState {
    pub conf: Config,
    pub tenant_states: MultiTenant<TenantState>,
}

impl AppState {
    pub async fn from_config(config: Config) -> Self {
        let mut tenants = FxHashMap::default();

        for (tenant_id, tenant) in &config.multitenancy.tenants.0 {
            tenants.insert(
                TenantId::new(tenant_id.clone()),
                TenantState::new(Arc::new(SessionState::from_config(&config, tenant).await)),
            );
        }

        Self {
            conf: config,
            tenant_states: tenants,
        }
    }
}

pub struct SessionState {
    pub cache_prefix: String,
    pub thread_pool: ThreadPool,
    pub keymanager_client: KeyManagerClient,
    db_pool: StorageState,
    global_db_pool: StorageState,
    pub hash_client: Blake3,
}

impl SessionState {
    /// # Panics
    ///
    /// Panics if failed to build thread pool
    #[allow(clippy::expect_used)]
    pub async fn from_config(config: &Config, tenant_config: &TenantConfig) -> Self {
        let secrets = config.secrets.clone();
        let db_pool = StorageState::from_config(config, &tenant_config.schema).await;
        let global_db_pool =
            StorageState::from_config(config, &config.multitenancy.global_tenant.0.schema).await;

        let num_threads = config.pool_config.pool;
        let hash_client = Blake3::from_config(config).await;

        Self {
            cache_prefix: tenant_config.cache_prefix.clone(),
            keymanager_client: secrets.create_keymanager_client().await,
            db_pool,
            global_db_pool,
            hash_client,
            thread_pool: ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .expect("Failed to create a threadpool"),
        }
    }

    pub fn global_db_pool(&self) -> &StorageState {
        &self.global_db_pool
    }

    pub fn db_pool(&self) -> &StorageState {
        &self.db_pool
    }
}
