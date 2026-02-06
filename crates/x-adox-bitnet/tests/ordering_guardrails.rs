// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use std::path::Path;
use x_adox_bitnet::{BitNetModel, HeuristicsConfig, PredictContext};

#[test]
fn test_critical_scenery_ordering_pairs() {
    let mut model = BitNetModel::new().unwrap();
    // CRITICAL: Force load the defaults from the CODE, ignoring any local heuristics.json file.
    // This ensures the test validates the shipped logic, not the developer's local state.
    model.update_config(x_adox_bitnet::HeuristicsConfig::default());

    let context = PredictContext::default();

    // Define Critical Pairs: (Higher Priority Name, Lower Priority Name, Test Description)
    // NOTE: Lower Score = Higher Priority
    // Expected: score(p1) < score(p2)
    let critical_pairs = vec![
        (
            "Custom Airport KSEA",
            "Global Airports",
            "Custom Airports must be above Global Airports",
        ),
        // CRITICAL: X-Plane's official landmarks must be ABOVE Global Airports
        // These are enhancement packs that should override default airport scenery
        (
            "X-Plane Landmarks - London",
            "Global Airports",
            "X-Plane Landmarks must be above Global Airports (official enhancement packs)",
        ),
        (
            "X-Plane Landmarks - New York",
            "Global Airports",
            "X-Plane Landmarks must be above Global Airports",
        ),
        (
            "Global Airports",
            "simHeaven_X-World_Europe",
            "Global Airports must be above SimHeaven (Corrective requirement)",
        ),
        (
            "simHeaven_X-World_America",
            "yOrtho4XP_Overlays",
            "SimHeaven must be above Generic Overlays (if using yOrtho naming)",
        ),
        (
            "simHeaven_X-World_Europe",
            "yOrtho4XP_+40-080",
            "Overlays must be above Ortho",
        ),
        (
            "yOrtho4XP_+40-080",
            "zzz_UHD_Mesh_V4",
            "Ortho must be above Mesh",
        ),
        // Additional Regression Checks
        (
            "Global Airports",
            "simHeaven_Vegetation_Library",
            "Global Airports must be above Vegetation Libraries (scored as Overlay/Library)",
        ),
        // Orbx & Global Forests Checks
        (
            "simHeaven_X-World_Europe",
            "Global_Forests_v2",
            "SimHeaven must be above Global Forests (SimHeaven forests take priority)",
        ),
        (
            "Orbx_A_US_NorCal_TE_Custom",
            "Global Airports",
            "Orbx Custom/Airports must be Top Priority (above Global Airports)",
        ),
        (
            "Orbx_B_US_NorCal_TE_Overlay",
            "simHeaven_X-World_Europe",
            "Orbx TrueEarth Overlays must be above SimHeaven (for UK priority)",
        ),
        (
            "Orbx_A_US_NorCal_TE_Custom",
            "simHeaven_X-World_Europe",
            "Orbx Custom must be above SimHeaven",
        ),
        // User Reported Regression: Overlays (FlyTampa/DarkBlue) must be ABOVE Orthos/Mesh (Orbx C/D)
        (
            "FlyTampa_Amsterdam_1_overlays",
            "Orbx_C_GB_Central_TrueEarth_Orthos",
            "Manufacturer Overlays must be above Orbx Orthos",
        ),
        (
            "DarkBlue-RJTT_Haneda_Overlays1",
            "Orbx_D_GB_North_TrueEarth_Orthos",
            "Manufacturer Overlays must be above Orbx Mesh/Ortho",
        ),
        (
            "Riga Latvija",
            "simHeaven_X-World_Europe",
            "Regional enhancements (Riga) must be above Generic Overlays (SimHeaven)",
        ),
        (
            "Orbx_A_EGLC_LondonCity",
            "Global Airports",
            "Orbx EGLC must be Top Priority (Airports) above Global (Precedence Bug Check)",
        ),
        (
            "Global Airports",
            "FlyTampa_Amsterdam_3_mesh",
            "User Regression: FlyTampa Mesh must NOT be promoted to Top Priority (remains below Global Airports)",
        ),
        // VFR-Objects must be above Mesh/Terrain
        (
            "VFR-Objects_GK_+47+007__+48+007_D_Schwarzwald",
            "FlyTampa_Amsterdam_3_mesh",
            "VFR-Objects must be above mesh packs (regional fluff, not terrain)",
        ),
        // Shoreline must be above Mesh
        (
            "Shoreline_Objects",
            "FlyTampa_Amsterdam_3_mesh",
            "Shoreline packs must be above mesh (regional fluff)",
        ),
        // Orbx A sub-ordering: airport-specific packs must be above regional TrueEarth packs
        (
            "Orbx_A_EGLC_LondonCity",
            "Orbx_A_GB_South_TrueEarth_Custom",
            "Orbx A airport-specific (EGLC) must be above Orbx A regional (TrueEarth)",
        ),
    ];

    let dummy_path = Path::new("/dummy/path");

    for (high_prio_name, low_prio_name, description) in critical_pairs {
        let (score_high, rule_high) =
            model.predict_with_rule_name(high_prio_name, dummy_path, &context);
        let (score_low, rule_low) =
            model.predict_with_rule_name(low_prio_name, dummy_path, &context);

        println!(
            "Testing '{}':\n  {} -> Score {} ({})\n  {} -> Score {} ({})",
            description, high_prio_name, score_high, rule_high, low_prio_name, score_low, rule_low
        );

        // Verification for specific Orbx GB North labeling
        if low_prio_name == "Orbx_D_GB_North_TrueEarth_Orthos" {
            assert_eq!(
                rule_low, "Orbx TrueEarth Orthos",
                "GB North should be labeled as Ortho, not Mesh"
            );
            assert_eq!(score_low, 58, "GB North Ortho should have score 58");
        }

        assert!(
            score_high < score_low,
            "FAILED: {}\n  '{}' (Score {}) should be < '{}' (Score {})",
            description,
            high_prio_name,
            score_high,
            low_prio_name,
            score_low
        );
    }
}

