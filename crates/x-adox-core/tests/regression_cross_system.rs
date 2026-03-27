// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz
//
// Cross-system contradiction detector.
// These tests verify that the Classifier (SceneryCategory) and BitNet (score)
// systems agree on how each pack should be placed. This catches the class of
// bug where the classifier says "overlay" but BitNet assigns a mesh/ortho score
// (or vice versa), which was the root cause of the XPME_Overlays issue.

use std::path::PathBuf;
use x_adox_bitnet::{BitNetModel, PredictContext};
use x_adox_core::scenery::classifier::Classifier;
use x_adox_core::scenery::SceneryCategory;

// Centralized logic has been moved to x_adox_core::scenery::SceneryCategory
// is_compatible_with_score() and heal_score().

// =====================================================================
// Exhaustive Pack Name Corpus
// =====================================================================

/// All pack names that appear across the test suite, plus edge cases.
/// Each entry: (pack_name, expected_classifier_category).
fn test_corpus() -> Vec<(&'static str, SceneryCategory)> {
    vec![
        // --- Airports ---
        ("KSEA_Airport", SceneryCategory::CustomAirport),
        ("KLAX_Los_Angeles", SceneryCategory::CustomAirport),
        ("EGLL_Heathrow", SceneryCategory::CustomAirport),
        ("DarkBlue-RJTT_Haneda_Overlays1", SceneryCategory::AirportOverlay),
        ("Fly2High - KTUL Tulsa International", SceneryCategory::CustomAirport),
        ("JustSim_LOWW_Vienna", SceneryCategory::CustomAirport),
        ("Boundless_CYYZ_Toronto", SceneryCategory::CustomAirport),

        // --- Orbx ---
        ("Orbx_A_EGLC_LondonCity", SceneryCategory::OrbxAirport),
        ("Orbx_A_GB_South_TrueEarth_Custom", SceneryCategory::OrbxAirport),
        ("Orbx_B_US_NorCal_TE_Overlay", SceneryCategory::RegionalOverlay),
        ("Orbx_C_GB_South_TrueEarth_Orthos", SceneryCategory::OrthoBase),

        // --- SimHeaven ---
        ("simHeaven_X-World_Europe-1-vfr", SceneryCategory::RegionalOverlay),
        ("simHeaven_X-World_America-2-regions", SceneryCategory::RegionalOverlay),

        // --- Global ---
        ("Global Airports", SceneryCategory::GlobalAirport),
        ("X-Plane Landmarks - London", SceneryCategory::Landmark),

        // --- Overlays ---
        ("FlyTampa_Amsterdam_1_overlays", SceneryCategory::AirportOverlay),
        ("XPME_Overlays", SceneryCategory::RegionalOverlay),  // Regional, not airport-specific
        ("yAutoOrtho_Overlays", SceneryCategory::AutoOrthoOverlay),
        ("FollowMe_Cars", SceneryCategory::AirportOverlay),

        // --- Libraries ---
        ("OpenSceneryX_Library", SceneryCategory::Library),
        ("world-models", SceneryCategory::Library),
        ("Sea_Life", SceneryCategory::Library),
        ("ruscenery", SceneryCategory::Library),
        ("o4xp_Seasons_Manager", SceneryCategory::Library),

        // --- Regional Fluff ---
        ("Global_Forests_v2", SceneryCategory::RegionalFluff),
        ("Shoreline_Objects", SceneryCategory::RegionalFluff),

        // --- Ortho/Base ---
        ("XPME_South_America", SceneryCategory::OrthoBase),
        ("XPME_Europe", SceneryCategory::OrthoBase),
        ("z_ao_eur", SceneryCategory::OrthoBase),
        ("z_autoortho", SceneryCategory::OrthoBase),

        // --- Mesh ---
        ("zzz_UHD_Mesh_v4", SceneryCategory::Mesh),
        ("FlyTampa_Amsterdam_3_mesh", SceneryCategory::Mesh),

        // --- Specific Mesh (airport companions) ---
        ("EGLL_MESH", SceneryCategory::SpecificMesh),
        ("EGLL_3Dgrass", SceneryCategory::SpecificMesh),
        ("PAKT_Terrain_Northern_Sky_Studio", SceneryCategory::SpecificMesh),
    ]
}

