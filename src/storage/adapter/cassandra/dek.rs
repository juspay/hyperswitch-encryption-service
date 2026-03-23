use charybdis::{operations::Insert, options::Consistency};
use error_stack::ResultExt;

use super::DbState;
use crate::{
    env::observability as logger,
    errors::{self, CustomResult, DatabaseError, SwitchError},
    storage::{
        adapter::Cassandra,
        dek::DataKeyStorageInterface,
        types::{DataKey, DataKeyNew},
    },
    types::{Identifier, key::Version},
};

#[async_trait::async_trait]
impl DataKeyStorageInterface for DbState<scylla::CachingSession, Cassandra> {
    async fn get_or_insert_data_key(
        &self,
        new: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let connection = self.get_conn().await.switch()?;
        let key: DataKey = new.into();

        let find_query = self
            .get_key(
                key.version,
                &Identifier::try_from((key.data_identifier.clone(), key.key_identifier.clone()))
                    .change_context(errors::DatabaseError::Others)?,
            )
            .await;

        match find_query {
            Ok(key) => Ok(key),
            Err(err) => {
                if let DatabaseError::NotFound = err.current_context() {
                    logger::error!(database_err=?err);
                }

                key.insert()
                    .consistency(Consistency::EachQuorum)
                    .execute(connection)
                    .await
                    .map_err(|err| {
                        logger::error!(cassandra_insert_err=?err);
                        err
                    })
                    .switch()?;
                Ok(key)
            }
        }
    }

    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        let (data_id, key_id) = identifier.get_identifier();
        let connection = self.get_conn().await.switch()?;

        let data_key = DataKey::find_first_by_key_identifier_and_data_identifier(key_id.clone(), data_id.clone())
            .consistency(scylla::statement::Consistency::LocalQuorum)
            .execute(connection)
            .await
            .switch()
            .map_err(|err| {
                logger::error!(error=?err, data_identifier=%data_id, key_identifier=%key_id, "Failed to get latest key version from cassandra");
                err
            })?;

        Ok(data_key.version)
    }

    async fn get_key(
        &self,
        v: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let (data_id, key_id) = identifier.get_identifier();
        let connection = self.get_conn().await.switch()?;

        let data_key =
            DataKey::find_by_key_identifier_and_data_identifier_and_version(key_id.clone(), data_id.clone(), v)
                .consistency(scylla::statement::Consistency::LocalQuorum)
                .execute(connection)
                .await
                .switch()
                .map_err(|err| {
                    logger::error!(error=?err, %v, data_identifier=%data_id, key_identifier=%key_id, "Failed to get data key from cassandra");
                    err
                })?;

        Ok(data_key)
    }
}
