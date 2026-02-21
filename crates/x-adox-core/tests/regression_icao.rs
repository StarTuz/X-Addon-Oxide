// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz
//
// Regression tests for ICAO code detection in classifier.rs.
// The regex pattern `(^|[^A-Z])[A-Z]{4}([^A-Z]|$)` matches 4 consecutive
// uppercase letters NOT surrounded by other uppercase letters.
// Tests exercise this indirectly through classify_heuristic().

use std::path::PathBuf;
use x_adox_core::scenery::classifier::Classifier;
use x_adox_core::scenery::SceneryCategory;

fn classify(name: &str) -> SceneryCategory {
    Classifier::classify_heuristic(&PathBuf::from(format!("Custom Scenery/{}", name)), name)
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
    // "Ksea" has only one uppercase letter
    let result = classify("Ksea_Custom");
    // This might match "airport" or something else, but NOT via ICAO
    // The key point: it should not match ICAO regex (K + sea is mixed case)
    // Without other keyword matches, this would be Unknown
    assert_eq!(
        result,
        SceneryCategory::Unknown,
        "Mixed case 'Ksea' should not trigger ICAO detection"
    );
}

#[test]
fn test_no_icao_three_uppercase() {
    // Only 3 uppercase letters is not enough
    let result = classify("Custom_ABC_pack");
    assert_eq!(
        result,
        SceneryCategory::Unknown,
        "3 uppercase letters should not trigger ICAO"
    );
}

#[test]
fn test_no_icao_five_uppercase_embedded() {
    // 5+ consecutive uppercase letters: the regex requires exactly 4 NOT surrounded by uppercase
    // "ABCDE" has A-B-C-D-E: ABCD is surrounded by E on the right → no match for ABCD
    // BCDE: B is preceded by A (uppercase) → no match
    let result = classify("Pack_ABCDE_thing");
    assert_eq!(
        result,
        SceneryCategory::Unknown,
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
    // "library" rule (priority 2) fires before ICAO
    assert_eq!(classify("KLAX_library_pack"), SceneryCategory::Library);
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
