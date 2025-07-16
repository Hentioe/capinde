use crate::{
    errors::{Error, Result},
    models::payload::{DeployedInfo, Success},
    provider::{
        self,
        archive::{self, ArchiveInfo},
        get_manifest,
    },
    vars::{CAPINDE_ALBUMS_BASE, CAPINDE_UPLOADED_DIR},
};
use axum::{Json, extract::Multipart};
use std::path::PathBuf;

const ARCHIVE_FIELD_NAME: &str = "archive";
const OUTPUT_FILE_NAME: &str = "albums.zip";

pub async fn deployed() -> Result<Json<DeployedInfo>> {
    let manifest = get_manifest()?.to_owned();
    let total_images = provider::total_images();

    Ok(Json(DeployedInfo {
        manifest,
        total_images,
    }))
}

pub async fn upload(mut multipart: Multipart) -> Result<Json<ArchiveInfo>> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name() == Some(ARCHIVE_FIELD_NAME) {
            let bytes = field.bytes().await?;
            // 检查上传目录是否存在，如果不存在则创建
            let uploaded_dir = PathBuf::from(*CAPINDE_UPLOADED_DIR);
            std::fs::create_dir_all(&uploaded_dir)?;
            // 将上传的文件保存到上传目录
            let archive_file = uploaded_dir.join(OUTPUT_FILE_NAME);
            std::fs::write(&archive_file, bytes)?;
            // 从压缩包中加载清单数据
            let info = archive::read_info(archive_file)?;

            return Ok(Json(info));
        }
    }

    Err(Error::MissingField(ARCHIVE_FIELD_NAME.to_string()))
}

pub async fn get_uploaded() -> Result<Json<ArchiveInfo>> {
    let archive_file = PathBuf::from(*CAPINDE_UPLOADED_DIR).join(OUTPUT_FILE_NAME);
    if archive_file.exists() {
        // 从压缩包中加载清单数据
        let info = archive::read_info(archive_file)?;
        Ok(Json(info))
    } else {
        Err(Error::NoUploadedArchive)
    }
}

pub async fn delete_uploaded() -> Result<Json<Success>> {
    let archive_file = PathBuf::from(*CAPINDE_UPLOADED_DIR).join(OUTPUT_FILE_NAME);
    if archive_file.exists() {
        // 删除上传的压缩包
        std::fs::remove_file(&archive_file)?;
        Ok(Json(Success::default()))
    } else {
        Err(Error::NoUploadedArchive)
    }
}

pub async fn deploy() -> Result<Json<Success>> {
    // 将压缩包部署到 albums 目录
    archive::deyloy(
        PathBuf::from(*CAPINDE_UPLOADED_DIR).join(OUTPUT_FILE_NAME),
        PathBuf::from(*CAPINDE_ALBUMS_BASE),
    )?;
    // 重新初始化提供商
    provider::reinit(PathBuf::from(*CAPINDE_ALBUMS_BASE))?;

    Ok(Json(Success::default()))
}

pub async fn reload() -> Result<Json<Success>> {
    provider::reinit(PathBuf::from(*CAPINDE_ALBUMS_BASE))?;

    Ok(Json(Success::default()))
}
