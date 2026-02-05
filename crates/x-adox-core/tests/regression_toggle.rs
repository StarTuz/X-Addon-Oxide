// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use tempfile::tempdir;
use x_adox_core::scenery::{SceneryManager, SceneryPackType};

#[test]
fn test_scenery_toggle_persistence() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let custom_scenery = root.join("Custom Scenery");
    std::fs::create_dir_all(&custom_scenery).unwrap();

    let ini_path = custom_scenery.join("scenery_packs.ini");

    // Create a mock INI
    let content = "I\n1000 Version\nSCENERY\n\nSCENERY_PACK Custom Scenery/Test_Pack/\n";
    std::fs::write(&ini_path, content).unwrap();

    // Load
    let mut sm = SceneryManager::new(ini_path.clone());
    sm.load().unwrap();

    assert_eq!(sm.packs.len(), 1);
    assert_eq!(sm.packs[0].name, "Test_Pack");
    assert_eq!(sm.packs[0].status, SceneryPackType::Active);

    // Toggle to Disabled
    sm.packs[0].status = SceneryPackType::Disabled;

    // Save
    sm.save(None).unwrap();

    // Verify file content
    let saved_content = std::fs::read_to_string(&ini_path).unwrap();
    assert!(saved_content.contains("SCENERY_PACK_DISABLED Custom Scenery/Test_Pack/"));
    assert!(!saved_content.contains("SCENERY_PACK Custom Scenery/Test_Pack/"));
}
