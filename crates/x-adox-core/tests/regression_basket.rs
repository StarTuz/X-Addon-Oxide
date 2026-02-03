use std::path::PathBuf;
use tempfile::tempdir;
use x_adox_bitnet::{BitNetModel, PredictContext};
use x_adox_core::scenery::{
    SceneryCategory, SceneryDescriptor, SceneryManager, SceneryPack, SceneryPackType,
};

fn create_test_pack(name: &str) -> SceneryPack {
    SceneryPack {
        name: name.to_string(),
        path: PathBuf::from(name),
        raw_path: None,
        status: SceneryPackType::Active,
        category: SceneryCategory::Unknown,
        airports: Vec::new(),
        tiles: Vec::new(),
        tags: Vec::new(),
        descriptor: SceneryDescriptor::default(),
        region: None,
    }
}

#[test]
fn test_basket_drop_and_pin() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let mut sm = SceneryManager::new(root.join("scenery_packs.ini"));

    // Add some initial packs
    sm.packs.push(create_test_pack("Neighbor"));
    sm.packs.push(create_test_pack("Below"));

    // Create a mock model and set an override for Neighbor
    let mut model = BitNetModel::default();
    std::sync::Arc::make_mut(&mut model.config)
        .overrides
        .insert("Neighbor".to_string(), 15);

    let items_to_move = vec!["Moving".to_string()];

    // Add "Moving" to the "input" simulation
    sm.packs.insert(0, create_test_pack("Moving"));

    // Drop "Moving" between "Neighbor" and "Below"
    // List is currently: [Moving, Neighbor, Below]
    // After removal: [Neighbor, Below]
    // target_idx 1 means between them

    sm.drop_basket_at(
        &items_to_move,
        1,
        &mut model,
        &PredictContext::default(),
        true, // autopin
    );

    // Expected: [Moving, Neighbor, Below] (Semantic drop above Neighbor)
    assert_eq!(sm.packs[0].name, "Moving");
    assert_eq!(sm.packs[1].name, "Neighbor");
    assert_eq!(sm.packs[2].name, "Below");

    // "Moving" should be pinned to Neighbor's score (15)
    // In the new logic, neighbor_idx for index 0 is idx+1 (1) which is "Neighbor"
    assert_eq!(model.config.overrides.get("Moving"), Some(&15));
}

#[test]
fn test_basket_multi_move() {
    let mut sm = SceneryManager::new(PathBuf::new());
    sm.packs = vec![
        create_test_pack("A"),
        create_test_pack("B"),
        create_test_pack("C"),
    ];

    let mut model = BitNetModel::default();

    // Move A and C to index 1 (between A and B)
    // Semantically, user dropped at Gap 1 (above B)
    sm.drop_basket_at(
        &vec!["A".to_string(), "C".to_string()],
        1,
        &mut model,
        &PredictContext::default(),
        false,
    );

    // Expect: [A, C, B]
    // A stays at index 0 (above B)
    // C moves to index 1 (above B)
    assert_eq!(sm.packs[0].name, "A");
    assert_eq!(sm.packs[1].name, "C");
    assert_eq!(sm.packs[2].name, "B");
}
