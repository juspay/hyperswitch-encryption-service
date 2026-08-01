use std::str::FromStr;

use error_stack::IntoReport;
use hyperswitch_masking::PeekInterface;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    crypto::{Crypto, Source, aes256::GcmAes256},
    env::metrics,
    errors::{self, SwitchError},
    multitenancy::TenantState,
    storage::types::{DataKey, DataKeyNew},
    types::{
        DecryptedData, DecryptedDataGroup, EncryptedData, EncryptedDataGroup, Identifier, Key,
        MultipleDecryptionDataGroup, MultipleEncryptionDataGroup, key::Version,
    },
};

#[async_trait::async_trait]
pub trait KeyEncrypter<ToType> {
    async fn encrypt(
        self,
        state: &TenantState,
    ) -> errors::CustomResult<ToType, errors::CryptoError>;
}

#[async_trait::async_trait]
pub trait KeyDecrypter<ToType> {
    async fn decrypt(
        self,
        state: &TenantState,
    ) -> errors::CustomResult<ToType, errors::CryptoError>;
}

#[async_trait::async_trait]
impl KeyEncrypter<DataKeyNew> for Key {
    async fn encrypt(
        self,
        state: &TenantState,
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
impl KeyDecrypter<Key> for DataKey {
    async fn decrypt(self, state: &TenantState) -> errors::CustomResult<Key, errors::CryptoError> {
        let decrypted_key = state
            .keymanager_client
            .decrypt_key(self.encryption_key)
            .await?;

        let decrypted_key = <[u8; 32]>::try_from(decrypted_key.peek().to_vec())
            .map_err(|_| errors::CryptoError::DecryptionFailed("KMS").into_report())?;

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
pub trait DataEncrypter<ToType> {
    async fn encrypt(
        self,
        state: &TenantState,
        identifier: &Identifier,
    ) -> errors::CustomResult<ToType, errors::CryptoError>;
}

#[async_trait::async_trait]
pub trait DataDecrypter<ToType> {
    async fn decrypt(
        self,
        state: &TenantState,
        identifier: &Identifier,
    ) -> errors::CustomResult<ToType, errors::CryptoError>;
}

#[async_trait::async_trait]
impl DataEncrypter<MultipleEncryptionDataGroup> for MultipleDecryptionDataGroup {
    async fn encrypt(
        self,
        state: &TenantState,
        identifier: &Identifier,
    ) -> errors::CustomResult<MultipleEncryptionDataGroup, errors::CryptoError> {
        let version = Version::get_latest(identifier, state).await;
        let decrypted_key = Key::get_key(state, identifier, version).await.switch()?;

        let key = GcmAes256::new(decrypted_key.key)?;
        let chunk_size = std::cmp::max(self.0.len() / state.thread_pool.current_num_threads(), 1);

        // Helper closure to encrypt a single DecryptedDataGroup into an EncryptedDataGroup.
        let encrypt_data_group = |group: DecryptedDataGroup| -> errors::CustomResult<
            EncryptedDataGroup,
            errors::CryptoError,
        > {
            group
                .0
                .into_par_iter()
                .map(|(hash_key, data)| {
                    let input = data.inner();
                    let input_size = input.peek().len();
                    let encrypted_data = record_data_crypto(
                        || key.encrypt(input),
                        metrics::DataCryptoOperation::Encrypt,
                        input_size,
                    )?;

                    Ok((
                        hash_key,
                        EncryptedData {
                            version: decrypted_key.version,
                            data: encrypted_data,
                        },
                    ))
                })
                .collect::<errors::CustomResult<FxHashMap<_, _>, _>>()
                .map(EncryptedDataGroup)
        };

        let multiple_groups = state.thread_pool.install(|| {
            self.0
                .into_par_iter()
                .chunks(chunk_size)
                .map(|chunk| {
                    // Encrypt each group within the chunk.
                    let groups = chunk
                        .into_par_iter()
                        .map(encrypt_data_group)
                        .collect::<errors::CustomResult<Vec<_>, _>>()?;
                    Ok(MultipleEncryptionDataGroup(groups))
                })
                .collect::<errors::CustomResult<Vec<_>, _>>()
        })?;

        // "Unchunking" all encrypted groups
        let all_encrypted_groups = multiple_groups
            .into_iter()
            .flat_map(|group| group.0)
            .collect();

        Ok(MultipleEncryptionDataGroup(all_encrypted_groups))
    }
}

#[async_trait::async_trait]
impl DataDecrypter<MultipleDecryptionDataGroup> for MultipleEncryptionDataGroup {
    async fn decrypt(
        self,
        state: &TenantState,
        identifier: &Identifier,
    ) -> errors::CustomResult<MultipleDecryptionDataGroup, errors::CryptoError> {
        let versions = self
            .0
            .iter()
            .flat_map(|group| group.0.values().map(|data| data.version))
            .collect::<FxHashSet<_>>();

        let decrypted_keys = Key::get_multiple_keys(state, identifier, versions)
            .await
            .switch()?;

        let chunk_size = std::cmp::max(self.0.len() / state.thread_pool.current_num_threads(), 1);

        // Helper closure to decrypt a single entity from an encrypted group.
        let decrypt_entity = |(hash_key, data): (String, EncryptedData)| -> errors::CustomResult<(String, DecryptedData), _> {
            let version = data.version;
            let decrypted_key = decrypted_keys.get(&version)
            .ok_or_else(|| errors::CryptoError::DecryptionFailed("AES").into_report())?;
            let key = GcmAes256::new(decrypted_key.key.clone())?;
            let input = data.inner();
            let input_size = input.peek().len();
            let decrypted_data = record_data_crypto(
                || key.decrypt(input),
                metrics::DataCryptoOperation::Decrypt,
                input_size,
            )?;

            Ok((hash_key, DecryptedData::from_data(decrypted_data)))
        };

        // Helper closure to decrypt an entire group.
        let decrypt_group =
            |encrypted_group: EncryptedDataGroup| -> errors::CustomResult<DecryptedDataGroup, _> {
                let decrypted_entities = encrypted_group
                    .0
                    .into_par_iter()
                    .map(decrypt_entity)
                    .collect::<errors::CustomResult<FxHashMap<_, _>, _>>()?;
                Ok(DecryptedDataGroup(decrypted_entities))
            };

        // Process groups in parallel in chunks.
        let multiple_groups = state.thread_pool.install(|| {
            self.0
                .into_par_iter()
                .chunks(chunk_size)
                .map(|chunk| {
                    chunk
                        .into_par_iter()
                        .map(decrypt_group)
                        .collect::<errors::CustomResult<Vec<_>, _>>()
                        .map(MultipleDecryptionDataGroup)
                })
                .collect::<errors::CustomResult<Vec<_>, _>>()
        })?;

        // "Unchunk" all decrypted groups.
        let all_decrypted_groups = multiple_groups
            .into_iter()
            .flat_map(|group| group.0)
            .collect();

        Ok(MultipleDecryptionDataGroup(all_decrypted_groups))
    }
}

#[async_trait::async_trait]
impl DataEncrypter<EncryptedDataGroup> for DecryptedDataGroup {
    async fn encrypt(
        self,
        state: &TenantState,
        identifier: &Identifier,
    ) -> errors::CustomResult<EncryptedDataGroup, errors::CryptoError> {
        let version = Version::get_latest(identifier, state).await;
        let decrypted_key = Key::get_key(state, identifier, version).await.switch()?;
        let key = GcmAes256::new(decrypted_key.key)?;

        state.thread_pool.install(|| {
            self.0
                .into_par_iter()
                .map(|(hash_key, data)| {
                    let input = data.inner();
                    let input_size = input.peek().len();
                    let encrypted_data = record_data_crypto(
                        || key.encrypt(input),
                        metrics::DataCryptoOperation::Encrypt,
                        input_size,
                    )?;

                    Ok::<_, error_stack::Report<errors::CryptoError>>((hash_key,EncryptedData {
                        version: decrypted_key.version,
                        data: encrypted_data,
                    }))
                })
                .collect::<errors::CustomResult<FxHashMap<String, EncryptedData>,errors::CryptoError>>()
        }).map(EncryptedDataGroup)
    }
}

#[async_trait::async_trait]
impl DataDecrypter<DecryptedDataGroup> for EncryptedDataGroup {
    async fn decrypt(
        self,
        state: &TenantState,
        identifier: &Identifier,
    ) -> errors::CustomResult<DecryptedDataGroup, errors::CryptoError> {
        let version = FxHashSet::from_iter(self.0.values().map(|d| d.version));
        let decrypted_keys = Key::get_multiple_keys(state, identifier, version)
            .await
            .switch()?;

        state
            .thread_pool
            .install(|| {
                self
            .0
            .into_par_iter()
            .map(|(hash_key, data)| {
                let version = data.version;
                let decrypted_key = decrypted_keys
                    .get(&version)
                    .ok_or(errors::CryptoError::DecryptionFailed("AES").into_report())?.clone();

                let key = GcmAes256::new(decrypted_key.key)?;
                let input = data.inner();
                let input_size = input.peek().len();
                let decrypted_data = record_data_crypto(
                    || key.decrypt(input),
                    metrics::DataCryptoOperation::Decrypt,
                    input_size,
                )?;

                Ok::<_, error_stack::Report<errors::CryptoError>>((
                    hash_key,
                    DecryptedData::from_data(decrypted_data),
                ))
            })
            .collect::<errors::CustomResult<FxHashMap<String, DecryptedData>, errors::CryptoError>>(
            )
            })
            .map(DecryptedDataGroup)
    }
}

#[async_trait::async_trait]
impl DataEncrypter<EncryptedData> for DecryptedData {
    async fn encrypt(
        self,
        state: &TenantState,
        identifier: &Identifier,
    ) -> errors::CustomResult<EncryptedData, errors::CryptoError> {
        let version = Version::get_latest(identifier, state).await;
        let decrypted_key = Key::get_key(state, identifier, version).await.switch()?;

        let key = GcmAes256::new(decrypted_key.key)?;

        let input = self.inner();
        let input_size = input.peek().len();
        let encrypted_data = record_data_crypto(
            || key.encrypt(input),
            metrics::DataCryptoOperation::Encrypt,
            input_size,
        )?;

        Ok(EncryptedData {
            version: decrypted_key.version,
            data: encrypted_data,
        })
    }
}

#[async_trait::async_trait]
impl DataDecrypter<DecryptedData> for EncryptedData {
    async fn decrypt(
        self,
        state: &TenantState,
        identifier: &Identifier,
    ) -> errors::CustomResult<DecryptedData, errors::CryptoError> {
        let version = self.version;
        let decrypted_key = Key::get_key(state, identifier, version).await.switch()?;

        let key = GcmAes256::new(decrypted_key.key)?;

        let input = self.inner();
        let input_size = input.peek().len();
        let decrypted_data = record_data_crypto(
            || key.decrypt(input),
            metrics::DataCryptoOperation::Decrypt,
            input_size,
        )?;

        Ok(DecryptedData::from_data(decrypted_data))
    }
}

/// Maps an input length in bytes to a bounded bucket label.
///
/// Buckets are power-of-two ranges `(prev_upper, upper]`, the written upper bound is inclusive:
/// `8-16B` covers 9..=16 bytes, `512B-1KiB` covers 513..=1024.
///
/// The bucket index is `ceil(log2(len))` (computed as `len.next_power_of_two().trailing_zeros()`,
/// e.g. 9..=16 -> 16 -> `trailing_zeros` 4 -> index 1), shifted down by 3 so the `0-8B` bucket
/// starts at index 0.
/// Values above 1 MiB collapse into a single `>1MiB` bucket, keeping label cardinality bounded
/// regardless of input size.
const fn size_bucket(len: usize) -> &'static str {
    const BUCKETS: [&str; 18] = [
        "0-8B",
        "8-16B",
        "16-32B",
        "32-64B",
        "64-128B",
        "128-256B",
        "256-512B",
        "512B-1KiB",
        "1-2KiB",
        "2-4KiB",
        "4-8KiB",
        "8-16KiB",
        "16-32KiB",
        "32-64KiB",
        "64-128KiB",
        "128-256KiB",
        "256-512KiB",
        "512KiB-1MiB",
    ];

    if len > 1024 * 1024 {
        return ">1MiB";
    }

    #[expect(
        clippy::as_conversions,
        reason = "u32 to usize cast should succeed on 64-bit targets"
    )]
    BUCKETS[len.next_power_of_two().trailing_zeros().saturating_sub(3) as usize]
}

fn record_data_crypto<T, E>(
    f: impl FnOnce() -> Result<T, E>,
    operation: metrics::DataCryptoOperation,
    input_size: usize,
) -> Result<T, E> {
    let start = std::time::Instant::now();
    let result = f();
    let duration = start.elapsed();
    let outcome = if result.is_ok() { "success" } else { "error" };

    metrics::CRYPTO_OPERATION_DURATION.record(
        duration.as_secs_f64(),
        metrics_utils::metric_attributes!(
            ("operation", operation),
            ("input_size", size_bucket(input_size)),
            ("outcome", outcome),
        ),
    );

    result
}

#[cfg(test)]
mod tests {
    use super::size_bucket;

    #[test]
    fn test_size_bucket_boundaries() {
        assert_eq!(size_bucket(0), "0-8B");
        assert_eq!(size_bucket(1), "0-8B");
        assert_eq!(size_bucket(8), "0-8B");
        assert_eq!(size_bucket(9), "8-16B");
        assert_eq!(size_bucket(16), "8-16B");
        assert_eq!(size_bucket(1024), "512B-1KiB");
        assert_eq!(size_bucket(1025), "1-2KiB");
        assert_eq!(size_bucket(2048), "1-2KiB");
        assert_eq!(size_bucket(2049), "2-4KiB");
        assert_eq!(size_bucket(524288), "256-512KiB");
        assert_eq!(size_bucket(1024 * 1024), "512KiB-1MiB");
        assert_eq!(size_bucket(1024 * 1024 + 1), ">1MiB");
        assert_eq!(size_bucket(usize::MAX), ">1MiB");
    }
}
