// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use x_adox_core::scenery::coords_to_region;

#[test]
fn test_continent_mapping() {
    // Europe
    assert_eq!(coords_to_region(48.35, 11.78), "Europe"); // EDDM Munich
    assert_eq!(coords_to_region(51.47, -0.45), "Europe"); // EGLL London

    // North America
    assert_eq!(coords_to_region(40.64, -73.78), "North America"); // KJFK New York
    assert_eq!(coords_to_region(34.05, -118.24), "North America"); // KLAX Los Angeles

    // South America
    assert_eq!(coords_to_region(-23.43, -46.47), "South America"); // SBGR Sao Paulo

    // Africa
    assert_eq!(coords_to_region(-26.13, 28.24), "Africa"); // FAOR Johannesburg
    assert_eq!(coords_to_region(30.12, 31.40), "Africa"); // HECA Cairo

    // Asia
    assert_eq!(coords_to_region(35.77, 140.39), "Asia"); // RJAA Tokyo
    assert_eq!(coords_to_region(22.30, 113.91), "Asia"); // VHHH Hong Kong

    // Oceania
    assert_eq!(coords_to_region(-33.94, 151.17), "Oceania"); // YSSY Sydney

    // Antarctica
    assert_eq!(coords_to_region(-77.84, 166.66), "Antarctica"); // NZWD McMurdo

    // Other / Global (Middle of Pacific)
    assert_eq!(coords_to_region(0.0, -150.0), "Other / Global");
}

#[test]
fn test_get_region_priority() {
    use std::path::PathBuf;
    use x_adox_core::scenery::{SceneryCategory, SceneryPack, SceneryPackType};

    let mut pack = SceneryPack {
        name: "Test Pack".to_string(),
        path: PathBuf::from("/test/pack"),
        raw_path: None,
        status: SceneryPackType::Active,
        category: SceneryCategory::CustomAirport,
        airports: vec![],
        tiles: vec![(48, 11)], // Munich (Europe)
        tags: vec![],
        descriptor: Default::default(),
        region: Some("Custom Region".to_string()),
    };

    // Priority 1: Should return cached region
    assert_eq!(pack.get_region(), "Custom Region");

    // Clear cached region
    pack.region = None;

    // Priority 3: Should fallback to coordinate lookup (Europe)
    assert_eq!(pack.get_region(), "Europe");
}
