mod dek;

use diesel::ConnectionError;
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, ManagerConfig, bb8::Pool},
};
use error_stack::ResultExt;
use hyperswitch_masking::{ExposeInterface, PeekInterface};
#[cfg(feature = "postgres_ssl")]
use rustls::pki_types::pem::PemObject;

use crate::{
    env::metrics,
    multitenancy::TenantId,
    storage::{
        Config, Connection, DbState, adapter::PostgreSQL, errors, metrics as storage_metrics,
    },
};

pub struct PostgresPoolMetrics {
    // The observable instruments aren't read directly, we just need to keep them alive for metrics
    // to be recorded.
    _pool_size: metrics_utils::opentelemetry::ObservableGauge<u64>,
    _pool_available: metrics_utils::opentelemetry::ObservableGauge<u64>,
    _pool_waiting: metrics_utils::opentelemetry::ObservableGauge<u64>,
    _created_connections: metrics_utils::opentelemetry::ObservableCounter<u64>,
    _closed_connections: metrics_utils::opentelemetry::ObservableCounter<u64>,
}

impl PostgresPoolMetrics {
    pub(super) fn new(pool: Pool<AsyncPgConnection>, tenant_id: &TenantId) -> Self {
        let db_pool = storage_metrics::DbPool::Primary;

        let _pool_size = Self::build_pool_gauge(
            "database.pool.size",
            "Total number of connections in the database pool",
            "{connection}",
            db_pool,
            pool.clone(),
            tenant_id.clone(),
            |pool| u64::from(pool.state().connections),
        );

        let _pool_available = Self::build_pool_gauge(
            "database.pool.available",
            "Number of available connections in the database pool",
            "{connection}",
            db_pool,
            pool.clone(),
            tenant_id.clone(),
            |pool| u64::from(pool.state().idle_connections),
        );

        let _pool_waiting = Self::build_pool_gauge(
            "database.pool.waiting",
            "Number of callers waiting for a database connection",
            "{connection}",
            db_pool,
            pool.clone(),
            tenant_id.clone(),
            |pool| pool.state().statistics.pending_gets(),
        );

        let _created_connections = {
            let pool_clone = pool.clone();
            let tenant_id_clone = tenant_id.to_owned();
            metrics::CRIPTA_METER
                .u64_observable_counter("database.pool.created")
                .with_description("Total database connections created")
                .with_unit("{connection}")
                .with_callback(move |observer| {
                    let stats = pool_clone.state().statistics;
                    let tenant_id = tenant_id_clone.clone().into_inner();

                    observer.observe(
                        stats.connections_created,
                        metrics_utils::metric_attributes!(
                            ("pool", db_pool),
                            ("tenant_id", tenant_id)
                        ),
                    );
                })
                .build()
        };

        let _closed_connections = {
            let pool_clone = pool;
            let tenant_id_clone = tenant_id.to_owned();
            metrics::CRIPTA_METER
                .u64_observable_counter("database.pool.closed")
                .with_description("Total database connections closed")
                .with_unit("{connection}")
                .with_callback(move |observer| {
                    let stats = pool_clone.state().statistics;
                    let tenant_id = tenant_id_clone.clone().into_inner();

                    for (value, reason) in [
                        (stats.connections_closed_broken, "broken"),
                        (stats.connections_closed_invalid, "invalid"),
                        (stats.connections_closed_max_lifetime, "max_lifetime"),
                        (stats.connections_closed_idle_timeout, "idle_timeout"),
                    ] {
                        observer.observe(
                            value,
                            metrics_utils::metric_attributes!(
                                ("pool", db_pool),
                                ("tenant_id", tenant_id.clone()),
                                ("reason", reason)
                            ),
                        );
                    }
                })
                .build()
        };

        Self {
            _pool_size,
            _pool_available,
            _pool_waiting,
            _created_connections,
            _closed_connections,
        }
    }

    fn build_pool_gauge(
        name: &'static str,
        description: &'static str,
        unit: &'static str,
        db_pool: storage_metrics::DbPool,
        pool: Pool<AsyncPgConnection>,
        tenant_id: TenantId,
        read: impl Fn(&Pool<AsyncPgConnection>) -> u64 + Send + Sync + 'static,
    ) -> metrics_utils::opentelemetry::ObservableGauge<u64> {
        metrics::CRIPTA_METER
            .u64_observable_gauge(name)
            .with_description(description)
            .with_unit(unit)
            .with_callback(move |observer| {
                let tenant_id = tenant_id.clone().into_inner();

                observer.observe(
                    read(&pool),
                    metrics_utils::metric_attributes!(("pool", db_pool), ("tenant_id", tenant_id)),
                );
            })
            .build()
    }
}

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
    async fn from_config(config: &Config, tenant_id: &TenantId, schema: &str) -> Self {
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
                Box::pin({
                    let root_ca = root_ca.clone();
                    async move {
                        let mut root_certificate = rustls::RootCertStore::empty();
                        for cert in rustls::pki_types::CertificateDer::pem_slice_iter(
                            root_ca.peek().as_ref(),
                        ) {
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
                    }
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

        let _metrics = PostgresPoolMetrics::new(pool.clone(), tenant_id);

        Self { pool, _metrics }
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
