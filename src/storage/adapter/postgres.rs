mod dek;

use error_stack::ResultExt;

use crate::storage::{adapter::PostgreSQL, errors, Config, Connection, DbState};
use masking::PeekInterface;

#[cfg(feature = "postgres_ssl")]
use diesel::ConnectionError;

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
    async fn from_config(config: &Config, schema: &str) -> Self {
        let database = &config.database;
        let password = database.password.expose(config).await;
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}?application_name={}&options=-c search_path%3D{}",
            database.user.peek(),
            password.peek(),
            database.host,
            database.port,
            database.dbname.peek(),
            schema,
            schema
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
                        let mut root_certificate = rustls::RootCertStore::empty();
                        for cert in rustls_pemfile::certs(&mut root_ca.peek().as_ref()) {
                            root_certificate
                                .add(cert.expect("Failed to load db server root cert"))
                                .expect("Failed to add cert to RootCertStore");
                        }
                        let rustls_config = rustls::ClientConfig::builder()
                            .with_root_certificates(root_certificate)
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
