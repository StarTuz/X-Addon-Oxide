use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use x_adox_core::normalize_install_path;

#[test]
fn test_regression_path_normalization_registry() {
    // Setup a temp workspace
    let tmp_home = tempdir().unwrap();
    let home_path = tmp_home.path().to_path_buf();

    // Create fake installation location
    let install_dir = home_path.join("MyXPlane");
    fs::create_dir_all(&install_dir).unwrap();
    let canonical_install = install_dir.canonicalize().unwrap();

    // Create an "alias" (simulated via symlink if unix, or just another path if we can mapping it)
    // Testing exact registry match vs canonical match

    // 1. Setup Registry
    #[cfg(target_os = "linux")]
    {
        let dot_xplane = home_path.join(".x-plane");
        fs::create_dir_all(&dot_xplane).unwrap();
        let registry_file = dot_xplane.join("x-plane_install_12.txt");
        // Write the CANONICAL path to the registry
        fs::write(&registry_file, format!("{}\n", canonical_install.display())).unwrap();

        // Mock HOME
        unsafe {
            env::set_var("HOME", &home_path);
        }
    }
    #[cfg(target_os = "windows")]
    {
        // Mock LOCALAPPDATA
        unsafe {
            env::set_var("LOCALAPPDATA", &home_path);
        }
        let registry_file = home_path.join("x-plane_install_12.txt");
        fs::write(&registry_file, format!("{}\n", canonical_install.display())).unwrap();
    }
    #[cfg(target_os = "macos")]
    {
        let dot_xplane = home_path.join(".x-plane"); // One of the fallback locations
        fs::create_dir_all(&dot_xplane).unwrap();
        let registry_file = dot_xplane.join("x-plane_install_12.txt");
        fs::write(&registry_file, format!("{}\n", canonical_install.display())).unwrap();
        unsafe {
            env::set_var("HOME", &home_path);
        }
    }

    // 2. Test Normalization
    // Case A: Input is exactly the registry path
    let normalized_a = normalize_install_path(&canonical_install);
    assert_eq!(
        normalized_a, canonical_install,
        "Registry path should return itself"
    );

    // Case B: Input is a symlink to the registry path (Unix only usually)
    #[cfg(unix)]
    {
        let symlink_path = home_path.join("AliasXPlane");
        std::os::unix::fs::symlink(&install_dir, &symlink_path).unwrap();

        let normalized_b = normalize_install_path(&symlink_path);
        // CRITICAL CHECK: It must return the REGISTRY path (canonical_install), NOT the symlink path
        assert_eq!(
            normalized_b, canonical_install,
            "Symlink/Alias must resolve to Registry Path"
        );

        // Ensure hashes would match
        let hash_a = x_adox_core::calculate_path_hash(&normalized_a);
        let hash_b = x_adox_core::calculate_path_hash(&normalized_b);
        assert_eq!(hash_a, hash_b, "Hashes must match for aliased paths!");
    }
}
