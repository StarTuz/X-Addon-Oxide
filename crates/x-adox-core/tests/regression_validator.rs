// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz
//
// Regression tests for scenery validator (validator.rs).
// Covers: SimHeaven placement, mesh ordering, mesh shadowing,
// disabled pack exclusion, library position-independence.

use std::path::PathBuf;
use x_adox_core::scenery::validator::{SceneryValidator, ValidationSeverity};
use x_adox_core::scenery::{SceneryCategory, SceneryDescriptor, SceneryPack, SceneryPackType};

fn make_pack(name: &str, category: SceneryCategory) -> SceneryPack {
    SceneryPack {
        name: name.to_string(),
        path: PathBuf::from(name),
        raw_path: None,
        status: SceneryPackType::Active,
        category,
        airports: Vec::new(),
        tiles: Vec::new(),
        tags: Vec::new(),
        descriptor: SceneryDescriptor::default(),
        region: None,
    }
}

fn make_mesh(name: &str, tiles: Vec<(i32, i32)>) -> SceneryPack {
    let mut pack = make_pack(name, SceneryCategory::Mesh);
    pack.tiles = tiles;
    pack
}

// =====================================================================
// SimHeaven Placement
// =====================================================================

#[test]
fn test_simheaven_above_global_airports_is_ok() {
    // Community standard: SimHeaven ABOVE Global Airports is correct.
    // SimHeaven at index 0, Global Airports at index 1 → simheaven is ABOVE → OK
    let packs = vec![
        make_pack(
            "simHeaven_X-World_Europe-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack("Global Airports", SceneryCategory::GlobalAirport),
    ];

    let report = SceneryValidator::validate(&packs);
    let simheaven_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "simheaven_below_global")
        .collect();
    assert!(
        simheaven_issues.is_empty(),
        "SimHeaven above Global Airports is correct per community standard"
    );
}

#[test]
fn test_simheaven_below_global_airports_is_critical() {
    // Global Airports at index 0, SimHeaven at index 1 → simheaven is BELOW → Critical
    // Community standard: SimHeaven must be ABOVE Global Airports.
    let packs = vec![
        make_pack("Global Airports", SceneryCategory::GlobalAirport),
        make_pack(
            "simHeaven_X-World_Europe-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
    ];

    let report = SceneryValidator::validate(&packs);
    let simheaven_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "simheaven_below_global")
        .collect();
    assert_eq!(
        simheaven_issues.len(),
        1,
        "SimHeaven below Global Airports should be flagged as critical"
    );
    assert_eq!(simheaven_issues[0].severity, ValidationSeverity::Critical);
}

#[test]
fn test_multiple_simheaven_below_global_airports() {
    // Two SimHeaven packs below Global Airports → two Critical issues
    // Community standard: SimHeaven must be ABOVE Global Airports.
    let packs = vec![
        make_pack("Global Airports", SceneryCategory::GlobalAirport),
        make_pack(
            "simHeaven_X-World_Europe-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_America-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
    ];

    let report = SceneryValidator::validate(&packs);
    let simheaven_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "simheaven_below_global")
        .collect();
    assert_eq!(simheaven_issues.len(), 2);
}

#[test]
fn test_x_world_name_also_triggers_simheaven_check() {
    // "x-world" in name (without "simheaven") should also be caught when BELOW GA
    let packs = vec![
        make_pack("Global Airports", SceneryCategory::GlobalAirport),
        make_pack(
            "X-World_Europe_Vegetation_Library",
            SceneryCategory::Library,
        ),
    ];

    let report = SceneryValidator::validate(&packs);
    let simheaven_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "simheaven_below_global")
        .collect();
    assert_eq!(simheaven_issues.len(), 1);
}

#[test]
fn test_no_global_airports_means_no_simheaven_issue() {
    // If there's no Global Airports pack at all, no SimHeaven placement issue
    let packs = vec![
        make_pack(
            "simHeaven_X-World_Europe-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack("Some_Airport", SceneryCategory::CustomAirport),
    ];

    let report = SceneryValidator::validate(&packs);
    let simheaven_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "simheaven_below_global")
        .collect();
    assert!(simheaven_issues.is_empty());
}

// =====================================================================
// Mesh Ordering
// =====================================================================

#[test]
fn test_mesh_above_airport_triggers_warning() {
    // Mesh at index 0, Airport at index 1 → mesh is above overlay → Warning
    let packs = vec![
        make_pack("zzz_UHD_Mesh", SceneryCategory::Mesh),
        make_pack("KSEA_Airport", SceneryCategory::CustomAirport),
    ];

    let report = SceneryValidator::validate(&packs);
    let mesh_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "mesh_above_overlay")
        .collect();
    assert_eq!(mesh_issues.len(), 1);
    assert_eq!(mesh_issues[0].severity, ValidationSeverity::Warning);
    assert_eq!(mesh_issues[0].pack_name, "zzz_UHD_Mesh");
}

