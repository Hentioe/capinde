use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::errors::{Error, Result};

pub static LATEST_VERSION: &str = "0.1.2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub datetime: DateTime<Utc>,
    pub width: Option<usize>,
    pub include_formats: Vec<String>,
    pub albums: Vec<Album>,
    pub conflicts: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: I18nName,
}

impl std::fmt::Display for Album {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Album(id: {})", self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct I18nName {
    #[serde(rename = "zh-hans")]
    pub zh_hans: Option<String>,
    #[serde(rename = "zh-hant")]
    pub zh_hant: Option<String>,
    pub en: Option<String>,
}

impl Manifest {
    fn load(path: &PathBuf) -> Result<Manifest> {
        let file = std::fs::File::open(path)?;

        Ok(serde_yaml::from_reader(file)?)
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let file = std::fs::File::create(path)?;
        serde_yaml::to_writer(file, self)?;

        Ok(())
    }
}

impl std::str::FromStr for Manifest {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let manifest: Manifest = serde_yaml::from_str(s)?;

        Ok(manifest)
    }
}

pub fn load(path: &PathBuf) -> Result<Manifest> {
    Manifest::load(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        let manifest = load(&PathBuf::from("tests/fixtures/albums/Manifest.yaml"))
            .expect("Failed to load manifest");

        assert_eq!(manifest.version, LATEST_VERSION);
        assert_eq!(
            manifest.datetime,
            DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap()
        );
        assert_eq!(manifest.width, Some(250));
        assert_eq!(manifest.include_formats, vec!["jpg", "png"]);
        assert_eq!(manifest.albums.len(), 10);
        assert_eq!(manifest.albums[0].id, "cats");
        assert_eq!(manifest.albums[0].name.zh_hans, Some("猫".to_string()));
        assert_eq!(manifest.albums[0].name.zh_hant, Some("貓".to_string()));
        assert_eq!(manifest.albums[0].name.en, Some("Cat".to_string()));
        assert!(manifest.conflicts.as_ref().unwrap().len() > 0);
        for conflict in manifest.conflicts.as_ref().unwrap() {
            assert!(conflict.len() > 0);
        }
    }
}
