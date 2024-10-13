use crate::consts::base64::BASE64_ENGINE;
use crate::crypto::{Crypto, Source};
use crate::errors::{self, CryptoError, CustomResult, SwitchError};
use base64::Engine;
use error_stack::report;
use futures::Future;
use masking::{PeekInterface, Secret, StrongSecret};
use serde::Deserialize;
use std::array::TryFromSliceError;
use std::pin::Pin;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::{api, transit};
#[derive(Debug, Deserialize, Clone)]
pub struct VaultSettings {
    url: String,
    token: Secret<String>,
    mount_point: String,
    encryption_key: String,
}

pub struct Vault {
    inner_client: VaultClient,
    settings: VaultSettings,
}

pub fn init_vault(settings: VaultSettings) -> CustomResult<Vault, CryptoError> {
    let client = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address(&settings.url)
            .token(settings.token.peek())
            .build()
            .switch()?,
    )
    .switch()?;
    Ok(Vault {
        inner_client: client,
        settings: settings,
    })
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
        let response = transit::generate::random_bytes(
            &self.inner_client,
            &self.settings.mount_point,
            api::transit::OutputFormat::Base64,
            api::transit::requests::RandomBytesSource::All,
            None,
        )
        .await
        .switch()?;
        let buffer: [u8; 32] =
            response
                .random_bytes
                .as_bytes()
                .try_into()
                .map_err(|err: TryFromSliceError| {
                    report!(err).change_context(CryptoError::KeyStoreFailed)
                })?;
        Ok((Source::VAULT, buffer.into()))
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
            .switch()?
            .ciphertext
            .as_bytes()
            .to_vec()
            .into())
        })
    }

    fn decrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        Box::pin(async move {
            // let inner = input.peek();
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
            .switch()?
            .plaintext;
            Ok(BASE64_ENGINE
                .decode(b64_encoded_str)
                .map_err(|err| report!(err).change_context(CryptoError::DecryptionFailed("Vault")))?
                .into())
        })
    }
}