#[test]
fn test_ortho_above_airport_triggers_warning() {
    // OrthoBase is also a mesh category in the validator
    let packs = vec![
        make_pack("yOrtho4XP_+40-080", SceneryCategory::OrthoBase),
        make_pack("KSEA_Airport", SceneryCategory::CustomAirport),
    ];

    let report = SceneryValidator::validate(&packs);
    let mesh_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "mesh_above_overlay")
        .collect();
    assert_eq!(mesh_issues.len(), 1);
}

#[test]
fn test_mesh_below_all_overlays_is_ok() {
    // Correct ordering: Airport → Overlay → Mesh
    let packs = vec![
        make_pack("KSEA_Airport", SceneryCategory::CustomAirport),
        make_pack(
            "simHeaven_X-World_Europe",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack("zzz_UHD_Mesh", SceneryCategory::Mesh),
    ];

    let report = SceneryValidator::validate(&packs);
    let mesh_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "mesh_above_overlay")
        .collect();
    assert!(mesh_issues.is_empty());
}

#[test]
fn test_library_between_mesh_and_overlay_no_warning() {
    // Library is NOT position-sensitive, so mesh above library doesn't count.
    // Mesh at 0, Library at 1, no position-sensitive pack after mesh → no issue
    let packs = vec![
        make_pack("zzz_UHD_Mesh", SceneryCategory::Mesh),
        make_pack("OpenSceneryX_Library", SceneryCategory::Library),
    ];

    let report = SceneryValidator::validate(&packs);
    let mesh_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "mesh_above_overlay")
        .collect();
    assert!(
        mesh_issues.is_empty(),
        "Library is position-independent; mesh above library is not an issue"
    );
}

#[test]
fn test_mesh_above_regional_fluff_triggers_warning() {
    // RegionalFluff is position-sensitive
    let packs = vec![
        make_pack("zzz_UHD_Mesh", SceneryCategory::Mesh),
        make_pack("Global_Forests_v2", SceneryCategory::RegionalFluff),
    ];

    let report = SceneryValidator::validate(&packs);
    let mesh_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "mesh_above_overlay")
        .collect();
    assert_eq!(mesh_issues.len(), 1);
}

#[test]
fn test_mesh_above_auto_ortho_overlay_triggers_warning() {
    // AutoOrthoOverlay is position-sensitive
    let packs = vec![
        make_pack("zzz_UHD_Mesh", SceneryCategory::Mesh),
        make_pack("yAutoOrtho_Overlays", SceneryCategory::AutoOrthoOverlay),
    ];

    let report = SceneryValidator::validate(&packs);
    let mesh_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "mesh_above_overlay")
        .collect();
    assert_eq!(mesh_issues.len(), 1);
}

// =====================================================================
// Mesh Shadowing
// =====================================================================

#[test]
fn test_mesh_fully_shadowed_triggers_warning() {
    // Higher mesh covers tiles (10,20), (11,21)
    // Lower mesh has only (10,20) — fully inside higher mesh
    let packs = vec![
        make_mesh("HD_Mesh_v4", vec![(10, 20), (11, 21)]),
        make_mesh("Old_Mesh", vec![(10, 20)]),
    ];

    let report = SceneryValidator::validate(&packs);
    let shadow_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "shadowed_mesh")
        .collect();
    assert_eq!(shadow_issues.len(), 1);
    assert_eq!(shadow_issues[0].pack_name, "Old_Mesh");
}

#[test]
fn test_mesh_partially_overlapping_no_shadow() {
    // Meshes share tile (10,20) but lower mesh has (12,22) which isn't in higher
    let packs = vec![
        make_mesh("HD_Mesh_v4", vec![(10, 20), (11, 21)]),
        make_mesh("Regional_Mesh", vec![(10, 20), (12, 22)]),
    ];

    let report = SceneryValidator::validate(&packs);
    let shadow_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "shadowed_mesh")
        .collect();
    assert!(
        shadow_issues.is_empty(),
        "Partial overlap is not a shadow"
    );
}

#[test]
fn test_non_mesh_tiles_not_checked_for_shadowing() {
    // OrthoBase packs with overlapping tiles should NOT trigger shadowing
    // (is_mesh only matches Mesh — SpecificMesh and OrthoBase are excluded)
    let mut pack1 = make_pack("Ortho_Region_A", SceneryCategory::OrthoBase);
    pack1.tiles = vec![(10, 20), (11, 21)];
    let mut pack2 = make_pack("Ortho_Region_B", SceneryCategory::OrthoBase);
    pack2.tiles = vec![(10, 20)];

    let packs = vec![pack1, pack2];
    let report = SceneryValidator::validate(&packs);
    let shadow_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "shadowed_mesh")
        .collect();
    assert!(
        shadow_issues.is_empty(),
        "OrthoBase should not trigger mesh shadowing checks"
    );
}

