use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use ru5ty_gate_http::{ApiPath, AppError, ValidatedJson};

use crate::dtos::{
    DeviceContext, PolicyResponseBody, SyncResponseBody, ValidateSessionResponseBody,
};
use crate::schemas::{DeviceVenuePath, HeartbeatBody, SyncBatchBody, ValidateSessionBody};
use crate::services::CaptiveSessionService;
use crate::types::AppState;

fn ensure_venue(context: &DeviceContext, venue_ref: &str) -> Result<(), AppError> {
    if context.venue.code == venue_ref {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "This gateway does not belong to the requested venue".to_owned(),
        ))
    }
}

fn ensure_body_venue(context: &DeviceContext, venue_id: &str) -> Result<(), AppError> {
    if context.venue.code == venue_id {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "The venue_id in the body does not match the gateway's venue".to_owned(),
        ))
    }
}

pub struct DeviceController;

impl DeviceController {
    pub async fn validate_session(
        context: DeviceContext,
        ApiPath(path): ApiPath<DeviceVenuePath>,
        ValidatedJson(body): ValidatedJson<ValidateSessionBody>,
    ) -> Result<Json<ValidateSessionResponseBody>, AppError> {
        ensure_venue(&context, &path.venue_ref)?;
        Ok(Json(CaptiveSessionService::validate(&context, &body)))
    }

    pub async fn policy(
        context: DeviceContext,
        ApiPath(path): ApiPath<DeviceVenuePath>,
    ) -> Result<Json<PolicyResponseBody>, AppError> {
        ensure_venue(&context, &path.venue_ref)?;
        Ok(Json(CaptiveSessionService::policy(&context)))
    }

    pub async fn heartbeat(
        State(state): State<AppState>,
        context: DeviceContext,
        ApiPath(path): ApiPath<DeviceVenuePath>,
        ValidatedJson(body): ValidatedJson<HeartbeatBody>,
    ) -> Result<StatusCode, AppError> {
        ensure_venue(&context, &path.venue_ref)?;
        ensure_body_venue(&context, &body.venue_id)?;
        state
            .services
            .gateway
            .record_heartbeat(context.gateway.id, &body)
            .await?;
        Ok(StatusCode::NO_CONTENT)
    }

    pub async fn sync(
        State(state): State<AppState>,
        context: DeviceContext,
        ApiPath(path): ApiPath<DeviceVenuePath>,
        ValidatedJson(body): ValidatedJson<SyncBatchBody>,
    ) -> Result<Json<SyncResponseBody>, AppError> {
        ensure_venue(&context, &path.venue_ref)?;
        ensure_body_venue(&context, &body.venue_id)?;
        let result = state.services.captive_session.sync(&context, &body).await?;
        Ok(Json(result))
    }
}
