use crate::types::{
    core::{DecryptedData, DecryptedDataGroup},
    method::EncryptionType,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum DecryptionResponse {
    Single(DecryptedData),
    Batch(DecryptedDataGroup),
}

impl From<EncryptionType> for DecryptionResponse {
    fn from(item: EncryptionType) -> Self {
        match item {
            EncryptionType::Single(data) => Self::Single(data),
            EncryptionType::Batch(data) => Self::Batch(data),
        }
    }
}
