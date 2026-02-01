use x_adox_core::scenery::classifier::Classifier;
use x_adox_core::scenery::{SceneryCategory, SceneryDescriptor};

#[test]
fn test_urban_promotion() {
    let descriptor = SceneryDescriptor {
        object_count: 100, // Many buildings
        facade_count: 50,
        ..Default::default()
    };

    // Even if initially Unknown or RegionalOverlay, it should become a Landmark
    let cat = Classifier::heal_classification(
        SceneryCategory::Unknown,
        false, // No airports
        true,  // Has tiles
        &descriptor,
    );

    assert_eq!(cat, SceneryCategory::Landmark);
}

#[test]
fn test_ortho_demotion() {
    let descriptor = SceneryDescriptor {
        polygon_count: 5000, // Many mesh points
        object_count: 0,     // No buildings
        facade_count: 0,
        ..Default::default()
    };

    let cat = Classifier::heal_classification(SceneryCategory::Unknown, false, true, &descriptor);

    assert_eq!(cat, SceneryCategory::OrthoBase);
}

#[test]
fn test_airport_overlay_promotion() {
    let descriptor = SceneryDescriptor {
        has_airport_properties: true,
        ..Default::default()
    };

    let cat = Classifier::heal_classification(SceneryCategory::Unknown, false, true, &descriptor);

    assert_eq!(cat, SceneryCategory::AirportOverlay);
}

#[test]
fn test_protected_stays_protected() {
    let descriptor = SceneryDescriptor {
        object_count: 0,
        polygon_count: 0,
        ..Default::default()
    };

    // SimHeaven etc. should not be demoted to Mesh even if the peeker fails or finds zero data
    let cat =
        Classifier::heal_classification(SceneryCategory::RegionalOverlay, false, true, &descriptor);

    assert_eq!(cat, SceneryCategory::RegionalOverlay);
}
