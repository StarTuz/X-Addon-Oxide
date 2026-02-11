// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz
//
// Regression tests for scenery classification fixes:
// - Community libraries (world-models, Sea_Life, ruscenery) must be classified as Library
// - Landmarks without hyphen (e.g., "X-Plane Landmarks Dubai") must be classified as Landmark
// - Packs with library.txt + airport data must remain Library (not promoted to CustomAirport)
// - Orbx airport-specific meshes must NOT be classified as generic Mesh

use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;
use x_adox_core::scenery::classifier::Classifier;
use x_adox_core::scenery::SceneryCategory;

#[test]
fn test_community_libraries_classified_by_name() {
    // These community libraries lack "library" or "lib" in their names
    // and must be caught by explicit classifier entries.
    let cases = [
        ("world-models", SceneryCategory::Library),
        ("Sea_Life", SceneryCategory::Library),
        ("ruscenery", SceneryCategory::Library),
    ];

    for (name, expected) in &cases {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let result = Classifier::classify_heuristic(&path, name);
        assert_eq!(
            &result, expected,
            "'{}' should be {:?}, got {:?}",
            name, expected, result
        );
    }
}

#[test]
fn test_landmarks_without_hyphen() {
    // "X-Plane Landmarks Dubai" has no trailing hyphen but is still a Landmark.
    let name = "X-Plane Landmarks Dubai";
    let path = PathBuf::from(format!("Custom Scenery/{}", name));
    let result = Classifier::classify_heuristic(&path, name);
    assert_eq!(
        result,
        SceneryCategory::Landmark,
        "'{}' should be Landmark, got {:?}",
        name,
        result
    );
}

#[test]
fn test_landmarks_with_hyphen_still_works() {
    // Ensure original format still works.
    let name = "X-Plane Landmarks - Paris";
    let path = PathBuf::from(format!("Custom Scenery/{}", name));
    let result = Classifier::classify_heuristic(&path, name);
    assert_eq!(
        result,
        SceneryCategory::Landmark,
        "'{}' should be Landmark, got {:?}",
        name,
        result
    );
}

#[test]
fn test_library_txt_prevents_airport_promotion() {
    // Simulates the mod.rs load pipeline for a pack that:
    // 1. Has an Unknown name (no heuristic match)
    // 2. Has a library.txt file
    // 3. Contains airport data (helipads/POIs)
    // Expected: Library (not CustomAirport)

    let dir = tempdir().unwrap();
    // Use a name that has NO heuristic keyword match (no "lib", "library", etc.)
    let pack_path = dir.path().join("Custom Scenery").join("Fauna_Collection");
    std::fs::create_dir_all(&pack_path).unwrap();

    // Create library.txt (the definitive X-Plane library signal)
    let mut lib_txt = std::fs::File::create(pack_path.join("library.txt")).unwrap();
    writeln!(lib_txt, "A").unwrap();
    writeln!(lib_txt, "800").unwrap();
    writeln!(lib_txt, "LIBRARY").unwrap();

    // Create a minimal apt.dat with a helipad entry
    let nav_path = pack_path.join("Earth nav data");
    std::fs::create_dir_all(&nav_path).unwrap();
    let mut apt_file = std::fs::File::create(nav_path.join("apt.dat")).unwrap();
    writeln!(apt_file, "I").unwrap();
    writeln!(apt_file, "1100 Version").unwrap();
    writeln!(
        apt_file,
        "17   40.00000  -74.00000 123.0 0 0 0  FAKE Helipad"
    )
    .unwrap();
    writeln!(apt_file, "99").unwrap();

    // Simulate the classification pipeline (same order as mod.rs load)
    let name = "Fauna_Collection";
    let mut category = Classifier::classify_heuristic(&pack_path, name);
    assert_eq!(
        category,
        SceneryCategory::Unknown,
        "Name should not match any heuristic"
    );

    // Discover airports (simulated: pack has airport data)
    let airports = x_adox_core::scenery::discover_airports_in_pack(&pack_path);

    // Step 3: Structural Library Detection (runs BEFORE promotion now)
    if category == SceneryCategory::Unknown {
        if pack_path.join("library.txt").exists() {
            category = SceneryCategory::Library;
        }
    }

    // Step 3b: Post-Discovery Promotion
    if !airports.is_empty() {
        match category {
            SceneryCategory::GlobalAirport
            | SceneryCategory::Library
            | SceneryCategory::GlobalBase
            | SceneryCategory::Landmark => {
                // Keep — Library is protected
            }
            _ => {
                category = SceneryCategory::CustomAirport;
            }
        }
    }

    assert_eq!(
        category,
        SceneryCategory::Library,
        "Pack with library.txt + airports must remain Library, got {:?}",
        category
    );
}

#[test]
fn test_orbx_airport_mesh_not_classified_as_generic_mesh() {
    // Orbx_B_EGLC_LondonCity_Mesh is an airport-specific mesh companion pack,
    // NOT a standalone terrain mesh. It should be SpecificMesh (not generic Mesh
    // or CustomAirport).
    let name = "Orbx_B_EGLC_LondonCity_Mesh";
    let path = PathBuf::from(format!("Custom Scenery/{}", name));
    let result = Classifier::classify_heuristic(&path, name);
    assert_ne!(
        result,
        SceneryCategory::Mesh,
        "'{}' should NOT be classified as generic Mesh (got {:?})",
        name,
        result
    );
    // Should be caught by Orbx B/C mesh rule → SpecificMesh
    assert_eq!(
        result,
        SceneryCategory::SpecificMesh,
        "'{}' should be SpecificMesh (airport-specific companion mesh), got {:?}",
        name,
        result
    );
}

#[test]
fn test_orbx_d_mesh_still_classified_as_mesh() {
    // Orbx_D_ packs are standalone terrain meshes — they should still be Mesh.
    // But they don't contain "mesh" in their exact name pattern usually...
    // Let's verify a generic non-Orbx mesh still works.
    let name = "zzz_UHD_Mesh_v4";
    let path = PathBuf::from(format!("Custom Scenery/{}", name));
    let result = Classifier::classify_heuristic(&path, name);
    assert_eq!(result, SceneryCategory::Mesh);
}

#[test]
fn test_icao_companion_packs_classified_as_specific_mesh() {
    // Non-Orbx companion packs with ICAO codes + mesh/terrain/grass keywords
    // should be SpecificMesh (not generic Mesh). They serve different purposes
    // (grass, terrain, sealane) for the same airport and must coexist without
    // triggering false mesh shadowing warnings.
    let cases = [
        ("EGLL_3Dgrass", SceneryCategory::SpecificMesh),
        ("EGLL_MESH", SceneryCategory::SpecificMesh),
        ("PAKT_Terrain_Northern_Sky_Studio", SceneryCategory::SpecificMesh),
        ("SFD_KLAX_Los_Angeles_HD_2_Mesh", SceneryCategory::SpecificMesh),
    ];

    for (name, expected) in &cases {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let result = Classifier::classify_heuristic(&path, name);
        assert_eq!(
            &result, expected,
            "'{}' should be {:?}, got {:?}",
            name, expected, result
        );
    }
}

#[test]
fn test_flytampa_mesh_still_classified_as_mesh() {
    // Non-Orbx packs with "mesh" should still be Mesh
    let name = "FlyTampa_Amsterdam_3_mesh";
    let path = PathBuf::from(format!("Custom Scenery/{}", name));
    let result = Classifier::classify_heuristic(&path, name);
    assert_eq!(
        result,
        SceneryCategory::Mesh,
        "Non-Orbx mesh packs should still be classified as Mesh"
    );
}
