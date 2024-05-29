use super::SwitchError;
use axum::response::{IntoResponse, Response};
use hyper::StatusCode;

use error_stack::ResultExt;

#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    #[error("Parsing failed with error {0}")]
    ParsingFailed(String),
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

#[derive(Debug, thiserror::Error)]
pub enum ApplicationErrorResponse {
    #[error("Something Went Wrong")]
    InternalServerError,
    #[error("The resource was not found {0}")]
    NotFound(&'static str),
    #[error("Invalid request provided {0}")]
    ParsingFailed(String),
    #[error("Wtf {0}")]
    Other(String),
}

impl<T> SwitchError<T, ApplicationErrorResponse> for super::CustomResult<T, ParsingError> {
    fn switch(self) -> super::CustomResult<T, ApplicationErrorResponse> {
        self.map_err(|err| match err.current_context() {
            ParsingError::ParsingFailed(s) => {
                error_stack::report!(ApplicationErrorResponse::ParsingFailed(s.to_string()))
            }
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
        self.change_context(ApplicationErrorResponse::InternalServerError)
    }
}

impl IntoResponse for ApplicationErrorResponse {
    fn into_response(self) -> Response {
        let body = match self {
            ApplicationErrorResponse::InternalServerError => "Something went wrong".to_string(),
            ApplicationErrorResponse::NotFound(s) => format!("The resource {s} was not found"),
            ApplicationErrorResponse::ParsingFailed(s) => s,
            ApplicationErrorResponse::Other(s) => s,
        };

        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}
