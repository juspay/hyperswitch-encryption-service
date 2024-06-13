use std::pin::Pin;

use futures::Future;

use crate::{
    crypto::{Crypto, Source},
    errors::{self, CustomResult, SwitchError},
    services::aws::AwsKmsClient,
};

use aws_sdk_kms::primitives::Blob;

use masking::{PeekInterface, StrongSecret};

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
            let encrypted_output = self
                .inner_client()
                .decrypt()
                .key_id(self.key_id())
                .ciphertext_blob(plaintext_blob)
                .send()
                .await
                .switch()?;

            let output = encrypted_output.plaintext.ok_or(error_stack::report!(
                errors::CryptoError::EncryptionFailed("KMS")
            ))?;

            Ok(output.into_inner().into())
        })
    }
}
