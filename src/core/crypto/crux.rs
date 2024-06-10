use masking::PeekInterface;
use std::str::FromStr;

use crate::{
    app::AppState,
    crypto::{aes256::GcmAes256, Crypto, Source},
    errors::{self, SwitchError},
    storage::types::{DataKey, DataKeyNew},
    types::{key::Version, DecryptedData, EncryptedData, Identifier, Key},
};

#[async_trait::async_trait]
pub trait KeyEncrypt<ToType> {
    async fn encrypt(self, state: &AppState) -> errors::CustomResult<ToType, errors::CryptoError>;
}

#[async_trait::async_trait]
pub trait KeyDecrypt<ToType> {
    async fn decrypt(self, state: &AppState) -> errors::CustomResult<ToType, errors::CryptoError>;
}

#[async_trait::async_trait]
impl KeyEncrypt<DataKeyNew> for Key {
    async fn encrypt(
        self,
        state: &AppState,
    ) -> errors::CustomResult<DataKeyNew, errors::CryptoError> {
        let encryption_key = state
            .encryption_client
            .encrypt(self.key.peek().to_vec().into())
            .await?;

        let (data_identifier, key_identifier) = self.identifier.get_identifier();
        Ok(DataKeyNew {
            data_identifier,
            key_identifier,
            encryption_key,
            version: self.version,
            source: self.source.to_string(),
            created_at: time::PrimitiveDateTime::new(
                time::OffsetDateTime::now_utc().date(),
                time::OffsetDateTime::now_utc().time(),
            ),
        })
    }
}

#[async_trait::async_trait]
impl KeyDecrypt<Key> for DataKey {
    async fn decrypt(self, state: &AppState) -> errors::CustomResult<Key, errors::CryptoError> {
        let decrypted_key = state.encryption_client.decrypt(self.encryption_key).await?;

        let decrypted_key = <[u8; 32]>::try_from(decrypted_key.peek().to_vec())
            .map_err(|_| error_stack::report!(errors::CryptoError::DecryptionFailed("KMS")))?;

        let identifier: errors::CustomResult<Identifier, errors::ParsingError> =
            (self.data_identifier, self.key_identifier).try_into();

        let source = Source::from_str(&self.source).switch()?;
        Ok(Key {
            identifier: identifier.switch()?,
            version: self.version,
            key: decrypted_key.into(),
            source,
        })
    }
}

#[async_trait::async_trait]
pub trait DataEncrypt<ToType> {
    async fn encrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
    ) -> errors::CustomResult<ToType, errors::CryptoError>;
}

#[async_trait::async_trait]
pub trait DataDecrypt<ToType> {
    async fn decrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
    ) -> errors::CustomResult<ToType, errors::CryptoError>;
}

#[async_trait::async_trait]
impl DataEncrypt<EncryptedData> for DecryptedData {
    async fn encrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
    ) -> errors::CustomResult<EncryptedData, errors::CryptoError> {
        let version = Version::get_latest(identifier, state).await;
        let decrypted_key = Key::get_key(state, identifier, version).await.switch()?;
        let key = GcmAes256::new(decrypted_key.key).await?;

        let encrypted_data = key.encrypt(self.inner()).await?;

        Ok(EncryptedData {
            version: decrypted_key.version,
            data: encrypted_data,
        })
    }
}

#[async_trait::async_trait]
impl DataDecrypt<DecryptedData> for EncryptedData {
    async fn decrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
    ) -> errors::CustomResult<DecryptedData, errors::CryptoError> {
        let version = self.version;
        let decrypted_key = Key::get_key(state, identifier, version).await.switch()?;
        let key = GcmAes256::new(decrypted_key.key).await?;

        let decrypted_data = key.decrypt(self.inner()).await?;

        Ok(DecryptedData::from_data(decrypted_data))
    }
}
