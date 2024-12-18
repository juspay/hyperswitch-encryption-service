use crate::crypto::aes256::GcmAes256;
use crate::{
    crypto::KeyManagerClient,
    env::observability::LogConfig,
    errors::{self, CustomResult},
};
use config::File;
use serde::Deserialize;
use std::sync::Arc;

use crate::services::aws::{AwsKmsClient, AwsKmsConfig};

use aws_sdk_kms::primitives::Blob;

use masking::PeekInterface;

use crate::crypto::vault::{Vault, VaultSettings};

use vaultrs::{
    client::{VaultClient, VaultClientSettingsBuilder},
    transit,
};

use std::path::PathBuf;

pub mod vars {
    pub const RUN_ENV: &str = "RUN_ENV";
}

#[derive(Copy, Clone, strum::Display, strum::EnumString)]
pub enum Environment {
    Dev,
    Production,
}

impl Environment {
    fn config_path(&self) -> &str {
        match self {
            Self::Production => "production.toml",
            Self::Dev => "development.toml",
        }
    }
    pub fn which() -> Self {
        #[cfg(debug_assertions)]
        let default_env = Self::Dev;
        #[cfg(not(debug_assertions))]
        let default_env = Self::Production;

        std::env::var(vars::RUN_ENV)
            .map_or_else(|_| default_env, |v| v.parse().unwrap_or(default_env))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SecretContainer(masking::Secret<String>);

impl SecretContainer {
    /// # Panics
    ///
    /// Panics when secret cannot be decrypted with KMS
    #[allow(clippy::expect_used, unused_variables)]
    pub async fn expose(&self, config: &Config) -> masking::Secret<String> {
        if cfg!(feature = "aws") {
            use base64::Engine;

            let kms = AwsKmsClient::new(&config.secrets.kms_config).await;
            let data = crate::consts::base64::BASE64_ENGINE
                .decode(self.0.peek())
                .expect("Unable to base64 decode secret");

            let plaintext_blob = Blob::new(data);
            let decrypted_output = kms
                .inner_client()
                .decrypt()
                .key_id(kms.key_id())
                .ciphertext_blob(plaintext_blob)
                .send()
                .await
                .expect("Unable to decrypt KMS encrypted secret")
                .plaintext
                .expect("Plaintext secret is empty")
                .into_inner();

            let secret = String::from_utf8(decrypted_output).expect("Invalid secret");
            masking::Secret::new(secret)
        } else if cfg!(feature = "vault") {
            use base64::Engine;

            let client = VaultClient::new(
                VaultClientSettingsBuilder::default()
                    .address(&config.secrets.vault_config.url)
                    .token(config.secrets.vault_config.vault_token.peek())
                    .build()
                    .expect("Unable to build HashiCorp Vault Settings"),
            )
            .expect("Unable to build HashiCorp Vault client");

            let cypher_text = self.0.peek();

            let b64_encoded_str = transit::data::decrypt(
                &client,
                &config.secrets.vault_config.mount_point,
                &config.secrets.vault_config.encryption_key,
                cypher_text,
                None,
            )
            .await
            .expect("Failed while decrypting vault encrypted secret")
            .plaintext;

            return masking::Secret::new(
                String::from_utf8(
                    crate::consts::base64::BASE64_ENGINE
                        .decode(b64_encoded_str)
                        .expect("Failed to base64 decode the vault data"),
                )
                .expect("Invalid secret"),
            );
        } else {
            self.0.clone()
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PoolConfig {
    pub pool: usize,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: Server,
    pub metrics_server: Server,
    pub database: Database,
    pub secrets: Secrets,
    pub log: LogConfig,
    pub pool_config: PoolConfig,
    #[cfg(feature = "mtls")]
    pub certs: Certs,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub port: u16,
    pub host: String,
    pub user: masking::Secret<String>,
    pub password: SecretContainer,
    pub dbname: masking::Secret<String>,
    pub pool_size: Option<u32>,
    pub min_idle: Option<u32>,
    pub enable_ssl: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Certs {
    pub tls_cert: SecretContainer,
    pub tls_key: SecretContainer,
    pub root_ca: SecretContainer,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Secrets {
    #[serde(default)]
    pub master_key: GcmAes256,
    #[serde(default)]
    pub kms_config: AwsKmsConfig,
    #[serde(default)]
    pub vault_config: VaultSettings,
    pub access_token: masking::Secret<String>,
    pub hash_context: masking::Secret<String>,
}

#[derive(Deserialize, Debug)]
pub struct Server {
    pub port: u16,
    pub host: String,
}

impl Secrets {
    fn validate(&self) -> CustomResult<(), errors::ParsingError> {
        if cfg!(feature = "aws") && (self.kms_config == AwsKmsConfig::default()) {
            Err(error_stack::report!(errors::ParsingError::DecodingFailed(
                "AWS config is not provided".to_string()
            )))
        } else if cfg!(feature = "vault") && (self.vault_config == VaultSettings::default()) {
            Err(error_stack::report!(errors::ParsingError::DecodingFailed(
                "Vault config is not provided".to_string()
            )))
        } else {
            Ok(())
        }
    }
}
impl Config {
    pub fn config_path(environment: Environment, explicit_config_path: Option<PathBuf>) -> PathBuf {
        let mut config_path = PathBuf::new();
        if let Some(explicit_config_path_val) = explicit_config_path {
            config_path.push(explicit_config_path_val);
        } else {
            let config_directory =
                std::env::var(crate::consts::CONFIG_DIR).unwrap_or_else(|_| "config".into());

            config_path.push(config_directory);
            config_path.push(environment.config_path());
        }
        config_path
    }

    /// # Panics
    ///
    /// Panics for an invalid configuration
    #[allow(clippy::panic, clippy::expect_used)]
    pub fn with_config_path(environment: Environment, config_path: Option<PathBuf>) -> Self {
        let config = config::Config::builder()
            .add_source(File::from(Self::config_path(environment, config_path)).required(false))
            .add_source(
                config::Environment::with_prefix("CRIPTA")
                    .try_parsing(true)
                    .separator("__"),
            )
            .build()
            .expect("Unable to find configuration");

        serde_path_to_error::deserialize(config)
            .expect("Unable to deserialize application configuration")
    }
    /// # Panics
    ///
    /// Panics for a validation fail
    #[allow(clippy::panic, clippy::expect_used)]
    pub fn validate(&self) {
        self.secrets
            .validate()
            .expect("Failed to valdiate secrets some missing configuration found")
    }
}

impl Secrets {
    pub async fn create_keymanager_client(self) -> KeyManagerClient {
        if cfg!(feature = "aws") {
            let client = AwsKmsClient::new(&self.kms_config).await;
            KeyManagerClient::new(Arc::new(client))
        } else if cfg!(feature = "vault") {
            let client = Vault::new(self.vault_config);
            KeyManagerClient::new(Arc::new(client))
        } else {
            let client = self.master_key;
            KeyManagerClient::new(Arc::new(client))
        }
    }
}
