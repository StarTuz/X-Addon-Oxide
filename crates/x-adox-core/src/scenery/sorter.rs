use crate::scenery::{SceneryCategory, SceneryPack};
use x_adox_bitnet::BitNetModel;

impl SceneryCategory {
    // Lower value = Higher priority (loads first)
    // Priority: Airport > GlobalAirport > Library > Overlay > Ortho > Mesh
    pub fn priority(&self) -> u8 {
        match self {
            SceneryCategory::EarthAirports | SceneryCategory::MarsAirports => 10,
            SceneryCategory::GlobalAirport => 20,
            SceneryCategory::Library => 30,
            SceneryCategory::Overlay | SceneryCategory::MarsScenery => 40,
            SceneryCategory::EarthScenery => 45, // Generic Earth Scenery
            SceneryCategory::Ortho => 50,
            SceneryCategory::Mesh => 60,
            SceneryCategory::Group | SceneryCategory::Unknown => 100,
        }
    }
}

// Custom sort implementation
pub fn sort_packs(packs: &mut [SceneryPack]) {
    // Attempt to load the BitNet model (even if it's currently a mock/distilled version)
    let model = BitNetModel::new().ok();

    packs.sort_by(|a, b| {
        // 1. BitNet Smart Sort (if available)
        if let Some(model) = &model {
            let score_a = model.predict(&a.name, &a.path);
            let score_b = model.predict(&b.name, &b.path);

            // If the model is confident in a difference, use it
            if score_a != score_b {
                return score_a.cmp(&score_b);
            }
        }

        // 2. Fallback to Category Priority
        let prio_a = a.category.priority();
        let prio_b = b.category.priority();

        if prio_a != prio_b {
            return prio_a.cmp(&prio_b);
        }

        // 3. Alphabetical tie-breaker
        a.name.cmp(&b.name)
    });
}
