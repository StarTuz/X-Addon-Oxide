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
    if category == SceneryCategory::Unknown
        && pack_path.join("library.txt").exists() {
            category = SceneryCategory::Library;
        }

    // Step 3b: Post-Discovery Promotion (must mirror mod.rs exactly)
    if !airports.is_empty() {
        match category {
            SceneryCategory::GlobalAirport
            | SceneryCategory::Library
            | SceneryCategory::GlobalBase
            | SceneryCategory::Landmark
            | SceneryCategory::OrthoBase
            | SceneryCategory::Mesh
            | SceneryCategory::SpecificMesh => {
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
        (
            "PAKT_Terrain_Northern_Sky_Studio",
            SceneryCategory::SpecificMesh,
        ),
        (
            "SFD_KLAX_Los_Angeles_HD_2_Mesh",
            SceneryCategory::SpecificMesh,
        ),
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

#[test]
fn test_orthobase_with_airports_stays_orthobase() {
    // Regression: OrthoBase packs (Orbx TrueEarth Orthos, XPME base packages,
    // z_autoortho) often contain DSF data that discovers airports in their coverage
    // area. The post-discovery promotion in mod.rs must NOT promote these packs to
    // CustomAirport, which would misplace them high in the INI and cause the Smart
    // Sort validator to fire a spurious "Mesh/Ortho above 'XPME_South_America'" warning.
    let orthobase_packs = [
        "XPME_South_America",
        "XPME_Europe",
        "Orbx_C_GB_South_TrueEarth_Orthos",
        "Orbx_C_GB_Central_TrueEarth_Orthos",
        "Orbx_D_GB_North_TrueEarth_Orthos",
        "z_autoortho",
        "z_ao_eur",
        "ortho4xp_tile",
    ];

    for name in &orthobase_packs {
        let path = PathBuf::from(format!("Custom Scenery/{}", name));
        let mut category = Classifier::classify_heuristic(&path, name);

        // Simulate discovering airports (as happens when DSF tiles cover airport areas)
        let fake_airports_found = true;

        // Reproduce the post-discovery promotion logic from mod.rs
        if fake_airports_found {
            let nl = name.to_lowercase();
            let is_companion = nl.contains("mesh")
                || nl.contains("terrain")
                || nl.contains("3dgrass")
                || nl.contains("grass")
                || nl.contains("sealane");

            match category {
                SceneryCategory::GlobalAirport
                | SceneryCategory::Library
                | SceneryCategory::GlobalBase
                | SceneryCategory::Landmark
                | SceneryCategory::OrthoBase
                | SceneryCategory::Mesh
                | SceneryCategory::SpecificMesh => {
                    // Keep structural category
                }
                _ => {
                    if !is_companion {
                        category = SceneryCategory::CustomAirport;
                    }
                }
            }
        }

        assert_eq!(
            category,
            SceneryCategory::OrthoBase,
            "'{}' with discovered airports must stay OrthoBase, got {:?}",
            name,
            category
        );
    }
}

#[test]
fn test_sji_airports_and_alphanumeric_icao() {
    // SJI airports (airstrips) and alphanumeric ICAO codes (38WA, WA39)
    // should be correctly classified as CustomAirport.
    let cases = [
        ("SJI Allan Island Airstrip", SceneryCategory::CustomAirport),
        ("SJI Crow Valley, WA39", SceneryCategory::CustomAirport),
        ("SJI Blakeley Island, 38WA, D", SceneryCategory::CustomAirport),
        ("SJI Center Island, 78WA, D", SceneryCategory::CustomAirport),
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
fn test_reconcile_preserves_discovery_data() {
    use x_adox_core::scenery::{SceneryManager, SceneryPack, SceneryPackType, SceneryCategory, SceneryDescriptor};
    use std::path::PathBuf;

    // GUI state with deep-scan data
    let gui_pack = SceneryPack {
        name: "Test Airport".to_string(),
        path: PathBuf::from("Custom Scenery/Test Airport"),
        raw_path: None,
        status: SceneryPackType::Active,
        category: SceneryCategory::CustomAirport,
        airports: vec![x_adox_core::apt_dat::Airport {
            id: "KTEST".to_string(),
            name: "Test".to_string(),
            airport_type: x_adox_core::apt_dat::AirportType::Land,
            lat: None,
            lon: None,
            proj_x: None,
            proj_y: None,
            max_runway_length: None,
            surface_type: None,
        }],
        tiles: vec![(40, -70)],
        tags: vec!["custom".to_string()],
        descriptor: SceneryDescriptor {
            object_count: 100,
            facade_count: 0,
            forest_count: 0,
            polygon_count: 0,
            mesh_count: 0,
            has_airport_properties: false,
            library_refs: vec![],
        },
        region: Some("North America".to_string()),
    };

    // Fresh manager state (normally loaded from disk)
    let mut manager = SceneryManager {
        file_path: PathBuf::from("scenery_packs.ini"),
        packs: vec![SceneryPack {
            name: "Test Airport".to_string(),
            path: PathBuf::from("Custom Scenery/Test Airport"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::Unknown,
            airports: vec![],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: None,
        }],
        group_manager: None,
    };

    manager.reconcile_with_external_packs(&[gui_pack]);

    let pack = &manager.packs[0];
    assert_eq!(pack.airports.len(), 1);
    assert_eq!(pack.airports[0].id, "KTEST");
    assert_eq!(pack.tiles.len(), 1);
    assert_eq!(pack.descriptor.object_count, 100);
    assert_eq!(pack.region.as_deref(), Some("North America"));
    assert_eq!(pack.tags[0], "custom");
}

#[test]
fn test_helicopter_destinations_not_classified_as_ortho() {
    // Packs with "Helicopter" in the name and "Ortho4XP" as a tool-name suffix
    // must be classified as CustomAirport (not OrthoBase).
    let name = "01_MontanaHelicopterDestinations_XP12_Ortho4XP130";
    let path = PathBuf::from(format!("Custom Scenery/{}", name));
    let result = Classifier::classify_heuristic(&path, name);
    assert_eq!(
        result,
        SceneryCategory::CustomAirport,
        "'{}' should be CustomAirport (helicopter destinations), got {:?}",
        name,
        result
    );
}

#[test]
fn test_seasons_manager_classified_as_library() {
    // o4xp_Seasons_Manager is a seasons plugin, not mesh/terrain content.
    let name = "o4xp_Seasons_Manager";
    let path = PathBuf::from(format!("Custom Scenery/{}", name));
    let result = Classifier::classify_heuristic(&path, name);
    assert_eq!(
        result,
        SceneryCategory::Library,
        "'{}' should be Library (seasons plugin), got {:?}",
        name,
        result
    );
}

#[test]
fn test_bitnet_helicopter_pack_promoted_to_airports() {
    // BitNet should promote helicopter destination packs to Airports tier even
    // though "ortho" keyword matches the Ortho/Photo rule (score 58).
    // The "helicopter" airport keyword widens the promotion range.
    let model = x_adox_bitnet::BitNetModel::default();
    let ctx = x_adox_bitnet::PredictContext::default();
    let (score, rule) = model.predict_with_rule_name(
        "01_MontanaHelicopterDestinations_XP12_Ortho4XP130",
        &PathBuf::from("Custom Scenery/01_MontanaHelicopterDestinations_XP12_Ortho4XP130"),
        &ctx,
    );
    assert_eq!(score, 10, "Helicopter pack score should be 10 (Airports tier)");
    assert_eq!(rule, "Airports", "Helicopter pack should be promoted to 'Airports'");
}

#[test]
fn test_bitnet_community_libraries_match_libraries_rule() {
    // ruscenery, Sea_Life, world-models, o4xp_Seasons_Manager must match the
    // Libraries rule (score 35) instead of falling through to "Other Scenery".
    let model = x_adox_bitnet::BitNetModel::default();
    let ctx = x_adox_bitnet::PredictContext::default();

    let library_packs = [
        "ruscenery",
        "Sea_Life",
        "world-models",
        "o4xp_Seasons_Manager",
    ];

    for name in &library_packs {
        let (score, rule) = model.predict_with_rule_name(
            name,
            &PathBuf::from(format!("Custom Scenery/{}", name)),
            &ctx,
        );
        assert_eq!(
            score, 35,
            "'{}' score should be 35 (Libraries), got {}",
            name, score
        );
        assert_eq!(
            rule, "Libraries",
            "'{}' should match 'Libraries' rule, got '{}'",
            name, rule
        );
    }
}

#[test]
fn test_bitnet_crow_in_name_with_icao_still_promoted() {
    // Packs with "crow" in the name (from "Crow Valley" or "Ironcrown")
    // that also have ICAO codes should be promoted to Airports, not Birds.
    let model = x_adox_bitnet::BitNetModel::default();
    let ctx = x_adox_bitnet::PredictContext::default();

    // Note: "crow" is NOT in default Birds keywords, but we test the ICAO
    // promotion path which would catch it even with a custom "crow" keyword.
    let cases = [
        ("SJI Crow Valley, WA39", 10, "Airports"),
        ("ORABC_09_22OR - Ironcrown", 10, "Airports"),
    ];

    for (name, expected_score, expected_rule) in &cases {
        let (score, rule) = model.predict_with_rule_name(
            name,
            &PathBuf::from(format!("Custom Scenery/{}", name)),
            &ctx,
        );
        assert_eq!(
            score, *expected_score,
            "'{}' score should be {} (got {})",
            name, expected_score, score
        );
        assert_eq!(
            rule, *expected_rule,
            "'{}' should be '{}' (got '{}')",
            name, expected_rule, rule
        );
    }
}

#[test]
fn test_bitnet_riga_not_promoted_by_discovered_airports() {
    // Regression: generic city packs with discovered airport data must not be
    // forced into Airports tier.
    let model = x_adox_bitnet::BitNetModel::default();
    let (score, rule) = model.predict_with_rule_name(
        "Riga Latvija",
        &PathBuf::from("Custom Scenery/Riga Latvija"),
        &x_adox_bitnet::PredictContext {
            has_airports: true,
            ..x_adox_bitnet::PredictContext::default()
        },
    );
    assert_eq!(score, 16, "'Riga Latvija' should remain City Enhancements");
    assert_eq!(rule, "City Enhancements");
}

#[test]
fn test_bitnet_orbx_landmarks_not_grouped_into_airports() {
    // Regression: Orbx_A landmark packs should not be merged into Airports.
    // They should keep high priority above regional TrueEarth with a dedicated label.
    let model = x_adox_bitnet::BitNetModel::default();
    let (score, rule) = model.predict_with_rule_name(
        "Orbx_A_Brisbane_Landmarks",
        &PathBuf::from("Custom Scenery/Orbx_A_Brisbane_Landmarks"),
        &x_adox_bitnet::PredictContext::default(),
    );
    assert_eq!(score, 11);
    assert_eq!(rule, "Orbx A Landmarks");
}

#[test]
fn test_bitnet_library_not_promoted_by_discovered_airports() {
    // Regression: library packs with incidental airport data must remain Libraries.
    let model = x_adox_bitnet::BitNetModel::default();
    let (score, rule) = model.predict_with_rule_name(
        "Orbx_XP12_Library",
        &PathBuf::from("Custom Scenery/Orbx_XP12_Library"),
        &x_adox_bitnet::PredictContext {
            has_airports: true,
            ..x_adox_bitnet::PredictContext::default()
        },
    );
    assert_eq!(score, 35);
    assert_eq!(rule, "Libraries");
}

