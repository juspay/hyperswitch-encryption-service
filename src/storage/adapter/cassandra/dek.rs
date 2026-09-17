use charybdis::{operations::Insert, options::Consistency};
use error_stack::ResultExt;

use super::DbState;
use crate::{
    env::observability as logger,
    errors::{self, CustomResult, DatabaseError, SwitchError},
    storage::{
        ReadView,
        adapter::Cassandra,
        dek::{DataKeyReadInterface, DataKeyStorageInterface},
        metrics,
        types::{CassandraDataKey, DataKey, DataKeyNew},
    },
    types::{Identifier, key::Version},
};

#[async_trait::async_trait]
impl DataKeyStorageInterface
    for DbState<scylla::client::caching_session::CachingSession, Cassandra>
{
    async fn get_or_insert_data_key(
        &self,
        _operation: metrics::DataKeyStorageOperation,
        new_key: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let connection = self.get_write_pool().await.switch()?;
        let new_row = CassandraDataKey::from(DataKey::from(new_key));

        let existing = self
            .get_key(
                new_row.version,
                &Identifier::try_from((
                    new_row.data_identifier.clone(),
                    new_row.key_identifier.clone(),
                ))
                .change_context(errors::DatabaseError::Others)?,
            )
            .await;

        match existing {
            Ok(existing_key) => Ok(existing_key),
            Err(err) => {
                if let DatabaseError::NotFound = err.current_context() {
                    logger::error!(database_err=?err);
                }

                new_row
                    .insert()
                    .consistency(Consistency::EachQuorum)
                    .execute(connection)
                    .await
                    .switch()?;
                Ok(DataKey::from(new_row))
            }
        }
    }

    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        let (data_id, key_id) = identifier.get_identifier();
        let connection = self.get_write_pool().await.switch()?;

        let data_key =
            CassandraDataKey::find_first_by_key_identifier_and_data_identifier(key_id, data_id)
                .consistency(scylla::statement::Consistency::LocalQuorum)
                .execute(connection)
                .await
                .switch()?;

        Ok(data_key.version)
    }

    async fn get_key(
        &self,
        key_version: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let (data_id, key_id) = identifier.get_identifier();
        let connection = self.get_write_pool().await.switch()?;

        let data_key = CassandraDataKey::find_by_key_identifier_and_data_identifier_and_version(
            key_id, data_id, key_version,
        )
        .consistency(scylla::statement::Consistency::LocalQuorum)
        .execute(connection)
        .await
        .switch()?;

        Ok(DataKey::from(data_key))
    }
}

// No replica concept for Cassandra: the read view just delegates to the primary.
#[async_trait::async_trait]
impl DataKeyReadInterface
    for ReadView<'_, DbState<scylla::client::caching_session::CachingSession, Cassandra>>
{
    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        self.state.get_latest_version(identifier).await
    }

    async fn get_key(
        &self,
        key_version: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        self.state.get_key(key_version, identifier).await
    }
}
