use super::SwitchError;
use axum::response::{IntoResponse, Response};
use hyper::StatusCode;

pub type ApiResponseResult<T> = Result<T, ApiErrorContainer>;

pub struct ApiErrorContainer {
    pub error: error_stack::Report<ApplicationErrorResponse>,
}

mod error_codes {
    pub const IE_00: &str = "IE_00";
    pub const BR_00: &str = "BR_00";
    pub const NF_00: &str = "NF_00";
}

#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    #[error("Tenant ID that was passed was invalid")]
    InvalidTenantId,
    #[error("Could not find any tenant with the passed tenant id")]
    TenantIdNotFound,
    #[error("Parsing failed with error {0}")]
    ParsingFailed(String),
    #[error("Decoding failed with error {0}")]
    DecodingFailed(String),
}

pub trait ToContainerError<T> {
    fn to_container_error(self) -> Result<T, ApiErrorContainer>;
}

impl<T> SwitchError<T, ParsingError> for Result<T, T::Err>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: core::fmt::Display,
{
    fn switch(self) -> super::CustomResult<T, ParsingError> {
        self.map_err(|err| error_stack::report!(ParsingError::ParsingFailed(err.to_string())))
    }
}

#[derive(serde::Serialize)]
struct ApiErrorResponse<'a> {
    error_code: &'a str,
    error_message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationErrorResponse {
    #[error("Internal Server Error Occurred - {0}")]
    InternalServerError(&'static str),
    #[error("The resource was not found in the {0}")]
    NotFound(&'static str),
    #[error("Invalid request provided {0}")]
    ParsingFailed(String),
    #[error(
        "Unique violation occurred. Please try to create the data with another key/identifier"
    )]
    UniqueViolation,
    #[error("Authentication failed")]
    Unauthorized,
    #[error("Tenant ID that was passed was not configured")]
    TenantIDNotFound,
    #[error("Tenant ID which was passed in the headers was invalid")]
    InvalidTenantId,
}

impl<T> SwitchError<T, ApplicationErrorResponse> for super::CustomResult<T, ParsingError> {
    fn switch(self) -> super::CustomResult<T, ApplicationErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                ParsingError::ParsingFailed(s) => {
                    ApplicationErrorResponse::ParsingFailed(s.to_string())
                }
                ParsingError::DecodingFailed(s) => {
                    ApplicationErrorResponse::ParsingFailed(s.to_string())
                }
                ParsingError::TenantIdNotFound => ApplicationErrorResponse::TenantIDNotFound,
                ParsingError::InvalidTenantId => ApplicationErrorResponse::InvalidTenantId,
            };
            err.change_context(new_err)
        })
    }
}

impl<T> SwitchError<T, ApplicationErrorResponse> for super::CustomResult<T, super::CryptoError> {
    fn switch(self) -> super::CustomResult<T, ApplicationErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                super::CryptoError::EncryptionFailed(_) => {
                    ApplicationErrorResponse::InternalServerError("Encryption failed")
                }
                super::CryptoError::DecryptionFailed(_) => {
                    ApplicationErrorResponse::InternalServerError("Decryption failed")
                }
                super::CryptoError::KeyGeneration => {
                    ApplicationErrorResponse::InternalServerError("Key generation failed")
                }
                super::CryptoError::InvalidKey => {
                    ApplicationErrorResponse::InternalServerError("Invalid key detected")
                }
                super::CryptoError::KeyGetFailed => {
                    ApplicationErrorResponse::InternalServerError("Failed to get the key")
                }
                super::CryptoError::AuthenticationFailed => ApplicationErrorResponse::Unauthorized,
                _ => ApplicationErrorResponse::InternalServerError("Unexpected error occurred"),
            };
            err.change_context(new_err)
        })
    }
}

impl<T> SwitchError<T, ApplicationErrorResponse> for super::CustomResult<T, super::DatabaseError> {
    fn switch(self) -> super::CustomResult<T, ApplicationErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                super::DatabaseError::NotFound => ApplicationErrorResponse::NotFound("Database"),
                super::DatabaseError::ConnectionError(_)
                | super::DatabaseError::NotNullViolation
                | super::DatabaseError::InvalidValue
                | super::DatabaseError::Others => {
                    ApplicationErrorResponse::InternalServerError("Database error occurred")
                }
                super::DatabaseError::UniqueViolation => ApplicationErrorResponse::UniqueViolation,
            };
            err.change_context(new_err)
        })
    }
}

impl<T> ToContainerError<T> for super::CustomResult<T, ApplicationErrorResponse> {
    fn to_container_error(self) -> Result<T, ApiErrorContainer> {
        self.map_err(|error| ApiErrorContainer { error })
    }
}

impl IntoResponse for ApiErrorContainer {
    fn into_response(self) -> Response {
        match self.error.current_context() {
            err @ ApplicationErrorResponse::InternalServerError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ApiErrorResponse {
                    error_message: err.to_string(),
                    error_code: error_codes::IE_00,
                }),
            ),
            err @ ApplicationErrorResponse::NotFound(_) => (
                StatusCode::NOT_FOUND,
                axum::Json(ApiErrorResponse {
                    error_message: err.to_string(),
                    error_code: error_codes::NF_00,
                }),
            ),
            err @ ApplicationErrorResponse::ParsingFailed(_) => (
                StatusCode::BAD_REQUEST,
                axum::Json(ApiErrorResponse {
                    error_message: err.to_string(),
                    error_code: error_codes::BR_00,
                }),
            ),
            err @ ApplicationErrorResponse::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                axum::Json(ApiErrorResponse {
                    error_message: err.to_string(),
                    error_code: error_codes::BR_00,
                }),
            ),
            err @ ApplicationErrorResponse::UniqueViolation => (
                StatusCode::BAD_REQUEST,
                axum::Json(ApiErrorResponse {
                    error_message: err.to_string(),
                    error_code: error_codes::BR_00,
                }),
            ),
            err @ ApplicationErrorResponse::TenantIDNotFound => (
                StatusCode::BAD_REQUEST,
                axum::Json(ApiErrorResponse {
                    error_message: err.to_string(),
                    error_code: error_codes::BR_00,
                }),
            ),
            err @ ApplicationErrorResponse::InvalidTenantId => (
                StatusCode::BAD_REQUEST,
                axum::Json(ApiErrorResponse {
                    error_message: err.to_string(),
                    error_code: error_codes::BR_00,
                }),
            ),
        }
        .into_response()
    }
}