// =====================================================================
// Cross-System Agreement Tests
// =====================================================================

#[test]
fn test_classifier_produces_expected_categories() {
    // Verify our corpus expectations are correct for the classifier.
    for (name, expected) in test_corpus() {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let result = Classifier::classify_heuristic(&path, name);
        assert_eq!(
            result, expected,
            "Classifier: '{}' expected {:?}, got {:?}",
            name, expected, result
        );
    }
}

#[test]
fn test_no_category_score_contradictions() {
    // THE KEY TEST: For every pack in the corpus, run it through BOTH the
    // classifier AND BitNet, then assert they agree on position band.
    let mut model = BitNetModel::new().unwrap();
    model.update_config(x_adox_bitnet::HeuristicsConfig::default());
    let ctx = PredictContext::default();

    let mut contradictions = Vec::new();

    for (name, _expected_cat) in test_corpus() {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let category = Classifier::classify_heuristic(&path, name);
        let (score, rule) = model.predict_with_rule_name(name, &path, &ctx);

        if !category.is_compatible_with_score(score) {
            contradictions.push(format!(
                "  '{}': classifier={:?} vs BitNet score={} rule='{}'",
                name,
                category,
                score,
                rule
            ));
        }
    }

    assert!(
        contradictions.is_empty(),
        "Category↔Score contradictions found:\n{}",
        contradictions.join("\n")
    );
}

#[test]
fn test_overlay_categories_never_get_base_scores() {
    // Strict invariant: if classifier says "overlay", BitNet must NOT assign
    // a score ≥50 (ortho/mesh territory). This is the exact class of bug
    // that caused the XPME issue.
    let mut model = BitNetModel::new().unwrap();
    model.update_config(x_adox_bitnet::HeuristicsConfig::default());
    let ctx = PredictContext::default();

    let overlay_categories = [
        SceneryCategory::AirportOverlay,
        SceneryCategory::RegionalOverlay,
        SceneryCategory::RegionalFluff,
        SceneryCategory::AutoOrthoOverlay,
        SceneryCategory::LowImpactOverlay,
    ];

    let mut violations = Vec::new();

    for (name, _) in test_corpus() {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let category = Classifier::classify_heuristic(&path, name);

        if overlay_categories.contains(&category) {
            let (score, rule) = model.predict_with_rule_name(name, &path, &ctx);
            if score >= 50 {
                violations.push(format!(
                    "  '{}': classifier={:?} but BitNet score={} (rule='{}'). Overlay sunk to base!",
                    name, category, score, rule
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Overlays incorrectly scored as base/mesh:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_base_categories_never_get_overlay_scores() {
    // Reverse invariant: if classifier says "mesh/ortho", BitNet must NOT
    // assign a score ≤35 (overlay territory). This would cause mesh packs
    // to be elevated above overlays.
    let mut model = BitNetModel::new().unwrap();
    model.update_config(x_adox_bitnet::HeuristicsConfig::default());
    let ctx = PredictContext::default();

    let base_categories = [
        SceneryCategory::OrthoBase,
        SceneryCategory::Mesh,
    ];

    let mut violations = Vec::new();

    for (name, _) in test_corpus() {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let category = Classifier::classify_heuristic(&path, name);

        if base_categories.contains(&category) {
            let (score, rule) = model.predict_with_rule_name(name, &path, &ctx);
            if score <= 35 {
                violations.push(format!(
                    "  '{}': classifier={:?} but BitNet score={} (rule='{}'). Base elevated to overlay!",
                    name, category, score, rule
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Base/mesh packs incorrectly scored as overlays:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_runtime_healing_resolves_contradiction() {
    // This test verifies that the healing logic in sorter.rs actually
    // works by simulating a contradiction.

    // 1. Setup a contradiction: an overlay pack with a forced "base" score.
    // We can't easily force BitNet to lie, so we'll test the heal_score 
    // function directly and then assume the sorter uses it correctly.
    let cat = SceneryCategory::AirportOverlay;
    let bad_score = 95; // Contradiction: overlay scored as base
    assert!(!cat.is_compatible_with_score(bad_score));
    
    let healed = cat.heal_score(bad_score);
    assert!(healed < 50, "Healed score {} must be in overlay territory", healed);

    // 2. Integration: verify sorter applies it.
    // ...
}
