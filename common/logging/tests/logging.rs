#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_logging::{REDACTED, hash_ip, mask_sensitive, normalize_ip};
use serde_json::json;

#[test]
fn masks_nested_sensitive_keys() {
    let body = json!({
        "email": "a@b.com",
        "password": "hunter2",
        "profile": { "refreshToken": "abc", "name": "Pat" },
        "items": [{ "otp": "123456" }]
    });

    let masked = mask_sensitive(&body);

    assert_eq!(masked["email"], "a@b.com");
    assert_eq!(masked["password"], REDACTED);
    assert_eq!(masked["profile"]["refreshToken"], REDACTED);
    assert_eq!(masked["profile"]["name"], "Pat");
    assert_eq!(masked["items"][0]["otp"], REDACTED);
}

#[test]
fn normalises_ipv4_mapped_addresses() {
    assert_eq!(normalize_ip("::ffff:10.0.0.1"), "10.0.0.1");
    assert_eq!(normalize_ip("10.0.0.1"), "10.0.0.1");
}

#[test]
fn hashes_mapped_and_plain_ips_identically() {
    assert_eq!(
        hash_ip("::ffff:10.0.0.1", "pepper"),
        hash_ip("10.0.0.1", "pepper")
    );
    assert_ne!(hash_ip("10.0.0.1", "a"), hash_ip("10.0.0.1", "b"));
}

#[test]
fn hash_mac_is_case_insensitive_and_depends_on_the_pepper() {
    let upper = ru5ty_gate_logging::hash_mac("AA:BB:CC:DD:EE:FF", "pepper");
    let lower = ru5ty_gate_logging::hash_mac("aa:bb:cc:dd:ee:ff", "pepper");
    let other = ru5ty_gate_logging::hash_mac("aa:bb:cc:dd:ee:ff", "different");

    assert_eq!(upper, lower);
    assert_ne!(lower, other);
    assert_eq!(lower.len(), 64);
}

#[test]
fn mask_mac_keeps_only_the_vendor_prefix() {
    assert_eq!(
        ru5ty_gate_logging::mask_mac("AA:BB:CC:DD:EE:FF"),
        "aa:bb:cc:xx:xx:xx"
    );
    assert_eq!(ru5ty_gate_logging::mask_mac("garbage"), "xx:xx:xx:xx:xx:xx");
}
