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
use diesel_async::pooled_connection::bb8::Pool;

#[derive(Clone)]
pub struct DbState<T: DbAdapterType> {
    pub pool: Pool<AsyncPgConnection>,
    _adapter: std::marker::PhantomData<T>,
}

type Connection<'a> = PooledConnection<'a, AsyncPgConnection>;

impl<T: DbAdapterType> DbState<T>
where
    Self: DbAdapter,
{
    /// # Panics
    ///
    /// Panics if unable to connect to Database
    #[allow(clippy::expect_used)]
    pub async fn from_config(config: &Config) -> DbState<<Self as DbAdapter>::AdapterType> {
        <Self as DbAdapter>::from_config(config).await
    }

    pub async fn get_conn<'a>(
        &'a self,
    ) -> CustomResult<<Self as DbAdapter>::Conn<'a>, errors::ConnectionError> {
        <Self as DbAdapter>::get_conn(self).await
    }
}
