use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use ru5ty_gate_session_store::unix_now;
use serde::Deserialize;

use crate::AppState;
use crate::fas_payload_validators::is_valid_mac;

const DEAUTH_METHODS: [&str; 7] = [
    "client_deauth",
    "idle_deauth",
    "timeout_deauth",
    "downquota_deauth",
    "upquota_deauth",
    "ndsctl_deauth",
    "shutdown_deauth",
];

#[derive(Debug, Clone, Deserialize)]
pub struct BinauthEvent {
    pub mac: String,
    pub method: String,
}

pub struct BinauthController;

impl BinauthController {
    pub async fn event(
        State(state): State<AppState>,
        Json(event): Json<BinauthEvent>,
    ) -> StatusCode {
        if !is_valid_mac(&event.mac) {
            return StatusCode::BAD_REQUEST;
        }
        if !DEAUTH_METHODS.contains(&event.method.as_str()) {
            return StatusCode::NO_CONTENT;
        }
        match state
            .store
            .end_session(&event.mac, event.method.as_str(), unix_now())
            .await
        {
            Ok(Some(_)) => {
                tracing::info!(
                    mac = %state.identifiers.log_mac(&event.mac),
                    method = %event.method,
                    "ended client session after openNDS deauthentication"
                );
                StatusCode::NO_CONTENT
            }
            Ok(None) => StatusCode::NO_CONTENT,
            Err(err) => {
                tracing::error!(?err, "failed to end session after deauthentication");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
