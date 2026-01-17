pub mod classifier;
pub mod ini_handler;
pub mod sorter;

use crate::apt_dat::{Airport, AptDatParser};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SceneryPackType {
    Active,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum SceneryCategory {
    #[default]
    Unknown,
    Group,         // Virtual group pack
    GlobalAirport, // Global Airports (from X-Plane)
    Library,       // L - Contains library.txt
    EarthScenery,  // ES - Earth scenery (ortho, mesh, etc.)
    EarthAirports, // EA - Contains apt.dat
    MarsScenery,   // MS - Mars scenery
    MarsAirports,  // MA - Mars airports
    Overlay,       // Overlay scenery (SimHeaven, etc.)
    Ortho,         // Photorealistic scenery
    Mesh,          // Mesh scenery
}

impl SceneryCategory {
    pub fn short_code(&self) -> &'static str {
        match self {
            SceneryCategory::Unknown => "",
            SceneryCategory::Group => "GRP",
            SceneryCategory::GlobalAirport => "GA",
            SceneryCategory::Library => "L",
            SceneryCategory::EarthScenery => "ES",
            SceneryCategory::EarthAirports => "EA",
            SceneryCategory::MarsScenery => "MS",
            SceneryCategory::MarsAirports => "MA",
            SceneryCategory::Overlay => "OV",
            SceneryCategory::Ortho => "OR",
            SceneryCategory::Mesh => "ME",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneryPack {
    pub name: String,
    pub path: PathBuf,
    pub status: SceneryPackType,
    pub category: SceneryCategory,
    pub airports: Vec<Airport>,
    pub tiles: Vec<(i32, i32)>, // SW corner (lat, lon)
}

#[derive(Error, Debug)]
pub enum SceneryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error on line {0}: {1}")]
    Parse(usize, String),
}

pub struct SceneryManager {
    pub file_path: PathBuf,
    pub packs: Vec<SceneryPack>,
}

impl SceneryManager {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            packs: Vec::new(),
        }
    }

    pub fn load(&mut self) -> Result<(), SceneryError> {
        let custom_scenery_dir = self.file_path.parent().unwrap_or(&self.file_path);
        println!("[SceneryManager] Loading from INI: {:?}", self.file_path);
        println!(
            "[SceneryManager] Custom Scenery Dir: {:?}",
            custom_scenery_dir
        );

        // 1. Read existing INI entries
        let mut packs = match ini_handler::read_ini(&self.file_path, custom_scenery_dir) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("[SceneryManager] INI file not found, starting fresh");
                Vec::new()
            }
            Err(e) => return Err(SceneryError::Io(e)),
        };
        println!("[SceneryManager] Read {} packs from INI", packs.len());

        // De-duplicate INI entries just in case
        let initial_count = packs.len();
        let mut seen = HashSet::new();
        packs.retain(|p| seen.insert(p.path.clone()));
        if packs.len() < initial_count {
            println!(
                "[SceneryManager] Removed {} duplicate INI entries",
                initial_count - packs.len()
            );
        }

        // 2. Scan the directory for new packs not yet in the INI
        let discovered = crate::discovery::DiscoveryManager::scan_scenery(custom_scenery_dir);
        println!(
            "[SceneryManager] Discovered {} folders on disk",
            discovered.len()
        );

        for disc in discovered {
            // Check if this path is already in the packs list
            let already_present = packs.iter().any(|p| p.path == disc.path);

            if !already_present {
                println!("[SceneryManager] Adding NEW discovered pack: {}", disc.name);
                // Prepend new discovery (X-Plane style)
                packs.insert(
                    0,
                    SceneryPack {
                        name: disc.name,
                        path: disc.path,
                        status: SceneryPackType::Active,
                        category: SceneryCategory::Unknown,
                        airports: Vec::new(),
                        tiles: Vec::new(),
                    },
                );
            }
        }

        for pack in &mut packs {
            // Apply heuristic classification
            pack.category = classifier::Classifier::classify_heuristic(&pack.path, &pack.name);

            // Discover details
            pack.airports = discover_airports_in_pack(&pack.path);
            pack.tiles = discover_tiles_in_pack(&pack.path);

            if !pack.tiles.is_empty() || !pack.airports.is_empty() {
                println!(
                    "[SceneryManager] Pack '{}' valid: {} airports, {} tiles",
                    pack.name,
                    pack.airports.len(),
                    pack.tiles.len()
                );
            }
        }

        self.packs = packs;
        Ok(())
    }

    pub fn enable_pack(&mut self, name: &str) {
        if let Some(pack) = self.packs.iter_mut().find(|p| p.name == name) {
            pack.status = SceneryPackType::Active;
        }
    }

    pub fn disable_pack(&mut self, name: &str) {
        if let Some(pack) = self.packs.iter_mut().find(|p| p.name == name) {
            pack.status = SceneryPackType::Disabled;
        }
    }

    pub fn sort(&mut self) {
        sorter::sort_packs(&mut self.packs);
    }

    pub fn save(&self) -> Result<(), SceneryError> {
        ini_handler::write_ini(&self.file_path, &self.packs)?;
        Ok(())
    }
}

