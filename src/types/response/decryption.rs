use serde::{Deserialize, Serialize};

use crate::types::method::EncryptionType;

#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptionResponse {
    pub data: EncryptionType,
}
