// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

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
    // CRITICAL: Sorting must be stable and deterministic.
    // To prevent section fragmentation in scenery_packs.ini, we MUST use the 
    // matched rule name as a secondary tie-breaker after the priority score.
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
                // Pinned packs keep their exact position; do not reorder relative to others.
                if _name_a == x_adox_bitnet::PINNED_RULE_NAME || _name_b == x_adox_bitnet::PINNED_RULE_NAME {
                    return std::cmp::Ordering::Equal;
                }
                // Secondary Sort Rules: Group by Rule Name to prevent INI fragmentation
                // When scores are equal, items belonging to the same "# Section" must be adjacent.
                let section_a = x_adox_bitnet::canonical_section_name(&_name_a);
                let section_b = x_adox_bitnet::canonical_section_name(&_name_b);
                match section_a.cmp(&section_b) {
                    std::cmp::Ordering::Equal => {
                        // Tertiary Sort: SimHeaven specialized layers (if applicable)
                        if let Some((cont_a, layer_a)) = extract_simheaven_info(&a.name) {
                            if let Some((cont_b, layer_b)) = extract_simheaven_info(&b.name) {
                                match cont_a.cmp(&cont_b) {
                                    std::cmp::Ordering::Equal => {
                                        return layer_a
                                            .partial_cmp(&layer_b)
                                            .unwrap_or(std::cmp::Ordering::Equal);
                                    }
                                    ord => return ord,
                                }
                            } else {
                                return std::cmp::Ordering::Less;
                            }
                        } else if extract_simheaven_info(&b.name).is_some() {
                            return std::cmp::Ordering::Greater;
                        }

                        // Quaternary Sort: overlay-specific deterministic ordering.
                        // This keeps numbered families (e.g. Amsterdam 1/2 overlays) in natural order
                        // even when other overlay packs are interleaved.
                        if section_a == "Airport Overlays" && section_b == "Airport Overlays" {
                            let key_a = overlay_order_key(&a.name);
                            let key_b = overlay_order_key(&b.name);
                            if key_a != key_b {
                                return key_a.cmp(&key_b);
                            }
                        }

                        // Preserve user/discovery order for true ties.
                        // sort_by is stable, so returning Equal keeps relative order.
                        std::cmp::Ordering::Equal
                    }
                    ord => ord,
                }
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

fn extract_numbered_overlay_family(name: &str) -> Option<(String, u32)> {
    let lower = name.to_lowercase();
    if !lower.contains("overlay") {
        return None;
    }

    let bytes = lower.as_bytes();
    for i in 1..bytes.len().saturating_sub(1) {
        if bytes[i] == b'_' && bytes[i - 1].is_ascii_digit() {
            let mut start = i - 1;
            while start > 0 && bytes[start - 1].is_ascii_digit() {
                start -= 1;
            }
            if i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() {
                let num = lower[start..i].parse::<u32>().ok()?;
                let family = lower[..start].trim_end_matches('_').to_string();
                return Some((family, num));
            }
        }
    }

    None
}

fn overlay_order_key(name: &str) -> (String, u32, String) {
    let lower = name.to_lowercase();
    if let Some((family, part)) = extract_numbered_overlay_family(name) {
        (family, part, lower)
    } else {
        // Non-numbered overlays sort after numbered family parts by default.
        (lower.clone(), u32::MAX, lower)
    }
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
            region: None,
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
            max_runway_length: None,
            surface_type: None,
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

    #[test]
    fn test_numbered_overlay_family_ordering() {
        let mut packs = vec![
            make_pack("FlyTampa_Amsterdam_2_default_overlays"),
            make_pack("FlyTampa_Amsterdam_1_overlays"),
        ];
        let model = x_adox_bitnet::BitNetModel::default();

        sort_packs(&mut packs, Some(&model), &x_adox_bitnet::PredictContext::default());

        assert_eq!(packs[0].name, "FlyTampa_Amsterdam_1_overlays");
        assert_eq!(packs[1].name, "FlyTampa_Amsterdam_2_default_overlays");
    }

    #[test]
    fn test_numbered_overlay_family_ordering_with_interleaved_packs() {
        let mut packs = vec![
            make_pack("FlyTampa_Amsterdam_2_default_overlays"),
            make_pack("Aircraft-Static_and_Animated"),
            make_pack("DarkBlue-RJTT_Haneda_Overlays1"),
            make_pack("FlyTampa_Amsterdam_1_overlays"),
        ];
        let model = x_adox_bitnet::BitNetModel::default();
        let ctx = x_adox_bitnet::PredictContext::default();
        let (s1, r1) = model.predict_with_rule_name(
            "FlyTampa_Amsterdam_1_overlays",
            &std::path::PathBuf::from("FlyTampa_Amsterdam_1_overlays"),
            &ctx,
        );
        let (s2, r2) = model.predict_with_rule_name(
            "FlyTampa_Amsterdam_2_default_overlays",
            &std::path::PathBuf::from("FlyTampa_Amsterdam_2_default_overlays"),
            &ctx,
        );
        assert_eq!(r1, "Airport Overlays");
        assert_eq!(r2, "Airport Overlays");
        assert_eq!(s1, s2);

        sort_packs(&mut packs, Some(&model), &ctx);
        let idx_amsterdam_1 = packs
            .iter()
            .position(|p| p.name == "FlyTampa_Amsterdam_1_overlays")
            .unwrap();
        let idx_amsterdam_2 = packs
            .iter()
            .position(|p| p.name == "FlyTampa_Amsterdam_2_default_overlays")
            .unwrap();
        assert!(idx_amsterdam_1 < idx_amsterdam_2);
    }
}
