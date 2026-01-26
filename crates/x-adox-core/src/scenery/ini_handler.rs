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
                // Normalize backslashes (Windows) to forward slashes
                let normalized_path = relative_path_str.replace('\\', "/");

                // Remove trailing slash and extra whitespace
                let clean_path = normalized_path
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
                } else if clean_path.starts_with("Custom Scenery") {
                    // Custom Scenery/PackName
                    scenery_root.join(&name)
                } else {
                    // System packs like "Global Scenery/Global Airports"
                    // These are root-relative. scenery_root is "<root>/Custom Scenery"
                    let xplane_root = scenery_root.parent().unwrap_or(scenery_root);
                    xplane_root.join(&pack_path)
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
                &x_adox_bitnet::PredictContext {
                    has_airports: !pack.airports.is_empty(),
                    has_tiles: !pack.tiles.is_empty(),
                    ..Default::default()
                },
            );
            format!("# {}", rule_name)
        } else {
            // Fallback to category-based headers if no model
            match pack.category {
                crate::scenery::SceneryCategory::CustomAirport
                | crate::scenery::SceneryCategory::OrbxAirport => "# Airports".to_string(),
                crate::scenery::SceneryCategory::GlobalAirport => "# Global Airports".to_string(),
                crate::scenery::SceneryCategory::Landmark => "# Landmarks".to_string(),
                crate::scenery::SceneryCategory::RegionalOverlay
                | crate::scenery::SceneryCategory::AirportOverlay
                | crate::scenery::SceneryCategory::LowImpactOverlay
                | crate::scenery::SceneryCategory::RegionalFluff => {
                    "# Regional & Overlays".to_string()
                }
                crate::scenery::SceneryCategory::AutoOrthoOverlay => {
                    "# AutoOrtho Overlays".to_string()
                }
                crate::scenery::SceneryCategory::Library => "# Libraries".to_string(),
                crate::scenery::SceneryCategory::OrthoBase => "# Ortho Scenery".to_string(),
                crate::scenery::SceneryCategory::Mesh
                | crate::scenery::SceneryCategory::SpecificMesh => "# Meshes".to_string(),
                _ => "# Other Scenery".to_string(),
            }
        };

        if current_section != last_section {
            writeln!(file)?;
            writeln!(file, "{}", current_section)?;
            last_section = current_section;
        }

        // Determine the correct relative path for the INI file
        // System packs (Global Scenery, etc.) should stay as-is
        // Custom packs should use "Custom Scenery/<name>/" format
        let pack_path_str = if pack.name.starts_with('*') {
            pack.name.clone()
        } else {
            let path_str = pack.path.to_string_lossy();
            if path_str.contains("Global Scenery") {
                // Preserve system pack paths relative to X-Plane root
                if let Some(idx) = path_str.find("Global Scenery") {
                    format!("{}/", &path_str[idx..])
                } else {
                    format!("Custom Scenery/{}/", pack.name)
                }
            } else {
                // Standard Custom Scenery pack
                format!("Custom Scenery/{}/", pack.name)
            }
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
