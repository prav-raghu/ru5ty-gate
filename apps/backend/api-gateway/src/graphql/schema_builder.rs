use std::sync::Arc;

use async_graphql::{EmptySubscription, Schema};
use reqwest::Client;

use crate::graphql::resolvers::{MutationRoot, QueryRoot, UserApi};

const MAX_QUERY_DEPTH: usize = 10;
const MAX_QUERY_COMPLEXITY: usize = 200;

pub type GatewaySchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(customer_api_url: &str, introspection: bool) -> GatewaySchema {
    let api = Arc::new(UserApi::new(Client::new(), customer_api_url.to_owned()));
    let builder = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .limit_depth(MAX_QUERY_DEPTH)
        .limit_complexity(MAX_QUERY_COMPLEXITY)
        .data(api);
    if introspection {
        builder.finish()
    } else {
        builder.disable_introspection().finish()
    }
}
