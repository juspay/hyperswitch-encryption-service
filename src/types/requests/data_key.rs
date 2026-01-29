use serde::{Deserialize, Serialize};

#[cfg(feature = "aws")]
use crate::storage::types::ListKeyInfo;
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
pub struct ListKeysRequest {
    pub key_source: Option<Source>,
    pub batch_size: Option<usize>,
}

#[cfg(feature = "aws")]
#[derive(Deserialize, Serialize)]
pub struct ReEncryptDataKeysRequest {
    pub keys: Option<Vec<ListKeyInfo>>,
}
