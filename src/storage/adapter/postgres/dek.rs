use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, associations::HasTable};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::Pool};
use error_stack::ResultExt;

use super::DbState;
use crate::{
    env::observability as logger,
    errors::{self, CustomResult, SwitchError},
    schema::data_key_store::*,
    storage::{
        ReadDbPool, ReadRoute,
        adapter::PostgreSQL,
        dek::{DataKeyReadInterface, DataKeyStorageInterface},
        metrics::{self, DbPool},
        types::{DataKey, DataKeyNew},
    },
    types::{Identifier, key::Version},
};

impl DbState<Pool<AsyncPgConnection>, PostgreSQL> {
    async fn get_latest_version_from(
        &self,
        from: DbPool,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        let mut connection = self.get_conn(from).await.switch()?;

        let (d_id, k_id) = identifier.get_identifier();
        let query = DataKey::table()
            .select(version)
            .filter(data_identifier.eq(d_id).and(key_identifier.eq(k_id)))
            .order_by(version.desc())
            .limit(1);

        metrics::log_db_query::<table, _>(&query, metrics::DbOperation::Filter, from);

        metrics::record_db_query::<table, _, _, _>(
            query.get_result(&mut connection),
            metrics::DbOperation::Filter,
            from,
        )
        .await
        .switch()
    }

    async fn get_key_from(
        &self,
        from: DbPool,
        v: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let mut connection = self.get_conn(from).await.switch()?;

        let (d_id, k_id) = identifier.get_identifier();

        let query = DataKey::table().filter(
            version
                .eq(v)
                .and(data_identifier.eq(d_id).and(key_identifier.eq(k_id))),
        );

        metrics::log_db_query::<table, _>(&query, metrics::DbOperation::FindOne, from);

        metrics::record_db_query::<table, _, _, _>(
            query.get_result(&mut connection),
            metrics::DbOperation::FindOne,
            from,
        )
        .await
        .switch()
    }
}

/// Map a replica failure to a bounded-cardinality metric label.
const fn fallback_reason(error: &errors::DatabaseError) -> &'static str {
    match error {
        errors::DatabaseError::ConnectionError(_) => "connection_error",
        errors::DatabaseError::NotFound => "not_found",
        errors::DatabaseError::UniqueViolation => "unique_violation",
        errors::DatabaseError::NotNullViolation => "not_null_violation",
        errors::DatabaseError::InvalidValue => "invalid_value",
        errors::DatabaseError::Others => "others",
    }
}

/// Run a read against the pool(s) selected by `route`. `ReplicaThenPrimary` retries on the primary when the replica fails for *any* reason, including `NotFound` (replication lag).
async fn with_read_fallback<T, F, Fut, R>(
    route: ReadRoute,
    db_op: metrics::DbOperation,
    attempt: F,
) -> CustomResult<R, errors::DatabaseError>
where
    T: diesel::associations::HasTable<Table = T>,
    F: Fn(DbPool) -> Fut,
    Fut: std::future::Future<Output = CustomResult<R, errors::DatabaseError>>,
{
    match route {
        ReadRoute::Only(pool) => attempt(pool).await,
        ReadRoute::ReplicaThenPrimary => match attempt(DbPool::Replica).await {
            Ok(value) => Ok(value),
            Err(replica_error) => {
                let reason = fallback_reason(replica_error.current_context());
                logger::warn!(
                    error = ?replica_error,
                    reason,
                    "Replica read failed; retrying on primary"
                );
                metrics::record_db_read_fallback::<T>(db_op, reason);
                // The primary is authoritative: when both pools fail, its error is returned with the replica's reason attached.
                attempt(DbPool::Primary)
                    .await
                    .attach(format!("replica read failed first: {reason}"))
            }
        },
    }
}

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

        let mut connection = self.get_conn(DbPool::Primary).await.switch()?;
        let query = diesel::insert_into(DataKey::table()).values(new);

        let pool = DbPool::Primary;
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
                    // Read-your-own-write: `get_key` on this trait is pinned to the primary.
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
        self.get_latest_version_from(DbPool::Primary, identifier)
            .await
    }

    async fn get_key(
        &self,
        v: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        self.get_key_from(DbPool::Primary, v, identifier).await
    }
}

#[async_trait::async_trait]
impl DataKeyReadInterface for ReadDbPool<'_, DbState<Pool<AsyncPgConnection>, PostgreSQL>> {
    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        let state = self.state;
        with_read_fallback::<table, _, _, _>(
            self.route,
            metrics::DbOperation::Filter,
            move |pool| state.get_latest_version_from(pool, identifier),
        )
        .await
    }

    async fn get_key(
        &self,
        v: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let state = self.state;
        with_read_fallback::<table, _, _, _>(
            self.route,
            metrics::DbOperation::FindOne,
            move |pool| state.get_key_from(pool, v, identifier),
        )
        .await
    }
}
