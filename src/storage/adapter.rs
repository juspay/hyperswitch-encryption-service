mod cassandra;
mod postgres;

use crate::{config::Config, errors, multitenancy::TenantId, storage::DbState};

#[derive(Clone)]
pub struct PostgreSQL;
pub struct Cassandra;

pub trait DbAdapterType {
    type Metrics;
}

impl DbAdapterType for PostgreSQL {
    type Metrics = postgres::PostgresPoolMetrics;
}

impl DbAdapterType for Cassandra {
    type Metrics = cassandra::CassandraMetrics;
}

#[async_trait::async_trait]
pub trait DbAdapter {
    type Conn<'a>
    where
        Self: 'a;
    type AdapterType: DbAdapterType;
    type Pool;

    async fn from_config(
        config: &Config,
        tenant_id: &TenantId,
        schema: &str,
    ) -> DbState<Self::Pool, Self::AdapterType>;

    async fn get_conn<'a>(
        &'a self,
    ) -> errors::CustomResult<Self::Conn<'a>, errors::ConnectionError>;
}
