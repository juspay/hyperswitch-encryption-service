use std::sync::Arc;

use error_stack::ResultExt;
use hyper::header;
use rustc_hash::FxHashMap;

use crate::{
    app::{AppState, SessionState, StorageState},
    consts::TENANT_HEADER,
    env::observability as logger,
    errors::{self, ApiErrorContainer, SwitchError, ToContainerError},
};

pub type MultiTenant<T> = FxHashMap<TenantId, T>;

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct TenantId(String);

impl TenantId {
    pub fn new(val: String) -> Self {
        Self(val)
    }
}

#[derive(Clone)]
pub struct TenantState(pub Arc<SessionState>);

impl TenantState {
    pub fn new(session: Arc<SessionState>) -> Self {
        Self(session)
    }

    pub(crate) fn get_db_pool(&self) -> &StorageState {
        self.db_pool()
    }
}

impl std::ops::Deref for TenantState {
    type Target = Arc<SessionState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[axum::async_trait]
impl axum::extract::FromRequestParts<Arc<AppState>> for TenantState {
    type Rejection = ApiErrorContainer;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get(TENANT_HEADER)
            .ok_or_else(|| {
                logger::error!("Tenant header is missing from request");
                error_stack::Report::new(
                    errors::ApplicationErrorResponse::TenantIdNotFound,
                )
            })
            .and_then(|header| extract_tenant(state, header).switch())
            .to_container_error()
    }
}

fn extract_tenant(
    state: &AppState,
    header: &header::HeaderValue,
) -> errors::CustomResult<TenantState, errors::ParsingError> {
    let tenant = header
        .to_str()
        .change_context(errors::ParsingError::InvalidTenantId)?
        .to_string();

    state
        .tenant_states
        .get(&TenantId::new(tenant.clone()))
        .cloned()
        .ok_or_else(|| {
            logger::error!(tenant_id=%tenant, "Tenant ID not found in configured tenants");
            error_stack::Report::new(
                errors::ParsingError::TenantIdNotFound,
            )
        })
}
