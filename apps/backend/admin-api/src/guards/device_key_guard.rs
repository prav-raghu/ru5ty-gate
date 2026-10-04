use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::AppError;

use crate::services::GatewayService;

pub async fn device_key_guard(
    State(gateways): State<GatewayService>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| value.trim().to_owned());
    let Some(token) = token else {
        return AppError::Unauthorized.into_response();
    };
    match gateways.authenticate(&token).await {
        Some(context) => {
            request.extensions_mut().insert(context);
            next.run(request).await
        }
        None => AppError::Unauthorized.into_response(),
    }
}
