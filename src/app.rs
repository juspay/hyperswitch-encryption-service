use crate::{config::Config, crypto::EC, storage::DbState};

pub struct AppState {
    pub conf: Config,
    pub db_pool: DbState,
    pub encryption_client: EC,
}

impl AppState {
    pub async fn from_config(config: Config) -> Self {
        let secrets = config.secrets.clone();
        let db_pool = DbState::from_config(&config).await;
        Self {
            conf: config,
            encryption_client: secrets.create_encryption_client().await,
            db_pool,
        }
    }
}
