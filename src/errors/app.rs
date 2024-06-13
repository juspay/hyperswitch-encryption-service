use super::SwitchError;
use axum::response::{IntoResponse, Response};
use hyper::StatusCode;

use error_stack::ResultExt;

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
    #[error("Parsing failed with error {0}")]
    ParsingFailed(String),
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
    #[error("Something Went Wrong")]
    InternalServerError,
    #[error("The resource was not found in the {0}")]
    NotFound(&'static str),
    #[error("Invalid request provided {0}")]
    ParsingFailed(String),
    #[error("Unique violation occured. Please try to create the data with another key/identifier")]
    UniqueViolation,
}

impl<T> SwitchError<T, ApplicationErrorResponse> for super::CustomResult<T, ParsingError> {
    fn switch(self) -> super::CustomResult<T, ApplicationErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                ParsingError::ParsingFailed(s) => {
                    ApplicationErrorResponse::ParsingFailed(s.to_string())
                }
            };
            err.change_context(new_err)
        })
    }
}

impl<T> SwitchError<T, ApplicationErrorResponse> for super::CustomResult<T, super::CryptoError> {
    fn switch(self) -> super::CustomResult<T, ApplicationErrorResponse> {
        self.change_context(ApplicationErrorResponse::InternalServerError)
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
                | super::DatabaseError::Others => ApplicationErrorResponse::InternalServerError,
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
            err @ ApplicationErrorResponse::InternalServerError => (
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
            err @ ApplicationErrorResponse::UniqueViolation => (
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
