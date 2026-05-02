use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_kms::{Client, config::Region};

/// Configuration parameters for KMS encryption operations.
#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AwsKmsEncryptionConfig {
    /// The AWS key identifier of the KMS key used to encrypt data.
    pub key_id: String,

    /// The AWS region to send KMS encryption requests to.
    pub region: String,
}

/// Configuration parameters for KMS decryption operations.
#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AwsKmsDecryptionConfig {
    /// The AWS key identifier of the KMS key used to decrypt data.
    pub key_id: String,

    /// The AWS region to send KMS decryption requests to.
    pub region: String,

    /// When true, omits key_id from decryption requests, allowing KMS to determine the key from ciphertext metadata.
    pub skip_key_id_on_decrypt: bool,
}

/// Client for AWS KMS operations.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    inner_client: Client,
    key_id: String,
    skip_key_id_on_decrypt: bool,
}

impl AwsKmsClient {
    async fn new_inner(key_id: String, region: String, skip_key_id_on_decrypt: bool) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(region));
        let sdk_config = aws_config::defaults(BehaviorVersion::v2024_03_28())
            .region(region_provider)
            .load()
            .await;

        Self {
            inner_client: Client::new(&sdk_config),
            key_id,
            skip_key_id_on_decrypt,
        }
    }

    pub async fn new_for_encryption(config: &AwsKmsEncryptionConfig) -> Self {
        Self::new_inner(config.key_id.clone(), config.region.clone(), false).await
    }

    pub async fn new_for_decryption(config: &AwsKmsDecryptionConfig) -> Self {
        Self::new_inner(
            config.key_id.clone(),
            config.region.clone(),
            config.skip_key_id_on_decrypt,
        )
        .await
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
}
