use serde::{Deserialize, Serialize};

use crate::types::core::{DecryptedData, Identifier};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct EncryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptedData,
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use crate::{consts::base64::BASE64_ENGINE, types::core::DecryptedData};

    use super::*;

    #[test]
    fn test_enc_request_deserialize() {
        let test_data = serde_json::json!({
            "data_identifier": "User",
            "key_identifier": "123",
            "data": "U2VjcmV0RGF0YQo="
        });
        let data = BASE64_ENGINE.decode("U2VjcmV0RGF0YQo=").unwrap();
        let actual_data: EncryptDataRequest = serde_json::from_value(test_data).unwrap();
        let expected_data = EncryptDataRequest {
            identifier: Identifier::User(String::from("123")),
            data: DecryptedData::from_data(data.into()),
        };

        assert_eq!(actual_data, expected_data);
    }
}
