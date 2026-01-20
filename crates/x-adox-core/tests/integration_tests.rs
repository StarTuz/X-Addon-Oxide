use std::fs;
use tempfile::TempDir;
use x_adox_core::XPlaneManager;

/// Helper to create a mock X-Plane directory structure
struct MockXPlane {
    // Keep TempDir alive so the directory isn't deleted
    _dir: TempDir,
    pub root: std::path::PathBuf,
}

impl MockXPlane {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let root = dir.path().to_path_buf();

        // Create standard folders
        fs::create_dir_all(root.join("Resources")).unwrap();
        fs::create_dir_all(root.join("Custom Scenery")).unwrap();
        fs::create_dir_all(root.join("Aircraft")).unwrap();

        // Create a fake executable or key file if we needed strict validation
        // fs::write(root.join("X-Plane-x86_64"), "").unwrap();

        Self { _dir: dir, root }
    }
}

#[test]
fn test_detect_valid_root() {
    let mock = MockXPlane::new();
    let manager = XPlaneManager::new(&mock.root);
    assert!(manager.is_ok(), "Should accept valid mock root");
}

#[test]
fn test_reject_invalid_root() {
    let dir = tempfile::tempdir().unwrap(); // Empty dir
    let manager = XPlaneManager::new(dir.path());
    assert!(manager.is_err(), "Should reject empty directory");
}

#[test]
fn test_get_scenery_packs_path() {
    let mock = MockXPlane::new();
    let manager = XPlaneManager::new(&mock.root).unwrap();
    let path = manager.get_scenery_packs_path();
    assert_eq!(
        path,
        mock.root.join("Custom Scenery").join("scenery_packs.ini")
    );
}

use x_adox_core::discovery::{AddonType, DiscoveryManager};

#[test]
fn test_discovery_aircraft() {
    let mock = MockXPlane::new();
    let aircraft_dir = mock
        .root
        .join("Aircraft")
        .join("Laminar Research")
        .join("Cessna 172");
    fs::create_dir_all(&aircraft_dir).unwrap();
    fs::write(aircraft_dir.join("Cessna_172.acf"), "ACF BODY").unwrap();

    let mut cache = x_adox_core::cache::DiscoveryCache::new();
    let results = DiscoveryManager::scan_aircraft(&mock.root, &mut cache, &[]);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Cessna 172");
    if let AddonType::Aircraft(ref name) = results[0].addon_type {
        assert_eq!(name, "Cessna_172.acf");
    } else {
        panic!("Wrong addon type");
    }
}

#[test]
fn test_discovery_scenery() {
    let mock = MockXPlane::new();
    let pack_dir = mock.root.join("Custom Scenery").join("My_Airport");
    fs::create_dir_all(&pack_dir).unwrap();

    let mut cache = x_adox_core::cache::DiscoveryCache::new();
    let results = DiscoveryManager::scan_scenery(&mock.root.join("Custom Scenery"), &mut cache);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "My_Airport");
}
