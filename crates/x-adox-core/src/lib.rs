// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

pub mod apt_dat;
pub mod archive;
pub mod cache;
pub mod discovery;
pub mod flight_gen;
pub mod groups;
pub mod logbook;
pub mod management;
pub mod profiles;
pub mod scenery;
pub mod weather;

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

    let path = if let Some(proj_dirs) = ProjectDirs::from("org", "x-adox", "x-adox") {
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
    calculate_stable_hash(path)
}

/// Legacy hash implementation using DefaultHasher (non-deterministic across restarts/versions)
pub fn calculate_legacy_hash(path: &Path) -> String {
    let mut s = DefaultHasher::new();
    let p = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    p.hash(&mut s);
    let hash = format!("{:016x}", s.finish());
    log::debug!("[Hash-Legacy] Path {:?} -> Hash {}", p, hash);
    hash
}

/// Deterministic FNV-1a hash for cross-platform stability.
pub fn calculate_stable_hash(path: &Path) -> String {
    let mut h: u64 = 0xcbf29ce484222325;

    // Use canonical path for hashing consistency
    let p = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // Normalize: strip trailing separators to ensure /path/to/xp == /path/to/xp/
    let p_str = p.to_string_lossy();
    let trimmed = p_str.trim_end_matches(['/', '\\']);

    for &b in trimmed.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }

    let hash = format!("{:016x}", h);
    log::debug!("[Hash-Stable] Path {:?} -> Hash {}", p, hash);
    hash
}

/// Normalizes an X-Plane installation path by checking against the global registry files.
/// This ensures that aliases (e.g. symlinks, different mount points) that resolve to the same
/// physical installation are treated as the SAME config scope.
pub fn normalize_install_path(path: &Path) -> PathBuf {
    log::debug!("[Normalize] Input: {:?}", path);
    // If the path is not absolute or doesn't exist, we can't do much.
    // Try canonicalizing first.
    let canonical_input = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // Normalize: strip trailing separators
    let p_str = canonical_input.to_string_lossy();
    let trimmed = p_str.trim_end_matches(['/', '\\']);
    let normalized_input = PathBuf::from(trimmed);

    log::debug!("[Normalize] Canonical Normalized: {:?}", normalized_input);

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
            if let Ok(content) = fs::read_to_string(&config_path) {
                for line in content.lines() {
                    let trimmed_line = line.trim();
                    if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
                        continue;
                    }

                    let registry_path = PathBuf::from(trimmed_line);
                    // Match check: Registry path vs normalized input (both should be stripped of trailers)
                    let reg_str = registry_path.to_string_lossy();
                    let reg_trimmed = PathBuf::from(reg_str.trim_end_matches(['/', '\\']));

                    if reg_trimmed == normalized_input {
                        log::debug!(
                            "[Normalize] Direct match found in registry: {:?}",
                            registry_path
                        );
                        return registry_path;
                    }

                    if let Ok(registry_canonical) = registry_path.canonicalize() {
                        let rc_str = registry_canonical.to_string_lossy();
                        let rc_trimmed = PathBuf::from(rc_str.trim_end_matches(['/', '\\']));
                        if rc_trimmed == normalized_input {
                            log::debug!("[Normalize] Canonical match found in registry: {:?} (for input {:?})", registry_path, path);
                            return registry_path;
                        }
                    }
                }
            }
        }
    }

    // Default: return normalized input if no match found
    log::debug!("[Normalize] No registry match found, using normalized path");
    normalized_input
}

