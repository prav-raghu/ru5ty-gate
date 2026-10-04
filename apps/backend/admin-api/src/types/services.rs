use ru5ty_gate_auth::TokenService;

use crate::services::{
    AuthService, BatchOperationService, BootstrapService, ReportingService, UserService,
};

pub struct Services {
    pub token: TokenService,
    pub user: UserService,
    pub auth: AuthService,
    pub batch: BatchOperationService,
    pub reporting: ReportingService,
    pub bootstrap: BootstrapService,
}
