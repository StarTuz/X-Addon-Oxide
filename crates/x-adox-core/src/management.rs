// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use crate::profiles::{ProfileCollection, ProfileManager};
use crate::scenery::SceneryManager;
use crate::XPlaneManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use x_adox_bitnet::BitNetModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddonType {
    Scenery,
    Aircraft,
    Plugins,
    CSLs,
}

pub struct ModManager;

impl ModManager {
    /// Enables or disables a plugin by moving it between `Resources/plugins` and `Resources/plugins (disabled)`.
    pub fn set_plugin_enabled(
        xplane_root: &Path,
        plugin_path: &Path,
        enabled: bool,
    ) -> Result<PathBuf, std::io::Error> {
        let name = plugin_path.file_name().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid plugin path",
        ))?;

        let plugins_dir = xplane_root.join("Resources").join("plugins");
        let disabled_dir = xplane_root.join("Resources").join("plugins (disabled)");

        if !disabled_dir.exists() {
            fs::create_dir_all(&disabled_dir)?;
        }

        let target_dir = if enabled { &plugins_dir } else { &disabled_dir };
        let target_path = target_dir.join(name);

        if plugin_path != target_path {
            fs::rename(plugin_path, &target_path)?;
        }

        Ok(target_path)
    }

    /// Enables or disables a plugin sub-script by moving it between
    /// `Scripts/` and `Scripts (disabled)/` within the plugin folder.
    /// Follows FlyWithLua's native convention for script management.
    pub fn set_script_enabled(
        script_path: &Path,
        enabled: bool,
    ) -> Result<PathBuf, std::io::Error> {
        let script_name = script_path.file_name().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid script path",
        ))?;

        // Walk up from the script to find the plugin root.
        // Script lives in: <plugin>/Scripts/<file> or <plugin>/Scripts (disabled)/.../<file>
        let mut plugin_root = None;
        let mut ancestor = script_path.parent();
        while let Some(dir) = ancestor {
            let dir_name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if dir_name == "scripts" || dir_name == "scripts (disabled)" {
                plugin_root = dir.parent();
                break;
            }
            ancestor = dir.parent();
        }

        let plugin_root = plugin_root.ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cannot determine plugin root from script path",
        ))?;

        let scripts_dir = plugin_root.join("Scripts");
        let disabled_dir = plugin_root.join("Scripts (disabled)");

        let target_dir = if enabled { &scripts_dir } else { &disabled_dir };

        if !target_dir.exists() {
            fs::create_dir_all(target_dir)?;
        }

        let target_path = target_dir.join(script_name);

        if script_path != target_path {
            fs::rename(script_path, &target_path)?;
        }

        // Clean up empty parent directories in the source tree
        if let Some(parent) = script_path.parent() {
            let parent_name = parent
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            // Only clean up subdirectories within Scripts (disabled), not the root dirs
            if parent_name != "scripts" && parent_name != "scripts (disabled)" {
                if parent.read_dir()?.next().is_none() {
                    let _ = fs::remove_dir(parent);
                }
            }
        }

        Ok(target_path)
    }

    /// Enables or disables a scenery pack by modifying `scenery_packs.ini`.
    pub fn set_scenery_enabled(
        xplane_root: &Path,
        scenery_name: &str,
        enabled: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let scenery_ini_path = xplane_root.join("Custom Scenery").join("scenery_packs.ini");
        let mut manager = SceneryManager::new(scenery_ini_path);
        manager.load()?;

        if enabled {
            manager.enable_pack(scenery_name);
        } else {
            manager.disable_pack(scenery_name);
        }

        manager.save(None)?;
        Ok(())
    }

    pub fn set_bulk_scenery_enabled(
        xplane_root: &Path,
        states: &std::collections::HashMap<String, bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let scenery_ini_path = xplane_root.join("Custom Scenery").join("scenery_packs.ini");
        let mut manager = SceneryManager::new(scenery_ini_path);
        manager.load()?;
        manager.set_bulk_states(states);
        manager.save(None)?;
        Ok(())
    }

    /// Enables or disables an aircraft by moving it between `Aircraft` and `Aircraft (Disabled)`.
    pub fn set_aircraft_enabled(
        xplane_root: &Path,
        path: &Path,
        enabled: bool,
    ) -> Result<PathBuf, std::io::Error> {
        let aircraft_root = xplane_root.join("Aircraft");
        let disabled_root = xplane_root.join("Aircraft (Disabled)");

        // Determine source and target roots based on current state
        let (source_root, target_root) = if enabled {
            (&disabled_root, &aircraft_root)
        } else {
            (&aircraft_root, &disabled_root)
        };

        // Calculate relative path from source root
        let relative_path = path.strip_prefix(source_root).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Path {} is not inside source root {}",
                    path.display(),
                    source_root.display()
                ),
            )
        })?;

        let target_path = target_root.join(relative_path);

        // Ensure parent directory exists in target
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Move the file/bucket
        fs::rename(path, &target_path)?;

        // Optional: Clean up empty parent directories in source
        if let Some(parent) = path.parent() {
            // Only remove if it's not the root itself and is empty
            if parent != source_root && parent.read_dir()?.next().is_none() {
                let _ = fs::remove_dir(parent);
            }
        }

        Ok(target_path)
    }

    /// Enables or disables an individual aircraft variant within its folder.
    /// Renames <name>.acf <-> <name>.acf.disabled
    pub fn set_variant_enabled(
        aircraft_path: &Path,
        variant_file_name: &str,
        enabled: bool,
    ) -> Result<PathBuf, std::io::Error> {
        let source_path = aircraft_path.join(variant_file_name);

        let target_file_name = if enabled {
            // "B737.acf.disabled" -> "B737.acf"
            variant_file_name
                .strip_suffix(".disabled")
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Variant is already enabled or invalid name",
                    )
                })?
                .to_string()
        } else {
            // "B737.acf" -> "B737.acf.disabled"
            if variant_file_name.ends_with(".disabled") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Variant is already disabled",
                ));
            }
            format!("{}.disabled", variant_file_name)
        };

        let target_path = aircraft_path.join(target_file_name);

        if source_path.exists() && source_path != target_path {
            fs::rename(&source_path, &target_path)?;
        } else if !source_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Variant file not found: {}", source_path.display()),
            ));
        }

        Ok(target_path)
    }

    /// Deletes an addon and performs necessary cleanup (e.g. removing scenery from INI).
    pub fn delete_addon(
        xplane_root: &Path,
        path: &Path,
        addon_type: AddonType,
    ) -> Result<(), String> {
        // Resolve the path
        let full_path = if path.is_relative() {
            xplane_root.join(path)
        } else {
            path.to_path_buf()
        };

        // Safety check - make sure we're deleting from the right folder
        if matches!(addon_type, AddonType::CSLs) {
            // CSL can be in CSL or CSL (disabled) under any of the standard CSL roots
            let csl_roots = [
                xplane_root
                    .join("Resources")
                    .join("plugins")
                    .join("X-Ivap Resources"),
                xplane_root
                    .join("Resources")
                    .join("plugins")
                    .join("xPilot")
                    .join("Resources"),
                xplane_root.join("Custom Data"),
            ];

            let mut allowed = false;
            for csl_root in csl_roots {
                let csl_enabled = csl_root.join("CSL");
                let csl_disabled = csl_root.join("CSL (disabled)");
                if full_path.starts_with(&csl_enabled) || full_path.starts_with(&csl_disabled) {
                    allowed = true;
                    break;
                }
            }

            if !allowed {
                return Err(format!(
                    "Safety check failed: {} is not inside CSL folders",
                    full_path.display()
                ));
            }
        } else {
            let allowed_dir = match addon_type {
                AddonType::Scenery => xplane_root.join("Custom Scenery"),
                AddonType::Aircraft => xplane_root.join("Aircraft"),
                AddonType::Plugins => xplane_root.join("Resources").join("plugins"),
                AddonType::CSLs => unreachable!(),
            };

            if !full_path.starts_with(&allowed_dir) {
                return Err(format!(
                    "Safety check failed: {} is not inside {}",
                    full_path.display(),
                    allowed_dir.display()
                ));
            }
        }

        // Delete the folder/file
        if full_path.exists() {
            if full_path.is_dir() {
                fs::remove_dir_all(&full_path)
                    .map_err(|e| format!("Failed to delete dir: {}", e))?;
            } else {
                fs::remove_file(&full_path).map_err(|e| format!("Failed to delete file: {}", e))?;
            }
        }

        // Special handling for Scenery: remove from scenery_packs.ini
        if matches!(addon_type, AddonType::Scenery) {
            if let Ok(xpm) = XPlaneManager::new(xplane_root) {
                let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
                let _ = sm.load();

                // We use the 'path' as provided to the function (which is relative to Custom Scenery in the INI)
                sm.packs.retain(|p| p.path != path);
                sm.save(None).map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Scans for Laminar Research aircraft that exist in BOTH `Aircraft/Laminar Research/`
    /// AND `Aircraft (Disabled)/Laminar Research/`. This means X-Plane's updater has
    /// reinstalled them after the user previously disabled them.
    /// Returns a list of aircraft names that were automatically re-disabled.
    pub fn suppress_laminar_duplicates(xplane_root: &Path) -> Vec<String> {
        let disabled_laminar = xplane_root
            .join("Aircraft (Disabled)")
            .join("Laminar Research");
        let active_laminar = xplane_root.join("Aircraft").join("Laminar Research");

        let mut suppressed = Vec::new();

        if !disabled_laminar.exists() || !active_laminar.exists() {
            return suppressed;
        }

        // List all subdirectories in the disabled Laminar folder
        let disabled_entries = match fs::read_dir(&disabled_laminar) {
            Ok(entries) => entries,
            Err(_) => return suppressed,
        };

        for entry in disabled_entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };

            // Check if the same aircraft also exists in the active Laminar folder
            let active_copy = active_laminar.join(&name);
            if active_copy.exists() && active_copy.is_dir() {
                // The updater restored this aircraft — remove the active copy
                println!(
                    "[LaminarSuppress] Removing updater-restored copy: {}",
                    active_copy.display()
                );
                if let Err(e) = fs::remove_dir_all(&active_copy) {
                    eprintln!(
                        "[LaminarSuppress] Failed to remove {}: {}",
                        active_copy.display(),
                        e
                    );
                    continue;
                }
                suppressed.push(name);
            }
        }

        // Clean up empty Laminar Research directory in Aircraft/
        if active_laminar.exists() {
            if let Ok(mut entries) = fs::read_dir(&active_laminar) {
                if entries.next().is_none() {
                    let _ = fs::remove_dir(&active_laminar);
                }
            }
        }

        if !suppressed.is_empty() {
            println!(
                "[LaminarSuppress] Auto-suppressed {} aircraft: {:?}",
                suppressed.len(),
                suppressed
            );
        }

        suppressed
    }

    /// Identifies what kind of addon an archive contains based on a list of its file paths.
    pub fn detect_archive_type(file_paths: &[String]) -> ArchiveType {
        let mut has_plugin_binary = false;
        let mut has_lua_scripts = false;
        let mut has_python_scripts = false;

        for path in file_paths {
            let lower = path.to_lowercase();
            // Binary plugin indicators
            if lower.ends_with(".xpl")
                || lower.contains("/64/")
                || lower.contains("/lin_x64/")
                || lower.contains("/win_x64/")
                || lower.contains("/mac_x64/")
            {
                has_plugin_binary = true;
            }
            // Script indicators
            if lower.ends_with(".lua") {
                has_lua_scripts = true;
            }
            if lower.ends_with(".py") {
                has_python_scripts = true;
            }
        }

        if has_plugin_binary {
            ArchiveType::StandardPlugin
        } else if has_lua_scripts {
            ArchiveType::LuaScripts
        } else if has_python_scripts {
            ArchiveType::PythonScripts
        } else {
            ArchiveType::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveType {
    StandardPlugin,
    LuaScripts,
    PythonScripts,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupData {
    pub heuristics: x_adox_bitnet::HeuristicsConfig,
    pub profiles: ProfileCollection,
    pub export_date: String,
    pub version: String,
}

pub struct BackupManager;

impl BackupManager {
    pub fn backup_user_data(
        xplane_root: &Path,
        output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model = BitNetModel::new()?;
        let profile_mgr = ProfileManager::new(xplane_root);
        let profiles = profile_mgr.load()?;

        let backup = BackupData {
            heuristics: model.config.as_ref().clone(),
            profiles,
            export_date: chrono::Local::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let content = serde_json::to_string_pretty(&backup)?;
        fs::write(output_path, content)?;
        Ok(())
    }

    pub fn restore_user_data(
        xplane_root: &Path,
        input_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(input_path)?;

        // 1. Try parsing as full backup file (.xback)
        if let Ok(backup) = serde_json::from_str::<BackupData>(&content) {
            // Restore Heuristics
            let mut model = BitNetModel::new()?;
            model.update_config(backup.heuristics);
            model.save()?;

            // Restore Profiles
            let profile_mgr = ProfileManager::new(xplane_root);
            profile_mgr.save(&backup.profiles)?;

            return Ok(format!("Restored backup from {}", backup.export_date));
        }

        // 2. Try parsing as legacy heuristics.json
        if let Ok(heuristics) = serde_json::from_str::<x_adox_bitnet::HeuristicsConfig>(&content) {
            let mut model = BitNetModel::new()?;
            model.update_config(heuristics);
            model.save()?;
            return Ok("Imported heuristics only (legacy format)".to_string());
        }

        // 3. Try parsing as legacy profiles.json
        if let Ok(profiles) = serde_json::from_str::<ProfileCollection>(&content) {
            let profile_mgr = ProfileManager::new(xplane_root);
            profile_mgr.save(&profiles)?;
            return Ok("Imported profiles only (legacy format)".to_string());
        }

        Err(
            "File format not recognized. Please select a valid backup or legacy config file."
                .into(),
        )
    }
}
