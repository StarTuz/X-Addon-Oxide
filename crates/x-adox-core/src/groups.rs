// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroupCollection {
    /// Map of pack name -> List of tags
    pub pack_tags: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct GroupManager {
    config_path: PathBuf,
}

impl GroupManager {
    pub fn new(_xplane_root: &Path) -> Self {
        let config_path = crate::get_config_root().join("scenery_groups.json");
        Self { config_path }
    }

    pub fn load(&self) -> Result<GroupCollection> {
        if !self.config_path.exists() {
            return Ok(GroupCollection::default());
        }

        let content =
            fs::read_to_string(&self.config_path).context("Failed to read scenery_groups.json")?;

        serde_json::from_str(&content).context("Failed to parse scenery_groups.json")
    }

    pub fn save(&self, collection: &GroupCollection) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).context("Failed to create .xad_oxide directory")?;
            }
        }

        let content =
            serde_json::to_string_pretty(collection).context("Failed to serialize groups")?;

        fs::write(&self.config_path, content).context("Failed to write scenery_groups.json")
    }
}
