// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use std::fs;
use tempfile::TempDir;
use x_adox_core::discovery::DiscoveryManager;
use x_adox_core::management::ModManager;

/// Helper to create a mock X-Plane root with standard folder structure
fn create_mock_root() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("Resources")).unwrap();
    fs::create_dir_all(root.join("Custom Scenery")).unwrap();
    fs::create_dir_all(root.join("Aircraft")).unwrap();
    dir
}

#[test]
fn test_suppress_laminar_duplicates_basic() {
    let dir = create_mock_root();
    let root = dir.path();

    // Simulate: user previously disabled Cessna 172 (moved to Aircraft (Disabled))
    let disabled_cessna = root
        .join("Aircraft (Disabled)")
        .join("Laminar Research")
        .join("Cessna 172");
    fs::create_dir_all(&disabled_cessna).unwrap();
    fs::write(disabled_cessna.join("Cessna_172.acf"), "ACF BODY").unwrap();

    // Simulate: X-Plane updater restored the same aircraft
    let active_cessna = root
        .join("Aircraft")
        .join("Laminar Research")
        .join("Cessna 172");
    fs::create_dir_all(&active_cessna).unwrap();
    fs::write(active_cessna.join("Cessna_172.acf"), "ACF BODY").unwrap();

    // Run suppression
    let suppressed = ModManager::suppress_laminar_duplicates(root);

    // The active copy should be removed
    assert!(!active_cessna.exists(), "Active copy should be deleted");
    // The disabled copy should remain
    assert!(disabled_cessna.exists(), "Disabled copy should remain");
    // The function should report the suppressed aircraft
    assert_eq!(suppressed, vec!["Cessna 172"]);
}

#[test]
fn test_suppress_no_duplicates() {
    let dir = create_mock_root();
    let root = dir.path();

    // Only active Laminar aircraft exist — no disabled copy
    let active_cessna = root
        .join("Aircraft")
        .join("Laminar Research")
        .join("Cessna 172");
    fs::create_dir_all(&active_cessna).unwrap();
    fs::write(active_cessna.join("Cessna_172.acf"), "ACF BODY").unwrap();

    let suppressed = ModManager::suppress_laminar_duplicates(root);

    // Nothing should be suppressed
    assert!(suppressed.is_empty());
    // Aircraft should still exist
    assert!(active_cessna.exists());
}

#[test]
fn test_is_laminar_default_tagging() {
    let dir = create_mock_root();
    let root = dir.path();

    // Create a Laminar Research aircraft
    let laminar_dir = root
        .join("Aircraft")
        .join("Laminar Research")
        .join("Cessna 172");
    fs::create_dir_all(&laminar_dir).unwrap();
    fs::write(laminar_dir.join("Cessna_172.acf"), "ACF BODY").unwrap();

    // Create a third-party aircraft
    let custom_dir = root.join("Aircraft").join("MyAddons").join("TBM 930");
    fs::create_dir_all(&custom_dir).unwrap();
    fs::write(custom_dir.join("TBM_930.acf"), "ACF BODY").unwrap();

    let mut cache = x_adox_core::cache::DiscoveryCache::new();
    let results = DiscoveryManager::scan_aircraft(root, &mut cache, &[]);

    assert_eq!(results.len(), 2);

    let cessna = results.iter().find(|a| a.name == "Cessna 172").unwrap();
    let tbm = results.iter().find(|a| a.name == "TBM 930").unwrap();

    assert!(
        cessna.is_laminar_default,
        "Cessna 172 should be tagged as Laminar default"
    );
    assert!(
        !tbm.is_laminar_default,
        "TBM 930 should NOT be tagged as Laminar default"
    );
}
