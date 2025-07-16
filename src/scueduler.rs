use crate::{errors::Result, fail, janitor, verification};
use chrono::{DateTime, Utc};
use log::info;
use std::{
    pin::Pin,
    sync::{Arc, OnceLock},
};
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

pub async fn init() {
    tokio::spawn(async {
        init_scheduler()
            .await
            .expect("Failed to initialize scheduler")
    });
}

pub struct MyScheduler {
    sched: JobScheduler,
    fallback_job_id: Uuid,
}

type TaskRun = Arc<
    dyn Fn() -> Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync + 'static>>
        + Send
        + Sync
        + 'static,
>;

pub struct Task {
    run: TaskRun,
}

impl Task {
    pub fn new<F, Fut>(run: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + Sync + 'static,
    {
        Self {
            run: Arc::new(move || Box::pin(run())),
        }
    }

    pub fn create_job(&self, schedule: &str) -> Result<Job> {
        let run = Arc::clone(&self.run);
        let job = Job::new_async(schedule, move |_, _| run())?;

        Ok(job)
    }
}

impl MyScheduler {
    pub async fn fallback_next_run(&mut self) -> Result<Option<DateTime<Utc>>> {
        Ok(self.sched.next_tick_for_job(self.fallback_job_id).await?)
    }
}

static SCHEDULER: OnceLock<RwLock<MyScheduler>> = OnceLock::new();

const EVERY_15_SECONDS: &str = "*/15 * * * * *"; // 每 15 秒执行一次
const EVERY_1_HOUR: &str = "0 0 */1 * * *"; // 每小时执行一次

async fn init_scheduler() -> Result<()> {
    let sched = JobScheduler::new().await?;

    // 创建任务
    let ttl_janitor_cleanup = Task::new(|| async move { janitor::ttl_cleanup().await }); // 验证图片 TTL 过期清理任务
    let fallback_janitor_cleanup = Task::new(|| async move { janitor::fallback_cleanup().await }); // 验证图片备用清理任务
    let ttl_verification_cleanup =
        Task::new(|| async move { verification::cleanup_expired().await }); // 验证图片 TTL 验证清理任务

    // 从任务中创建定时作业
    let ttl_janitor_cleanup_job = ttl_janitor_cleanup.create_job(EVERY_15_SECONDS)?;
    let fallback_janitor_cleanup_job = fallback_janitor_cleanup.create_job(EVERY_1_HOUR)?;
    let ttl_verification_cleanup_job = ttl_verification_cleanup.create_job(EVERY_15_SECONDS)?;

    SCHEDULER
        .set(RwLock::new(MyScheduler {
            sched,
            fallback_job_id: ttl_janitor_cleanup_job.guid(),
        }))
        .map_err(|_| fail!("failed to set scheduler"))?;

    let mut schedule = use_shceduler().await;

    schedule.sched.add(ttl_janitor_cleanup_job).await?;
    info!("TTLJanitor scheduled to run every 15 seconds");

    schedule.sched.add(fallback_janitor_cleanup_job).await?;
    info!("FallbackJanitor scheduled to run every 1 hour");

    schedule.sched.add(ttl_verification_cleanup_job).await?;
    info!("Verification caches cleaner scheduled to run every 15 seconds");

    // Feature 'signal' must be enabled
    schedule.sched.shutdown_on_ctrl_c();

    // Add code to be run during/after shutdown
    schedule.sched.set_shutdown_handler(Box::new(|| {
        Box::pin(async move {
            info!("Job scheduler is shutting down");
        })
    }));

    // Start the scheduler
    schedule.sched.start().await?;
    Ok(())
}

pub async fn use_shceduler() -> RwLockWriteGuard<'static, MyScheduler> {
    SCHEDULER
        .get()
        .expect("Scheduler not initialized")
        .write()
        .await
}
