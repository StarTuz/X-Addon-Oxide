// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use std::collections::HashMap;
use std::fs;
use x_adox_core::profiles::{Profile, ProfileCollection, ProfileManager};

#[test]
fn test_profile_pin_persistence() -> anyhow::Result<()> {
    // 1. Setup temporary test directory
    let temp_dir = tempfile::tempdir()?;
    let xplane_root = temp_dir.path().join("X-Plane 12");
    fs::create_dir_all(&xplane_root)?;

    // 2. Initialize ProfileManager
    let manager = ProfileManager::new(&xplane_root);

    // 3. Create a collection with a Default profile locally (simulate app state)
    let mut collection = ProfileCollection::default();

    // 4. Set a pin in the Default profile
    if let Some(profile) = collection.get_active_profile_mut() {
        profile
            .scenery_overrides
            .insert("Test Scenery Pack".to_string(), 50);
        profile
            .scenery_states
            .insert("Test Scenery Pack".to_string(), true);
    }

    // 5. Save functionality
    manager.save(&collection)?;

    // 6. Verify persistence by loading into a NEW manager
    let loaded_collection = manager.load()?;
    let loaded_profile = loaded_collection.profiles.first().unwrap();

    assert_eq!(
        loaded_profile.scenery_overrides.get("Test Scenery Pack"),
        Some(&50),
        "Pin should be persisted to disk"
    );

    // 7. Simulate Profile Switch Logic (Verify 'Clone' behavior)
    // Create a second profile
    let mut collection = loaded_collection;
    let new_profile = Profile::new_default("VFR Profile".to_string());
    collection.profiles.push(new_profile);

    // Switch to VFR (simulate app logic: update active profile index/name)
    collection.active_profile = Some("VFR Profile".to_string());

    // In the real app, we would implicitly 'clear' the heuristic model's overrides here
    // or rely on the profile applying its own (empty) map.

    // Switch back to Default
    collection.active_profile = Some("Default".to_string());
    let active = collection
        .profiles
        .iter()
        .find(|p| p.name == "Default")
        .unwrap();

    assert_eq!(
        active.scenery_overrides.get("Test Scenery Pack"),
        Some(&50),
        "Pin should still exist in Default profile after switch"
    );

    Ok(())
}
