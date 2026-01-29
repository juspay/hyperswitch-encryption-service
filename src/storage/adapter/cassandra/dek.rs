use charybdis::{operations::Insert, options::Consistency};
use error_stack::ResultExt;

use super::DbState;
#[cfg(feature = "aws")]
use crate::storage::types::{ListKeyInfo, UpdateReEncryptedKey};
use crate::{
    crypto::Source,
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

        let data_key = DataKey::find_first_by_key_identifier_and_data_identifier(key_id, data_id)
            .consistency(scylla::statement::Consistency::LocalQuorum)
            .execute(connection)
            .await
            .switch()?;

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
            DataKey::find_by_key_identifier_and_data_identifier_and_version(key_id, data_id, v)
                .consistency(scylla::statement::Consistency::LocalQuorum)
                .execute(connection)
                .await
                .switch()?;

        Ok(data_key)
    }

    async fn get_keys_by_filter(
        &self,
        _key_source: Option<Source>,
    ) -> CustomResult<Vec<DataKey>, errors::DatabaseError> {
        Err(error_stack::report!(errors::DatabaseError::Others)
            .attach_printable("get_keys_by_filter is not supported for Cassandra"))
    }

    #[cfg(feature = "aws")]
    async fn get_keys_by_unique_index(
        &self,
        _key_infos: Option<&Vec<ListKeyInfo>>,
    ) -> CustomResult<Vec<DataKey>, errors::DatabaseError> {
        Err(error_stack::report!(errors::DatabaseError::Others)
            .attach_printable("get_keys_by_unique_index is not supported for Cassandra"))
    }

    #[cfg(feature = "aws")]
    async fn update_key(
        &self,
        _key: &UpdateReEncryptedKey,
    ) -> CustomResult<(), errors::DatabaseError> {
        Err(error_stack::report!(errors::DatabaseError::Others)
            .attach_printable("update_key is not supported for Cassandra"))
    }
}
