// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

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

        // 4. Check for Shadowed Meshes (Complete overlap)
        Self::check_mesh_shadowing(packs, &mut report);

        report
    }

    fn check_mesh_shadowing(packs: &[SceneryPack], report: &mut ValidationReport) {
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
            // Strict Filtering: ONLY Mesh and EarthScenery (Default Mesh)
            // Explicitly EXCLUDE Ortho, Overlay, Airports
            if !is_mesh(high_pack) {
                continue;
            }

            for low_pack in active_packs.iter().skip(i + 1) {
                if low_pack.tiles.is_empty() {
                    continue;
                }
                if !is_mesh(low_pack) {
                    continue;
                }

                // If B is fully inside A
                if is_subset(&low_pack.tiles, &high_pack.tiles) {
                    println!(
                        "DEBUG: Mesh Shadow detected! High: '{}' ({:?}) | Low: '{}' ({:?})",
                        high_pack.name, high_pack.tiles, low_pack.name, low_pack.tiles
                    );
                    report.issues.push(ValidationIssue {
                        pack_name: low_pack.name.clone(),
                        severity: ValidationSeverity::Warning,
                        issue_type: "shadowed_mesh".to_string(),
                        message: format!("Mesh completely shadowed by '{}'", high_pack.name),
                        fix_suggestion: "Disable or remove this mesh pack as it is completely obscured by a higher priority mesh.".to_string(),
                        details: "In X-Plane, only the highest priority mesh for a given tile is rendered. This pack is entirely covered by a mesh above it and will never be shown.".to_string(),
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
                if sh_idx < ga_idx {
                    report.issues.push(ValidationIssue {
                        pack_name: packs[sh_idx].name.clone(),
                        severity: ValidationSeverity::Critical,
                        issue_type: "simheaven_below_global".to_string(),
                        message: "simHeaven layer is above Global Airports".to_string(),
                        fix_suggestion: "Move simHeaven layers below Global Airports to avoid visual artifacts.".to_string(),
                        details: "In X-Plane 12, simHeaven X-World packages should be placed below the Global Airports entry for correct layering and to ensure libraries are properly referenced.".to_string(),
                    });
                }
            }
        }
    }

    fn check_mesh_ordering(packs: &[SceneryPack], report: &mut ValidationReport) {
        // Simplified position-based check:
        // If the list is already sorted such that mesh-like packs are at the bottom,
        // don't report issues even if their category labels are wrong.
        //
        // Strategy: Check if any pack with "mesh", "ortho", "z_ao_" in the name
        // appears before a pack that looks like an airport/overlay by name.

        let is_mesh_by_name = |name: &str| -> bool {
            let lower = name.to_lowercase();
            lower.contains("mesh")
                || lower.contains("z_ao_")
                || lower.contains("z_autoortho")
                || lower.contains("ortho4xp")
                || (lower.contains("orthos") && !lower.contains("overlay"))
                || lower.starts_with("zzz")
        };

        let is_overlay_by_name = |name: &str| -> bool {
            let lower = name.to_lowercase();
            // Check for definitive overlay/airport patterns
            lower.contains("airport")
                || lower.contains("_overlay")
                || lower.contains("landmarks")
                || lower.contains("global_airports")
                || lower.contains("simheaven")
                || lower.contains("library")
        };

        // Find first mesh-by-name pack
        let mut first_mesh_idx = None;
        let mut first_mesh_name = String::new();
        for (i, pack) in packs.iter().enumerate() {
            if is_mesh_by_name(&pack.name) {
                first_mesh_idx = Some(i);
                first_mesh_name = pack.name.clone();
                break;
            }
        }

        // Find last overlay-by-name pack
        let mut last_overlay_idx = None;
        let mut last_overlay_name = String::new();
        for (i, pack) in packs.iter().enumerate() {
            if is_overlay_by_name(&pack.name) {
                last_overlay_idx = Some(i);
                last_overlay_name = pack.name.clone();
            }
        }

        if let (Some(m_idx), Some(o_idx)) = (first_mesh_idx, last_overlay_idx) {
            if m_idx < o_idx {
                report.issues.push(ValidationIssue {
                    pack_name: first_mesh_name,
                    severity: ValidationSeverity::Warning,
                    issue_type: "mesh_above_overlay".to_string(),
                    message: format!("Mesh/Ortho pack is above '{}'", last_overlay_name),
                    fix_suggestion: "Move Mesh and Ortho packs to the bottom of the list.".to_string(),
                    details: "X-Plane draws scenery from bottom to top. Terrain meshes and orthophotos should be at the bottom.".to_string(),
                });
            }
        }
    }

    fn check_library_placement(_packs: &[SceneryPack], _report: &mut ValidationReport) {
        // Libraries are flexible
    }
}

fn is_mesh(pack: &crate::scenery::SceneryPack) -> bool {
    use crate::scenery::SceneryCategory;
    matches!(
        pack.category,
        SceneryCategory::Mesh | SceneryCategory::SpecificMesh
    )
}

pub(crate) fn is_subset(small: &[(i32, i32)], big: &[(i32, i32)]) -> bool {
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
