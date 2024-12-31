mod dek;

use crate::{
    multitenancy::tenant_kind::TenantKind,
    storage::{adapter::Cassandra, errors, Config, DatabaseUrl, DbState},
};

#[async_trait::async_trait]
impl super::DbAdapter for DbState<scylla::CachingSession, Cassandra> {
    type Conn<'a> = &'a scylla::CachingSession;
    type AdapterType = Cassandra;
    type Pool = scylla::CachingSession;

    #[allow(clippy::expect_used)]
    async fn from_config<Tenant: TenantKind + DatabaseUrl<Self::AdapterType>>(
        config: &Config,
        schema: &str,
    ) -> Self {
        let session = scylla::SessionBuilder::new()
            .known_nodes(&config.cassandra.known_nodes)
            .pool_size(scylla::transport::session::PoolSize::PerHost(
                config.cassandra.pool_size,
            ))
            .use_keyspace(Tenant::get_database_url(config, schema).await, false)
            .build()
            .await
            .expect("Unable to build the cassandra Pool");

        Self {
            _adapter: std::marker::PhantomData,
            pool: scylla::CachingSession::from(session, config.cassandra.cache_size),
        }
    }

    async fn get_conn<'a>(
        &'a self,
    ) -> errors::CustomResult<Self::Conn<'a>, errors::ConnectionError> {
        Ok(&self.pool)
    }
}
