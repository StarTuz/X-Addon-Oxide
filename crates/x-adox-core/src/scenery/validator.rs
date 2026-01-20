use crate::scenery::{SceneryCategory, SceneryPack};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationIssue {
    pub pack_name: String,
    pub severity: ValidationSeverity,
    pub issue_type: String, // e.g., "simheaven_below_global"
    pub message: String,
    pub fix_suggestion: String,
    pub details: String, // Detailed explanation for tooltips
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

pub struct SceneryValidator;

impl SceneryValidator {
    pub fn validate(packs: &[SceneryPack]) -> ValidationReport {
        let mut report = ValidationReport::default();

        // 1. Check for simHeaven placement vs generic airports
        Self::check_simheaven_placement(packs, &mut report);

        // 2. Check for Mesh overlap/ordering (Meshes should be at the bottom)
        Self::check_mesh_ordering(packs, &mut report);

        // 3. Check for Library placement (Libraries should be above Orthos/Meshes)
        Self::check_library_placement(packs, &mut report);

        // 4. Check for Shadowed Packs (Complete overlap)
        // DISABLED due to false positives on additive scenery (e.g. Airports shadowing Global Airports).
        // See shadow_analysis.md for details.
        // Self::check_shadowing(packs, &mut report);

        report
    }

    fn check_shadowing(packs: &[SceneryPack], report: &mut ValidationReport) {
        use crate::scenery::SceneryPackType;

        // Only compare Active packs
        let active_packs: Vec<&SceneryPack> = packs
            .iter()
            .filter(|p| p.status == SceneryPackType::Active)
            .collect();

        // O(N^2) check - acceptable for ~1000 packs on explicit refresh
        for (i, high_pack) in active_packs.iter().enumerate() {
            if high_pack.tiles.is_empty() {
                continue;
            }
            // Only consider "substantial" scenery types (Airports, Overlays, Orthos)
            if !is_substantial(high_pack) {
                continue;
            }

            for low_pack in active_packs.iter().skip(i + 1) {
                if low_pack.tiles.is_empty() {
                    continue;
                }
                if !is_substantial(low_pack) {
                    continue;
                }

                // If B is fully inside A
                if is_subset(&low_pack.tiles, &high_pack.tiles) {
                    println!(
                        "DEBUG: Shadow detected! High: '{}' ({:?}) | Low: '{}' ({:?})",
                        high_pack.name, high_pack.tiles, low_pack.name, low_pack.tiles
                    );
                    report.issues.push(ValidationIssue {
                        pack_name: low_pack.name.clone(),
                        severity: ValidationSeverity::Warning,
                        issue_type: "shadowed_pack".to_string(),
                        message: format!("Completely shadowed by '{}'", high_pack.name),
                        fix_suggestion: "Disable or remove this pack as it is completely obscured by a higher priority pack.".to_string(),
                        details: "This pack's geographical coverage is completely contained within the higher priority pack above it. The lower pack will likely never be seen.".to_string(),
                    });
                }
            }
        }
    }

    fn check_simheaven_placement(packs: &[SceneryPack], report: &mut ValidationReport) {
        let mut simheaven_indices = Vec::new();
        let mut global_airport_idx = None;

        for (i, pack) in packs.iter().enumerate() {
            if pack.name.to_lowercase().contains("simheaven")
                || pack.name.to_lowercase().contains("x-world")
            {
                simheaven_indices.push(i);
            }
            if pack.category == SceneryCategory::GlobalAirport {
                global_airport_idx = Some(i);
            }
        }

        if let Some(ga_idx) = global_airport_idx {
            for &sh_idx in &simheaven_indices {
                if sh_idx > ga_idx {
                    report.issues.push(ValidationIssue {
                        pack_name: packs[sh_idx].name.clone(),
                        severity: ValidationSeverity::Critical,
                        issue_type: "simheaven_below_global".to_string(),
                        message: "simHeaven layer is below Global Airports".to_string(),
                        fix_suggestion: "Move simHeaven layers above Global Airports to avoid missing buildings.".to_string(),
                        details: "Global Airports can override custom scenery below it, flattening runways or hiding terminals. simHeaven layers contain buildings and VFR elements that must be visible above the airport flattened area.".to_string(),
                    });
                }
            }
        }
    }

    fn check_mesh_ordering(packs: &[SceneryPack], report: &mut ValidationReport) {
        let mut first_mesh_idx = None;
        let mut last_overlay_idx = None;

        for (i, pack) in packs.iter().enumerate() {
            if pack.category == SceneryCategory::Mesh || pack.category == SceneryCategory::Ortho {
                if first_mesh_idx.is_none() {
                    first_mesh_idx = Some(i);
                }
            }
            if pack.category == SceneryCategory::Overlay
                || pack.category == SceneryCategory::EarthAirports
                || pack.category == SceneryCategory::GlobalAirport
            {
                last_overlay_idx = Some(i);
            }
        }

        if let (Some(m_idx), Some(o_idx)) = (first_mesh_idx, last_overlay_idx) {
            if m_idx < o_idx {
                // If a mesh is above an overlay (which includes airports/simheaven)
                report.issues.push(ValidationIssue {
                    pack_name: packs[m_idx].name.clone(),
                    severity: ValidationSeverity::Warning,
                    issue_type: "mesh_above_overlay".to_string(),
                    message: "Mesh/Terrain pack is above an Overlay/Airport".to_string(),
                    fix_suggestion: "Move Mesh and Terrain packs to the bottom of the list for correct layering.".to_string(),
                    details: "X-Plane draws scenery from bottom to top. Terrain meshes should be at the very bottom so that airports and overlays can be 'draped' over them. If a mesh is above an overlay, the overlay might be hidden.".to_string(),
                });
            }
        }
    }

    fn check_library_placement(_packs: &[SceneryPack], _report: &mut ValidationReport) {
        // Libraries are flexible but usually should not be at the very bottom
    }
}

fn is_substantial(pack: &crate::scenery::SceneryPack) -> bool {
    use crate::scenery::SceneryCategory;
    matches!(
        pack.category,
        SceneryCategory::EarthScenery
            | SceneryCategory::EarthAirports
            | SceneryCategory::Overlay
            | SceneryCategory::Ortho
            | SceneryCategory::Mesh
            | SceneryCategory::Unknown // Treat unknown as potential scenery
    )
}

fn is_subset(small: &[(i32, i32)], big: &[(i32, i32)]) -> bool {
    // Both are sorted.
    // Check if every element of 'small' exists in 'big'.
    let mut big_iter = big.iter();
    for s_tile in small {
        let mut found = false;
        while let Some(b_tile) = big_iter.next() {
            if b_tile == s_tile {
                found = true;
                break;
            }
            if b_tile > s_tile {
                // If big tile is past the small tile, then small tile isn't in big (sorted).
                return false;
            }
        }
        if !found {
            return false;
        }
    }
    true
}
