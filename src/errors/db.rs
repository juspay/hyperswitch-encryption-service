use crate::env::observability as logger;
use diesel::result::{DatabaseErrorKind, Error as diesel_error};
use error_stack::{report, ResultExt};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("Failed to get the connection out of the pool")]
    ConnectionEstablishFailed,
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to get the connection out of the pool")]
    ConnectionError(error_stack::Report<ConnectionError>),
    #[error("Failed to get the value from the database")]
    NotFound,
    #[error("Unique Violation occured")]
    UniqueViolation,
    #[error("Not null violation")]
    NotNullViolation,
    #[error("Invalid value found in the database")]
    InvalidValue,
    #[error("Other errors")]
    Others,
}

impl<T> super::SwitchError<T, DatabaseError> for super::CustomResult<T, ConnectionError> {
    fn switch(self) -> super::CustomResult<T, DatabaseError> {
        self.map_err(|err| report!(DatabaseError::ConnectionError(err)))
    }
}

impl<T> super::SwitchError<T, DatabaseError> for Result<T, diesel::result::Error> {
    fn switch(self) -> super::CustomResult<T, DatabaseError> {
        self.map_err(|err| match err {
            diesel_error::NotFound => report!(err).change_context(DatabaseError::NotFound),
            diesel_error::DatabaseError(db_error, _) => match db_error {
                DatabaseErrorKind::UniqueViolation => {
                    report!(err).change_context(DatabaseError::UniqueViolation)
                }
                DatabaseErrorKind::NotNullViolation => {
                    report!(err).change_context(DatabaseError::NotNullViolation)
                }
                _ => report!(err).change_context(DatabaseError::Others),
            },
            _ => report!(err).change_context(DatabaseError::Others),
        })
    }
}

impl<T> super::SwitchError<T, DatabaseError> for Result<T, charybdis::errors::CharybdisError> {
    fn switch(self) -> super::CustomResult<T, DatabaseError> {
        self.map_err(|err| {
            let (err, message) = match err {
                charybdis::errors::CharybdisError::NotFoundError(err) => {
                    (DatabaseError::NotFound, err)
                }
                err => {
                    logger::error!(err=?err);
                    (DatabaseError::Others, "An unknown error occurred")
                }
            };
            report!(err).attach_printable(message)
        })
    }
}
impl<T> super::SwitchError<T, DatabaseError> for super::CustomResult<T, super::CryptoError> {
    fn switch(self) -> super::CustomResult<T, super::DatabaseError> {
        self.change_context(DatabaseError::InvalidValue)
    }
}
