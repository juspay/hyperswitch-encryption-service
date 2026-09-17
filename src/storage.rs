pub(crate) mod adapter;
pub(crate) mod cache;
pub(crate) mod dek;
pub(crate) mod metrics;
pub(crate) mod types;

use diesel_async::{AsyncPgConnection, pooled_connection::bb8::PooledConnection};

use self::adapter::{DbAdapter, DbAdapterType};
use crate::{
    config::{Config, ReadFrom},
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
    read_strategy: ReadFrom,
}

/// The resolved route for a single logical read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadRoute {
    Only(metrics::DbPool),
    ReplicaThenPrimary,
}

/// Pure resolution of a caller's strategy against the pools that exist.
pub(crate) const fn read_route(strategy: ReadFrom, has_replica: bool) -> ReadRoute {
    if !has_replica {
        return ReadRoute::Only(metrics::DbPool::Primary);
    }
    match strategy {
        ReadFrom::Primary => ReadRoute::Only(metrics::DbPool::Primary),
        ReadFrom::Replica => ReadRoute::Only(metrics::DbPool::Replica),
        ReadFrom::ReplicaThenPrimary => ReadRoute::ReplicaThenPrimary,
    }
}

/// A read view over a `DbState`, carrying the route resolved once from the configured strategy and the pools that exist.
pub(crate) struct ReadDbPool<'a, S> {
    state: &'a S,
    route: ReadRoute,
}

// Manual (not derived): the derived impls would wrongly require `S: Copy`.
impl<S> Clone for ReadDbPool<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ReadDbPool<'_, S> {}

type Connection<'a> = PooledConnection<'a, AsyncPgConnection>;

impl<C, T: DbAdapterType> DbState<C, T> {
    pub(crate) fn read_db_pool(&self) -> ReadDbPool<'_, Self> {
        ReadDbPool {
            state: self,
            route: read_route(self.read_strategy, self.replica.is_some()),
        }
    }

    /// Normalise a requested pool so metric labels never lie: a `Replica` request degrades to `Primary` when no replica is configured.
    pub(crate) fn effective_pool(&self, from: metrics::DbPool) -> metrics::DbPool {
        match from {
            metrics::DbPool::Replica if self.replica.is_none() => metrics::DbPool::Primary,
            pool => pool,
        }
    }

    /// The handle for `from`, falling back to the primary when no replica exists.
    pub(crate) fn handle(&self, from: metrics::DbPool) -> &PoolHandle<C, T> {
        match self.effective_pool(from) {
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

    pub(crate) async fn get_conn(
        &self,
        from: metrics::DbPool,
    ) -> CustomResult<<Self as DbAdapter>::Conn<'_>, errors::ConnectionError> {
        <Self as DbAdapter>::get_conn(self, from).await
    }
}
