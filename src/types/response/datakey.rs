use serde::{Deserialize, Serialize};

#[cfg(feature = "aws")]
use crate::storage::types::ListKeyInfo;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<ListKeyInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batched_keys: Option<Vec<Vec<ListKeyInfo>>>,
}

#[cfg(feature = "aws")]
#[derive(Deserialize, Serialize)]
pub struct ReEncryptDataKeysResponse {
    pub total_processed_keys: usize,
    pub succeeded_keys: usize,
    pub skipped_keys: usize,
    pub failed_keys: usize,
    pub failed_keys_info: Vec<ListKeyInfo>,
}
