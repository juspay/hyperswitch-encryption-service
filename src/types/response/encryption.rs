use crate::types::EncryptedData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionResponse {
    pub data: EncryptedData,
}
