mod cassandra;
mod postgres;

use crate::{
    config::Config,
    errors,
    multitenancy::tenant_kind::TenantKind,
    storage::{DatabaseUrl, DbState},
};

#[derive(Clone)]
pub struct PostgreSQL;
pub struct Cassandra;

pub trait DbAdapterType {}

impl DbAdapterType for PostgreSQL {}
impl DbAdapterType for Cassandra {}

#[async_trait::async_trait]
pub trait DbAdapter {
    type Conn<'a>
    where
        Self: 'a;
    type AdapterType: DbAdapterType;
    type Pool;
    async fn from_config<Tenant: TenantKind + DatabaseUrl<Self::AdapterType>>(
        config: &Config,
        schema: &str,
    ) -> DbState<Self::Pool, Self::AdapterType>;
    async fn get_conn<'a>(
        &'a self,
    ) -> errors::CustomResult<Self::Conn<'a>, errors::ConnectionError>;
}
