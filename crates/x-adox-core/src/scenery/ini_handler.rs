// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

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

        if line.starts_with("SCENERY_PACK") {
            let (is_disabled, tag_len) = if line.starts_with("SCENERY_PACK_DISABLED ") {
                (true, "SCENERY_PACK_DISABLED ".len())
            } else if line.starts_with("SCENERY_PACK ") {
                (false, "SCENERY_PACK ".len())
            } else {
                continue; // Malformed or comment
            };

            let status = if is_disabled {
                SceneryPackType::Disabled
            } else {
                SceneryPackType::Active
            };

            // Grab the rest of the line LITERALLY
            let raw_path_str = &line[tag_len..];
            if raw_path_str.is_empty() {
                continue;
            }

            // Normalize backslashes (Windows) ONLY for internal path resolution
            let normalized_path = raw_path_str.replace('\\', "/");

            // Strip trailing slash for the internal 'name' calculation
            let clean_path = if normalized_path.ends_with('/') {
                &normalized_path[..normalized_path.len() - 1]
            } else {
                &normalized_path
            };

            let pack_path = PathBuf::from(clean_path);

            let name = pack_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Resolve full path
            let full_path = if pack_path.is_absolute() {
                pack_path
            } else if clean_path.starts_with("Custom Scenery/") {
                let sub_path = &clean_path["Custom Scenery/".len()..];
                scenery_root.join(sub_path)
            } else {
                let xplane_root = scenery_root.parent().unwrap_or(scenery_root);
                xplane_root.join(pack_path)
            };

            packs.push(SceneryPack {
                name,
                path: full_path,
                raw_path: Some(raw_path_str.to_string()),
                status,
                category: SceneryCategory::Unknown,
                airports: Vec::new(),
                tiles: Vec::new(),
                tags: Vec::new(),
                descriptor: crate::scenery::SceneryDescriptor::default(),
                region: None,
            });
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
        // Determine section header from the matched rule name, then normalize it
        // through the canonical section mapping used by sorter.rs.
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
            let canonical = x_adox_bitnet::canonical_section_name(&rule_name);
            format!("# {}", canonical)
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
        let pack_path_str = if let Some(raw) = &pack.raw_path {
            // Priority 1: Use the literal raw path from the original file
            Some(raw.clone())
        } else if pack.name.starts_with('*') {
            Some(pack.name.clone())
        } else {
            let path_str = pack.path.to_string_lossy();
            if path_str.contains("/Global Scenery/") || path_str.contains("\\Global Scenery\\") {
                if pack.name == "Global Airports"
                    || path_str.ends_with("Global Airports")
                    || path_str.ends_with("Global Airports/")
                    || path_str.ends_with("Global Airports\\")
                {
                    Some("*GLOBAL_AIRPORTS*".to_string())
                } else {
                    // Skip other Global Scenery items (system internal stuff)
                    None
                }
            } else {
                // FALLBACK: Calculate path (for new items)
                let xplane_root = file_path.parent().and_then(|p| p.parent());

                let mut is_relative = false;
                let mut final_path = pack.path.to_string_lossy().to_string();

                if let Some(root) = xplane_root {
                    if let Ok(rel) = pack.path.strip_prefix(root) {
                        // It's inside the X-Plane root.
                        if rel.starts_with("Custom Scenery") {
                            // Standard Custom Scenery relative format
                            let rel_str = rel.to_string_lossy().replace('\\', "/");
                            final_path = rel_str;
                            is_relative = true;
                        }
                    }
                }

                if !is_relative {
                    // Normalize for INI
                    final_path = final_path.replace('\\', "/");
                }

                if !final_path.ends_with('/') {
                    final_path.push('/');
                }
                Some(final_path)
            }
        };

        let pack_path_str = match pack_path_str {
            Some(s) => s,
            None => continue, // Skip writing this entry
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
