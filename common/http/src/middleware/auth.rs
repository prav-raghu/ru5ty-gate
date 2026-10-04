use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{FromRequestParts, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_types::{Permission, TokenScope};
use uuid::Uuid;

use crate::app_error::AppError;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub permissions: Vec<Permission>,
    pub scope: TokenScope,
}

#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn authenticate(&self, bearer_token: &str) -> Option<AuthUser>;
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}

fn bearer_token(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .split(' ')
        .nth(1)
        .filter(|token| !token.is_empty())
}

pub async fn authenticate(
    State(authenticator): State<Arc<dyn Authenticator>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = bearer_token(&request).map(str::to_owned) else {
        return AppError::Unauthorized.into_response();
    };
    match authenticator.authenticate(&token).await {
        Some(user) => {
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        None => AppError::Unauthorized.into_response(),
    }
}

pub async fn require_permission(
    State(required): State<Permission>,
    request: Request,
    next: Next,
) -> Response {
    let allowed = request
        .extensions()
        .get::<AuthUser>()
        .is_some_and(|user| user.permissions.contains(&required));
    if allowed {
        next.run(request).await
    } else {
        AppError::Forbidden("Forbidden: insufficient permissions".to_owned()).into_response()
    }
}
