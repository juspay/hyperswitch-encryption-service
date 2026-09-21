//! Storage metrics and timing wrappers.

use std::future::Future;

use crate::env::metrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum DbPool {
    Primary,
    Replica,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum DbOperation {
    Insert,
    FindOne,
    Filter,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum DataKeyStorageOperation {
    Create,
    Rotate,
}

#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum DataKeyStorageOutcome {
    Created,
    FoundExisting,
    Error,
}

crate::impl_metric_value_from!(
    DbPool,
    DbOperation,
    DataKeyStorageOperation,
    DataKeyStorageOutcome
);

pub(super) async fn record_db_connection_acquire_duration<Fut, T, E>(
    future: Fut,
    pool: DbPool,
) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>>,
{
    let start = std::time::Instant::now();
    let result = future.await;
    let duration = start.elapsed();
    let outcome = if result.is_ok() { "success" } else { "error" };

    metrics::DATABASE_CONNECTION_ACQUIRE_DURATION.record(
        duration.as_secs_f64(),
        metrics_utils::metric_attributes!(("pool", pool), ("outcome", outcome)),
    );

    result
}

/// Extract the table name from `T`'s module path (e.g. `data_key_store`).
fn table_name<T>() -> &'static str
where
    T: diesel::associations::HasTable<Table = T>,
{
    std::any::type_name::<T>()
        .rsplit("::")
        .nth(1)
        .unwrap_or("UNKNOWN")
}

#[track_caller]
pub(super) fn log_db_query<T, Q>(query: &Q, operation: DbOperation, pool: DbPool)
where
    T: diesel::associations::HasTable<Table = T>,
    Q: diesel::query_builder::QueryFragment<diesel::pg::Pg>,
{
    tracing::debug!(
        query = %diesel::debug_query(query),
        table = %table_name::<T>(),
        operation = %<&'static str>::from(operation),
        pool = %<&'static str>::from(pool),
        "Executing database query",
    );
}

/// Record that a replica read was retried on the primary.
pub(super) fn record_db_read_fallback<T>(operation: DbOperation, reason: &'static str)
where
    T: diesel::associations::HasTable<Table = T>,
{
    metrics::DATABASE_READ_FALLBACK_COUNT.add(
        1,
        metrics_utils::metric_attributes!(
            ("table", table_name::<T>()),
            ("operation", operation),
            ("reason", reason)
        ),
    );
}

pub(super) async fn record_db_query<T, Fut, R, E>(
    future: Fut,
    operation: DbOperation,
    pool: DbPool,
) -> Result<R, E>
where
    T: diesel::associations::HasTable<Table = T>,
    Fut: Future<Output = Result<R, E>>,
{
    metrics::DATABASE_QUERY_COUNT.add(
        1,
        metrics_utils::metric_attributes!(
            ("table", table_name::<T>()),
            ("operation", operation),
            ("pool", pool)
        ),
    );

    let start = std::time::Instant::now();
    let result = future.await;
    let duration = start.elapsed();
    let outcome = if result.is_ok() { "success" } else { "error" };

    metrics::DATABASE_QUERY_DURATION.record(
        duration.as_secs_f64(),
        metrics_utils::metric_attributes!(
            ("table", table_name::<T>()),
            ("operation", operation),
            ("pool", pool),
            ("outcome", outcome)
        ),
    );

    result
}
