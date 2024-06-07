use super::DbState;
use crate::{
    errors::{self, CustomResult, SwitchError},
    schema::data_key_store::*,
    storage::types::{DataKey, DataKeyNew},
    types::{key::Version, Identifier},
};
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

#[async_trait::async_trait]
pub trait DataKeyStorageInterface {
    async fn insert_data_key(
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
}

#[async_trait::async_trait]
impl DataKeyStorageInterface for DbState {
    async fn insert_data_key(
        &self,
        new: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;
        let query = diesel::insert_into(DataKey::table()).values(new);

        query.get_result(&mut connection).await.switch()
    }

    async fn get_latest_version(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Version, errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;

        let (d_id, k_id) = identifier.get_identifier();
        let query = DataKey::table()
            .select(version)
            .order_by(version.desc())
            .filter(data_identifier.eq(d_id).and(key_identifier.eq(k_id)));

        query.get_result(&mut connection).await.switch()
    }

    async fn get_key(
        &self,
        v: Version,
        identifier: &Identifier,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;

        let (d_id, k_id) = identifier.get_identifier();

        let query = DataKey::table().filter(
            version
                .eq(v)
                .and(data_identifier.eq(d_id).and(key_identifier.eq(k_id))),
        );
        query.get_result(&mut connection).await.switch()
    }
}
