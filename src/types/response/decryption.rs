use crate::types::method::EncryptionType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptionResponse {
    pub data: EncryptionType,
}
