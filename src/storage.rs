pub(crate) mod adapter;
pub(crate) mod cache;
pub(crate) mod dek;
pub(crate) mod types;

use crate::{
    config::Config,
    errors::{self, CustomResult},
    multitenancy::tenant_kind::{GlobalTenant, MultiTenant, TenantKind},
    storage::adapter::{Cassandra, PostgreSQL},
};
use diesel_async::{pooled_connection::bb8::PooledConnection, AsyncPgConnection};
use masking::PeekInterface;

use self::adapter::{DbAdapter, DbAdapterType};

#[async_trait::async_trait]
pub trait DatabaseUrl<T: DbAdapterType>: TenantKind {
    async fn get_database_url(config: &Config, schema: &str) -> String;
}

#[async_trait::async_trait]
impl DatabaseUrl<PostgreSQL> for GlobalTenant {
    async fn get_database_url(config: &Config, _schema: &str) -> String {
        let database = &config.database;
        let password = database.password.expose(config).await;

        format!(
            "postgres://{}:{}@{}:{}/{}?application_name={}&options=-c search_path%3D{}",
            database.user.peek(),
            password.peek(),
            database.host,
            database.port,
            database.dbname.peek(),
            config.multitenancy.global_tenant.0.schema,
            config.multitenancy.global_tenant.0.schema
        )
    }
}

#[async_trait::async_trait]
impl DatabaseUrl<PostgreSQL> for MultiTenant {
    async fn get_database_url(config: &Config, schema: &str) -> String {
        let database = &config.database;
        let password = database.password.expose(config).await;

        format!(
            "postgres://{}:{}@{}:{}/{}?application_name={}&options=-c search_path%3D{}",
            database.user.peek(),
            password.peek(),
            database.host,
            database.port,
            database.dbname.peek(),
            schema,
            schema
        )
    }
}

#[async_trait::async_trait]
impl<K: TenantKind> DatabaseUrl<Cassandra> for K {
    async fn get_database_url(_config: &Config, _schema: &str) -> String {
        _schema.to_string()
    }
}

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
    pub async fn from_config<Tenant: TenantKind + DatabaseUrl<<Self as DbAdapter>::AdapterType>>(
        config: &Config,
        schema: &str,
    ) -> DbState<<Self as DbAdapter>::Pool, <Self as DbAdapter>::AdapterType> {
        <Self as DbAdapter>::from_config::<Tenant>(config, schema).await
    }

    pub async fn get_conn(
        &self,
    ) -> CustomResult<<Self as DbAdapter>::Conn<'_>, errors::ConnectionError> {
        <Self as DbAdapter>::get_conn(self).await
    }
}
