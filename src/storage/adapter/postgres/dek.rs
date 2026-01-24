use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, associations::HasTable};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::Pool};
use error_stack::ResultExt;

use super::DbState;
use crate::{
    crypto::Source as KeySource, errors::{self, CustomResult, SwitchError}, schema::data_key_store::*, storage::{
        adapter::PostgreSQL,
        dek::DataKeyStorageInterface,
        types::{DataKey, DataKeyNew},
    }, types::{Identifier, key::Version}
};

#[async_trait::async_trait]
impl DataKeyStorageInterface for DbState<Pool<AsyncPgConnection>, PostgreSQL> {
    async fn get_or_insert_data_key(
        &self,
        new: DataKeyNew,
    ) -> CustomResult<DataKey, errors::DatabaseError> {
        let identifier: errors::CustomResult<Identifier, errors::ParsingError> =
            (new.data_identifier.clone(), new.key_identifier.clone()).try_into();

        let v = new.version;

        let mut connection = self.get_conn().await.switch()?;
        let query = diesel::insert_into(DataKey::table()).values(new);

        match query.get_result(&mut connection).await.switch() {
            Ok(result) => Ok(result),
            Err(err) => match err.current_context() {
                errors::DatabaseError::UniqueViolation => {
                    self.get_key(
                        v,
                        &identifier
                            .change_context(errors::DatabaseError::Others)
                            .attach_printable("Failed to parse identifier")?,
                    )
                    .await
                }
                _ => Err(err),
            },
        }
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

    async fn get_keys_by_filter(
        &self,
        key_source: Option<KeySource>,
    ) -> CustomResult<Vec<DataKey>, errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;

        let mut query = DataKey::table().into_boxed();

        if let Some(k_src) = key_source {
            query = query.filter(source.eq(k_src.to_string()));
        }

        query.get_results(&mut connection).await.switch()
    }

    #[cfg(feature = "aws")]
    async fn get_all_keys_for_identifier(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<Vec<DataKey>, errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;

        let (d_id, k_id) = identifier.get_identifier();

        let query = DataKey::table()
            .filter(data_identifier.eq(d_id).and(key_identifier.eq(k_id)))
            .order_by(version.desc());

        query.get_results(&mut connection).await.switch()
    }

    #[cfg(feature = "aws")]
    async fn get_all_keys(&self) -> CustomResult<Vec<DataKey>, errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;

        let query = DataKey::table().order_by((data_identifier, key_identifier, version.desc()));

        query.get_results(&mut connection).await.switch()
    }

    #[cfg(feature = "aws")]
    async fn update_key(&self, key: &DataKey) -> CustomResult<(), errors::DatabaseError> {
        let mut connection = self.get_conn().await.switch()?;

        let query = diesel::update(DataKey::table().find(key.id)).set((
            encryption_key.eq(&key.encryption_key),
            source.eq(&key.source),
            token.eq(&key.token),
        ));

        query.execute(&mut connection).await.switch()?;
        Ok(())
    }
}
