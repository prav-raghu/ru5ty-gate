use ru5ty_gate_database::{
    DatabaseError, NewVenue, PageRequest, PgPool, Repository, Venue, VenueChanges, VenueRepository,
};
use ru5ty_gate_http::{AppError, AuthUser};
use uuid::Uuid;

use crate::dtos::VenueDto;
use crate::schemas::{CreateVenueRequest, PageQuery, UpdateVenueRequest};

const DEFAULT_SESSION_SECS: i32 = 3600;

#[derive(Clone)]
pub struct VenueService {
    pool: PgPool,
}

fn map_error(error: DatabaseError) -> AppError {
    if error.is_unique_violation() {
        AppError::Conflict("A venue with this code already exists".to_owned())
    } else {
        AppError::internal(error)
    }
}

impl VenueService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        request: &CreateVenueRequest,
        actor: &AuthUser,
    ) -> Result<VenueDto, AppError> {
        let venue = Repository::<Venue>::insert(
            &self.pool,
            &NewVenue {
                code: request.code.clone(),
                name: request.name.clone(),
                session_duration_secs: request
                    .session_duration_secs
                    .unwrap_or(DEFAULT_SESSION_SECS),
                redirect_url: request.redirect_url.clone(),
                allow_new_sessions: request.allow_new_sessions.unwrap_or(true),
                created_by: actor.username.clone(),
            },
        )
        .await
        .map_err(map_error)?;
        Ok(venue.into())
    }

    pub async fn list(&self, query: &PageQuery) -> Result<Vec<VenueDto>, AppError> {
        let venues =
            Repository::<Venue>::list(&self.pool, PageRequest::new(query.limit, query.offset))
                .await
                .map_err(AppError::internal)?;
        Ok(venues
            .into_iter()
            .filter(|venue| venue.is_active)
            .map(VenueDto::from)
            .collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<VenueDto, AppError> {
        self.find_active(id).await.map(VenueDto::from)
    }

    pub async fn find_active(&self, id: Uuid) -> Result<Venue, AppError> {
        Repository::<Venue>::find_by_id(&self.pool, id)
            .await
            .map_err(AppError::internal)?
            .filter(|venue| venue.is_active)
            .ok_or_else(|| AppError::NotFound("Venue not found".to_owned()))
    }

    pub async fn find_active_by_code(&self, code: &str) -> Result<Option<Venue>, AppError> {
        VenueRepository::find_active_by_code(&self.pool, code)
            .await
            .map_err(AppError::internal)
    }

    pub async fn update(
        &self,
        id: Uuid,
        request: &UpdateVenueRequest,
        actor: &AuthUser,
    ) -> Result<VenueDto, AppError> {
        let current = self.find_active(id).await?;
        let redirect_url = if request.clear_redirect_url.unwrap_or(false) {
            None
        } else {
            request.redirect_url.clone().or(current.redirect_url)
        };
        let changes = VenueChanges {
            name: request.name.clone().unwrap_or(current.name),
            session_duration_secs: request
                .session_duration_secs
                .unwrap_or(current.session_duration_secs),
            redirect_url,
            allow_new_sessions: request
                .allow_new_sessions
                .unwrap_or(current.allow_new_sessions),
            modified_by: actor.username.clone(),
        };
        Repository::<Venue>::update_by_id(&self.pool, id, &changes)
            .await
            .map_err(map_error)?
            .map(VenueDto::from)
            .ok_or_else(|| AppError::NotFound("Venue not found".to_owned()))
    }
}
