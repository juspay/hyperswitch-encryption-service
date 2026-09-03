use base64::Engine;
use error_stack::ResultExt;
use google_cloud_kms::{
    client::{Client, ClientConfig},
    grpc::kms::v1::DecryptRequest,
};

use crate::errors::{self, CustomResult};

/// Configuration parameters required for constructing a [`GcpKmsClient`].
#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct GcpKmsConfig {
    /// The GCP project ID that owns the KMS key ring.
    pub project_id: String,

    /// The location ID (e.g. `"global"`, `"us-east1"`) of the KMS key ring.
    pub location_id: String,

    /// The ID of the KMS key ring.
    pub key_ring_id: String,

    /// The ID of the KMS key used to encrypt or decrypt data.
    pub key_id: String,
}

impl GcpKmsConfig {
    /// All four fields are required to build the full KMS resource path.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.project_id.trim().is_empty() {
            return Err("GCP KMS project ID must not be empty");
        }
        if self.location_id.trim().is_empty() {
            return Err("GCP KMS location ID must not be empty");
        }
        if self.key_ring_id.trim().is_empty() {
            return Err("GCP KMS key ring ID must not be empty");
        }
        if self.key_id.trim().is_empty() {
            return Err("GCP KMS key ID must not be empty");
        }
        Ok(())
    }
}

/// Client for GCP Cloud KMS operations.
#[derive(Clone)]
pub struct GcpKmsClient {
    inner_client: Client,
    key_name: String,
    location: String,
}

impl std::fmt::Debug for GcpKmsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcpKmsClient")
            .field("key_name", &self.key_name)
            .field("location", &self.location)
            .finish()
    }
}

impl GcpKmsClient {
    /// Authenticates via Application Default Credentials.
    pub async fn new(config: &GcpKmsConfig) -> CustomResult<Self, errors::CryptoError> {
        let client_config = ClientConfig::default()
            .with_auth()
            .await
            .change_context(errors::CryptoError::ClientCreationFailed)?;
        let inner_client = Client::new(client_config)
            .await
            .change_context(errors::CryptoError::ClientCreationFailed)?;

        Ok(Self {
            inner_client,
            key_name: format!(
                "projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}",
                config.project_id, config.location_id, config.key_ring_id, config.key_id
            ),
            location: format!(
                "projects/{}/locations/{}",
                config.project_id, config.location_id
            ),
        })
    }

    pub fn inner_client(&self) -> &Client {
        &self.inner_client
    }

    pub fn key_name(&self) -> &str {
        &self.key_name
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    /// Decrypts base64-encoded `data` via GCP Cloud KMS. Used for bootstrap secrets read
    /// from TOML config.
    pub async fn decrypt_secret(
        &self,
        data: impl AsRef<[u8]>,
    ) -> CustomResult<String, errors::CryptoError> {
        let ciphertext = crate::consts::base64::BASE64_ENGINE
            .decode(data)
            .change_context(errors::CryptoError::ParseError(
                "Failed to base64 decode GCP KMS ciphertext".to_string(),
            ))?;

        let request = DecryptRequest {
            name: self.key_name.clone(),
            ciphertext,
            additional_authenticated_data: Vec::new(),
            ciphertext_crc32c: None,
            additional_authenticated_data_crc32c: None,
        };

        let plaintext = self
            .inner_client
            .decrypt(request, None)
            .await
            .change_context(errors::CryptoError::DecryptionFailed("GCP KMS"))?
            .plaintext;

        String::from_utf8(plaintext).change_context(errors::CryptoError::ParseError(
            "Invalid GCP KMS decrypted secret".to_string(),
        ))
    }
}
