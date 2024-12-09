mod dek;

use error_stack::ResultExt;

use crate::storage::{adapter::PostgreSQL, errors, Config, Connection, DbState};
use diesel_async::pooled_connection::{bb8::Pool, AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::AsyncPgConnection;
use masking::PeekInterface;

#[async_trait::async_trait]
impl super::DbAdapter for DbState<PostgreSQL> {
    type Conn<'a> = Connection<'a>;
    type AdapterType = PostgreSQL;

    async fn from_config(config: &Config) -> Self {
        let database = &config.database;

        let password = database.password.expose(config).await;

        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            database.user.peek(),
            password.peek(),
            database.host,
            database.port,
            database.dbname.peek()
        );

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
