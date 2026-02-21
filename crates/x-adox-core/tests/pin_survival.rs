// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use std::collections::BTreeMap;
use std::fs;
use tempfile::tempdir;
use x_adox_bitnet::BitNetModel;
use x_adox_core::profiles::{Profile, ProfileCollection, ProfileManager};

#[test]
fn test_pins_survive_empty_profile_startup() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".xad_oxide");
    fs::create_dir_all(&config_dir).unwrap();

    // Set environment variable for the test
    std::env::set_var("X_ADOX_CONFIG_DIR", &config_dir);

    let xplane_root = dir.path().join("X-Plane 12");
    fs::create_dir_all(&xplane_root.join("Resources")).unwrap();
    fs::create_dir_all(&xplane_root.join("Custom Scenery")).unwrap();

    // 1. Setup heuristics.json with pins in the scoped directory
    let hash = x_adox_core::calculate_stable_hash(&xplane_root);
    let scoped_dir = config_dir.join("installs").join(&hash);
    fs::create_dir_all(&scoped_dir).unwrap();

    let heuristics_path = scoped_dir.join("heuristics.json");
    let mut model = BitNetModel::at_path(heuristics_path);
    let mut pins = BTreeMap::new();
    pins.insert("Pack A".to_string(), 100);
    model.apply_overrides(pins);
    model.save().unwrap();

    // 2. Setup profiles.json with an EMPTY profile (simulates upgrade from pre-profile version)
    let pm = ProfileManager::new(&xplane_root);
    let collection = ProfileCollection {
        profiles: vec![Profile {
            name: "Default".to_string(),
            scenery_states: std::collections::HashMap::new(),
            plugin_states: std::collections::HashMap::new(),
            aircraft_states: std::collections::HashMap::new(),
            scenery_overrides: std::collections::HashMap::new(),
            launch_args: String::new(),
        }],
        active_profile: Some("Default".to_string()),
    };
    pm.save(&collection).unwrap();

    // 3. Simulate App::new startup logic (simplified migration guard)
    let mut loaded_collection = pm.load().unwrap();
    let loaded_model = BitNetModel::at_path(scoped_dir.join("heuristics.json"));

    let active_name_opt = loaded_collection.active_profile.clone();
    let has_heuristics_pins = !loaded_model.config.overrides.is_empty();

    if let Some(active_name) = active_name_opt {
        if let Some(profile) = loaded_collection
            .profiles
            .iter_mut()
            .find(|p| p.name == active_name)
        {
            // Simplified rule: migrate if profile has no pins but heuristics does
            let should_migrate = profile.scenery_overrides.is_empty() && has_heuristics_pins;

            if should_migrate {
                profile.scenery_overrides = loaded_model
                    .config
                    .overrides
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
            }
        }
    }

    // 4. Assert: Pins should have been migrated to the profile
    assert!(
        !loaded_collection.profiles[0].scenery_overrides.is_empty(),
        "Pins should have been migrated to the profile"
    );
    assert_eq!(
        loaded_collection.profiles[0]
            .scenery_overrides
            .get("Pack A"),
        Some(&100),
        "Pack A should have priority 100"
    );
}

#[test]
fn test_profile_pins_take_priority() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".xad_oxide");
    fs::create_dir_all(&config_dir).unwrap();
    std::env::set_var("X_ADOX_CONFIG_DIR", &config_dir);

    let xplane_root = dir.path().join("X-Plane 12");
    fs::create_dir_all(&xplane_root.join("Resources")).unwrap();
    fs::create_dir_all(&xplane_root.join("Custom Scenery")).unwrap();

    // 1. Setup heuristics.json with pins
    let hash = x_adox_core::calculate_stable_hash(&xplane_root);
    let scoped_dir = config_dir.join("installs").join(&hash);
    fs::create_dir_all(&scoped_dir).unwrap();

    let heuristics_path = scoped_dir.join("heuristics.json");
    let mut model = BitNetModel::at_path(heuristics_path.clone());
    let mut global_pins = BTreeMap::new();
    global_pins.insert("Pack A".to_string(), 100);
    model.apply_overrides(global_pins);
    model.save().unwrap();

    // 2. Setup profiles.json with a profile that HAS pins (different value)
    let pm = ProfileManager::new(&xplane_root);
    let mut profile_pins = std::collections::HashMap::new();
    profile_pins.insert("Pack A".to_string(), 50); // Different value!

    let collection = ProfileCollection {
        profiles: vec![Profile {
            name: "Winter".to_string(),
            scenery_states: std::collections::HashMap::new(),
            plugin_states: std::collections::HashMap::new(),
            aircraft_states: std::collections::HashMap::new(),
            scenery_overrides: profile_pins,
            launch_args: String::new(),
        }],
        active_profile: Some("Winter".to_string()),
    };
    pm.save(&collection).unwrap();

    // 3. Simulate App::new startup logic
    let loaded_collection = pm.load().unwrap();
    let mut loaded_model = BitNetModel::at_path(heuristics_path);

    if let Some(active_name) = loaded_collection.active_profile.clone() {
        if let Some(profile) = loaded_collection
            .profiles
            .iter()
            .find(|p| p.name == active_name)
        {
            let has_heuristics_pins = !loaded_model.config.overrides.is_empty();
            let should_migrate = profile.scenery_overrides.is_empty() && has_heuristics_pins;

            if should_migrate {
                // Would migrate, but profile already has pins so this won't run
            } else {
                // Standard path: Profile pins take priority
                let overrides = profile
                    .scenery_overrides
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect::<BTreeMap<_, _>>();
                loaded_model.apply_overrides(overrides);
            }
        }
    }

    // 4. Assert: Profile's pins should have overwritten heuristics
    assert_eq!(
        loaded_model.config.overrides.get("Pack A"),
        Some(&50),
        "Profile pins (50) should take priority over heuristics pins (100)"
    );
}

