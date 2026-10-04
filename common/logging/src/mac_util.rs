use sha2::{Digest, Sha256};

pub fn hash_mac(mac: &str, pepper: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(mac.to_ascii_lowercase().as_bytes());
    hasher.update(pepper.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn mask_mac(mac: &str) -> String {
    let lowered = mac.to_ascii_lowercase();
    let mut octets = lowered.split(':');
    match (octets.next(), octets.next(), octets.next()) {
        (Some(a), Some(b), Some(c)) => format!("{a}:{b}:{c}:xx:xx:xx"),
        _ => "xx:xx:xx:xx:xx:xx".to_owned(),
    }
}
