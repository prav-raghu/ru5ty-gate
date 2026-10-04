use axum::Router;

use crate::config::ServiceConfig;
use crate::graphql::build_schema;
use crate::routes::GraphqlRoutes;

pub fn graphql_router(config: &ServiceConfig) -> Option<Router> {
    if !config.graphql.enabled {
        tracing::info!("GraphQL is disabled");
        return None;
    }
    let schema = build_schema(&config.customer_api_url, config.graphql.introspection);
    tracing::info!(path = %config.graphql.path, "GraphQL endpoint available");
    Some(GraphqlRoutes::register(&config.graphql, schema))
}
