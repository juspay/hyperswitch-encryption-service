use serde::{Deserialize, Serialize};

use crate::types::Identifier;

#[derive(Deserialize, Serialize)]
pub struct CreateDataKeyRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
}

#[derive(Deserialize, Serialize)]
pub struct RotateDataKeyRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
}

#[derive(Deserialize, Serialize)]
pub struct TransferKeyRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub key: masking::StrongSecret<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ReEncryptDataKeysRequest {
    /// Optional identifier to re-encrypt specific DEK. If not provided, all DEKs will be re-encrypted.
    #[serde(flatten)]
    pub identifier: Option<Identifier>,
}
