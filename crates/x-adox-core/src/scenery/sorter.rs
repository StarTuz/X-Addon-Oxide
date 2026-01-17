use crate::scenery::{SceneryCategory, SceneryPack};

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
    packs.sort_by(|a, b| {
        let prio_a = a.category.priority();
        let prio_b = b.category.priority();

        if prio_a != prio_b {
            return prio_a.cmp(&prio_b);
        }

        // Secondary sort: Alphabetical by name
        a.name.cmp(&b.name)
    });
}
