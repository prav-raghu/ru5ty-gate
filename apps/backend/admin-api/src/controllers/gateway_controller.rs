use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{ApiPath, AppError, AuthUser, ValidatedJson};
use ru5ty_gate_types::ApiResponse;

use crate::schemas::{CreateGatewayRequest, GatewayIdPath, VenueRefPath};
use crate::types::AppState;

pub struct GatewayController;

impl GatewayController {
    pub async fn create_gateway(
        State(state): State<AppState>,
        user: AuthUser,
        ApiPath(path): ApiPath<VenueRefPath>,
        ValidatedJson(body): ValidatedJson<CreateGatewayRequest>,
    ) -> Result<Response, AppError> {
        let created = state
            .services
            .gateway
            .create(path.venue_ref, &body, &user)
            .await?;
        Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(created).stamped()),
        )
            .into_response())
    }

    pub async fn list_gateways(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<VenueRefPath>,
    ) -> Result<Response, AppError> {
        state.services.venue.find_active(path.venue_ref).await?;
        let gateways = state.services.gateway.list(path.venue_ref).await?;
        Ok(Json(ApiResponse::success(gateways).stamped()).into_response())
    }

    pub async fn rotate_key(
        State(state): State<AppState>,
        user: AuthUser,
        ApiPath(path): ApiPath<GatewayIdPath>,
    ) -> Result<Response, AppError> {
        let rotated = state
            .services
            .gateway
            .rotate_key(path.gateway_id, &user)
            .await?;
        Ok(Json(ApiResponse::success(rotated).stamped()).into_response())
    }
}
