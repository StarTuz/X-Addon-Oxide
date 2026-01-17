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

        manager.save()?;
        Ok(())
    }
}
