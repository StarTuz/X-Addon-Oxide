pub mod apt_dat;
pub mod cache;
pub mod discovery;
pub mod management;
pub mod scenery;

use regex::Regex;
use std::collections::HashSet;
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogIssue {
    pub resource_path: String,
    pub package_path: String,
    pub potential_library: Option<String>,
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

    pub fn get_log_path(&self) -> PathBuf {
        self.root.join("Log.txt")
    }

    pub fn check_log(&self) -> Result<Vec<LogIssue>, XamError> {
        let log_path = self.get_log_path();
        if !log_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(log_path)?;
        let re = Regex::new(
            r"E/SCN: Failed to find resource '([^']*)', referenced from scenery package '([^']*)'",
        )
        .unwrap();

        let mut issues = Vec::new();
        let mut seen = HashSet::new();

        for cap in re.captures_iter(&content) {
            let resource_path = cap[1].to_string();
            let package_path = cap[2].to_string();

            let key = format!("{}:{}", resource_path, package_path);
            if seen.insert(key) {
                let potential_library = resource_path.split('/').next().map(|s| s.to_string());
                issues.push(LogIssue {
                    resource_path,
                    package_path,
                    potential_library,
                });
            }
        }

        Ok(issues)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_check_log() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create mock X-Plane structure
        fs::create_dir_all(root.join("Custom Scenery")).unwrap();
        fs::create_dir_all(root.join("Resources")).unwrap();

        let log_content = "0:00:00.000 E/SCN: Failed to find resource 'madagascar_lib/cars/landrover.obj', referenced from scenery package 'Custom Scenery/CYSJ/'\n\
                           0:00:00.000 E/SCN: Failed to find resource 'BS2001/trees/pine.obj', referenced from scenery package 'Custom Scenery/Airport_A/'\n\
                           0:00:00.000 E/SCN: Failed to find resource 'madagascar_lib/cars/landrover.obj', referenced from scenery package 'Custom Scenery/CYSJ/'"; // Duplicate

        fs::write(root.join("Log.txt"), log_content).unwrap();

        let xpm = XPlaneManager::new(root).unwrap();
        let issues = xpm.check_log().unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].resource_path, "madagascar_lib/cars/landrover.obj");
        assert_eq!(issues[0].package_path, "Custom Scenery/CYSJ/");
        assert_eq!(
            issues[0].potential_library,
            Some("madagascar_lib".to_string())
        );

        assert_eq!(issues[1].resource_path, "BS2001/trees/pine.obj");
        assert_eq!(issues[1].package_path, "Custom Scenery/Airport_A/");
        assert_eq!(issues[1].potential_library, Some("BS2001".to_string()));
    }
}
