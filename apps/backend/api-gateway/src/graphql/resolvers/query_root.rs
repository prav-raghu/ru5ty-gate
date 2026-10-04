use std::sync::Arc;

use async_graphql::{Context, ID, Object};
use reqwest::Method;

use crate::graphql::BearerToken;
use crate::graphql::resolvers::UserApi;
use crate::graphql::schemas::{UserResponse, UsersResponse};

pub struct QueryRoot;

fn token(context: &Context<'_>) -> Option<String> {
    context
        .data_opt::<BearerToken>()
        .and_then(|bearer| bearer.0.clone())
}

#[Object]
impl QueryRoot {
    async fn empty(&self) -> Option<String> {
        None
    }

    async fn get_user(&self, context: &Context<'_>, id: ID) -> async_graphql::Result<UserResponse> {
        let api = context.data::<Arc<UserApi>>()?;
        api.user::<()>(
            Method::GET,
            &format!("/api/v1/users/{}", id.as_str()),
            None,
            token(context).as_deref(),
        )
        .await
    }

    async fn get_users(&self, context: &Context<'_>) -> async_graphql::Result<UsersResponse> {
        context
            .data::<Arc<UserApi>>()?
            .users(token(context).as_deref())
            .await
    }

    async fn get_current_user(&self, context: &Context<'_>) -> async_graphql::Result<UserResponse> {
        let api = context.data::<Arc<UserApi>>()?;
        api.user::<()>(
            Method::GET,
            "/api/v1/users/me",
            None,
            token(context).as_deref(),
        )
        .await
    }
}
