pub(crate) mod adapter;
pub(crate) mod cache;
pub(crate) mod dek;
pub(crate) mod metrics;
pub(crate) mod types;

use diesel_async::{AsyncPgConnection, pooled_connection::bb8::PooledConnection};

use self::adapter::{DbAdapter, DbAdapterType};
use crate::{
    config::Config,
    errors::{self, CustomResult},
    multitenancy::TenantId,
};

#[derive(Clone)]
pub struct DbState<C, T: DbAdapterType> {
    pub pool: C,
    _metrics: T::Metrics,
}

type Connection<'a> = PooledConnection<'a, AsyncPgConnection>;

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

    pub async fn get_conn(
        &self,
    ) -> CustomResult<<Self as DbAdapter>::Conn<'_>, errors::ConnectionError> {
        <Self as DbAdapter>::get_conn(self).await
    }
}
