use sha2::{Digest, Sha256};

pub fn normalize_ip(ip: &str) -> &str {
    ip.strip_prefix("::ffff:").unwrap_or(ip)
}

pub fn hash_ip(ip: &str, pepper: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalize_ip(ip).as_bytes());
    hasher.update(pepper.as_bytes());
    hex::encode(hasher.finalize())
}
