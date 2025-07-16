use super::{ALBUM_IMAGES, MANIFEST, manifest, manifest::Album};
use crate::{
    err,
    errors::Result,
    fail,
    provider::{
        CONFLICTS, Conflicts, get_manifest, manifest::Manifest, reset_album_images,
        reset_conflicts, reset_manifest,
    },
    vars::CAPINDE_ALBUMS_BASE,
};
use log::{debug, error, info, warn};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Once, RwLock},
};

static INIT: Once = Once::new();

pub fn init() {
    let albums_base = PathBuf::from(&*CAPINDE_ALBUMS_BASE);
    if CAPINDE_ALBUMS_BASE.is_empty() || !albums_base.exists() {
        warn!(
            "Variable `CAPINDE_ALBUMS_BASE` is not set or the directory does not exist, skipping initialization"
        );
    } else {
        INIT.call_once(|| match run(albums_base) {
            Ok(_) => {
                info!("Provider initialized successfully");
            }
            Err(e) => {
                error!("Failed to initialize provider: {e}");
            }
        });
    }
}

pub fn reinit(albums_base: PathBuf) -> Result<()> {
    if INIT.is_completed() {
        info!("Provider is already initialized, reinitializing...");
        let manifest = load_manifest(albums_base)?;
        let album_images = load_album_images(&manifest);

        reset_manifest(manifest.clone())?;
        reset_album_images(album_images);
        reset_conflicts(Conflicts::from(&manifest.conflicts.unwrap_or(vec![])));
    } else {
        info!("Provider is not initialized, initializing now...");

        init();
    }

    Ok(())
}

fn run(albums_base: PathBuf) -> Result<()> {
    // 初始化全局的清单配置
    let manifest = load_manifest(albums_base)?;
    MANIFEST
        .set(RwLock::new(manifest.clone()))
        .expect("Failed to set manifest");
    // 初始化全局的图集和图片列表映射
    let album_images = load_album_images(&manifest);
    ALBUM_IMAGES
        .set(RwLock::new(album_images))
        .expect("Failed to set album images");

    CONFLICTS
        .set(RwLock::new(Conflicts::from(
            &manifest.conflicts.unwrap_or(vec![]),
        )))
        .expect("Failed to set conflicts");

    Ok(())
}

fn load_manifest(albums_base: PathBuf) -> Result<Manifest> {
    // 从路径加载清单文件
    let file_path = albums_base.join("Manifest.yaml");

    if file_path.exists() {
        manifest::load(&file_path)
    } else {
        // 创建一个基本的 Manifest 实例，并序列化成 YAML 文件
        // todo: 扫描图集目录，如果发现图片则自动生成图集
        let manifest = Manifest {
            version: manifest::LATEST_VERSION.to_string(),
            datetime: chrono::Utc::now(),
            width: None,
            albums: vec![],
            include_formats: vec!["jpg".to_string(), "png".to_string()],
            conflicts: Some(vec![]),
        };
        manifest.save(&file_path)?;

        warn!(
            "Manifest file not found at {}, created a new one with default values",
            file_path.display()
        );

        Ok(manifest)
    }
}

fn load_album_images(manifest: &Manifest) -> HashMap<String, Vec<PathBuf>> {
    debug!("Loading album images...");
    // 扫描所有图集并存储图片路径
    let mut album_images = HashMap::new();
    for album in manifest.albums.iter() {
        match scan_images(album) {
            Ok(images) => {
                info!(
                    "Successfully loaded {} album: {} image(s)",
                    album.id,
                    images.len()
                );
                album_images.insert(album.id.clone(), images);
            }
            Err(e) => warn!("Failed to scan album `{}`: {}", album.id, e),
        }
    }

    album_images
}

fn scan_images(album: &Album) -> Result<Vec<PathBuf>> {
    let dir_path = PathBuf::from(&*CAPINDE_ALBUMS_BASE).join(&album.id);
    if dir_path.exists() && dir_path.is_dir() {
        let mut images = vec![];
        let entries = std::fs::read_dir(&dir_path)
            .map_err(|e| fail!("Failed to read album directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| fail!("failed to read entry: {}", e))?;
            let path = entry.path();

            if path.is_file() && includes_format(&path)? {
                images.push(path);
            } else if path.is_dir() {
                warn!("Skipping subdirectory in album: {path:?}");
            }
        }

        if images.is_empty() {
            err!("no images found in album: {}", dir_path.display())
        } else {
            Ok(images)
        }
    } else {
        err!(
            "the album path does not exist or is not a directory: {}",
            dir_path.display()
        )
    }
}

fn includes_format(path: &Path) -> Result<bool> {
    let is_includes = get_manifest()?
        .include_formats
        .iter()
        .any(|ext| path.extension().is_some_and(|e| e.to_str() == Some(ext)));

    Ok(is_includes)
}
