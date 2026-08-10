use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use central_client::ValidateSessionRequest;
use session_store::{Session, SyncEventKind};

use crate::opennds::FasQuery;
use crate::AppState;

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// `GET /fas` -- the endpoint openNDS calls, with the client's MAC/IP,
/// gateway identity, and per-attempt token in the query string. We decide
/// grant/deny and, on grant, redirect the client's browser back to
/// openNDS's own auth endpoint so it can install the firewall rule.
pub async fn fas_auth(State(state): State<AppState>, Query(query): Query<FasQuery>) -> Response {
    if let Some(expected) = &state.config.gateway_name {
        if &query.gatewayname != expected {
            tracing::warn!(
                got = %query.gatewayname,
                expected = %expected,
                "rejecting FAS request for unexpected gateway (v1 is single-gateway)"
            );
            return (
                StatusCode::BAD_REQUEST,
                "unrecognized gateway for this agent instance",
            )
                .into_response();
        }
    }

    let decision = decide(&state, &query).await;

    match decision {
        Decision::Grant {
            duration_secs,
            redirect_url,
        } => {
            let now = now_unix();
            let session = Session {
                mac: query.clientmac.clone(),
                token: query.hid.clone(),
                venue: state.config.venue_id.clone(),
                granted_at: now,
                expires_at: now + duration_secs as i64,
                redirect_url: redirect_url.clone(),
            };

            if let Err(err) = state.store.upsert_session(session).await {
                tracing::error!(?err, mac = %query.clientmac, "failed to persist granted session locally");
            }

            if let Err(err) = state
                .store
                .enqueue_event(
                    SyncEventKind::SessionStart,
                    json!({
                        "mac": query.clientmac,
                        "venue": state.config.venue_id,
                        "gateway_name": query.gatewayname,
                        "client_ip": query.clientip,
                        "granted_at": now,
                        "expires_at": now + duration_secs as i64,
                    }),
                    now,
                )
                .await
            {
                tracing::error!(?err, "failed to enqueue session_start sync event");
            }

            tracing::info!(mac = %query.clientmac, duration_secs, "granted client session");

            let location = query.auth_redirect_url();
            (StatusCode::FOUND, [(header::LOCATION, location)]).into_response()
        }
        Decision::Deny { reason } => {
            tracing::info!(mac = %query.clientmac, %reason, "denied client session");
            (
                StatusCode::FORBIDDEN,
                format!("Access denied by captive portal agent: {reason}"),
            )
                .into_response()
        }
    }
}

enum Decision {
    Grant {
        duration_secs: u64,
        redirect_url: Option<String>,
    },
    Deny {
        reason: String,
    },
}

/// Ask the central platform whether to grant this client, falling back to
/// local policy if the central platform is unreachable. This is the
/// offline-tolerance behavior called out in the spec: an outage on the
/// central-platform side should never itself be the reason a client at the
/// gateway is denied, unless the operator has explicitly configured
/// fail-closed via `allow_offline = false`.
async fn decide(state: &AppState, query: &FasQuery) -> Decision {
    let req = ValidateSessionRequest {
        mac: query.clientmac.clone(),
        token: query.hid.clone(),
        gateway_name: query.gatewayname.clone(),
        client_ip: query.clientip.clone(),
    };

    match state.central.validate_session(&req).await {
        Ok(resp) if resp.allow => Decision::Grant {
            duration_secs: resp
                .session_seconds
                .unwrap_or(state.config.default_session_secs),
            redirect_url: resp.redirect_url,
        },
        Ok(resp) => Decision::Deny {
            reason: resp
                .reason
                .unwrap_or_else(|| "denied by central platform".to_string()),
        },
        Err(err) => {
            tracing::warn!(
                ?err,
                "central platform unreachable, falling back to local policy"
            );
            if state.config.allow_offline {
                Decision::Grant {
                    duration_secs: state.config.default_session_secs,
                    redirect_url: None,
                }
            } else {
                Decision::Deny {
                    reason: "central platform unreachable and fail-closed policy is set"
                        .to_string(),
                }
            }
        }
    }
}

/// `GET /health` -- liveness probe for the agent process itself.
pub async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

/// `GET /status` -- lightweight local diagnostics: active session count
/// and how many sync events are still buffered waiting for the central
/// platform.
pub async fn status(State(state): State<AppState>) -> Response {
    let now = now_unix();
    let active_sessions = state.store.active_session_count(now).await.unwrap_or(-1);
    let pending_sync_events = state.store.pending_event_count().await.unwrap_or(-1);

    Json(json!({
        "venue_id": state.config.venue_id,
        "active_sessions": active_sessions,
        "pending_sync_events": pending_sync_events,
    }))
    .into_response()
}
