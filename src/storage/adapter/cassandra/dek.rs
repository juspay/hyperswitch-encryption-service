use super::DbState;

use crate::{
    errors::{self, CustomResult},
    storage::{
        adapter::Cassandra,
        dek::DataKeyStorageInterface,
        types::{DataKey, DataKeyNew},
    },
    types::{key::Version, Identifier},
};

#[async_trait::async_trait]
impl DataKeyStorageInterface
    for DbState<
        diesel_async::pooled_connection::bb8::Pool<diesel_async::AsyncPgConnection>,
        Cassandra,
    >
{
    async fn get_or_insert_data_key(
        &self,
        _new: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        Err(error_stack::report!(errors::DatabaseError::UniqueViolation))
    }

    async fn get_latest_version(
        &self,
        _identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        Err(error_stack::report!(errors::DatabaseError::UniqueViolation))
    }

    async fn get_key(
        &self,
        _v: Version,
        _identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        Err(error_stack::report!(errors::DatabaseError::UniqueViolation))
    }
}
