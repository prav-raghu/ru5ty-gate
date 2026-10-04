use async_graphql::ErrorExtensions;
use reqwest::{Client, Method};
use serde::Serialize;
use serde_json::Value;

use crate::graphql::schemas::{User, UserResponse, UsersResponse};

pub struct UserApi {
    client: Client,
    customer_api_url: String,
}

fn internal_error(message: impl Into<String>) -> async_graphql::Error {
    async_graphql::Error::new(message).extend_with(|_, extensions| {
        extensions.set("code", "INTERNAL_SERVER_ERROR");
    })
}

fn is_successful(body: &Value) -> bool {
    body.get("isSuccessful")
        .or_else(|| body.get("success"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn error_text(body: &Value) -> Option<String> {
    if is_successful(body) {
        None
    } else {
        body.get("message")
            .or_else(|| body.get("error"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| Some("Request failed".to_owned()))
    }
}

impl UserApi {
    pub fn new(client: Client, customer_api_url: String) -> Self {
        Self {
            client,
            customer_api_url,
        }
    }

    async fn request<B: Serialize + Sync>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        token: Option<&str>,
    ) -> Result<Value, async_graphql::Error> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.customer_api_url));
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        request
            .send()
            .await
            .map_err(|error| internal_error(error.to_string()))?
            .json::<Value>()
            .await
            .map_err(|error| internal_error(error.to_string()))
    }

    pub async fn user<B: Serialize + Sync>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        token: Option<&str>,
    ) -> Result<UserResponse, async_graphql::Error> {
        let response = self.request(method, path, body, token).await?;
        Ok(UserResponse {
            success: is_successful(&response),
            data: response.get("data").and_then(User::from_json),
            error: error_text(&response),
        })
    }

    pub async fn users(&self, token: Option<&str>) -> Result<UsersResponse, async_graphql::Error> {
        let response = self
            .request::<()>(Method::GET, "/api/v1/users", None, token)
            .await?;
        let data = response
            .get("data")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(User::from_json).collect());
        Ok(UsersResponse {
            success: is_successful(&response),
            data,
            error: error_text(&response),
        })
    }
}
