use crate::types::Identifier;
use serde::{Deserialize, Serialize};

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
    pub key: String,
}
