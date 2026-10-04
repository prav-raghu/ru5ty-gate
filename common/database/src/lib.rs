mod database_config;
mod database_error;
mod inputs;
mod models;
mod pool;
mod records;
mod repositories;
mod seed;

pub use database_config::DatabaseConfig;
pub use database_error::DatabaseError;
pub use inputs::{
    HeartbeatRecord, NewCaptiveSession, NewGateway, NewVenue, NewWebhookSubscription, PageRequest,
    SessionListFilter, UserListFilter, VenueChanges, WebhookSubscriptionChanges,
};
pub use models::{
    CaptiveSession, Gateway, Role, User, UserStatus, Venue, WebhookDelivery, WebhookSubscription,
};
pub use pool::{connect, run_migrations};
pub use records::{AuthorizedUserRecord, ExportUserRecord, UserSummaryRecord};
pub use repositories::{
    CaptiveSessionRepository, Entity, GatewayRepository, Repository, SoftDeletable, UserRepository,
    VenueRepository, Writable,
};
pub use seed::{
    SeedAdmin, USER_STATUS_NAMES, seed_admin, seed_all, seed_roles, seed_user_statuses,
};
pub use sqlx::{self, PgPool};
