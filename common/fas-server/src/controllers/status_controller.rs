use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_session_store::unix_now;
use serde_json::json;

use crate::AppState;

pub struct StatusController;

impl StatusController {
    pub async fn status(State(state): State<AppState>) -> Response {
        let active_sessions = state
            .store
            .active_session_count(unix_now())
            .await
            .unwrap_or(-1);
        let pending_sync_events = state.store.pending_event_count().await.unwrap_or(-1);

        Json(json!({
            "venue_id": state.config.venue_id,
            "active_sessions": active_sessions,
            "pending_sync_events": pending_sync_events,
        }))
        .into_response()
    }
}
