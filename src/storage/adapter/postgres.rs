mod dek;

use diesel::ConnectionError;
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, ManagerConfig, bb8::Pool},
};
use error_stack::ResultExt;
use hyperswitch_masking::{ExposeInterface, PeekInterface};

use crate::storage::{Config, Connection, DbState, adapter::PostgreSQL, errors};

fn no_tls_custom_setup(
    pg_config: tokio_postgres::Config,
) -> diesel_async::pooled_connection::SetupCallback<AsyncPgConnection> {
    Box::new(move |_url| {
        let pg_config = pg_config.clone();
        Box::pin(async move {
            let (client, conn) = pg_config
                .connect(tokio_postgres::NoTls)
                .await
                .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
            AsyncPgConnection::try_from_client_and_connection(client, conn).await
        })
    })
}

// We're accepting a decrypted password separately instead of reading the encrypted password from
// the database config and decrypting it.
// It helps keep this function pure, sync and infallible.
fn build_pg_config(
    database: &crate::config::Database,
    schema: &str,
    password: hyperswitch_masking::Secret<String>,
) -> tokio_postgres::Config {
    let mut pg_config = tokio_postgres::Config::new();
    pg_config.host(database.host.clone());
    pg_config.port(database.port);
    pg_config.user(database.user.peek().clone());
    pg_config.password(password.expose());
    pg_config.dbname(database.dbname.peek().clone());
    pg_config.application_name(schema);
    pg_config.options(format!("-c search_path={schema}"));
    if let Some(connect_timeout) = database.connect_timeout_secs {
        pg_config.connect_timeout(std::time::Duration::from_secs(connect_timeout.get()));
    }

    pg_config
}

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
        let pg_config = build_pg_config(database, schema, password);

        // Minimal URL passed to `AsyncDieselConnectionManager::new_with_config()`,
        // our `custom_setup` closure currently ignores the URL.
        let database_url = format!(
            "postgres://{}@{}:{}/{}",
            database.user.peek(),
            database.host,
            database.port,
            database.dbname.peek(),
        );

        let mut mgr_config = ManagerConfig::default();

        #[cfg(feature = "postgres_ssl")]
        if database.enable_ssl == Some(true) {
            let root_ca = database
                .root_ca
                .clone()
                .expect("Failed to load db server root cert from the config")
                .expose(config)
                .await;
            let pg_config_for_closure = pg_config.clone();

            mgr_config.custom_setup = Box::new(move |_url| {
                let pg_config = pg_config_for_closure.clone();
                let root_ca = root_ca.clone();
                Box::pin(async move {
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
                    let (client, conn) = pg_config
                        .connect(tls)
                        .await
                        .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
                    AsyncPgConnection::try_from_client_and_connection(client, conn).await
                })
            });
        } else {
            mgr_config.custom_setup = no_tls_custom_setup(pg_config);
        }

        #[cfg(not(feature = "postgres_ssl"))]
        {
            mgr_config.custom_setup = no_tls_custom_setup(pg_config);
        }

        let mgr = AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
            database_url,
            mgr_config,
        );

        let mut pool_builder = Pool::builder()
            .max_size(database.pool_size.unwrap_or(10))
            .min_idle(database.min_idle);

        if let Some(max_lifetime) = database.max_lifetime_secs {
            pool_builder =
                pool_builder.max_lifetime(std::time::Duration::from_secs(max_lifetime.get()));
        }
        if let Some(idle_timeout) = database.idle_timeout_secs {
            pool_builder =
                pool_builder.idle_timeout(std::time::Duration::from_secs(idle_timeout.get()));
        }
        if let Some(connection_acquire_timeout) = database.connection_acquire_timeout_secs {
            pool_builder = pool_builder.connection_timeout(std::time::Duration::from_secs(
                connection_acquire_timeout.get(),
            ));
        }

        let pool = pool_builder
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
        let pool = crate::storage::metrics::DbPool::Primary;
        crate::storage::metrics::record_db_connection_acquire_duration(self.pool.get(), pool)
            .await
            .change_context(errors::ConnectionError::ConnectionEstablishFailed)
    }
}