#[test]
fn test_disabled_mesh_not_checked_for_shadowing() {
    // Disabled packs should be excluded from shadowing checks
    let pack1 = make_mesh("HD_Mesh_v4", vec![(10, 20), (11, 21)]);
    let mut pack2 = make_mesh("Old_Mesh", vec![(10, 20)]);
    pack2.status = SceneryPackType::Disabled;

    let packs = vec![pack1, pack2];
    let report = SceneryValidator::validate(&packs);
    let shadow_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "shadowed_mesh")
        .collect();
    assert!(
        shadow_issues.is_empty(),
        "Disabled mesh should be excluded from shadowing checks"
    );
}

#[test]
fn test_empty_tiles_no_shadowing() {
    // Meshes with no tiles should not produce shadow warnings
    let packs = vec![
        make_mesh("HD_Mesh_v4", vec![]),
        make_mesh("Old_Mesh", vec![]),
    ];

    let report = SceneryValidator::validate(&packs);
    let shadow_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "shadowed_mesh")
        .collect();
    assert!(shadow_issues.is_empty());
}

// =====================================================================
// Combined / Edge Cases
// =====================================================================

#[test]
fn test_multiple_validation_issues_simultaneously() {
    // Bad ordering: SimHeaven below Global, Mesh above Airport
    let packs = vec![
        make_pack("zzz_UHD_Mesh", SceneryCategory::Mesh),
        make_pack("Global Airports", SceneryCategory::GlobalAirport),
        make_pack(
            "simHeaven_X-World_Europe",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack("KSEA_Airport", SceneryCategory::CustomAirport),
    ];

    let report = SceneryValidator::validate(&packs);
    // Should have at least: SimHeaven below Global + Mesh above Airport
    assert!(
        report.issues.len() >= 2,
        "Expected multiple issues, got {}",
        report.issues.len()
    );

    let types: Vec<&str> = report.issues.iter().map(|i| i.issue_type.as_str()).collect();
    assert!(types.contains(&"simheaven_below_global"));
    assert!(types.contains(&"mesh_above_overlay"));
}

#[test]
fn test_clean_ordering_produces_no_issues() {
    // Correct community standard: Airports → SimHeaven → Global → Libraries → Ortho → Mesh
    let packs = vec![
        make_pack("KSEA_Airport", SceneryCategory::CustomAirport),
        make_pack(
            "simHeaven_X-World_Europe",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack("Global Airports", SceneryCategory::GlobalAirport),
        make_pack("OpenSceneryX", SceneryCategory::Library),
        make_pack("yOrtho4XP", SceneryCategory::OrthoBase),
        make_pack("zzz_UHD_Mesh", SceneryCategory::Mesh),
    ];

    let report = SceneryValidator::validate(&packs);
    assert!(
        report.issues.is_empty(),
        "Clean ordering should produce no issues, got: {:?}",
        report.issues.iter().map(|i| &i.issue_type).collect::<Vec<_>>()
    );
}

#[test]
fn test_specific_mesh_not_checked_for_shadowing() {
    // SpecificMesh packs (airport companion packs like grass, terrain, sealane)
    // serve different purposes and should NOT trigger mesh shadowing warnings.
    let mut pack1 = make_pack("EGLL_3Dgrass", SceneryCategory::SpecificMesh);
    pack1.tiles = vec![(51, 0), (51, -1)];
    let mut pack2 = make_pack("EGLL_MESH", SceneryCategory::SpecificMesh);
    pack2.tiles = vec![(51, 0)];

    let packs = vec![pack1, pack2];
    let report = SceneryValidator::validate(&packs);
    let shadow_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "shadowed_mesh")
        .collect();
    assert!(
        shadow_issues.is_empty(),
        "SpecificMesh companion packs should not trigger mesh shadowing"
    );
}

#[test]
fn test_specific_mesh_above_overlay_no_warning() {
    // SpecificMesh packs are airport-adjacent and should not trigger
    // "mesh above overlay" warnings — they're not bottom-of-list terrain.
    let packs = vec![
        make_pack("EGLL_3Dgrass", SceneryCategory::SpecificMesh),
        make_pack("yAutoOrtho_Overlays", SceneryCategory::AutoOrthoOverlay),
    ];

    let report = SceneryValidator::validate(&packs);
    let mesh_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type == "mesh_above_overlay")
        .collect();
    assert!(
        mesh_issues.is_empty(),
        "SpecificMesh above AutoOrtho should not be flagged"
    );
}

#[test]
fn test_empty_pack_list_no_issues() {
    let report = SceneryValidator::validate(&[]);
    assert!(report.issues.is_empty());
}
