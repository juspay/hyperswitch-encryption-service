use error_stack::report;

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
    #[error("Invalid value")]
    InvalidValue,
}

impl super::SwitchError<(), CryptoError> for Result<(), ring::error::Unspecified> {
    fn switch(self) -> super::CustomResult<(), CryptoError> {
        self.map_err(|_| report!(CryptoError::KeyGeneration))
    }
}

impl<T> super::SwitchError<T, CryptoError> for super::CustomResult<T, super::ParsingError> {
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|_| report!(CryptoError::InvalidValue))
    }
}

impl<T> super::SwitchError<T, CryptoError> for super::CustomResult<T, super::DatabaseError> {
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|_| report!(CryptoError::KeyGetFailed))
    }
}

#[cfg(feature = "aws")]
impl<T, U> super::SwitchError<T, CryptoError>
    for Result<T, aws_sdk_kms::error::SdkError<aws_sdk_kms::operation::encrypt::EncryptError, U>>
{
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|_| report!(CryptoError::EncryptionFailed("KMS")))
    }
}

#[cfg(feature = "aws")]
impl<T, U> super::SwitchError<T, CryptoError>
    for Result<T, aws_sdk_kms::error::SdkError<aws_sdk_kms::operation::decrypt::DecryptError, U>>
{
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|_| report!(CryptoError::DecryptionFailed("KMS")))
    }
}

#[cfg(feature = "aws")]
impl<T, U> super::SwitchError<T, CryptoError>
    for Result<
        T,
        aws_sdk_kms::error::SdkError<
            aws_sdk_kms::operation::generate_data_key::GenerateDataKeyError,
            U,
        >,
    >
{
    fn switch(self) -> super::CustomResult<T, CryptoError> {
        self.map_err(|_| report!(CryptoError::KeyGeneration))
    }
}
