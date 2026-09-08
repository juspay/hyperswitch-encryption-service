use std::{
    num::{NonZeroU64, NonZeroUsize},
    path::PathBuf,
    sync::Arc,
};

use config::File;
#[cfg(any(feature = "aws", feature = "gcp", feature = "vault"))]
use hyperswitch_masking::PeekInterface;
use rustc_hash::FxHashMap;
use serde::Deserialize;

#[cfg(not(feature = "release"))]
use crate::crypto::aes256::AesLocalConfig;
#[cfg(feature = "vault")]
use crate::crypto::vault::{Vault, VaultSettings};
#[cfg(feature = "aws")]
use crate::services::aws::{AwsKmsClient, AwsKmsConfig};
#[cfg(feature = "gcp")]
use crate::services::gcp::{GcpKmsClient, GcpKmsConfig};
use crate::{
    crypto::KeyManagerClient,
    env::observability::LogConfig,
    errors::{self, CustomResult},
};

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
pub struct SecretContainer(hyperswitch_masking::Secret<String>);

impl SecretContainer {
    /// # Panics
    ///
    /// Panics when secret cannot be decrypted with the configured backend.
    #[allow(clippy::expect_used, unused_variables)]
    pub async fn expose(&self, config: &Config) -> hyperswitch_masking::Secret<String> {
        match &config.secrets {
            #[cfg(feature = "aws")]
            Secrets::AwsKms { aws_kms } => {
                let secret = AwsKmsClient::new(aws_kms)
                    .await
                    .decrypt_secret(self.0.peek())
                    .await
                    .expect("Unable to decrypt AWS KMS encrypted secret");
                hyperswitch_masking::Secret::new(secret)
            }
            #[cfg(feature = "gcp")]
            Secrets::GcpKms { gcp_kms } => {
                let secret = GcpKmsClient::new(gcp_kms)
                    .await
                    .expect("Unable to build GCP KMS client")
                    .decrypt_secret(self.0.peek())
                    .await
                    .expect("Unable to decrypt GCP KMS encrypted secret");
                hyperswitch_masking::Secret::new(secret)
            }
            #[cfg(feature = "vault")]
            Secrets::HashicorpVault { hashicorp_vault } => {
                let secret = Vault::new(hashicorp_vault.clone())
                    .decrypt_secret(self.0.peek())
                    .await
                    .expect("Unable to decrypt HashiCorp Vault encrypted secret");
                hyperswitch_masking::Secret::new(secret)
            }
            #[cfg(not(feature = "release"))]
            Secrets::AesLocal { .. } => self.0.clone(),
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
    #[serde(default)]
    pub management_server: ManagementServer,
    pub database: Database,
    pub secrets: Secrets,
    #[serde(default)]
    pub cassandra: Cassandra,
    pub log: LogConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    pub multitenancy: MultiTenancy,
    pub pool_config: PoolConfig,
    #[cfg(feature = "mtls")]
    pub certs: Certs,
}

#[derive(Deserialize, Debug)]
pub struct MultiTenancy {
    pub tenants: TenantsConfig,
}

#[derive(Deserialize, Debug)]
pub struct TenantsConfig(pub FxHashMap<String, TenantConfig>);

#[derive(Deserialize, Debug)]
pub struct TenantConfig {
    pub schema: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct Cassandra {
    pub known_nodes: Vec<String>,
    pub timeout: u32,
    pub pool_size: NonZeroUsize,
    pub cache_size: usize,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub port: u16,
    pub host: String,
    pub user: hyperswitch_masking::Secret<String>,
    pub password: SecretContainer,
    pub dbname: hyperswitch_masking::Secret<String>,
    pub pool_size: Option<u32>,
    pub min_idle: Option<u32>,
    pub enable_ssl: Option<bool>,
    pub root_ca: Option<SecretContainer>,
    pub max_lifetime_secs: Option<NonZeroU64>,
    pub idle_timeout_secs: Option<NonZeroU64>,
    pub connection_acquire_timeout_secs: Option<NonZeroU64>,
    pub connect_timeout_secs: Option<NonZeroU64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Certs {
    pub tls_cert: SecretContainer,
    pub tls_key: SecretContainer,
    pub root_ca: SecretContainer,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "manager", rename_all = "snake_case")]
pub enum Secrets {
    #[cfg(feature = "aws")]
    AwsKms { aws_kms: AwsKmsConfig },
    #[cfg(feature = "gcp")]
    GcpKms { gcp_kms: GcpKmsConfig },
    #[cfg(feature = "vault")]
    HashicorpVault { hashicorp_vault: VaultSettings },
    #[cfg(not(feature = "release"))]
    AesLocal { aes_local: AesLocalConfig },
}

#[derive(Deserialize, Debug)]
pub struct Server {
    pub port: u16,
    pub host: String,
    #[serde(default = "default_tcp_nodelay")]
    pub set_tcp_nodelay: bool,
}

const fn default_tcp_nodelay() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct ManagementServer {
    pub host: String,
    pub port: u16,
}

impl Default for ManagementServer {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6128,
        }
    }
}

impl ManagementServer {
    pub fn validate(&self) -> CustomResult<(), errors::ParsingError> {
        if self.host.parse::<std::net::IpAddr>().is_err() {
            return Err(error_stack::Report::new(
                errors::ParsingError::DecodingFailed(
                    r#"management_server.host must be a valid IP address"#.into(),
                ),
            ));
        }

        if self.port == 0 {
            return Err(error_stack::Report::new(
                errors::ParsingError::DecodingFailed(
                    r#"management_server.port must be a non-zero value"#.into(),
                ),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum MetricsConfig {
    #[default]
    Disabled,

    Otlp {
        endpoint: String,
        #[serde(default = "default_endpoint_timeout")]
        endpoint_timeout_secs: u64,
        #[serde(default = "default_export_interval")]
        metrics_export_interval_secs: u64,
    },

    Prometheus,
}

const fn default_endpoint_timeout() -> u64 {
    10
}

const fn default_export_interval() -> u64 {
    15
}

impl MetricsConfig {
    pub fn validate(&self) -> CustomResult<(), errors::ParsingError> {
        match self {
            Self::Disabled => Ok(()),
            Self::Otlp { endpoint, .. } => {
                if endpoint.trim().is_empty() {
                    return Err(error_stack::Report::new(
                        errors::ParsingError::DecodingFailed(
                            r#"metrics.endpoint is required when mode is "otlp""#.into(),
                        ),
                    ));
                }
                Ok(())
            }
            Self::Prometheus => Ok(()),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(default)]
pub struct CacheConfig {
    pub time_to_live_secs: u64,
    pub time_to_idle_secs: u64,
    pub max_capacity: std::num::NonZeroU64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            time_to_live_secs: 30,
            time_to_idle_secs: 30,
            #[allow(clippy::expect_used)]
            max_capacity: std::num::NonZeroU64::new(10_000).expect("10_000 is non-zero"),
        }
    }
}

impl Secrets {
    fn validate(&self) -> CustomResult<(), errors::ParsingError> {
        match self {
            #[cfg(feature = "aws")]
            Self::AwsKms { aws_kms } => aws_kms.validate().map_err(|message| {
                error_stack::Report::new(errors::ParsingError::DecodingFailed(message.to_string()))
            }),
            #[cfg(feature = "gcp")]
            Self::GcpKms { gcp_kms } => gcp_kms.validate().map_err(|message| {
                error_stack::Report::new(errors::ParsingError::DecodingFailed(message.to_string()))
            }),
            #[cfg(feature = "vault")]
            Self::HashicorpVault { hashicorp_vault } => {
                hashicorp_vault.validate().map_err(|message| {
                    error_stack::Report::new(errors::ParsingError::DecodingFailed(
                        message.to_string(),
                    ))
                })
            }
            #[cfg(not(feature = "release"))]
            Self::AesLocal { .. } => Ok(()),
        }
    }
}

/// # Panics
///
/// Panics if the provided pool_size is not a non zero number
#[allow(clippy::expect_used)]
impl Default for Cassandra {
    fn default() -> Self {
        Self {
            known_nodes: Vec::new(),
            timeout: 0,
            cache_size: 0,
            pool_size: NonZeroUsize::new(1).expect("The provided number is non zero"),
        }
    }
}

impl Cassandra {
    fn validate(&self) -> CustomResult<(), errors::ParsingError> {
        if cfg!(feature = "cassandra") {
            error_stack::ensure!(
                !self.eq(&Self::default()),
                errors::ParsingError::DecodingFailed(
                    "Failed to validate Cassandra configuration, missing configuration found"
                        .to_string()
                )
            )
        }

        Ok(())
    }
}

impl MultiTenancy {
    fn validate(&self) -> CustomResult<(), errors::ParsingError> {
        error_stack::ensure!(
            !self.tenants.0.is_empty(),
            errors::ParsingError::DecodingFailed("Failed to validate multitenancy configuration. You need to configure atleast one tenant".to_string()
         )
       );
        Ok(())
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
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("cassandra.known_nodes"),
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
        self.management_server
            .validate()
            .expect("Failed to validate management server configuration");

        self.secrets
            .validate()
            .expect("Failed to valdiate secrets some missing configuration found");

        self.metrics
            .validate()
            .expect("Failed to validate metrics configuration");

        self.cassandra
            .validate()
            .expect("Failed to valdiate cassandra some missing configuration found");

        self.multitenancy
            .validate()
            .expect("Failed to validate multitenancy, some missing configuration found");
    }
}

impl Secrets {
    pub async fn create_keymanager_client(
        self,
    ) -> CustomResult<KeyManagerClient, errors::CryptoError> {
        Ok(match self {
            #[cfg(feature = "aws")]
            Self::AwsKms { aws_kms } => {
                let client = AwsKmsClient::new(&aws_kms).await;
                KeyManagerClient::new(Arc::new(client))
            }
            #[cfg(feature = "gcp")]
            Self::GcpKms { gcp_kms } => {
                let client = GcpKmsClient::new(&gcp_kms).await?;
                KeyManagerClient::new(Arc::new(client))
            }
            #[cfg(feature = "vault")]
            Self::HashicorpVault { hashicorp_vault } => {
                let client = Vault::new(hashicorp_vault);
                KeyManagerClient::new(Arc::new(client))
            }
            #[cfg(not(feature = "release"))]
            Self::AesLocal { aes_local } => KeyManagerClient::new(Arc::new(aes_local.master_key)),
        })
    }
}
