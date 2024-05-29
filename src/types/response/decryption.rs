use crate::types::DecryptedData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptionResponse {
    pub data: DecryptedData,
}
