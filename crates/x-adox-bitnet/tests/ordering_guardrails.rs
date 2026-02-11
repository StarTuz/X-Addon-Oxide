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
            "simHeaven_X-World_Europe",
            "Global Airports",
            "SimHeaven must be above Global Airports (community standard)",
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
        (
            "simHeaven_Vegetation_Library",
            "Global Airports",
            "Vegetation Libraries must be above Global Airports (community standard)",
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
        // Orbx A sub-ordering: location-specific packs must be above regional TrueEarth packs
        (
            "Orbx_A_EGLC_LondonCity",
            "Orbx_A_GB_South_TrueEarth_Custom",
            "Orbx A airport (EGLC) must be above Orbx A regional (TrueEarth)",
        ),
        (
            "Orbx_A_YBBNv2_Brisbane",
            "Orbx_A_GB_South_TrueEarth_Custom",
            "Orbx A airport (YBBN Brisbane) must be above Orbx A regional (TrueEarth)",
        ),
        (
            "Orbx_A_Brisbane_Landmarks",
            "Orbx_A_GB_Central_TrueEarth_Custom",
            "Orbx A landmarks (Brisbane) must be above Orbx A regional (TrueEarth)",
        ),
        // Regression: Airport packs with city-name keywords must not be demoted
        (
            "EGLL_LONDON_TAIMODELS",
            "Global Airports",
            "EGLL airport must be above Global Airports (city keyword 'london' must not demote it)",
        ),
        // Regression: Mesh/terrain companion packs must NOT be classified as airports
        (
            "Global Airports",
            "EGLL_MESH",
            "EGLL mesh must be below Global Airports (companion pack, not an airport)",
        ),
        (
            "Global Airports",
            "PAKT_Terrain_Northern_Sky_Studio",
            "PAKT terrain must be below Global Airports (companion pack)",
        ),
        (
            "Global Airports",
            "SFD_KLAX_Los_Angeles_HD_2_Mesh",
            "KLAX mesh must be below Global Airports (companion pack)",
        ),
        (
            "Global Airports",
            "EGLL_3Dgrass",
            "EGLL grass must be below Global Airports (companion pack)",
        ),
        // Airport Overlays must be above Global Airports for correct X-Plane rendering
        (
            "FlyTampa_Amsterdam_1_overlays",
            "Global Airports",
            "Airport overlays must be above Global Airports",
        ),
        (
            "DarkBlue-RJTT_Haneda_Overlays1",
            "Global Airports",
            "Airport overlays (Haneda) must be above Global Airports",
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
        score,
        rule
    );
}

#[test]
fn test_orbx_a_location_specific_above_regional() {
    // Location-specific Orbx A packs (airports, landmarks) → score 11, "Orbx A Airport"
    // Regional TrueEarth packs → score 12, "Orbx A Custom"
    // Detection: anything WITHOUT "trueearth" or "_te_" in name gets promoted.
    let mut model = BitNetModel::new().unwrap();
    model.update_config(HeuristicsConfig::default());

    let dummy_path = Path::new("/dummy/path");
    let context = PredictContext::default();

    // --- Location-specific packs (score 11) ---

    // Airport with clean ICAO
    let (score, rule) =
        model.predict_with_rule_name("Orbx_A_EGLC_LondonCity", dummy_path, &context);
    assert_eq!(score, 11, "EGLC should score 11");
    assert_eq!(rule, "Orbx A Airport");

    // Airport with version-suffixed ICAO (YBBNv2)
    let (score, rule) =
        model.predict_with_rule_name("Orbx_A_YBBNv2_Brisbane", dummy_path, &context);
    assert_eq!(score, 11, "YBBN Brisbane should score 11");
    assert_eq!(rule, "Orbx A Airport");

    // City landmarks pack (no ICAO, no TrueEarth)
    let (score, rule) =
        model.predict_with_rule_name("Orbx_A_Brisbane_Landmarks", dummy_path, &context);
    assert_eq!(score, 11, "Brisbane Landmarks should score 11");
    assert_eq!(rule, "Orbx A Airport");

    // --- Regional TrueEarth packs (score 12) ---

    let (score, rule) =
        model.predict_with_rule_name("Orbx_A_GB_South_TrueEarth_Custom", dummy_path, &context);
    assert_eq!(score, 12, "GB South TrueEarth should score 12");
    assert_eq!(rule, "Orbx A Custom");

    let (score, rule) =
        model.predict_with_rule_name("Orbx_A_GB_South_TrueEarth_Airports", dummy_path, &context);
    assert_eq!(score, 12, "GB South TrueEarth Airports should score 12");
    assert_eq!(rule, "Orbx A Custom");

    // US pack with _TE_ abbreviation
    let (score, rule) =
        model.predict_with_rule_name("Orbx_A_US_NorCal_TE_Custom", dummy_path, &context);
    assert_eq!(score, 12, "US NorCal TE should score 12");
    assert_eq!(rule, "Orbx A Custom");
}

#[test]
fn test_global_airports_guard_uses_rule_name_not_score() {
    // Regression: The airport override guard previously used `s != 25` to exclude
    // Global Airports. This failed when the user's heuristics.json had old scores
    // (Global Airports=20, City Enhancements=25), causing:
    //   - Global Airports (score 20, 20 != 25 → true) → overridden to 10 (WRONG)
    //   - EGLL_LONDON (score 25 via old City Enhancements, 25 != 25 → false) → stays 25 (WRONG)
    // Fix: Check rule NAME ("Global Airports") instead of score value.

    let dummy_path = Path::new("/dummy/path");
    let context = PredictContext::default();

    // Test with DEFAULT (current) config
    let mut model = BitNetModel::new().unwrap();
    model.update_config(HeuristicsConfig::default());

    let (ga_score, ga_rule) =
        model.predict_with_rule_name("Global Airports", dummy_path, &context);
    assert_eq!(ga_score, 25, "Global Airports must keep its score (25)");
    assert_eq!(ga_rule, "Global Airports", "Rule name must be 'Global Airports'");

    let (egll_score, egll_rule) =
        model.predict_with_rule_name("EGLL_LONDON_TAIMODELS", dummy_path, &context);
    assert_eq!(egll_score, 10, "EGLL must be promoted to airport priority (10)");
    assert_eq!(egll_rule, "Airports", "EGLL must be overridden to 'Airports'");

    // Test with SIMULATED OLD config (Global Airports=20, City Enhancements=25)
    let mut old_config = HeuristicsConfig::default();
    for rule in &mut old_config.rules {
        match rule.name.as_str() {
            "Global Airports" => rule.score = 20,
            "City Enhancements" => rule.score = 25,
            _ => {}
        }
    }
    model.update_config(old_config);

    let (ga_score, ga_rule) =
        model.predict_with_rule_name("Global Airports", dummy_path, &context);
    assert_eq!(ga_score, 20, "Global Airports must keep its old score (20)");
    assert_eq!(ga_rule, "Global Airports", "Rule name must remain 'Global Airports'");

    let (egll_score, egll_rule) =
        model.predict_with_rule_name("EGLL_LONDON_TAIMODELS", dummy_path, &context);
    assert_eq!(egll_score, 10, "EGLL must still be promoted even with old config");
    assert_eq!(egll_rule, "Airports", "EGLL must be overridden to 'Airports'");
}
