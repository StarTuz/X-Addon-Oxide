use std::path::PathBuf;
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

#[test]
fn test_all_regions_coverage() {
    let regions = get_all_regions();
    let mut pack = create_mock_pack("Global Airports");

    // 1. Populate World: Ensure every region has at least one airport
    for region in &regions {
        if let Some(bounds) = region.bounds.first() {
            // Pick center
            let lat = (bounds.min_lat + bounds.max_lat) / 2.0;
            let lon = (bounds.min_lon + bounds.max_lon) / 2.0;
            let id = format!("R_{}", region.id); // e.g. "R_US"

            // Add a "Hub" (Hard, Long)
            pack.airports
                .push(create_mock_airport(&id, lat, lon, 3000, SurfaceType::Hard));

            // Add a "GA Strip" (Soft, Short) slightly offset
            pack.airports.push(create_mock_airport(
                &format!("{}_GA", id),
                lat + 0.01,
                lon + 0.01,
                800,
                SurfaceType::Soft,
            ));
        }
    }

    // Add generic "World Hubs" to ensure destinations exist for long hauls
    pack.airports.push(create_mock_airport(
        "EGLL",
        51.47,
        -0.45,
        4000,
        SurfaceType::Hard,
    )); // London
    pack.airports.push(create_mock_airport(
        "KJFK",
        40.64,
        -73.78,
        4000,
        SurfaceType::Hard,
    )); // NY
    pack.airports.push(create_mock_airport(
        "RJAA",
        35.77,
        140.39,
        4000,
        SurfaceType::Hard,
    )); // Tokyo
    pack.airports.push(create_mock_airport(
        "FACT",
        -33.97,
        18.60,
        4000,
        SurfaceType::Hard,
    )); // Cape Town

    // Fix isolation issues (4 failures)
    // Instead of manual hubs, let's add a global grid to ensure connectivity everywhere
    // 30 degree steps = ~1800nm lat, varied lon. Sufficient for 3000nm range.
    for lat in (-90..=90).step_by(30) {
        for lon in (-180..180).step_by(30) {
            let id = format!("GRID_{}_{}", lat, lon);
            pack.airports.push(create_mock_airport(
                &id,
                lat as f64,
                lon as f64,
                4000,
                SurfaceType::Hard,
            ));
        }
    }

    // Add a Heliport in the Alps for constraint testing
    pack.airports.push(create_mock_airport(
        "HELI_ALPS",
        46.5,
        8.0,
        100,
        SurfaceType::Soft,
    ));

    // 2. Define Fleet
    let cessna = create_mock_aircraft("Cessna 172", vec!["General Aviation"]);
    let boeing = create_mock_aircraft("Boeing 737", vec!["Jet", "Airliner", "Heavy"]);
    let heli = create_mock_aircraft("Bell 407", vec!["Helicopter"]);

    // 3. Test Loop
    let mut failure_count = 0;

    // Re-run loops with better debug
    for region in &regions {
        // Add a "Regional Neighbor" to ensure GA connectivity strictly within region neighborhood
        if let Some(bounds) = region.bounds.first() {
            let lat = (bounds.min_lat + bounds.max_lat) / 2.0;
            let lon = (bounds.min_lon + bounds.max_lon) / 2.0;
            // Add neighbor at ~30nm (0.5 deg)
            pack.airports.push(create_mock_airport(
                &format!("N_{}", region.id),
                lat + 0.5,
                lon + 0.5,
                2000,
                SurfaceType::Hard,
            ));
        }
    }

    let packs = vec![pack]; // Re-bind with neighbors

    for region in &regions {
        // 1. Ensure Local Connectivity for GA
        {
            let prompt = format!("{} Cessna", region.name);
            if let Err(e) = generate_flight(&packs, &[cessna.clone()], &prompt, None, None) {
                println!("GA FAILED '{}' ({}): {}", prompt, region.id, e);
                failure_count += 1;
            }
        }

        // 2. Ensure Global Connectivity for Jets
        {
            let prompt = format!("{} Boeing", region.name);
            if let Err(e) = generate_flight(&packs, &[boeing.clone()], &prompt, None, None) {
                println!("JET FAILED '{}' ({}): {}", prompt, region.id, e);
                failure_count += 1;
            }
        }
    }

    // 4. Cross-Region and Constraints
    // Explicit route: UK (EGLL) to US (KJFK) - both in hub list
    // Use ICAO codes for robust "A to B" testing
    let cross_prompt = "EGLL to KJFK using Boeing";
    if let Err(e) = generate_flight(&packs, &[boeing.clone()], cross_prompt, None, None) {
        println!("CROSS-REGION FAILED: {}", e);
        failure_count += 1;
    }

    // Helicopter constraint
    let heli_prompt = "Alps Bell";
    if let Err(e) = generate_flight(&packs, &[heli.clone()], heli_prompt, None, None) {
        println!("HELI FAILED: {}", e);
        failure_count += 1;
    }

    assert_eq!(
        failure_count,
        0,
        "Failed to generate flights for {}/{} tests",
        failure_count,
        regions.len() * 2 + 2
    );
}

#[test]
fn test_glider_range_constraint() {
    let mut pack = create_mock_pack("Glider Area");
    // Origin
    pack.airports.push(create_mock_airport(
        "EGLL",
        51.47,
        -0.45,
        4000,
        SurfaceType::Hard,
    ));
    // Destination too far (>60nm so glider range excludes it)
    pack.airports.push(create_mock_airport(
        "DEST_FAR",
        53.5,
        -0.45,
        2000,
        SurfaceType::Hard,
    ));
    // Destination close (20nm)
    pack.airports.push(create_mock_airport(
        "DEST_CLOSE",
        51.67,
        -0.45,
        1000,
        SurfaceType::Hard,
    ));

    let ask21 = create_mock_aircraft("Schleicher ASK 21", vec!["Glider"]);
    let packs = vec![pack];

    // "to random" ensures the parser sees "from X to Y" so origin is EGLL (not unset)
    let plan = generate_flight(&packs, &[ask21], "from EGLL to random using glider", None, None)
        .expect("Failed to gen glider flight");
    assert!(
        plan.distance_nm <= 60,
        "Glider flight too long: {}nm",
        plan.distance_nm
    );
    assert_eq!(plan.origin.id, "EGLL");
    // Glider range 5–60nm: only DEST_CLOSE (~12nm) is in range; DEST_FAR is >60nm
    assert_eq!(plan.destination.id, "DEST_CLOSE");
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
        let plan = generate_flight(&packs, &[cessna.clone()], "from random to Italy", None, None)
            .expect("Failed to gen Italy flight");
        assert!(
            plan.destination.id.starts_with("LI"),
            "Italy flight ended in France: {}",
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
    let plan = generate_flight(&packs, &[cessna], "short flight from London UK to random", None, None)
        .expect("Failed to gen London UK flight");

    // Must be a UK airport (EG), not London Ontario (CYXU)
    assert!(
        plan.origin.id.starts_with("EG"),
        "Expected UK airport (EG*), got {} (e.g. London Ontario CYXU)",
        plan.origin.id
    );
}
