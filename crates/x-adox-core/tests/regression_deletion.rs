use std::fs;
use tempfile::tempdir;
use x_adox_core::management::{AddonType, ModManager};
use x_adox_core::scenery::SceneryManager;

#[test]
fn test_scenery_deletion_persistence() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let custom_scenery = root.join("Custom Scenery");
    let resources = root.join("Resources");

    // Create necessary directories
    fs::create_dir_all(&custom_scenery).unwrap();
    fs::create_dir_all(&resources).unwrap();

    // Create a mock scenery folder
    let pack_name = "test_pack";
    let pack_path = custom_scenery.join(pack_name);
    fs::create_dir_all(&pack_path).unwrap();

    // Create a mock INI
    let ini_path = custom_scenery.join("scenery_packs.ini");
    let content = format!(
        "I\n1000 Version\nSCENERY\n\nSCENERY_PACK Custom Scenery/{}/\n",
        pack_name
    );
    fs::write(&ini_path, content).unwrap();

    // Verify initial state
    let mut sm = SceneryManager::new(ini_path.clone());
    sm.load().unwrap();
    assert_eq!(sm.packs.len(), 1);
    assert_eq!(sm.packs[0].name, pack_name);

    // Delete the scenery addon
    // Note: delete_addon expects path relative to root if it is relative
    // Here we pass the absolute path for simplicity, or relative to root?
    // main.rs logic: if relative join root, else use absolute. Use absolute here.
    let result = ModManager::delete_addon(root, &pack_path, AddonType::Scenery);
    assert!(
        result.is_ok(),
        "Deletion should succeed: {:?}",
        result.err()
    );

    // Verify file system deletion
    assert!(!pack_path.exists(), "Scenery folder should be deleted");

    // Verify INI update
    let saved_content = fs::read_to_string(&ini_path).unwrap();
    assert!(
        !saved_content.contains(pack_name),
        "INI should not contain the deleted pack"
    );

    // Double check with SceneryManager load
    let mut sm2 = SceneryManager::new(ini_path.clone());
    sm2.load().unwrap();
    assert_eq!(sm2.packs.len(), 0, "SceneryManager should report 0 packs");
}

#[test]
fn test_aircraft_deletion() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let aircraft_dir = root.join("Aircraft");
    let resources = root.join("Resources");
    fs::create_dir_all(&aircraft_dir).unwrap();
    fs::create_dir_all(&resources).unwrap(); // Needed for XPlaneManager safety checks implicitly? No, but good practice

    // Create dummy aircraft
    let b737 = aircraft_dir.join("B737");
    fs::create_dir_all(&b737).unwrap();
    fs::write(b737.join("b737.acf"), "").unwrap();

    // Delete
    let result = ModManager::delete_addon(root, &b737, AddonType::Aircraft);
    assert!(result.is_ok());

    // Verify
    assert!(!b737.exists(), "Aircraft folder should be deleted");
}

#[test]
fn test_security_check_outside_folders() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    // Setup valid structure
    fs::create_dir_all(root.join("Aircraft")).unwrap();
    fs::create_dir_all(root.join("Custom Scenery")).unwrap();

    // Try to delete "Resources" using Aircraft type -> Should fail
    let resources = root.join("Resources");
    fs::create_dir_all(&resources).unwrap();

    let result = ModManager::delete_addon(root, &resources, AddonType::Aircraft);
    assert!(result.is_err(), "Should catch path outside allowed folder");
    let msg = result.unwrap_err();
    assert!(msg.contains("Safety check failed"));

    assert!(resources.exists(), "Resources folder must NOT be deleted");
}