pub fn get_scoped_config_root(xplane_root: &Path) -> PathBuf {
    let normalized = normalize_install_path(xplane_root);

    let stable_hash = calculate_stable_hash(&normalized);
    let legacy_hash = calculate_legacy_hash(&normalized);

    let installs_dir = get_config_root().join("installs");
    let stable_path = installs_dir.join(&stable_hash);
    let legacy_path = installs_dir.join(&legacy_hash);

    // MIGRATION LOGIC:
    // If stable doesn't exist but legacy does, move/rename legacy to stable.
    if !stable_path.exists() && legacy_path.exists() && stable_hash != legacy_hash {
        log::info!(
            "[Migration] Moving legacy config folder {} to stable folder {}",
            legacy_hash,
            stable_hash
        );
        if let Err(e) = move_dir_all(&legacy_path, &stable_path) {
            log::error!("[Migration] Failed to migrate legacy folder: {}", e);
        }
    } else if stable_path.exists() && legacy_path.exists() && stable_hash != legacy_hash {
        // Both exist - check if we should migrate profiles from legacy to stable.
        // This handles the case where dd58f05 created an empty stable folder before
        // migration logic existed, leaving user's profiles stranded in legacy folder.
        let stable_profiles = stable_path.join("profiles.json");
        let legacy_profiles = legacy_path.join("profiles.json");

        if legacy_profiles.exists() {
            let should_copy_profiles = if stable_profiles.exists() {
                // Check if stable profiles is essentially empty/default
                if let Ok(content) = fs::read_to_string(&stable_profiles) {
                    if let Ok(collection) =
                        serde_json::from_str::<crate::profiles::ProfileCollection>(&content)
                    {
                        // Use the robust helper from ProfileCollection
                        collection.is_empty_or_default()
                    } else {
                        log::warn!(
                            "[Migration] Failed to parse stable profiles.json, skipping copy"
                        );
                        false // Can't parse, don't overwrite to be safe
                    }
                } else {
                    true // Can't read stable file, safe to copy over it
                }
            } else {
                true // Stable profiles doesn't exist, safe to copy
            };

            if should_copy_profiles {
                log::info!(
                    "[Migration] Copying profiles.json from legacy ({}) to stable ({})",
                    legacy_hash,
                    stable_hash
                );
                if let Err(e) = fs::copy(&legacy_profiles, &stable_profiles) {
                    log::error!("[Migration] Failed to copy profiles.json: {}", e);
                }
            }
        }

        // Also migrate heuristics.json if stable doesn't have one or is corrupt
        let stable_heuristics = stable_path.join("heuristics.json");
        let legacy_heuristics = legacy_path.join("heuristics.json");

        if legacy_heuristics.exists() {
            let should_copy = if stable_heuristics.exists() {
                // Verify stable heuristics is parseable; if corrupt, back it up and copy legacy
                match fs::read_to_string(&stable_heuristics) {
                    Ok(content) => {
                        if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                            log::warn!("[Migration] Stable heuristics.json is corrupt, backing up and replacing with legacy");
                            let _ = fs::rename(
                                &stable_heuristics,
                                stable_path.join("heuristics.json.bak"),
                            );
                            true
                        } else {
                            false // Stable is valid, don't overwrite
                        }
                    }
                    Err(_) => true, // Can't read stable file, safe to copy over
                }
            } else {
                true // Stable doesn't exist, safe to copy
            };

            if should_copy {
                log::info!(
                    "[Migration] Copying heuristics.json from legacy ({}) to stable ({})",
                    legacy_hash,
                    stable_hash
                );
                if let Err(e) = fs::copy(&legacy_heuristics, &stable_heuristics) {
                    log::error!("[Migration] Failed to copy heuristics.json: {}", e);
                }
            }
        }
    }

    if !stable_path.exists() {
        let _ = fs::create_dir_all(&stable_path);
    }

    log::debug!(
        "[Config] Scoped config root for {:?} is {:?}",
        xplane_root,
        stable_path
    );
    stable_path
}

fn move_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    log::debug!("[Migration] Attempting move from {:?} to {:?}", from, to);
    if let Err(e) = fs::rename(from, to) {
        let is_cross_device = e.raw_os_error() == Some(18) // EXDEV on Linux/macOS
            || e.raw_os_error() == Some(17) // ERROR_NOT_SAME_DEVICE on Windows
            || e.kind() == std::io::ErrorKind::CrossesDevices;
        if is_cross_device {
            log::info!("[Migration] Cross-device move detected, falling back to copy+remove");
            copy_dir_all(from, to)?;
            fs::remove_dir_all(from)?;
            Ok(())
        } else {
            Err(e)
        }
    } else {
        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(&entry.path(), &dst.join(entry.file_name()))?;
        }
    }
    Ok(())
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum LogIssueKind {
    MissingResource,
    BadLightName,
    ObjectReadFailed,
    DsfRoadNetwork,
}

