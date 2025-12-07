use serde::{Deserialize, Serialize};

use crate::types::{Identifier, key::Version};

#[derive(Deserialize, Serialize)]
pub struct DataKeyCreateResponse {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub key_version: Version,
}
