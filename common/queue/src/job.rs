use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::job_options::{BackoffOptions, JobPriority};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobMetadata {
    pub correlation_id: Option<String>,
    pub user_id: Option<String>,
    pub source: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub name: String,
    pub payload: Value,
    pub priority: JobPriority,
    pub attempts_made: u32,
    pub max_attempts: u32,
    pub backoff: Option<BackoffOptions>,
    pub metadata: JobMetadata,
}
