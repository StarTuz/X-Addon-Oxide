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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileCollection {
    pub profiles: Vec<Profile>,
    pub active_profile: Option<String>,
}

impl ProfileCollection {
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
    pub fn new(_xplane_root: &Path) -> Self {
        let config_path = crate::get_config_root().join("profiles.json");
        Self { config_path }
    }

    pub fn load(&self) -> Result<ProfileCollection> {
        if !self.config_path.exists() {
            return Ok(ProfileCollection::default());
        }

        let content =
            fs::read_to_string(&self.config_path).context("Failed to read profiles.json")?;

        serde_json::from_str(&content).context("Failed to parse profiles.json")
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
