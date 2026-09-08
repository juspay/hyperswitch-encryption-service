use std::pin::Pin;

use error_stack::{IntoReport, ResultExt};
use futures::Future;
use google_cloud_kms::grpc::kms::v1::{
    DecryptRequest, EncryptRequest, GenerateRandomBytesRequest, ProtectionLevel,
};
use hyperswitch_masking::{PeekInterface, StrongSecret};

use crate::{
    crypto::{Crypto, Source},
    env::observability as logger,
    errors::{self, CustomResult},
    services::gcp::GcpKmsClient,
};

#[async_trait::async_trait]
impl Crypto for GcpKmsClient {
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
        let request = GenerateRandomBytesRequest {
            location: self.location().to_owned(),
            length_bytes: 32,
            protection_level: ProtectionLevel::Hsm.into(),
        };
        let response = self
            .inner_client()
            .generate_random_bytes(request, None)
            .await
            .inspect_err(|error| {
                logger::error!(gcp_kms_err = ?error, "Failed to GCP KMS generate random bytes");
            })
            .change_context(errors::CryptoError::KeyGeneration)?;

        let key = <[u8; 32]>::try_from(response.data)
            .map_err(|_| errors::CryptoError::KeyGeneration.into_report())?;

        Ok((Source::GcpKms, key.into()))
    }

    fn encrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            let request = EncryptRequest {
                name: self.key_name().to_owned(),
                plaintext: input.peek().to_vec(),
                additional_authenticated_data: Vec::new(),
                plaintext_crc32c: None,
                additional_authenticated_data_crc32c: None,
            };
            let response = self
                .inner_client()
                .encrypt(request, None)
                .await
                .inspect_err(|error| {
                    logger::error!(gcp_kms_err = ?error, "Failed to GCP KMS encrypt data");
                })
                .change_context(errors::CryptoError::EncryptionFailed("GCP KMS"))?;

            Ok(response.ciphertext.into())
        })
    }

    fn decrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            let request = DecryptRequest {
                name: self.key_name().to_owned(),
                ciphertext: input.peek().to_vec(),
                additional_authenticated_data: Vec::new(),
                ciphertext_crc32c: None,
                additional_authenticated_data_crc32c: None,
            };
            let response = self
                .inner_client()
                .decrypt(request, None)
                .await
                .inspect_err(|error| {
                    logger::error!(gcp_kms_err = ?error, "Failed to GCP KMS decrypt data");
                })
                .change_context(errors::CryptoError::DecryptionFailed("GCP KMS"))?;

            Ok(response.plaintext.into())
        })
    }
}
