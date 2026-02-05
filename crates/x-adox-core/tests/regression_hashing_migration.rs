// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use std::fs;
use tempfile::tempdir;
use x_adox_core::{calculate_legacy_hash, calculate_stable_hash, get_scoped_config_root};

#[test]
fn test_hashing_migration() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".xad_oxide");
    fs::create_dir_all(&config_dir).unwrap();

    // Set environment variable for the test to point to our temp config dir
    std::env::set_var("X_ADOX_CONFIG_DIR", &config_dir);

    let xplane_root = dir.path().join("X-Plane 12");
    fs::create_dir_all(&xplane_root).unwrap();

    // Calculate both hashes
    let legacy_hash = calculate_legacy_hash(&xplane_root);
    let stable_hash = calculate_stable_hash(&xplane_root);

    assert_ne!(
        legacy_hash, stable_hash,
        "Legacy and stable hashes should differ for this path"
    );

    let legacy_folder = config_dir.join("installs").join(&legacy_hash);
    let stable_folder = config_dir.join("installs").join(&stable_hash);

    // 1. Setup: Create legacy folder with some "data"
    fs::create_dir_all(&legacy_folder).unwrap();
    fs::write(
        legacy_folder.join("profiles.json"),
        "{\"profiles\":[], \"active_profile\": null}",
    )
    .unwrap();

    // 2. Execution: Call get_scoped_config_root which triggers migration
    let scoped_root = get_scoped_config_root(&xplane_root);

    // 3. Verification:
    // Scoped root should be the stable path
    assert_eq!(scoped_root, stable_folder);

    // Stable folder should exist and contain the migrated file
    assert!(stable_folder.exists());
    assert!(stable_folder.join("profiles.json").exists());

    // Legacy folder should NO LONGER exist (it was renamed)
    assert!(!legacy_folder.exists());
}

#[test]
fn test_hashing_stability_normalized() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("XP12");
    let p2 = dir.path().join("XP12/"); // Trailing slash

    fs::create_dir_all(&p1).unwrap();
    // No need to create p2 separately as it's the same path on many FS, but canonicalization handles it.

    let h1 = calculate_stable_hash(&p1);
    let h2 = calculate_stable_hash(&p2);

    assert_eq!(
        h1, h2,
        "Hashes should be identical regardless of trailing slash"
    );
}

#[test]
fn test_migration_both_folders_exist_prefers_legacy_data() {
    // This tests the scenario where dd58f05 created an empty stable folder
    // before migration logic existed, leaving user's profiles stranded in legacy.

    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".xad_oxide");
    fs::create_dir_all(&config_dir).unwrap();
    std::env::set_var("X_ADOX_CONFIG_DIR", &config_dir);

    let xplane_root = dir.path().join("X-Plane 12");
    fs::create_dir_all(&xplane_root).unwrap();

    let legacy_hash = calculate_legacy_hash(&xplane_root);
    let stable_hash = calculate_stable_hash(&xplane_root);

    let legacy_folder = config_dir.join("installs").join(&legacy_hash);
    let stable_folder = config_dir.join("installs").join(&stable_hash);

    // Verify hashes are different (key assumption for migration tests)
    assert_ne!(
        legacy_hash, stable_hash,
        "Regression: Legacy and stable hashes collided! Path: {:?}",
        xplane_root
    );

    // Setup: Legacy has meaningful data (Winter profile with pins)
    fs::create_dir_all(&legacy_folder).unwrap();
    fs::write(
        legacy_folder.join("profiles.json"),
        r#"{"profiles":[{"name":"Default","scenery_states":{},"plugin_states":{},"aircraft_states":{},"scenery_overrides":{},"launch_args":""},{"name":"Winter","scenery_states":{},"plugin_states":{},"aircraft_states":{},"scenery_overrides":{"Pack A":100},"launch_args":""}],"active_profile":"Winter"}"#,
    ).unwrap();

    // Setup: Stable exists but has only empty default profile (created by buggy dd58f05)
    fs::create_dir_all(&stable_folder).unwrap();
    fs::write(
        stable_folder.join("profiles.json"),
        r#"{"profiles":[{"name":"Default","scenery_states":{},"plugin_states":{},"aircraft_states":{},"scenery_overrides":{},"launch_args":""}],"active_profile":"Default"}"#,
    ).unwrap();

    // Execute: get_scoped_config_root should detect empty stable and copy from legacy
    let scoped_root = get_scoped_config_root(&xplane_root);
    assert_eq!(
        scoped_root, stable_folder,
        "Scoped root should be the stable folder"
    );

    // Verify: Stable should now have the Winter profile from legacy
    let stable_content = fs::read_to_string(stable_folder.join("profiles.json")).unwrap();
    assert!(
        stable_content.contains("Winter"),
        "Winter profile was NOT copied from legacy ({}) to stable ({}). Stable content: {}",
        legacy_hash,
        stable_hash,
        stable_content
    );
    assert!(
        stable_content.contains("Pack A"),
        "Pins should have been preserved in migration. Stable content: {}",
        stable_content
    );
}
