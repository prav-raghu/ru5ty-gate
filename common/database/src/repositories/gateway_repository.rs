use sqlx::PgExecutor;
use uuid::Uuid;

use crate::database_error::DatabaseError;
use crate::inputs::HeartbeatRecord;
use crate::models::Gateway;

pub struct GatewayRepository;

impl GatewayRepository {
    pub async fn find_active<'e, E>(executor: E, id: Uuid) -> Result<Option<Gateway>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let gateway = sqlx::query_as::<_, Gateway>(
            "SELECT * FROM gateways WHERE id = $1 AND is_active = TRUE",
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;
        Ok(gateway)
    }

    pub async fn list_by_venue<'e, E>(
        executor: E,
        venue_id: Uuid,
    ) -> Result<Vec<Gateway>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let gateways = sqlx::query_as::<_, Gateway>(
            "SELECT * FROM gateways WHERE venue_id = $1 AND is_active = TRUE \
             ORDER BY name, id",
        )
        .bind(venue_id)
        .fetch_all(executor)
        .await?;
        Ok(gateways)
    }

    pub async fn replace_key_hash<'e, E>(
        executor: E,
        id: Uuid,
        api_key_hash: &str,
        modified_by: &str,
    ) -> Result<Option<Gateway>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let gateway = sqlx::query_as::<_, Gateway>(
            "UPDATE gateways SET api_key_hash = $2, modified_by = $3 \
             WHERE id = $1 AND is_active = TRUE RETURNING *",
        )
        .bind(id)
        .bind(api_key_hash)
        .bind(modified_by)
        .fetch_optional(executor)
        .await?;
        Ok(gateway)
    }

    pub async fn record_heartbeat<'e, E>(
        executor: E,
        id: Uuid,
        heartbeat: &HeartbeatRecord,
    ) -> Result<bool, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let result = sqlx::query(
            "UPDATE gateways SET last_heartbeat_at = NOW(), agent_version = $2, \
             uptime_secs = $3, active_sessions = $4, pending_events = $5, \
             last_sync_ok_at = $6 WHERE id = $1",
        )
        .bind(id)
        .bind(&heartbeat.agent_version)
        .bind(heartbeat.uptime_secs)
        .bind(heartbeat.active_sessions)
        .bind(heartbeat.pending_events)
        .bind(heartbeat.last_sync_ok_at)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
