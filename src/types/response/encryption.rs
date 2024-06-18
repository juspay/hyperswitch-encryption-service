use crate::types::EncryptedDataGroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionResponse {
    pub data: EncryptedDataGroup,
}
