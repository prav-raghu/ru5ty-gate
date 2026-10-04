use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::Router;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use axum::response::Html;
use axum::routing::{get, post};

use crate::config::GraphqlConfig;
use crate::graphql::{BearerToken, GatewaySchema};

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_owned)
}

async fn execute(
    State(schema): State<GatewaySchema>,
    headers: HeaderMap,
    request: GraphQLRequest,
) -> GraphQLResponse {
    let request = request
        .into_inner()
        .data(BearerToken(bearer_token(&headers)));
    schema.execute(request).await.into()
}

async fn playground(State(endpoint): State<String>) -> Html<String> {
    Html(GraphiQLSource::build().endpoint(&endpoint).finish())
}

pub struct GraphqlRoutes;

impl GraphqlRoutes {
    pub fn register(config: &GraphqlConfig, schema: GatewaySchema) -> Router {
        let router = Router::new()
            .route(&config.path, post(execute))
            .with_state(schema);
        if config.playground {
            router.merge(
                Router::new()
                    .route(&config.path, get(playground))
                    .with_state(config.path.clone()),
            )
        } else {
            router
        }
    }
}
