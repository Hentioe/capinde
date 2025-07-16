use crate::{errors::Result, janitor, models::payload::Success, scueduler::use_shceduler};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    ttl: TtL,
    fallback: Fallback,
}

#[derive(Debug, Clone, Serialize)]
pub struct TtL {
    queue_size: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Fallback {
    expired_secs: u64,
    cleaned_total: usize,
    next_run: Option<DateTime<Utc>>,
}

pub async fn schedule() -> Json<Success> {
    janitor::ttl_cleanup().await;
    janitor::fallback_cleanup().await;

    Json(Success::default())
}

pub async fn status() -> Result<Json<Status>> {
    let next_run = {
        let mut guard = use_shceduler().await;
        guard.fallback_next_run().await?
    };

    let fallback = {
        let guard = janitor::fallback().await;

        Fallback {
            expired_secs: guard.expiration.as_secs(),
            cleaned_total: guard.cleaned_total,
            next_run,
        }
    };

    Ok(Json(Status {
        ttl: TtL {
            queue_size: janitor::queue_size().await,
        },
        fallback,
    }))
}
