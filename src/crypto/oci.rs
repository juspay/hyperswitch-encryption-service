use std::pin::Pin;

use error_stack::{Report, ResultExt};
use futures::Future;
use hyperswitch_masking::{PeekInterface, StrongSecret};

use crate::{
    crypto::{Crypto, Source},
    env::observability as logger,
    errors::{self, CustomResult},
    services::oci::OciKmsClient,
};

#[async_trait::async_trait]
impl Crypto for OciKmsClient {
    type DataReturn<'a> = Pin<
        Box<
            dyn Future<Output = CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError>>
                + Send
                + 'a,
        >,
    >;

    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        // OCI also returns the key wrapped under the vault key, but only the plaintext is used:
        // as with the other backends, the caller wraps it through `encrypt_key`.
        let data_key = self.inner().generate_data_key().await.map_err(|error| {
            logger::error!(oci_kms_err = %error, "Failed to OCI KMS generate data key");
            Report::new(error).change_context(errors::CryptoError::KeyGeneration)
        })?;

        Ok((Source::OciKms, (*data_key.plaintext()).into()))
    }

    fn encrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            let ciphertext = self.inner().encrypt(input.peek()).await.map_err(|error| {
                logger::error!(oci_kms_err = %error, "Failed to OCI KMS encrypt data");
                Report::new(error).change_context(errors::CryptoError::EncryptionFailed("OCI KMS"))
            })?;

            // Stored as the bytes of OCI's base64 ciphertext, which `decrypt` hands back as is.
            Ok(ciphertext.into_bytes().into())
        })
    }

    fn decrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            let ciphertext = std::str::from_utf8(input.peek()).change_context(
                errors::CryptoError::ParseError(
                    "OCI KMS ciphertext is not valid UTF-8".to_string(),
                ),
            )?;

            let plaintext = self.inner().decrypt(ciphertext).await.map_err(|error| {
                logger::error!(oci_kms_err = %error, "Failed to OCI KMS decrypt data");
                Report::new(error).change_context(errors::CryptoError::DecryptionFailed("OCI KMS"))
            })?;

            Ok(plaintext.into())
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::crypto::KeyManagement;

    fn env(name: &str) -> String {
        std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set for live tests"))
    }

    fn live_client() -> OciKmsClient {
        OciKmsClient::new(&crate::services::oci::OciKmsConfig {
            vault_crypto_endpoint: env("OCI_KMS_TEST_CRYPTO_ENDPOINT"),
            key_id: env("OCI_KMS_TEST_KEY_ID"),
        })
        .expect("client builds")
    }

    /// The data-key lifecycle the service runs against a real OCI Vault: generate a key, wrap it
    /// for storage, and unwrap it again. Skipped by default; run with:
    ///
    /// ```text
    /// OCI_KMS_TEST_CRYPTO_ENDPOINT=https://<vault>-crypto.kms.<region>.oci.oraclecloud.com \
    /// OCI_KMS_TEST_KEY_ID=ocid1.key.oc1... \
    /// cargo test --features oci oci -- --ignored
    /// ```
    #[tokio::test]
    #[ignore = "calls a real OCI Vault"]
    async fn data_key_round_trips_through_the_key_manager() {
        let client = live_client();

        let (source, key) = KeyManagement::generate_key(&client)
            .await
            .expect("generate_key succeeds");
        assert_eq!(source.to_string(), "OciKms");

        let wrapped = KeyManagement::encrypt_key(&client, key.peek().to_vec().into())
            .await
            .expect("encrypt_key succeeds");
        assert_ne!(wrapped.peek().as_slice(), key.peek().as_slice());

        let unwrapped = KeyManagement::decrypt_key(&client, wrapped)
            .await
            .expect("decrypt_key succeeds");
        assert_eq!(unwrapped.peek().as_slice(), key.peek().as_slice());
    }

    /// Decrypts a bootstrap secret encrypted with the `oci` CLI. Also needs
    /// `OCI_KMS_TEST_CLI_CIPHERTEXT` and `OCI_KMS_TEST_CLI_PLAINTEXT`.
    #[tokio::test]
    #[ignore = "calls a real OCI Vault"]
    async fn decrypt_secret_decrypts_cli_ciphertext() {
        let secret = live_client()
            .decrypt_secret(&env("OCI_KMS_TEST_CLI_CIPHERTEXT"))
            .await
            .expect("decrypt_secret succeeds");
        assert_eq!(secret, env("OCI_KMS_TEST_CLI_PLAINTEXT"));
    }
}
