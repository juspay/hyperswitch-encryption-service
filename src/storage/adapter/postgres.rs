mod dek;

use error_stack::ResultExt;

use crate::storage::{
    adapter::PostgreSQL, errors, Config, Connection, DatabaseUrl, DbState, TenantKind,
};
use diesel_async::pooled_connection::{bb8::Pool, AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::AsyncPgConnection;

#[async_trait::async_trait]
impl super::DbAdapter for DbState<Pool<AsyncPgConnection>, PostgreSQL> {
    type Conn<'a> = Connection<'a>;
    type AdapterType = PostgreSQL;
    type Pool = Pool<AsyncPgConnection>;

    /// # Panics
    ///
    /// Panics if unable to connect to Database
    #[allow(clippy::expect_used)]
    async fn from_config<Tenant: TenantKind + DatabaseUrl<Self::AdapterType>>(
        config: &Config,
        schema: &str,
    ) -> Self {
        let database = &config.database;
        let database_url = Tenant::get_database_url(&config, schema).await;

        let mgr_config = ManagerConfig::default();
        let mgr = AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
            database_url,
            mgr_config,
        );
        let pool = Pool::builder()
            .max_size(database.pool_size.unwrap_or(10))
            .min_idle(database.min_idle)
            .build(mgr)
            .await
            .expect("Failed to establish pool connection");

        Self {
            _adapter: std::marker::PhantomData,
            pool,
        }
    }

    async fn get_conn<'a>(
        &'a self,
    ) -> errors::CustomResult<Self::Conn<'a>, errors::ConnectionError> {
        self.pool
            .get()
            .await
            .change_context(errors::ConnectionError::ConnectionEstablishFailed)
    }
}
