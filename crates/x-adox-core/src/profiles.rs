use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    /// Map of pack names to their enabled status
    pub scenery_states: HashMap<String, bool>,
    /// Map of plugin paths (relative to X-Plane root) to their enabled status
    pub plugin_states: HashMap<String, bool>,
    /// Map of aircraft paths (relative to X-Plane root) to their enabled status
    pub aircraft_states: HashMap<String, bool>,
    /// Command-line arguments for launching X-Plane
    #[serde(default)]
    pub launch_args: String,
}

impl Profile {
    pub fn new_default(name: String) -> Self {
        Self {
            name,
            scenery_states: HashMap::new(),
            plugin_states: HashMap::new(),
            aircraft_states: HashMap::new(),
            launch_args: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCollection {
    pub profiles: Vec<Profile>,
    pub active_profile: Option<String>,
}

impl Default for ProfileCollection {
    fn default() -> Self {
        let default_profile = Profile::new_default("Default".to_string());
        Self {
            profiles: vec![default_profile],
            active_profile: Some("Default".to_string()),
        }
    }
}

impl ProfileCollection {
    /// Returns true if this collection has no meaningful user data.
    /// This is used to detect if we should attempt migration from legacy locations.
    pub fn is_empty_or_default(&self) -> bool {
        // Completely empty
        if self.profiles.is_empty() {
            return true;
        }

        // Only has one profile and it has no saved states
        if self.profiles.len() == 1 {
            let p = &self.profiles[0];
            if p.scenery_states.is_empty()
                && p.plugin_states.is_empty()
                && p.aircraft_states.is_empty()
            {
                return true;
            }
        }

        false
    }

    pub fn get_active_profile_mut(&mut self) -> Option<&mut Profile> {
        let active_name = self.active_profile.as_ref()?;
        self.profiles.iter_mut().find(|p| p.name == *active_name)
    }

    pub fn update_active_scenery(&mut self, states: HashMap<String, bool>) {
        if let Some(profile) = self.get_active_profile_mut() {
            profile.scenery_states = states;
        }
    }

    pub fn update_active_plugins(&mut self, states: HashMap<String, bool>) {
        if let Some(profile) = self.get_active_profile_mut() {
            profile.plugin_states = states;
        }
    }

    pub fn update_active_aircraft(&mut self, states: HashMap<String, bool>) {
        if let Some(profile) = self.get_active_profile_mut() {
            profile.aircraft_states = states;
        }
    }

    pub fn update_active_launch_args(&mut self, args: String) {
        if let Some(profile) = self.get_active_profile_mut() {
            profile.launch_args = args;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileManager {
    config_path: PathBuf,
}

impl ProfileManager {
    pub fn new(xplane_root: &Path) -> Self {
        let config_path = crate::get_scoped_config_root(xplane_root).join("profiles.json");
        Self { config_path }
    }

    pub fn load(&self) -> Result<ProfileCollection> {
        // Try loading from scoped path first
        let scoped_collection = if self.config_path.exists() {
            let content =
                fs::read_to_string(&self.config_path).context("Failed to read profiles.json")?;
            Some(serde_json::from_str::<ProfileCollection>(&content).context("Failed to parse profiles.json")?)
        } else {
            None
        };

        // If scoped file has meaningful data, use it directly
        if let Some(ref collection) = scoped_collection {
            if !collection.is_empty_or_default() {
                return Ok(collection.clone());
            }
        }

        // --- Migration Fallback ---
        // Scoped file is missing OR empty/default - check legacy locations for user data
        if let Some(config_root) = crate::get_config_root().parent() {
            let legacy_paths = [
                config_root.join("x-addon-oxide").join("profiles.json"),
                config_root.join("x-adox").join("profiles.json"),
            ];

            for path in &legacy_paths {
                if path.exists() {
                    if let Ok(content) = fs::read_to_string(path) {
                        if let Ok(collection) = serde_json::from_str::<ProfileCollection>(&content) {
                            // Only migrate if legacy has actual data
                            if !collection.is_empty_or_default() {
                                println!("[Migration] Migrating profiles from legacy location {:?}", path);
                                // Auto-save to scoped location so migration only happens once
                                if let Err(e) = self.save(&collection) {
                                    eprintln!("[Migration] Warning: Failed to save migrated profiles: {}", e);
                                }
                                return Ok(collection);
                            }
                        }
                    }
                }
            }
        }

        // Return scoped collection if we had one (even if empty), otherwise default
        Ok(scoped_collection.unwrap_or_default())
    }

    pub fn save(&self, collection: &ProfileCollection) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).context("Failed to create .xad_oxide directory")?;
            }
        }

        let content =
            serde_json::to_string_pretty(collection).context("Failed to serialize profiles")?;

        fs::write(&self.config_path, content).context("Failed to write profiles.json")
    }
}
