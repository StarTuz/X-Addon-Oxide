use std::path::PathBuf;
use x_adox_bitnet::flight_prompt::{FlightPrompt, LocationConstraint};
use x_adox_bitnet::geo::data::get_all_regions;
use x_adox_core::apt_dat::{Airport, AirportType, SurfaceType};
use x_adox_core::discovery::{AddonType, DiscoveredAddon};
use x_adox_core::flight_gen::generate_flight;
use x_adox_core::scenery::{SceneryCategory, SceneryDescriptor, SceneryPack, SceneryPackType};

// --- Mock Helpers ---

fn create_mock_airport(id: &str, lat: f64, lon: f64, len: u32, surf: SurfaceType) -> Airport {
    Airport {
        id: id.to_string(),
        name: format!("Airport {}", id),
        airport_type: AirportType::Land,
        lat: Some(lat),
        lon: Some(lon),
        proj_x: None,
        proj_y: None,
        max_runway_length: Some(len),
        surface_type: Some(surf),
    }
}

fn create_mock_pack(name: &str) -> SceneryPack {
    SceneryPack {
        name: name.to_string(),
        path: PathBuf::from(format!("Custom Scenery/{}", name)),
        raw_path: None,
        status: SceneryPackType::Active,
        category: SceneryCategory::GlobalAirport,
        airports: Vec::new(),
        tiles: Vec::new(),
        tags: Vec::new(),
        descriptor: SceneryDescriptor::default(),
        region: None,
    }
}

fn create_mock_aircraft(name: &str, tags: Vec<&str>) -> DiscoveredAddon {
    use x_adox_core::discovery::AcfVariant;
    DiscoveredAddon {
        path: PathBuf::from(format!("Aircraft/{}", name)),
        name: name.to_string(),
        addon_type: AddonType::Aircraft {
            variants: vec![AcfVariant {
                name: format!("{} Standard", name),
                file_name: format!("{}.acf", name),
                is_enabled: true,
            }],
            livery_count: 1,
            livery_names: vec!["Default".to_string()],
        },
        is_enabled: true,
        tags: tags.into_iter().map(|s| s.to_string()).collect(),
        is_laminar_default: false,
    }
}

// --- Test Suite ---

