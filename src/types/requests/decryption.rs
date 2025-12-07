use serde::{Deserialize, Serialize};

use crate::types::{core::Identifier, method::DecryptionType};

#[derive(Serialize, Deserialize, Debug)]
pub struct DecryptionRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptionType,
}
