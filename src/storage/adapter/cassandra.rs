mod dek;

use crate::storage::{Config, DbState, adapter::Cassandra, errors};

#[async_trait::async_trait]
impl super::DbAdapter for DbState<scylla::client::caching_session::CachingSession, Cassandra> {
    type Conn<'a> = &'a scylla::client::caching_session::CachingSession;
    type AdapterType = Cassandra;
    type Pool = scylla::client::caching_session::CachingSession;

    #[allow(clippy::expect_used)]
    async fn from_config(config: &Config, schema: &str) -> Self {
        let session = scylla::client::session_builder::SessionBuilder::new()
            .known_nodes(&config.cassandra.known_nodes)
            .pool_size(scylla::client::PoolSize::PerHost(
                config.cassandra.pool_size,
            ))
            .use_keyspace(schema, false)
            .build()
            .await
            .expect("Unable to build the cassandra Pool");

        Self {
            _adapter: std::marker::PhantomData,
            pool: scylla::client::caching_session::CachingSession::from(
                session,
                config.cassandra.cache_size,
            ),
        }
    }

    async fn get_conn<'a>(
        &'a self,
    ) -> errors::CustomResult<Self::Conn<'a>, errors::ConnectionError> {
        Ok(&self.pool)
    }
}
