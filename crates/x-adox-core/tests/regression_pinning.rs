use std::path::PathBuf;
use std::sync::Arc;
use x_adox_bitnet::{BitNetModel, PredictContext, PINNED_RULE_NAME};
use x_adox_core::scenery::{
    sorter, SceneryCategory, SceneryDescriptor, SceneryPack, SceneryPackType,
};

fn create_dummy_pack(name: &str) -> SceneryPack {
    SceneryPack {
        name: name.to_string(),
        path: PathBuf::from(format!("Custom Scenery/{}", name)),
        raw_path: None,
        status: SceneryPackType::Active,
        category: SceneryCategory::Unknown,
        airports: Vec::new(),
        tiles: Vec::new(),
        tags: Vec::new(),
        descriptor: SceneryDescriptor::default(),
    }
}

#[test]
fn test_pinning_priority_over_alphabetical() {
    // Create a model with an override for a pack that would normally sort late
    let mut model = BitNetModel::default();

    // Normal Airport (Score 10, Name "Airports")
    let pack_a = create_dummy_pack("A_Normal_Airport");

    // Pinned Mesh (Pinned to Score 10, Name "Pinned / Manual Override")
    // "Pinned" starts with P, so alphabetically it would sort AFTER "Airports" (A)
    let pack_p = create_dummy_pack("P_Pinned_Mesh");

    // Correctly mutate the Arc
    Arc::make_mut(&mut model.config)
        .overrides
        .insert("P_Pinned_Mesh".to_string(), 10);

    let mut packs = vec![pack_a.clone(), pack_p.clone()];
    let context = PredictContext::default();

    // Perform sort
    sorter::sort_packs(&mut packs, Some(&model), &context);

    // ASSERTION: The pinned item MUST be first, even though "Pinned" comes after "Airports"
    assert_eq!(
        packs[0].name, "P_Pinned_Mesh",
        "Pinned item should be at the top of the score tier"
    );
    assert_eq!(packs[1].name, "A_Normal_Airport");
}

#[test]
fn test_multiple_pins_preserve_relative_order() {
    let mut model = BitNetModel::default();

    // Two packs, both pinned to Score 10.
    // In the input, B is ABOVE A.
    // Alphabetically A < B.
    // We want to verify that Smart Sort does NOT reorder them alphabetically.
    let pack_b = create_dummy_pack("B_Pinned");
    let pack_a = create_dummy_pack("A_Pinned");

    Arc::make_mut(&mut model.config)
        .overrides
        .insert("A_Pinned".to_string(), 10);
    Arc::make_mut(&mut model.config)
        .overrides
        .insert("B_Pinned".to_string(), 10);

    let mut packs = vec![pack_b.clone(), pack_a.clone()];
    let context = PredictContext::default();

    sorter::sort_packs(&mut packs, Some(&model), &context);

    // ASSERTION: B must still be above A because stable sort preserves input order for ties.
    assert_eq!(
        packs[0].name, "B_Pinned",
        "Multiple pins should preserve their manual relative order"
    );
    assert_eq!(packs[1].name, "A_Pinned");
}
