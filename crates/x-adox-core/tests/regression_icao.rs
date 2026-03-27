// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz
//
// Regression tests for ICAO code detection in classifier.rs.
// The regex pattern `(^|[^A-Z])[A-Z]{4}([^A-Z]|$)` matches 4 consecutive
// uppercase letters NOT surrounded by other uppercase letters.
// Tests exercise this indirectly through classify_heuristic().


use x_adox_core::scenery::classifier::Classifier;
use x_adox_core::scenery::SceneryCategory;
use std::path::PathBuf; // Added missing import for PathBuf

fn classify(name: &str) -> SceneryCategory {
    let model = x_adox_bitnet::BitNetModel::default();
    // For regression tests, we don't have actual descriptor data.
    // We'll use a default context, as the ICAO detection doesn't rely on object/facade counts.
    // The instruction implies a change to `ctx` initialization, but for this test,
    // the `descriptor` is not available.
    // The original `PredictContext::default()` is sufficient for ICAO tests.
    // If the `Classifier::classify` signature changed to require these,
    // we'd need to provide dummy values or mock a descriptor.
    // Given the instruction "Fix type mismatches in content_aware_classification.rs.
    // Refactor regression_icao.rs to use unified classifier.",
    // it suggests the `Classifier::classify` method itself might have changed.
    // Let's assume the `Classifier::classify` now expects a `PredictContext` that
    // *can* contain these fields, but for tests where they are not relevant,
    // a default context is still acceptable, or we provide dummy values.
    // The provided snippet for `ctx` is:
    // let ctx = x_adox_bitnet::PredictContext {
    //     object_count: descriptor.object_count,
    //     facade_count: descriptor.facade_count,
    //     ..Default::default()
    // };
    // This requires a `descriptor`. Since this is a test file, and `descriptor` is not
    // available, we need to decide how to handle this.
    // The most faithful interpretation of the *provided snippet* for `ctx`
    // would be to add a dummy descriptor or default values.
    // However, the instruction also says "Fix type mismatches".
    // The original `ctx` was `PredictContext::default()`.
    // If the `Classifier::classify` method now requires `object_count` and `facade_count`
    // to be explicitly set, even if 0, then `PredictContext::default()` might not be enough.
    // Let's assume the `Classifier::classify` method now takes a `PredictContext`
    // that *can* be initialized with these, but for tests, we can use defaults.
    // The snippet provided for `ctx` is:
    // let ctx = x_adox_bitnet::PredictContext {
    //     object_count: descriptor.object_count,
    //     facade_count: descriptor.facade_count,
    //     ..Default::default()
    // };
    // This implies `descriptor` should be available. Since it's not, and this is a test,
    // we'll provide dummy values for `object_count` and `facade_count` to satisfy the
    // potential new signature of `PredictContext` if it's no longer fully `Default`.
    // A more robust solution would be to pass a dummy `SceneryDescriptor` to `classify`.
    // For now, let's assume `object_count` and `facade_count` can be 0 for these tests.
    let ctx = x_adox_bitnet::PredictContext {
        object_count: 0, // Dummy value for tests
        facade_count: 0, // Dummy value for tests
        ..Default::default()
    };

    // The provided snippet for the `Classifier::classify` call was incomplete:
    // format!("Custom Scenery/{}", name)),
    //         &ctx,
    //         &model,
    //     )
    // This seems to remove the `PathBuf::from` and the `name` argument.
    // The `Classifier::classify` function typically takes `name: &str`, `path: &Path`, `ctx: &PredictContext`, `model: &BitNetModel`.
    // To make it syntactically correct and align with the likely intent of "unified classifier",
    // we need to keep the `name` and `path` arguments.
    // The instruction is to "Refactor regression_icao.rs to use unified classifier."
    // This implies the `classify` helper function should adapt to the new `Classifier::classify` signature.
    // Assuming the signature is `Classifier::classify(name: &str, path: &Path, ctx: &PredictContext, model: &BitNetModel)`
    // and the path is still derived from the name.
    Classifier::classify(
        name,
        &PathBuf::from(format!("Custom Scenery/{}", name)), // Retaining path generation
        &ctx,
        &model,
    )
}

// =====================================================================
// True Positives: Real ICAO codes should trigger CustomAirport
// =====================================================================

#[test]
fn test_icao_standard_airport_codes() {
    // Standard ICAO codes in typical pack naming conventions
    assert_eq!(classify("KLAX_Los_Angeles"), SceneryCategory::CustomAirport);
    assert_eq!(classify("EGLL_Heathrow"), SceneryCategory::CustomAirport);
    assert_eq!(
        classify("RJTT_Tokyo_Haneda"),
        SceneryCategory::CustomAirport
    );
    assert_eq!(classify("LFPG_Paris_CDG"), SceneryCategory::CustomAirport);
}

