use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::types::AppState;

async fn health() -> Json<Value> {
    Json(json!({ "status": "healthy", "version": "v2" }))
}

pub struct V2Routes;

impl V2Routes {
    pub fn register() -> Router<AppState> {
        Router::new().route("/health", get(health))
    }
}
