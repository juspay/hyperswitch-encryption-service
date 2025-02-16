use rustc_hash::FxHashMap;

use crate::{consts::base64::BASE64_ENGINE, types::key::Version};
use base64::engine::Engine;
use masking::PeekInterface;
use serde::{
    de::{self, Deserialize, Deserializer, Unexpected, Visitor},
    Serialize,
};
use std::fmt;

#[derive(Eq, PartialEq, Serialize, serde::Deserialize, Debug, Clone)]
pub struct MultipleDecryptionDataGroup(pub Vec<DecryptedDataGroup>);

#[derive(Eq, PartialEq, Debug, Serialize, serde::Deserialize, Clone)]
pub struct DecryptedDataGroup(pub FxHashMap<String, DecryptedData>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DecryptedData(masking::StrongSecret<Vec<u8>>);

impl DecryptedData {
    pub fn from_data(data: masking::StrongSecret<Vec<u8>>) -> Self {
        Self(data)
    }

    pub fn inner(self) -> masking::StrongSecret<Vec<u8>> {
        self.0
    }
}

impl Serialize for DecryptedData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = BASE64_ENGINE.encode(self.0.peek());
        serializer.serialize_str(&data)
    }
}

impl<'de> Deserialize<'de> for DecryptedData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DecryptedDataVisitor;

        impl Visitor<'_> for DecryptedDataVisitor {
            type Value = DecryptedData;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string of the format {version}:{base64_encoded_data}'")
            }

            fn visit_str<E>(self, value: &str) -> Result<DecryptedData, E>
            where
                E: de::Error,
            {
                let dec_data = BASE64_ENGINE.decode(value).map_err(|err| {
                    let err = err.to_string();
                    E::invalid_value(Unexpected::Str(value), &err.as_str())
                })?;

                Ok(DecryptedData(dec_data.into()))
            }
        }

        deserializer.deserialize_str(DecryptedDataVisitor)
    }
}

#[derive(Eq, PartialEq, Serialize, serde::Deserialize, Debug, Clone)]
pub struct MultipleEncryptionDataGroup(pub Vec<EncryptedDataGroup>);

#[derive(Eq, PartialEq, Serialize, serde::Deserialize, Debug, Clone)]
pub struct EncryptedDataGroup(pub FxHashMap<String, EncryptedData>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EncryptedData {
    pub version: Version,
    pub data: masking::StrongSecret<Vec<u8>>,
}

impl EncryptedData {
    pub fn inner(self) -> masking::StrongSecret<Vec<u8>> {
        self.data
    }
}
impl Serialize for EncryptedData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = BASE64_ENGINE.encode(self.data.peek());
        let encoded = format!("{}:{}", &self.version, data);
        serializer.serialize_str(&encoded)
    }
}

impl<'de> Deserialize<'de> for EncryptedData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EncryptedDataVisitor;

        impl Visitor<'_> for EncryptedDataVisitor {
            type Value = EncryptedData;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string of the format {version}:{base64_encoded_data}'")
            }

            fn visit_str<E>(self, value: &str) -> Result<EncryptedData, E>
            where
                E: de::Error,
            {
                let (version, data) = value.split_once(':').ok_or_else(|| {
                    E::invalid_value(
                        Unexpected::Str(value),
                        &"String should of the format {version}:{base64_encoded_data}",
                    )
                })?;

                let dec_data = BASE64_ENGINE.decode(data).map_err(|err| {
                    let err = err.to_string();
                    E::invalid_value(Unexpected::Str(data), &err.as_str())
                })?;

                let (_, version) = version.split_once('v').ok_or_else(|| {
                    E::invalid_value(
                        Unexpected::Str(version),
                        &"Version should be in the format of v{version_num}",
                    )
                })?;

                let version = version.parse::<i32>().map_err(|_| {
                    E::invalid_value(Unexpected::Str(version), &"Unexpted version number")
                })?;

                Ok(EncryptedData {
                    version: Version::from(version),
                    data: masking::StrongSecret::new(dec_data),
                })
            }
        }

        deserializer.deserialize_str(EncryptedDataVisitor)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::core::key::Version;

    use super::*;

    #[allow(clippy::panic, clippy::unwrap_used)]
    #[test]
    fn test_data_deserialize() {
        #[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
        struct ExtractedEncryptedData {
            data: EncryptedData,
        }

        let data = serde_json::json!({
            "data": "v1:T21naXQnc3dvcmtpbmc=",
        });
        let actual_data: ExtractedEncryptedData = serde_json::from_value(data).unwrap();
        let expected_data = EncryptedData {
            version: Version::from(1),
            data: masking::StrongSecret::new(String::from("Omgit'sworking").as_bytes().to_vec()),
        };

        let expected_data = ExtractedEncryptedData {
            data: expected_data,
        };
        assert_eq!(actual_data, expected_data);
    }
}
