use futures::stream::{self, StreamExt};
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
    // Validate KMS backend: decrypt against a client in decrypt_region when set
    // (for DEKs encrypted by a KMS key in another region; the configured key_id
    // is region-specific, so it is never sent to that client), else require
    // skip_key_id_on_decrypt for same-region re-encryption.
    let backend = state.keymanager_client.client();
    let mut decrypt_client = None;
    if let Some(aws_client) = backend.as_any().downcast_ref::<AwsKmsClient>() {
        if let Some(region) = aws_client.decrypt_region() {
            decrypt_client = Some(AwsKmsClient::client_for_region(region).await);
        } else if !aws_client.skip_key_id_on_decrypt() {
            return Err(error_stack::report!(
                errors::ApplicationErrorResponse::InternalServerError(
                    "skip_key_id_on_decrypt must be enabled in KMS config for re-encryption"
                )
            ));
        }
        kms_key_id = aws_client.key_id().to_string();
    }
    let decrypt_client = std::sync::Arc::new(decrypt_client);

    // Fetch DEKs to re-encrypt
    let data_keys = db.get_keys_by_ids(req.key_ids.as_deref()).await.switch()?;

    let total_processed_keys = data_keys.len();
    let mut succeeded_keys = 0;
    let mut skipped_keys = 0;
    let mut failed_keys = 0;
    let mut failed_key_ids = Vec::new();

    logger::info!(
        total_keys = total_processed_keys,
        "Starting re-encryption of data keys"
    );

    // Process DEKs with bounded concurrency to respect KMS rate limits
    // and improve performance for large datasets
    const MAX_CONCURRENT_REENCRYPTIONS: usize = 10;

    let results = stream::iter(data_keys)
        .map(|data_key| {
            let state = state.clone();
            let kms_key_id = kms_key_id.clone();
            let decrypt_client = decrypt_client.clone();
            async move {
                let key_id = data_key.id;
                let identifier_str = format!(
                    "{}:{}:{}:{}",
                    data_key.id,
                    data_key.data_identifier,
                    data_key.key_identifier,
                    data_key.version
                );

                match reencrypt_single_key(&state, data_key, kms_key_id, decrypt_client).await {
                    Ok(ReencryptStatus::Reencrypted) => {
                        logger::info!(identifier = %identifier_str, "Successfully re-encrypted DEK");
                        ReencryptStatus::Reencrypted
                    }
                    Ok(ReencryptStatus::Skipped) => {
                        logger::info!(identifier = %identifier_str, "Skipped re-encryption for DEK");
                        ReencryptStatus::Skipped
                    }
                    Ok(ReencryptStatus::Failed(failed_key_id)) => {
                        logger::error!(identifier = %identifier_str, key_id = ?failed_key_id, "Failed to re-encrypt DEK");
                        ReencryptStatus::Failed(failed_key_id)
                    }
                    Err(err) => {
                        logger::error!(identifier = %identifier_str, error = ?err, "Failed to re-encrypt DEK");
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
            ReencryptStatus::Failed(k_id) => {
                failed_keys += 1;
                failed_key_ids.push(k_id)
            }
        }
    }

    logger::info!(
        total = total_processed_keys,
        succeeded = succeeded_keys,
        skipped = skipped_keys,
        failed_keys = failed_keys,
        "Completed re-encryption of data keys"
    );

    Ok(ReEncryptDataKeysResponse {
        total_processed_keys,
        succeeded_keys,
        skipped_keys,
        failed_keys,
        failed_key_ids,
    })
}

async fn reencrypt_single_key(
    state: &TenantState,
    data_key: crate::storage::types::DataKey,
    current_key_id: String,
    decrypt_client: std::sync::Arc<Option<aws_sdk_kms::Client>>,
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

        let ciphertext = data_key.encryption_key.clone();

        // First decrypt attempt: use the cross-region client when a decrypt_region is
        // configured, else the default region. KMS is strictly regional, so a DEK still
        // encrypted with the current key fails the cross-region call—fall back to the
        // default region for it.
        match aws_client
            .decrypt_with_metadata(ciphertext.clone(), decrypt_client.as_ref().as_ref())
            .await
        {
            Ok(output) => output,
            Err(err) if decrypt_client.is_some() => {
                logger::info!(
                    key_id = original_id,
                    error = ?err,
                    "Cross-region decrypt failed, falling back to default region"
                );
                aws_client
                    .decrypt_with_metadata(ciphertext, None)
                    .await
                    .switch()?
            }
            Err(err) => return Err(err).switch(),
        }
    };

    // Check if already encrypted with current key by comparing key IDs
    if let Some(source_key) = source_key_id {
        logger::info!(source_key_id = %source_key, current_key_id = %current_key_id);
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
