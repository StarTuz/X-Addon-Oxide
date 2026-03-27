// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use x_adox_core::scenery::classifier::Classifier;
use x_adox_core::scenery::{SceneryCategory, SceneryDescriptor};

#[test]
fn test_urban_promotion() {
    let model = x_adox_bitnet::BitNetModel::default();
    let descriptor = SceneryDescriptor {
        object_count: 100, // Many buildings
        facade_count: 50,
        ..Default::default()
    };

    let ctx = x_adox_bitnet::PredictContext {
        object_count: descriptor.object_count,
        facade_count: descriptor.facade_count,
        ..Default::default()
    };

    // Even if initially Unknown or RegionalOverlay, it should become a Landmark
    let cat = Classifier::classify(
        "Unknown",
        &std::path::PathBuf::from("Custom Scenery/Unknown"),
        &ctx,
        &model,
    );

    assert_eq!(cat, SceneryCategory::Landmark);
}

#[test]
fn test_ortho_demotion() {
    let model = x_adox_bitnet::BitNetModel::default();
    let descriptor = SceneryDescriptor {
        polygon_count: 5000, // Many mesh points
        object_count: 0,     // No buildings
        facade_count: 0,
        ..Default::default()
    };

    let ctx = x_adox_bitnet::PredictContext {
        has_tiles: true,
        object_count: descriptor.object_count,
        facade_count: descriptor.facade_count,
        ..Default::default()
    };

    let cat = Classifier::classify(
        "Unknown",
        &std::path::PathBuf::from("Custom Scenery/Unknown"),
        &ctx,
        &model,
    );

    assert_eq!(cat, SceneryCategory::OrthoBase);
}

#[test]
fn test_airport_overlay_promotion() {
    let model = x_adox_bitnet::BitNetModel::default();
    let descriptor = SceneryDescriptor {
        has_airport_properties: true,
        ..Default::default()
    };

    let ctx = x_adox_bitnet::PredictContext {
        has_airport_properties: descriptor.has_airport_properties,
        ..Default::default()
    };

    let cat = Classifier::classify(
        "Unknown",
        &std::path::PathBuf::from("Custom Scenery/Unknown"),
        &ctx,
        &model,
    );

    assert_eq!(cat, SceneryCategory::AirportOverlay);
}

#[test]
fn test_protected_stays_protected() {
    let model = x_adox_bitnet::BitNetModel::default();
    let descriptor = SceneryDescriptor {
        object_count: 0,
        polygon_count: 0,
        ..Default::default()
    };

    let ctx = x_adox_bitnet::PredictContext {
        object_count: descriptor.object_count,
        facade_count: descriptor.facade_count,
        ..Default::default()
    };

    // SimHeaven etc. should not be demoted to Mesh even if the peeker fails or finds zero data
    let cat = Classifier::classify(
        "simHeaven_X-Europe_1_Models",
        &std::path::PathBuf::from("Custom Scenery/simHeaven_X-Europe_1_Models"),
        &ctx,
        &model,
    );

    assert_eq!(cat, SceneryCategory::RegionalOverlay);
}
