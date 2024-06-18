use crate::types::core::EncryptedDataGroup;
use crate::types::core::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DecryptionRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: EncryptedDataGroup,
}
