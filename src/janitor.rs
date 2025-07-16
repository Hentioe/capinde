use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, SystemTime},
};

use log::{debug, error, info};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    errors::Result,
    vars::{CAPINDE_NAMESPACE_BASE, MAX_TTL_SECS},
};

static TTL_JANITOR: OnceLock<TTLJanitor> = OnceLock::new();
static FALLBACK_JANITOR: OnceLock<RwLock<FallbackJanitor>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpiringImage {
    pub path: PathBuf,
    pub expires_at: SystemTime,
}

impl Ord for ExpiringImage {
    fn cmp(&self, other: &Self) -> Ordering {
        other.expires_at.cmp(&self.expires_at) // 将更早过期的图片放在前面
    }
}

impl PartialOrd for ExpiringImage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct TTLJanitor {
    expiration_queue: Mutex<BinaryHeap<ExpiringImage>>,
}

impl TTLJanitor {
    fn new() -> Self {
        TTLJanitor {
            expiration_queue: Mutex::new(BinaryHeap::new()),
        }
    }

    pub async fn queue_size(&self) -> usize {
        self.expiration_queue.lock().await.len()
    }

    async fn add_image(&self, base_dir: String, name: String, ttl_secs: u64) -> Result<()> {
        let expires_at = SystemTime::now() + Duration::from_secs(ttl_secs);
        let path: PathBuf = PathBuf::from(base_dir).join(name);

        let mut queue = self.expiration_queue.lock().await;
        queue.push(ExpiringImage { path, expires_at });

        Ok(())
    }

    async fn cleanup_expired(&self) -> usize {
        let mut removed_total = 0;
        let mut queue = self.expiration_queue.lock().await;
        while let Some(expiring_image) = queue.peek() {
            if expiring_image.expires_at > SystemTime::now() {
                break;
            }
            if let Some(image) = queue.pop() {
                if let Err(e) = std::fs::remove_file(&image.path) {
                    error!(
                        "Failed to remove expired image {}: {}",
                        image.path.display(),
                        e
                    );
                } else {
                    removed_total += 1;
                    debug!("Removed expired image: {}", image.path.display());
                }
            }
        }

        removed_total
    }
}

#[derive(Debug)]
pub struct FallbackJanitor {
    // 过期时长
    pub expiration: Duration,
    // 扫描基础目录
    pub base_dir: PathBuf,
    // 清理总数
    pub cleaned_total: usize,
}

impl FallbackJanitor {
    pub fn new(expiration: Duration, base_dir: String) -> Self {
        FallbackJanitor {
            expiration,
            base_dir: PathBuf::from(base_dir),
            cleaned_total: 0,
        }
    }

    pub fn clean_expired_files(&mut self) -> Result<usize> {
        let total = self.scan_and_clean_recursive(&self.base_dir, SystemTime::now(), 0)?;
        self.cleaned_total += total;

        Ok(total)
    }

    fn scan_and_clean_recursive(
        &self,
        dir: &Path,
        now: SystemTime,
        mut total: usize,
    ) -> Result<usize> {
        if !dir.exists() {
            return Ok(total);
        }

        let entries = fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if self.should_delete_file(&path, now)? {
                    if let Err(e) = fs::remove_file(&path) {
                        error!("Failed to delete file {}: {e}", path.display());
                    } else {
                        debug!("Deleted expired file: {}", path.display());
                        total += 1;
                    }
                }
            } else if path.is_dir() {
                // 递归扫描子目录
                total = self.scan_and_clean_recursive(&path, now, total)?;
            }
        }

        Ok(total)
    }

    fn should_delete_file(&self, file_path: &Path, now: SystemTime) -> Result<bool> {
        let metadata = fs::metadata(file_path)?;

        // 获取文件创建时间
        let file_time = match metadata.created() {
            Ok(time) => time,
            Err(_) => {
                // 如果获取创建时间失败，尝试使用修改时间
                metadata.modified()?
            }
        };

        // 计算文件年龄
        match now.duration_since(file_time) {
            Ok(age) => Ok(age > self.expiration),
            Err(_) => {
                // 文件创建时间在未来，可能是系统时间问题，保守起见不删除
                Ok(false)
            }
        }
    }
}

pub fn init() {
    if TTL_JANITOR.get().is_none() {
        TTL_JANITOR
            .set(TTLJanitor::new())
            .expect("Failed to initialize TTLJanitor");
    }

    if FALLBACK_JANITOR.get().is_none() {
        FALLBACK_JANITOR
            .set(RwLock::new(FallbackJanitor::new(
                Duration::from_secs(*MAX_TTL_SECS),    // 过期时间
                String::from(*CAPINDE_NAMESPACE_BASE), // 扫描命名空间目录
            )))
            .expect("Failed to initialize FallbackJanitor");
    }
}

pub fn ttl_janitor() -> &'static TTLJanitor {
    TTL_JANITOR
        .get()
        .expect("The TTLJanitor is not initialized, call janitor::init() first")
}

pub async fn fallback() -> RwLockReadGuard<'static, FallbackJanitor> {
    FALLBACK_JANITOR
        .get()
        .expect("The FallbackJanitor is not initialized, call janitor::init() first")
        .read()
        .await
}

async fn use_fallback() -> RwLockWriteGuard<'static, FallbackJanitor> {
    FALLBACK_JANITOR
        .get()
        .expect("The FallbackJanitor is not initialized, call janitor::init() first")
        .write()
        .await
}

pub async fn collect(base_dir: String, name: &str, ttl_secs: u64) {
    match ttl_janitor()
        .add_image(base_dir, name.to_string(), ttl_secs)
        .await
    {
        Ok(_) => debug!("Image added to janitor: {name} with TTL: {ttl_secs} seconds"),
        Err(e) => error!("Failed to add image to janitor: {e}"),
    }
}

pub async fn ttl_cleanup() {
    debug!("Starting cleanup of expired images...");
    let removed_total = ttl_janitor().cleanup_expired().await;
    if removed_total > 0 {
        info!("Cleaned up {removed_total} expired image(s)");
    } else {
        debug!("No expired images to clean up");
    }
}

pub async fn fallback_cleanup() {
    debug!("Starting fallback cleanup of expired files...");
    match use_fallback().await.clean_expired_files() {
        Ok(total) if total > 0 => info!("Cleaned up {total} expired file(s)"),
        Ok(_) => debug!("Fallback cleanup completed"),
        Err(e) => error!("Failed to clean expired files: {e}"),
    }
}

pub async fn queue_size() -> usize {
    ttl_janitor().queue_size().await
}
