use std::net::IpAddr;

use axum::extract::{Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use ru5ty_gate_central_client::ValidateSessionRequest;
use ru5ty_gate_session_store::{Session, StoredPolicy, SyncEventKind, clock_is_trusted, unix_now};
use secrecy::ExposeSecret;
use serde_json::json;

use crate::decision::Decision;
use crate::peer_ip::PeerIp;
use crate::{AppState, FasError, FasPayload, FasPayloadDecoder, FasRequest, FasToken};

pub struct FasController;

impl FasController {
    pub async fn auth(
        State(state): State<AppState>,
        PeerIp(peer): PeerIp,
        Query(request): Query<FasRequest>,
    ) -> Response {
        match Self::handle(&state, peer, request).await {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(%error, "rejected FAS request");
                error.into_response()
            }
        }
    }

    async fn handle(
        state: &AppState,
        peer: Option<IpAddr>,
        request: FasRequest,
    ) -> Result<Response, FasError> {
        let encoded = request.fas.ok_or(FasError::MissingPayload)?;
        let payload = FasPayloadDecoder::decode(&encoded)?;
        Self::check_gateway(state, &payload)?;
        Self::check_peer(state, &payload, peer)?;

        let policy = state.store.load_policy().await.ok().flatten();
        let mac_label = state.identifiers.log_mac(&payload.clientmac);

        match Self::decide(state, &payload, policy.as_ref()).await {
            Decision::Grant {
                duration_secs,
                redirect_url,
            } => {
                let redirect = redirect_url
                    .filter(|url| Self::is_http_url(url))
                    .or_else(|| policy.and_then(|policy| policy.redirect_url))
                    .or_else(|| payload.originurl.clone());
                Self::grant(state, &payload, duration_secs, redirect, &mac_label).await
            }
            Decision::Deny { reason } => {
                tracing::info!(mac = %mac_label, %reason, "denied client session");
                Ok((
                    StatusCode::FORBIDDEN,
                    format!("Access denied by captive portal agent: {reason}"),
                )
                    .into_response())
            }
        }
    }

    fn check_gateway(state: &AppState, payload: &FasPayload) -> Result<(), FasError> {
        match &state.config.gateway_name {
            Some(expected) if payload.gatewayname != *expected => Err(FasError::UnexpectedGateway),
            _ => Ok(()),
        }
    }

    fn check_peer(
        state: &AppState,
        payload: &FasPayload,
        peer: Option<IpAddr>,
    ) -> Result<(), FasError> {
        if !state.config.verify_client_ip {
            return Ok(());
        }
        let claimed = payload
            .clientip
            .parse::<IpAddr>()
            .map(|ip| ip.to_canonical())
            .map_err(|_| FasError::InvalidField("clientip"))?;
        if peer == Some(claimed) {
            Ok(())
        } else {
            Err(FasError::UntrustedPeer)
        }
    }

    fn is_http_url(value: &str) -> bool {
        value.len() <= 2048 && (value.starts_with("http://") || value.starts_with("https://"))
    }

    fn local_duration(state: &AppState, policy: Option<&StoredPolicy>) -> u64 {
        policy.map_or(state.config.default_session_secs, |policy| {
            policy.session_duration_secs
        })
    }

    async fn decide(
        state: &AppState,
        payload: &FasPayload,
        policy: Option<&StoredPolicy>,
    ) -> Decision {
        let request = ValidateSessionRequest {
            mac: state.identifiers.mac(&payload.clientmac),
            token: payload.hid.clone(),
            gateway_name: payload.gatewayname.clone(),
            client_ip: state.identifiers.ip(&payload.clientip),
        };

        match state.central.validate_session(&request).await {
            Ok(response) if response.allow => Decision::Grant {
                duration_secs: response
                    .session_seconds
                    .unwrap_or_else(|| Self::local_duration(state, policy)),
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
                        duration_secs: Self::local_duration(state, policy),
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

    async fn grant(
        state: &AppState,
        payload: &FasPayload,
        duration_secs: u64,
        redirect: Option<String>,
        mac_label: &str,
    ) -> Result<Response, FasError> {
        let now = unix_now();
        let expires_at = now.saturating_add(i64::try_from(duration_secs).unwrap_or(i64::MAX));
        let session = Session {
            mac: payload.clientmac.clone(),
            token: payload.hid.clone(),
            venue: state.config.venue_id.clone(),
            granted_at: now,
            expires_at,
            redirect_url: redirect.clone(),
            clock_untrusted: !clock_is_trusted(now),
        };

        if let Err(err) = state.store.upsert_session(session).await {
            tracing::error!(?err, mac = %mac_label, "failed to persist granted session locally");
        }

        let event = json!({
            "mac": payload.clientmac,
            "venue": state.config.venue_id,
            "gateway_name": payload.gatewayname,
            "client_ip": payload.clientip,
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

        tracing::info!(mac = %mac_label, duration_secs, "granted client session");

        let token = FasToken::return_token(&payload.hid, state.config.faskey.expose_secret());
        let location = Self::auth_url(payload, &token, redirect.as_deref());
        let location = HeaderValue::from_str(&location)
            .map_err(|_| FasError::InvalidField("gatewayaddress"))?;
        Ok((StatusCode::FOUND, [(header::LOCATION, location)]).into_response())
    }

    fn auth_url(payload: &FasPayload, token: &str, redirect: Option<&str>) -> String {
        let base = format!(
            "http://{}/{}/?tok={}",
            payload.gatewayaddress, payload.authdir, token
        );
        match redirect {
            Some(url) => format!("{base}&redir={}", urlencoding::encode(url)),
            None => base,
        }
    }
}
