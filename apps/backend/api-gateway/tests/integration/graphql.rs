#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::Router;
use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::any;
use serde_json::{Value, json};
use tokio::net::TcpListener;

use crate::common::{app_with, call, config};

async fn fake_customer_api() -> String {
    async fn handle(request: Request) -> impl IntoResponse {
        let auth = request
            .headers()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let path = request.uri().path().to_owned();
        match (request.method().as_str(), path.as_str()) {
            ("GET", "/api/v1/users") => axum::Json(json!({
                "isSuccessful": auth.as_deref() == Some("Bearer good"),
                "message": "Unauthorized",
                "data": [
                    { "id": "u-1", "username": "Pat", "age": 30, "lastSeen": "2026-01-01T00:00:00Z" }
                ]
            }))
            .into_response(),
            ("GET", "/api/v1/users/u-1") => axum::Json(json!({
                "isSuccessful": true,
                "data": { "id": "u-1", "username": "Pat", "age": 30, "lastSeen": "2026-01-01T00:00:00Z" }
            }))
            .into_response(),
            _ => (StatusCode::NOT_FOUND, axum::Json(json!({ "isSuccessful": false, "message": "Not found" })))
                .into_response(),
        }
    }
    let app = Router::new().fallback(any(handle));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{address}")
}

async fn query(app: &Router, token: Option<&str>, query: &str) -> Value {
    let mut headers = Vec::new();
    let bearer = token.map(|value| format!("Bearer {value}"));
    if let Some(bearer) = &bearer {
        headers.push(("authorization", bearer.as_str()));
    }
    call(
        app,
        Method::POST,
        "/graphql",
        &headers,
        Some(json!({ "query": query })),
    )
    .await
    .2
}

#[tokio::test]
async fn graphql_is_not_mounted_unless_enabled() {
    let upstream = fake_customer_api().await;
    let app = app_with(config(&upstream, &upstream, &upstream, &[]));

    let (status, _, _) = call(
        &app,
        Method::POST,
        "/graphql",
        &[],
        Some(json!({ "query": "{ __typename }" })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn queries_forward_the_bearer_token_and_map_responses() {
    let upstream = fake_customer_api().await;
    let app = app_with(config(
        &upstream,
        &upstream,
        &upstream,
        &[("GRAPHQL_ENABLED", "true")],
    ));

    let authorised = query(
        &app,
        Some("good"),
        "{ getUsers { success data { id firstName role isActive } error } }",
    )
    .await;
    let unauthorised = query(&app, None, "{ getUsers { success error } }").await;
    let single = query(
        &app,
        Some("good"),
        "{ getUser(id: \"u-1\") { success data { id firstName createdAt } } }",
    )
    .await;
    let missing = query(&app, Some("good"), "{ getCurrentUser { success error } }").await;

    assert_eq!(authorised["data"]["getUsers"]["success"], true);
    assert_eq!(authorised["data"]["getUsers"]["data"][0]["id"], "u-1");
    assert_eq!(
        authorised["data"]["getUsers"]["data"][0]["firstName"],
        "Pat"
    );
    assert_eq!(
        authorised["data"]["getUsers"]["data"][0]["role"],
        "Chat User"
    );
    assert_eq!(unauthorised["data"]["getUsers"]["success"], false);
    assert_eq!(unauthorised["data"]["getUsers"]["error"], "Unauthorized");
    assert_eq!(
        single["data"]["getUser"]["data"]["createdAt"],
        "2026-01-01T00:00:00Z"
    );
    assert_eq!(missing["data"]["getCurrentUser"]["success"], false);
}

#[tokio::test]
async fn mutations_surface_upstream_failures_and_invalid_queries_are_rejected() {
    let upstream = fake_customer_api().await;
    let app = app_with(config(
        &upstream,
        &upstream,
        &upstream,
        &[("GRAPHQL_ENABLED", "true")],
    ));

    let created = query(
        &app,
        Some("good"),
        "mutation { createUser(input: { email: \"a@b.com\", password: \"pw\" }) { success error } }",
    )
    .await;
    let invalid = query(&app, Some("good"), "{ nope }").await;
    let introspection = query(&app, Some("good"), "{ __schema { queryType { name } } }").await;

    assert_eq!(created["data"]["createUser"]["success"], false);
    assert_eq!(created["data"]["createUser"]["error"], "Not found");
    assert!(invalid["errors"].is_array());
    assert!(
        introspection["data"]["__schema"].is_null(),
        "introspection is off by default"
    );
}

#[tokio::test]
async fn introspection_and_the_playground_are_opt_in_and_never_in_production() {
    let upstream = fake_customer_api().await;
    let enabled = app_with(config(
        &upstream,
        &upstream,
        &upstream,
        &[
            ("GRAPHQL_ENABLED", "true"),
            ("GRAPHQL_INTROSPECTION", "true"),
            ("GRAPHQL_PLAYGROUND", "true"),
        ],
    ));
    let production = app_with(config(
        &upstream,
        &upstream,
        &upstream,
        &[
            ("GRAPHQL_ENABLED", "true"),
            ("GRAPHQL_INTROSPECTION", "true"),
            ("GRAPHQL_PLAYGROUND", "true"),
            ("APP_ENV", "production"),
        ],
    ));

    let introspected = query(&enabled, None, "{ __schema { queryType { name } } }").await;
    let (playground, _, _) = call(&enabled, Method::GET, "/graphql", &[], None).await;
    let blocked = query(&production, None, "{ __schema { queryType { name } } }").await;
    let (prod_playground, _, _) = call(&production, Method::GET, "/graphql", &[], None).await;

    assert_eq!(
        introspected["data"]["__schema"]["queryType"]["name"],
        "QueryRoot"
    );
    assert_eq!(playground, StatusCode::OK);
    assert!(blocked["data"]["__schema"].is_null());
    assert_ne!(prod_playground, StatusCode::OK);
}
