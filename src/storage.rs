pub(crate) mod adapter;
pub(crate) mod cache;
pub(crate) mod dek;
pub(crate) mod metrics;
pub(crate) mod types;

use diesel_async::{AsyncPgConnection, pooled_connection::bb8::PooledConnection};

use self::adapter::{DbAdapter, DbAdapterType};
use crate::{
    config::{Config, ReadStrategy},
    errors::{self, CustomResult},
    multitenancy::TenantId,
};

/// One physical pool plus the observable instruments bound to it.
#[derive(Clone)]
pub struct PoolHandle<C, T: DbAdapterType> {
    pub(crate) pool: C,
    _metrics: T::Metrics,
}

/// Storage handle owned by `SessionState`.
pub struct DbState<C, T: DbAdapterType> {
    primary: PoolHandle<C, T>,
    /// `None` when `[replica_database]` is absent, or for adapters with no replica concept (Cassandra).
    replica: Option<PoolHandle<C, T>>,
    read_strategy: ReadStrategy,
}

/// Resolve the configured strategy against whether a replica exists. With no
/// replica every strategy collapses to `Primary`, so the read path never
/// re-derives replica existence and a metric label never names a missing pool.
pub(crate) const fn resolve_read_strategy(
    strategy: ReadStrategy,
    has_replica: bool,
) -> ReadStrategy {
    if has_replica {
        strategy
    } else {
        ReadStrategy::Primary
    }
}

/// A read view over a `DbState`, carrying the strategy resolved once from the
/// configured value and the pools that exist.
pub(crate) struct ReadView<'a, S> {
    state: &'a S,
    strategy: ReadStrategy,
}

// Manual (not derived): the derived impls would wrongly require `S: Copy`.
impl<S> Clone for ReadView<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ReadView<'_, S> {}

type Connection<'a> = PooledConnection<'a, AsyncPgConnection>;

impl<C, T: DbAdapterType> DbState<C, T> {
    pub(crate) fn read_view(&self) -> ReadView<'_, Self> {
        ReadView {
            state: self,
            strategy: resolve_read_strategy(self.read_strategy, self.replica.is_some()),
        }
    }

    /// The handle for `from`. A `Replica` request with no replica configured
    /// falls back to the primary (the routing layer never asks for a replica
    /// that does not exist; this keeps the pool valid regardless).
    pub(crate) fn handle(&self, from: metrics::DbPool) -> &PoolHandle<C, T> {
        match from {
            metrics::DbPool::Replica => self.replica.as_ref().unwrap_or(&self.primary),
            metrics::DbPool::Primary => &self.primary,
        }
    }
}

impl<C, T: DbAdapterType> DbState<C, T>
where
    Self: DbAdapter,
{
    /// # Panics
    ///
    /// Panics if unable to connect to Database
    pub async fn from_config(
        config: &Config,
        tenant_id: &TenantId,
        schema: &str,
    ) -> DbState<<Self as DbAdapter>::Pool, <Self as DbAdapter>::AdapterType> {
        <Self as DbAdapter>::from_config(config, tenant_id, schema).await
    }

    /// A connection to the write pool (primary). Use for writes and for reads a
    /// write depends on — read-before-write (rotate) and read-your-own-write
    /// (create/transfer) — where a lagging replica would break correctness.
    pub(crate) async fn get_write_pool(
        &self,
    ) -> CustomResult<<Self as DbAdapter>::Conn<'_>, errors::ConnectionError> {
        self.get_conn(metrics::DbPool::Primary).await
    }

    /// Acquire from a specific pool. Only the read-routing layer chooses `from`;
    /// every other caller goes through `get_write_pool`.
    async fn get_conn(
        &self,
        from: metrics::DbPool,
    ) -> CustomResult<<Self as DbAdapter>::Conn<'_>, errors::ConnectionError> {
        <Self as DbAdapter>::get_conn(self, from).await
    }
}
