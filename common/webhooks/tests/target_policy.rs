#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_webhooks::TargetPolicy;

#[tokio::test]
async fn allow_all_accepts_any_target() {
    assert!(
        TargetPolicy::AllowAll
            .ensure_allowed("http://127.0.0.1:8080/hook")
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn public_only_rejects_private_loopback_and_metadata_addresses() {
    for url in [
        "http://127.0.0.1/hook",
        "http://localhost/hook",
        "http://10.0.0.5/hook",
        "http://172.16.4.1/hook",
        "http://192.168.1.10/hook",
        "http://169.254.169.254/latest/meta-data",
        "http://100.64.0.1/hook",
        "http://0.0.0.0/hook",
        "http://[::1]/hook",
        "http://[fd00::1]/hook",
        "http://[fe80::1]/hook",
        "http://[::ffff:127.0.0.1]/hook",
    ] {
        assert!(
            TargetPolicy::PublicOnly.ensure_allowed(url).await.is_err(),
            "{url} should be rejected"
        );
    }
}

#[tokio::test]
async fn public_only_rejects_non_http_schemes_and_garbage() {
    for url in [
        "ftp://example.com/hook",
        "file:///etc/passwd",
        "not a url",
        "",
    ] {
        assert!(
            TargetPolicy::PublicOnly.ensure_allowed(url).await.is_err(),
            "{url} should be rejected"
        );
    }
}

#[tokio::test]
async fn public_only_accepts_public_ip_literals() {
    assert!(
        TargetPolicy::PublicOnly
            .ensure_allowed("https://93.184.216.34/hook")
            .await
            .is_ok()
    );
    assert!(
        TargetPolicy::PublicOnly
            .ensure_allowed("https://[2606:2800:220:1:248:1893:25c8:1946]/hook")
            .await
            .is_ok()
    );
}

#[test]
fn production_selects_the_public_only_policy() {
    assert_eq!(
        TargetPolicy::from_production(true),
        TargetPolicy::PublicOnly
    );
    assert_eq!(TargetPolicy::from_production(false), TargetPolicy::AllowAll);
}
