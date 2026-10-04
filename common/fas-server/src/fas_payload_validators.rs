use std::net::{IpAddr, SocketAddr};

use validator::ValidationError;

fn failure(code: &'static str) -> ValidationError {
    ValidationError::new(code)
}

pub fn is_valid_mac(value: &str) -> bool {
    let octets: Vec<&str> = value.split(':').collect();
    octets.len() == 6
        && octets
            .iter()
            .all(|octet| octet.len() == 2 && octet.chars().all(|c| c.is_ascii_hexdigit()))
}

pub fn validate_mac(value: &str) -> Result<(), ValidationError> {
    if is_valid_mac(value) {
        Ok(())
    } else {
        Err(failure("mac"))
    }
}

pub fn validate_ip(value: &str) -> Result<(), ValidationError> {
    value.parse::<IpAddr>().map(drop).map_err(|_| failure("ip"))
}

pub fn validate_hid(value: &str) -> Result<(), ValidationError> {
    if (1..=128).contains(&value.len()) && value.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err(failure("hid"))
    }
}

pub fn validate_gateway_address(value: &str) -> Result<(), ValidationError> {
    value
        .parse::<SocketAddr>()
        .map(drop)
        .map_err(|_| failure("gateway_address"))
}

pub fn validate_authdir(value: &str) -> Result<(), ValidationError> {
    let valid = (1..=64).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(())
    } else {
        Err(failure("authdir"))
    }
}
