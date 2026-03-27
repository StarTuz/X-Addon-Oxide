// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz
//
// Full-pipeline integration tests.
// These tests run realistic pack lists through the complete pipeline:
//   classify → sort(BitNet) → validate
// and assert zero warnings. Unlike other tests that exercise subsystems
// in isolation, these catch emergent bugs where two correct subsystems
// produce incorrect combined behavior.

use std::path::PathBuf;
use x_adox_bitnet::{BitNetModel, PredictContext};
use x_adox_core::scenery::classifier::Classifier;
use x_adox_core::scenery::sorter::sort_packs;
use x_adox_core::scenery::validator::SceneryValidator;
use x_adox_core::scenery::{SceneryDescriptor, SceneryPack, SceneryPackType};

/// Build a pack from a name, running it through the real classifier.
fn classified_pack(name: &str, model: &BitNetModel, context: &PredictContext) -> SceneryPack {
    let path = PathBuf::from(format!("Custom Scenery/{}", name));
    let category = Classifier::classify(name, &path, context, model);
    SceneryPack {
        name: name.to_string(),
        path,
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

// =====================================================================
// Pipeline Integration: classify → sort → validate
// =====================================================================

#[test]
fn test_pipeline_standard_xplane_install() {
    // Simulates a typical X-Plane 12 install with a mix of airports,
    // overlays, libraries, orthos, and mesh.
    let model = BitNetModel::default();
    let ctx = PredictContext::default();

    let mut packs = vec![
        classified_pack("KSEA_Airport", &model, &ctx),
        classified_pack("EGLL_Heathrow", &model, &ctx),
        classified_pack("Global Airports", &model, &ctx),
        classified_pack("X-Plane Landmarks - London", &model, &ctx),
        classified_pack("simHeaven_X-World_Europe-1-vfr", &model, &ctx),
        classified_pack("simHeaven_X-World_Europe-2-regions", &model, &ctx),
        classified_pack("OpenSceneryX_Library", &model, &ctx),
        classified_pack("zzz_UHD_Mesh_v4", &model, &ctx),
    ];

    sort_packs(&mut packs, Some(&model), &ctx);

    let report = SceneryValidator::validate(&packs);
    let real_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type != "shadowed_mesh") // Tile-based, not relevant here
        .collect();

    assert!(
        real_issues.is_empty(),
        "Standard install pipeline should produce no warnings after sort.\n\
         Sorted order: {}\n\
         Issues: {:?}",
        packs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" > "),
        real_issues
    );
}

#[test]
fn test_pipeline_xpme_with_orbx_orthos() {
    // The exact scenario that caused the XPME bug: XPME overlay + Orbx ortho.
    let model = BitNetModel::default();
    let ctx = PredictContext::default();

    let mut packs = vec![
        classified_pack("FlyTampa_Amsterdam_1_overlays", &model, &ctx),
        classified_pack("XPME_Overlays", &model, &ctx),
        classified_pack("Orbx_C_GB_South_TrueEarth_Orthos", &model, &ctx),
        classified_pack("XPME_South_America", &model, &ctx),
        classified_pack("XPME_Europe", &model, &ctx),
        classified_pack("Global Airports", &model, &ctx),
        classified_pack("zzz_UHD_Mesh_v4", &model, &ctx),
    ];

    sort_packs(&mut packs, Some(&model), &ctx);

    let report = SceneryValidator::validate(&packs);
    let real_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type != "shadowed_mesh")
        .collect();

    assert!(
        real_issues.is_empty(),
        "XPME + Orbx pipeline should produce no warnings.\n\
         Sorted order: {}\n\
         Issues: {:?}",
        packs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" > "),
        real_issues
    );

    // Verify XPME_Overlays is above Orbx_C orthos
    let xpme_overlay_idx = packs.iter().position(|p| p.name == "XPME_Overlays").unwrap();
    let orbx_ortho_idx = packs
        .iter()
        .position(|p| p.name == "Orbx_C_GB_South_TrueEarth_Orthos")
        .unwrap();
    assert!(
        xpme_overlay_idx < orbx_ortho_idx,
        "XPME_Overlays (idx {}) must be above Orbx Orthos (idx {})",
        xpme_overlay_idx,
        orbx_ortho_idx
    );
}

#[test]
fn test_pipeline_orbx_full_product_line() {
    // Orbx ships airports (A), overlays (B), orthos (C), and mesh (D).
    // All must sort in the correct order relative to each other and other packs.
    let model = BitNetModel::default();
    let ctx = PredictContext::default();

    let mut packs = vec![
        classified_pack("Orbx_A_EGLC_LondonCity", &model, &ctx),
        classified_pack("Orbx_A_GB_South_TrueEarth_Custom", &model, &ctx),
        classified_pack("Orbx_B_US_NorCal_TE_Overlay", &model, &ctx),
        classified_pack("Orbx_C_GB_South_TrueEarth_Orthos", &model, &ctx),
        classified_pack("Global Airports", &model, &ctx),
        classified_pack("simHeaven_X-World_Europe-1-vfr", &model, &ctx),
        classified_pack("zzz_UHD_Mesh_v4", &model, &ctx),
    ];

    sort_packs(&mut packs, Some(&model), &ctx);

    let report = SceneryValidator::validate(&packs);
    let real_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type != "shadowed_mesh")
        .collect();

    assert!(
        real_issues.is_empty(),
        "Orbx full product line should produce no warnings.\n\
         Sorted order: {}\n\
         Issues: {:?}",
        packs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" > "),
        real_issues
    );
}

