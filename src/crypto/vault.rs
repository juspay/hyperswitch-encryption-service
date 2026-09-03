use std::pin::Pin;

use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use futures::Future;
use hyperswitch_masking::{PeekInterface, StrongSecret};
use serde::Deserialize;
use vaultrs::{
    api,
    client::{VaultClient, VaultClientSettingsBuilder},
    transit,
};

use crate::{
    consts::base64::BASE64_ENGINE,
    crypto::{Crypto, Source},
    env::observability as logger,
    errors::{self, CryptoError, CustomResult},
};

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct VaultSettings {
    pub url: String,
    pub mount_point: String,
    pub encryption_key: String,
    pub vault_token: hyperswitch_masking::Secret<String>,
}

impl VaultSettings {
    /// All four fields are required to authenticate and address the transit engine.
    pub fn validate(&self) -> Result<(), &'static str> {
        let fields = [
            (self.url.as_str(), "Vault URL must not be empty"),
            (
                self.mount_point.as_str(),
                "Vault mount point must not be empty",
            ),
            (
                self.encryption_key.as_str(),
                "Vault encryption key must not be empty",
            ),
            (
                self.vault_token.peek().as_str(),
                "Vault token must not be empty",
            ),
        ];

        fields
            .into_iter()
            .find(|(value, _)| value.trim().is_empty())
            .map_or(Ok(()), |(_, error)| Err(error))
    }
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

    /// Decrypts `data` (already in Vault's own transit ciphertext format, not base64) via
    /// HashiCorp Vault. Used for bootstrap secrets read from TOML config.
    pub async fn decrypt_secret(&self, data: &str) -> CustomResult<String, errors::CryptoError> {
        let b64_encoded_str = transit::data::decrypt(
            &self.inner_client,
            &self.settings.mount_point,
            &self.settings.encryption_key,
            data,
            None,
        )
        .await
        .change_context(CryptoError::DecryptionFailed("HashiCorp Vault"))?
        .plaintext;

        let decoded = BASE64_ENGINE
            .decode(b64_encoded_str)
            .change_context(CryptoError::DecryptionFailed("HashiCorp Vault"))?;

        String::from_utf8(decoded).change_context(CryptoError::ParseError(
            "Invalid HashiCorp Vault decrypted secret".to_string(),
        ))
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
        .change_context(errors::CryptoError::KeyGeneration)?;
        let key = BASE64_ENGINE
            .decode(response.random_bytes)
            .change_context(CryptoError::KeyGeneration)?;
        let buffer: [u8; 32] = key.try_into().map_err(|err: Vec<u8>| {
            logger::debug!(
                key_length = err.len(),
                "Unexpected key length returned by Vault transit"
            );
            CryptoError::KeyGeneration.into_report()
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
            .change_context(CryptoError::EncryptionFailed("HashiCorp Vault"))?
            .ciphertext
            .as_bytes()
            .to_vec()
            .into())
        })
    }

    fn decrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            let ciphertext = String::from_utf8(input.peek().to_vec())
                .change_context(CryptoError::DecryptionFailed("Vault"))?;
            let b64_encoded_str = transit::data::decrypt(
                &self.inner_client,
                &self.settings.mount_point,
                &self.settings.encryption_key,
                &ciphertext,
                None,
            )
            .await
            .change_context(CryptoError::DecryptionFailed("HashiCorp Vault"))?
            .plaintext;
            Ok(BASE64_ENGINE
                .decode(b64_encoded_str)
                .change_context(CryptoError::DecryptionFailed("HashiCorp Vault"))?
                .into())
        })
    }
}