#[test]
fn test_pins_survive_profile_with_scenery_states() {
    // This test verifies the fix for the bug where users who had:
    // - Pins in heuristics.json
    // - Profile with scenery_states (toggled scenery) but empty scenery_overrides
    // Would have their pins wiped because is_default_state returned false.
    //
    // With the simplified migration guard, this should now work correctly.

    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".xad_oxide");
    fs::create_dir_all(&config_dir).unwrap();
    std::env::set_var("X_ADOX_CONFIG_DIR", &config_dir);

    let xplane_root = dir.path().join("X-Plane 12");
    fs::create_dir_all(&xplane_root.join("Resources")).unwrap();
    fs::create_dir_all(&xplane_root.join("Custom Scenery")).unwrap();

    // 1. Setup heuristics.json with pins
    let hash = x_adox_core::calculate_stable_hash(&xplane_root);
    let scoped_dir = config_dir.join("installs").join(&hash);
    fs::create_dir_all(&scoped_dir).unwrap();

    let heuristics_path = scoped_dir.join("heuristics.json");
    let mut model = BitNetModel::at_path(heuristics_path.clone());
    let mut pins = BTreeMap::new();
    pins.insert("Pack A".to_string(), 100);
    model.apply_overrides(pins);
    model.save().unwrap();

    // 2. Setup profiles.json with a profile that has scenery_states but NO pins
    // This simulates a user who toggled scenery on/off but never pinned anything
    let pm = ProfileManager::new(&xplane_root);
    let mut scenery_states = std::collections::HashMap::new();
    scenery_states.insert("Some Scenery".to_string(), true);
    scenery_states.insert("Other Scenery".to_string(), false);

    let collection = ProfileCollection {
        profiles: vec![Profile {
            name: "Default".to_string(),
            scenery_states, // Non-empty!
            plugin_states: std::collections::HashMap::new(),
            aircraft_states: std::collections::HashMap::new(),
            scenery_overrides: std::collections::HashMap::new(), // Empty!
            launch_args: String::new(),
        }],
        active_profile: Some("Default".to_string()),
    };
    pm.save(&collection).unwrap();

    // 3. Simulate App::new startup logic with SIMPLIFIED migration guard
    let mut loaded_collection = pm.load().unwrap();
    let loaded_model = BitNetModel::at_path(heuristics_path);

    if let Some(active_name) = loaded_collection.active_profile.clone() {
        if let Some(profile) = loaded_collection
            .profiles
            .iter_mut()
            .find(|p| p.name == active_name)
        {
            let has_heuristics_pins = !loaded_model.config.overrides.is_empty();
            // SIMPLIFIED: No is_default_state check!
            let should_migrate = profile.scenery_overrides.is_empty() && has_heuristics_pins;

            if should_migrate {
                profile.scenery_overrides = loaded_model
                    .config
                    .overrides
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
            }
        }
    }

    // 4. Assert: Pins should have been migrated even though profile had scenery_states
    assert!(
        !loaded_collection.profiles[0].scenery_overrides.is_empty(),
        "Pins should have been migrated despite profile having scenery_states"
    );
    assert_eq!(
        loaded_collection.profiles[0]
            .scenery_overrides
            .get("Pack A"),
        Some(&100),
        "Pack A should have priority 100"
    );
}
