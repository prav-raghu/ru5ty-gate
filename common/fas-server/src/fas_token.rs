use sha2::{Digest, Sha256};

pub struct FasToken;

impl FasToken {
    pub fn return_token(hid: &str, faskey: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(hid.as_bytes());
        hasher.update(faskey.as_bytes());
        hex::encode(hasher.finalize())
    }
}
