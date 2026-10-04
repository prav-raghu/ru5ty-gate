use ru5ty_gate_logging::{hash_ip, hash_mac, mask_mac};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct IdentifierPolicy {
    pepper: Option<SecretString>,
}

impl IdentifierPolicy {
    pub fn raw() -> Self {
        Self { pepper: None }
    }

    pub fn hashed(pepper: SecretString) -> Self {
        Self {
            pepper: Some(pepper),
        }
    }

    pub fn sends_raw(&self) -> bool {
        self.pepper.is_none()
    }

    pub fn mac(&self, mac: &str) -> String {
        match &self.pepper {
            Some(pepper) => hash_mac(mac, pepper.expose_secret()),
            None => mac.to_owned(),
        }
    }

    pub fn ip(&self, ip: &str) -> String {
        match &self.pepper {
            Some(pepper) => hash_ip(ip, pepper.expose_secret()),
            None => ip.to_owned(),
        }
    }

    pub fn log_mac(&self, mac: &str) -> String {
        match &self.pepper {
            Some(pepper) => hash_mac(mac, pepper.expose_secret())[..12].to_owned(),
            None => mask_mac(mac),
        }
    }

    pub fn apply_to_payload(&self, mut payload: Value) -> Value {
        if self.sends_raw() {
            return payload;
        }
        if let Value::Object(map) = &mut payload {
            if let Some(Value::String(mac)) = map.remove("mac") {
                map.insert("mac_hash".to_owned(), Value::String(self.mac(&mac)));
            }
            if let Some(Value::String(ip)) = map.remove("client_ip") {
                map.insert("client_ip_hash".to_owned(), Value::String(self.ip(&ip)));
            }
        }
        payload
    }
}
