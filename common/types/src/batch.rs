use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BatchOperationType {
    Create,
    Update,
    Delete,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationItem<T = Value> {
    pub id: String,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult<T = Value> {
    pub id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationSummary {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<BatchOperationResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOperationOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_on_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_before_execute: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_batch_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperation<T = Value> {
    #[serde(rename = "type")]
    pub operation_type: BatchOperationType,
    pub items: Vec<BatchOperationItem<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<BatchOperationOptions>,
}
