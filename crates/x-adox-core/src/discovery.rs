use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use x_adox_bitnet::BitNetModel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub struct PythonScript {
    pub name: String,
    pub path: PathBuf,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub enum AddonType {
    Scenery { airports: Vec<String> },
    Aircraft(String), // String name of the .acf file
    Plugin { scripts: Vec<PythonScript> },
    CSL(bool), // bool is_enabled
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq)]
pub struct DiscoveredAddon {
    pub path: PathBuf,
    pub name: String,
    pub addon_type: AddonType,
    pub is_enabled: bool,
    pub tags: Vec<String>,
}

pub struct DiscoveryManager;

fn is_path_excluded(path: &Path, exclusions: &[PathBuf]) -> bool {
    exclusions.iter().any(|ex| path.starts_with(ex))
}

impl DiscoveryManager {
    fn scan_folder(
        dir: &Path,
        is_enabled: bool,
        cache: &mut crate::cache::DiscoveryCache,
        bitnet: &BitNetModel,
        results: &mut Vec<DiscoveredAddon>,
        exclusions: &[PathBuf],
    ) {
        if !dir.exists() {
            return;
        }

        // Check if this folder itself is excluded
        if is_path_excluded(dir, exclusions) {
            return;
        }

        if let Some(entry) = cache.get(dir) {
            // Filter cached results based on current exclusions
            let filtered: Vec<_> = entry
                .addons
                .iter()
                .filter(|addon| !is_path_excluded(&addon.path, exclusions))
                .cloned()
                .collect();
            results.extend(filtered);
            return;
        }

        let mut folder_results = Vec::new();
        let walker = WalkDir::new(dir).follow_links(true);

        // Pre-convert exclusions to absolute strings for simpler comparison if needed,
        // but Path::starts_with is usually fine if both are absolute.
        // We assume exclusions are absolute since they come from file picker.

        let it = walker.into_iter().filter_entry(|e| {
            if is_hidden(e) {
                return false;
            }
            if e.file_type().is_dir() {
                let path = e.path();
                // If any exclusion starts with this path (parent), we must descend to find it?
                // No, we want to exclude if THIS path starts with an exclusion.
                if exclusions.iter().any(|ex| path.starts_with(ex)) {
                    return false; // Skip this directory and its children
                }
            }
            true
        });

        for entry in it {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Directory check moved to filter_entry, so we only need to handle files

            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "acf" {
                        let acf_name = entry.file_name().to_string_lossy().to_string();
                        if let Some(parent) = entry.path().parent() {
                            let path = parent.to_path_buf();
                            let name = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            if !folder_results
                                .iter()
                                .any(|d: &DiscoveredAddon| d.path == path)
                            {
                                let tags = bitnet.predict_aircraft_tags(&name, &path);
                                folder_results.push(DiscoveredAddon {
                                    path,
                                    name,
                                    addon_type: AddonType::Aircraft(acf_name),
                                    is_enabled,
                                    tags,
                                });
                            }
                        }
                    }
                }
            }
        }
        cache.insert(
            dir.to_path_buf(),
            folder_results.clone(),
            Vec::new(),
            Vec::new(),
        );
        results.extend(folder_results);
    }

    pub fn scan_aircraft(
        root: &Path,
        cache: &mut crate::cache::DiscoveryCache,
        exclusions: &[PathBuf],
    ) -> Vec<DiscoveredAddon> {
        let mut results = Vec::new();

        let aircraft_root = root.join("Aircraft");
        let disabled_root = root.join("Aircraft (Disabled)");

        let bitnet = BitNetModel::new().unwrap_or_default();

        DiscoveryManager::scan_folder(
            &aircraft_root,
            true,
            cache,
            &bitnet,
            &mut results,
            exclusions,
        );
        DiscoveryManager::scan_folder(
            &disabled_root,
            false,
            cache,
            &bitnet,
            &mut results,
            exclusions,
        );

        results
    }

    /// Scans Custom Scenery for valid packs.
    pub fn scan_scenery(
        root: &Path,
        cache: &mut crate::cache::DiscoveryCache,
    ) -> Vec<DiscoveredAddon> {
        if let Some(entry) = cache.get(root) {
            return entry.addons.clone();
        }
        let mut results = Vec::new();

        let read_dir = match std::fs::read_dir(root) {
            Ok(rd) => rd,
            Err(_) => return results,
        };

        for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if path.is_dir() && !name.starts_with('.') {
                results.push(DiscoveredAddon {
                    path: path.clone(),
                    name,
                    addon_type: AddonType::Scenery {
                        airports: Vec::new(),
                    },
                    is_enabled: true, // Default to true, calling code should reconcile with scenery_packs.ini
                    tags: Vec::new(),
                });
            }
        }
        cache.insert(root.to_path_buf(), results.clone(), Vec::new(), Vec::new());
        results
    }

    /// Scans Plugins in Resources/plugins and Resources/plugins (disabled).
    fn scan_plugin_dir(
        dir: &Path,
        enabled: bool,
        root: &Path,
        cache: &mut crate::cache::DiscoveryCache,
        results: &mut Vec<DiscoveredAddon>,
    ) {
        if !dir.exists() {
            return;
        }

        if let Some(entry) = cache.get(dir) {
            results.extend(entry.addons.clone());
            return;
        }

        let mut dir_results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    if name == "PythonScripts"
                        || name == "PythonPlugins"
                        || name == "X-Ivap Resources"
                        || name == "xPilot"
                        || name.starts_with('.')
                    {
                        continue;
                    }

                    let has_xpl = std::fs::read_dir(&path)
                        .map(|mut r| {
                            r.any(|e| {
                                e.ok()
                                    .map(|ee| {
                                        ee.path()
                                            .extension()
                                            .map(|ext| ext == "xpl")
                                            .unwrap_or(false)
                                    })
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                        || path.join("64").exists()
                        || path.join("lin_x64").exists();

                    if has_xpl {
                        dir_results.push(DiscoveredAddon {
                            path: path.clone(),
                            name: name.clone(),
                            addon_type: AddonType::Plugin {
                                scripts: DiscoveryManager::scan_python_scripts(root, "XPPython3"),
                            },
                            is_enabled: enabled,
                            tags: Vec::new(),
                        });
                    }
                }
            }
        }
        cache.insert(
            dir.to_path_buf(),
            dir_results.clone(),
            Vec::new(),
            Vec::new(),
        );
        results.extend(dir_results);
    }

    /// Scans Plugins in Resources/plugins and Resources/plugins (disabled).
    pub fn scan_plugins(
        root: &Path,
        cache: &mut crate::cache::DiscoveryCache,
    ) -> Vec<DiscoveredAddon> {
        let mut results = Vec::new();
        let plugins_dir = root.join("Resources").join("plugins");
        let disabled_dir = root.join("Resources").join("plugins (disabled)");

        DiscoveryManager::scan_plugin_dir(&plugins_dir, true, root, cache, &mut results);
        DiscoveryManager::scan_plugin_dir(&disabled_dir, false, root, cache, &mut results);

        results
    }

    /// Scans for CSL packages in X-IvAp, xPilot, and Custom Data directories.
    pub fn scan_csls(
        root: &Path,
        cache: &mut crate::cache::DiscoveryCache,
    ) -> Vec<DiscoveredAddon> {
        let mut results = Vec::new();
        let mut csl_roots = Vec::new();

        // 1. Standard roots
        csl_roots.push(root.join("Custom Data"));

        // 2. Dynamic Plugin Scan
        let plugins_dir = root.join("Resources").join("plugins");
        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    if name.starts_with('.') {
                        continue;
                    }

                    // Check direct CSL folder (e.g. IVAO_CSL/CSL)
                    csl_roots.push(path.clone());
                    // Check nested Resources/CSL folder (e.g. xPilot/Resources/CSL)
                    csl_roots.push(path.join("Resources"));
                }
            }
        }

        // Deduplicate and filter non-existent roots
        let mut unique_roots = std::collections::HashSet::new();
        let filtered_roots: Vec<_> = csl_roots
            .into_iter()
            .filter(|p| p.exists() && unique_roots.insert(p.clone()))
            .collect();

        for csl_root in filtered_roots {
            if !csl_root.exists() {
                continue;
            }

            if let Some(entry) = cache.get(&csl_root) {
                results.extend(entry.addons.clone());
                continue;
            }

            let mut csl_results = Vec::new();
            let enabled_path = csl_root.join("CSL");
            let disabled_path = csl_root.join("CSL (disabled)");

            // Scan enabled
            if enabled_path.exists() {
                if let Ok(entries) = std::fs::read_dir(enabled_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("xsb_aircraft.txt").exists() {
                            csl_results.push(DiscoveredAddon {
                                name: path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path: path.clone(),
                                addon_type: AddonType::CSL(true),
                                is_enabled: true,
                                tags: Vec::new(),
                            });
                        }
                    }
                }
            }

            // Scan disabled
            if disabled_path.exists() {
                if let Ok(entries) = std::fs::read_dir(disabled_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("xsb_aircraft.txt").exists() {
                            csl_results.push(DiscoveredAddon {
                                name: path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path: path.clone(),
                                addon_type: AddonType::CSL(false),
                                is_enabled: false,
                                tags: Vec::new(),
                            });
                        }
                    }
                }
            }
            cache.insert(
                csl_root.clone(),
                csl_results.clone(),
                Vec::new(),
                Vec::new(),
            );
            results.extend(csl_results);
        }

        results
    }

    /// Scans for Python scripts in standard script folders.
    pub fn scan_python_scripts(root: &Path, plugin_name: &str) -> Vec<PythonScript> {
        let mut results = Vec::new();
        let script_dir = match plugin_name {
            "PythonInterface" => root.join("Resources").join("plugins").join("PythonScripts"),
            "XPPython3" => root.join("Resources").join("plugins").join("PythonPlugins"),
            _ => return results,
        };

        if !script_dir.exists() {
            return results;
        }

        if let Ok(entries) = std::fs::read_dir(script_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                if path.is_file() && (name.ends_with(".py") || name.ends_with(".py.disabled")) {
                    results.push(PythonScript {
                        name: name.clone(),
                        path: path.clone(),
                        is_enabled: name.ends_with(".py"),
                    });
                }
            }
        }

        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}
