use crate::types::{core::Identifier, method::DecryptionType};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DecryptionRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptionType,
}
