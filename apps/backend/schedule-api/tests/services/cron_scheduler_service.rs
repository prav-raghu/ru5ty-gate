#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use schedule_api::jobs::ScheduledJob;
use schedule_api::services::{CronSchedulerService, SchedulerError};

struct CountingJob {
    runs: Arc<AtomicU32>,
    fail: bool,
}

#[async_trait]
impl ScheduledJob for CountingJob {
    fn name(&self) -> &'static str {
        "counter"
    }

    fn interval(&self) -> Duration {
        Duration::from_millis(20)
    }

    async fn run(&self) -> Result<(), String> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err("boom".to_owned())
        } else {
            Ok(())
        }
    }
}

fn scheduler(fail: bool) -> (CronSchedulerService, Arc<AtomicU32>) {
    let runs = Arc::new(AtomicU32::new(0));
    let job: Arc<dyn ScheduledJob> = Arc::new(CountingJob {
        runs: runs.clone(),
        fail,
    });
    (CronSchedulerService::new(vec![job]), runs)
}

#[tokio::test]
async fn started_jobs_run_immediately_and_repeatedly_until_stopped() {
    let (scheduler, runs) = scheduler(false);

    scheduler.start_all();
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert!(scheduler.is_running("counter"));
    scheduler.stop_all().await;
    let after_stop = runs.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(80)).await;

    assert!(after_stop >= 3, "ran {after_stop} times");
    assert_eq!(runs.load(Ordering::SeqCst), after_stop);
    assert!(!scheduler.is_running("counter"));
}

#[tokio::test]
async fn starting_twice_does_not_duplicate_the_loop_and_stopping_is_idempotent() {
    let (scheduler, runs) = scheduler(false);

    scheduler.start_job("counter").unwrap();
    scheduler.start_job("counter").unwrap();
    tokio::time::sleep(Duration::from_millis(70)).await;
    scheduler.stop_job("counter").await.unwrap();
    scheduler.stop_job("counter").await.unwrap();

    assert!(runs.load(Ordering::SeqCst) <= 6);
    assert!(!scheduler.is_running("counter"));
}

#[tokio::test]
async fn failing_jobs_keep_running_and_run_now_surfaces_the_error() {
    let (scheduler, runs) = scheduler(true);

    scheduler.start_all();
    tokio::time::sleep(Duration::from_millis(100)).await;
    scheduler.stop_all().await;
    let outcome = scheduler.run_now("counter").await;

    assert!(runs.load(Ordering::SeqCst) >= 2);
    assert_eq!(outcome, Err(SchedulerError::JobFailed("boom".to_owned())));
}

#[tokio::test]
async fn unknown_jobs_are_reported() {
    let (scheduler, _) = scheduler(false);

    assert_eq!(
        scheduler.start_job("missing"),
        Err(SchedulerError::JobNotFound("missing".to_owned()))
    );
    assert_eq!(
        scheduler.stop_job("missing").await,
        Err(SchedulerError::JobNotFound("missing".to_owned()))
    );
    assert_eq!(
        scheduler.run_now("missing").await,
        Err(SchedulerError::JobNotFound("missing".to_owned()))
    );
}

#[tokio::test]
async fn statuses_describe_each_job() {
    let (scheduler, _) = scheduler(false);

    let before = scheduler.statuses();
    scheduler.start_all();
    let during = scheduler.statuses();
    scheduler.stop_all().await;

    assert_eq!(before[0].name, "counter");
    assert!(!before[0].running);
    assert!(during[0].running);
}
