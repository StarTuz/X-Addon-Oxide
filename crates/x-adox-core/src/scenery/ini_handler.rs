use crate::scenery::{SceneryCategory, SceneryPack, SceneryPackType};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub fn read_ini(file_path: &Path, scenery_root: &Path) -> io::Result<Vec<SceneryPack>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut packs = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trim_line = line.trim();

        // Skip comments and header
        if trim_line.is_empty()
            || trim_line.starts_with('#')
            || trim_line == "I"
            || trim_line.contains("Version")
            || trim_line == "SCENERY"
        {
            continue;
        }

        if trim_line.starts_with("SCENERY_PACK") {
            let is_disabled = trim_line.starts_with("SCENERY_PACK_DISABLED");
            let status = if is_disabled {
                SceneryPackType::Disabled
            } else {
                SceneryPackType::Active
            };

            let parts: Vec<&str> = trim_line.split_whitespace().collect();
            if parts.len() >= 2 {
                let relative_path_str = parts[1..].join(" ");
                // Remove trailing slash and extra whitespace
                let clean_path = relative_path_str
                    .trim()
                    .trim_end_matches('/')
                    .trim()
                    .to_string();
                let pack_path = PathBuf::from(&clean_path);

                let name = pack_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .trim()
                    .to_string();

                // Resolve full path
                let full_path = if pack_path.is_absolute() {
                    pack_path
                } else if pack_path.starts_with("Custom Scenery") {
                    // Usually "Custom Scenery/PackName/"
                    scenery_root.join(&name)
                } else {
                    scenery_root.join(&pack_path)
                };

                packs.push(SceneryPack {
                    name,
                    path: full_path,
                    status,
                    category: SceneryCategory::Unknown, // Will be classified later
                    airports: Vec::new(),
                    tiles: Vec::new(),
                });
            }
        }
    }

    Ok(packs)
}

pub fn write_ini(file_path: &Path, packs: &[SceneryPack]) -> io::Result<()> {
    use x_adox_bitnet::BitNetModel;
    let model = BitNetModel::new().unwrap_or_default();
    let mut file = File::create(file_path)?;

    writeln!(file, "I")?;
    writeln!(file, "1000 Version")?;
    writeln!(file, "SCENERY")?;
    writeln!(file)?;

    let mut last_section = "";

    for pack in packs {
        // Determine section header based on BitNet score
        let score = model.predict(&pack.name, &pack.path);
        let current_section = match score {
            0..=10 => "# Payware & Custom Airports",
            11..=20 => "# Global Airports",
            21..=29 => "# Orbx Custom Landmarks",
            30..=36 => "# simHeaven X-World",
            37..=40 => "# Overlays & Landmarks",
            41..=42 => "# Orbx TrueEarth Overlays",
            43..=45 => "# Libraries",
            46..=48 => "# Birds",
            49..=50 => "# Orthos & Photoscenery",
            _ => "# Meshes & Terrain",
        };

        if current_section != last_section {
            writeln!(file)?;
            writeln!(file, "{}", current_section)?;
            last_section = current_section;
        }

        let prefix = match pack.status {
            SceneryPackType::Active => "SCENERY_PACK",
            SceneryPackType::Disabled => "SCENERY_PACK_DISABLED",
        };

        if pack.name.starts_with('*') {
            writeln!(file, "{} {}", prefix, pack.name)?;
        } else {
            // Write standard relative path format
            writeln!(file, "{} Custom Scenery/{}/", prefix, pack.name)?;
        }
    }

    Ok(())
}