#[test]
fn test_vfr_objects_not_healed_to_mesh() {
    // VFR-Objects packs with tiles should NOT be scored as "Mesh/Terrain (Healed)".
    // They should be protected by the is_protected_overlay whitelist.
    let mut model = BitNetModel::new().unwrap();
    model.update_config(HeuristicsConfig::default());

    let dummy_path = Path::new("/dummy/path");

    // Context: pack has tiles but no airports (the healing trigger)
    let context = PredictContext {
        has_airports: false,
        has_tiles: true,
        ..Default::default()
    };

    let vfr_cases = [
        "VFR-Objects_GK_+47+007__+48+007_D_Schwarzwald West  4.1",
        "VFR_Objects_Europe_Central",
        "Shoreline_Objects",
    ];

    for name in &vfr_cases {
        let (score, rule) = model.predict_with_rule_name(name, dummy_path, &context);
        assert_ne!(
            rule, "Mesh/Terrain (Healed)",
            "'{}' should NOT fall to Mesh/Terrain (Healed), got score {} ({})",
            name, score, rule
        );
        assert!(
            score < 60,
            "'{}' score {} should be below mesh threshold (60)",
            name,
            score
        );
    }
}

#[test]
fn test_orbx_b_mesh_scored_above_mesh_tier() {
    // Orbx_B_EGLC_LondonCity_Mesh must NOT be scored as Mesh/Foundation (60).
    // It matches "london" → City Enhancements (25), which is correct — it's an
    // airport-area product, not a standalone terrain mesh.
    let mut model = BitNetModel::new().unwrap();
    model.update_config(HeuristicsConfig::default());

    let dummy_path = Path::new("/dummy/path");
    let context = PredictContext::default();

    let (score, rule) =
        model.predict_with_rule_name("Orbx_B_EGLC_LondonCity_Mesh", dummy_path, &context);
    assert_ne!(
        rule, "Mesh/Foundation",
        "Orbx B mesh should NOT match Mesh/Foundation rule, got '{}' (score {})",
        rule, score
    );
    assert!(
        score < 60,
        "Orbx B mesh should score well above mesh tier (60), got {} ({})",
        score, rule
    );
}

#[test]
fn test_orbx_a_airport_specific_above_regional() {
    // Orbx_A_EGLC_LondonCity has an ICAO code (EGLC) → score 11, "Orbx A Airport"
    // Orbx_A_GB_South_TrueEarth_Custom is regional → score 12, "Orbx A Custom"
    // This ensures airport-specific packs override regional TrueEarth exclusion zones.
    let mut model = BitNetModel::new().unwrap();
    model.update_config(HeuristicsConfig::default());

    let dummy_path = Path::new("/dummy/path");
    let context = PredictContext::default();

    // Airport-specific Orbx A pack
    let (score_eglc, rule_eglc) =
        model.predict_with_rule_name("Orbx_A_EGLC_LondonCity", dummy_path, &context);
    assert_eq!(score_eglc, 11, "Orbx A airport-specific should score 11");
    assert_eq!(
        rule_eglc, "Orbx A Airport",
        "Orbx A with ICAO should be labeled 'Orbx A Airport'"
    );

    // Regional Orbx A pack (no ICAO code)
    let (score_regional, rule_regional) =
        model.predict_with_rule_name("Orbx_A_GB_South_TrueEarth_Custom", dummy_path, &context);
    assert_eq!(score_regional, 12, "Orbx A regional should score 12");
    assert_eq!(
        rule_regional, "Orbx A Custom",
        "Orbx A without ICAO should be labeled 'Orbx A Custom'"
    );

    // Another regional pack (US)
    let (score_us, rule_us) =
        model.predict_with_rule_name("Orbx_A_US_NorCal_TE_Custom", dummy_path, &context);
    assert_eq!(score_us, 12, "Orbx A US regional should score 12");
    assert_eq!(rule_us, "Orbx A Custom");
}
