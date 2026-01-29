#[cfg(feature = "aws")]
use crate::storage::types::{ListKeyInfo, UpdateReEncryptedKey};
use crate::{
    crypto::Source,
    errors::{self, CustomResult},
    storage::types::{DataKey, DataKeyNew},
    types::{Identifier, key::Version},
};

#[async_trait::async_trait]
pub trait DataKeyStorageInterface {
    async fn get_or_insert_data_key(
        &self,
        new: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError>;
    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError>;
    async fn get_key(
        &self,
        v: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError>;
    async fn get_keys_by_filter(
        &self,
        key_source: Option<Source>,
    ) -> CustomResult<Vec<DataKey>, errors::DatabaseError>;
    #[cfg(feature = "aws")]
    async fn get_keys_by_unique_index(
        &self,
        key_infos: Option<&Vec<ListKeyInfo>>,
    ) -> CustomResult<Vec<DataKey>, errors::DatabaseError>;
    #[cfg(feature = "aws")]
    async fn update_key(
        &self,
        key: &UpdateReEncryptedKey,
    ) -> CustomResult<(), errors::DatabaseError>;
}
