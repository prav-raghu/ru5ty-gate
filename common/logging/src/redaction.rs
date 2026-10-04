use serde_json::{Map, Value};

pub const REDACTED: &str = "[REDACTED]";

pub const SENSITIVE_KEYS: [&str; 23] = [
    "password",
    "currentPassword",
    "newPassword",
    "confirmPassword",
    "token",
    "accessToken",
    "refreshToken",
    "secret",
    "apiKey",
    "api_key",
    "clientSecret",
    "privateKey",
    "creditCard",
    "cardNumber",
    "cvv",
    "cvc",
    "ssn",
    "nationalId",
    "pin",
    "otp",
    "twoFactorCode",
    "authorization",
    "cookie",
];

pub fn mask_sensitive(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(mask_sensitive).collect()),
        Value::Object(entries) => {
            let masked: Map<String, Value> = entries
                .iter()
                .map(|(key, entry)| {
                    if SENSITIVE_KEYS.contains(&key.as_str()) {
                        (key.clone(), Value::String(REDACTED.to_owned()))
                    } else {
                        (key.clone(), mask_sensitive(entry))
                    }
                })
                .collect();
            Value::Object(masked)
        }
        other => other.clone(),
    }
}
