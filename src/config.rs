use crate::crypto::aes256::GcmAes256;
use config::File;
use router_env::config::Log;
use serde::Deserialize;

#[cfg(feature = "aws")]
use crate::services::aws::AwsKmsConfig;

use std::path::PathBuf;

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
}
#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: Server,
    pub database: Database,
    pub log: Log,
    pub secrets: Secrets,
    #[cfg(feature = "aws")]
    pub kms_config: AwsKmsConfig,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub port: u16,
    pub host: String,
    pub user: masking::Secret<String>,
    pub password: masking::Secret<String>,
    pub dbname: masking::Secret<String>,
    pub pool_size: Option<u32>,
    pub min_idle: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct Secrets {
    pub master_key: GcmAes256,
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

    pub fn with_config_path(environment: Environment, config_path: Option<PathBuf>) -> Self {
        let config = config::Config::builder()
            .add_source(File::from(Self::config_path(environment, config_path)))
            .build()
            .expect("Unable to find configuration");

        serde_path_to_error::deserialize(config)
            .map_err(|error| {
                eprintln!("Unable to deserialize application configuration: {error}");
            })
            .unwrap()
    }
}
