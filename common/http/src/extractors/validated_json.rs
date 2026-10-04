use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::app_error::AppError;
use crate::validation::{field_errors_from_serde, field_errors_from_validator};

pub struct ValidatedJson<T>(pub T);

pub fn parse_validated<T>(bytes: &[u8]) -> Result<T, AppError>
where
    T: DeserializeOwned + Validate,
{
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value: T = serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        if error.inner().is_syntax() || error.inner().is_eof() {
            AppError::BadRequest("Invalid request body".to_owned())
        } else {
            AppError::Validation(field_errors_from_serde(&error))
        }
    })?;
    value
        .validate()
        .map_err(|errors| AppError::Validation(field_errors_from_validator(&errors)))?;
    Ok(value)
}

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(request, state)
            .await
            .map_err(|_| AppError::BadRequest("Invalid request body".to_owned()))?;
        let value = parse_validated::<T>(&bytes)?;
        Ok(Self(value))
    }
}
