use crate::{
    core::{custodian::Custodian, DataDecrypter, DataEncrypter},
    errors,
    multitenancy::TenantState,
    types::Identifier,
};

#[derive(Eq, PartialEq, Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(untagged)]
pub enum EncryptionType {
    Single(super::DecryptedData),
    Batch(super::DecryptedDataGroup),
    MultiBatch(super::MultipleDecryptionDataGroup),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(untagged)]
pub enum DecryptionType {
    Single(super::EncryptedData),
    Batch(super::EncryptedDataGroup),
    MultiBatch(super::MultipleEncryptionDataGroup),
}

impl DecryptionType {
    pub async fn decrypt(
        self,
        state: &TenantState,
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
            Self::MultiBatch(data) => {
                EncryptionType::MultiBatch(data.decrypt(state, identifier, custodian).await?)
            }
        })
    }
}

impl EncryptionType {
    pub async fn encrypt(
        self,
        state: &TenantState,
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
            Self::MultiBatch(data) => {
                DecryptionType::MultiBatch(data.encrypt(state, identifier, custodian).await?)
            }
        })
    }
}
