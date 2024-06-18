use crate::types::DecryptedDataGroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptionResponse {
    pub data: DecryptedDataGroup,
}
