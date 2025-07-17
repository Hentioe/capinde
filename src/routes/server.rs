use crate::{
    errors::Result,
    models::payload::ServerInfo,
    vars::{CAPINDE_WORKING_MODE, STARTED_AT},
};
use axum::Json;

pub async fn info() -> Result<Json<ServerInfo>> {
    Ok(Json(ServerInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        started_at: STARTED_AT.get().cloned(),
        working_mode: &CAPINDE_WORKING_MODE,
        verification_queue_length: crate::verification::queue_size().await,
    }))
}
