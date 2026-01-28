use std::fs;
use tempfile::tempdir;
use x_adox_core::scenery::{discover_airports_in_pack, discover_tiles_in_pack};

#[test]
fn test_recursive_tile_nesting() {
    let dir = tempdir().unwrap();
    let pack_path = dir.path();

    // Deeply nested DSF
    // Pack/CustomSceneryLayer/Earth nav data/DSF/+50-010/+56-004.dsf
    let nav_path = pack_path
        .join("CustomSceneryLayer")
        .join("Earth nav data")
        .join("DSF")
        .join("+50-010");
    fs::create_dir_all(&nav_path).unwrap();
    fs::write(nav_path.join("+56-003.dsf"), "").unwrap();

    let tiles = discover_tiles_in_pack(pack_path);
    assert_eq!(tiles.len(), 1, "Should find the nested DSF tile");
    assert_eq!(tiles[0], (56, -3));
}

#[test]
fn test_case_insensitive_scanning() {
    let dir = tempdir().unwrap();
    let pack_path = dir.path();

    // Mixed case names (likely on Linux)
    // Pack/eaRTH nAV daTA/lat+50+020/+51+021.DSF
    let nav_path = pack_path.join("eaRTH nAV daTA").join("lat+50+020");
    fs::create_dir_all(&nav_path).unwrap();
    fs::write(nav_path.join("+51+021.DSF"), "").unwrap();

    // Also test apt.dat case
    fs::write(
        pack_path.join("eaRTH nAV daTA").join("APT.DAT"),
        "I\n1000 Version\n1 0 0 0 TEST Airport",
    )
    .unwrap();

    let tiles = discover_tiles_in_pack(pack_path);
    assert_eq!(tiles.len(), 1, "Should find tiles regardless of case");
    assert_eq!(tiles[0], (51, 21));

    let airports = discover_airports_in_pack(pack_path);
    assert_eq!(airports.len(), 1, "Should find apt.dat regardless of case");
    assert_eq!(airports[0].id, "TEST");
}

#[test]
fn test_multiple_roots_discovery() {
    let dir = tempdir().unwrap();
    let pack_path = dir.path();

    // Some packs have multiple 'layers' or folders that qualify as roots
    // Layer1/Earth nav data/...
    // Layer2/Earth nav data/...
    let root1 = pack_path
        .join("Layer1")
        .join("Earth nav data")
        .join("+50-010");
    let root2 = pack_path
        .join("Layer2")
        .join("Earth nav data")
        .join("+40-010");

    fs::create_dir_all(&root1).unwrap();
    fs::create_dir_all(&root2).unwrap();

    fs::write(root1.join("+51-009.dsf"), "").unwrap();
    fs::write(root2.join("+41-009.dsf"), "").unwrap();

    let tiles = discover_tiles_in_pack(pack_path);
    assert_eq!(tiles.len(), 2, "Should find tiles from multiple roots");
    assert!(tiles.contains(&(51, -9)));
    assert!(tiles.contains(&(41, -9)));
}
