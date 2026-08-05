use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_kms::{Client, config::Region};

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

    /// Optional AWS region used to decrypt DEKs that were encrypted by a KMS key in a different region.
    pub decrypt_region: Option<String>,
}

/// Client for AWS KMS operations.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    inner_client: Client,
    key_id: String,
    skip_key_id_on_decrypt: bool,
    decrypt_region: Option<String>,
}

impl AwsKmsClient {
    pub async fn new(config: &AwsKmsConfig) -> Self {
        Self {
            inner_client: Self::build_client(&config.region).await,
            key_id: config.key_id.clone(),
            skip_key_id_on_decrypt: config.skip_key_id_on_decrypt,
            decrypt_region: config.decrypt_region.clone(),
        }
    }

    async fn build_client(region: &str) -> Client {
        let region_provider = RegionProviderChain::first_try(Region::new(region.to_owned()));
        let sdk_config = aws_config::defaults(BehaviorVersion::v2024_03_28())
            .region(region_provider)
            .load()
            .await;

        Client::new(&sdk_config)
    }

    /// Builds a KMS client for the given region.
    pub async fn client_for_region(region: &str) -> Client {
        Self::build_client(region).await
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

    pub fn decrypt_region(&self) -> Option<&str> {
        self.decrypt_region.as_deref()
    }
}
