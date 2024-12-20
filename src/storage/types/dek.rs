use crate::{schema::data_key_store, types::key::Version};
use charybdis::macros::charybdis_model;
use diesel::{Identifiable, Insertable, Queryable};
use masking::StrongSecret;
use time::PrimitiveDateTime;

#[derive(Insertable)]
#[diesel(table_name = data_key_store)]
pub struct DataKeyNew {
    pub key_identifier: String,
    pub data_identifier: String,
    pub encryption_key: StrongSecret<Vec<u8>>,
    pub version: Version,
    pub created_at: PrimitiveDateTime,
    pub source: String,
    pub token: Option<StrongSecret<String>>,
}

#[charybdis_model(
    table_name = data_key_store,
    partition_keys = [key_identifier, data_identifier],
    clustering_keys = [version],
    table_options = r#"
          CLUSTERING ORDER BY (version DESC)
          AND gc_grace_seconds = 86400
      "#
)]
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
    pub token: Option<StrongSecret<String>>,
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
            token: value.token,
        }
    }
}
