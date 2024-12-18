use masking::{PeekInterface, Secret};

use crate::app::AppState;

pub struct Blake3;

impl Blake3 {
    pub fn hash(state: &AppState, key: Secret<String>) -> [u8; 32] {
        let context = state.conf.secrets.hash_context.peek();
        let token = state.conf.secrets.access_token.peek();
        let key = blake3::derive_key(context.as_str(), key.peek().as_bytes());
        let output = blake3::keyed_hash(&key, token.as_bytes());

        *output.as_bytes()
    }
}
