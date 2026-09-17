use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, associations::HasTable};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl,
    pooled_connection::bb8::{Pool, PooledConnection},
};
use error_stack::ResultExt;

use super::DbState;
use crate::{
    config::ReadStrategy,
    env::observability as logger,
    errors::{self, CustomResult, SwitchError},
    schema::data_key_store::*,
    storage::{
        ReadView,
        adapter::PostgreSQL,
        dek::{DataKeyReadInterface, DataKeyStorageInterface},
        metrics::{self, DbPool},
        types::{DataKey, DataKeyNew},
    },
    types::{Identifier, key::Version},
};

/// Build and run the latest-version query on `connection`, labelling metrics with `pool`.
async fn query_latest_version(
    connection: &mut PooledConnection<'_, AsyncPgConnection>,
    identifier: &Identifier,
    pool: DbPool,
) -> CustomResult<Version, errors::DatabaseError> {
    let (data_id, key_id) = identifier.get_identifier();
    let query = DataKey::table()
        .select(version)
        .filter(data_identifier.eq(data_id).and(key_identifier.eq(key_id)))
        .order_by(version.desc())
        .limit(1);

    metrics::log_db_query::<table, _>(&query, metrics::DbOperation::Filter, pool);

    metrics::record_db_query::<table, _, _, _>(
        query.get_result(connection),
        metrics::DbOperation::Filter,
        pool,
    )
    .await
    .switch()
}

/// Build and run the find-key query on `connection`, labelling metrics with `pool`.
async fn query_key(
    connection: &mut PooledConnection<'_, AsyncPgConnection>,
    key_version: Version,
    identifier: &Identifier,
    pool: DbPool,
) -> CustomResult<DataKey, errors::DatabaseError> {
    let (data_id, key_id) = identifier.get_identifier();

    let query = DataKey::table().filter(
        version
            .eq(key_version)
            .and(data_identifier.eq(data_id).and(key_identifier.eq(key_id))),
    );

    metrics::log_db_query::<table, _>(&query, metrics::DbOperation::FindOne, pool);

    metrics::record_db_query::<table, _, _, _>(
        query.get_result(connection),
        metrics::DbOperation::FindOne,
        pool,
    )
    .await
    .switch()
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

/// Run a read against the pool(s) selected by `strategy`. `ReplicaThenPrimary` retries on the primary when the replica fails for *any* reason, including `NotFound` (replication lag).
async fn with_read_fallback<T, F, Fut, R>(
    strategy: ReadStrategy,
    db_op: metrics::DbOperation,
    attempt: F,
) -> CustomResult<R, errors::DatabaseError>
where
    T: diesel::associations::HasTable<Table = T>,
    F: Fn(DbPool) -> Fut,
    Fut: std::future::Future<Output = CustomResult<R, errors::DatabaseError>>,
{
    match strategy {
        ReadStrategy::Primary => attempt(DbPool::Primary).await,
        ReadStrategy::Replica => attempt(DbPool::Replica).await,
        ReadStrategy::ReplicaThenPrimary => match attempt(DbPool::Replica).await {
            Ok(value) => Ok(value),
            Err(replica_error) => {
                let reason = fallback_reason(replica_error.current_context());
                // debug, not warn: under a replica outage this fires per read;
                // `database.read.fallback.count` is the aggregated alerting signal.
                logger::debug!(
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
        new_key: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let identifier: errors::CustomResult<Identifier, errors::ParsingError> =
            (new_key.data_identifier.clone(), new_key.key_identifier.clone()).try_into();

        let key_version = new_key.version;

        let mut connection = self.get_write_pool().await.switch()?;
        let query = diesel::insert_into(DataKey::table()).values(new_key);

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
                    // Read-your-own-write: `get_key` on this trait uses the write pool.
                    self.get_key(
                        key_version,
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
        let mut connection = self.get_write_pool().await.switch()?;
        query_latest_version(&mut connection, identifier, DbPool::Primary).await
    }

    async fn get_key(
        &self,
        key_version: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let mut connection = self.get_write_pool().await.switch()?;
        query_key(&mut connection, key_version, identifier, DbPool::Primary).await
    }
}

#[async_trait::async_trait]
impl DataKeyReadInterface for ReadView<'_, DbState<Pool<AsyncPgConnection>, PostgreSQL>> {
    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        let state = self.state;
        with_read_fallback::<table, _, _, _>(
            self.strategy,
            metrics::DbOperation::Filter,
            move |pool| async move {
                let mut connection = state.get_conn(pool).await.switch()?;
                query_latest_version(&mut connection, identifier, pool).await
            },
        )
        .await
    }

    async fn get_key(
        &self,
        key_version: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let state = self.state;
        with_read_fallback::<table, _, _, _>(
            self.strategy,
            metrics::DbOperation::FindOne,
            move |pool| async move {
                let mut connection = state.get_conn(pool).await.switch()?;
                query_key(&mut connection, key_version, identifier, pool).await
            },
        )
        .await
    }
}
