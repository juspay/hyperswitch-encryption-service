use crate::{schema::data_key_store, types::key::Version};
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
}
