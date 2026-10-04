use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use ru5ty_gate_central_client::ValidateSessionRequest;
use ru5ty_gate_session_store::{Session, SyncEventKind, unix_now};
use serde_json::json;

use crate::decision::Decision;
use crate::{AppState, FasQuery};

pub struct FasController;

impl FasController {
    pub async fn auth(State(state): State<AppState>, Query(query): Query<FasQuery>) -> Response {
        if let Some(expected) = &state.config.gateway_name
            && query.gatewayname != *expected
        {
            tracing::warn!(
                got = %query.gatewayname,
                expected = %expected,
                "rejecting FAS request for unexpected gateway"
            );
            return (
                StatusCode::BAD_REQUEST,
                "unrecognized gateway for this agent instance",
            )
                .into_response();
        }

        match Self::decide(&state, &query).await {
            Decision::Grant {
                duration_secs,
                redirect_url,
            } => Self::grant(&state, &query, duration_secs, redirect_url).await,
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

    async fn grant(
        state: &AppState,
        query: &FasQuery,
        duration_secs: u64,
        redirect_url: Option<String>,
    ) -> Response {
        let now = unix_now();
        let expires_at = now.saturating_add(i64::try_from(duration_secs).unwrap_or(i64::MAX));
        let session = Session {
            mac: query.clientmac.clone(),
            token: query.hid.clone(),
            venue: state.config.venue_id.clone(),
            granted_at: now,
            expires_at,
            redirect_url,
        };

        if let Err(err) = state.store.upsert_session(session).await {
            tracing::error!(?err, mac = %query.clientmac, "failed to persist granted session locally");
        }

        let event = json!({
            "mac": query.clientmac,
            "venue": state.config.venue_id,
            "gateway_name": query.gatewayname,
            "client_ip": query.clientip,
            "granted_at": now,
            "expires_at": expires_at,
        });
        if let Err(err) = state
            .store
            .enqueue_event(SyncEventKind::SessionStart, event, now)
            .await
        {
            tracing::error!(?err, "failed to enqueue session_start sync event");
        }

        tracing::info!(mac = %query.clientmac, duration_secs, "granted client session");

        (
            StatusCode::FOUND,
            [(header::LOCATION, query.auth_redirect_url())],
        )
            .into_response()
    }

    async fn decide(state: &AppState, query: &FasQuery) -> Decision {
        let request = ValidateSessionRequest {
            mac: query.clientmac.clone(),
            token: query.hid.clone(),
            gateway_name: query.gatewayname.clone(),
            client_ip: query.clientip.clone(),
        };

        match state.central.validate_session(&request).await {
            Ok(response) if response.allow => Decision::Grant {
                duration_secs: response
                    .session_seconds
                    .unwrap_or(state.config.default_session_secs),
                redirect_url: response.redirect_url,
            },
            Ok(response) => Decision::Deny {
                reason: response
                    .reason
                    .unwrap_or_else(|| "denied by central platform".to_owned()),
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
                            .to_owned(),
                    }
                }
            }
        }
    }
}
