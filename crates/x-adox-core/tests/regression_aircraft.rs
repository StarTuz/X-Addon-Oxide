// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use std::fs;
use tempfile::tempdir;
use x_adox_core::discovery::{AddonType, DiscoveryManager};

#[test]
fn test_aircraft_livery_counting() {
    let dir = tempdir().expect("Failed to create temp dir");
    let aircraft_path = dir.path().to_path_buf();

    // Setup: Create an .acf file
    fs::write(aircraft_path.join("B737.acf"), "ACF BODY").unwrap();

    // Setup: Create a liveries folder with 3 liveries (directories)
    let liveries_path = aircraft_path.join("liveries");
    fs::create_dir_all(&liveries_path).unwrap();

    fs::create_dir_all(liveries_path.join("Lufthansa")).unwrap();
    fs::create_dir_all(liveries_path.join("British Airways")).unwrap();
    fs::create_dir_all(liveries_path.join("Pan Am")).unwrap();

    // Also add a file in liveries (should be ignored)
    fs::write(liveries_path.join("read_me.txt"), "notes").unwrap();

    let count = DiscoveryManager::count_liveries(&aircraft_path);
    assert_eq!(count, 3, "Should count 3 subdirectories as liveries");
}

#[test]
fn test_aircraft_no_liveries() {
    let dir = tempdir().expect("Failed to create temp dir");
    let aircraft_path = dir.path().to_path_buf();
    fs::write(aircraft_path.join("Cessna.acf"), "ACF BODY").unwrap();

    let count = DiscoveryManager::count_liveries(&aircraft_path);
    assert_eq!(count, 0, "Should return 0 when liveries folder is missing");

    fs::create_dir_all(aircraft_path.join("liveries")).unwrap();
    let count = DiscoveryManager::count_liveries(&aircraft_path);
    assert_eq!(count, 0, "Should return 0 when liveries folder is empty");
}

#[test]
fn test_cirrus_sf50_classification() {
    use std::path::Path;
    use x_adox_bitnet::BitNetModel;
    use x_adox_bitnet::PredictContext;

    let model = BitNetModel::new().expect("Failed to load BitNet model");
    let mock_path = Path::new("/test/sf50");

    // Test exact name
    let tags = model.predict_aircraft_tags("Cirrus Vision SF50", mock_path);
    assert!(tags.contains(&"Cirrus".to_string()));
    assert!(tags.contains(&"Jet".to_string()));
    assert!(tags.contains(&"General Aviation".to_string()));
    assert!(!tags.contains(&"Fokker".to_string()));

    // Test lowercase name
    let tags = model.predict_aircraft_tags("sf50_vision", mock_path);
    assert!(tags.contains(&"Cirrus".to_string()));
    assert!(tags.contains(&"Jet".to_string()));
    assert!(!tags.contains(&"Fokker".to_string()));
}