impl std::fmt::Display for LogIssueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogIssueKind::MissingResource => write!(f, "Missing Resource"),
            LogIssueKind::BadLightName => write!(f, "Bad Light Name"),
            LogIssueKind::ObjectReadFailed => write!(f, "Object Read Failed"),
            LogIssueKind::DsfRoadNetwork => write!(f, "DSF Road Network"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogIssue {
    pub kind: LogIssueKind,
    pub primary: String,
    pub secondary: String,
    pub scenery_pack: Option<String>,
}

/// Extracts the scenery pack folder name from a path like `Custom Scenery/PackName/...`.
/// Falls back to the first path segment for resource-style paths.
fn extract_scenery_pack(path: &str) -> Option<String> {
    if let Some(rest) = path.strip_prefix("Custom Scenery/") {
        rest.split('/')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    } else {
        path.split('/')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }
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

        let re_missing = Regex::new(
            r"E/SCN: Failed to find resource '([^']+)', referenced from (?:scenery package|file) '([^']*)'",
        ).unwrap();
        let re_bad_light =
            Regex::new(r"E/OBJ: ERROR: object (.+) has a bad light name: (\S+)").unwrap();
        let re_obj_read =
            Regex::new(r"E/OBJ: OBJ read failed: the file path (.+) could not be opened").unwrap();
        let re_dsf_road =
            Regex::new(r"E/DSF: The DSF (.+) has a number of problems with its road network: (.+)")
                .unwrap();

        let mut issues = Vec::new();
        let mut seen = HashSet::new();

        for line in content.lines() {
            if let Some(cap) = re_missing.captures(line) {
                let primary = cap[1].to_string();
                let secondary = cap[2].to_string();
                let key = format!("0:{}:{}", primary, secondary);
                if seen.insert(key) {
                    let scenery_pack = extract_scenery_pack(&primary);
                    issues.push(LogIssue {
                        kind: LogIssueKind::MissingResource,
                        primary,
                        secondary,
                        scenery_pack,
                    });
                }
                continue;
            }
            if let Some(cap) = re_bad_light.captures(line) {
                let primary = cap[1].trim().to_string();
                let secondary = cap[2].to_string();
                let key = format!("1:{}:{}", primary, secondary);
                if seen.insert(key) {
                    let scenery_pack = extract_scenery_pack(&primary);
                    issues.push(LogIssue {
                        kind: LogIssueKind::BadLightName,
                        primary,
                        secondary,
                        scenery_pack,
                    });
                }
                continue;
            }
            if let Some(cap) = re_obj_read.captures(line) {
                let primary = cap[1].trim().to_string();
                let key = format!("2:{}", primary);
                if seen.insert(key) {
                    let scenery_pack = extract_scenery_pack(&primary);
                    issues.push(LogIssue {
                        kind: LogIssueKind::ObjectReadFailed,
                        primary,
                        secondary: String::new(),
                        scenery_pack,
                    });
                }
                continue;
            }
            if let Some(cap) = re_dsf_road.captures(line) {
                let primary = cap[1].trim().to_string();
                let secondary = cap[2].trim().to_string();
                let key = format!("3:{}:{}", primary, secondary);
                if seen.insert(key) {
                    let scenery_pack = extract_scenery_pack(&primary);
                    issues.push(LogIssue {
                        kind: LogIssueKind::DsfRoadNetwork,
                        primary,
                        secondary,
                        scenery_pack,
                    });
                }
                continue;
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
                           0:00:00.000 E/SCN: Failed to find resource 'madagascar_lib/cars/landrover.obj', referenced from scenery package 'Custom Scenery/CYSJ/'\n\
                           0:00:00.000 E/SCN: Failed to find resource 'opensceneryx/objects/furniture/bench.obj', referenced from file 'Custom Scenery/LFPG/'\n\
                           0:00:00.000 E/SCN: Failed to find resource '', referenced from file 'Custom Scenery/AEP/Polygons/Pavement/'"; // empty resource skipped

        fs::write(root.join("Log.txt"), log_content).unwrap();

        let xpm = XPlaneManager::new(root).unwrap();
        let issues = xpm.check_log().unwrap();

        assert_eq!(issues.len(), 3);
        assert_eq!(issues[0].kind, LogIssueKind::MissingResource);
        assert_eq!(issues[0].primary, "madagascar_lib/cars/landrover.obj");
        assert_eq!(issues[0].secondary, "Custom Scenery/CYSJ/");
        assert_eq!(issues[0].scenery_pack, Some("madagascar_lib".to_string()));

        assert_eq!(issues[1].primary, "BS2001/trees/pine.obj");
        assert_eq!(issues[1].secondary, "Custom Scenery/Airport_A/");
        assert_eq!(issues[1].scenery_pack, Some("BS2001".to_string()));

        // X-Plane 12 format: "referenced from file" instead of "referenced from scenery package"
        assert_eq!(
            issues[2].primary,
            "opensceneryx/objects/furniture/bench.obj"
        );
        assert_eq!(issues[2].secondary, "Custom Scenery/LFPG/");
        assert_eq!(issues[2].scenery_pack, Some("opensceneryx".to_string()));
    }

    #[test]
    fn test_check_log_bad_light_name() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Custom Scenery")).unwrap();
        fs::create_dir_all(root.join("Resources")).unwrap();

        let log_content = "\
0:00:01.000 E/OBJ: ERROR: object Custom Scenery/LFOB Vehicles/objects/car.obj has a bad light name: fakeLightXYZ\n\
0:00:01.000 E/OBJ: ERROR: object Custom Scenery/LFOB Vehicles/objects/car.obj has a bad light name: fakeLightXYZ\n\
0:00:02.000 E/OBJ: ERROR: object Custom Scenery/KJFK/objects/tower.obj has a bad light name: badGlow\n";

        fs::write(root.join("Log.txt"), log_content).unwrap();

        let xpm = XPlaneManager::new(root).unwrap();
        let issues = xpm.check_log().unwrap();

        assert_eq!(issues.len(), 2); // dedup removes duplicate
        assert_eq!(issues[0].kind, LogIssueKind::BadLightName);
        assert_eq!(
            issues[0].primary,
            "Custom Scenery/LFOB Vehicles/objects/car.obj"
        );
        assert_eq!(issues[0].secondary, "fakeLightXYZ");
        assert_eq!(issues[0].scenery_pack, Some("LFOB Vehicles".to_string()));

        assert_eq!(issues[1].kind, LogIssueKind::BadLightName);
        assert_eq!(issues[1].primary, "Custom Scenery/KJFK/objects/tower.obj");
        assert_eq!(issues[1].scenery_pack, Some("KJFK".to_string()));
    }

    #[test]
    fn test_check_log_object_read_failed() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Custom Scenery")).unwrap();
        fs::create_dir_all(root.join("Resources")).unwrap();

        let log_content = "\
0:00:03.000 E/OBJ: OBJ read failed: the file path Custom Scenery/EGLL/objects/missing.obj could not be opened\n";

        fs::write(root.join("Log.txt"), log_content).unwrap();

        let xpm = XPlaneManager::new(root).unwrap();
        let issues = xpm.check_log().unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, LogIssueKind::ObjectReadFailed);
        assert_eq!(issues[0].primary, "Custom Scenery/EGLL/objects/missing.obj");
        assert_eq!(issues[0].secondary, "");
        assert_eq!(issues[0].scenery_pack, Some("EGLL".to_string()));
    }

    #[test]
    fn test_check_log_dsf_road_network() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Custom Scenery")).unwrap();
        fs::create_dir_all(root.join("Resources")).unwrap();

        let log_content = "\
0:00:04.000 E/DSF: The DSF Custom Scenery/SimHeaven_X-World/Earth nav data/+40-080.dsf has a number of problems with its road network: 3 junctions and 12 segments were removed.\n";

        fs::write(root.join("Log.txt"), log_content).unwrap();

        let xpm = XPlaneManager::new(root).unwrap();
        let issues = xpm.check_log().unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, LogIssueKind::DsfRoadNetwork);
        assert_eq!(
            issues[0].primary,
            "Custom Scenery/SimHeaven_X-World/Earth nav data/+40-080.dsf"
        );
        assert_eq!(
            issues[0].secondary,
            "3 junctions and 12 segments were removed."
        );
        assert_eq!(
            issues[0].scenery_pack,
            Some("SimHeaven_X-World".to_string())
        );
    }

    #[test]
    fn test_check_log_mixed_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Custom Scenery")).unwrap();
        fs::create_dir_all(root.join("Resources")).unwrap();

        let log_content = "\
0:00:00.000 E/SCN: Failed to find resource 'opensceneryx/objects/bench.obj', referenced from scenery package 'Custom Scenery/LFPG/'\n\
0:00:01.000 E/OBJ: ERROR: object Custom Scenery/KJFK/objects/tower.obj has a bad light name: badGlow\n\
0:00:02.000 E/OBJ: OBJ read failed: the file path Custom Scenery/EGLL/objects/missing.obj could not be opened\n\
0:00:03.000 E/DSF: The DSF Custom Scenery/SimHeaven/Earth nav data/+40-080.dsf has a number of problems with its road network: 5 issues found\n";

        fs::write(root.join("Log.txt"), log_content).unwrap();

        let xpm = XPlaneManager::new(root).unwrap();
        let issues = xpm.check_log().unwrap();

        assert_eq!(issues.len(), 4);
        assert_eq!(issues[0].kind, LogIssueKind::MissingResource);
        assert_eq!(issues[1].kind, LogIssueKind::BadLightName);
        assert_eq!(issues[2].kind, LogIssueKind::ObjectReadFailed);
        assert_eq!(issues[3].kind, LogIssueKind::DsfRoadNetwork);
    }
}
