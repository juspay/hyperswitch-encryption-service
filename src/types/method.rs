use crate::{
    core::{custodian::Custodian, DataDecrypter, DataEncrypter},
    errors,
    multitenancy::TenantState,
    types::Identifier,
};
use futures::future;

#[derive(Eq, PartialEq, Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(untagged)]
pub enum EncryptionType {
    Single(super::DecryptedData),
    Batch(super::DecryptedDataGroup),
    MultiBatch(Vec<super::DecryptedDataGroup>)
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(untagged)]
pub enum DecryptionType {
    Single(super::EncryptedData),
    Batch(super::EncryptedDataGroup),
    MultiBatch(Vec<super::EncryptedDataGroup>)
}

pub struct MultiBatchProcessor;

impl MultiBatchProcessor {
    pub async fn decrypt(
        data: Vec<super::EncryptedDataGroup>,
        state: &TenantState,
        identifier: &Identifier,
        custodian: Custodian,
    ) -> errors::CustomResult<EncryptionType, errors::CryptoError> {
        let decrypted_data_groups = future::try_join_all(data.into_iter().map(|encrypted_data_group| {
            encrypted_data_group.decrypt(state, identifier, custodian.clone())
        }))
        .await?;

        Ok(EncryptionType::MultiBatch(decrypted_data_groups))
    }

    pub async fn encrypt(
        data: Vec<super::DecryptedDataGroup>,
        state: &TenantState,
        identifier: &Identifier,
        custodian: Custodian,
    ) -> errors::CustomResult<DecryptionType, errors::CryptoError> {
        let encrypted_data_groups = future::try_join_all(data.into_iter().map(|decrypted_data_group| {
            decrypted_data_group.encrypt(state, identifier, custodian.clone())
        }))
        .await?;

        Ok(DecryptionType::MultiBatch(encrypted_data_groups))
    }
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
                MultiBatchProcessor::decrypt(data, state, identifier, custodian).await?
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
            Self::MultiBatch(data) =>{
                MultiBatchProcessor::encrypt(data, state, identifier, custodian).await?
            }
        })
    }
}
