use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_database::sqlx;
use serde_json::json;

use crate::types::AppState;

pub struct HealthController;

impl HealthController {
    pub async fn check_health() -> Response {
        (StatusCode::OK, Json(json!({ "status": "pong" }))).into_response()
    }

    pub async fn get_readiness(State(state): State<AppState>) -> Response {
        let database = sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&state.pool)
            .await
            .is_ok();
        let redis = !state.redis.is_available() || state.redis.ping().await.is_ok();
        if database && redis {
            (
                StatusCode::OK,
                Json(json!({ "status": "ready", "db": "ok", "redis": "ok" })),
            )
                .into_response()
        } else {
            tracing::error!(database, redis, "readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "status": "unavailable",
                    "reason": "Service dependencies unavailable"
                })),
            )
                .into_response()
        }
    }
}
