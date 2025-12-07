pub(crate) mod aes256;
pub(crate) mod blake3;
pub(crate) mod kms;
pub(crate) mod vault;

use std::{ops::Deref, sync::Arc};

use masking::StrongSecret;
use strum::{Display, EnumString};

use crate::{
    crypto::{aes256::GcmAes256, vault::Vault},
    errors::{self, CustomResult},
    services::aws::AwsKmsClient,
};

#[derive(Clone, EnumString, Display)]
pub enum Source {
    KMS,
    AESLocal,
    HashicorpVault,
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

#[async_trait::async_trait]
impl KeyManagement for Vault {
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

pub type Backend = dyn KeyManagement + Send + Sync;

#[derive(Clone)]
pub struct KeyManagerClient {
    client: Arc<Backend>,
}

impl KeyManagerClient {
    pub fn new(client: Arc<Backend>) -> Self {
        Self { client }
    }
}

impl KeyManagerClient {
    pub fn client(&self) -> &Arc<Backend> {
        &self.client
    }
}

impl Deref for KeyManagerClient {
    type Target = Arc<Backend>;
    fn deref(&self) -> &Self::Target {
        self.client()
    }
}
