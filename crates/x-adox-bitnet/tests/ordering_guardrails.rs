use std::path::Path;
use x_adox_bitnet::{BitNetModel, PredictContext};

#[test]
fn test_critical_scenery_ordering_pairs() {
    let mut model = BitNetModel::new().expect("Failed to initialize BitNetModel");
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
    ];

    let dummy_path = Path::new("/dummy/path");

    for (high_prio_name, low_prio_name, description) in critical_pairs {
        let score_high = model.predict(high_prio_name, dummy_path, &context);
        let score_low = model.predict(low_prio_name, dummy_path, &context);

        println!(
            "Testing '{}':\n  {} -> Score {}\n  {} -> Score {}",
            description, high_prio_name, score_high, low_prio_name, score_low
        );

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
