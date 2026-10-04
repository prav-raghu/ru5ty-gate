#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::http::{HeaderMap, HeaderValue, header};
use ru5ty_gate_http::{
    clear_refresh_cookie_header, refresh_cookie_header, refresh_token_from_cookies,
};

fn cookies(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, HeaderValue::from_str(value).unwrap());
    headers
}

#[test]
fn builds_an_http_only_strict_cookie_scoped_to_the_auth_routes() {
    let value = refresh_cookie_header("abc.def", 86_400, false).unwrap();

    assert_eq!(
        value.to_str().unwrap(),
        "rg_refresh=abc.def; Max-Age=86400; Path=/api/v1/auth; HttpOnly; SameSite=Strict"
    );
}

#[test]
fn adds_secure_when_requested() {
    let value = refresh_cookie_header("abc", 60, true).unwrap();

    assert!(value.to_str().unwrap().ends_with("; Secure"));
}

#[test]
fn rejects_tokens_that_are_not_valid_header_values() {
    assert!(refresh_cookie_header("bad\ntoken", 60, false).is_none());
}

#[test]
fn clears_with_zero_max_age_and_the_same_attributes() {
    let value = clear_refresh_cookie_header(true).unwrap();

    assert_eq!(
        value.to_str().unwrap(),
        "rg_refresh=; Max-Age=0; Path=/api/v1/auth; HttpOnly; SameSite=Strict; Secure"
    );
}

#[test]
fn reads_the_token_among_other_cookies() {
    let headers = cookies("theme=dark; rg_refresh=abc.def; other=1");

    assert_eq!(
        refresh_token_from_cookies(&headers).as_deref(),
        Some("abc.def")
    );
}

#[test]
fn reads_the_token_from_a_second_cookie_header() {
    let mut headers = cookies("theme=dark");
    headers.append(header::COOKIE, HeaderValue::from_static("rg_refresh=xyz"));

    assert_eq!(refresh_token_from_cookies(&headers).as_deref(), Some("xyz"));
}

#[test]
fn ignores_missing_empty_and_similarly_named_cookies() {
    assert!(refresh_token_from_cookies(&HeaderMap::new()).is_none());
    assert!(refresh_token_from_cookies(&cookies("rg_refresh=")).is_none());
    assert!(refresh_token_from_cookies(&cookies("xrg_refresh=abc; rg_refresh_x=abc")).is_none());
}