/// Verifies that every region's English name in regions.json resolves to the correct
/// `LocationConstraint::Region(id)` via FlightPrompt NLP parsing.
///
/// This catches:
/// - Missing regions (Ukraine bug): "Ukraine" had no entry → parsed as None
/// - Misparsed sub-regions (England bug): "England" → Region("UK") instead of Region("UK:England")
/// - Country names that fall through to ICAO heuristic ("iran" is 4 chars)
/// - Alternative names not in alias table ("burma" → ICAO, not Region("MM"))
///
/// The test uses "flight to {name}" which exercises the full NLP → try_as_region → RegionIndex
/// pipeline end-to-end.
#[test]
fn test_region_nlp_parsing() {
    // Regions whose names intentionally resolve to NearCity because a same-named city
    // takes priority in the explicit alias table (by design — city is more precise).
    let near_city_overrides: std::collections::HashSet<&str> = [
        "US:NY",     // "New York" → NearCity (city preferred over state for routing)
        "UK:London", // "London" → NearCity (city more precise than UK:London sub-region)
    ]
    .iter()
    .copied()
    .collect();

    let regions = get_all_regions();
    let mut failures = Vec::new();

    for region in regions.iter() {
        if near_city_overrides.contains(region.id.as_str()) {
            continue;
        }

        let prompt = format!("flight to {}", region.name);
        let parsed = FlightPrompt::parse(&prompt, &x_adox_bitnet::NLPRulesConfig::default());
        let expected = LocationConstraint::Region(region.id.clone());

        if parsed.destination.as_ref() != Some(&expected) {
            failures.push(format!(
                "  {:30} (id={:15}) => got {:?}",
                region.name, region.id, parsed.destination
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} region name(s) failed NLP parsing:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Verifies that flight generation succeeds for every non-trivial region and that
/// the selected destination airport is geographically within the region's bounds.
///
/// This catches:
/// - Wrong ICAO prefix mappings: seeds have correct real-world ICAO prefixes (e.g. UKBB
///   for Ukraine). If `icao_prefixes_for_region("UA")` returns a wrong prefix, seeds are
///   filtered out too → generation fails → test fails.
/// - Bounds drift (England/Scotland bug): if UK:England's bounds accidentally include
///   Edinburgh (55.95°N), the bounds assertion detects it.
/// - Missing seed airports: countries with no seeds and no pack airports → generation fails.
///
/// Mock setup: world hub airports (with real ICAO prefixes) + a dense grid for geographic
/// features that have no ICAO prefix filter. Seed airports (with real-world ICAO IDs)
/// handle country-level filtering automatically.
#[test]
fn test_all_regions_flight_generation() {
    let mut pack = create_mock_pack("World Hubs");

    // World hubs — real ICAO IDs so they pass any prefix filter when used as origins/dests.
    for (id, lat, lon) in [
        ("EGLL", 51.47, -0.45),
        ("KJFK", 40.64, -73.78),
        ("RJAA", 35.77, 140.39),
        ("FACT", -33.97, 18.60),
        ("YSSY", -33.94, 151.18),
        ("SBGR", -23.43, -46.47),
        ("OMDB", 25.25, 55.37),
        ("ZBAA", 40.08, 116.59),
    ] {
        pack.airports
            .push(create_mock_airport(id, lat, lon, 4000, SurfaceType::Hard));
    }

    // Dense grid for geographic features (Alps, Mediterranean, Himalayas, etc.) which have
    // no ICAO prefix filter — bounds-only → these grid airports are visible within any region.
    for lat in (-80..=80i32).step_by(15) {
        for lon in (-175..180i32).step_by(15) {
            pack.airports.push(create_mock_airport(
                &format!("GRID_{}_{}", lat, lon),
                lat as f64,
                lon as f64,
                4000,
                SurfaceType::Hard,
            ));
        }
    }

    let packs = vec![pack];
    let boeing = create_mock_aircraft("Boeing 737", vec!["Jet", "Airliner", "Heavy"]);
    let regions = get_all_regions();

    // US state sub-regions use parent ICAO prefix "K", but our mock airports don't carry
    // K-prefix and these sub-regions have no seeds. They're skipped here because:
    // (a) the parent US region is separately tested, and (b) coverage within a US state
    // depends on installed scenery, not seed airports.
    let skip: std::collections::HashSet<&str> = [
        "US:SoCal",
        "US:NorCal",
        "US:OR",
        "US:WA",
        "US:TX",
        "US:FL",
        "US:CA",
        "US:AK:SE",
        "US:NY",
    ]
    .iter()
    .copied()
    .collect();

    let mut oob_failures = Vec::new(); // destination selected but outside region bounds — always a bug
    let mut gen_failures = Vec::new(); // generation failed — expected for geographic features

    for region in regions.iter() {
        if skip.contains(region.id.as_str()) {
            continue;
        }

        // Use a proper directional prompt so NLP resolves the region name.
        // "{name} Boeing" (old approach) doesn't parse — no "to"/"from" keyword.
        let prompt = format!("flight to {}", region.name);

        match generate_flight(&packs, &[boeing.clone()], &prompt, None, None, None) {
            Ok(plan) => {
                if let (Some(lat), Some(lon)) = (plan.destination.lat, plan.destination.lon) {
                    if !region.contains(lat, lon) {
                        oob_failures.push(format!(
                            "  {:25} ({:12}): destination {} at ({:.2}°N, {:.2}°E) is OUTSIDE region bounds",
                            region.name, region.id, plan.destination.id, lat, lon
                        ));
                    }
                }
            }
            Err(e) => {
                gen_failures.push(format!("  {:25} ({:12}): {}", region.name, region.id, e));
            }
        }
    }

    // Print generation failures as informational — geographic features (Alps, Caribbean, etc.)
    // legitimately have no airports in this mock setup. These are not bugs.
    if !gen_failures.is_empty() {
        println!(
            "\nNote: {} region(s) could not generate a flight in this mock setup \
             (geographic features / no seeds — not a bug):",
            gen_failures.len()
        );
        for f in &gen_failures {
            println!("{}", f);
        }
    }

    // Bounds violations ARE bugs: if we generated a flight for a region, the destination
    // must be geographically within that region's claimed bounds.
    assert!(
        oob_failures.is_empty(),
        "{} region(s) produced destinations OUTSIDE their bounds:\n{}",
        oob_failures.len(),
        oob_failures.join("\n")
    );

    // Countries with ICAO prefix mappings must generate successfully (seeds + prefix ensure this).
    // If a seeded country fails, its ICAO prefix mapping is broken or seeds are missing.
    let seeded_countries: &[&str] = &[
        // Europe
        "UK",
        "UK:England",
        "UK:Scotland",
        "UK:Wales",
        "IE",
        "FR",
        "DE",
        "IT",
        "ES",
        "PT",
        "NL",
        "BE",
        "CH",
        "AT",
        "GR",
        "NO",
        "SE",
        "FI",
        "DK",
        "IS",
        "PL",
        "CZ",
        "TR",
        "UA",
        "BG",
        "EE",
        "HR",
        "HU",
        "LT",
        "LV",
        "RO",
        "RS",
        "AL",
        "BA",
        "BY",
        "MD",
        "ME",
        "MK",
        "SI",
        "SK",
        // Americas
        "US",
        "US:AK",
        "US:HI",
        "CA",
        "MX",
        "BR",
        "AR",
        "CO",
        "PE",
        "CL",
        "BO",
        "BS",
        "CR",
        "CU",
        "DO",
        "EC",
        "GT",
        "HN",
        "HT",
        "JM",
        "NI",
        "PA",
        "PY",
        "SV",
        "UY",
        "VE",
        // Asia-Pacific
        "JP",
        "CN",
        "KR",
        "IN",
        "TH",
        "VN",
        "ID",
        "AU",
        "SG",
        "MY",
        "PH",
        "HK",
        "TW",
        "NZ",
        "BD",
        "KH",
        "LA",
        "LK",
        "MM",
        "MN",
        "NP",
        "PG",
        "PK",
        "FJ",
        // Middle East
        "IL",
        "SAU",
        "UAE",
        "QA",
        "BH",
        "IQ",
        "IR",
        "JO",
        "KW",
        "LB",
        "OM",
        // Africa
        "EG",
        "ZA",
        "KE",
        "TZ",
        "ET",
        "NG",
        "MA",
        "AO",
        "CM",
        "GH",
        "LY",
        "MG",
        "MZ",
        "RU",
        "RW",
        "SD",
        "SN",
        "TN",
        "UG",
        "ZM",
        "ZW",
        // Geographic features (seeded with real airports within their bounds)
        "Alps",
        "Pyrenees",
        "Himalayas",
        "Atlas",
        "Mediterranean",
        "Andes",
        "Rockies",
        "Amazon",
        "Patagonia",
        "Caribbean",
        // Pacific Islands and sub-regions
        "PacIsles",
        "PacIsles:Micronesia",
        "PacIsles:Melanesia",
        "PacIsles:Polynesia",
    ];

    let gen_failure_ids: std::collections::HashSet<String> = gen_failures
        .iter()
        .filter_map(|f| {
            // Extract ID from "  Name (ID): ..." format
            f.split('(')
                .nth(1)
                .and_then(|s| s.split(')').next())
                .map(|s| s.trim().to_string())
        })
        .collect();

    let seeded_failures: Vec<&str> = seeded_countries
        .iter()
        .copied()
        .filter(|id| gen_failure_ids.contains(*id))
        .collect();

    assert!(
        seeded_failures.is_empty(),
        "{} seeded country/region(s) failed generation (missing seeds or broken ICAO prefix):\n  {}",
        seeded_failures.len(),
        seeded_failures.join(", ")
    );
}

#[test]
fn test_glider_short_keyword_constrains_range() {
    // Aircraft type no longer sets distance limits — keywords do.
    // A glider with no keyword gets wide-open distance; with "short" it stays ≤200nm.
    let mut pack = create_mock_pack("Glider Area");
    pack.airports.push(create_mock_airport(
        "EGLL",
        51.47,
        -0.45,
        4000,
        SurfaceType::Hard,
    ));
    pack.airports.push(create_mock_airport(
        "DEST_FAR",
        53.5,
        -0.45,
        2000,
        SurfaceType::Hard,
    ));
    pack.airports.push(create_mock_airport(
        "DEST_CLOSE",
        51.67,
        -0.45,
        1000,
        SurfaceType::Hard,
    ));

    let ask21 = create_mock_aircraft("Schleicher ASK 21", vec!["Glider"]);
    let packs = vec![pack];

    // "short flight" keyword caps distance at 200nm
    let plan = generate_flight(
        &packs,
        &[ask21],
        "short flight from EGLL using glider",
        None,
        None,
        None,
    )
    .expect("Should generate a short glider flight");
    assert!(
        plan.distance_nm <= 200,
        "Short keyword should cap at 200nm, got {}nm",
        plan.distance_nm
    );
}

#[test]
fn test_italy_accuracy() {
    let mut pack = create_mock_pack("Border Area");
    // Italy Airport
    pack.airports.push(create_mock_airport(
        "LIRF",
        41.8,
        12.2,
        3000,
        SurfaceType::Hard,
    ));
    // France Airport (inside Italy's broad bounding box but starting with LF)
    pack.airports.push(create_mock_airport(
        "LFKD",
        45.3,
        6.8,
        800,
        SurfaceType::Hard,
    ));

    let cessna = create_mock_aircraft("Cessna 172", vec!["GA"]);
    let packs = vec![pack];

    // Request flight to Italy
    for _ in 0..10 {
        let plan = generate_flight(
            &packs,
            &[cessna.clone()],
            "from random to Italy",
            None,
            None,
            None,
        )
        .expect("Failed to gen Italy flight");
        assert!(
            plan.destination.id.starts_with("LI"),
            "Italy flight ended in France: {}",
            plan.destination.id
        );
    }
}

/// "Rome Italy" must resolve to Italy (LI), not Rome GA (KRMG). See flight_prompt "City Country" aliases.
#[test]
fn test_rome_italy_resolves_to_italy_not_usa() {
    let mut pack = create_mock_pack("Europe");
    pack.airports.push(create_mock_airport(
        "EGMC",
        51.57,
        0.70,
        1800,
        SurfaceType::Hard,
    ));
    pack.airports.push(create_mock_airport(
        "LIRF",
        41.80,
        12.24,
        3900,
        SurfaceType::Hard,
    ));
    pack.airports.push(create_mock_airport(
        "KRMG",
        34.35,
        -85.16,
        6000,
        SurfaceType::Hard,
    ));

    // Use jet so EGMC->LIRF (~753 nm) is within range (50–3000 nm); GA would cap at 500 nm
    let jet = create_mock_aircraft("B737", vec!["Jet"]);
    let packs = vec![pack];

    for _ in 0..5 {
        let plan = generate_flight(
            &packs,
            &[jet.clone()],
            "Flight from EGMC to Rome Italy",
            None,
            None,
            None,
        )
        .expect("Rome Italy should produce a plan");
        assert!(
            plan.destination.id.starts_with("LI"),
            "Rome Italy must resolve to Italy (LI), not Rome GA (KRMG): got {}",
            plan.destination.id
        );
    }
}

#[test]
fn test_search_accuracy_london_uk() {
    let mut pack = create_mock_pack("London Area");
    // London UK
    let mut egll = create_mock_airport("EGLL", 51.47, -0.45, 4000, SurfaceType::Hard);
    egll.name = "London Heathrow".to_string();
    pack.airports.push(egll);

    // London Ontario (Canada)
    let mut cyxu = create_mock_airport("CYXU", 43.03, -81.15, 2000, SurfaceType::Hard);
    cyxu.name = "London Ontario".to_string();
    pack.airports.push(cyxu);

    // Close destination for London UK
    pack.airports.push(create_mock_airport(
        "EGLC",
        51.5,
        0.05,
        1500,
        SurfaceType::Hard,
    ));

    let cessna = create_mock_aircraft("Cessna 172", vec!["GA"]);
    let packs = vec![pack];

    // Request flight from "London UK" (parsed as Region UK)
    let plan = generate_flight(
        &packs,
        &[cessna],
        "short flight from London UK to random",
        None,
        None,
        None,
    )
    .expect("Failed to gen London UK flight");

    // Must be a UK airport (EG), not London Ontario (CYXU)
    assert!(
        plan.origin.id.starts_with("EG"),
        "Expected UK airport (EG*), got {} (e.g. London Ontario CYXU)",
        plan.origin.id
    );
}

#[test]
fn test_search_accuracy_london_england() {
    let mut pack = create_mock_pack("UK Area");

    // "Wrong" airport: Deenethorpe (has no "London" in name, but is in England/UK)
    let mut eg30 = create_mock_airport("EG30", 52.51, -0.61, 1000, SurfaceType::Hard);
    eg30.name = "Deenethorpe".to_string();
    pack.airports.push(eg30);

    // "Right" airport: London City
    let mut eglc = create_mock_airport("EGLC", 51.5, 0.05, 1500, SurfaceType::Hard);
    eglc.name = "London City".to_string();
    pack.airports.push(eglc);

    // Some destination
    let mut eddh = create_mock_airport("EDDH", 53.63, 9.98, 3000, SurfaceType::Hard);
    eddh.name = "Hamburg".to_string();
    pack.airports.push(eddh);

    let boeing = create_mock_aircraft("Boeing 737", vec!["Jet"]);
    let packs = vec![pack];

    // Request flight from "London England"
    // Token matching should prize "London" + "England" prefix boost over just "England" prefix boost
    let plan = generate_flight(
        &packs,
        &[boeing],
        "London England to Hamburg",
        None,
        None,
        None,
    )
    .expect("Failed to gen London England flight");

    assert_eq!(
        plan.origin.id, "EGLC",
        "Expected London City (EGLC) for 'London England', got {} ({})",
        plan.origin.id, plan.origin.name
    );
}

/// "England to Ukraine" must resolve both endpoints and produce a cross-regional flight.
/// Before the fix: Ukraine was missing from regions.json → parse failed; England mapped to
/// all of UK so destinations could be in Scotland.
#[test]
fn test_england_to_ukraine() {
    let pack = create_mock_pack("Empty");
    let jet = create_mock_aircraft("B737", vec!["Jet"]);
    let packs = vec![pack];

    // Should resolve via seed airports: UK:England origin (EG*) → Ukraine dest (UK* ICAO prefix)
    let plan = generate_flight(&packs, &[jet], "England to Ukraine", None, None, None)
        .expect("England to Ukraine should produce a valid flight plan");

    // Origin must be an English airport (EG prefix, within England bounds ~49.9-55.8°N)
    assert!(
        plan.origin.id.starts_with("EG"),
        "Origin should be an English airport (EG*), got {}",
        plan.origin.id
    );
    let orig_lat = plan.origin.lat.unwrap_or(0.0);
    assert!(
        orig_lat < 55.9,
        "Origin should be in England (lat < 55.9°N, not Scotland), got lat={:.2}",
        orig_lat
    );

    // Destination must be a Ukrainian airport (UK prefix: UKBB, UKLL, etc.)
    assert!(
        plan.destination.id.starts_with("UK"),
        "Destination should be a Ukrainian airport (UK*), got {}",
        plan.destination.id
    );
}

/// "California to England" must stay in England — not drift to Scotland.
/// Before the fix: "England" mapped to Region("UK") which included Edinburgh (EGPH, 55.95°N).
#[test]
fn test_california_to_england_stays_in_england() {
    let mut pack = create_mock_pack("West Coast");
    pack.airports.push(create_mock_airport(
        "KLAX",
        33.94,
        -118.41,
        12000,
        SurfaceType::Hard,
    ));
    pack.airports.push(create_mock_airport(
        "KSFO",
        37.62,
        -122.38,
        11000,
        SurfaceType::Hard,
    ));
    let jet = create_mock_aircraft("B737", vec!["Jet"]);
    let packs = vec![pack];

    for _ in 0..5 {
        let plan = generate_flight(
            &packs,
            &[jet.clone()],
            "California to England",
            None,
            None,
            None,
        )
        .expect("California to England should produce a plan");

        let dest_lat = plan.destination.lat.unwrap_or(0.0);
        assert!(
            dest_lat < 55.9,
            "Destination should be in England (lat < 55.9°N), not Scotland. Got {} at lat={:.2}",
            plan.destination.id,
            dest_lat
        );
        assert!(
            plan.destination.id.starts_with("EG"),
            "Destination should be a UK airport (EG*), got {}",
            plan.destination.id
        );
    }
}

/// "Nairobi to Lamu" must produce a valid Kenya flight plan using seed airports.
/// Before the fix, "Lamu" was misidentified as ICAO "LAMU" and "Nairobi" fell
/// through to AirportName with no match.
#[test]
fn test_nairobi_to_lamu() {
    // Empty pack: flight gen should fall back to seed airports for Kenya
    let pack = create_mock_pack("Empty");
    let cessna = create_mock_aircraft("Cessna 208", vec!["General Aviation"]);
    let packs = vec![pack];

    let plan = generate_flight(&packs, &[cessna], "Nairobi to Lamu", None, None, None)
        .expect("Nairobi to Lamu should produce a valid flight plan");

    // Both should be Kenyan airports (HK prefix)
    assert!(
        plan.origin.id.starts_with("HK"),
        "Origin should be a Kenyan airport (HK*), got {}",
        plan.origin.id
    );
    assert!(
        plan.destination.id.starts_with("HK"),
        "Destination should be a Kenyan airport (HK*), got {}",
        plan.destination.id
    );
}
