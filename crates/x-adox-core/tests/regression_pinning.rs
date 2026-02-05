// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use std::path::PathBuf;
use std::sync::Arc;
use x_adox_bitnet::{BitNetModel, PredictContext};
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
        region: None,
    }
}

#[test]
fn test_pin_preserves_local_position() {
    // We want to verify that a pin stays exactly where the user put it,
    // even if it's interleaved with other items in the same score tier.
    let mut model = BitNetModel::default();

    // Normal Airport A (Score 10)
    let pack_a = create_dummy_pack("A_Airport");

    // Pinned Mesh (Pinned to Score 10) -> Placed between two airports
    let pack_p = create_dummy_pack("P_Pinned_Mesh");
    Arc::make_mut(&mut model.config)
        .overrides
        .insert("P_Pinned_Mesh".to_string(), 10);

    // Normal Airport B (Score 10)
    let pack_b = create_dummy_pack("B_Airport");

    // Input order: A, P, B
    let mut packs = vec![pack_a.clone(), pack_p.clone(), pack_b.clone()];
    let context = PredictContext::default();

    // Perform sort
    sorter::sort_packs(&mut packs, Some(&model), &context);

    // ASSERTION: Order MUST be preserved as A, P, B.
    // P should NOT float to the top. B should NOT jump above A alphabetically.
    assert_eq!(packs[0].name, "A_Airport");
    assert_eq!(packs[1].name, "P_Pinned_Mesh");
    assert_eq!(packs[2].name, "B_Airport");
}

#[test]
fn test_multiple_pins_preserve_relative_order() {
    let mut model = BitNetModel::default();

    // Verify that even if pins are adjacent, they don't reorder themselves alphabetically.
    let pack_b = create_dummy_pack("B_Pinned");
    let pack_a = create_dummy_pack("A_Pinned");

    Arc::make_mut(&mut model.config)
        .overrides
        .insert("A_Pinned".to_string(), 10);
    Arc::make_mut(&mut model.config)
        .overrides
        .insert("B_Pinned".to_string(), 10);

    // Input order: B then A
    let mut packs = vec![pack_b.clone(), pack_a.clone()];
    let context = PredictContext::default();

    sorter::sort_packs(&mut packs, Some(&model), &context);

    // ASSERTION: Order must be B then A.
    assert_eq!(packs[0].name, "B_Pinned");
    assert_eq!(packs[1].name, "A_Pinned");
}
