use chrono::Utc;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use serde::Serialize;
use uuid::Uuid;

use crate::job::{Job, JobMetadata};
use crate::job_options::{JobPriority, QueueJobOptions};
use crate::queue_error::QueueError;
use crate::queue_metrics::QueueMetrics;

const DEFAULT_ATTEMPTS: u32 = 3;
const PRIORITY_WEIGHT: f64 = 1e13;

#[derive(Clone)]
pub struct QueueService {
    connection: ConnectionManager,
    name: String,
}

impl QueueService {
    pub async fn connect(url: &str, name: &str) -> Result<Self, QueueError> {
        let client = redis::Client::open(url)?;
        let connection = ConnectionManager::new(client).await?;
        Ok(Self {
            connection,
            name: name.to_owned(),
        })
    }

    pub fn key(&self, suffix: &str) -> String {
        format!("queue:{}:{suffix}", self.name)
    }

    pub fn waiting_score(priority: JobPriority, enqueued_ms: i64) -> f64 {
        f64::from(priority.value()) * PRIORITY_WEIGHT + enqueued_ms as f64
    }

    pub async fn add<T: Serialize + Sync>(
        &self,
        job_name: &str,
        payload: &T,
        options: QueueJobOptions,
    ) -> Result<String, QueueError> {
        let priority = options.priority.unwrap_or(JobPriority::Normal);
        let now = Utc::now().timestamp_millis();
        let job = Job {
            id: options.job_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            name: job_name.to_owned(),
            payload: serde_json::to_value(payload)?,
            priority,
            attempts_made: 0,
            max_attempts: options.attempts.unwrap_or(DEFAULT_ATTEMPTS),
            backoff: options.backoff,
            metadata: JobMetadata {
                timestamp: Some(now),
                ..JobMetadata::default()
            },
        };
        let member = serde_json::to_string(&job)?;
        let mut connection = self.connection.clone();
        match options.delay_ms {
            Some(delay) if delay > 0 => {
                let run_at = now.saturating_add(i64::try_from(delay).unwrap_or(i64::MAX));
                let _: i64 = connection
                    .zadd(self.key("delayed"), member, run_at as f64)
                    .await?;
            }
            _ => {
                let _: i64 = connection
                    .zadd(
                        self.key("waiting"),
                        member,
                        Self::waiting_score(priority, now),
                    )
                    .await?;
            }
        }
        Ok(job.id)
    }

    pub async fn schedule_retry(&self, job: &Job, delay_ms: u64) -> Result<(), QueueError> {
        let run_at = Utc::now()
            .timestamp_millis()
            .saturating_add(i64::try_from(delay_ms).unwrap_or(i64::MAX));
        let mut connection = self.connection.clone();
        let _: i64 = connection
            .zadd(
                self.key("delayed"),
                serde_json::to_string(job)?,
                run_at as f64,
            )
            .await?;
        Ok(())
    }

    pub async fn promote_due(&self) -> Result<usize, QueueError> {
        let mut connection = self.connection.clone();
        let now = Utc::now().timestamp_millis();
        let due: Vec<String> = connection
            .zrangebyscore_limit(self.key("delayed"), f64::NEG_INFINITY, now as f64, 0, 100)
            .await?;
        let mut promoted = 0;
        for member in due {
            let removed: i64 = connection.zrem(self.key("delayed"), &member).await?;
            if removed == 1 {
                let job: Job = serde_json::from_str(&member)?;
                let _: i64 = connection
                    .zadd(
                        self.key("waiting"),
                        &member,
                        Self::waiting_score(job.priority, now),
                    )
                    .await?;
                promoted += 1;
            }
        }
        Ok(promoted)
    }

    pub async fn take_next(&self) -> Result<Option<Job>, QueueError> {
        let mut connection = self.connection.clone();
        let popped: Vec<(String, f64)> = connection.zpopmin(self.key("waiting"), 1).await?;
        match popped.into_iter().next() {
            Some((member, _)) => Ok(Some(serde_json::from_str(&member)?)),
            None => Ok(None),
        }
    }

    pub async fn record_completed(&self) -> Result<(), QueueError> {
        let mut connection = self.connection.clone();
        let _: i64 = connection.incr(self.key("completed"), 1).await?;
        Ok(())
    }

    pub async fn record_failed(&self) -> Result<(), QueueError> {
        let mut connection = self.connection.clone();
        let _: i64 = connection.incr(self.key("failed"), 1).await?;
        Ok(())
    }

    pub async fn metrics(&self) -> Result<QueueMetrics, QueueError> {
        let mut connection = self.connection.clone();
        let waiting: u64 = connection.zcard(self.key("waiting")).await?;
        let delayed: u64 = connection.zcard(self.key("delayed")).await?;
        let completed: Option<u64> = connection.get(self.key("completed")).await?;
        let failed: Option<u64> = connection.get(self.key("failed")).await?;
        Ok(QueueMetrics {
            waiting,
            delayed,
            completed: completed.unwrap_or(0),
            failed: failed.unwrap_or(0),
        })
    }

    pub async fn obliterate(&self) -> Result<(), QueueError> {
        let mut connection = self.connection.clone();
        let keys: Vec<String> = ["waiting", "delayed", "completed", "failed"]
            .iter()
            .map(|suffix| self.key(suffix))
            .collect();
        let _: i64 = connection.del(keys).await?;
        Ok(())
    }
}
