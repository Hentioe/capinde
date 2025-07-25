use crate::{captchas, provider::manifest::Manifest};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Success {
    pub ok: bool,
}

impl Default for Success {
    fn default() -> Self {
        Self { ok: true }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    // 版本号
    pub version: String,
    // 启动于
    pub started_at: Option<DateTime<Utc>>,
    // 工作模式
    pub working_mode: &'static str,
    // 验证队列长度
    pub verification_queue_length: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Generated {
    pub working_mode: &'static str,
    pub namespace: String,
    pub unique_id: String,
    pub file_name: String,
    pub right_index: usize,
    pub special_payload: SpecialPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SpecialPayload {
    #[serde(rename = "classic")]
    Classic(captchas::classic::Payload),
    #[serde(rename = "grid")]
    Grid(captchas::grid::Payload),
    #[serde(rename = "image")]
    Image(captchas::image::Payload),
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployedInfo {
    pub manifest: Manifest,
    pub total_images: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct VefifyResult {
    pub ok: bool,
}
