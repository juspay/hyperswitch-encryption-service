use error_stack::ResultExt;

use core::fmt;

use crate::{
    crypto::{Crypto, Source},
    errors::{self, CustomResult, SwitchError},
};

use serde::de::{self, Deserialize, Deserializer, Unexpected, Visitor};

use masking::PeekInterface;

use ring::aead::{self, BoundKey, OpeningKey, SealingKey, UnboundKey};

use masking::StrongSecret;

#[derive(Debug, Clone)]
pub struct GcmAes256 {
    key: StrongSecret<[u8; 32]>,
}

impl GcmAes256 {
    fn key(&self) -> &[u8] {
        self.key.peek()
    }
    pub fn new(key: StrongSecret<[u8; 32]>) -> errors::CustomResult<Self, errors::CryptoError> {
        Ok(Self { key })
    }

    #[allow(dead_code)]
    pub async fn from_vec(
        key: StrongSecret<Vec<u8>>,
    ) -> errors::CustomResult<Self, errors::CryptoError> {
        let key = <[u8; 32]>::try_from(key.peek().to_vec())
            .map_err(|_| error_stack::report!(errors::CryptoError::InvalidKey))?;

        Ok(Self { key: key.into() })
    }
}

#[derive(Clone, Debug)]
struct NonceSequence(u128);

impl NonceSequence {
    /// Byte index at which sequence number starts in a 16-byte (128-bit) sequence.
    /// This byte index considers the big endian order used while encoding and decoding the nonce
    /// to/from a 128-bit unsigned integer.
    const SEQUENCE_NUMBER_START_INDEX: usize = 4;

    /// Generate a random nonce sequence.
    fn new() -> Result<Self, ring::error::Unspecified> {
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // 96-bit sequence number, stored in a 128-bit unsigned integer in big-endian order
        let mut sequence_number = [0_u8; 128 / 8];
        rng.fill(&mut sequence_number[Self::SEQUENCE_NUMBER_START_INDEX..])?;
        let sequence_number = u128::from_be_bytes(sequence_number);

        Ok(Self(sequence_number))
    }

    /// Returns the current nonce value as bytes.
    fn current(&self) -> [u8; aead::NONCE_LEN] {
        let mut nonce = [0_u8; aead::NONCE_LEN];
        nonce.copy_from_slice(&self.0.to_be_bytes()[Self::SEQUENCE_NUMBER_START_INDEX..]);
        nonce
    }

    /// Constructs a nonce sequence from bytes
    fn from_bytes(bytes: [u8; aead::NONCE_LEN]) -> Self {
        let mut sequence_number = [0_u8; 128 / 8];
        sequence_number[Self::SEQUENCE_NUMBER_START_INDEX..].copy_from_slice(&bytes);
        let sequence_number = u128::from_be_bytes(sequence_number);
        Self(sequence_number)
    }
}

impl aead::NonceSequence for NonceSequence {
    fn advance(&mut self) -> Result<aead::Nonce, ring::error::Unspecified> {
        let mut nonce = [0_u8; aead::NONCE_LEN];
        nonce.copy_from_slice(&self.0.to_be_bytes()[Self::SEQUENCE_NUMBER_START_INDEX..]);

        // Increment sequence number
        self.0 = self.0.wrapping_add(1);

        // Return previous sequence number as bytes
        Ok(aead::Nonce::assume_unique_for_key(nonce))
    }
}

impl<'de> Deserialize<'de> for GcmAes256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Aes256Visitor;

        impl<'de> Visitor<'de> for Aes256Visitor {
            type Value = GcmAes256;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string of the format {version}:{base64_encoded_data}'")
            }

            fn visit_str<E>(self, value: &str) -> Result<GcmAes256, E>
            where
                E: de::Error,
            {
                let dec_data = hex::decode(value).map_err(|err| {
                    let err = err.to_string();
                    E::invalid_value(Unexpected::Str(value), &err.as_str())
                })?;

                let len = dec_data.len();

                Ok(GcmAes256 {
                    key: <[u8; 32]>::try_from(dec_data)
                        .map_err(|_| {
                            E::invalid_value(
                                Unexpected::Str(value),
                                &format!("Invalid encryption key length expected length is 32 found length {len}").as_str(),
                            )
                        })?
                        .into(),
                })
            }
        }

        deserializer.deserialize_str(Aes256Visitor)
    }
}

#[async_trait::async_trait]
impl Crypto for GcmAes256 {
    type DataReturn<'a> = CustomResult<StrongSecret<Vec<u8>>, errors::CryptoError>;
    async fn generate_key(
        &self,
    ) -> CustomResult<(Source, StrongSecret<[u8; 32]>), errors::CryptoError> {
        use ring::rand::SecureRandom;

        let rng = ring::rand::SystemRandom::new();
        let mut key: [u8; 32] = [0_u8; 32];
        rng.fill(&mut key).switch()?;
        Ok((Source::AESLocal, key.into()))
    }

    fn encrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        let secret = self.key();

        let nonce_sequence =
            NonceSequence::new().change_context(errors::CryptoError::EncryptionFailed("AES256"))?;
        let current_nonce = nonce_sequence.current();
        let key = UnboundKey::new(&aead::AES_256_GCM, secret)
            .change_context(errors::CryptoError::EncryptionFailed("AES256"))?;
        let mut key = SealingKey::new(key, nonce_sequence);
        let mut in_out = input.peek().to_vec();

        key.seal_in_place_append_tag(aead::Aad::empty(), &mut in_out)
            .change_context(errors::CryptoError::EncryptionFailed("AES256"))?;
        in_out.splice(0..0, current_nonce);

        Ok(in_out.into())
    }
    fn decrypt(&self, input: StrongSecret<Vec<u8>>) -> Self::DataReturn<'_> {
        let secret = self.key();

        let msg = input.peek().to_vec();
        let key = UnboundKey::new(&aead::AES_256_GCM, secret)
            .change_context(errors::CryptoError::DecryptionFailed("AES256"))?;

        let nonce_sequence = NonceSequence::from_bytes(
            <[u8; aead::NONCE_LEN]>::try_from(
                msg.get(..aead::NONCE_LEN)
                    .ok_or(errors::CryptoError::DecryptionFailed("AES256"))
                    .attach_printable("Failed to read the nonce form the encrypted ciphertext")?,
            )
            .change_context(errors::CryptoError::DecryptionFailed("AES256"))?,
        );

        let mut key = OpeningKey::new(key, nonce_sequence);
        let mut binding = msg;
        let output = binding.as_mut_slice();

        let result = key
            .open_within(aead::Aad::empty(), output, aead::NONCE_LEN..)
            .change_context(errors::CryptoError::DecryptionFailed("AES256"))?;

        Ok(result.to_vec().into())
    }
}
