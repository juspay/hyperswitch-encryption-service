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
