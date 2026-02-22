// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz
//
// Regression tests for scenery score modifiers in sorter.rs.
// Tests internal scoring logic indirectly through sort_packs() behavior.
// Covers: category scores, VFR boost, y/z prefix penalty, mesh name cap.

use std::path::PathBuf;
use x_adox_core::scenery::sorter::sort_packs;
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

fn sorted_names(packs: &mut Vec<SceneryPack>) -> Vec<String> {
    sort_packs(packs, None, &x_adox_bitnet::PredictContext::default());
    packs.iter().map(|p| p.name.clone()).collect()
}

// =====================================================================
// Category Score Values (higher score = higher priority = sorts first)
// =====================================================================

#[test]
fn test_category_score_ordering() {
    // Full tier ordering from top to bottom
    let mut packs = vec![
        make_pack("zzz_UHD_Mesh", SceneryCategory::Mesh), // 30
        make_pack("Ortho4XP_Region", SceneryCategory::OrthoBase), // 50
        make_pack("Demo_Area", SceneryCategory::GlobalBase), // 60
        make_pack("AutoOrtho_Overlays", SceneryCategory::AutoOrthoOverlay), // 65
        make_pack("Global_Forests", SceneryCategory::RegionalFluff), // 70
        make_pack("simHeaven_Europe", SceneryCategory::RegionalOverlay), // 75
        make_pack("FollowMe_Cars", SceneryCategory::AirportOverlay), // 80
        make_pack("OpenSceneryX", SceneryCategory::Library), // 85
        make_pack("Global Airports", SceneryCategory::GlobalAirport), // 90
        make_pack("X-Plane Landmarks Paris", SceneryCategory::Landmark), // 95
        make_pack("Orbx_A_EGLC", SceneryCategory::OrbxAirport), // 95
        make_pack("KSEA_Airport", SceneryCategory::CustomAirport), // 100
    ];

    let names = sorted_names(&mut packs);

    // CustomAirport (100) must be first
    assert_eq!(names[0], "KSEA_Airport");

    // OrbxAirport and Landmark both score 95, stable sort preserves input order
    // They were given as Landmark then Orbx, so that's how they appear
    assert!(
        names.iter().position(|n| n == "Orbx_A_EGLC").unwrap()
            < names.iter().position(|n| n == "Global Airports").unwrap(),
        "OrbxAirport (95) should be above GlobalAirport (90)"
    );

    assert!(
        names.iter().position(|n| n == "Global Airports").unwrap()
            < names.iter().position(|n| n == "OpenSceneryX").unwrap(),
        "GlobalAirport (90) should be above Library (85)"
    );

    assert!(
        names.iter().position(|n| n == "OpenSceneryX").unwrap()
            < names.iter().position(|n| n == "FollowMe_Cars").unwrap(),
        "Library (85) should be above AirportOverlay (80)"
    );

    assert!(
        names.iter().position(|n| n == "simHeaven_Europe").unwrap()
            < names.iter().position(|n| n == "Global_Forests").unwrap(),
        "RegionalOverlay (75) should be above RegionalFluff (70)"
    );

    assert!(
        names
            .iter()
            .position(|n| n == "AutoOrtho_Overlays")
            .unwrap()
            < names.iter().position(|n| n == "Demo_Area").unwrap(),
        "AutoOrthoOverlay (65) should be above GlobalBase (60)"
    );

    assert!(
        names.iter().position(|n| n == "Ortho4XP_Region").unwrap()
            < names.iter().position(|n| n == "zzz_UHD_Mesh").unwrap(),
        "OrthoBase (50) should be above Mesh (30)"
    );

    // Mesh must be last
    assert_eq!(names.last().unwrap(), "zzz_UHD_Mesh");
}

// =====================================================================
// VFR Boost (+5)
// =====================================================================

