use masking::StrongSecret;

use crate::{
    env::observability as logger,
    errors::{self, SwitchError},
    multitenancy::TenantState,
    services::aws::AwsKmsClient,
    storage::{dek::DataKeyStorageInterface, types::UpdateReEncryptedKey},
    types::{requests::ReEncryptDataKeysRequest, response::ReEncryptDataKeysResponse},
};

enum ReencryptStatus {
    Reencrypted,
    Skipped,
    Failed(i32),
}

pub async fn reencrypt_data_keys(
    state: TenantState,
    req: ReEncryptDataKeysRequest,
) -> errors::CustomResult<ReEncryptDataKeysResponse, errors::ApplicationErrorResponse> {
    let db = state.get_db_pool();

    let mut kms_key_id = String::new();
    // Validate KMS backend configuration for skip_key_id_on_decrypt
    let backend = state.keymanager_client.client();
    if let Some(aws_client) = backend.as_any().downcast_ref::<AwsKmsClient>() {
        if !aws_client.skip_key_id_on_decrypt() {
            return Err(error_stack::report!(
                errors::ApplicationErrorResponse::InternalServerError(
                    "skip_key_id_on_decrypt must be enabled in KMS config for re-encryption"
                )
            ));
        }
        kms_key_id = aws_client.key_id().to_string();
    }

    // Fetch DEKs to re-encrypt
    let data_keys = db.get_keys_by_ids(req.key_ids.as_ref()).await.switch()?;

    let total_processed_keys = data_keys.len();
    let mut succeeded_keys = 0;
    let mut skipped_keys = 0;
    let mut failed_key_ids = Vec::new();

    logger::info!(
        total_keys = total_processed_keys,
        "Starting re-encryption of data keys"
    );

    // Process DEKs with bounded concurrency to respect KMS rate limits
    // and improve performance for large datasets
    const MAX_CONCURRENT_REENCRYPTIONS: usize = 10;

    use futures::stream::{self, StreamExt};

    let results = stream::iter(data_keys)
        .map(|data_key| {
            let state = state.clone();
            let kms_key_id = kms_key_id.clone();
            async move {
                let key_id = data_key.id;
                let identifier_str = format!(
                    "{}:{}:{}",
                    data_key.data_identifier, data_key.key_identifier, data_key.version
                );

                match reencrypt_single_key(&state, data_key, kms_key_id).await {
                    Ok(ReencryptStatus::Reencrypted) => {
                        logger::info!(
                            identifier = identifier_str.as_str(),
                            "Successfully re-encrypted DEK"
                        );
                        ReencryptStatus::Reencrypted
                    }
                    Ok(ReencryptStatus::Skipped) => {
                        logger::info!(
                            identifier = identifier_str.as_str(),
                            "Skipped re-encryption for DEK"
                        );
                        ReencryptStatus::Skipped
                    }
                    Ok(ReencryptStatus::Failed(id)) => {
                        logger::error!(
                            identifier = identifier_str.as_str(),
                            key_id = id,
                            "Failed to re-encrypt DEK"
                        );
                        ReencryptStatus::Failed(id)
                    }
                    Err(err) => {
                        logger::error!(
                            identifier = identifier_str.as_str(),
                            error = ?err,
                            "Failed to re-encrypt DEK"
                        );
                        ReencryptStatus::Failed(key_id)
                    }
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT_REENCRYPTIONS)
        .collect::<Vec<ReencryptStatus>>()
        .await;

    // Count successes, skipped, and failures
    for res in results {
        match res {
            ReencryptStatus::Reencrypted => succeeded_keys += 1,
            ReencryptStatus::Skipped => skipped_keys += 1,
            ReencryptStatus::Failed(id) => failed_key_ids.push(id),
        }
    }

    logger::info!(
        total = total_processed_keys,
        succeeded = succeeded_keys,
        skipped = skipped_keys,
        failed = failed_key_ids.len(),
        "Completed re-encryption of data keys"
    );

    Ok(ReEncryptDataKeysResponse {
        total_processed_keys,
        succeeded_keys,
        skipped_keys,
        failed_key_ids,
    })
}

async fn reencrypt_single_key(
    state: &TenantState,
    data_key: crate::storage::types::DataKey,
    current_key_id: String,
) -> errors::CustomResult<ReencryptStatus, errors::ApplicationErrorResponse> {
    let db = state.get_db_pool();

    let original_id = data_key.id;

    let crypto = state.keymanager_client.client();

    // decrypt DEK + capture source key id
    let (decrypted_key, source_key_id): (StrongSecret<Vec<u8>>, Option<String>) = {
        let aws_client = crypto
            .as_any()
            .downcast_ref::<AwsKmsClient>()
            .ok_or_else(|| {
                error_stack::report!(errors::ApplicationErrorResponse::InternalServerError(
                    "decrypt_with_metadata is only supported for AWS KMS backend"
                ))
            })?;
        aws_client
            .decrypt_with_metadata(data_key.encryption_key.clone())
            .await
            .switch()?
    };

    // Check if already encrypted with current key by comparing key IDs
    if let Some(source_key) = source_key_id {
        if source_key == current_key_id {
            return Ok(ReencryptStatus::Skipped);
        }
    }

    // re-encrypt with current configured key
    let reencrypted_key = crypto.encrypt_key(decrypted_key).await.switch()?;

    // Step 4: Create updated DataKey with re-encrypted data
    let updated_data_key = UpdateReEncryptedKey {
        id: original_id,
        encryption_key: reencrypted_key,
    };

    db.update_key(&updated_data_key).await.switch()?;

    Ok(ReencryptStatus::Reencrypted)
}
