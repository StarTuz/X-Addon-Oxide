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
                    tags: Vec::new(),
                });
            }
        }
    }

    Ok(packs)
}

pub fn write_ini(
    file_path: &Path,
    packs: &[SceneryPack],
    model: Option<&x_adox_bitnet::BitNetModel>,
) -> io::Result<()> {
    let mut file = File::create(file_path)?;

    writeln!(file, "I")?;
    writeln!(file, "1000 Version")?;
    writeln!(file, "SCENERY")?;
    writeln!(file)?;

    let mut last_section = String::new();

    for pack in packs {
        // Determine section header based on matched rule name (dynamic!)
        let current_section = if let Some(m) = model {
            let (_score, rule_name) = m.predict_with_rule_name(
                &pack.name,
                &pack.path,
                &x_adox_bitnet::PredictContext::default(),
            );
            format!("# {}", rule_name)
        } else {
            // Fallback to category-based headers if no model
            match pack.category {
                crate::scenery::SceneryCategory::EarthAirports
                | crate::scenery::SceneryCategory::MarsAirports => "# Airports".to_string(),
                crate::scenery::SceneryCategory::GlobalAirport => "# Global Airports".to_string(),
                crate::scenery::SceneryCategory::Library => "# Libraries".to_string(),
                crate::scenery::SceneryCategory::Overlay => "# Overlays".to_string(),
                crate::scenery::SceneryCategory::Ortho => "# Ortho Scenery".to_string(),
                crate::scenery::SceneryCategory::Mesh => "# Meshes".to_string(),
                _ => "# Other Scenery".to_string(),
            }
        };

        if current_section != last_section {
            writeln!(file)?;
            writeln!(file, "{}", current_section)?;
            last_section = current_section;
        }

        let pack_path_str = if pack.name.starts_with('*') {
            pack.name.clone()
        } else {
            format!("Custom Scenery/{}/", pack.name)
        };

        match pack.status {
            SceneryPackType::Active => {
                writeln!(file, "SCENERY_PACK {}", pack_path_str)?;
            }
            SceneryPackType::Disabled => {
                writeln!(file, "SCENERY_PACK_DISABLED {}", pack_path_str)?;
            }
            SceneryPackType::DuplicateHidden => {
                writeln!(
                    file,
                    "# Disabled duplicate - original is above: SCENERY_PACK_DISABLED {}",
                    pack_path_str
                )?;
            }
        }
    }

    Ok(())
}
