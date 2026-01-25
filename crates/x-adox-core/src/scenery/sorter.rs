use crate::scenery::{SceneryCategory, SceneryPack};

impl SceneryCategory {
    // Higher value = Higher priority (loads first/top)
    pub fn score(&self) -> i32 {
        match self {
            SceneryCategory::CustomAirport => 100,
            SceneryCategory::OrbxAirport => 95,
            SceneryCategory::GlobalAirport => 90,
            SceneryCategory::Landmark => 88,
            SceneryCategory::RegionalOverlay => 85,
            SceneryCategory::RegionalFluff => 80,
            SceneryCategory::AirportOverlay => 75,
            SceneryCategory::AutoOrthoOverlay => 70, // Keep for now
            SceneryCategory::Library => 65,
            SceneryCategory::GlobalBase => 60,
            SceneryCategory::OrthoBase => 55,
            SceneryCategory::Mesh | SceneryCategory::SpecificMesh => 30,
            SceneryCategory::Unknown => 0, // Fallback safety net: sink to bottom
            _ => 0,
        }
    }
}

// Custom sort implementation
pub fn sort_packs(
    packs: &mut [SceneryPack],
    _model: Option<&x_adox_bitnet::BitNetModel>,
    _context: &x_adox_bitnet::PredictContext,
) {
    packs.sort_by(|a, b| {
        // Calculate Version 5.0 Scores
        let score_a = calculate_score(a);
        let score_b = calculate_score(b);

        // Sorting is DESCENDING (Higher Score = Top of file)
        match score_b.cmp(&score_a) {
            std::cmp::Ordering::Equal => {
                // Secondary Sort Rules

                // 1. SimHeaven Internal Order
                // Group by Continent FIRST, then by numeric layer (1-8).
                // This prevents "Continent mixing" (e.g., Australia interleaved with America).
                if let Some((cont_a, layer_a)) = extract_simheaven_info(&a.name) {
                    if let Some((cont_b, layer_b)) = extract_simheaven_info(&b.name) {
                        match cont_a.cmp(&cont_b) {
                            std::cmp::Ordering::Equal => {
                                // Within same continent, sort by layer (1-8)
                                return layer_a
                                    .partial_cmp(&layer_b)
                                    .unwrap_or(std::cmp::Ordering::Equal);
                            }
                            ord => return ord,
                        }
                    }
                }

                // 2. Alphabetical (Ascending) for ties
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
            ord => ord,
        }
    });
}

fn calculate_score(pack: &SceneryPack) -> i32 {
    let mut score = pack.category.score();
    let name_lower = pack.name.to_lowercase();

    // VFR Boost check (+5)
    // "If name contains '_vfr' or 'VFR' -> +5 points"
    // EXCEPTION: SimHeaven packs manage their own VFR layer (Layer 1) internally.
    // Boosting them breaks the Continent grouping (splits Layer 1 from others).
    if (name_lower.contains("_vfr") || name_lower.contains("vfr"))
        && extract_simheaven_info(&pack.name).is_none()
    {
        score += 5;
    }

    // y/z Prefix Penalty (-20)
    // "Exception: Airport matches (CustomAirport OR AirportOverlay) OR System blocks (Library, GlobalBase) ignore this"
    if pack.category != SceneryCategory::CustomAirport
        && pack.category != SceneryCategory::AirportOverlay
        && pack.category != SceneryCategory::Library
        && pack.category != SceneryCategory::GlobalBase
    {
        if name_lower.starts_with('y') || name_lower.starts_with('z') {
            score -= 20;
        }
    }

    // Mesh Protection (Cap at 30)
    // "Any 'Mesh' in name -> cap/force <=30"
    if name_lower.contains("mesh") {
        score = 30; // Force exact 30 per spec
    }

    score
}

fn extract_simheaven_info(name: &str) -> Option<(String, f32)> {
    let lower = name.to_lowercase();
    if !lower.contains("x-world") && !lower.contains("simheaven") {
        return None;
    }

    // Extract continent: "simHeaven_X-World_Europe-1-vfr" -> "europe"
    let continent = if lower.contains("america") {
        "america"
    } else if lower.contains("europe") {
        "europe"
    } else if lower.contains("australia") || lower.contains("oceania") {
        "australia"
    } else if lower.contains("africa") {
        "africa"
    } else if lower.contains("asia") {
        "asia"
    } else if lower.contains("antarctica") {
        "antarctica"
    } else {
        "z_other" // Sinks to bottom of same-layer groups
    }
    .to_string();

    let layer = if lower.contains("-1-") || lower.contains("1-vfr") {
        1.0
    } else if lower.contains("-2-") || lower.contains("2-region") {
        2.0
    } else if lower.contains("-3-") || lower.contains("3-detail") {
        3.0
    } else if lower.contains("-4-") || lower.contains("4-extra") {
        4.0
    } else if lower.contains("-5-") || lower.contains("5-footprint") {
        5.0
    } else if lower.contains("-6-") || lower.contains("6-scenery") {
        6.0
    } else if lower.contains("vegetation_library") {
        6.5 // After Scenery (6) but before Forests (7)
    } else if lower.contains("-7-") || lower.contains("7-forest") {
        7.0
    } else if lower.contains("-8-") || lower.contains("8-network") {
        8.0
    } else {
        9.0 // Unknown layer
    };

    Some((continent, layer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenery::{SceneryCategory, SceneryPack, SceneryPackType};
    use std::path::PathBuf;

    fn make_pack(name: &str) -> SceneryPack {
        let mut pack = SceneryPack {
            name: name.to_string(),
            path: PathBuf::from(name),
            status: SceneryPackType::Active,
            category: SceneryCategory::Unknown,
            airports: Vec::new(),
            tiles: Vec::new(),
            tags: Vec::new(),
        };
        // Re-classify using the same heuristic as the manager
        pack.category =
            crate::scenery::classifier::Classifier::classify_heuristic(&pack.path, &pack.name);
        pack
    }

    #[test]
    fn test_vfr_boost() {
        let mut packs = vec![
            make_pack("simHeaven_X-World_Europe-1-vfr"), // Score 85 + 5 = 90
            make_pack("simHeaven_X-World_Europe-2-regions"), // Score 85
        ];
        sort_packs(&mut packs, None, &x_adox_bitnet::PredictContext::default());
        assert_eq!(packs[0].name, "simHeaven_X-World_Europe-1-vfr");
    }

    #[test]
    fn test_unusual_airport_names() {
        let mut packs = vec![
            make_pack("X-Plane 12 Global Scenery"), // Global Base (Score 60)
            make_pack("KPSP_Palm_Springs_xp12_Axonos"), // Matches ICAO Regex -> Score 100
            make_pack("X-scenery_UKOO_XP12"),       // Unknown by name -> Score 0 -> promoted below
            make_pack("z_ao_na"),                   // Ortho (Score 35)
        ];

        // Force UKOO to be an airport manually to simulate post-discovery promotion
        packs[2].category = SceneryCategory::CustomAirport;

        sort_packs(&mut packs, None, &x_adox_bitnet::PredictContext::default());

        // Expected Order:
        // 1. KPSP_Palm_Springs_xp12_Axonos (Score 100)
        // 2. X-scenery_UKOO_XP12 (Score 100)
        // 3. X-Plane 12 Global Scenery (Score 60)
        // 4. z_ao_na (Score 35)

        assert_eq!(packs[0].name, "KPSP_Palm_Springs_xp12_Axonos");
        assert_eq!(packs[1].name, "X-scenery_UKOO_XP12");
        assert_eq!(packs[2].name, "X-Plane 12 Global Scenery");
        assert_eq!(packs[3].name, "z_ao_na");
    }

    #[test]
    fn test_simheaven_layer_priority() {
        let mut packs = vec![
            make_pack("simHeaven_X-World_Europe-2-regions"),
            make_pack("simHeaven_X-World_America-1-vfr"),
            make_pack("simHeaven_X-World_Europe-1-vfr"),
            make_pack("simHeaven_X-World_America-2-regions"),
        ];

        sort_packs(&mut packs, None, &x_adox_bitnet::PredictContext::default());

        // Expected Order: 1-vfr (America, then Europe), then 2-regions (America, then Europe)
        // Expected Order: Continent-grouped.
        // America-1, America-2, then Europe-1, Europe-2.
        assert_eq!(packs[0].name, "simHeaven_X-World_America-1-vfr");
        assert_eq!(packs[1].name, "simHeaven_X-World_America-2-regions");
        assert_eq!(packs[2].name, "simHeaven_X-World_Europe-1-vfr");
        assert_eq!(packs[3].name, "simHeaven_X-World_Europe-2-regions");
    }

    #[test]
    fn test_robust_airport_promotion() {
        let mut packs = vec![
            make_pack("Some_Unknown_Regional_Airport"), // Heuristic -> Unknown
            make_pack("Global Airports"),               // Heuristic -> GlobalAirport (90)
        ];

        // Simulate discovery finding 1 airport in the first pack
        packs[0].airports.push(crate::apt_dat::Airport {
            id: "YCKN".to_string(),
            name: "Cooktown".to_string(),
            airport_type: crate::apt_dat::AirportType::Land,
            lat: Some(0.0),
            lon: Some(0.0),
            proj_x: None,
            proj_y: None,
        });

        // Simulate the "Post-Discovery Promotion" logic from SceneryManager::load
        for pack in &mut packs {
            if !pack.airports.is_empty() {
                match pack.category {
                    SceneryCategory::GlobalAirport
                    | SceneryCategory::Library
                    | SceneryCategory::GlobalBase => {}
                    _ => pack.category = SceneryCategory::CustomAirport,
                }
            }
        }

        sort_packs(&mut packs, None, &x_adox_bitnet::PredictContext::default());

        // Now the Unknown Airport should be CustomAirport (100) and sort ABOVE Global (90)
        assert_eq!(packs[0].name, "Some_Unknown_Regional_Airport");
        assert_eq!(packs[0].category, SceneryCategory::CustomAirport);
        assert_eq!(packs[1].name, "Global Airports");
    }
}
