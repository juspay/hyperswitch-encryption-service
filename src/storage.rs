pub(crate) mod cache;
pub(crate) mod dek;
pub(crate) mod types;

use crate::{
    config::Config,
    errors::{self, CustomResult},
};

use error_stack::ResultExt;

#[cfg(feature = "postgres_ssl")]
use diesel::ConnectionError;
#[cfg(feature = "postgres_ssl")]
use masking::Secret;

use diesel_async::pooled_connection::{bb8::Pool, AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::{pooled_connection::bb8::PooledConnection, AsyncPgConnection};
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

        let password = database.password.expose(config).await;

        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            database.user.peek(),
            password.peek(),
            database.host,
            database.port,
            database.dbname.peek()
        );

        #[cfg(not(feature = "postgres_ssl"))]
        let mgr_config = ManagerConfig::default();

        #[cfg(feature = "postgres_ssl")]
        let mut mgr_config = ManagerConfig::default();

        #[cfg(feature = "postgres_ssl")]
        if database.enable_ssl == Some(true) {
            let root_ca = database
                .root_ca
                .clone()
                .expect("Failed to load db server root cert from the config")
                .expose(config)
                .await;
            mgr_config.custom_setup = Box::new(move |config: &str| {
                Box::pin({
                    let root_ca = root_ca.clone();
                    async move {
                        let rustls_config = rustls::ClientConfig::builder()
                            .with_root_certificates(Self::root_certs(root_ca))
                            .with_no_client_auth();
                        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
                        let (client, conn) = tokio_postgres::connect(config, tls)
                            .await
                            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

                        AsyncPgConnection::try_from_client_and_connection(client, conn).await
                    }
                })
            });
        }

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

    #[allow(clippy::expect_used)]
    #[cfg(feature = "postgres_ssl")]
    fn root_certs(root_ca: Secret<String>) -> rustls::RootCertStore {
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut root_ca.peek().as_ref()) {
            roots
                .add(cert.expect("Failed to load db server root cert"))
                .expect("Failed to add cert to RootCertStore");
        }
        roots
    }
}
