use crate::{
    captchas::{classic, grid, idgen, image},
    errors::{Error, Result},
    janitor,
    models::{
        params::{Input, SpecialParams, verification::Answer},
        payload::Generated,
    },
    vars::{CAPINDE_NAMESPACE_BASE, CAPINDE_WORKING_MODE, MAX_TTL_SECS},
    verification,
};
use axum::Json;
use std::path::PathBuf;
use tokio::task::spawn_blocking;

pub async fn generate(input: Json<Input>) -> Result<Json<Generated>> {
    // 读取 TTL
    let ttl_secs = input.ttl_secs.unwrap_or(60 * 15); // 后备过期时间：15 分钟
    if ttl_secs > *MAX_TTL_SECS {
        return Err(Error::TTLTooLarge {
            max: *MAX_TTL_SECS,
            provided: ttl_secs,
        });
    }
    // 从命名空间生成输出目录
    let out_base = build_out_base(&input.namespace)?;
    let out_dir = out_base.clone();
    let choices_control = input.choices_control.clone().unwrap_or_default();

    let created = match &input.special_params {
        SpecialParams::Grid(params) => {
            let params = params.clone();
            spawn_blocking(move || grid::create(&out_base, &choices_control, &params)).await??
        }
        SpecialParams::Image(params) => {
            let params = params.clone();
            spawn_blocking(move || image::create(&out_base, &choices_control, &params)).await??
        }
        SpecialParams::Classic(params) => {
            let params = params.clone();
            spawn_blocking(move || classic::create(&out_base, &choices_control, &params)).await??
        }
    };

    let generated = Generated {
        working_mode: *CAPINDE_WORKING_MODE,
        namespace: input.namespace.clone(),
        file_name: created.file_name,
        unique_id: idgen(),
        right_index: created.right_index,
        special_payload: created.payload,
    };

    let answer = if input.use_index.unwrap_or(false) {
        Answer::Index {
            value: generated.right_index,
        }
    } else {
        Answer::try_from((&input.special_params, &generated.special_payload))?
    };

    // 添加到验证缓存
    verification::add_cache(generated.unique_id.clone(), answer, ttl_secs).await;
    // 添加到清理器
    janitor::collect(out_dir, &generated.file_name, ttl_secs).await;

    Ok(Json(generated))
}

fn build_out_base(namespace: &str) -> Result<String> {
    // 仅允许 namespace 包含字母、数字、下划线、短划线和斜杠
    if !namespace
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '/')
    {
        Err(Error::IllegalNamespace)
    } else if namespace.trim().is_empty() {
        Err(Error::MissingField("namespace".to_string()))
    } else if namespace.starts_with('/') {
        Err(Error::NamespaceStartsWithSlash)
    } else {
        let out_dir = PathBuf::from(*CAPINDE_NAMESPACE_BASE).join(namespace);
        Ok(out_dir
            .to_str()
            .ok_or(Error::InvalidPathEncoding)?
            .to_string())
    }
}
