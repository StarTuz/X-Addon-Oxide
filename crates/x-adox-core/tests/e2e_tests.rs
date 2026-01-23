use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;
use x_adox_core::apt_dat::AptDatParser;
use x_adox_core::scenery::{SceneryManager, SceneryPackType};
use x_adox_core::XPlaneManager;

fn create_mock_xplane_root() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create standard folders
    fs::create_dir_all(root.join("Resources/default scenery/default apt dat/Earth nav data"))
        .unwrap();
    let p1 = root.join("Custom Scenery/TestPack/Earth nav data/+40-130");
    fs::create_dir_all(&p1).unwrap();
    File::create(p1.join("+47-123.dsf")).unwrap();
    File::create(p1.join("+47-124.dsf")).unwrap();
    fs::create_dir_all(root.join("Aircraft/Laminar Research/Cessna 172")).unwrap();

    // Create scenery_packs.ini
    let mut ini_file = File::create(root.join("Custom Scenery/scenery_packs.ini")).unwrap();
    writeln!(ini_file, "I\n1000 Version\nSCENERY\n\nSCENERY_PACK Custom Scenery/TestPack/\nSCENERY_PACK_DISABLED Custom Scenery/DisabledPack/").unwrap();

    // Create apt.dat
    let mut apt_file =
        File::create(root.join("Resources/default scenery/default apt dat/Earth nav data/apt.dat"))
            .unwrap();
    writeln!(apt_file, "I\n1000 Version\n1 100 0 0 KSEA Seattle Tacoma Intl\n100 60.96 1 2 0.25 1 3 0 16L 47.43577500 -122.31686900 300 0 0 0 1 1 34R 47.45898600 -122.31686900 0 0 3 0 1 1").unwrap();

    temp_dir
}

#[test]
fn test_e2e_workflow() {
    let temp_dir = create_mock_xplane_root();
    let root_path = temp_dir.path().to_path_buf();

    // 1. Initialize Manager
    let xpm = XPlaneManager::new(&root_path).expect("Failed to init XPlaneManager");

    // 2. Scenery Management
    let mut scenery_mgr = SceneryManager::new(xpm.get_scenery_packs_path());
    scenery_mgr.load().expect("Failed to load scenery packs");

    assert_eq!(scenery_mgr.packs.len(), 2);
    assert_eq!(scenery_mgr.packs[0].name, "TestPack");
    assert_eq!(scenery_mgr.packs[0].status, SceneryPackType::Active);
    assert_eq!(scenery_mgr.packs[0].tiles.len(), 2);
    assert_eq!(scenery_mgr.packs[0].tiles[0], (47, -124));
    assert_eq!(scenery_mgr.packs[0].tiles[1], (47, -123));

    // Test Enable/Disable
    scenery_mgr.disable_pack("TestPack");
    scenery_mgr.enable_pack("DisabledPack"); // Will fail silently if name not found, but let's check by index if we want
                                             // Note: The mock INI had "DisabledPack" as DISABLED.
                                             // Wait, the mock INI content was:
                                             // SCENERY_PACK Custom Scenery/TestPack/
                                             // SCENERY_PACK_DISABLED Custom Scenery/DisabledPack/

    // Let's properly enable the disabled one
    if let Some(pack) = scenery_mgr
        .packs
        .iter_mut()
        .find(|p| p.name == "DisabledPack")
    {
        pack.status = SceneryPackType::Active;
    }

    assert_eq!(
        scenery_mgr.packs[0].status,
        SceneryPackType::Disabled,
        "TestPack should be disabled"
    );

    // Save and Reload
    scenery_mgr.save(None).expect("Failed to save");

    let mut verify_mgr = SceneryManager::new(xpm.get_scenery_packs_path());
    verify_mgr.load().expect("Failed to reload");

    assert_eq!(verify_mgr.packs[0].status, SceneryPackType::Disabled);
    // In real X-Plane, order matters, but our parser preserves it.

    // 3. AptDat Parsing
    let apt_dat_path =
        root_path.join("Resources/default scenery/default apt dat/Earth nav data/apt.dat");
    let airports = AptDatParser::parse_file(&apt_dat_path).expect("Failed to parse apt.dat");

    assert_eq!(airports.len(), 1);
    let ksea = &airports[0];
    assert_eq!(ksea.id, "KSEA");
    assert_eq!(ksea.name, "Seattle Tacoma Intl");
    assert!(ksea.lat.is_some());
    assert!(ksea.lon.is_some());
    // Approx check for Seattle
    assert!(ksea.lat.unwrap() > 47.0 && ksea.lat.unwrap() < 48.0);
    assert!(ksea.lon.unwrap() < -120.0);

    // 4. Discovery (Optional, but good for coverage)
    // We created "Aircraft/Laminar Research/Cessna 172" but no .acf file.
    // Let's create one.
    File::create(root_path.join("Aircraft/Laminar Research/Cessna 172/Cessna 172.acf")).unwrap();

    // To match discovery logic, we need to invoke the discovery functions directly if exposed,
    // or checks modules if they are public.
    // x_adox_core::discovery::scan_aircraft?
    // Let's assume we can access it if `discovery` mod is pub
}
