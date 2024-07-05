#[cfg(feature = "mtls")]
pub mod tls;

use crate::{config::Config, crypto::KeyManagerClient, storage::DbState};
use rayon::{ThreadPool, ThreadPoolBuilder};

pub struct AppState {
    pub conf: Config,
    pub db_pool: DbState,
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
        let db_pool = DbState::from_config(&config).await;
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
