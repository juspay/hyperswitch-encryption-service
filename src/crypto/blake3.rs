use crate::config::Config;
use masking::{PeekInterface, Secret, StrongSecret};

pub struct Blake3(StrongSecret<[u8; 32]>);

impl Blake3 {
    pub async fn from_config(config: &Config) -> Self {
        let access_token = &config.secrets.access_token;
        let context = config.secrets.hash_context.peek();

        let access_token = access_token.expose(config).await;

        let key = blake3::derive_key(context, access_token.peek().as_bytes());

        Self(key.into())
    }

    pub fn hash(&self, token: Secret<String>) -> [u8; 32] {
        let output = blake3::keyed_hash(self.0.peek(), token.peek().as_bytes());

        *output.as_bytes()
    }
}
