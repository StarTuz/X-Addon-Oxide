pub mod apt_dat;
pub mod cache;
pub mod discovery;
pub mod groups;
pub mod logbook;
pub mod management;
pub mod profiles;
pub mod scenery;

use directories::ProjectDirs;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{env, fs};
use thiserror::Error;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn get_config_root() -> PathBuf {
    // Allow overriding via environment variable for tests
    if let Ok(env_path) = env::var("X_ADOX_CONFIG_DIR") {
        return PathBuf::from(env_path);
    }

    let path = if let Some(proj_dirs) = ProjectDirs::from("org", "x-adox", "X-Addon-Oxide") {
        proj_dirs.config_dir().to_path_buf()
    } else {
        // Fallback to a local hidden folder if ProjectDirs fails
        PathBuf::from(".xad_oxide")
    };

    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

pub fn calculate_path_hash(path: &Path) -> String {
    let mut s = DefaultHasher::new();
    // Canonicalize to ensure same path always has same hash (handle trailing slashes/symlinks)
    let p = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    p.hash(&mut s);
    format!("{:016x}", s.finish())
}

/// Normalizes an X-Plane installation path by checking against the global registry files.
/// This ensures that aliases (e.g. symlinks, different mount points) that resolve to the same
/// physical installation are treated as the SAME config scope.
pub fn normalize_install_path(path: &Path) -> PathBuf {
    // If the path is not absolute or doesn't exist, we can't do much.
    // Try canonicalizing first.
    let canonical_input = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // Registry files to check
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
            // macOS typically uses Application Support for this, but could be in Preferences
            candidate_dirs.push(PathBuf::from(&home).join("Library/Application Support/X-Plane"));
            candidate_dirs.push(PathBuf::from(&home).join("Library/Preferences"));
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
            if let Ok(content) = fs::read_to_string(config_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }

                    let registry_path = PathBuf::from(trimmed);
                    // Check if this registry entry matches our input
                    // We check both raw and canonical equality
                    if registry_path == *path || registry_path == canonical_input {
                        return registry_path;
                    }

                    if let Ok(registry_canonical) = registry_path.canonicalize() {
                        if registry_canonical == canonical_input {
                            // MATCH FOUND! Return the registry version of the path.
                            // This guarantees the hash will match what is stored in the registry.
                            return registry_path;
                        }
                    }
                }
            }
        }
    }

    // Default: return canonical input if no match found
    canonical_input
}

pub fn get_scoped_config_root(xplane_root: &Path) -> PathBuf {
    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    // CRITICAL ARCHITECTURE REQUIREMENT: PATH NORMALIZATION
    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    // We MUST normalize the installation path using the X-Plane Registry (install_xx.txt).
    // Failing to do this results in DATA LOSS (Profile Reversal) because different aliases
    // (e.g., /home/user/XP12 vs /mnt/data/XP12) will produce different hashes, treating
    // the same physical installation as two separate buckets.
    //
    // DO NOT REMOVE `normalize_install_path` CALL UNDER ANY CIRCUMSTANCES.
    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    let normalized = normalize_install_path(xplane_root);

    let hash = calculate_path_hash(&normalized);
    let path = get_config_root().join("installs").join(hash);
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

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

    pub fn get_default_apt_dat_path(&self) -> PathBuf {
        self.root
            .join("Global Scenery")
            .join("Global Airports")
            .join("Earth nav data")
            .join("apt.dat")
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

    /// Returns all valid X-Plane installations found in the system config files.
    /// Parses `x-plane_install_12.txt` and `x-plane_install_11.txt` completely.
    pub fn find_all_xplane_roots() -> Vec<PathBuf> {
        let filenames = ["x-plane_install_12.txt", "x-plane_install_11.txt"];
        let mut candidate_dirs = Vec::new();
        let mut results = Vec::new();

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
                if let Ok(content) = fs::read_to_string(&config_path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty()
                            || trimmed.starts_with('#')
                            || trimmed.starts_with(';')
                        {
                            continue;
                        }
                        let path = PathBuf::from(trimmed);
                        if path.exists() && path.join("Resources").exists() {
                            // Normalize path to avoid duplicates (e.g. trailing slashes)
                            if let Ok(canonical) = path.canonicalize() {
                                if !results.contains(&canonical) {
                                    results.push(canonical);
                                }
                            } else if !results.contains(&path) {
                                // Fallback if canonicalization fails for some reason
                                results.push(path);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Returns the path to the X-Plane executable for the current platform.
    pub fn get_executable_path(&self) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            // Try x86_64 first, then arm64
            let exe = self.root.join("X-Plane-x86_64");
            if exe.exists() {
                return Some(exe);
            }
            let exe = self.root.join("X-Plane-arm64");
            if exe.exists() {
                return Some(exe);
            }
        }

        #[cfg(target_os = "windows")]
        {
            let exe = self.root.join("X-Plane.exe");
            if exe.exists() {
                return Some(exe);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let exe = self.root.join("X-Plane.app/Contents/MacOS/X-Plane");
            if exe.exists() {
                return Some(exe);
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
