use crate::scenery::SceneryManager;
use std::fs;
use std::path::{Path, PathBuf};

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
}
