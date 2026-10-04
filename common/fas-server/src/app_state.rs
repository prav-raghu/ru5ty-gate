use ru5ty_gate_central_client::{CentralClient, IdentifierPolicy};
use ru5ty_gate_session_store::SessionStore;

use crate::FasConfig;

#[derive(Clone)]
pub struct AppState {
    pub store: SessionStore,
    pub central: CentralClient,
    pub config: FasConfig,
    pub identifiers: IdentifierPolicy,
}
