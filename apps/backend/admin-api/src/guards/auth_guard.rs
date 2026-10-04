use async_trait::async_trait;
use ru5ty_gate_auth::TokenService;
use ru5ty_gate_http::{AuthUser, Authenticator};
use ru5ty_gate_types::TokenScope;
use uuid::Uuid;

use crate::services::UserService;

pub struct AuthGuard {
    tokens: TokenService,
    users: UserService,
}

impl AuthGuard {
    pub fn new(tokens: TokenService, users: UserService) -> Self {
        Self { tokens, users }
    }
}

#[async_trait]
impl Authenticator for AuthGuard {
    async fn authenticate(&self, bearer_token: &str) -> Option<AuthUser> {
        let payload = self.tokens.verify_access_token(bearer_token)?;
        if payload.scope != TokenScope::Admin {
            return None;
        }
        if self.tokens.is_token_blacklisted(&payload.jti).await {
            return None;
        }
        if self
            .tokens
            .is_session_invalidated(&payload.id, payload.iat)
            .await
        {
            return None;
        }
        let user_id = Uuid::parse_str(&payload.id).ok()?;
        let user = self.users.get_authorized_admin(user_id).await.ok()??;
        Some(AuthUser {
            id: user.id,
            username: user.username,
            email: Some(user.email),
            role: payload.role,
            permissions: payload.permissions,
            scope: payload.scope,
        })
    }
}
