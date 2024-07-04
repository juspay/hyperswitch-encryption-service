use crate::types::method::DecryptionType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionResponse {
    pub data: DecryptionType,
}