/// Recursively find all directories within a pack that look like actual scenery roots
/// (containing 'Earth nav data', 'library.txt', or 'earth.wed.xml').
fn find_pack_roots(pack_path: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // 1. Check the pack path itself first
    if is_scenery_root(pack_path) {
        roots.push(pack_path.to_path_buf());
    }

    // 2. Check one and two levels deeper (common for nested/wrapped packs)
    if let Ok(entries) = std::fs::read_dir(pack_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if is_scenery_root(&path) {
                    roots.push(path.clone());
                } else {
                    // One more level deep (e.g. Pack/Sub/Data)
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.is_dir() && is_scenery_root(&sub_path) {
                                roots.push(sub_path);
                            }
                        }
                    }
                }
            }
        }
    }

    // If no specific roots found but it's a valid directory, at least return the base
    // so heuristic classification still works.
    if roots.is_empty() {
        roots.push(pack_path.to_path_buf());
    }

    roots.sort();
    roots.dedup();
    roots
}

fn is_scenery_root(path: &Path) -> bool {
    let signals = [
        "earth nav data",
        "library.txt",
        "earth.wed.xml",
        "earth.wed.bak.xml",
        "mars nav data",
    ];
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if signals.iter().any(|&s| s == name) {
                return true;
            }
        }
    }
    false
}

