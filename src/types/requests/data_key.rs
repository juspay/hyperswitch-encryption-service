use serde::{Deserialize, Serialize};

use crate::{crypto::Source, types::Identifier};

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

#[derive(Deserialize, Serialize)]
pub struct ListKeysRequest {
    pub key_source: Option<Source>,
    pub batch_size: Option<usize>,
}
