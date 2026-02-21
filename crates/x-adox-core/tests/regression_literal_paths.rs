// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use tempfile::tempdir;
use x_adox_core::scenery::{SceneryManager, SceneryPackType};

#[test]
fn test_literal_path_preservation() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create folder with trailing space
    let custom_scenery = root.join("Custom Scenery");
    let riga_path = custom_scenery.join("Riga Latvija ");
    std::fs::create_dir_all(&riga_path).unwrap();

    let ini_path = custom_scenery.join("scenery_packs.ini");

    // Create INI with trailing space in entry
    // NOTE: The entry has a space before the trailing slash
    let content = "I\n1000 Version\nSCENERY\n\nSCENERY_PACK Custom Scenery/Riga Latvija /\n";
    std::fs::write(&ini_path, content).unwrap();

    // 1. Load
    let mut sm = SceneryManager::new(ini_path.clone());
    sm.load().unwrap();

    assert_eq!(sm.packs.len(), 1);
    // The 'name' should be "Riga Latvija " (matching folder)
    assert_eq!(sm.packs[0].name, "Riga Latvija ");

    // 2. Toggle to Disabled
    sm.packs[0].status = SceneryPackType::Disabled;

    // 3. Save
    sm.save(None).unwrap();

    // 4. Verify file content matches EXACTLY
    let saved_content = std::fs::read_to_string(&ini_path).unwrap();
    println!("Saved content:\n{}", saved_content);

    // Verify it preserved the literal path including that space
    assert!(saved_content.contains("SCENERY_PACK_DISABLED Custom Scenery/Riga Latvija /"));
}
