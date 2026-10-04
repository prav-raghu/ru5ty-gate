use chrono::DateTime;
use ru5ty_gate_database::{
    DatabaseError, Gateway, GatewayRepository, HeartbeatRecord, NewGateway, PgPool, Repository,
    Venue,
};
use ru5ty_gate_http::{AppError, AuthUser};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::dtos::{DeviceContext, GatewayDto, GatewayWithKeyDto};
use crate::schemas::{CreateGatewayRequest, HeartbeatBody};

const MAX_SECRET_LENGTH: usize = 128;

#[derive(Clone)]
pub struct GatewayService {
    pool: PgPool,
}

fn new_secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

fn secret_matches(secret: &str, stored_hash: &str) -> bool {
    let provided = hash_secret(secret);
    let expected = stored_hash.as_bytes();
    expected.len() == provided.len() && bool::from(expected.ct_eq(provided.as_bytes()))
}

fn map_error(error: DatabaseError) -> AppError {
    if error.is_unique_violation() {
        AppError::Conflict("A gateway with this name already exists in the venue".to_owned())
    } else {
        AppError::internal(error)
    }
}

fn clamp_to_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

impl GatewayService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        venue_id: Uuid,
        request: &CreateGatewayRequest,
        actor: &AuthUser,
    ) -> Result<GatewayWithKeyDto, AppError> {
        Repository::<Venue>::find_by_id(&self.pool, venue_id)
            .await
            .map_err(AppError::internal)?
            .filter(|venue| venue.is_active)
            .ok_or_else(|| AppError::NotFound("Venue not found".to_owned()))?;

        let id = Uuid::new_v4();
        let secret = new_secret();
        let gateway = Repository::<Gateway>::insert(
            &self.pool,
            &NewGateway {
                id,
                venue_id,
                name: request.name.clone(),
                api_key_hash: hash_secret(&secret),
                created_by: actor.username.clone(),
            },
        )
        .await
        .map_err(map_error)?;
        Ok(GatewayWithKeyDto {
            gateway: gateway.into(),
            api_key: format!("{id}.{secret}"),
        })
    }

    pub async fn list(&self, venue_id: Uuid) -> Result<Vec<GatewayDto>, AppError> {
        let gateways = GatewayRepository::list_by_venue(&self.pool, venue_id)
            .await
            .map_err(AppError::internal)?;
        Ok(gateways.into_iter().map(GatewayDto::from).collect())
    }

    pub async fn rotate_key(
        &self,
        gateway_id: Uuid,
        actor: &AuthUser,
    ) -> Result<GatewayWithKeyDto, AppError> {
        let secret = new_secret();
        let gateway = GatewayRepository::replace_key_hash(
            &self.pool,
            gateway_id,
            &hash_secret(&secret),
            &actor.username,
        )
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("Gateway not found".to_owned()))?;
        Ok(GatewayWithKeyDto {
            gateway: gateway.into(),
            api_key: format!("{gateway_id}.{secret}"),
        })
    }

    pub async fn authenticate(&self, api_key: &str) -> Option<DeviceContext> {
        let (id, secret) = api_key.split_once('.')?;
        if secret.is_empty() || secret.len() > MAX_SECRET_LENGTH {
            return None;
        }
        let gateway_id = Uuid::parse_str(id).ok()?;
        let gateway = GatewayRepository::find_active(&self.pool, gateway_id)
            .await
            .map_err(|error| tracing::error!(%error, "gateway lookup failed"))
            .ok()??;
        if !secret_matches(secret, &gateway.api_key_hash) {
            return None;
        }
        let venue = Repository::<Venue>::find_by_id(&self.pool, gateway.venue_id)
            .await
            .map_err(|error| tracing::error!(%error, "venue lookup failed"))
            .ok()??;
        venue.is_active.then_some(DeviceContext { gateway, venue })
    }

    pub async fn record_heartbeat(
        &self,
        gateway_id: Uuid,
        heartbeat: &HeartbeatBody,
    ) -> Result<(), AppError> {
        let record = HeartbeatRecord {
            agent_version: heartbeat.agent_version.clone(),
            uptime_secs: i64::try_from(heartbeat.uptime_secs).unwrap_or(i64::MAX),
            active_sessions: clamp_to_i32(heartbeat.active_sessions),
            pending_events: clamp_to_i32(heartbeat.pending_events),
            last_sync_ok_at: heartbeat
                .last_sync_ok_at
                .and_then(|seconds| DateTime::from_timestamp(seconds, 0)),
        };
        GatewayRepository::record_heartbeat(&self.pool, gateway_id, &record)
            .await
            .map_err(AppError::internal)?;
        Ok(())
    }
}
