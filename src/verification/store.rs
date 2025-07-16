use crate::models::params::verification::Answer;
use log::{debug, info};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    sync::{Arc, LazyLock},
    time::{Duration, SystemTime},
};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

type Store = HashMap<Arc<String>, Answer>;
static STORE: LazyLock<RwLock<Store>> = LazyLock::new(|| RwLock::new(HashMap::new()));
static CACHE: LazyLock<RwLock<Cache>> = LazyLock::new(|| RwLock::new(Cache::new()));

struct Cache {
    queue: Mutex<BinaryHeap<Expiring>>,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            queue: Mutex::new(BinaryHeap::new()),
        }
    }

    pub async fn add(&self, expiring: Expiring) {
        let mut queue = self.queue.lock().await;
        queue.push(expiring);
    }

    // 清理已过期的答案缓存
    pub async fn cleanup_expired(&self) -> usize {
        let mut count = 0;
        let mut queue = self.queue.lock().await;
        while let Some(expiring) = queue.peek() {
            if expiring.expires_at > SystemTime::now() {
                break;
            }
            if let Some(expiring) = queue.pop() {
                let mut store = use_store().await;
                // 删除答案缓存
                store.remove(&expiring.unique_id);
                debug!(
                    "Removed expired verification cache form generated: {}",
                    expiring.unique_id
                );
                count += 1;
            }
        }

        count
    }
}

pub async fn cleanup_expired() {
    let removed_count = cache().await.cleanup_expired().await;
    if removed_count > 0 {
        info!("Removed {removed_count} expired verification cache(s)");
    } else {
        debug!("No expired verification caches to remove");
    }
}

async fn use_store() -> RwLockWriteGuard<'static, Store> {
    STORE.write().await
}

async fn store() -> RwLockReadGuard<'static, Store> {
    STORE.read().await
}

async fn cache() -> RwLockReadGuard<'static, Cache> {
    CACHE.read().await
}

pub async fn add_cache(unique_id: String, answer: Answer, ttl_secs: u64) {
    let key = Arc::new(unique_id);
    let mut store = use_store().await;
    let cache = cache().await;
    // 将答案存入缓存
    store.insert(key.clone(), answer);
    // 添加到过期检查
    cache.add(Expiring::new(key, ttl_secs)).await;
}

pub async fn get_cache(unique_id: &str) -> Option<Answer> {
    let store = store().await;

    store.get(&Arc::new(unique_id.to_string())).cloned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expiring {
    // 过期于
    pub expires_at: std::time::SystemTime,
    // ID 的引用
    pub unique_id: Arc<String>,
}

impl Expiring {
    pub fn new(unique_id: Arc<String>, ttl_secs: u64) -> Self {
        let expires_at = SystemTime::now() + Duration::from_secs(ttl_secs);
        Expiring {
            expires_at,
            unique_id,
        }
    }
}

impl Ord for Expiring {
    fn cmp(&self, other: &Self) -> Ordering {
        other.expires_at.cmp(&self.expires_at) // 将过期时间更早的放在前面
    }
}

impl PartialOrd for Expiring {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
