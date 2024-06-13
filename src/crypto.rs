use masking::StrongSecret;
use strum::{Display, EnumString};

use std::sync::Arc;

use crate::errors::{self, CustomResult};

pub(crate) mod aes256;

#[cfg(feature = "aws")]
use crate::services::aws::AwsKmsClient;

#[cfg(not(feature = "aws"))]
use crate::crypto::aes256::GcmAes256;

#[cfg(feature = "aws")]
pub(crate) mod kms;

#[derive(Clone, EnumString, Display)]
pub enum Source {
    KMS,
    AESLocal,
}

#[async_trait::async_trait]
pub trait Crypto {
    type DataReturn<'a>
    where
        Self: 'a;

    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError>;
    fn encrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_>;
    fn decrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_>;
}

pub enum EncryptionClient {
    #[cfg(feature = "aws")]
    Aws(Arc<AwsKmsClient>),
    #[cfg(not(feature = "aws"))]
    Aes(Arc<GcmAes256>),
}

impl EncryptionClient {
    pub async fn encrypt(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        match self {
            #[cfg(feature = "aws")]
            Self::Aws(client) => client.encrypt(input).await,
            #[cfg(not(feature = "aws"))]
            Self::Aes(client) => client.encrypt(input),
        }
    }

    pub async fn decrypt(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        match self {
            #[cfg(feature = "aws")]
            Self::Aws(client) => client.decrypt(input).await,
            #[cfg(not(feature = "aws"))]
            Self::Aes(client) => client.decrypt(input),
        }
    }

    pub async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        match self {
            #[cfg(feature = "aws")]
            Self::Aws(client) => client.generate_key().await,
            #[cfg(not(feature = "aws"))]
            Self::Aes(client) => client.generate_key().await,
        }
    }
}
