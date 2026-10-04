mod auth_route;
mod batch_route;
mod health_route;
mod reporting_route;
mod user_route;
pub mod v1;
pub mod v2;

pub use auth_route::AuthRoutes;
pub use batch_route::BatchRoutes;
pub use health_route::HealthRoutes;
pub use reporting_route::ReportingRoutes;
pub use user_route::UserRoutes;
