//! OCI Vault KMS client, built on the shared `oci_kms` crate from the Hyperswitch repository.
//!
//! Authentication (OKE Workload Identity inside Kubernetes, `~/.oci/config` outside it),
//! request signing, timeouts and retries all live in that crate.

use error_stack::{Report, ResultExt};
pub use oci_kms::OciKmsConfig;

use crate::errors::{self, CustomResult};

/// Client for OCI Vault KMS operations.
#[derive(Clone, Debug)]
pub struct OciKmsClient {
    inner: oci_kms::OciKmsClient,
}

impl OciKmsClient {
    /// Credentials are resolved on first use, from the environment rather than from config.
    pub fn new(config: &OciKmsConfig) -> CustomResult<Self, errors::CryptoError> {
        oci_kms::OciKmsClient::new(config)
            .map(|inner| Self { inner })
            .map_err(|error| {
                Report::new(error).change_context(errors::CryptoError::ClientCreationFailed)
            })
    }

    pub fn inner(&self) -> &oci_kms::OciKmsClient {
        &self.inner
    }

    /// Decrypts OCI base64 ciphertext, as produced by `oci kms crypto encrypt`. Used for
    /// bootstrap secrets read from TOML config.
    pub async fn decrypt_secret(&self, data: &str) -> CustomResult<String, errors::CryptoError> {
        let plaintext = self.inner.decrypt(data).await.map_err(|error| {
            Report::new(error).change_context(errors::CryptoError::DecryptionFailed("OCI KMS"))
        })?;

        String::from_utf8(plaintext).change_context(errors::CryptoError::ParseError(
            "Invalid OCI KMS decrypted secret".to_string(),
        ))
    }
}
