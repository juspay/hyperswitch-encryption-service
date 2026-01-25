use std::pin::Pin;

use aws_sdk_kms::primitives::Blob;
use futures::Future;
use masking::{PeekInterface, StrongSecret};

use crate::{
    crypto::{Crypto, Source},
    errors::{self, CustomResult, SwitchError},
    services::aws::AwsKmsClient,
};

#[async_trait::async_trait]
impl Crypto for AwsKmsClient {
    type DataReturn<'a> = Pin<
        Box<
            dyn Future<Output = CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError>>
                + Send
                + 'a,
        >,
    >;
    async fn generate_key(
        &self,
    ) -> errors::CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        let resp = self
            .inner_client()
            .generate_data_key()
            .key_id(self.key_id())
            .key_spec(aws_sdk_kms::types::DataKeySpec::Aes256)
            .send()
            .await
            .switch()?;

        let plaintext_blob = <[u8; 32]>::try_from(
            resp.plaintext
                .ok_or(error_stack::report!(errors::CryptoError::KeyGeneration))?
                .into_inner(),
        )
        .map_err(|_| error_stack::report!(errors::CryptoError::KeyGeneration))?;

        Ok((Source::KMS, plaintext_blob.into()))
    }

    fn encrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            let plaintext_blob = Blob::new(input.peek().to_vec());
            let encrypted_output = self
                .inner_client()
                .encrypt()
                .key_id(self.key_id())
                .plaintext(plaintext_blob)
                .send()
                .await
                .switch()?;

            let output = encrypted_output
                .ciphertext_blob
                .ok_or(error_stack::report!(errors::CryptoError::EncryptionFailed(
                    "KMS"
                )))?;

            Ok(output.into_inner().into())
        })
    }
    fn decrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            let plaintext_blob = Blob::new(input.peek().to_vec());
            let mut decrypt_request = self
                .inner_client()
                .decrypt()
                .ciphertext_blob(plaintext_blob);

            // Only include key_id in decrypt if skip_key_id_on_decrypt is false
            // When true, KMS determines the key from the ciphertext metadata
            if !self.skip_key_id_on_decrypt() {
                decrypt_request = decrypt_request.key_id(self.key_id());
            }

            let encrypted_output = decrypt_request.send().await.switch()?;

            let output = encrypted_output.plaintext.ok_or(error_stack::report!(
                errors::CryptoError::EncryptionFailed("KMS")
            ))?;

            Ok(output.into_inner().into())
        })
    }
}

#[cfg(feature = "aws")]
impl AwsKmsClient {
    /// Decrypt and return both the plaintext and the key ID used for decryption
    pub async fn decrypt_with_metadata(
        &self,
        input: StrongSecret<Vec<u8>>,
    ) -> CustomResult<(StrongSecret<Vec<u8>>, Option<String>), errors::CryptoError> {
        let plaintext_blob = Blob::new(input.peek().to_vec());
        let mut decrypt_request = self
            .inner_client()
            .decrypt()
            .ciphertext_blob(plaintext_blob);

        if !self.skip_key_id_on_decrypt() {
            decrypt_request = decrypt_request.key_id(self.key_id());
        }

        let decrypt_output = decrypt_request.send().await.switch()?;

        let key_id = decrypt_output.key_id().map(|s| s.to_string());

        let plaintext = decrypt_output.plaintext.ok_or(error_stack::report!(
            errors::CryptoError::DecryptionFailed("KMS")
        ))?;

        Ok((plaintext.into_inner().into(), key_id))
    }
}
