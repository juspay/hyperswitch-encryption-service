use crate::{config::Config, storage::DbState};
use std::sync::Arc;

#[cfg(feature = "aws")]
use crate::services::aws::AwsKmsClient;

#[derive(Clone)]
pub struct AppState {
    pub conf: Arc<Config>,
    pub db_pool: Arc<DbState>,
    #[cfg(feature = "aws")]
    pub aws_client: Arc<AwsKmsClient>,
}

impl AppState {
    pub async fn from_config(config: &Arc<Config>) -> Self {
        Self {
            conf: config.clone(),
            db_pool: Arc::new(DbState::from_config(config).await),
            #[cfg(feature = "aws")]
            aws_client: Arc::new(AwsKmsClient::new(&config.kms_config).await),
        }
    }
}
