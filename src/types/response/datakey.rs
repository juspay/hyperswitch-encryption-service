use crate::types::{key::Version, Identifier};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DataKeyCreateResponse {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub key_version: Version,
}
