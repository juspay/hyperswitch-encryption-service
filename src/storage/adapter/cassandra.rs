mod dek;

use crate::storage::{adapter::Cassandra, errors, Config, DbState};

#[async_trait::async_trait]
impl super::DbAdapter for DbState<scylla::CachingSession, Cassandra> {
    type Conn<'a> = &'a scylla::CachingSession;
    type AdapterType = Cassandra;
    type Pool = scylla::CachingSession;

    #[allow(clippy::expect_used)]
    async fn from_config(config: &Config) -> Self {
        let session = scylla::SessionBuilder::new()
            .known_nodes(&config.cassandra.known_nodes)
            .pool_size(scylla::transport::session::PoolSize::PerHost(
                config.cassandra.pool_size,
            ))
            .use_keyspace(&config.cassandra.keyspace, false)
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
