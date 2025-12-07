use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request};
use base64::Engine;
use error_stack::{ResultExt, ensure};
use hyper::header;
use masking::{PeekInterface, Secret, StrongSecret};

use crate::{
    app::AppState,
    consts::base64::BASE64_ENGINE,
    errors::{ApiErrorContainer, CustomResult, ParsingError, SwitchError, ToContainerError},
    multitenancy::TenantState,
};

#[derive(Clone)]
pub struct Custodian {
    pub keys: Option<(StrongSecret<String>, StrongSecret<String>)>,
}

impl Custodian {
    pub fn new(keys: Option<(String, String)>) -> Self {
        let keys = keys.map(|(key1, key2)| (StrongSecret::new(key1), StrongSecret::new(key2)));
        Self { keys }
    }

    pub fn into_access_token(self, state: &TenantState) -> Option<StrongSecret<String>> {
        self.keys
            .map(|(x, y)| format!("{}:{}", x.peek(), y.peek()))
            .map(|token| state.hash_client.hash(Secret::new(token)))
            .map(hex::encode)
            .map(StrongSecret::new)
    }
}

#[axum::async_trait]
impl FromRequestParts<Arc<AppState>> for Custodian {
    type Rejection = ApiErrorContainer;
    async fn from_request_parts(
        parts: &mut request::Parts,
        _state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get(header::AUTHORIZATION)
            .map(extract_credential)
            .transpose()
            .switch()
            .to_container_error()
            .map(Self::new)
    }
}

fn extract_credential(
    header: &header::HeaderValue,
) -> CustomResult<(String, String), ParsingError> {
    let header = header.to_str().change_context(ParsingError::ParsingFailed(
        "Failed while converting header to string".to_string(),
    ))?;

    let credential = header
        .strip_prefix("Basic ")
        .ok_or(ParsingError::ParsingFailed(
            "Authorization scheme is not basic".to_string(),
        ))?;
    let credential = credential.trim();
    let credential =
        BASE64_ENGINE
            .decode(credential)
            .change_context(ParsingError::DecodingFailed(
                "Failed while base64 decoding the authorization header".to_string(),
            ))?;
    let credential = String::from_utf8(credential).change_context(ParsingError::DecodingFailed(
        "Failed while converting base64 to utf8".to_string(),
    ))?;
    let mut parts = credential.split(':');
    let key1 = parts.next().ok_or(ParsingError::ParsingFailed(
        "Failed while extracting key1 from credential".to_string(),
    ))?;
    let key2 = parts.next().ok_or(ParsingError::ParsingFailed(
        "Failed while extracting key2 from credential".to_string(),
    ))?;

    ensure!(
        parts.next().is_none(),
        ParsingError::ParsingFailed("Credential has more than 2 parts".to_string())
    );

    Ok((key1.to_string(), key2.to_string()))
}
