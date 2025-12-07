use crate::{
    errors::{self, CustomResult},
    storage::types::{DataKey, DataKeyNew},
    types::{Identifier, key::Version},
};

#[async_trait::async_trait]
pub trait DataKeyStorageInterface {
    async fn get_or_insert_data_key(
        &self,
        new: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError>;
    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError>;
    async fn get_key(
        &self,
        v: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError>;
}
