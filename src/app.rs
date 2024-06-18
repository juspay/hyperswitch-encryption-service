#[cfg(feature = "mtls")]
pub mod tls;

use crate::{config::Config, crypto::KeyManagerClient, storage::DbState};

pub struct AppState {
    pub conf: Config,
    pub db_pool: DbState,
    pub keymanager_client: KeyManagerClient,
}

impl AppState {
    pub async fn from_config(config: Config) -> Self {
        let secrets = config.secrets.clone();
        let db_pool = DbState::from_config(&config).await;
        Self {
            conf: config,
            keymanager_client: secrets.create_keymanager_client().await,
            db_pool,
        }
    }
}
