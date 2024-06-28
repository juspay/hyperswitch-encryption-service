use error_stack::report;

#[cfg(feature = "aws")]
use crate::env::observability as logger;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Provided Key is invalid")]
    InvalidKey,
    #[error("Failed to generate data key")]
    KeyGeneration,
    #[error("Failed to Get the data key")]
    KeyGetFailed,
    #[error("Failed encrypt the data using {0}")]
    EncryptionFailed(&'static str),
    #[error("Failed decrypt the data using {0}")]
    DecryptionFailed(&'static str),
    #[error("Unable to parse the stored Key {0}")]
    ParseError(String),
    #[error("Invalid value")]
    InvalidValue,
}

impl super::SwitchError<(), CryptoError> for Result<(), ring::error::Unspecified> {
    fn switch(self) -> super::CustomResult<(), CryptoError> {
        self.map_err(|err| report!(err).change_context(CryptoError::KeyGeneration))
    }
}

impl<T> super::SwitchError<T, CryptoError> for super::CustomResult<T, super::ParsingError> {
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|err| err.change_context(CryptoError::InvalidValue))
    }
}

impl<T> super::SwitchError<T, CryptoError> for super::CustomResult<T, super::DatabaseError> {
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|err| err.change_context(CryptoError::KeyGetFailed))
    }
}

impl<T> super::SwitchError<T, CryptoError> for Result<T, strum::ParseError> {
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|err| report!(err).change_context(CryptoError::ParseError(err.to_string())))
    }
}

#[cfg(feature = "aws")]
impl<T, U: core::fmt::Debug> super::SwitchError<T, CryptoError>
    for Result<T, aws_sdk_kms::error::SdkError<aws_sdk_kms::operation::encrypt::EncryptError, U>>
{
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|err| {
            logger::error!(aws_kms_err=?err);
            report!(CryptoError::EncryptionFailed("KMS"))
        })
    }
}

#[cfg(feature = "aws")]
impl<T, U: core::fmt::Debug> super::SwitchError<T, CryptoError>
    for Result<T, aws_sdk_kms::error::SdkError<aws_sdk_kms::operation::decrypt::DecryptError, U>>
{
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|err| {
            logger::error!(aws_kms_err=?err);
            report!(CryptoError::DecryptionFailed("KMS"))
        })
    }
}

#[cfg(feature = "aws")]
impl<T, U: core::fmt::Debug> super::SwitchError<T, CryptoError>
    for Result<
        T,
        aws_sdk_kms::error::SdkError<
            aws_sdk_kms::operation::generate_data_key::GenerateDataKeyError,
            U,
        >,
    >
{
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|err| {
            logger::error!(aws_kms_err=?err);
            report!(CryptoError::KeyGeneration)
        })
    }
}
