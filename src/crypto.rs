pub(crate) mod aes256;
#[cfg(feature = "gcp")]
pub(crate) mod gcp;
#[cfg(feature = "aws")]
pub(crate) mod aws;
#[cfg(feature = "vault")]
pub(crate) mod vault;

use std::{ops::Deref, sync::Arc};

use hyperswitch_masking::StrongSecret;
use strum::{Display, EnumString};

#[cfg(feature = "vault")]
use crate::crypto::vault::Vault;
#[cfg(feature = "aws")]
use crate::services::aws::AwsKmsClient;
#[cfg(feature = "gcp")]
use crate::services::gcp::GcpKmsClient;
use crate::{
    crypto::aes256::GcmAes256,
    env::metrics,
    errors::{self, CustomResult},
};

#[derive(Clone, EnumString, Display)]
pub enum Source {
    KMS,
    AESLocal,
    HashicorpVault,
    GcpKms,
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
        record_key_manager_call(
            <Self as Crypto>::generate_key(self),
            metrics::KeyManagerBackend::AwsKms,
            metrics::KeyManagerOperation::GenerateKey,
        )
        .await
    }

    async fn encrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::encrypt(self, input),
            metrics::KeyManagerBackend::AwsKms,
            metrics::KeyManagerOperation::Encrypt,
        )
        .await
    }

    async fn decrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::decrypt(self, input),
            metrics::KeyManagerBackend::AwsKms,
            metrics::KeyManagerOperation::Decrypt,
        )
        .await
    }
}
#[async_trait::async_trait]
impl KeyManagement for GcmAes256 {
    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::generate_key(self),
            metrics::KeyManagerBackend::Aes256,
            metrics::KeyManagerOperation::GenerateKey,
        )
        .await
    }

    async fn encrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        record_key_manager_call(
            async { <Self as Crypto>::encrypt(self, input) },
            metrics::KeyManagerBackend::Aes256,
            metrics::KeyManagerOperation::Encrypt,
        )
        .await
    }

    async fn decrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        record_key_manager_call(
            async { <Self as Crypto>::decrypt(self, input) },
            metrics::KeyManagerBackend::Aes256,
            metrics::KeyManagerOperation::Decrypt,
        )
        .await
    }
}

#[cfg(feature = "vault")]
#[async_trait::async_trait]
impl KeyManagement for Vault {
    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::generate_key(self),
            metrics::KeyManagerBackend::Vault,
            metrics::KeyManagerOperation::GenerateKey,
        )
        .await
    }

    async fn encrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::encrypt(self, input),
            metrics::KeyManagerBackend::Vault,
            metrics::KeyManagerOperation::Encrypt,
        )
        .await
    }

    async fn decrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::decrypt(self, input),
            metrics::KeyManagerBackend::Vault,
            metrics::KeyManagerOperation::Decrypt,
        )
        .await
    }
}

#[cfg(feature = "gcp")]
#[async_trait::async_trait]
impl KeyManagement for GcpKmsClient {
    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::generate_key(self),
            metrics::KeyManagerBackend::GcpKms,
            metrics::KeyManagerOperation::GenerateKey,
        )
        .await
    }

    async fn encrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::encrypt(self, input),
            metrics::KeyManagerBackend::GcpKms,
            metrics::KeyManagerOperation::Encrypt,
        )
        .await
    }

    async fn decrypt_key(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError> {
        record_key_manager_call(
            <Self as Crypto>::decrypt(self, input),
            metrics::KeyManagerBackend::GcpKms,
            metrics::KeyManagerOperation::Decrypt,
        )
        .await
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

async fn record_key_manager_call<Fut, T, E>(
    future: Fut,
    backend: metrics::KeyManagerBackend,
    operation: metrics::KeyManagerOperation,
) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>> + Send,
{
    let start = std::time::Instant::now();
    let result = future.await;
    let duration = start.elapsed();
    let outcome = if result.is_ok() { "success" } else { "error" };

    metrics::KEY_MANAGER_CALL_DURATION.record(
        duration.as_secs_f64(),
        metrics_utils::metric_attributes!(
            ("backend", backend),
            ("operation", operation),
            ("outcome", outcome),
        ),
    );

    result
}