#[test]
fn test_vfr_boost_non_simheaven() {
    // A VFR overlay should get +5, pushing it above a non-VFR overlay of same category
    // RegionalOverlay = 75, with VFR boost = 80
    // AirportOverlay = 80
    // So VFR RegionalOverlay should tie with AirportOverlay
    let mut packs = vec![
        make_pack("FollowMe_Cars", SceneryCategory::AirportOverlay), // 80
        make_pack("Some_VFR_Overlay", SceneryCategory::RegionalOverlay), // 75 + 5 = 80
    ];

    let names = sorted_names(&mut packs);
    // Equal scores → stable sort preserves input order
    assert_eq!(names[0], "FollowMe_Cars");
    assert_eq!(names[1], "Some_VFR_Overlay");
}

#[test]
fn test_vfr_boost_simheaven_exempt() {
    // SimHeaven VFR layers should NOT get the VFR boost (they manage layers internally)
    // Both are RegionalOverlay (75). If SimHeaven got +5, it would be 80 and sort above the other.
    // Since it's exempt, both stay at 75 and stable sort preserves input order.
    let mut packs = vec![
        make_pack(
            "simHeaven_X-World_Europe-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_Europe-2-regions",
            SceneryCategory::RegionalOverlay,
        ),
    ];

    let names = sorted_names(&mut packs);
    // SimHeaven secondary sort: continent grouping then layer number
    // Both are Europe, layers 1 then 2
    assert_eq!(names[0], "simHeaven_X-World_Europe-1-vfr");
    assert_eq!(names[1], "simHeaven_X-World_Europe-2-regions");
}

// =====================================================================
// y/z Prefix Penalty (-20)
// =====================================================================

#[test]
fn test_yz_prefix_penalty_applied() {
    // OrthoBase = 50. With z prefix: 50 - 20 = 30 (same as Mesh)
    // Regular OrthoBase = 50
    let mut packs = vec![
        make_pack("z_ao_europe", SceneryCategory::OrthoBase), // 50 - 20 = 30
        make_pack("Regular_Ortho", SceneryCategory::OrthoBase), // 50
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "Regular_Ortho",
        "Regular ortho (50) should be above z-prefix ortho (30)"
    );
    assert_eq!(names[1], "z_ao_europe");
}

