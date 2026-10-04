use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::database_error::DatabaseError;
use crate::inputs::{NewCaptiveSession, SessionListFilter};
use crate::models::CaptiveSession;

pub struct CaptiveSessionRepository;

impl CaptiveSessionRepository {
    pub async fn record_start<'e, E>(
        executor: E,
        session: &NewCaptiveSession,
    ) -> Result<bool, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let result = sqlx::query(
            "INSERT INTO captive_sessions \
             (venue_id, gateway_id, mac_identifier, client_identifier, gateway_name, \
              granted_at, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (gateway_id, mac_identifier, granted_at) DO NOTHING",
        )
        .bind(session.venue_id)
        .bind(session.gateway_id)
        .bind(&session.mac_identifier)
        .bind(&session.client_identifier)
        .bind(&session.gateway_name)
        .bind(session.granted_at)
        .bind(session.expires_at)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn record_end<'e, E>(
        executor: E,
        gateway_id: Uuid,
        mac_identifier: &str,
        ended_at: DateTime<Utc>,
        reason: &str,
    ) -> Result<bool, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let result = sqlx::query(
            "UPDATE captive_sessions SET ended_at = $3, end_reason = $4 \
             WHERE id = ( \
                SELECT id FROM captive_sessions \
                WHERE gateway_id = $1 AND mac_identifier = $2 \
                  AND ended_at IS NULL AND granted_at <= $3 \
                ORDER BY granted_at DESC LIMIT 1)",
        )
        .bind(gateway_id)
        .bind(mac_identifier)
        .bind(ended_at)
        .bind(reason)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list<'e, E>(
        executor: E,
        filter: &SessionListFilter,
    ) -> Result<Vec<CaptiveSession>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let sessions = sqlx::query_as::<_, CaptiveSession>(
            "SELECT * FROM captive_sessions \
             WHERE venue_id = $1 AND ($2 = FALSE OR ended_at IS NULL) \
             ORDER BY granted_at DESC, id LIMIT $3 OFFSET $4",
        )
        .bind(filter.venue_id)
        .bind(filter.open_only)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(executor)
        .await?;
        Ok(sessions)
    }
}
