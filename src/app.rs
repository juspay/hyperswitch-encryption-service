#[cfg(feature = "mtls")]
pub mod tls;

use crate::{config::Config, crypto::KeyManagerClient, storage::DbState};

use crate::storage::adapter;

#[cfg(not(feature = "cassandra"))]
use diesel_async::pooled_connection::bb8::Pool;
#[cfg(not(feature = "cassandra"))]
use diesel_async::AsyncPgConnection;

use rayon::{ThreadPool, ThreadPoolBuilder};

#[cfg(not(feature = "cassandra"))]
type StorageState = DbState<Pool<AsyncPgConnection>, adapter::PostgreSQL>;

#[cfg(feature = "cassandra")]
type StorageState = DbState<scylla::CachingSession, adapter::Cassandra>;

pub struct AppState {
    pub conf: Config,
    pub db_pool: StorageState,
    pub keymanager_client: KeyManagerClient,
    pub thread_pool: ThreadPool,
}

impl AppState {
    /// # Panics
    ///
    /// Panics if failed to build thread pool
    #[allow(clippy::expect_used)]
    pub async fn from_config(config: Config) -> Self {
        let secrets = config.secrets.clone();
        let db_pool = StorageState::from_config(&config).await;
        let num_threads = config.pool_config.pool;

        Self {
            conf: config,
            keymanager_client: secrets.create_keymanager_client().await,
            db_pool,
            thread_pool: ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .expect("Failed to create a threadpool"),
        }
    }
}
