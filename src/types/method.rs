use crate::{
    app::AppState,
    core::{custodian::Custodian, DataDecrypter, DataEncrypter},
    errors,
    types::Identifier,
};

#[derive(Eq, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum EncryptionType {
    Single(super::DecryptedData),
    Batch(super::DecryptedDataGroup),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum DecryptionType {
    Single(super::EncryptedData),
    Batch(super::EncryptedDataGroup),
}

impl DecryptionType {
    pub async fn decrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
        custodian: Custodian,
    ) -> errors::CustomResult<EncryptionType, errors::CryptoError> {
        Ok(match self {
            Self::Single(data) => {
                EncryptionType::Single(data.decrypt(state, identifier, custodian).await?)
            }
            Self::Batch(data) => {
                EncryptionType::Batch(data.decrypt(state, identifier, custodian).await?)
            }
        })
    }
}

impl EncryptionType {
    pub async fn encrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
        custodian: Custodian,
    ) -> errors::CustomResult<DecryptionType, errors::CryptoError> {
        Ok(match self {
            Self::Single(data) => {
                DecryptionType::Single(data.encrypt(state, identifier, custodian).await?)
            }
            Self::Batch(data) => {
                DecryptionType::Batch(data.encrypt(state, identifier, custodian).await?)
            }
        })
    }
}
