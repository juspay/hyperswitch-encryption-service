mod dek;

use crate::storage::{adapter::Cassandra, errors, Config, Connection, DbState};

#[async_trait::async_trait]
impl super::DbAdapter
    for DbState<
        diesel_async::pooled_connection::bb8::Pool<diesel_async::AsyncPgConnection>,
        Cassandra,
    >
{
    type Conn<'a> = Connection<'a>;
    type AdapterType = Cassandra;
    type Pool = diesel_async::pooled_connection::bb8::Pool<diesel_async::AsyncPgConnection>;

    async fn from_config(_config: &Config) -> Self {
        unimplemented!("Not implemented Yet")
    }

    async fn get_conn<'a>(
        &'a self,
    ) -> errors::CustomResult<Self::Conn<'a>, errors::ConnectionError> {
        unimplemented!("Not implemented Yet")
    }
}
