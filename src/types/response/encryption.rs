use crate::types::{
    core::{EncryptedData, EncryptedDataGroup},
    method::DecryptionType,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum EncryptionResponse {
    Single(EncryptedData),
    Batch(EncryptedDataGroup),
}

impl From<DecryptionType> for EncryptionResponse {
    fn from(item: DecryptionType) -> Self {
        match item {
            DecryptionType::Single(data) => Self::Single(data),
            DecryptionType::Batch(data) => Self::Batch(data),
        }
    }
}
