use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{ApiPath, ApiQuery, AppError, AuthUser, ValidatedJson};
use ru5ty_gate_types::ApiResponse;

use crate::schemas::{CreateVenueRequest, PageQuery, UpdateVenueRequest, VenueRefPath};
use crate::types::AppState;

pub struct VenueController;

impl VenueController {
    pub async fn create_venue(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<CreateVenueRequest>,
    ) -> Result<Response, AppError> {
        let venue = state.services.venue.create(&body, &user).await?;
        Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(venue).stamped()),
        )
            .into_response())
    }

    pub async fn list_venues(
        State(state): State<AppState>,
        ApiQuery(query): ApiQuery<PageQuery>,
    ) -> Result<Response, AppError> {
        let venues = state.services.venue.list(&query).await?;
        Ok(Json(ApiResponse::success(venues).stamped()).into_response())
    }

    pub async fn get_venue(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<VenueRefPath>,
    ) -> Result<Response, AppError> {
        let venue = state.services.venue.get(path.venue_ref).await?;
        Ok(Json(ApiResponse::success(venue).stamped()).into_response())
    }

    pub async fn update_venue(
        State(state): State<AppState>,
        user: AuthUser,
        ApiPath(path): ApiPath<VenueRefPath>,
        ValidatedJson(body): ValidatedJson<UpdateVenueRequest>,
    ) -> Result<Response, AppError> {
        let venue = state
            .services
            .venue
            .update(path.venue_ref, &body, &user)
            .await?;
        Ok(Json(ApiResponse::success(venue).stamped()).into_response())
    }
}
