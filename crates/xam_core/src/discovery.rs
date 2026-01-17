use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PythonScript {
    pub name: String,
    pub path: PathBuf,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AddonType {
    Scenery { airports: Vec<String> },
    Aircraft(String), // String name of the .acf file
    Plugin { scripts: Vec<PythonScript> },
    CSL(bool), // bool is_enabled
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredAddon {
    pub path: PathBuf,
    pub name: String,
    pub addon_type: AddonType,
    pub is_enabled: bool,
}

pub struct DiscoveryManager;

impl DiscoveryManager {
    /// Scans a given root directory for Aircraft.
    /// Returns a list of paths to directories containing .acf files.
    pub fn scan_aircraft(root: &Path) -> Vec<DiscoveredAddon> {
        let mut results = Vec::new();
        let walker = WalkDir::new(root).into_iter();

        for entry in walker.filter_entry(|e| !is_hidden(e)) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "acf" {
                        let acf_name = entry.file_name().to_string_lossy().to_string();
                        if let Some(parent) = entry.path().parent() {
                            if !results.iter().any(|d: &DiscoveredAddon| d.path == parent) {
                                results.push(DiscoveredAddon {
                                    path: parent.to_path_buf(),
                                    name: parent
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    addon_type: AddonType::Aircraft(acf_name),
                                    is_enabled: true,
                                });
                            }
                        }
                    }
                }
            }
        }
        results
    }

    /// Scans Custom Scenery for valid packs.
    pub fn scan_scenery(root: &Path) -> Vec<DiscoveredAddon> {
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
            if path.is_dir() {
                results.push(DiscoveredAddon {
                    path: path.clone(),
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    addon_type: AddonType::Scenery {
                        airports: Vec::new(),
                    },
                    is_enabled: true, // Default to true, calling code should reconcile with scenery_packs.ini
                });
            }
        }
        results
    }

    /// Scans Plugins in Resources/plugins and Resources/plugins (disabled).
    pub fn scan_plugins(root: &Path) -> Vec<DiscoveredAddon> {
        let mut results = Vec::new();
        let plugins_dir = root.join("Resources").join("plugins");
        let disabled_dir = root.join("Resources").join("plugins (disabled)");

        // Helper to scan a specific directory
        let scan_dir = |dir: &Path, enabled: bool, results: &mut Vec<DiscoveredAddon>| {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();

                        // Skip standard plugins/folders
                        if name == "PythonScripts"
                            || name == "PythonPlugins"
                            || name == "X-Ivap Resources"
                            || name == "xPilot"
                            || name.starts_with('.')
                        {
                            continue;
                        }

                        // Basic check: is it a plugin? (Has mac.xpl, lin.xpl, win.xpl or is a folder with these?)
                        // For now we assume any folder here is a plugin unless excluded.
                        // Ideally we check for signatures (xpl files).
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
                            || path.join("64").exists() // Often in a '64' subdir
                            || path.join("lin_x64").exists();

                        if has_xpl {
                            results.push(DiscoveredAddon {
                                path: path.clone(),
                                name,
                                addon_type: AddonType::Plugin {
                                    scripts: DiscoveryManager::scan_python_scripts(
                                        root,
                                        "XPPython3",
                                    ), // Simple hook, might need refinement
                                },
                                is_enabled: enabled,
                            });
                        }
                    }
                }
            }
        };

        scan_dir(&plugins_dir, true, &mut results);
        scan_dir(&disabled_dir, false, &mut results);

        results
    }

    /// Scans for CSL packages in X-IvAp, xPilot, and Custom Data directories.
    pub fn scan_csls(root: &Path) -> Vec<DiscoveredAddon> {
        let mut results = Vec::new();
        let csl_roots = [
            root.join("Resources")
                .join("plugins")
                .join("X-Ivap Resources"),
            root.join("Resources")
                .join("plugins")
                .join("xPilot")
                .join("Resources"),
            root.join("Custom Data"),
        ];

        for csl_root in csl_roots {
            if !csl_root.exists() {
                continue;
            }

            let enabled_path = csl_root.join("CSL");
            let disabled_path = csl_root.join("CSL (disabled)");

            // Scan enabled
            if enabled_path.exists() {
                if let Ok(entries) = std::fs::read_dir(enabled_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("xsb_aircraft.txt").exists() {
                            results.push(DiscoveredAddon {
                                name: path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path: path.clone(),
                                addon_type: AddonType::CSL(true),
                                is_enabled: true,
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
                            results.push(DiscoveredAddon {
                                name: path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path: path.clone(),
                                addon_type: AddonType::CSL(false),
                                is_enabled: false,
                            });
                        }
                    }
                }
            }
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
