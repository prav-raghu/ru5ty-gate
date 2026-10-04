use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use thiserror::Error;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};

use crate::jobs::ScheduledJob;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchedulerError {
    #[error("Job not found: {0}")]
    JobNotFound(String),
    #[error("Job failed: {0}")]
    JobFailed(String),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobStatus {
    pub name: String,
    pub running: bool,
    pub interval_seconds: u64,
}

struct RunningJob {
    stop: watch::Sender<bool>,
    task: JoinHandle<()>,
}

pub struct CronSchedulerService {
    jobs: BTreeMap<String, Arc<dyn ScheduledJob>>,
    running: Mutex<BTreeMap<String, RunningJob>>,
}

impl CronSchedulerService {
    pub fn new(jobs: Vec<Arc<dyn ScheduledJob>>) -> Self {
        Self {
            jobs: jobs
                .into_iter()
                .map(|job| (job.name().to_owned(), job))
                .collect(),
            running: Mutex::new(BTreeMap::new()),
        }
    }

    fn job(&self, name: &str) -> Result<Arc<dyn ScheduledJob>, SchedulerError> {
        self.jobs
            .get(name)
            .cloned()
            .ok_or_else(|| SchedulerError::JobNotFound(name.to_owned()))
    }

    pub fn start_all(&self) {
        tracing::info!("Starting {} cron jobs", self.jobs.len());
        for name in self.jobs.keys() {
            if let Err(error) = self.start_job(name) {
                tracing::error!(%error, job = name, "Failed to start job");
            }
        }
    }

    pub fn start_job(&self, name: &str) -> Result<(), SchedulerError> {
        let job = self.job(name)?;
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        if running.contains_key(name) {
            return Ok(());
        }
        let (stop, mut stopped) = watch::channel(false);
        let task = tokio::spawn(async move {
            let mut ticker = interval(job.interval());
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(error) = job.run().await {
                            tracing::error!(%error, job = job.name(), "Error running scheduled job");
                        }
                    }
                    _ = stopped.changed() => break,
                }
            }
        });
        running.insert(name.to_owned(), RunningJob { stop, task });
        tracing::info!(job = name, "Started job");
        Ok(())
    }

    pub async fn stop_job(&self, name: &str) -> Result<(), SchedulerError> {
        self.job(name)?;
        let handle = self
            .running
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(name);
        if let Some(handle) = handle {
            let _ = handle.stop.send(true);
            let _ = handle.task.await;
            tracing::info!(job = name, "Stopped job");
        }
        Ok(())
    }

    pub async fn stop_all(&self) {
        let names: Vec<String> = self.jobs.keys().cloned().collect();
        tracing::info!("Stopping {} cron jobs", names.len());
        for name in names {
            let _ = self.stop_job(&name).await;
        }
    }

    pub async fn run_now(&self, name: &str) -> Result<(), SchedulerError> {
        self.job(name)?
            .run()
            .await
            .map_err(SchedulerError::JobFailed)
    }

    pub fn is_running(&self, name: &str) -> bool {
        self.running
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(name)
    }

    pub fn statuses(&self) -> Vec<JobStatus> {
        self.jobs
            .values()
            .map(|job| JobStatus {
                name: job.name().to_owned(),
                running: self.is_running(job.name()),
                interval_seconds: job.interval().as_secs(),
            })
            .collect()
    }
}
