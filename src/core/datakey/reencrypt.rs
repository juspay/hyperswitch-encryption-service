use crate::{
    core::crypto::{KeyDecrypter, KeyEncrypter},
    env::observability as logger,
    errors::{self, SwitchError},
    multitenancy::TenantState,
    storage::dek::DataKeyStorageInterface,
    types::{requests::ReEncryptDataKeysRequest, response::ReEncryptDataKeysResponse},
};

pub async fn reencrypt_data_keys(
    state: TenantState,
    req: ReEncryptDataKeysRequest,
) -> errors::CustomResult<ReEncryptDataKeysResponse, errors::ApplicationErrorResponse> {
    let db = state.get_db_pool();

    // Validate that skip_key_id_on_decrypt is enabled for AWS KMS
    #[cfg(feature = "aws")]
    {
        use crate::services::aws::AwsKmsClient;

        let backend = state.keymanager_client.client();
        if let Some(aws_client) = backend.as_any().downcast_ref::<AwsKmsClient>() {
            if !aws_client.skip_key_id_on_decrypt() {
                return Err(error_stack::report!(
                    errors::ApplicationErrorResponse::InternalServerError(
                        "skip_key_id_on_decrypt must be enabled in KMS config for re-encryption"
                    )
                ));
            }
        }
    }

    // Fetch DEKs to re-encrypt
    let data_keys = if let Some(identifier) = req.identifier {
        db.get_all_keys_for_identifier(&identifier).await.switch()?
    } else {
        db.get_all_keys().await.switch()?
    };

    let total_keys = data_keys.len();
    let mut success_count = 0;
    let mut failure_count = 0;

    logger::info!(
        total_keys = total_keys,
        "Starting re-encryption of data keys"
    );

    // Process DEKs with bounded concurrency to respect KMS rate limits
    // and improve performance for large datasets
    const MAX_CONCURRENT_REENCRYPTIONS: usize = 10;

    use futures::stream::{self, StreamExt};

    let results = stream::iter(data_keys)
        .map(|data_key| {
            let state = state.clone();
            async move {
                let identifier_str = format!(
                    "{}:{}:{}",
                    data_key.data_identifier, data_key.key_identifier, data_key.version
                );

                match reencrypt_single_key(&state, data_key).await {
                    Ok(()) => {
                        logger::info!(
                            identifier = identifier_str.as_str(),
                            "Successfully re-encrypted DEK"
                        );
                        Ok(())
                    }
                    Err(err) => {
                        logger::error!(
                            identifier = identifier_str.as_str(),
                            error = ?err,
                            "Failed to re-encrypt DEK"
                        );
                        Err(())
                    }
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT_REENCRYPTIONS)
        .collect::<Vec<_>>()
        .await;

    // Count successes and failures
    for result in results {
        match result {
            Ok(()) => success_count += 1,
            Err(()) => failure_count += 1,
        }
    }

    logger::info!(
        total = total_keys,
        success = success_count,
        failed = failure_count,
        "Completed re-encryption of data keys"
    );

    Ok(ReEncryptDataKeysResponse {
        total_keys,
        success_count,
        failure_count,
    })
}

async fn reencrypt_single_key(
    state: &TenantState,
    data_key: crate::storage::types::DataKey,
) -> errors::CustomResult<(), errors::ApplicationErrorResponse> {
    let db = state.get_db_pool();

    // Save fields we need before data_key is consumed
    let original_id = data_key.id;
    let original_key_identifier = data_key.key_identifier.clone();
    let original_data_identifier = data_key.data_identifier.clone();
    let original_version = data_key.version;
    let original_created_at = data_key.created_at;

    // Step 1: Decrypt the DEK with the old key (KMS will use ciphertext metadata)
    let decrypted_key = data_key.decrypt(state).await.switch()?;

    // Step 2: Re-encrypt with the new key (using current config's key_id)
    let reencrypted_key = decrypted_key.encrypt(state).await.switch()?;

    // Step 3: Create updated DataKey with re-encrypted data
    let updated_data_key = crate::storage::types::DataKey {
        id: original_id,
        key_identifier: original_key_identifier,
        data_identifier: original_data_identifier,
        encryption_key: reencrypted_key.encryption_key,
        version: original_version,
        created_at: original_created_at,
        source: reencrypted_key.source,
        token: reencrypted_key.token,
    };

    db.update_key(&updated_data_key).await.switch()?;

    Ok(())
}
