use chrono::{DateTime, Utc};
use ru5ty_gate_database::{
    CaptiveSessionRepository, NewCaptiveSession, PageRequest, PgPool, SessionListFilter, Venue,
};
use ru5ty_gate_http::AppError;
use serde_json::Value;
use uuid::Uuid;

use crate::dtos::{
    CaptiveSessionDto, DeviceContext, PolicyResponseBody, SyncResponseBody,
    ValidateSessionResponseBody,
};
use crate::schemas::{SessionListQuery, SyncBatchBody, SyncEventBody, ValidateSessionBody};

const MAX_IDENTIFIER_LENGTH: usize = 128;
const MAX_REASON_LENGTH: usize = 64;

#[derive(Clone)]
pub struct CaptiveSessionService {
    pool: PgPool,
}

fn duration_secs(venue: &Venue) -> u64 {
    u64::try_from(venue.session_duration_secs).unwrap_or(0)
}

fn text(payload: &Value, keys: &[&str], max_length: usize) -> Option<String> {
    keys.iter()
        .find_map(|key| payload.get(*key)?.as_str())
        .filter(|value| !value.is_empty() && value.len() <= max_length)
        .map(str::to_owned)
}

fn timestamp(payload: &Value, key: &str) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(payload.get(key)?.as_i64()?, 0)
}

impl CaptiveSessionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn validate(
        context: &DeviceContext,
        _request: &ValidateSessionBody,
    ) -> ValidateSessionResponseBody {
        if !context.venue.allow_new_sessions {
            return ValidateSessionResponseBody {
                allow: false,
                session_seconds: None,
                redirect_url: None,
                reason: Some("venue is not accepting new sessions".to_owned()),
            };
        }
        ValidateSessionResponseBody {
            allow: true,
            session_seconds: Some(duration_secs(&context.venue)),
            redirect_url: context.venue.redirect_url.clone(),
            reason: None,
        }
    }

    pub fn policy(context: &DeviceContext) -> PolicyResponseBody {
        PolicyResponseBody {
            session_duration_secs: duration_secs(&context.venue),
            redirect_url: context.venue.redirect_url.clone(),
        }
    }

    pub async fn sync(
        &self,
        context: &DeviceContext,
        batch: &SyncBatchBody,
    ) -> Result<SyncResponseBody, AppError> {
        let mut transaction = self.pool.begin().await.map_err(AppError::internal)?;
        let mut accepted = 0;
        for event in &batch.events {
            if self.apply(&mut transaction, context, event).await? {
                accepted += 1;
            } else {
                tracing::warn!(
                    gateway_id = %context.gateway.id,
                    event_type = %event.event_type,
                    "ignored a sync event that could not be applied"
                );
            }
        }
        transaction.commit().await.map_err(AppError::internal)?;
        Ok(SyncResponseBody { accepted })
    }

    async fn apply(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        context: &DeviceContext,
        event: &SyncEventBody,
    ) -> Result<bool, AppError> {
        let payload = &event.payload;
        let Some(mac) = text(payload, &["mac", "mac_hash"], MAX_IDENTIFIER_LENGTH) else {
            return Ok(false);
        };
        let mac = mac.to_ascii_lowercase();
        let occurred_at = DateTime::from_timestamp(event.occurred_at, 0);

        match event.event_type.as_str() {
            "session_start" => {
                let granted_at = timestamp(payload, "granted_at").or(occurred_at);
                let (Some(granted_at), Some(expires_at)) =
                    (granted_at, timestamp(payload, "expires_at"))
                else {
                    return Ok(false);
                };
                if expires_at <= granted_at {
                    return Ok(false);
                }
                CaptiveSessionRepository::record_start(
                    &mut **transaction,
                    &NewCaptiveSession {
                        venue_id: context.venue.id,
                        gateway_id: context.gateway.id,
                        mac_identifier: mac,
                        client_identifier: text(
                            payload,
                            &["client_ip", "client_ip_hash"],
                            MAX_IDENTIFIER_LENGTH,
                        ),
                        gateway_name: text(payload, &["gateway_name"], MAX_IDENTIFIER_LENGTH),
                        granted_at,
                        expires_at,
                    },
                )
                .await
                .map_err(AppError::internal)?;
                Ok(true)
            }
            "session_end" => {
                let Some(ended_at) = timestamp(payload, "ended_at").or(occurred_at) else {
                    return Ok(false);
                };
                let reason = text(payload, &["reason"], MAX_REASON_LENGTH)
                    .unwrap_or_else(|| "unknown".to_owned());
                CaptiveSessionRepository::record_end(
                    &mut **transaction,
                    context.gateway.id,
                    &mac,
                    ended_at,
                    &reason,
                )
                .await
                .map_err(AppError::internal)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub async fn list(
        &self,
        venue_id: Uuid,
        query: &SessionListQuery,
    ) -> Result<Vec<CaptiveSessionDto>, AppError> {
        let page = PageRequest::new(query.limit, query.offset);
        let sessions = CaptiveSessionRepository::list(
            &self.pool,
            &SessionListFilter {
                venue_id,
                open_only: query.open_only.unwrap_or(false),
                limit: page.limit,
                offset: page.offset,
            },
        )
        .await
        .map_err(AppError::internal)?;
        Ok(sessions.into_iter().map(CaptiveSessionDto::from).collect())
    }
}
