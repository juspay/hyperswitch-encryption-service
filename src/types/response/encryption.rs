use serde::{Deserialize, Serialize};

use crate::types::method::DecryptionType;

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionResponse {
    pub data: DecryptionType,
}
