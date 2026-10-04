use std::sync::Arc;

use async_graphql::{Context, ID, Object};
use reqwest::Method;

use crate::graphql::BearerToken;
use crate::graphql::resolvers::UserApi;
use crate::graphql::schemas::{CreateUserInput, UpdateUserInput, UserResponse};

pub struct MutationRoot;

fn token(context: &Context<'_>) -> Option<String> {
    context
        .data_opt::<BearerToken>()
        .and_then(|bearer| bearer.0.clone())
}

#[Object]
impl MutationRoot {
    async fn empty(&self) -> Option<String> {
        None
    }

    async fn create_user(
        &self,
        context: &Context<'_>,
        input: CreateUserInput,
    ) -> async_graphql::Result<UserResponse> {
        let api = context.data::<Arc<UserApi>>()?;
        api.user(
            Method::POST,
            "/api/v1/users",
            Some(&input),
            token(context).as_deref(),
        )
        .await
    }

    async fn update_user(
        &self,
        context: &Context<'_>,
        id: ID,
        input: UpdateUserInput,
    ) -> async_graphql::Result<UserResponse> {
        let api = context.data::<Arc<UserApi>>()?;
        api.user(
            Method::PUT,
            &format!("/api/v1/users/{}", id.as_str()),
            Some(&input),
            token(context).as_deref(),
        )
        .await
    }

    async fn delete_user(
        &self,
        context: &Context<'_>,
        id: ID,
    ) -> async_graphql::Result<UserResponse> {
        let api = context.data::<Arc<UserApi>>()?;
        api.user::<()>(
            Method::DELETE,
            &format!("/api/v1/users/{}", id.as_str()),
            None,
            token(context).as_deref(),
        )
        .await
    }
}
