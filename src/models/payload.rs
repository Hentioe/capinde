use crate::{captchas, provider::manifest::Manifest};
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
