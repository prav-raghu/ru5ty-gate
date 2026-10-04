use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_types::ApiResponse;
use tower_http::catch_panic::CatchPanicLayer;

pub async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::<()>::failure("Not found")),
    )
        .into_response()
}

fn panic_response(_payload: Box<dyn std::any::Any + Send + 'static>) -> Response {
    tracing::error!("request handler panicked");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<()>::failure("Internal server error")),
    )
        .into_response()
}

pub fn catch_panic_layer()
-> CatchPanicLayer<fn(Box<dyn std::any::Any + Send + 'static>) -> Response> {
    CatchPanicLayer::custom(panic_response as fn(_) -> Response)
}
