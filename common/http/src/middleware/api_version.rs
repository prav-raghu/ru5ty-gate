use axum::Json;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

const SUPPORTED: [&str; 2] = ["v1", "v2"];
const DEFAULT_VERSION: &str = "v1";
const DEPRECATED_VERSION: &str = "v1";
const SUNSET: &str = "2026-12-31";
const DEPRECATION_MESSAGE: &str = "v1 will be deprecated on 2026-12-31. Please migrate to v2.";

#[derive(Debug, Clone)]
pub struct ApiVersion(pub String);

fn version_from_accept(accept: &str) -> Option<String> {
    let marker = "application/vnd.api.";
    let start = accept.find(marker)? + marker.len();
    let rest = &accept[start..];
    let end = rest.find("+json")?;
    let version = &rest[..end];
    let valid = version.starts_with('v')
        && version.len() > 1
        && version[1..]
            .chars()
            .all(|character| character.is_ascii_digit());
    valid.then(|| version.to_owned())
}

fn version_from_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/api/")?;
    let segment = rest.split('/').next()?;
    let valid = segment.starts_with('v')
        && segment.len() > 1
        && segment[1..]
            .chars()
            .all(|character| character.is_ascii_digit());
    (valid && rest.len() > segment.len()).then(|| segment.to_owned())
}

pub async fn api_version(mut request: Request, next: Next) -> Response {
    let headers = request.headers();
    let detected = headers
        .get("api-version")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .or_else(|| {
            headers
                .get("accept")
                .and_then(|value| value.to_str().ok())
                .and_then(version_from_accept)
        })
        .or_else(|| version_from_path(request.uri().path()))
        .unwrap_or_else(|| DEFAULT_VERSION.to_owned());

    if !SUPPORTED.contains(&detected.as_str()) {
        let body = json!({
            "success": false,
            "error": "Unsupported API version",
            "supportedVersions": SUPPORTED,
        });
        return (StatusCode::BAD_REQUEST, Json(body)).into_response();
    }

    request
        .extensions_mut()
        .insert(ApiVersion(detected.clone()));
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&detected) {
        headers.insert("x-api-version", value);
    }
    if detected == DEPRECATED_VERSION {
        headers.insert("deprecation", HeaderValue::from_static("true"));
        headers.insert("sunset", HeaderValue::from_static(SUNSET));
        headers.insert(
            "x-api-deprecation-info",
            HeaderValue::from_static(DEPRECATION_MESSAGE),
        );
    }
    response
}
