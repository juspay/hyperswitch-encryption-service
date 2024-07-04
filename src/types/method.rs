use crate::{
    app::AppState,
    core::{DataDecrypt, DataEncrypt},
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
    ) -> errors::CustomResult<EncryptionType, errors::CryptoError> {
        Ok(match self {
            Self::Single(data) => EncryptionType::Single(data.decrypt(state, identifier).await?),
            Self::Batch(data) => EncryptionType::Batch(data.decrypt(state, identifier).await?),
        })
    }
}

impl EncryptionType {
    pub async fn encrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
    ) -> errors::CustomResult<DecryptionType, errors::CryptoError> {
        Ok(match self {
            Self::Single(data) => DecryptionType::Single(data.encrypt(state, identifier).await?),
            Self::Batch(data) => DecryptionType::Batch(data.encrypt(state, identifier).await?),
        })
    }
}
