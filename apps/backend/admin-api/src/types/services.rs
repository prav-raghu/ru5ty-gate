use ru5ty_gate_auth::TokenService;

use crate::services::{
    AuthService, BatchOperationService, BootstrapService, CaptiveSessionService, GatewayService,
    ReportingService, UserService, VenueService,
};

pub struct Services {
    pub token: TokenService,
    pub user: UserService,
    pub auth: AuthService,
    pub batch: BatchOperationService,
    pub reporting: ReportingService,
    pub bootstrap: BootstrapService,
    pub venue: VenueService,
    pub gateway: GatewayService,
    pub captive_session: CaptiveSessionService,
}
