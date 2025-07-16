use super::manifest::Manifest;
use crate::{err, errors::Result, fail};
use serde::Serialize;
use std::{
    fs::{self, File},
    io::{self, Read},
    path::PathBuf,
    str::FromStr,
};
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveInfo {
    // 图片总数
    pub total_images: usize,
    // 已解析的清单
    pub manifest: Manifest,
}

pub fn read_info(archive_file: PathBuf) -> Result<ArchiveInfo> {
    let mut archive: ZipArchive<File> = ZipArchive::new(File::open(&archive_file)?)?;
    let manifest = {
        let mut file = match archive.by_name("Manifest.yaml") {
            Ok(file) => file,
            Err(..) => {
                return err!("the Manifest.yaml not found in the archive");
            }
        };

        let mut content = String::new();
        let _ = file.read_to_string(&mut content)?;
        Manifest::from_str(&content)?
    };
    let album_ids = manifest
        .albums
        .iter()
        .map(|album| album.id.as_str())
        .collect::<Vec<_>>();
    let mut total_images = 0;
    for file in archive.file_names() {
        let file_path = PathBuf::from(file);
        let file_parent = file_path.parent().and_then(|p| p.to_str());
        let file_ext = file_path.extension().and_then(|s| s.to_str());
        if let Some(parent) = file_parent
            && let Some(extension) = file_ext
            && album_ids.contains(&parent)
            && manifest.include_formats.contains(&extension.to_string())
        {
            total_images += 1;
        }
    }

    Ok(ArchiveInfo {
        total_images,
        manifest,
    })
}

pub fn deyloy(archive_file: PathBuf, target_dir: PathBuf) -> Result<()> {
    let zipfile = fs::File::open(&archive_file)?;
    let mut archive = ZipArchive::new(zipfile)?;

    // 将 target_dir 目录清空
    if target_dir.exists() {
        clear_directory(&target_dir).map_err(|e| fail!("failed to clear target directory: {e}"))?;
    }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        // 按照官方示例建议，此处应该用 `enclosed_name` 方法以避免攻击。但 `enclosed_name` 方法可能存在乱码问题，且上传压缩文件暂时无需担心攻击。
        let out = String::from_utf8(file.name_raw().to_vec())?;
        let output_path = PathBuf::from(&target_dir).join(out);

        if (*file.name()).ends_with('/') {
            // 如果是目录，则创建目录
            fs::create_dir_all(&output_path)?;
        } else {
            // 如果是文件，在写入前检查父级目录是否存在
            if let Some(p) = output_path.parent() {
                if !p.exists() {
                    // 不存在则创建父级目录
                    fs::create_dir_all(p)?;
                }
            }
            // 写入文件
            let mut outfile = fs::File::create(&output_path)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

fn clear_directory(target_dir: &PathBuf) -> Result<()> {
    let dir = fs::read_dir(target_dir)?;

    for entry in dir {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn test_load_manifest() {
        let info = read_info(PathBuf::from("tests/fixtures/albums.zip")).unwrap();

        assert_eq!(
            info.manifest.datetime,
            DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap()
        );
        assert_eq!(info.manifest.version, "0.1.2");
        assert_eq!(info.manifest.albums.len(), 10);
        assert_eq!(info.total_images, 0);
    }
}
