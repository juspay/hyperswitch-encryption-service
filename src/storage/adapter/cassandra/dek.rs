use charybdis::operations::Insert;

use super::DbState;

use crate::{
    errors::{self, CustomResult, SwitchError},
    storage::{
        adapter::Cassandra,
        dek::DataKeyStorageInterface,
        types::{DataKey, DataKeyNew},
    },
    types::{key::Version, Identifier},
};

use charybdis::options::Consistency;
use error_stack::ResultExt;

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
            Err(_) => {
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
}
