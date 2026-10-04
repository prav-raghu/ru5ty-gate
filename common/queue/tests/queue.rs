#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;
use ru5ty_gate_queue::{
    BackoffKind, BackoffOptions, Job, JobHandler, JobPriority, QueueJobOptions, QueueService,
    WorkerOptions, WorkerService,
};
use serde_json::{Value, json};

struct FlakyHandler {
    calls: AtomicU32,
    fail_first: u32,
}

#[async_trait]
impl JobHandler for FlakyHandler {
    async fn handle(&self, _job: &Job) -> Result<Value, String> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call < self.fail_first {
            Err("boom".to_owned())
        } else {
            Ok(json!({"ok": true}))
        }
    }
}

#[test]
fn critical_jobs_sort_before_low_jobs() {
    assert!(
        QueueService::waiting_score(JobPriority::Critical, 2_000)
            < QueueService::waiting_score(JobPriority::Low, 1_000)
    );
}

#[test]
fn exponential_backoff_doubles_per_attempt() {
    let backoff = BackoffOptions {
        kind: BackoffKind::Exponential,
        delay_ms: 100,
    };

    assert_eq!(backoff.delay_for_attempt(1), 100);
    assert_eq!(backoff.delay_for_attempt(2), 200);
    assert_eq!(backoff.delay_for_attempt(3), 400);
}

#[test]
fn fixed_backoff_is_constant() {
    let backoff = BackoffOptions {
        kind: BackoffKind::Fixed,
        delay_ms: 250,
    };

    assert_eq!(backoff.delay_for_attempt(5), 250);
}

#[tokio::test]
async fn processes_and_retries_jobs_against_a_live_server_when_configured() {
    let Ok(url) = std::env::var("TEST_REDIS_URL") else {
        return;
    };
    let name = format!("test-{}", std::process::id());
    let queue = QueueService::connect(&url, &name).await.unwrap();
    queue.obliterate().await.unwrap();
    let handler = Arc::new(FlakyHandler {
        calls: AtomicU32::new(0),
        fail_first: 1,
    });
    let mut handlers: HashMap<String, Arc<dyn JobHandler>> = HashMap::new();
    handlers.insert("work".to_owned(), handler.clone());
    let worker = WorkerService::new(queue.clone(), handlers, WorkerOptions::default());

    queue
        .add(
            "work",
            &json!({"n": 1}),
            QueueJobOptions {
                attempts: Some(3),
                backoff: Some(BackoffOptions {
                    kind: BackoffKind::Fixed,
                    delay_ms: 1,
                }),
                ..QueueJobOptions::default()
            },
        )
        .await
        .unwrap();

    assert!(worker.run_once().await.unwrap());
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(worker.run_once().await.unwrap());

    let metrics = queue.metrics().await.unwrap();
    assert_eq!(handler.calls.load(Ordering::SeqCst), 2);
    assert_eq!(metrics.completed, 1);
    assert_eq!(metrics.failed, 0);
    queue.obliterate().await.unwrap();
}

#[tokio::test]
async fn exhausted_jobs_are_counted_as_failed_when_configured() {
    let Ok(url) = std::env::var("TEST_REDIS_URL") else {
        return;
    };
    let name = format!("test-fail-{}", std::process::id());
    let queue = QueueService::connect(&url, &name).await.unwrap();
    queue.obliterate().await.unwrap();
    let mut handlers: HashMap<String, Arc<dyn JobHandler>> = HashMap::new();
    handlers.insert(
        "work".to_owned(),
        Arc::new(FlakyHandler {
            calls: AtomicU32::new(0),
            fail_first: 99,
        }),
    );
    let worker = WorkerService::new(queue.clone(), handlers, WorkerOptions::default());

    queue
        .add(
            "work",
            &json!({}),
            QueueJobOptions {
                attempts: Some(1),
                ..QueueJobOptions::default()
            },
        )
        .await
        .unwrap();
    worker.run_once().await.unwrap();

    assert_eq!(queue.metrics().await.unwrap().failed, 1);
    queue.obliterate().await.unwrap();
}
