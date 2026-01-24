use serde::{Deserialize, Serialize};

use crate::{storage::types::ListKeyInfo, types::{Identifier, key::Version}};

#[derive(Deserialize, Serialize)]
pub struct DataKeyCreateResponse {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub key_version: Version,
}

#[derive(Deserialize, Serialize)]
pub struct ReEncryptDataKeysResponse {
    pub total_keys: usize,
    pub success_count: usize,
    pub failure_count: usize,
}

#[derive(Deserialize, Serialize)]
pub struct ListKeysResponse {
    pub total_keys: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<ListKeyInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batched_keys: Option<Vec<Vec<ListKeyInfo>>>,
}
