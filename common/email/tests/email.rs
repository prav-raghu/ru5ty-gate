#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use ru5ty_gate_config::EnvReader;
use ru5ty_gate_email::{EmailConfig, EmailSender, EmailService, TemplateError, render_template};

fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn renders_variables_and_escapes_html() {
    let html = render_template(
        "verify-email",
        &vars(&[
            ("username", "<b>Pat</b>"),
            ("verificationLink", "https://x.test/?a=1&b=2"),
        ]),
        2026,
    )
    .unwrap();

    assert!(html.contains("&lt;b&gt;Pat&lt;/b&gt;"));
    assert!(html.contains("https://x.test/?a=1&amp;b=2"));
    assert!(html.contains("2026"));
    assert!(!html.contains("{{username}}"));
}

#[test]
fn every_referenced_template_exists() {
    for name in [
        "verify-email",
        "reset-password",
        "admin-login-notification",
        "admin-password-reset",
        "admin-forgot-password",
        "admin-onboarding-notification",
    ] {
        assert!(
            render_template(name, &BTreeMap::new(), 2026).is_ok(),
            "{name}"
        );
    }
}

#[test]
fn unknown_template_is_an_error() {
    assert_eq!(
        render_template("missing", &BTreeMap::new(), 2026),
        Err(TemplateError::NotFound("missing".to_owned()))
    );
}

#[test]
fn config_selects_sandbox_when_inbox_is_set() {
    let sandbox = EmailConfig::from_env(&EnvReader::from_pairs([
        ("MAILTRAP_API_KEY", "k"),
        ("MAILTRAP_TEST_INBOX_ID", "42"),
    ]))
    .unwrap();
    let live = EmailConfig::from_env(&EnvReader::from_pairs([("MAILTRAP_API_KEY", "k")])).unwrap();

    assert_eq!(
        sandbox.send_url(),
        "https://sandbox.api.mailtrap.io/api/send/42"
    );
    assert_eq!(live.send_url(), "https://send.api.mailtrap.io/api/send");
    assert_eq!(live.from_name, "Ru5ty Gate");
}

#[test]
fn config_rejects_non_numeric_inbox() {
    assert!(
        EmailConfig::from_env(&EnvReader::from_pairs([("MAILTRAP_TEST_INBOX_ID", "abc")])).is_err()
    );
}

#[tokio::test]
async fn send_returns_false_without_api_key() {
    let service = EmailService::new(EmailConfig::from_env(&EnvReader::default()).unwrap());

    assert!(
        !service
            .send_mail("a@b.com", "Hi", "verify-email", &BTreeMap::new())
            .await
    );
}
