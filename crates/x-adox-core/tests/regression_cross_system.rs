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
        ("XPME_Overlays", SceneryCategory::AutoOrthoOverlay),  // Photo overlay tier (score 48), analogous to yAutoOrtho_Overlays
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
        ("Europe_Birds_Birdofprey500m_A2", SceneryCategory::RegionalFluff),
        ("Asia_Birds_Pigeon_Pack", SceneryCategory::RegionalFluff),

        // --- Ortho/Base ---
        ("XPME_Europe", SceneryCategory::OrthoBase),
        ("z_ao_eur", SceneryCategory::OrthoBase),
        ("z_autoortho", SceneryCategory::OrthoBase),

        // --- Mesh ---
        ("zzz_UHD_Mesh_v4", SceneryCategory::Mesh),

        // --- Specific Mesh (airport companions, Orbx D mesh, FlyTampa mesh) ---
        ("FlyTampa_Amsterdam_3_mesh", SceneryCategory::SpecificMesh),
        ("Orbx_D_GB_South_TrueEarth_Mesh", SceneryCategory::SpecificMesh),
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
    let model = BitNetModel::new().unwrap();
    let ctx = PredictContext::default();
    // Verify our corpus expectations are correct for the classifier.
    for (name, expected) in test_corpus() {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let result = Classifier::classify(name, &path, &ctx, &model);
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
        let category = Classifier::classify(name, &path, &ctx, &model);
        let (score, _, rule) = model.predict_with_rule_name(name, &path, &ctx);

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
        let category = Classifier::classify(name, &path, &ctx, &model);

        if overlay_categories.contains(&category) {
            let (score, _, rule) = model.predict_with_rule_name(name, &path, &ctx);
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
        let category = Classifier::classify(name, &path, &ctx, &model);

        if base_categories.contains(&category) {
            let (score, _, rule) = model.predict_with_rule_name(name, &path, &ctx);
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

// =====================================================================
// INI Coherence Tests - Prevent Scrambled Output
// =====================================================================

/// CRITICAL: Verifies that INI section headers never repeat.
/// This catches the bug where the INI writer used different context than the sorter,
/// causing the same header to appear multiple times.
#[test]
fn test_ini_sections_never_repeat() {
    let model = BitNetModel::new().unwrap();
    let ctx = PredictContext::default();

    // Collect (score, rule_name) for all corpus packs
    let mut score_rules: Vec<(u8, String)> = test_corpus()
        .iter()
        .map(|(name, _)| {
            let path = PathBuf::from(format!("Custom Scenery/{}", name));
            let (score, _, rule) = model.predict_with_rule_name(name, &path, &ctx);
            (score, rule)
        })
        .collect();

    // Sort by (score, rule_name) - mimics sorter behavior
    // The sorter uses canonical_section_name for grouping, but the INI writer
    // should now use the actual rule name (after the fix)
    score_rules.sort_by(|(score_a, rule_a), (score_b, rule_b)| {
        match score_a.cmp(score_b) {
            std::cmp::Ordering::Equal => rule_a.cmp(rule_b),
            ord => ord,
        }
    });

    // Track section headers as they would appear in INI
    let mut seen_sections: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut last_section = String::new();
    let mut repeated_sections = Vec::new();

    for (_score, rule) in &score_rules {
        if rule != &last_section {
            if seen_sections.contains(rule) {
                // Section reappeared after being used earlier - this is the scramble bug
                repeated_sections.push(rule.clone());
            }
            seen_sections.insert(rule.clone());
            last_section = rule.clone();
        }
    }

    assert!(
        repeated_sections.is_empty(),
        "INI sections would repeat (scrambled output): {:?}\n\
         This happens when the sorter and INI writer disagree on grouping.\n\
         Full order: {:?}",
        repeated_sections,
        score_rules
    );
}

/// Verifies that packs with the same rule name get the same score.
/// If they don't, sections will be fragmented in the INI.
#[test]
fn test_same_rule_same_score() {
    let model = BitNetModel::new().unwrap();
    let ctx = PredictContext::default();

    // Group packs by rule name and check scores are consistent
    let mut rule_scores: std::collections::HashMap<String, Vec<(String, u8)>> =
        std::collections::HashMap::new();

    for (name, _) in test_corpus() {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let (score, _, rule) = model.predict_with_rule_name(name, &path, &ctx);
        rule_scores
            .entry(rule)
            .or_default()
            .push((name.to_string(), score));
    }

    let mut inconsistencies = Vec::new();
    for (rule, packs) in &rule_scores {
        if packs.len() > 1 {
            let first_score = packs[0].1;
            for (name, score) in packs.iter().skip(1) {
                if *score != first_score {
                    inconsistencies.push(format!(
                        "Rule '{}': '{}' has score {} but '{}' has score {}",
                        rule, packs[0].0, first_score, name, score
                    ));
                }
            }
        }
    }

    assert!(
        inconsistencies.is_empty(),
        "Packs matching the same rule have different scores (causes INI fragmentation):\n{}",
        inconsistencies.join("\n")
    );
}

/// Verifies that RegionalFluff packs are NOT healed to other categories
/// regardless of object count. This catches the bird pack bug.
#[test]
fn test_regional_fluff_not_healed_with_objects() {
    let model = BitNetModel::new().unwrap();

    // Context with many objects (would trigger urban enhancement healing)
    let ctx_with_objects = PredictContext {
        object_count: 100,
        ..Default::default()
    };

    let fluff_packs = [
        "Global_Forests_v2",
        "Shoreline_Objects",
        "Europe_Birds_Birdofprey500m_A2",
        "Asia_Birds_Pigeon_Pack",
    ];

    for name in fluff_packs {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let (_, cat, rule) = model.predict_with_rule_name(name, &path, &ctx_with_objects);

        assert_eq!(
            cat,
            SceneryCategory::RegionalFluff,
            "'{}' with high object count should stay RegionalFluff, not be healed to {:?} (rule='{}')",
            name, cat, rule
        );
    }
}

/// CRITICAL: Verifies that context variations don't change rule assignment.
/// This catches the bug where INI writer used partial context while sorter
/// used full context, causing mismatched section headers.
#[test]
fn test_context_does_not_change_rule_for_named_packs() {
    let model = BitNetModel::new().unwrap();

    // Various context configurations that might trigger structural healing
    let contexts = [
        ("default", PredictContext::default()),
        (
            "with_objects",
            PredictContext {
                object_count: 100,
                facade_count: 50,
                ..Default::default()
            },
        ),
        (
            "with_tiles",
            PredictContext {
                has_tiles: true,
                ..Default::default()
            },
        ),
        (
            "full_context",
            PredictContext {
                has_airports: false,
                has_tiles: true,
                object_count: 200,
                facade_count: 100,
                has_airport_properties: true,
                ..Default::default()
            },
        ),
    ];

    // Named packs should always match the same rule regardless of context
    // (structural healing should NOT change the rule for packs that matched a keyword)
    for (name, expected_cat) in test_corpus() {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));

        // Get baseline rule with default context
        let (_, _, baseline_rule) = model.predict_with_rule_name(name, &path, &contexts[0].1);

        // Verify all other contexts produce the same rule
        for (ctx_name, ctx) in &contexts[1..] {
            let (_, _, rule) = model.predict_with_rule_name(name, &path, ctx);

            // Allow score changes but rule should be consistent for named packs
            // Exception: packs that fall through to structural healing rules
            let is_structural_rule = rule.contains("Healed")
                || rule == "Other Scenery"
                || rule == "Mesh/Foundation";
            let baseline_is_structural = baseline_rule.contains("Healed")
                || baseline_rule == "Other Scenery"
                || baseline_rule == "Mesh/Foundation";

            if !is_structural_rule && !baseline_is_structural {
                assert_eq!(
                    rule, baseline_rule,
                    "'{}' changed rule from '{}' to '{}' with {} context. \
                     Expected category: {:?}",
                    name, baseline_rule, rule, ctx_name, expected_cat
                );
            }
        }
    }
}
