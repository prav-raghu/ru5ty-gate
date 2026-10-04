use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, Default)]
pub struct WebhookSignatureService;

impl WebhookSignatureService {
    pub fn generate_signature(&self, payload: &str, secret: &str) -> String {
        match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(mut mac) => {
                mac.update(payload.as_bytes());
                hex::encode(mac.finalize().into_bytes())
            }
            Err(_) => String::new(),
        }
    }

    pub fn verify_signature(&self, payload: &str, signature: &str, secret: &str) -> bool {
        let Ok(expected) = hex::decode(signature) else {
            return false;
        };
        let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
            return false;
        };
        mac.update(payload.as_bytes());
        mac.verify_slice(&expected).is_ok()
    }

    pub fn generate_secret(&self) -> String {
        let bytes: [u8; 32] = rand::random();
        hex::encode(bytes)
    }
}
