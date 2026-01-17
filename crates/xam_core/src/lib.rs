pub mod apt_dat;
pub mod discovery;
pub mod management;
pub mod scenery;

use std::path::{Path, PathBuf};
use std::{env, fs};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum XamError {
    #[error("X-Plane root directory not found")]
    RootNotFound,
    #[error("Invalid X-Plane directory: {0}")]
    InvalidRoot(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct XPlaneManager {
    pub root: PathBuf,
}

impl XPlaneManager {
    /// Tries to create a new manager from a given path.
    /// Validates that the path looks like an X-Plane installation.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, XamError> {
        let root = path.as_ref().to_path_buf();

        if !root.exists() {
            return Err(XamError::RootNotFound);
        }

        // minimal validation: check for "Resources" and "Custom Scenery"
        if !root.join("Resources").exists() || !root.join("Custom Scenery").exists() {
            return Err(XamError::InvalidRoot(
                "Missing Resources or Custom Scenery folder".to_string(),
            ));
        }

        Ok(Self { root })
    }

    pub fn get_scenery_packs_path(&self) -> PathBuf {
        self.root.join("Custom Scenery").join("scenery_packs.ini")
    }

    /// Attempts to find the X-Plane root directory automatically.
    /// Checks standard locations for `x-plane_install_12.txt` or `x-plane_install_11.txt`.
    pub fn try_find_root() -> Option<PathBuf> {
        let filenames = ["x-plane_install_12.txt", "x-plane_install_11.txt"];
        let mut candidate_dirs = Vec::new();

        #[cfg(target_os = "linux")]
        {
            if let Ok(home) = env::var("HOME") {
                candidate_dirs.push(PathBuf::from(home).join(".x-plane"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = env::var("HOME") {
                candidate_dirs.push(PathBuf::from(&home).join("Library/Preferences"));
                candidate_dirs.push(PathBuf::from(&home).join(".x-plane"));
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
                candidate_dirs.push(PathBuf::from(local_appdata));
            }
        }

        for dir in candidate_dirs {
            for filename in &filenames {
                let config_path = dir.join(filename);
                if config_path.exists() {
                    if let Ok(content) = fs::read_to_string(config_path) {
                        for line in content.lines() {
                            let path = PathBuf::from(line.trim());
                            if path.exists() && path.join("Resources").exists() {
                                return Some(path);
                            }
                        }
                    }
                }
            }
        }

        None
    }
}
