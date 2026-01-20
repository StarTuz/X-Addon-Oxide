use anyhow::Result;
use crate::discovery::DiscoveredAddon;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub mtime: SystemTime,
    pub addons: Vec<DiscoveredAddon>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryCache {
    pub entries: HashMap<PathBuf, CacheEntry>,
}

impl DiscoveryCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Self {
        let path = Self::get_cache_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(cache) = serde_json::from_str(&content) {
                    return cache;
                }
            }
        }
        Self::new()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_cache_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn get_cache_path() -> PathBuf {
        // We use the same config dir as heuristics
        directories::ProjectDirs::from("com", "x-adox", "X-Addon-Oxide")
            .map(|dirs| dirs.config_dir().join("discovery_cache.json"))
            .unwrap_or_else(|| PathBuf::from("discovery_cache.json"))
    }

    pub fn get(&self, path: &Path) -> Option<&Vec<DiscoveredAddon>> {
        if let Some(entry) = self.entries.get(path) {
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(mtime) = metadata.modified() {
                    if mtime == entry.mtime {
                        return Some(&entry.addons);
                    }
                }
            }
        }
        None
    }

    pub fn insert(&mut self, path: PathBuf, addons: Vec<DiscoveredAddon>) {
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(mtime) = metadata.modified() {
                self.entries.insert(path, CacheEntry { mtime, addons });
            }
        }
    }
}
