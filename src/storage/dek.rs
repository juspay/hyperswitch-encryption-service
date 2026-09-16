use crate::{
    errors::{self, CustomResult},
    storage::{
        metrics,
        types::{DataKey, DataKeyNew},
    },
    types::{Identifier, key::Version},
};

/// Write access, plus reads that are pinned to the primary.
///
/// Implemented for `DbState`. Reads on this trait always hit the primary:
/// use them for read-before-write and read-your-own-write paths (duplicate
/// detection, version computation), where a lagging replica must not be able
/// to produce a stale answer. Config-routed reads live on
/// [`DataKeyReadInterface`] instead.
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

/// Config-routed reads.
///
/// Implemented for the `ReadDbPool` view obtained from
/// `DbState::read_db_pool` (or `TenantState::get_read_db_pool`): each read
/// follows the configured `read_strategy`, including the
/// replica-then-primary fallback.
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
