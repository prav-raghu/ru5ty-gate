use aes_gcm::aead::{AeadInOut, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};

use crate::crypto_error::CryptoError;

const NONCE_LENGTH: usize = 12;
const TAG_LENGTH: usize = 16;

pub struct CryptoUtil {
    cipher: Aes256Gcm,
}

impl CryptoUtil {
    pub fn from_hex_key(hex_key: &str) -> Result<Self, CryptoError> {
        let key_bytes = hex::decode(hex_key).map_err(|_| CryptoError::InvalidKey)?;
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|_| CryptoError::InvalidKey)?;
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, text: &str) -> Result<String, CryptoError> {
        let nonce_bytes: [u8; NONCE_LENGTH] = rand::random();
        let mut buffer = text.as_bytes().to_vec();
        let tag = self
            .cipher
            .encrypt_inout_detached(&Nonce::from(nonce_bytes), b"", buffer.as_mut_slice().into())
            .map_err(|_| CryptoError::EncryptionFailed)?;
        Ok(format!(
            "{}:{}:{}",
            hex::encode(nonce_bytes),
            hex::encode(tag),
            hex::encode(buffer)
        ))
    }

    pub fn decrypt(&self, encrypted_text: &str) -> Result<String, CryptoError> {
        let mut parts = encrypted_text.split(':');
        let (Some(nonce_hex), Some(tag_hex), Some(cipher_hex), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(CryptoError::MalformedPayload);
        };
        let nonce_bytes: [u8; NONCE_LENGTH] = hex::decode(nonce_hex)
            .map_err(|_| CryptoError::MalformedPayload)?
            .try_into()
            .map_err(|_| CryptoError::MalformedPayload)?;
        let tag_bytes: [u8; TAG_LENGTH] = hex::decode(tag_hex)
            .map_err(|_| CryptoError::MalformedPayload)?
            .try_into()
            .map_err(|_| CryptoError::MalformedPayload)?;
        let mut buffer = hex::decode(cipher_hex).map_err(|_| CryptoError::MalformedPayload)?;
        self.cipher
            .decrypt_inout_detached(
                &Nonce::from(nonce_bytes),
                b"",
                buffer.as_mut_slice().into(),
                &tag_bytes.into(),
            )
            .map_err(|_| CryptoError::DecryptionFailed)?;
        String::from_utf8(buffer).map_err(|_| CryptoError::DecryptionFailed)
    }
}
