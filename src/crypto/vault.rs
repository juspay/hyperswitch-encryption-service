use crate::consts::base64::BASE64_ENGINE;
use crate::crypto::{Crypto, Source};
use crate::env::observability as logger;
use crate::errors::{self, CryptoError, CustomResult};
use base64::Engine;
use error_stack::report;
use futures::Future;
use masking::{PeekInterface, StrongSecret};
use serde::Deserialize;
use std::pin::Pin;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::{api, transit};

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct VaultSettings {
    pub url: String,
    pub mount_point: String,
    pub encryption_key: String,
    pub vault_token: masking::Secret<String>,
}

pub struct Vault {
    inner_client: VaultClient,
    settings: VaultSettings,
}

impl Vault {
    #[allow(clippy::expect_used)]
    pub fn new(settings: VaultSettings) -> Self {
        let client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(&settings.url)
                .token(settings.vault_token.peek())
                .build()
                .expect("Unable to build HashiCorp Vault Settings"),
        )
        .expect("Unable to build HashiCorp Vault client");
        Self {
            inner_client: client,
            settings,
        }
    }
}

#[async_trait::async_trait]
impl Crypto for Vault {
    type DataReturn<'a> = Pin<
        Box<
            dyn Future<Output = CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError>>
                + 'a
                + Send,
        >,
    >;

    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        //According to Vault transit engine can genarate high entropy random bytes of different lengths.
        //https://developer.hashicorp.com/vault/docs/secrets/transit
        let response = transit::generate::random_bytes(
            &self.inner_client,
            &self.settings.mount_point,
            api::transit::OutputFormat::Base64,
            api::transit::requests::RandomBytesSource::All,
            None,
        )
        .await
        .map_err(|err| report!(err).change_context(errors::CryptoError::KeyGeneration))?;
        let key = BASE64_ENGINE
            .decode(response.random_bytes)
            .map_err(|err| report!(err).change_context(CryptoError::KeyGeneration))?;
        let buffer: [u8; 32] = key.try_into().map_err(|err: Vec<u8>| {
            let err_bytes = format!("{:?}", err);
            logger::debug!(err_bytes);
            report!(CryptoError::KeyGeneration)
        })?;
        Ok((Source::HashicorpVault, buffer.into()))
    }

    fn encrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        let b64_text = BASE64_ENGINE.encode(input.peek());
        Box::pin(async move {
            Ok(transit::data::encrypt(
                &self.inner_client,
                &self.settings.mount_point,
                &self.settings.encryption_key,
                &b64_text,
                None,
            )
            .await
            .map_err(|err| {
                report!(err).change_context(CryptoError::EncryptionFailed("HashiCorp Vault"))
            })?
            .ciphertext
            .as_bytes()
            .to_vec()
            .into())
        })
    }

    fn decrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            let cypher_text = String::from_utf8(input.peek().to_vec()).map_err(|err| {
                report!(err).change_context(CryptoError::DecryptionFailed("Vault"))
            })?;
            let b64_encoded_str = transit::data::decrypt(
                &self.inner_client,
                &self.settings.mount_point,
                &self.settings.encryption_key,
                &cypher_text,
                None,
            )
            .await
            .map_err(|err| {
                report!(err).change_context(CryptoError::DecryptionFailed("HashiCorp Vault"))
            })?
            .plaintext;
            Ok(BASE64_ENGINE
                .decode(b64_encoded_str)
                .map_err(|err| {
                    report!(err).change_context(CryptoError::DecryptionFailed("HashiCorp Vault"))
                })?
                .into())
        })
    }
}
