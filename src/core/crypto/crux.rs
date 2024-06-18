use masking::PeekInterface;
use rayon::prelude::*;

use rustc_hash::{FxHashMap, FxHashSet};
use std::str::FromStr;

use crate::{
    app::AppState,
    crypto::{aes256::GcmAes256, Crypto, KeyManagement, Source},
    errors::{self, SwitchError},
    storage::types::{DataKey, DataKeyNew},
    types::{
        key::Version, DecryptedData, DecryptedDataGroup, EncryptedData, EncryptedDataGroup,
        Identifier, Key,
    },
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
            .keymanager_client
            .encrypt_key(self.key.peek().to_vec().into())
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
        let decrypted_key = state
            .keymanager_client
            .decrypt_key(self.encryption_key)
            .await?;

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
impl DataEncrypt<EncryptedDataGroup> for DecryptedDataGroup {
    async fn encrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
    ) -> errors::CustomResult<EncryptedDataGroup, errors::CryptoError> {
        let version = Version::get_latest(identifier, state).await;
        let decrypted_key = Key::get_key(state, identifier, version).await.switch()?;
        let key = GcmAes256::new(decrypted_key.key)?;

        Ok(EncryptedDataGroup(
            self.0
                .into_par_iter()
                .map(|(hash_key, data)| {
                    let encrypted_data = key.encrypt(data.inner())?;
                    Ok::<_, error_stack::Report<errors::CryptoError>>((hash_key,EncryptedData {
                        version: decrypted_key.version,
                        data: encrypted_data,
                    }))
                })
                .collect::<errors::CustomResult<FxHashMap<String, EncryptedData>,errors::CryptoError>>()?,
        ))
    }
}

#[async_trait::async_trait]
impl DataDecrypt<DecryptedDataGroup> for EncryptedDataGroup {
    async fn decrypt(
        self,
        state: &AppState,
        identifier: &Identifier,
    ) -> errors::CustomResult<DecryptedDataGroup, errors::CryptoError> {
        let version = FxHashSet::from_iter(self.0.values().map(|d| d.version));
        let decrypted_keys = Key::get_multiple_keys(state, identifier, version)
            .await
            .switch()?;

        Ok(DecryptedDataGroup(self
            .0
            .into_par_iter()
            .map(|(hash_key, data)| {
                let version = data.version;
                let decrypted_key = decrypted_keys
                    .get(&version)
                    .ok_or(error_stack::report!(errors::CryptoError::DecryptionFailed("AES")))?.clone();

                let key = GcmAes256::new(decrypted_key.key)?;
                let decrypted_data = key.decrypt(data.inner())?;
                Ok::<_, error_stack::Report<errors::CryptoError>>((
                    hash_key,
                    DecryptedData::from_data(decrypted_data),
                ))
            })
            .collect::<errors::CustomResult<FxHashMap<String, DecryptedData>, errors::CryptoError>>(
            )?))
    }
}
