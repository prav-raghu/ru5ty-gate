use std::sync::Arc;

use ru5ty_gate_http::Authenticator;

use crate::guards::AuthGuard;
use crate::types::AppState;

pub fn build_authenticator(state: &AppState) -> Arc<dyn Authenticator> {
    Arc::new(AuthGuard::new(
        state.services.token.clone(),
        state.services.user.clone(),
    ))
}