#[test]
fn test_pipeline_autoortho_with_overlays() {
    // z_ao_ base packs must sort below overlays.
    let model = BitNetModel::default();
    let ctx = PredictContext::default();

    let mut packs = vec![
        classified_pack("FlyTampa_Amsterdam_1_overlays", &model, &ctx),
        classified_pack("yAutoOrtho_Overlays", &model, &ctx),
        classified_pack("z_ao_eur", &model, &ctx),
        classified_pack("Global Airports", &model, &ctx),
        classified_pack("zzz_UHD_Mesh_v4", &model, &ctx),
    ];

    sort_packs(&mut packs, Some(&model), &ctx);

    let report = SceneryValidator::validate(&packs);
    let real_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type != "shadowed_mesh")
        .collect();

    assert!(
        real_issues.is_empty(),
        "AutoOrtho pipeline should produce no warnings.\n\
         Sorted order: {}\n\
         Issues: {:?}",
        packs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" > "),
        real_issues
    );
}

#[test]
fn test_pipeline_companion_packs_with_airports() {
    // Mesh companion packs (EGLL_MESH, EGLL_3Dgrass) must not be elevated
    // above overlays even after sort.
    let model = BitNetModel::default();
    let ctx = PredictContext::default();

    let mut packs = vec![
        classified_pack("EGLL_Heathrow", &model, &ctx),
        classified_pack("EGLL_MESH", &model, &ctx),
        classified_pack("EGLL_3Dgrass", &model, &ctx),
        classified_pack("FlyTampa_Amsterdam_1_overlays", &model, &ctx),
        classified_pack("Global Airports", &model, &ctx),
        classified_pack("zzz_UHD_Mesh_v4", &model, &ctx),
    ];

    sort_packs(&mut packs, Some(&model), &ctx);

    let report = SceneryValidator::validate(&packs);
    let real_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type != "shadowed_mesh")
        .collect();

    assert!(
        real_issues.is_empty(),
        "Companion packs pipeline should produce no warnings.\n\
         Sorted order: {}\n\
         Issues: {:?}",
        packs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" > "),
        real_issues
    );
}

#[test]
fn test_pipeline_large_mixed_install() {
    // Large realistic install with everything thrown in.
    let model = BitNetModel::default();
    let ctx = PredictContext::default();

    let mut packs = vec![
        // Airports
        classified_pack("KSEA_Airport", &model, &ctx),
        classified_pack("EGLL_Heathrow", &model, &ctx),
        classified_pack("DarkBlue-RJTT_Haneda_Overlays1", &model, &ctx),
        classified_pack("JustSim_LOWW_Vienna", &model, &ctx),
        // Orbx
        classified_pack("Orbx_A_EGLC_LondonCity", &model, &ctx),
        classified_pack("Orbx_B_US_NorCal_TE_Overlay", &model, &ctx),
        classified_pack("Orbx_C_GB_South_TrueEarth_Orthos", &model, &ctx),
        // Global
        classified_pack("Global Airports", &model, &ctx),
        classified_pack("X-Plane Landmarks - London", &model, &ctx),
        // SimHeaven
        classified_pack("simHeaven_X-World_Europe-1-vfr", &model, &ctx),
        classified_pack("simHeaven_X-World_America-2-regions", &model, &ctx),
        // Libraries
        classified_pack("OpenSceneryX_Library", &model, &ctx),
        classified_pack("world-models", &model, &ctx),
        classified_pack("Sea_Life", &model, &ctx),
        // Fluff
        classified_pack("Global_Forests_v2", &model, &ctx),
        classified_pack("Shoreline_Objects", &model, &ctx),
        // XPME
        classified_pack("XPME_Overlays", &model, &ctx),
        classified_pack("XPME_South_America", &model, &ctx),
        // AutoOrtho
        classified_pack("yAutoOrtho_Overlays", &model, &ctx),
        classified_pack("z_ao_eur", &model, &ctx),
        // Airport Overlays
        classified_pack("FlyTampa_Amsterdam_1_overlays", &model, &ctx),
        classified_pack("FollowMe_Cars", &model, &ctx),
        // Companion & Mesh
        classified_pack("EGLL_MESH", &model, &ctx),
        classified_pack("EGLL_3Dgrass", &model, &ctx),
        classified_pack("zzz_UHD_Mesh_v4", &model, &ctx),
    ];

    sort_packs(&mut packs, Some(&model), &ctx);

    let report = SceneryValidator::validate(&packs);
    let real_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.issue_type != "shadowed_mesh")
        .collect();

    // Print the full sorted order for debugging
    println!("Sorted order:");
    for (i, p) in packs.iter().enumerate() {
        println!("  {}: {} ({:?})", i, p.name, p.category);
    }

    assert!(
        real_issues.is_empty(),
        "Large mixed install should produce no warnings.\n\
         Issues: {:?}",
        real_issues
    );
}
