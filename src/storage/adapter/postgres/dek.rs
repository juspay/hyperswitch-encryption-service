use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, associations::HasTable};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::Pool};
use error_stack::ResultExt;

use super::DbState;
use crate::{
    env::observability as logger,
    errors::{self, CustomResult, SwitchError},
    schema::data_key_store::*,
    storage::{
        adapter::PostgreSQL,
        dek::DataKeyStorageInterface,
        types::{DataKey, DataKeyNew},
    },
    types::{Identifier, key::Version},
};

#[async_trait::async_trait]
impl DataKeyStorageInterface for DbState<Pool<AsyncPgConnection>, PostgreSQL> {
    async fn get_or_insert_data_key(
        &self,
        new: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let identifier: errors::CustomResult<Identifier, errors::ParsingError> =
            (new.data_identifier.clone(), new.key_identifier.clone()).try_into();

        let v = new.version;

        let mut connection = self.get_conn().await.switch()?;
        let query = diesel::insert_into(DataKey::table()).values(new);

        match query.get_result(&mut connection).await.switch() {
            Ok(result) => Ok(result),
            Err(err) => match err.current_context() {
                errors::DatabaseError::UniqueViolation => {
                    logger::warn!(
                        "Data key already exists, fetching existing key"
                    );
                    self.get_key(
                        v,
                        &identifier
                            .change_context(errors::DatabaseError::Others)
                            .attach_printable("Failed to parse identifier")?,
                    )
                    .await
                }
                _ => {
                    logger::error!(error=?err, "Failed to insert data key into database");
                    Err(err)
                }
            },
        }
    }

    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;

        let (d_id, k_id) = identifier.get_identifier();
        let query = DataKey::table()
            .select(version)
            .order_by(version.desc())
            .filter(data_identifier.eq(d_id.clone()).and(key_identifier.eq(k_id.clone())));

        query.get_result(&mut connection).await.switch().map_err(|err| {
            logger::error!(error=?err, data_identifier=%d_id, key_identifier=%k_id, "Failed to get latest key version from database");
            err
        })
    }

    async fn get_key(
        &self,
        v: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;

        let (d_id, k_id) = identifier.get_identifier();

        let query = DataKey::table().filter(
            version
                .eq(v)
                .and(data_identifier.eq(d_id.clone()).and(key_identifier.eq(k_id.clone()))),
        );
        query.get_result(&mut connection).await.switch().map_err(|err| {
            logger::error!(error=?err, %v, data_identifier=%d_id, key_identifier=%k_id, "Failed to get data key from database");
            err
        })
    }
}
