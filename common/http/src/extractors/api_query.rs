use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use ru5ty_gate_types::FieldError;
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::app_error::AppError;
use crate::validation::field_errors_from_validator;

pub struct ApiQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) =
            Query::<T>::from_request_parts(parts, state)
                .await
                .map_err(|rejection| {
                    AppError::Validation(vec![FieldError {
                        field: "query".to_owned(),
                        message: rejection.body_text(),
                    }])
                })?;
        value
            .validate()
            .map_err(|errors| AppError::Validation(field_errors_from_validator(&errors)))?;
        Ok(Self(value))
    }
}
