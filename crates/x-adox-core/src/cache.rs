use crate::discovery::DiscoveredAddon;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub mtime: chrono::DateTime<chrono::Utc>,
    pub addons: Vec<DiscoveredAddon>,
    #[serde(default)]
    pub airports: Vec<crate::apt_dat::Airport>,
    #[serde(default)]
    pub tiles: Vec<(i32, i32)>,
    #[serde(default)]
    pub descriptor: crate::scenery::SceneryDescriptor,
}

const CURRENT_CACHE_VERSION: u32 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryCache {
    #[serde(default)]
    pub version: u32,
    pub entries: HashMap<PathBuf, CacheEntry>,
}

impl Default for DiscoveryCache {
    fn default() -> Self {
        Self {
            version: CURRENT_CACHE_VERSION,
            entries: HashMap::new(),
        }
    }
}

impl DiscoveryCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(xplane_root: Option<&Path>) -> Self {
        let (path, is_scoped) = if let Some(root) = xplane_root {
            (
                crate::get_scoped_config_root(root).join("discovery_cache.json"),
                true,
            )
        } else {
            (Self::get_cache_path(), false)
        };

        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cache) = serde_json::from_str::<DiscoveryCache>(&content) {
                    if cache.version == CURRENT_CACHE_VERSION {
                        return cache;
                    }
                }
            }
        }

        // --- Migration Fallback ---
        if is_scoped {
            if let Some(config_root) = crate::get_config_root().parent() {
                let legacy_paths = [
                    config_root
                        .join("x-addon-oxide")
                        .join("discovery_cache.json"),
                    config_root.join("x-adox").join("discovery_cache.json"),
                ];

                for legacy_path in &legacy_paths {
                    if legacy_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(legacy_path) {
                            if let Ok(cache) = serde_json::from_str::<DiscoveryCache>(&content) {
                                if cache.version == CURRENT_CACHE_VERSION {
                                    println!(
                                        "[Migration] Loaded legacy cache from {:?}",
                                        legacy_path
                                    );
                                    return cache;
                                }
                            }
                        }
                    }
                }
            }
        }

        Self::new()
    }

    pub fn save(&self, xplane_root: Option<&Path>) -> Result<()> {
        let path = if let Some(root) = xplane_root {
            crate::get_scoped_config_root(root).join("discovery_cache.json")
        } else {
            Self::get_cache_path()
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn get_cache_path() -> PathBuf {
        // We use the same config dir as heuristics
        directories::ProjectDirs::from("org", "x-adox", "X-Addon-Oxide")
            .map(|dirs| dirs.config_dir().join("discovery_cache.json"))
            .unwrap_or_else(|| PathBuf::from("discovery_cache.json"))
    }

    pub fn get(&self, path: &Path) -> Option<&CacheEntry> {
        if let Some(entry) = self.entries.get(path) {
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(mtime) = metadata.modified() {
                    let mtime: chrono::DateTime<chrono::Utc> = mtime.into();
                    if mtime == entry.mtime {
                        return Some(entry);
                    }
                }
            }
        }
        None
    }

    pub fn insert(
        &mut self,
        path: PathBuf,
        addons: Vec<DiscoveredAddon>,
        airports: Vec<crate::apt_dat::Airport>,
        tiles: Vec<(i32, i32)>,
        descriptor: crate::scenery::SceneryDescriptor,
    ) {
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(mtime) = metadata.modified() {
                let mtime: chrono::DateTime<chrono::Utc> = mtime.into();
                self.entries.insert(
                    path,
                    CacheEntry {
                        mtime,
                        addons,
                        airports,
                        tiles,
                        descriptor,
                    },
                );
            }
        }
    }
}
