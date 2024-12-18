pub(crate) mod cache;
pub(crate) mod dek;
pub(crate) mod types;

use crate::{
    config::Config,
    errors::{self, CustomResult},
};

use error_stack::ResultExt;

#[cfg(feature = "postgres_ssl")]
use diesel::{ConnectionError, ConnectionResult};

#[cfg(feature = "postgres_ssl")]
use futures_util::future::{BoxFuture, FutureExt};

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
            mgr_config.custom_setup = Box::new(Self::establish_connection);
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

    #[cfg(feature = "postgres_ssl")]
    fn establish_connection(config: &str) -> BoxFuture<'_, ConnectionResult<AsyncPgConnection>> {
        let fut = async {
            let rustls_config = rustls::ClientConfig::builder()
                .with_root_certificates(Self::root_certs())
                .with_no_client_auth();
            let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
            let (client, conn) = tokio_postgres::connect(config, tls)
                .await
                .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

            AsyncPgConnection::try_from_client_and_connection(client, conn).await
        };
        fut.boxed()
    }

    #[allow(clippy::expect_used)]
    #[cfg(feature = "postgres_ssl")]
    fn root_certs() -> rustls::RootCertStore {
        use crate::{consts::DB_ROOT_CA_PATH, env::observability as logger};
        use std::{env, fs, io::BufReader};

        let mut roots = rustls::RootCertStore::empty();

        match env::var(DB_ROOT_CA_PATH) {
            Ok(root_ca_path) => {
                logger::info!("Trying to load server root cert from the path {root_ca_path}",);
                let cert_data = fs::read(&root_ca_path).expect("Failed to read root cert");
                let mut reader = BufReader::new(&cert_data[..]);
                let certs = rustls_pemfile::certs(&mut reader)
                    .flatten()
                    .collect::<Vec<_>>();
                for cert in certs {
                    roots
                        .add(cert)
                        .expect("Failed to add cert to RootCertStore");
                }
            }
            Err(_) => {
                logger::info!("Trying to load server root cert from the System trusted store");
                // Loads certs from the system's trusted store.
                let certs = rustls_native_certs::load_native_certs()
                    .expect("Failed to load certs from system for SSL connection");
                roots.add_parsable_certificates(certs);
            }
        }
        roots
    }
}
