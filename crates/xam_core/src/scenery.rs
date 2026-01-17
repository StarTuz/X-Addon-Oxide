use crate::apt_dat::{Airport, AptDatParser};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
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
    Library,       // L - Contains library.txt
    EarthScenery,  // ES - Earth scenery (ortho, mesh, etc.)
    EarthAirports, // EA - Contains apt.dat
    MarsScenery,   // MS - Mars scenery
    MarsAirports,  // MA - Mars airports
}

impl SceneryCategory {
    pub fn short_code(&self) -> &'static str {
        match self {
            SceneryCategory::Unknown => "",
            SceneryCategory::Library => "L",
            SceneryCategory::EarthScenery => "ES",
            SceneryCategory::EarthAirports => "EA",
            SceneryCategory::MarsScenery => "MS",
            SceneryCategory::MarsAirports => "MA",
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
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);

        // Get the Custom Scenery base directory
        let custom_scenery_dir = self.file_path.parent().unwrap_or(&self.file_path);

        self.packs.clear();

        for (_index, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('I') || line.starts_with('#') {
                continue; // Skip header (I 1000 Version) or comments
            }

            if let Some(path_str) = line.strip_prefix("SCENERY_PACK ") {
                let full_path = resolve_scenery_path(custom_scenery_dir, path_str);
                self.packs.push(SceneryPack {
                    name: extract_name(path_str),
                    path: PathBuf::from(path_str),
                    status: SceneryPackType::Active,
                    category: classify_scenery(&full_path),
                    airports: discover_airports_in_pack(&full_path),
                    tiles: discover_tiles_in_pack(&full_path),
                });
            } else if let Some(path_str) = line.strip_prefix("SCENERY_PACK_DISABLED ") {
                let full_path = resolve_scenery_path(custom_scenery_dir, path_str);
                self.packs.push(SceneryPack {
                    name: extract_name(path_str),
                    path: PathBuf::from(path_str),
                    status: SceneryPackType::Disabled,
                    category: classify_scenery(&full_path),
                    airports: discover_airports_in_pack(&full_path),
                    tiles: discover_tiles_in_pack(&full_path),
                });
            }
        }

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

    pub fn save(&self) -> Result<(), SceneryError> {
        let mut file = File::create(&self.file_path)?;
        writeln!(file, "I")?;
        writeln!(file, "1000 Version")?;
        writeln!(file, "SCENERY")?;
        writeln!(file)?;

        for pack in &self.packs {
            let prefix = match pack.status {
                SceneryPackType::Active => "SCENERY_PACK",
                SceneryPackType::Disabled => "SCENERY_PACK_DISABLED",
            };
            writeln!(file, "{} {}", prefix, pack.path.display())?;
        }

        Ok(())
    }
}

fn extract_name(path_str: &str) -> String {
    let trimmed = path_str.trim_end_matches(&['/', '\\'][..]);
    PathBuf::from(trimmed)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path_str.to_string())
}

fn resolve_scenery_path(custom_scenery_dir: &Path, path_str: &str) -> PathBuf {
    let path = PathBuf::from(path_str.trim_end_matches(&['/', '\\'][..]));
    if path.is_relative() {
        if let Some(xplane_root) = custom_scenery_dir.parent() {
            return xplane_root.join(&path);
        }
    }
    path
}

fn classify_scenery(scenery_path: &Path) -> SceneryCategory {
    if scenery_path.join("library.txt").exists() {
        return SceneryCategory::Library;
    }

    let earth_nav = scenery_path.join("Earth nav data");
    if earth_nav.join("apt.dat").exists() {
        let name_lower = scenery_path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if name_lower.contains("mars") {
            return SceneryCategory::MarsAirports;
        }
        return SceneryCategory::EarthAirports;
    }

    if earth_nav.exists() {
        let name_lower = scenery_path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if name_lower.contains("mars") {
            return SceneryCategory::MarsScenery;
        }
        return SceneryCategory::EarthScenery;
    }

    SceneryCategory::Unknown
}

fn discover_tiles_in_pack(pack_path: &Path) -> Vec<(i32, i32)> {
    let mut tiles = Vec::new();
    let nav_data_dirs = ["Earth nav data", "Mars nav data"];
    let pack_path_str = pack_path.to_string_lossy().to_lowercase();

    // Comprehensive list of keywords to filter out global/regional scenery
    // These packs cover large areas and would clutter the map
    let excluded_keywords = [
        // Default X-Plane scenery
        "global scenery",
        "global_airports",
        "resources",
        "default scenery",
        "landmarks",
        // Mesh and terrain
        "mesh",
        "ortho",
        "terrain",
        "elevation",
        // simHeaven X-World regional packs
        "simheaven",
        "x-world",
        // Orbx TrueEarth regional packs
        "trueearth",
        "true_earth",
        // Common regional scenery pack components
        "forests",
        "global_forests",
        "-vfr",
        "-details",
        "-footprints",
        "-extras",
        "-network",
        "-regions",
        "-scenery", // Like "America-6-scenery"
        // Other global enhancement packs
        "autogen",
        "opensceneryx",
        "world2xplane",
        "hd_mesh",
        "uw_scenery",
        "alpilotx",
        // Wildlife/environment packs
        "birds",
    ];

    for keyword in excluded_keywords {
        if pack_path_str.contains(keyword) {
            return tiles;
        }
    }

    for dir_name in nav_data_dirs {
        let nav_path = pack_path.join(dir_name);
        if !nav_path.exists() {
            continue;
        }

        // Search for folders like +40-090
        if let Ok(entries) = std::fs::read_dir(nav_path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let folder_path = entry.path();

                    // Scan inside the folder for .dsf files (e.g., +41-088.dsf)
                    if let Ok(file_entries) = std::fs::read_dir(folder_path) {
                        for file_entry in file_entries.flatten() {
                            let file_name = file_entry.file_name().to_string_lossy().to_string();
                            if file_name.ends_with(".dsf") {
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

    tiles.sort();
    tiles.dedup();

    // Airport scenery typically has at most 1-4 tiles (local area).
    // Regional packs have 20+ tiles. Filter them out to keep the map clean.
    if tiles.len() > 20 {
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
    let apt_dat_path = pack_path.join("Earth nav data").join("apt.dat");
    if apt_dat_path.exists() {
        match AptDatParser::parse_file(&apt_dat_path) {
            Ok(airports) => airports,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
