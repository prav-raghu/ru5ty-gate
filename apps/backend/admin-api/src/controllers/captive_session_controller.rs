use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{ApiPath, ApiQuery, AppError};
use ru5ty_gate_types::ApiResponse;

use crate::schemas::{SessionListQuery, VenueRefPath};
use crate::types::AppState;

pub struct CaptiveSessionController;

impl CaptiveSessionController {
    pub async fn list_sessions(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<VenueRefPath>,
        ApiQuery(query): ApiQuery<SessionListQuery>,
    ) -> Result<Response, AppError> {
        state.services.venue.find_active(path.venue_ref).await?;
        let sessions = state
            .services
            .captive_session
            .list(path.venue_ref, &query)
            .await?;
        Ok(Json(ApiResponse::success(sessions).stamped()).into_response())
    }
}
