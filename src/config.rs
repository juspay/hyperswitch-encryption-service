use crate::{
    crypto::{EncryptionClient, KeyManagerClient},
    env::observability::LogConfig,
};
use config::File;
use serde::Deserialize;

#[cfg(feature = "aes")]
use crate::crypto::aes256::GcmAes256;

#[cfg(feature = "aws")]
use crate::services::aws::{AwsKmsClient, AwsKmsConfig};

#[cfg(feature = "aws")]
use aws_sdk_kms::primitives::Blob;

#[cfg(feature = "aws")]
use masking::PeekInterface;

#[cfg(feature = "vault")]
use crate::crypto::vault::{Vault, VaultSettings};

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
    // TODO: Create AWS Client for once.
    #[allow(clippy::expect_used, unused_variables)]
    pub async fn expose(&self, config: &Config) -> masking::Secret<String> {
        #[cfg(feature = "aws")]
        {
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
        }

        #[cfg(feature = "aes")]
        {
            self.0.clone()
        }

        #[cfg(feature = "vault")]
        // TODO: Temp fix to connect to db
        {
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct Certs {
    pub tls_cert: SecretContainer,
    pub tls_key: SecretContainer,
    pub root_ca: SecretContainer,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Secrets {
    #[cfg(feature = "aes")]
    pub master_key: GcmAes256,
    #[cfg(feature = "aws")]
    pub kms_config: AwsKmsConfig,
    // TODO: Add Vault's initialized object
    #[cfg(feature = "vault")]
    pub vault_config: VaultSettings,
    #[cfg(feature = "vault")]
    pub vault_token: masking::Secret<String>,

    pub access_token: masking::Secret<String>,
    pub hash_context: masking::Secret<String>,
}

#[derive(Deserialize, Debug)]
pub struct Server {
    pub port: u16,
    pub host: String,
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
}

impl Secrets {
    pub async fn create_keymanager_client(self) -> KeyManagerClient {
        #[cfg(feature = "aws")]
        {
            let client = AwsKmsClient::new(&self.kms_config).await;
            EncryptionClient::new(client)
        }

        #[cfg(feature = "aes")]
        {
            let client = self.master_key;
            EncryptionClient::new(client)
        }

        #[cfg(feature = "vault")]
        {
            let client = Vault::new(self.vault_config, self.vault_token);
            EncryptionClient::new(client)
        }
    }
}
