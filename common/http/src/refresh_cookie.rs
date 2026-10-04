use axum::http::header::COOKIE;
use axum::http::{HeaderMap, HeaderValue};

pub const REFRESH_COOKIE_NAME: &str = "rg_refresh";
const REFRESH_COOKIE_PATH: &str = "/api/v1/auth";

fn attributes(secure: bool) -> String {
    let secure_attribute = if secure { "; Secure" } else { "" };
    format!("; Path={REFRESH_COOKIE_PATH}; HttpOnly; SameSite=Strict{secure_attribute}")
}

pub fn refresh_cookie_header(
    token: &str,
    max_age_seconds: i64,
    secure: bool,
) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{REFRESH_COOKIE_NAME}={token}; Max-Age={max_age_seconds}{}",
        attributes(secure)
    ))
    .ok()
}

pub fn clear_refresh_cookie_header(secure: bool) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{REFRESH_COOKIE_NAME}=; Max-Age=0{}",
        attributes(secure)
    ))
    .ok()
}

pub fn refresh_token_from_cookies(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, value)| *name == REFRESH_COOKIE_NAME && !value.is_empty())
        .map(|(_, value)| value.to_owned())
}
