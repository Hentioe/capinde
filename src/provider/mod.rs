pub mod archive;
mod initializer;
pub mod manifest;

pub use initializer::{init, reinit};

use itertools::Itertools;
use manifest::{Album, Manifest};
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{OnceLock, RwLock, RwLockReadGuard},
};

use crate::errors::Result;
use crate::{err, fail};

static MANIFEST: OnceLock<RwLock<Manifest>> = OnceLock::new();
static ALBUM_IMAGES: OnceLock<RwLock<HashMap<String, Vec<PathBuf>>>> = OnceLock::new();
static CONFLICTS: OnceLock<RwLock<Conflicts>> = OnceLock::new();

#[derive(Debug, Clone)]
struct ConflictPair(String, String);

impl ConflictPair {
    pub fn new(a: &str, b: &str) -> Self {
        Self(a.to_string(), b.to_string())
    }
}

impl PartialEq for ConflictPair {
    fn eq(&self, other: &Self) -> bool {
        // (a, b) == (b, a) 或 (a, b) == (a, b)
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

impl Eq for ConflictPair {}

impl Hash for ConflictPair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // 为了保证哈希一致性，我们总是以规范化（例如，按字母顺序排序）的形式哈希这对字符串
        let (first, second) = if self.0 < self.1 {
            (&self.0, &self.1)
        } else {
            (&self.1, &self.0)
        };
        first.hash(state);
        second.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct Conflicts {
    pairs: HashSet<ConflictPair>,
}

impl Conflicts {
    pub fn from(vec: &Vec<Vec<String>>) -> Self {
        let mut pairs = HashSet::new();
        for conflict in vec {
            for pair in conflict.iter().combinations(2) {
                pairs.insert(ConflictPair::new(pair[0], pair[1]));
            }
        }

        Self { pairs }
    }

    pub fn contains(&self, a: &str, b: &str) -> bool {
        self.pairs
            .contains(&ConflictPair(a.to_string(), b.to_string()))
    }
}

pub fn get_manifest() -> Result<RwLockReadGuard<'static, Manifest>> {
    let gurard = MANIFEST
        .get()
        .ok_or(fail!("manifest not initialized"))?
        .read()
        .map_err(|_| fail!("failed to get manifest"))?;

    Ok(gurard)
}

fn reset_manifest(manifest: Manifest) -> Result<()> {
    let mut guard = MANIFEST
        .get()
        .ok_or(fail!("manifest not initialized"))?
        .write()
        .map_err(|_| fail!("failed to get mutable manifest"))?;

    *guard = manifest;

    Ok(())
}

fn reset_album_images(album_images: HashMap<String, Vec<PathBuf>>) {
    let mut guard = ALBUM_IMAGES
        .get()
        .expect("Album images not initialized")
        .write()
        .expect("Failed to get mutable album images");

    *guard = album_images;
}

fn reset_conflicts(conflicts: Conflicts) {
    let mut guard = CONFLICTS
        .get()
        .expect("Conflicts not initialized")
        .write()
        .expect("Failed to get mutable conflicts");

    *guard = conflicts;
}

pub fn images_get(album_id: &str) -> Option<Vec<PathBuf>> {
    let albums = ALBUM_IMAGES
        .get()
        .expect("Album images not initialized")
        .read()
        .expect("Failed to read album images");

    albums.get(album_id).cloned()
}

pub fn total_images() -> usize {
    let albums = ALBUM_IMAGES
        .get()
        .expect("Album images not initialized")
        .read()
        .expect("Failed to read album images");

    albums.values().map(|v| v.len()).sum()
}

pub fn is_conflict(album1: &str, album2: &str) -> bool {
    CONFLICTS
        .get()
        .expect("Conflicts not initialized")
        .read()
        .expect("Failed to read conflicts")
        .contains(album1, album2)
}

pub fn random_right_with_wrongs(
    right_min_children: usize,
    total_albums: usize,
) -> Result<(Album, Vec<Album>)> {
    let manifest = get_manifest()?;
    let mut albums = manifest.albums.iter().collect::<Vec<&Album>>();
    let mut right = None;
    let mut wrongs = vec![];
    // 打乱图集列表
    albums.shuffle(&mut rand::rng());
    // 找出第一个包含 right_min_children 张图片的图集作为正确答案
    for album in albums.iter() {
        if let Some(images) = images_get(&album.id) {
            if images.len() >= right_min_children {
                right = Some((*album).clone());
                break;
            }
        }
    }

    let right = if let Some(album) = right {
        album
    } else {
        return err!("no album with enough images found");
    };

    // 从图集列表中选择特定数量无冲突的错误答案
    while wrongs.len() < total_albums - 1 {
        if albums.is_empty() {
            return err!("not enough albums to generate wrongs");
        }
        // 随机选择一个图集
        let random_index = rand::rng().random_range(0..albums.len());
        let random = albums.remove(random_index);

        if random.id == right.id {
            // 如果生成了正确答案，继续下一个循环
            continue;
        } else if is_conflict(&right.id, &random.id) {
            // 如果冲突，继续
            continue;
        } else if wrongs.iter().any(|a: &Album| a.id == random.id) {
            // 如果已经存在于 wrongs 中，继续
            continue;
        } else {
            // 没有冲突，添加到 wrongs
            wrongs.push((*random).clone());
        }
    }

    Ok((right, wrongs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        super::init();
    }

    #[test]
    fn test_pair() {
        // 乱序对应该相等
        assert_eq!(
            ConflictPair::new("cats", "dogs"),
            ConflictPair::new("dogs", "cats")
        );
        // 乱序对的 hash 相等
        let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
        ConflictPair::new("cats", "dogs").hash(&mut hasher1);
        ConflictPair::new("dogs", "cats").hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());
        // 不同的内容不相等
        assert_ne!(
            ConflictPair::new("cats", "dogs"),
            ConflictPair::new("cats", "birds")
        );
    }

    #[test]
    fn test_conflicts() {
        setup();

        let conflicts = Conflicts::from(&vec![
            vec!["cats".to_string(), "dogs".to_string()],
            vec!["cats".to_string(), "birds".to_string()],
            vec!["dogs".to_string(), "fishs".to_string(), "birds".to_string()],
        ]);

        // 判断冲突与参数顺序无关
        assert!(conflicts.contains("cats", "dogs"));
        assert!(conflicts.contains("dogs", "cats"));
        // 重复出现的元素（cat）会合并而不是覆盖
        assert!(conflicts.contains("cats", "birds"));
        // 两个以上也能正确建立关系
        assert!(conflicts.contains("dogs", "fishs"));
        assert!(conflicts.contains("fishs", "dogs"));
        assert!(conflicts.contains("dogs", "birds"));
    }

    #[test]
    fn test_random_right_with_wrongs() {
        setup();

        let (right, wrongs) = random_right_with_wrongs(1, 9).unwrap();
        // 生成数量是否满足
        assert_eq!(wrongs.len(), 8);

        // 判断 right 不在 wrongs 中
        assert!(!wrongs.iter().any(|a| a.id == right.id));

        for other in wrongs.iter() {
            // 判断 right 是否不和 wrongs 中的任何一个冲突
            assert!(!is_conflict(&right.id, &other.id));
        }
    }
}
