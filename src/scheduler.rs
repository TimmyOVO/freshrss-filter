use anyhow::Result;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

use crate::config::SchedulerConfig;

pub struct Scheduler {
    sched: JobScheduler,
    cfg: SchedulerConfig,
}

impl Scheduler {
    pub async fn new(cfg: SchedulerConfig) -> Result<Self> {
        Ok(Self {
            sched: JobScheduler::new().await?,
            cfg,
        })
    }

    pub async fn add_job<F, Fut>(&mut self, f: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let cron = self.cfg.cron.clone();
        let running = Arc::new(AtomicBool::new(false));
        let job = Job::new_async(cron.as_str(), move |_uuid, _l| {
            let running = running.clone();

            if running
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                tracing::warn!("scheduler_job_skipped_previous_run_in_progress");
                return Box::pin(async {});
            }

            let fut = f();

            struct RunningGuard {
                flag: Arc<AtomicBool>,
            }

            impl Drop for RunningGuard {
                fn drop(&mut self) {
                    self.flag.store(false, Ordering::Release);
                }
            }

            let guard = RunningGuard { flag: running };

            Box::pin(async move {
                let _guard = guard;
                fut.await;
            })
        })?;
        self.sched.add(job).await?;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        self.sched.start().await?;
        info!("scheduler_started");
        Ok(())
    }
    pub async fn shutdown(&mut self) {
        let _ = self.sched.shutdown().await;
    }
}
