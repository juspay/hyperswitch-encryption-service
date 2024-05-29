use crate::schema::data_key_store;
use diesel::{Identifiable, Insertable, Queryable};
use masking::StrongSecret;
use time::PrimitiveDateTime;

#[derive(Insertable)]
#[diesel(table_name = data_key_store)]
pub struct DataKeyNew {
    pub key_identifier: String,
    pub data_identifier: String,
    pub encryption_key: StrongSecret<Vec<u8>>,
    pub version: String,
    pub created_at: PrimitiveDateTime,
}

#[derive(Queryable, Identifiable)]
#[diesel(table_name = data_key_store)]
pub struct DataKey {
    pub id: i32,
    pub key_identifier: String,
    pub data_identifier: String,
    pub encryption_key: StrongSecret<Vec<u8>>,
    pub version: String,
    pub created_at: PrimitiveDateTime,
}