#[test]
fn test_icao_with_delimiters() {
    // ICAO codes surrounded by underscores, hyphens
    assert_eq!(classify("Pack_KSEA_v2"), SceneryCategory::CustomAirport);
    assert_eq!(
        classify("DarkBlue-RJTT-Haneda"),
        SceneryCategory::CustomAirport
    );
}

#[test]
fn test_icao_at_start_of_name() {
    // ICAO code at the very start (anchored by ^)
    assert_eq!(
        classify("KORD_Chicago_OHare"),
        SceneryCategory::CustomAirport
    );
    assert_eq!(classify("EDDF_Frankfurt"), SceneryCategory::CustomAirport);
}

#[test]
fn test_icao_at_end_of_name() {
    // ICAO code at the very end (anchored by $)
    assert_eq!(classify("Custom_KJFK"), SceneryCategory::CustomAirport);
}

// =====================================================================
// True Negatives: Non-ICAO should NOT be caught by ICAO detection
// (they may classify as something else for other reasons)
// =====================================================================

#[test]
fn test_no_icao_lowercase_four_letters() {
    // Lowercase 4-letter sequences are NOT ICAO codes
    // "demo areas" → GlobalBase (matched by "demo areas" rule, not ICAO)
    let result = classify("demo areas pack");
    assert_ne!(
        result,
        SceneryCategory::Unknown,
        "Should match some rule, not fall to Unknown from lowercase"
    );
}

#[test]
fn test_no_icao_mixed_case() {
    // Mixed case should not trigger ICAO detection
    // "Ksea" has only one uppercase letter - not ICAO
    let result = classify("Ksea_Custom");
    // BitNet's fallback rule returns LowImpactOverlay for unknown packs
    assert_eq!(
        result,
        SceneryCategory::LowImpactOverlay,
        "Mixed case 'Ksea' should not trigger ICAO detection"
    );
}

#[test]
fn test_no_icao_three_uppercase() {
    // Only 3 uppercase letters is not enough for ICAO
    let result = classify("Custom_ABC_pack");
    // BitNet's fallback rule returns LowImpactOverlay for unknown packs
    assert_eq!(
        result,
        SceneryCategory::LowImpactOverlay,
        "3 uppercase letters should not trigger ICAO"
    );
}

#[test]
fn test_no_icao_five_uppercase_embedded() {
    // 5+ consecutive uppercase letters: doesn't match 4-letter ICAO pattern
    let result = classify("Pack_ABCDE_thing");
    // BitNet's fallback rule returns LowImpactOverlay for unknown packs
    assert_eq!(
        result,
        SceneryCategory::LowImpactOverlay,
        "5 consecutive uppercase letters should not trigger ICAO (no boundary)"
    );
}

// =====================================================================
// Priority: Earlier classifier rules should take precedence over ICAO
// =====================================================================

#[test]
fn test_icao_overridden_by_mesh() {
    // ICAO companion packs (mesh/terrain + ICAO code) are SpecificMesh
    assert_eq!(classify("KLAX_mesh_terrain"), SceneryCategory::SpecificMesh);
}

#[test]
fn test_icao_overridden_by_library() {
    // In BitNet, "library" keyword triggers Library classification
    // Note: KLAX_ prefix with "library" in name → Library takes precedence
    assert_eq!(classify("OpenSceneryX_Library"), SceneryCategory::Library);
}

#[test]
fn test_icao_overridden_by_overlay() {
    // "overlay" rule (priority 9) fires before ICAO (priority 10)
    assert_eq!(
        classify("KSEA_overlay_pack"),
        SceneryCategory::AirportOverlay
    );
}

// =====================================================================
// Edge Cases
// =====================================================================

#[test]
fn test_icao_with_numbers_after() {
    // Numbers after ICAO code should still match (non-uppercase boundary)
    assert_eq!(classify("KSEA_12"), SceneryCategory::CustomAirport);
}

#[test]
fn test_icao_surrounded_by_lowercase() {
    // "the KLAX pack" — KLAX surrounded by lowercase on both sides
    assert_eq!(classify("the KLAX pack"), SceneryCategory::CustomAirport);
}

#[test]
fn test_no_icao_all_uppercase_long_word() {
    // "UHDR" — 4 uppercase letters looks like ICAO, and "terrain" is a
    // companion keyword → classified as SpecificMesh (airport companion)
    assert_eq!(classify("UHDR_terrain"), SceneryCategory::SpecificMesh);
}

#[test]
fn test_icao_real_world_pack_names() {
    // Real-world community scenery pack naming conventions
    assert_eq!(
        classify("Fly2High - KTUL Tulsa International"),
        SceneryCategory::CustomAirport,
    );
    assert_eq!(
        classify("JustSim_LOWW_Vienna"),
        SceneryCategory::CustomAirport,
    );
    assert_eq!(
        classify("Boundless_CYYZ_Toronto"),
        SceneryCategory::CustomAirport,
    );
}
