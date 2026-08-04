use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, associations::HasTable};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::Pool};
use error_stack::ResultExt;

use super::DbState;
use crate::{
    errors::{self, CustomResult, SwitchError},
    schema::data_key_store::*,
    storage::{
        adapter::PostgreSQL,
        dek::DataKeyStorageInterface,
        metrics,
        types::{DataKey, DataKeyNew},
    },
    types::{Identifier, key::Version},
};

#[async_trait::async_trait]
impl DataKeyStorageInterface for DbState<Pool<AsyncPgConnection>, PostgreSQL> {
    async fn get_or_insert_data_key(
        &self,
        operation: metrics::DataKeyStorageOperation,
        new: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let identifier: errors::CustomResult<Identifier, errors::ParsingError> =
            (new.data_identifier.clone(), new.key_identifier.clone()).try_into();

        let v = new.version;

        let mut connection = self.get_conn().await.switch()?;
        let query = diesel::insert_into(DataKey::table()).values(new);

        let pool = metrics::DbPool::Primary;
        let db_op = metrics::DbOperation::Insert;
        metrics::log_db_query::<table, _>(&query, db_op, pool);

        let result = metrics::record_db_query::<table, _, _, _>(
            query.get_result(&mut connection),
            db_op,
            pool,
        )
        .await
        .switch();

        match result {
            Ok(result) => {
                crate::env::metrics::DATA_KEY_OPERATION_COUNT.add(
                    1,
                    metrics_utils::metric_attributes!(
                        ("operation", operation),
                        ("outcome", metrics::DataKeyStorageOutcome::Created),
                    ),
                );
                Ok(result)
            }
            Err(err) => match err.current_context() {
                errors::DatabaseError::UniqueViolation => {
                    crate::env::metrics::DATA_KEY_OPERATION_COUNT.add(
                        1,
                        metrics_utils::metric_attributes!(
                            ("operation", operation),
                            ("outcome", metrics::DataKeyStorageOutcome::FoundExisting),
                        ),
                    );
                    self.get_key(
                        v,
                        &identifier
                            .change_context(errors::DatabaseError::Others)
                            .attach("Failed to parse identifier")?,
                    )
                    .await
                }
                _ => {
                    crate::env::metrics::DATA_KEY_OPERATION_COUNT.add(
                        1,
                        metrics_utils::metric_attributes!(
                            ("operation", operation),
                            ("outcome", metrics::DataKeyStorageOutcome::Error),
                        ),
                    );
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
            .filter(data_identifier.eq(d_id).and(key_identifier.eq(k_id)))
            .order_by(version.desc())
            .limit(1);

        let pool = metrics::DbPool::Primary;
        let db_op = metrics::DbOperation::Filter;
        metrics::log_db_query::<table, _>(&query, db_op, pool);

        metrics::record_db_query::<table, _, _, _>(query.get_result(&mut connection), db_op, pool)
            .await
            .switch()
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
                .and(data_identifier.eq(d_id).and(key_identifier.eq(k_id))),
        );

        let pool = metrics::DbPool::Primary;
        let db_op = metrics::DbOperation::FindOne;
        metrics::log_db_query::<table, _>(&query, db_op, pool);

        metrics::record_db_query::<table, _, _, _>(query.get_result(&mut connection), db_op, pool)
            .await
            .switch()
    }
}
