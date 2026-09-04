use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_kms::{Client, config::Region, primitives::Blob};
use base64::Engine;
use error_stack::{IntoReport, ResultExt};

use crate::errors::{self, CustomResult, SwitchError};

/// Configuration parameters required for constructing a [`AwsKmsClient`].
#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AwsKmsConfig {
    /// The AWS key identifier of the KMS key used to encrypt or decrypt data.
    pub key_id: String,

    /// The AWS region to send KMS requests to.
    pub region: String,

    /// When true, omits key_id from decryption requests, allowing KMS to determine the key from ciphertext metadata.
    pub skip_key_id_on_decrypt: bool,
}

impl AwsKmsConfig {
    /// Both `key_id` and `region` are required to send KMS requests.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.key_id.trim().is_empty() {
            return Err("AWS KMS key ID must not be empty");
        }
        if self.region.trim().is_empty() {
            return Err("AWS KMS region must not be empty");
        }
        Ok(())
    }
}

/// Client for AWS KMS operations.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    inner_client: Client,
    key_id: String,
    skip_key_id_on_decrypt: bool,
}

impl AwsKmsClient {
    pub async fn new(config: &AwsKmsConfig) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let sdk_config = aws_config::defaults(BehaviorVersion::v2026_01_12())
            .region(region_provider)
            .load()
            .await;

        Self {
            inner_client: Client::new(&sdk_config),
            key_id: config.key_id.clone(),
            skip_key_id_on_decrypt: config.skip_key_id_on_decrypt,
        }
    }

    pub fn inner_client(&self) -> &Client {
        &self.inner_client
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn skip_key_id_on_decrypt(&self) -> bool {
        self.skip_key_id_on_decrypt
    }

    /// Decrypts base64-encoded `data` via AWS KMS. Used for bootstrap secrets read from
    /// TOML config.
    pub async fn decrypt_secret(
        &self,
        data: impl AsRef<[u8]>,
    ) -> CustomResult<String, errors::CryptoError> {
        let ciphertext_blob_data = crate::consts::base64::BASE64_ENGINE
            .decode(data)
            .change_context(errors::CryptoError::ParseError(
                "Failed to base64 decode AWS KMS ciphertext".to_string(),
            ))?;

        let ciphertext_blob = Blob::new(ciphertext_blob_data);
        let mut decrypt_request = self.inner_client.decrypt().ciphertext_blob(ciphertext_blob);

        if !self.skip_key_id_on_decrypt {
            decrypt_request = decrypt_request.key_id(&self.key_id);
        }

        let decrypted_output = decrypt_request.send().await.switch()?;

        let plaintext = decrypted_output
            .plaintext
            .ok_or(errors::CryptoError::DecryptionFailed("KMS").into_report())?
            .into_inner();

        String::from_utf8(plaintext).change_context(errors::CryptoError::ParseError(
            "Invalid AWS KMS decrypted secret".to_string(),
        ))
    }
}
