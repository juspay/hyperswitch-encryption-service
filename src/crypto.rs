use masking::StrongSecret;
use strum::{Display, EnumString};

use std::{ops::Deref, sync::Arc};

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

#[async_trait::async_trait]
pub trait KeyManagement {
    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError>;
    async fn encrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError>;
    async fn decrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError>;
}

#[cfg(feature = "aws")]
#[async_trait::async_trait]
impl KeyManagement for AwsKmsClient {
    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        <Self as Crypto>::generate_key(self).await
    }
    async fn encrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        <Self as Crypto>::encrypt(self, input).await
    }
    async fn decrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        <Self as Crypto>::decrypt(self, input).await
    }
}

#[cfg(not(feature = "aws"))]
#[async_trait::async_trait]
impl KeyManagement for GcmAes256 {
    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        <Self as Crypto>::generate_key(self).await
    }
    async fn encrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        <Self as Crypto>::encrypt(self, input)
    }
    async fn decrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        <Self as Crypto>::decrypt(self, input)
    }
}

pub struct EncryptionClient<T: KeyManagement> {
    client: Arc<T>,
}

impl<T: KeyManagement> EncryptionClient<T> {
    pub fn new(client: T) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}
#[cfg(feature = "aws")]
pub type KeyManagerClient = EncryptionClient<AwsKmsClient>;

#[cfg(not(feature = "aws"))]
pub type KeyManagerClient = EncryptionClient<GcmAes256>;

impl<T: KeyManagement> EncryptionClient<T> {
    pub fn client(&self) -> &T {
        &self.client
    }
}

impl<T: KeyManagement> Deref for EncryptionClient<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.client()
    }
}
