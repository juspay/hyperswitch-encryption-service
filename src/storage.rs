pub(crate) mod adapter;
pub(crate) mod cache;
pub(crate) mod dek;
pub(crate) mod types;

use crate::{
    config::Config,
    errors::{self, CustomResult},
};

use diesel_async::{pooled_connection::bb8::PooledConnection, AsyncPgConnection};

use self::adapter::{DbAdapter, DbAdapterType};

#[derive(Clone)]
pub struct DbState<C, T: DbAdapterType> {
    pub pool: C,
    _adapter: std::marker::PhantomData<T>,
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
    ) -> DbState<<Self as DbAdapter>::Pool, <Self as DbAdapter>::AdapterType> {
        <Self as DbAdapter>::from_config(config).await
    }

    pub async fn get_conn(
        &self,
    ) -> CustomResult<<Self as DbAdapter>::Conn<'_>, errors::ConnectionError> {
        <Self as DbAdapter>::get_conn(self).await
    }
}
