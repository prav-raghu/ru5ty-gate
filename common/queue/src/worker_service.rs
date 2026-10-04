use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use crate::job::Job;
use crate::job_handler::JobHandler;
use crate::queue_error::QueueError;
use crate::queue_service::QueueService;

#[derive(Debug, Clone, Copy)]
pub struct WorkerOptions {
    pub concurrency: usize,
    pub idle_poll: Duration,
}

impl Default for WorkerOptions {
    fn default() -> Self {
        Self {
            concurrency: 1,
            idle_poll: Duration::from_millis(250),
        }
    }
}

#[derive(Clone)]
pub struct WorkerService {
    queue: QueueService,
    handlers: Arc<HashMap<String, Arc<dyn JobHandler>>>,
    options: WorkerOptions,
}

impl WorkerService {
    pub fn new(
        queue: QueueService,
        handlers: HashMap<String, Arc<dyn JobHandler>>,
        options: WorkerOptions,
    ) -> Self {
        Self {
            queue,
            handlers: Arc::new(handlers),
            options,
        }
    }

    pub async fn run_once(&self) -> Result<bool, QueueError> {
        self.queue.promote_due().await?;
        let Some(job) = self.queue.take_next().await? else {
            return Ok(false);
        };
        self.process(job).await?;
        Ok(true)
    }

    async fn process(&self, mut job: Job) -> Result<(), QueueError> {
        job.attempts_made += 1;
        let outcome = match self.handlers.get(&job.name) {
            Some(handler) => handler.handle(&job).await,
            None => Err(format!("no handler registered for job '{}'", job.name)),
        };
        match outcome {
            Ok(_) => self.queue.record_completed().await,
            Err(reason) if job.attempts_made < job.max_attempts => {
                tracing::warn!(job = %job.id, %reason, "job failed, scheduling retry");
                let delay = job
                    .backoff
                    .map_or(0, |backoff| backoff.delay_for_attempt(job.attempts_made));
                self.queue.schedule_retry(&job, delay).await
            }
            Err(reason) => {
                tracing::error!(job = %job.id, %reason, "job failed permanently");
                self.queue.record_failed().await
            }
        }
    }

    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        let mut tasks = Vec::new();
        for _ in 0..self.options.concurrency.max(1) {
            let worker = self.clone();
            let mut stop = shutdown.clone();
            tasks.push(tokio::spawn(async move {
                while !*stop.borrow() {
                    match worker.run_once().await {
                        Ok(true) => continue,
                        Ok(false) => {}
                        Err(error) => tracing::error!(%error, "queue worker error"),
                    }
                    tokio::select! {
                        () = tokio::time::sleep(worker.options.idle_poll) => {}
                        _ = stop.changed() => {}
                    }
                }
            }));
        }
        let _ = shutdown.changed().await;
        for task in tasks {
            let _ = task.await;
        }
    }
}
