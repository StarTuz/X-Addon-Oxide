use std::collections::HashMap;
use tempfile::tempdir;
use x_adox_core::profiles::{Profile, ProfileCollection, ProfileManager};

#[test]
fn test_profile_state_updates() {
    // Start with empty collection to have full control over indices
    let mut collection = ProfileCollection {
        profiles: Vec::new(),
        active_profile: None,
    };

    let p1 = Profile {
        name: "Profile 1".to_string(),
        scenery_states: HashMap::new(),
        plugin_states: HashMap::new(),
        aircraft_states: HashMap::new(),
        launch_args: String::new(),
    };

    let p2 = Profile {
        name: "Profile 2".to_string(),
        scenery_states: HashMap::new(),
        plugin_states: HashMap::new(),
        aircraft_states: HashMap::new(),
        launch_args: String::new(),
    };

    collection.profiles.push(p1);
    collection.profiles.push(p2);

    // Set active
    collection.active_profile = Some("Profile 1".to_string());

    // Update scenery
    let mut states = HashMap::new();
    states.insert("Test Pack".to_string(), true);
    collection.update_active_scenery(states.clone());

    // Verify Profile 1 updated (index 0)
    assert_eq!(
        collection.profiles[0].scenery_states.get("Test Pack"),
        Some(&true)
    );
    // Verify Profile 2 untouched (index 1)
    assert!(collection.profiles[1].scenery_states.is_empty());

    // Switch active
    collection.active_profile = Some("Profile 2".to_string());
    collection.update_active_launch_args("--test".to_string());

    assert_eq!(collection.profiles[1].launch_args, "--test");
    assert!(collection.profiles[0].launch_args.is_empty());
}

#[test]
fn test_profile_persistence() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".xad_oxide");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Set environment variable for the test
    std::env::set_var("X_ADOX_CONFIG_DIR", &config_dir);

    let root = dir.path().join("X-Plane"); // Mock root
    let manager = ProfileManager::new(&root);

    // Create collection with just one profile (no default)
    let collection = ProfileCollection {
        profiles: vec![Profile {
            name: "Persist Test".to_string(),
            scenery_states: HashMap::new(),
            plugin_states: HashMap::new(),
            aircraft_states: HashMap::new(),
            launch_args: "--persist".to_string(),
        }],
        active_profile: Some("Persist Test".to_string()),
    };

    // Save
    manager.save(&collection).unwrap();

    // Load fresh
    let loaded = manager.load().unwrap();
    assert_eq!(loaded.profiles.len(), 1);
    assert_eq!(loaded.active_profile, Some("Persist Test".to_string()));
    assert_eq!(loaded.profiles[0].launch_args, "--persist");
}
