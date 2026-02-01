use crate::scenery::{SceneryCategory, SceneryPack};

impl SceneryCategory {
    // Higher value = Higher priority (loads first/top)
    pub fn score(&self) -> i32 {
        match self {
            SceneryCategory::CustomAirport => 100,
            SceneryCategory::OrbxAirport => 95,
            SceneryCategory::Landmark => 95,
            SceneryCategory::GlobalAirport => 90,
            SceneryCategory::Library => 85,
            SceneryCategory::AirportOverlay => 80,
            SceneryCategory::RegionalOverlay => 75,
            SceneryCategory::RegionalFluff => 70,
            SceneryCategory::AutoOrthoOverlay => 65,
            SceneryCategory::GlobalBase => 60,
            SceneryCategory::OrthoBase => 50,
            SceneryCategory::Mesh | SceneryCategory::SpecificMesh => 30,
            SceneryCategory::Unknown => 0,
            _ => 0,
        }
    }
}

// Custom sort implementation
pub fn sort_packs(
    packs: &mut [SceneryPack],
    model: Option<&x_adox_bitnet::BitNetModel>,
    context: &x_adox_bitnet::PredictContext,
) {
    packs.sort_by(|a, b| {
        // Calculate scores - use BitNet model if provided, otherwise fall back to category scores
        let (score_a, score_b, _name_a, _name_b, lower_is_better) = if let Some(m) = model {
            // BitNet: lower score = higher priority
            let mut ctx_a = context.clone();
            ctx_a.has_airports = !a.airports.is_empty();
            ctx_a.has_tiles = !a.tiles.is_empty();

            let mut ctx_b = context.clone();
            ctx_b.has_airports = !b.airports.is_empty();
            ctx_b.has_tiles = !b.tiles.is_empty();

            let (sa, na) = m.predict_with_rule_name(&a.name, &a.path, &ctx_a);
            let (sb, nb) = m.predict_with_rule_name(&b.name, &b.path, &ctx_b);
            (sa as i32, sb as i32, na, nb, true)
        } else {
            // Category-based: higher score = higher priority
            (
                calculate_score(a),
                calculate_score(b),
                String::new(),
                String::new(),
                false,
            )
        };

        // Primary sort by score
        let primary = if lower_is_better {
            score_a.cmp(&score_b) // ASCENDING for BitNet (lower = top)
        } else {
            score_b.cmp(&score_a) // DESCENDING for category (higher = top)
        };

        match primary {
            std::cmp::Ordering::Equal => {
                // Secondary Sort Rules (SimHeaven only)
                if let Some((cont_a, layer_a)) = extract_simheaven_info(&a.name) {
                    if let Some((cont_b, layer_b)) = extract_simheaven_info(&b.name) {
                        if cont_a == cont_b {
                            return layer_a
                                .partial_cmp(&layer_b)
                                .unwrap_or(std::cmp::Ordering::Equal);
                        }
                    }
                }

                // Pure Stability: items with the same score tier stay exactly where they were
                std::cmp::Ordering::Equal
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
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::Unknown,
            airports: Vec::new(),
            tiles: Vec::new(),
            tags: Vec::new(),
            descriptor: crate::scenery::SceneryDescriptor::default(),
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

    #[test]
    fn test_stable_sort_preserves_order() {
        // Create packs that will have the same score
        // CustomAirport score is 100
        let mut packs = vec![
            make_pack("aaa_Airport_B"), // Originally first
            make_pack("aaa_Airport_A"), // Originally second
        ];

        // Ensure they are both classified as CustomAirport (or at least same category)
        packs[0].category = SceneryCategory::CustomAirport;
        packs[1].category = SceneryCategory::CustomAirport;

        // Sort them
        sort_packs(&mut packs, None, &x_adox_bitnet::PredictContext::default());

        // Because we use stable sort and removed alphabetical tie-breaker,
        // B should still be before A despite A coming first alphabetically.
        assert_eq!(packs[0].name, "aaa_Airport_B");
        assert_eq!(packs[1].name, "aaa_Airport_A");
    }

    #[test]
    fn test_healed_mesh_whitelist() {
        use crate::scenery::classifier::Classifier;

        // 1. SimHeaven (Protected)
        let simheaven = "simHeaven_X-World_Europe-7-forests";
        let cat = Classifier::classify_heuristic(&PathBuf::from(simheaven), simheaven);
        assert_eq!(cat, SceneryCategory::RegionalOverlay);

        // Healing should NOT change it even with tiles and no airports
        let healed = Classifier::heal_classification(
            cat,
            false,
            true,
            &crate::scenery::SceneryDescriptor::default(),
        );
        assert_eq!(healed, SceneryCategory::RegionalOverlay);

        // 2. Random unknown pack with tiles and no airports (Unprotected)
        let unknown = "Random_Pack";
        let cat_unk = Classifier::classify_heuristic(&PathBuf::from(unknown), unknown);
        assert_eq!(cat_unk, SceneryCategory::Unknown);

        // Healing SHOULD turn it into Mesh
        let healed_unk = Classifier::heal_classification(
            cat_unk,
            false,
            true,
            &crate::scenery::SceneryDescriptor::default(),
        );
        assert_eq!(healed_unk, SceneryCategory::Mesh);

        // 3. AutoOrtho (Protected)
        let ao = "yAutoOrtho_Overlays";
        let cat_ao = Classifier::classify_heuristic(&PathBuf::from(ao), ao);
        assert_eq!(cat_ao, SceneryCategory::AutoOrthoOverlay);

        let healed_ao = Classifier::heal_classification(
            cat_ao,
            false,
            true,
            &crate::scenery::SceneryDescriptor::default(),
        );
        assert_eq!(healed_ao, SceneryCategory::AutoOrthoOverlay);
    }
}
