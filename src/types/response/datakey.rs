use serde::{Deserialize, Serialize};

use crate::types::{Identifier, key::Version};

#[derive(Deserialize, Serialize)]
pub struct DataKeyCreateResponse {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub key_version: Version,
}

#[derive(Deserialize, Serialize)]
pub struct ListKeysResponse {
    pub total_keys: usize,
    pub key_ids: Vec<Vec<i32>>,
}

#[cfg(feature = "aws")]
#[derive(Deserialize, Serialize)]
pub struct ReEncryptDataKeysResponse {
    pub total_processed_keys: usize,
    pub succeeded_keys: usize,
    pub skipped_keys: usize,
    pub failed_keys: usize,
    pub failed_key_ids: Vec<i32>,
}
