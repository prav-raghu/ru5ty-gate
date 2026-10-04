mod auth_route;
mod batch_route;
mod device_route;
mod health_route;
mod reporting_route;
mod user_route;
pub mod v1;
pub mod v2;
mod venue_route;

pub use auth_route::AuthRoutes;
pub use batch_route::BatchRoutes;
pub use device_route::DeviceRoutes;
pub use health_route::HealthRoutes;
pub use reporting_route::ReportingRoutes;
pub use user_route::UserRoutes;
pub use venue_route::VenueRoutes;
