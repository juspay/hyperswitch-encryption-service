use crate::{
    app::AppState,
    core::KeyDecrypt,
    crypto::Source,
    errors::{self, SwitchError},
    storage::{cache, dek::DataKeyStorageInterface},
    types::Identifier,
};
use core::fmt::Display;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    expression::AsExpression,
    serialize::ToSql,
    sql_types, Queryable,
};

use core::fmt;

use masking::StrongSecret;
use masking::{Deserialize, Serialize};
use serde::de::{self, Deserializer, Unexpected, Visitor};

#[derive(Clone)]
pub struct Key {
    pub identifier: Identifier,
    pub key: StrongSecret<[u8; 32]>,
    pub version: Version,
    pub source: Source,
}

impl Key {
    pub async fn get_key(
        state: &AppState,
        identifier: &Identifier,
        version: Version,
    ) -> errors::CustomResult<Self, errors::DatabaseError> {
        let db = &state.db_pool;
        let get_and_decrypt_key = || async {
            let key = db.get_key(version, identifier).await?;
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

#[derive(AsExpression, Eq, PartialEq, Debug, Clone, Copy)]
#[diesel(sql_type = diesel::sql_types::Integer)]
pub struct Version(i32);

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("v{}", self.0))
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EncryptedDataVisitor;

        impl<'de> Visitor<'de> for EncryptedDataVisitor {
            type Value = Version;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string of the format {version}:{base64_encoded_data}'")
            }

            fn visit_str<E>(self, value: &str) -> Result<Version, E>
            where
                E: de::Error,
            {
                let (_, version) = value.split_once('v').ok_or_else(|| {
                    E::invalid_value(
                        Unexpected::Str(value),
                        &"Version should be in the format of v{version_num}",
                    )
                })?;

                let version = version.parse::<i32>().map_err(|_| {
                    E::invalid_value(Unexpected::Str(version), &"Unexpted version number")
                })?;

                Ok(Version::from(version))
            }
        }

        deserializer.deserialize_str(EncryptedDataVisitor)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
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

        v.unwrap_or_default()
    }

    pub fn increment(self) -> errors::CustomResult<Self, errors::ParsingError> {
        Ok(Self(self.0 + 1))
    }
    pub fn inner(self) -> i32 {
        self.0
    }
}

impl Default for Version {
    fn default() -> Self {
        Self(1)
    }
}

impl From<i32> for Version {
    fn from(v: i32) -> Self {
        Self(v)
    }
}

impl<DB> FromSql<sql_types::Integer, DB> for Version
where
    DB: Backend,
    i32: FromSql<sql_types::Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        i32::from_sql(bytes).map(Self::from)
    }
}

impl<DB> ToSql<sql_types::Integer, DB> for Version
where
    DB: Backend,
    i32: ToSql<sql_types::Integer, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> Queryable<sql_types::Integer, DB> for Version
where
    DB: Backend,
    i32: FromSql<sql_types::Integer, DB>,
{
    type Row = i32;
    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(Self::from(row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::panic, clippy::expect_used)]
    #[test]
    fn test_version_deserialize() {
        #[derive(Debug, serde::Serialize, Deserialize, PartialEq, Eq)]
        struct Data {
            version: Version,
        }

        let version = serde_json::json!({
            "version": "v1"
        });

        let actual: Data = serde_json::from_value(version).expect("Failed to deserialize version");

        let expected = Data {
            version: Version(1),
        };

        assert_eq!(actual, expected)
    }
}
