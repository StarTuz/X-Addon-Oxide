// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use x_adox_bitnet::BitNetModel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub struct AddonScript {
    pub name: String,
    pub path: PathBuf,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub struct AcfVariant {
    pub name: String,
    pub file_name: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub enum AddonType {
    Scenery {
        airports: Vec<String>,
    },
    Aircraft {
        variants: Vec<AcfVariant>,
        livery_count: usize,
        livery_names: Vec<String>,
    },
    Plugin {
        scripts: Vec<AddonScript>,
    },
    CSL(bool), // bool is_enabled
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq)]
pub struct DiscoveredAddon {
    pub path: PathBuf,
    pub name: String,
    pub addon_type: AddonType,
    pub is_enabled: bool,
    pub tags: Vec<String>,
    /// True if this aircraft lives under `Aircraft/Laminar Research/`
    /// or `Aircraft (Disabled)/Laminar Research/`.
    #[serde(default)]
    pub is_laminar_default: bool,
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
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_acf = ext == "acf";
                    let is_disabled_acf = file_name.ends_with(".acf.disabled");

                    if is_acf || is_disabled_acf {
                        if let Some(parent) = entry.path().parent() {
                            let path = parent.to_path_buf();
                            let name = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();

                            let variant_name = if is_acf {
                                file_name
                                    .strip_suffix(".acf")
                                    .unwrap_or(&file_name)
                                    .to_string()
                            } else {
                                file_name
                                    .strip_suffix(".acf.disabled")
                                    .unwrap_or(&file_name)
                                    .to_string()
                            };

                            let variant = AcfVariant {
                                name: variant_name,
                                file_name: file_name.clone(),
                                is_enabled: is_acf,
                            };

                            if let Some(existing) = folder_results
                                .iter_mut()
                                .find(|d: &&mut DiscoveredAddon| d.path == path)
                            {
                                if let AddonType::Aircraft { variants, .. } =
                                    &mut existing.addon_type
                                {
                                    if !variants.iter().any(|v| v.file_name == variant.file_name) {
                                        variants.push(variant);
                                    }
                                }
                            } else {
                                let tags = bitnet.predict_aircraft_tags(&name, &path);
                                let is_laminar = path
                                    .components()
                                    .any(|c| c.as_os_str() == "Laminar Research");
                                folder_results.push(DiscoveredAddon {
                                    path: path.clone(),
                                    name: name.clone(),
                                    addon_type: AddonType::Aircraft {
                                        variants: vec![variant],
                                        livery_count: DiscoveryManager::count_liveries(&path),
                                        livery_names: DiscoveryManager::get_livery_names(&path),
                                    },
                                    is_enabled,
                                    tags,
                                    is_laminar_default: is_laminar,
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
            crate::scenery::SceneryDescriptor::default(),
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
                    is_enabled: true,
                    tags: Vec::new(),
                    is_laminar_default: false,
                });
            }
        }

        cache.insert(
            root.to_path_buf(),
            results.clone(),
            Vec::new(),
            Vec::new(),
            crate::scenery::SceneryDescriptor::default(),
        );
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
                        // Determine script type based on plugin
                        let scripts = if name == "FlyWithLua" || name == "X-Lua" {
                            DiscoveryManager::scan_lua_scripts(&path)
                        } else if name == "XPPython3" {
                            DiscoveryManager::scan_python_scripts(root, "XPPython3")
                        } else if name == "PythonInterface" {
                            DiscoveryManager::scan_python_scripts(root, "PythonInterface")
                        } else {
                            Vec::new()
                        };

                        dir_results.push(DiscoveredAddon {
                            path: path.clone(),
                            name: name.clone(),
                            addon_type: AddonType::Plugin { scripts },
                            is_enabled: enabled,
                            tags: Vec::new(),
                            is_laminar_default: false,
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
            crate::scenery::SceneryDescriptor::default(),
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
                                is_laminar_default: false,
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
                                is_laminar_default: false,
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
                crate::scenery::SceneryDescriptor::default(),
            );
            results.extend(csl_results);
        }

        results
    }

    /// Scans for Python scripts in standard script folders.
    pub fn scan_python_scripts(root: &Path, plugin_name: &str) -> Vec<AddonScript> {
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
                    results.push(AddonScript {
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

    /// Scans for Lua scripts in a FlyWithLua/X-Lua plugin folder.
    /// Detects scripts in both `Scripts/` (enabled) and `Scripts (disabled)/` (disabled).
    /// Also detects the `.lua.disabled` suffix convention within `Scripts/`.
    pub fn scan_lua_scripts(plugin_path: &Path) -> Vec<AddonScript> {
        let mut results = Vec::new();

        // 1. Scan active Scripts/ folder
        let script_dir = plugin_path.join("Scripts");
        if script_dir.exists() {
            Self::collect_lua_files(&script_dir, true, &mut results);
        }

        // 2. Scan disabled Scripts (disabled)/ folder (FlyWithLua native convention)
        let disabled_dir = plugin_path.join("Scripts (disabled)");
        if disabled_dir.exists() {
            Self::collect_lua_files(&disabled_dir, false, &mut results);
        }

        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    /// Recursively collects `.lua` files from a directory.
    /// Files with `.lua.disabled` suffix in an enabled dir are treated as disabled.
    fn collect_lua_files(dir: &Path, dir_is_enabled: bool, results: &mut Vec<AddonScript>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                if path.is_dir() {
                    // Recurse into subdirectories (e.g. Scripts (disabled)/Custom/)
                    Self::collect_lua_files(&path, dir_is_enabled, results);
                } else if name.ends_with(".lua") || name.ends_with(".lua.disabled") {
                    let is_enabled = dir_is_enabled && name.ends_with(".lua");
                    results.push(AddonScript {
                        name: name.clone(),
                        path: path.clone(),
                        is_enabled,
                    });
                }
            }
        }
    }

    pub fn count_liveries(aircraft_path: &Path) -> usize {
        let liveries_path = aircraft_path.join("liveries");
        if !liveries_path.exists() || !liveries_path.is_dir() {
            return 0;
        }

        match std::fs::read_dir(liveries_path) {
            Ok(entries) => entries.flatten().filter(|e| e.path().is_dir()).count(),
            Err(_) => 0,
        }
    }

    pub fn get_livery_names(aircraft_path: &Path) -> Vec<String> {
        let liveries_path = aircraft_path.join("liveries");
        if !liveries_path.exists() || !liveries_path.is_dir() {
            return Vec::new();
        }

        match std::fs::read_dir(liveries_path) {
            Ok(entries) => {
                let mut names: Vec<_> = entries
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                names.sort();
                names
            }
            Err(_) => Vec::new(),
        }
    }

    /// Scans an aircraft folder for PDF manuals.
    /// Checks the root folder and common documentation subfolders.
    pub fn find_manuals(aircraft_path: &Path) -> Vec<PathBuf> {
        let mut manuals = Vec::new();
        let doc_dirs = [
            "", // root folder itself
            "Documentation",
            "Manuals",
            "Manual",
            "Docs",
            "docs",
            "documentation",
            "manuals",
            "manual",
            "Reference",
            "reference",
        ];

        for dir_name in &doc_dirs {
            let dir = if dir_name.is_empty() {
                aircraft_path.to_path_buf()
            } else {
                aircraft_path.join(dir_name)
            };
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if ext.eq_ignore_ascii_case("pdf") {
                                manuals.push(path);
                            }
                        }
                    }
                }
            }
        }

        manuals.sort();
        manuals.dedup();
        manuals
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}
