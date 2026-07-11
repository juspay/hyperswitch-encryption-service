use charybdis::macros::charybdis_model;
use diesel::{Identifiable, Insertable, Queryable};
use hyperswitch_masking::StrongSecret;
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::{schema::data_key_store, types::key::Version};

#[derive(Insertable)]
#[diesel(table_name = data_key_store)]
pub struct DataKeyNew {
    pub key_identifier: String,
    pub data_identifier: String,
    pub encryption_key: StrongSecret<Vec<u8>>,
    pub version: Version,
    pub created_at: PrimitiveDateTime,
    pub source: String,
}

#[derive(Queryable, Identifiable)]
#[diesel(table_name = data_key_store)]
pub struct DataKey {
    pub id: i32,
    pub key_identifier: String,
    pub data_identifier: String,
    pub encryption_key: StrongSecret<Vec<u8>>,
    pub version: Version,
    pub created_at: PrimitiveDateTime,
    pub source: String,
}

// Cassandra representation of `DataKey`.
//
// This uses `OffsetDateTime` because Scylla does not support the required
// traits for `PrimitiveDateTime`.
#[charybdis_model(
    table_name = data_key_store,
    partition_keys = [key_identifier, data_identifier],
    clustering_keys = [version],
    table_options = r#"
          CLUSTERING ORDER BY (version DESC)
          AND gc_grace_seconds = 86400
      "#
)]
pub struct CassandraDataKey {
    pub id: i32,
    pub key_identifier: String,
    pub data_identifier: String,
    pub encryption_key: StrongSecret<Vec<u8>>,
    pub version: Version,
    pub created_at: OffsetDateTime,
    pub source: String,
}

impl From<CassandraDataKey> for DataKey {
    fn from(value: CassandraDataKey) -> Self {
        let utc_created_at = value.created_at.to_utc();
        Self {
            id: value.id,
            key_identifier: value.key_identifier,
            data_identifier: value.data_identifier,
            encryption_key: value.encryption_key,
            version: value.version,
            created_at: PrimitiveDateTime::new(utc_created_at.date(), utc_created_at.time()),
            source: value.source,
        }
    }
}

impl From<DataKey> for CassandraDataKey {
    fn from(value: DataKey) -> Self {
        Self {
            id: value.id,
            key_identifier: value.key_identifier,
            data_identifier: value.data_identifier,
            encryption_key: value.encryption_key,
            version: value.version,
            created_at: value.created_at.assume_utc(),
            source: value.source,
        }
    }
}

impl From<DataKeyNew> for DataKey {
    fn from(value: DataKeyNew) -> Self {
        Self {
            id: 0,
            key_identifier: value.key_identifier,
            data_identifier: value.data_identifier,
            encryption_key: value.encryption_key,
            version: value.version,
            created_at: value.created_at,
            source: value.source,
        }
    }
}
