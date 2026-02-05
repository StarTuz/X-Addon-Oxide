use std::path::PathBuf;
use x_adox_core::scenery::sorter::sort_packs;
use x_adox_core::scenery::{SceneryCategory, SceneryPack, SceneryPackType};

fn make_pack(name: &str, category: SceneryCategory) -> SceneryPack {
    SceneryPack {
        name: name.to_string(),
        path: PathBuf::from(name),
        raw_path: None,
        status: SceneryPackType::Active,
        category,
        airports: Vec::new(),
        tiles: Vec::new(),
        tags: Vec::new(),
        descriptor: Default::default(),
        region: None,
    }
}

#[test]
fn test_simheaven_layer_numerical_sorting() {
    let mut packs = vec![
        make_pack(
            "simHeaven_X-World_Europe-8-network",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_Europe-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_Europe-5-footprints",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_Europe-3-details",
            SceneryCategory::RegionalOverlay,
        ),
    ];

    sort_packs(&mut packs, None, &x_adox_bitnet::PredictContext::default());

    // Expected: 1, 3, 5, 8
    assert_eq!(packs[0].name, "simHeaven_X-World_Europe-1-vfr");
    assert_eq!(packs[1].name, "simHeaven_X-World_Europe-3-details");
    assert_eq!(packs[2].name, "simHeaven_X-World_Europe-5-footprints");
    assert_eq!(packs[3].name, "simHeaven_X-World_Europe-8-network");
}

#[test]
fn test_simheaven_continent_grouping() {
    let mut packs = vec![
        make_pack(
            "simHeaven_X-World_Europe-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_America-2-regions",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_Europe-2-regions",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_America-1-vfr",
            SceneryCategory::RegionalOverlay,
        ),
    ];

    sort_packs(&mut packs, None, &x_adox_bitnet::PredictContext::default());

    // Expected: America (1, 2), then Europe (1, 2)
    assert_eq!(packs[0].name, "simHeaven_X-World_America-1-vfr");
    assert_eq!(packs[1].name, "simHeaven_X-World_America-2-regions");
    assert_eq!(packs[2].name, "simHeaven_X-World_Europe-1-vfr");
    assert_eq!(packs[3].name, "simHeaven_X-World_Europe-2-regions");
}

#[test]
fn test_simheaven_vegetation_library_position() {
    let mut packs = vec![
        make_pack(
            "simHeaven_X-World_Europe-7-forests",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_Europe-6-scenery",
            SceneryCategory::RegionalOverlay,
        ),
        make_pack(
            "simHeaven_X-World_Europe_Vegetation_Library",
            SceneryCategory::RegionalOverlay,
        ),
    ];

    sort_packs(&mut packs, None, &x_adox_bitnet::PredictContext::default());

    // Expected: 6, Vegetation (6.5), 7
    assert_eq!(packs[0].name, "simHeaven_X-World_Europe-6-scenery");
    assert_eq!(packs[1].name, "simHeaven_X-World_Europe_Vegetation_Library");
    assert_eq!(packs[2].name, "simHeaven_X-World_Europe-7-forests");
}
