use serde::{Deserialize, Serialize};

use crate::types::core::{DecryptedDataGroup, Identifier};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct EncryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptedDataGroup,
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use rustc_hash::FxHashMap;

    use crate::{
        consts::base64::BASE64_ENGINE,
        types::core::{DecryptedData, DecryptedDataGroup},
    };

    use super::*;

    #[allow(clippy::panic, clippy::unwrap_used)]
    #[test]
    fn test_enc_request_deserialize() {
        let test_data = serde_json::json!({
            "data_identifier": "User",
            "key_identifier": "123",
            "data": {
                "ff": "U2VjcmV0RGF0YQo="
            }
        });
        let data = BASE64_ENGINE.decode("U2VjcmV0RGF0YQo=").unwrap();
        let actual_data: EncryptDataRequest = serde_json::from_value(test_data).unwrap();
        let mut hash = FxHashMap::default();
        hash.insert(String::from("ff"), DecryptedData::from_data(data.into()));

        let expected_data = EncryptDataRequest {
            identifier: Identifier::User(String::from("123")),
            data: DecryptedDataGroup(hash),
        };

        assert_eq!(actual_data, expected_data);
    }
}
