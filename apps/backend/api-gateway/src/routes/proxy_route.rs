use axum::Router;
use axum::routing::any;

use crate::config::ProxyTarget;
use crate::controllers::ProxyController;

pub struct ProxyRoutes;

impl ProxyRoutes {
    pub fn register(targets: Vec<ProxyTarget>) -> Router {
        targets.into_iter().fold(Router::new(), |router, target| {
            let prefix = target.prefix;
            router.merge(
                Router::new()
                    .route(prefix, any(ProxyController::forward))
                    .route(
                        &format!("{prefix}/{{*rest}}"),
                        any(ProxyController::forward),
                    )
                    .with_state(target),
            )
        })
    }
}
