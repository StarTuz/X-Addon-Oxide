// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use crate::scenery::SceneryPack;


// Custom sort implementation
pub fn sort_packs(
    packs: &mut [SceneryPack],
    model: Option<&x_adox_bitnet::BitNetModel>,
    context: &x_adox_bitnet::PredictContext,
) {
    if packs.is_empty() {
        return;
    }

    // Pre-calculate all sort metadata once per pack to avoid redundant calls
    // to BitNet/Classifier and to allow for single-pass logging of contradictions.
    // Category is captured here so pack.category stays in sync with the sort score —
    // this prevents the validator from seeing stale categories set during load_quick()
    // with empty context (e.g. an unmatched pack loaded as LowImpactOverlay that sorts
    // at score 55 because has_tiles=true in enriched context).
    let mut sort_data: Vec<(usize, i32, x_adox_bitnet::SceneryCategory, String)> = packs
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let mut ctx = context.clone();
            ctx.has_airports = !p.airports.is_empty();
            ctx.has_tiles = !p.tiles.is_empty();
            ctx.object_count = p.descriptor.object_count;
            ctx.facade_count = p.descriptor.facade_count;
            ctx.has_airport_properties = p.descriptor.has_airport_properties;

            let (score, category, rule) = if let Some(m) = model {
                m.predict_with_rule_name(&p.name, &p.path, &ctx)
            } else {
                // Fallback to default model heuristics if no external model provided
                let m = x_adox_bitnet::BitNetModel::default();
                m.predict_with_rule_name(&p.name, &p.path, &ctx)
            };

            (idx, score as i32, category, rule)
        })
        .collect();

    // Perform the actual sort using the pre-calculated data
    sort_data.sort_by(|&(idx_a, score_a, _, ref rule_a), &(idx_b, score_b, _, ref rule_b)| {
        let a = &packs[idx_a];
        let b = &packs[idx_b];

        // Primary sort by BitNet score (ASCENDING: lower = top)
        let primary = score_a.cmp(&score_b);

        match primary {
            std::cmp::Ordering::Equal => {
                // Pinned packs keep their exact position; do not reorder relative to others.
                if rule_a.as_str() == x_adox_bitnet::PINNED_RULE_NAME
                    || rule_b.as_str() == x_adox_bitnet::PINNED_RULE_NAME
                {
                    return std::cmp::Ordering::Equal;
                }
                // Secondary Sort Rules: Group by Rule Name to prevent INI fragmentation
                let section_a = x_adox_bitnet::canonical_section_name(rule_a);
                let section_b = x_adox_bitnet::canonical_section_name(rule_b);
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
                        if section_a == "Airport Overlays" && section_b == "Airport Overlays" {
                            let key_a = overlay_order_key(&a.name);
                            let key_b = overlay_order_key(&b.name);
                            if key_a != key_b {
                                return key_a.cmp(&key_b);
                            }
                        }

                        // Preserve original order for true ties (stable sort).
                        // With index-based sorting, compare indices to maintain stability.
                        idx_a.cmp(&idx_b)
                    }
                    ord => ord,
                }
            }
            ord => ord,
        }
    });

    // Reorder the original vector and update each pack's category to match what
    // sort_packs computed with enriched context. This keeps pack.category in sync
    // with the sort score so the validator never sees stale load_quick() categories.
    let sorted_packs: Vec<SceneryPack> = sort_data
        .into_iter()
        .map(|(idx, _, category, _)| {
            let mut pack = packs[idx].clone();
            // Only update category if it's a meaningful classification (not Group,
            // which is user-assigned and must be preserved).
            if pack.category != x_adox_bitnet::SceneryCategory::Group {
                pack.category = category;
            }
            pack
        })
        .collect();

    for (i, p) in sorted_packs.into_iter().enumerate() {
        packs[i] = p;
    }
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
        let context = x_adox_bitnet::PredictContext {
            has_airports: !pack.airports.is_empty(),
            has_tiles: !pack.tiles.is_empty(),
            ..Default::default()
        };
        pack.category =
            crate::scenery::classifier::Classifier::classify(&pack.name, &pack.path, &context, &x_adox_bitnet::BitNetModel::default());
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
            id: "EGLL".to_string(),
            name: "London Heathrow".to_string(),
            airport_type: crate::apt_dat::AirportType::Land,
            lat: Some(51.47),
            lon: Some(-0.45),
            proj_x: None,
            proj_y: None,
            max_runway_length: Some(3902),
            surface_type: Some(crate::apt_dat::SurfaceType::Hard),
            elevation_ft: None,
            frequencies: Vec::new(),
            city: None,
            country: None,
            max_runway_width: None,
            has_lighting: false,
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
        let model = x_adox_bitnet::BitNetModel::default();

        // 1. SimHeaven (Protected)
        // Even with tiles and no airports, it should stay RegionalOverlay
        let simheaven = "simHeaven_X-World_Europe-7-forests";
        let context = x_adox_bitnet::PredictContext {
            has_tiles: true,
            has_airports: false,
            ..Default::default()
        };
        let cat = Classifier::classify(simheaven, &PathBuf::from(simheaven), &context, &model);
        assert_eq!(cat, SceneryCategory::RegionalOverlay);

        // 2. Random unknown pack with tiles and no airports (Unprotected)
        // This SHOULD be healed to Mesh
        let unknown = "Random_Pack";
        let context_unk = x_adox_bitnet::PredictContext {
            has_tiles: true,
            has_airports: false,
            object_count: 10,
            ..Default::default()
        };
        let cat_unk = Classifier::classify(unknown, &PathBuf::from(unknown), &context_unk, &model);
        assert_eq!(cat_unk, SceneryCategory::Mesh);

        // 3. AutoOrtho (Protected)
        // Should stay AutoOrtho Overlay
        let ao = "yAutoOrtho_Overlays";
        let context_ao = x_adox_bitnet::PredictContext {
            has_tiles: true,
            has_airports: false,
            ..Default::default()
        };
        let cat_ao = Classifier::classify(ao, &PathBuf::from(ao), &context_ao, &model);
        assert_eq!(cat_ao, SceneryCategory::AutoOrthoOverlay);
    }

    #[test]
    fn test_numbered_overlay_family_ordering() {
        let mut packs = vec![
            make_pack("FlyTampa_Amsterdam_2_default_overlays"),
            make_pack("FlyTampa_Amsterdam_1_overlays"),
        ];
        let model = x_adox_bitnet::BitNetModel::default();

        sort_packs(
            &mut packs,
            Some(&model),
            &x_adox_bitnet::PredictContext::default(),
        );

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
        let (s1, _, r1) = model.predict_with_rule_name(
            "FlyTampa_Amsterdam_1_overlays",
            &std::path::PathBuf::from("FlyTampa_Amsterdam_1_overlays"),
            &ctx,
        );
        let (s2, _, r2) = model.predict_with_rule_name(
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

    #[test]
    fn test_orbx_c_orthos_sorts_below_simheaven() {
        // CRITICAL: Orbx C TrueEarth Orthos (score 58) must sort BELOW
        // SimHeaven overlays (score 20) in the pack list.
        // "Below" in sorted order = higher index = appears later in INI = lower priority
        let mut packs = vec![
            make_pack("Orbx_C_GB_South_TrueEarth_Orthos"),
            make_pack("simHeaven_X-World_Europe-4-extras"),
            make_pack("Orbx_C_GB_North_TrueEarth_Orthos"),
            make_pack("simHeaven_X-World_Europe-1-vfr"),
        ];

        let model = x_adox_bitnet::BitNetModel::default();
        sort_packs(&mut packs, Some(&model), &x_adox_bitnet::PredictContext::default());

        // Find positions
        let orbx_south_idx = packs.iter().position(|p| p.name == "Orbx_C_GB_South_TrueEarth_Orthos").unwrap();
        let orbx_north_idx = packs.iter().position(|p| p.name == "Orbx_C_GB_North_TrueEarth_Orthos").unwrap();
        let simheaven_4_idx = packs.iter().position(|p| p.name == "simHeaven_X-World_Europe-4-extras").unwrap();
        let simheaven_1_idx = packs.iter().position(|p| p.name == "simHeaven_X-World_Europe-1-vfr").unwrap();

        // SimHeaven (score 20) should be at lower indices than Orbx C (score 58)
        assert!(
            simheaven_4_idx < orbx_south_idx,
            "SimHeaven (idx {}) must be ABOVE (lower idx) Orbx C Orthos (idx {})",
            simheaven_4_idx, orbx_south_idx
        );
        assert!(
            simheaven_1_idx < orbx_north_idx,
            "SimHeaven (idx {}) must be ABOVE Orbx C Orthos (idx {})",
            simheaven_1_idx, orbx_north_idx
        );

        // Print order for debugging
        println!("Sorted order:");
        for (i, p) in packs.iter().enumerate() {
            println!("  {}: {} (category={:?})", i, p.name, p.category);
        }
    }

    #[test]
    fn test_orbx_c_sorted_packs_pass_validation() {
        // CRITICAL: After sorting, the validator must NOT flag mesh_above_overlay
        // This is the full integration test that proves the sorter+validator agree.
        use crate::scenery::validator::SceneryValidator;

        let mut packs = vec![
            make_pack("Orbx_C_GB_South_TrueEarth_Orthos"),
            make_pack("simHeaven_X-World_Europe-4-extras"),
            make_pack("Orbx_C_GB_North_TrueEarth_Orthos"),
            make_pack("simHeaven_X-World_Europe-1-vfr"),
        ];

        let model = x_adox_bitnet::BitNetModel::default();
        sort_packs(&mut packs, Some(&model), &x_adox_bitnet::PredictContext::default());

        // Print categories for debugging
        println!("Pack categories after sort:");
        for (i, p) in packs.iter().enumerate() {
            println!("  {}: {} -> category={:?}", i, p.name, p.category);
        }

        // Validate - should have NO mesh_above_overlay issues
        let report = SceneryValidator::validate(&packs);

        let mesh_issues: Vec<_> = report.issues.iter()
            .filter(|i| i.issue_type == "mesh_above_overlay")
            .collect();

        assert!(
            mesh_issues.is_empty(),
            "Validator should NOT flag mesh_above_overlay after correct sort, but got: {:?}",
            mesh_issues
        );
    }

    #[test]
    fn test_realistic_mixed_packs_pass_validation() {
        // Realistic scenario: Multiple pack types mixed together, including
        // airports, overlays, libraries, orthos, and mesh. After sorting,
        // the validator must NOT flag any mesh_above_overlay issues.
        use crate::scenery::validator::SceneryValidator;

        let mut packs = vec![
            make_pack("zzz_UHD_Mesh_v4"),                     // Mesh (score ~60)
            make_pack("Orbx_C_GB_South_TrueEarth_Orthos"),    // OrthoBase (score 58)
            make_pack("EGLL_Heathrow"),                       // CustomAirport (score ~10)
            make_pack("simHeaven_X-World_Europe-1-vfr"),      // RegionalOverlay (score 20)
            make_pack("OpenSceneryX_Library"),                // Library (score 40)
            make_pack("Global Airports"),                     // GlobalAirport (score 13)
            make_pack("FlyTampa_Amsterdam_1_overlays"),       // AirportOverlay (score ~12)
            make_pack("XPME_Overlays"),                       // AutoOrthoOverlay (score 48)
            make_pack("XPME_Europe"),                         // OrthoBase (score 95)
        ];

        let model = x_adox_bitnet::BitNetModel::default();
        sort_packs(&mut packs, Some(&model), &x_adox_bitnet::PredictContext::default());

        // Print sorted order with categories
        println!("Realistic mixed pack sort order:");
        for (i, p) in packs.iter().enumerate() {
            println!("  {}: {} -> category={:?}", i, p.name, p.category);
        }

        // Find first mesh/ortho and last overlay for manual verification
        let first_mesh_idx = packs.iter().position(|p|
            matches!(p.category, SceneryCategory::Mesh | SceneryCategory::OrthoBase)
        );
        let last_overlay_idx = packs.iter().rposition(|p|
            matches!(p.category,
                SceneryCategory::CustomAirport
                | SceneryCategory::OrbxAirport
                | SceneryCategory::GlobalAirport
                | SceneryCategory::Landmark
                | SceneryCategory::RegionalOverlay
                | SceneryCategory::RegionalFluff
                | SceneryCategory::AirportOverlay
                | SceneryCategory::LowImpactOverlay
                | SceneryCategory::AutoOrthoOverlay
            )
        );

        println!("First mesh/ortho idx: {:?}, Last overlay idx: {:?}", first_mesh_idx, last_overlay_idx);

        // Validate
        let report = SceneryValidator::validate(&packs);

        let mesh_issues: Vec<_> = report.issues.iter()
            .filter(|i| i.issue_type == "mesh_above_overlay")
            .collect();

        assert!(
            mesh_issues.is_empty(),
            "Validator should NOT flag mesh_above_overlay after correct sort.\n\
             First mesh at {:?}, last overlay at {:?}\n\
             Issues: {:?}",
            first_mesh_idx, last_overlay_idx, mesh_issues
        );
    }

    #[test]
    fn test_orbx_c_category_with_empty_vs_full_context() {
        // In load_quick() mode, uncached packs get empty context.
        // Verify Orbx C still gets OrthoBase category in both cases.
        use crate::scenery::classifier::Classifier;

        let model = x_adox_bitnet::BitNetModel::default();
        let path = std::path::PathBuf::from("Custom Scenery/Orbx_C_GB_South_TrueEarth_Orthos");

        // Empty context (what load_quick produces for uncached packs)
        let empty_ctx = x_adox_bitnet::PredictContext::default();
        let empty_category = Classifier::classify("Orbx_C_GB_South_TrueEarth_Orthos", &path, &empty_ctx, &model);

        // Full context (what load_with_progress produces after disk scan)
        let full_ctx = x_adox_bitnet::PredictContext {
            has_tiles: true,
            object_count: 0,
            facade_count: 0,
            ..Default::default()
        };
        let full_category = Classifier::classify("Orbx_C_GB_South_TrueEarth_Orthos", &path, &full_ctx, &model);

        println!("Orbx_C with empty context: {:?}", empty_category);
        println!("Orbx_C with full context: {:?}", full_category);

        assert_eq!(
            empty_category,
            SceneryCategory::OrthoBase,
            "Orbx_C with empty context should still be OrthoBase"
        );
        assert_eq!(
            full_category,
            SceneryCategory::OrthoBase,
            "Orbx_C with full context should be OrthoBase"
        );
    }

    #[test]
    fn test_all_overlay_scores_and_categories() {
        // Debug test: Print scores and categories for all overlay-like packs
        // to verify they all get scores < 50 (overlay territory)
        let model = x_adox_bitnet::BitNetModel::default();
        let ctx = x_adox_bitnet::PredictContext::default();
        let path = std::path::PathBuf::from("test");

        let overlay_packs = [
            "XPME_Overlays",
            "simHeaven_X-World_Europe-1-vfr",
            "simHeaven_X-World_Europe-4-extras",
            "yAutoOrtho_Overlays",
            "FlyTampa_Amsterdam_1_overlays",
            "Global_Forests_v2",
            "Shoreline_Objects",
        ];

        let ortho_packs = [
            "Orbx_C_GB_South_TrueEarth_Orthos",
            "XPME_Europe",
            "z_ao_eur",
            "zzz_UHD_Mesh_v4",
        ];

        println!("\n=== OVERLAY PACKS (should be score < 50) ===");
        for name in overlay_packs {
            let (score, cat, rule) = model.predict_with_rule_name(name, &path, &ctx);
            println!("  {}: score={}, category={:?}, rule='{}'", name, score, cat, rule);
            assert!(
                score < 50,
                "Overlay pack '{}' should have score < 50, got {}",
                name, score
            );
        }

        println!("\n=== ORTHO/MESH PACKS (should be score >= 50) ===");
        for name in ortho_packs {
            let (score, cat, rule) = model.predict_with_rule_name(name, &path, &ctx);
            println!("  {}: score={}, category={:?}, rule='{}'", name, score, cat, rule);
            assert!(
                score >= 50,
                "Ortho/mesh pack '{}' should have score >= 50, got {}",
                name, score
            );
        }
    }
}