#[test]
fn test_yz_prefix_penalty_y_prefix() {
    // y prefix also gets penalty
    let mut packs = vec![
        make_pack("yOrtho4XP_+40", SceneryCategory::OrthoBase), // 50 - 20 = 30
        make_pack("Regular_Ortho", SceneryCategory::OrthoBase), // 50
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(names[0], "Regular_Ortho");
    assert_eq!(names[1], "yOrtho4XP_+40");
}

#[test]
fn test_yz_prefix_exempt_custom_airport() {
    // CustomAirport is EXEMPT from y/z penalty
    // CustomAirport = 100. Even with y prefix, should stay 100.
    let mut packs = vec![
        make_pack("yFlyTampa_YSSY", SceneryCategory::CustomAirport), // 100 (exempt)
        make_pack("Global Airports", SceneryCategory::GlobalAirport), // 90
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "yFlyTampa_YSSY",
        "CustomAirport is exempt from y/z penalty"
    );
}

#[test]
fn test_yz_prefix_exempt_airport_overlay() {
    // AirportOverlay is EXEMPT from y/z penalty
    let mut packs = vec![
        make_pack("yAutoOrtho_Overlays", SceneryCategory::AirportOverlay), // 80 (exempt)
        make_pack("simHeaven_Europe", SceneryCategory::RegionalOverlay),   // 75
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "yAutoOrtho_Overlays",
        "AirportOverlay is exempt from y/z penalty"
    );
}

#[test]
fn test_yz_prefix_exempt_library() {
    // Library is EXEMPT from y/z penalty
    let mut packs = vec![
        make_pack("zLib_Something", SceneryCategory::Library), // 85 (exempt)
        make_pack("FollowMe_Cars", SceneryCategory::AirportOverlay), // 80
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "zLib_Something",
        "Library is exempt from y/z penalty"
    );
}

#[test]
fn test_yz_prefix_exempt_global_base() {
    // GlobalBase is EXEMPT from y/z penalty
    let mut packs = vec![
        make_pack("zGlobal_Base_Pack", SceneryCategory::GlobalBase), // 60 (exempt)
        make_pack("Regular_Ortho", SceneryCategory::OrthoBase),      // 50
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "zGlobal_Base_Pack",
        "GlobalBase is exempt from y/z penalty"
    );
}

#[test]
fn test_yz_prefix_not_exempt_regional_overlay() {
    // RegionalOverlay is NOT exempt
    // RegionalOverlay = 75, with z prefix = 55
    // OrthoBase = 50
    let mut packs = vec![
        make_pack("z_Regional_Thing", SceneryCategory::RegionalOverlay), // 75 - 20 = 55
        make_pack("Regular_Ortho", SceneryCategory::OrthoBase),          // 50
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "z_Regional_Thing",
        "Penalized RegionalOverlay (55) still above OrthoBase (50)"
    );
    assert_eq!(names[1], "Regular_Ortho");
}

// =====================================================================
// Mesh Name-Based Score Cap
// =====================================================================

#[test]
fn test_mesh_name_caps_score_at_30() {
    // A pack classified as RegionalOverlay (75) but with "mesh" in name → capped to 30
    let mut packs = vec![
        make_pack(
            "FlyTampa_Amsterdam_3_mesh",
            SceneryCategory::RegionalOverlay,
        ), // 75 → 30
        make_pack("Regular_Ortho", SceneryCategory::OrthoBase), // 50
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "Regular_Ortho",
        "OrthoBase (50) should be above mesh-named pack (capped to 30)"
    );
    assert_eq!(names[1], "FlyTampa_Amsterdam_3_mesh");
}

#[test]
fn test_mesh_name_overrides_category() {
    // Even CustomAirport (100) with "mesh" in name gets capped to 30
    let mut packs = vec![
        make_pack("Airport_mesh_terrain", SceneryCategory::CustomAirport), // 100 → 30
        make_pack("Regular_Ortho", SceneryCategory::OrthoBase),            // 50
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "Regular_Ortho",
        "Mesh-named pack should be capped to 30 regardless of category"
    );
}

#[test]
fn test_mesh_category_pack_stays_at_30() {
    // Mesh category = 30, no additional penalty from name
    let mut packs = vec![
        make_pack("zzz_UHD_Mesh_v4", SceneryCategory::Mesh), // 30 (name has mesh, already 30)
        make_pack("Another_Mesh", SceneryCategory::Mesh),    // 30
    ];

    let names = sorted_names(&mut packs);
    // Both at 30, stable sort preserves input order
    assert_eq!(names[0], "zzz_UHD_Mesh_v4");
    assert_eq!(names[1], "Another_Mesh");
}

// =====================================================================
// Unknown Category
// =====================================================================

#[test]
fn test_unknown_category_sorts_last() {
    // Unknown = 0, which is below Mesh (30)
    let mut packs = vec![
        make_pack("Mystery_Pack", SceneryCategory::Unknown), // 0
        make_pack("zzz_Mesh", SceneryCategory::Mesh),        // 30
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(names[0], "zzz_Mesh");
    assert_eq!(names[1], "Mystery_Pack");
}

// =====================================================================
// Score Modifier Combinations
// =====================================================================

#[test]
fn test_yz_penalty_plus_vfr_boost() {
    // RegionalOverlay = 75, VFR boost = +5, z prefix = -20
    // Total: 75 + 5 - 20 = 60
    let mut packs = vec![
        make_pack("z_VFR_Overlay", SceneryCategory::RegionalOverlay), // 75 + 5 - 20 = 60
        make_pack("Demo_Area", SceneryCategory::GlobalBase),          // 60
    ];

    let names = sorted_names(&mut packs);
    // Both at 60, stable sort preserves input order
    assert_eq!(names[0], "z_VFR_Overlay");
    assert_eq!(names[1], "Demo_Area");
}

#[test]
fn test_mesh_name_overrides_all_modifiers() {
    // Even with VFR boost, "mesh" in name caps to 30
    let mut packs = vec![
        make_pack("VFR_mesh_terrain", SceneryCategory::RegionalOverlay), // 75 + 5 → mesh cap → 30
        make_pack("Regular_Ortho", SceneryCategory::OrthoBase),          // 50
    ];

    let names = sorted_names(&mut packs);
    assert_eq!(
        names[0], "Regular_Ortho",
        "Mesh name cap (30) should override VFR boost"
    );
}
