use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::field_error::FieldError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    pub is_successful: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_time_stamp: Option<DateTime<Utc>>,
}

impl<T> ApiResponse<T> {
    #[must_use]
    pub fn stamped(mut self) -> Self {
        self.date_time_stamp = Some(Utc::now());
        self
    }

    pub fn success(data: T) -> Self {
        Self {
            is_successful: true,
            data: Some(data),
            message: None,
            errors: None,
            date_time_stamp: None,
        }
    }

    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            is_successful: true,
            data: Some(data),
            message: Some(message.into()),
            errors: None,
            date_time_stamp: None,
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            is_successful: false,
            data: None,
            message: Some(message.into()),
            errors: None,
            date_time_stamp: None,
        }
    }

    pub fn validation_failure(errors: Vec<FieldError>) -> Self {
        Self {
            is_successful: false,
            data: None,
            message: Some("Validation failed".to_owned()),
            errors: Some(errors),
            date_time_stamp: None,
        }
    }
}
