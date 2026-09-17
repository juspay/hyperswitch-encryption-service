use crate::{
    errors::{self, CustomResult},
    storage::{
        metrics,
        types::{DataKey, DataKeyNew},
    },
    types::{Identifier, key::Version},
};

/// Write access, plus reads pinned to the primary (read-before-write and read-your-own-write paths). Config-routed reads live on [`DataKeyReadInterface`] instead.
#[async_trait::async_trait]
pub trait DataKeyStorageInterface {
    async fn get_or_insert_data_key(
        &self,
        operation: metrics::DataKeyStorageOperation,
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

/// Config-routed reads following the configured `read_strategy`. Implemented for `ReadDbPool`.
#[async_trait::async_trait]
pub trait DataKeyReadInterface {
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