fn discover_tiles_in_pack(pack_path: &Path) -> Vec<(i32, i32)> {
    let mut tiles = Vec::new();
    let nav_data_dirs = ["Earth nav data", "Mars nav data"];
    let pack_path_str = pack_path.to_string_lossy().to_lowercase();

    // Clutter filter
    let excluded_keywords = [
        "global scenery",
        "global_airports",
        "resources",
        "default scenery",
        "landmark",
        "mesh",
        "ortho",
        "terrain",
        "elevation",
        "simheaven",
        "x-world",
        "trueearth",
        "true_earth",
        "forest",
        "autogen",
        "opensceneryx",
        "world2xplane",
        "hd_mesh",
        "alpilotx",
    ];

    for keyword in excluded_keywords {
        if pack_path_str.contains(keyword) {
            return tiles;
        }
    }

    // Find real roots (might be nested)
    let roots = find_pack_roots(pack_path);

    for root in roots {
        // Search for nav data folders case-insensitively within each root
        let real_nav_path = if let Ok(entries) = std::fs::read_dir(&root) {
            entries.flatten().find_map(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if nav_data_dirs.iter().any(|&d| d.to_lowercase() == name) {
                    Some(e.path())
                } else {
                    None
                }
            })
        } else {
            None
        };

        if let Some(nav_path) = real_nav_path {
            // Search for folders like +40-090
            if let Ok(entries) = std::fs::read_dir(nav_path) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let folder_path = entry.path();

                        // Scan inside the folder for .dsf files (e.g., +41-088.dsf)
                        if let Ok(file_entries) = std::fs::read_dir(folder_path) {
                            for file_entry in file_entries.flatten() {
                                let file_name =
                                    file_entry.file_name().to_string_lossy().to_string();
                                if file_name.to_lowercase().ends_with(".dsf") {
                                    if let Some(tile) = parse_tile_name(&file_name) {
                                        tiles.push(tile);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    tiles.sort();
    tiles.dedup();

    // Filter massive regional packs (max 100 tiles for the map)
    if tiles.len() > 100 {
        return Vec::new();
    }

    tiles
}

fn parse_tile_name(name: &str) -> Option<(i32, i32)> {
    // Strips .dsf if present
    let name = name.strip_suffix(".dsf").unwrap_or(name);

    // Format: +40-120 or -10+030
    if name.len() < 6 {
        return None;
    }

    let lat_str = &name[0..3];
    let lon_str = &name[3..];

    let lat = lat_str.parse::<i32>().ok()?;
    let lon = lon_str.parse::<i32>().ok()?;

    Some((lat, lon))
}

fn discover_airports_in_pack(pack_path: &Path) -> Vec<Airport> {
    let mut all_airports = Vec::new();
    let roots = find_pack_roots(pack_path);

    for root in roots {
        // 1. Find "Earth nav data" case-insensitively
        let real_nav_path = if let Ok(entries) = std::fs::read_dir(&root) {
            entries.flatten().find_map(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if name == "earth nav data" {
                    Some(e.path())
                } else {
                    None
                }
            })
        } else {
            None
        };

        if let Some(nav_path) = real_nav_path {
            // 2. Find "apt.dat" case-insensitively
            let real_apt_path = if let Ok(entries) = std::fs::read_dir(&nav_path) {
                entries.flatten().find_map(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    if name == "apt.dat" {
                        Some(e.path())
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            if let Some(apt_path) = real_apt_path {
                match AptDatParser::parse_file(&apt_path) {
                    Ok(airports) => all_airports.extend(airports),
                    Err(_) => {}
                }
            }
        }
    }

    all_airports.sort_by(|a, b| a.id.cmp(&b.id));
    all_airports.dedup_by(|a, b| a.id == b.id);
    all_airports
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn test_discover_tiles_dsf() {
        let dir = tempdir().unwrap();
        let pack_path = dir.path();
        let nav_path = pack_path.join("Earth nav data");
        let grid_path = nav_path.join("+30-010");
        std::fs::create_dir_all(&grid_path).unwrap();

        // Faro DSF
        std::fs::write(grid_path.join("+37-008.dsf"), "").unwrap();
        // Another tile in the same grid
        std::fs::write(grid_path.join("+38-009.dsf"), "").unwrap();

        let tiles = discover_tiles_in_pack(pack_path);
        assert_eq!(tiles.len(), 2);
        assert_eq!(tiles[0], (37, -8));
        assert_eq!(tiles[1], (38, -9));
    }

    #[test]
    fn test_parse_scenery_ini() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "I").unwrap();
        writeln!(file, "1000 Version").unwrap();
        writeln!(file, "SCENERY").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "SCENERY_PACK Custom Scenery/simHeaven_X-Europe/").unwrap();
        writeln!(file, "SCENERY_PACK_DISABLED Custom Scenery/Orbx_NorCal/").unwrap();

        let mut manager = SceneryManager::new(file.path().to_path_buf());
        manager.load().expect("Failed to load ini");

        assert_eq!(manager.packs.len(), 2);
        assert_eq!(manager.packs[0].name, "simHeaven_X-Europe");
        assert_eq!(manager.packs[1].name, "Orbx_NorCal");
    }

    #[test]
    fn test_save_scenery_ini() {
        let file = NamedTempFile::new().unwrap();
        let mut manager = SceneryManager::new(file.path().to_path_buf());

        manager.packs.push(SceneryPack {
            name: "TestPack".to_string(),
            path: PathBuf::from("Custom Scenery/TestPack/"),
            status: SceneryPackType::Active,
            category: SceneryCategory::default(),
            airports: Vec::new(),
            tiles: Vec::new(),
        });

        manager.save().expect("Failed to save");

        let mut verify_manager = SceneryManager::new(file.path().to_path_buf());
        verify_manager.load().expect("Failed to reload");

        assert_eq!(verify_manager.packs.len(), 1);
        assert_eq!(verify_manager.packs[0].name, "TestPack");
    }
}
