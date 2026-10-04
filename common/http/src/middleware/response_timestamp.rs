use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::Method;
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::middleware::Next;
use axum::response::Response;
use chrono::{SecondsFormat, Utc};
use serde_json::Value;

const MAX_BUFFERED_BODY_BYTES: usize = 16 * 1024 * 1024;

pub async fn response_timestamp(request: Request, next: Next) -> Response {
    let skip = request.method() == Method::OPTIONS || request.uri().path().starts_with("/docs");
    let response = next.run(request).await;
    let is_json = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if skip || !is_json {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, MAX_BUFFERED_BODY_BYTES).await else {
        return Response::from_parts(parts, Body::empty());
    };
    let stamped = match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(mut object)) => {
            object.insert(
                "responseDateTime".to_owned(),
                Value::String(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
            );
            serde_json::to_vec(&Value::Object(object)).unwrap_or_else(|_| bytes.to_vec())
        }
        _ => bytes.to_vec(),
    };
    parts.headers.remove(CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(stamped))
}
