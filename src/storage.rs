pub(crate) mod cache;
pub(crate) mod dek;
pub(crate) mod types;

use crate::{
    config::Config,
    errors::{self, CustomResult},
};

use error_stack::ResultExt;

use diesel_async::{pooled_connection::bb8::PooledConnection, AsyncPgConnection};

use diesel_async::pooled_connection::{bb8::Pool, AsyncDieselConnectionManager, ManagerConfig};
use masking::PeekInterface;

#[derive(Clone)]
pub struct DbState {
    pub pool: Pool<AsyncPgConnection>,
}

type Connection<'a> = PooledConnection<'a, AsyncPgConnection>;

impl DbState {
    /// # Panics
    ///
    /// Panics if unable to connect to Database
    #[allow(clippy::expect_used)]
    pub async fn from_config(config: &Config) -> Self {
        let database = &config.database;

        let password = database
            .password
            .expose(
                #[cfg(feature = "aws")]
                config,
            )
            .await;

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

        Self { pool }
    }

    pub async fn get_conn(&self) -> CustomResult<Connection<'_>, errors::ConnectionError> {
        let conn = self
            .pool
            .get()
            .await
            .change_context(errors::ConnectionError::ConnectionEstablishFailed)?;

        Ok(conn)
    }
}
