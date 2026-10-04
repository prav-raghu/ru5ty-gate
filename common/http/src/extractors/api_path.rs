use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use ru5ty_gate_types::FieldError;
use serde::de::DeserializeOwned;

use crate::app_error::AppError;

pub struct ApiPath<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Path::<T>::from_request_parts(parts, state)
            .await
            .map(|Path(value)| Self(value))
            .map_err(|_| {
                AppError::Validation(vec![FieldError {
                    field: "params".to_owned(),
                    message: "Invalid path parameter".to_owned(),
                }])
            })
    }
}
