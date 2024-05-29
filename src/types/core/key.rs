use crate::{
    app::AppState,
    core::KeyDecrypt,
    errors::{self, SwitchError},
    storage::{cache, dek::DataKeyStorageInterface},
    types::Identifier,
};
use core::fmt::Display;
use masking::StrongSecret;
use masking::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Key {
    pub identifier: Identifier,
    pub key: StrongSecret<[u8; 32]>,
    pub version: Version,
}

impl Key {
    pub async fn get_key(
        state: &AppState,
        identifier: &Identifier,
        version: Version,
    ) -> errors::CustomResult<Self, errors::DatabaseError> {
        let db = &state.db_pool;
        let get_and_decrypt_key = || async {
            let key = db.get_key(version.to_string(), identifier).await?;
            key.decrypt(state).await.switch()
        };

        cache::get_or_populate_cache(
            format!("key_{}:{}", identifier, version),
            &cache::KEY_CACHE,
            get_and_decrypt_key(),
        )
        .await
    }
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Version(String);

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Version {
    pub async fn get_latest(identifier: &Identifier, state: &AppState) -> Self {
        let db = &state.db_pool;
        let latest_version = db.get_latest_version(identifier);
        let v = cache::get_or_populate_cache(
            format!("latest_version_{}", identifier),
            &cache::VERSION_CACHE,
            latest_version,
        )
        .await;

        match v {
            Ok(v) => Version(v),
            Err(_) => Version::default(),
        }
    }

    pub fn increment(self) -> errors::CustomResult<Self, errors::ParsingError> {
        let (_, num) = self.0.split_once('v').ok_or_else(|| {
            error_stack::report!(errors::ParsingError::ParsingFailed(
                "Version parsing failed".to_string()
            ))
        })?;

        let num: u32 = num.parse().switch()?;
        Ok(Version(format!("v{}", num + 1)))
    }
    pub fn inner(self) -> String {
        self.0
    }
}

impl Default for Version {
    fn default() -> Self {
        Self("v1".to_string())
    }
}

impl From<String> for Version {
    fn from(v: String) -> Self {
        Self(v)
    }
}
