use crate::{config::Config, crypto::EncryptionClient, storage::DbState};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub conf: Arc<Config>,
    pub db_pool: Arc<DbState>,
    pub encryption_client: Arc<dyn EncryptionClient>,
}

impl AppState {
    pub async fn from_config(config: &Arc<Config>) -> Self {
        let secrets = config.secrets.clone();
        let db_pool = Arc::new(DbState::from_config(config).await);
        Self {
            conf: config.clone(),
            encryption_client: secrets.get_encryption_client().await,
            db_pool,
        }
    }
}
