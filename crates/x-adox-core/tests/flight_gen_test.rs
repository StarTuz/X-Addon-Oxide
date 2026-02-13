#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use x_adox_core::apt_dat::{Airport, AirportType, SurfaceType};
    use x_adox_core::discovery::{AddonType, DiscoveredAddon};
    use x_adox_core::flight_gen::{export_lnmpln, generate_flight};
    use x_adox_core::scenery::{
        SceneryCategory, SceneryDescriptor, SceneryManager, SceneryPack, SceneryPackType,
    };

    fn make_test_airport(
        id: &str,
        name: &str,
        lat: f64,
        lon: f64,
        len: u32,
        surf: SurfaceType,
    ) -> Airport {
        Airport {
            id: id.to_string(),
            name: name.to_string(),
            lat: Some(lat),
            lon: Some(lon),
            airport_type: AirportType::Land,
            proj_x: None,
            proj_y: None,
            max_runway_length: Some(len),
            surface_type: Some(surf),
        }
    }

    fn make_test_aircraft(name: &str, tags: Vec<&str>) -> DiscoveredAddon {
        DiscoveredAddon {
            path: PathBuf::from(format!("/aircraft/{}", name)),
            name: name.to_string(),
            addon_type: AddonType::Aircraft {
                variants: vec![],
                livery_count: 1,
                livery_names: vec![],
            },
            is_enabled: true,
            tags: tags.into_iter().map(|s| s.to_string()).collect(),
            is_laminar_default: false,
        }
    }

    #[test]
    fn test_generate_flight_simple() {
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        // Mock packs
        manager.packs.push(SceneryPack {
            name: "UK Pack".to_string(),
            path: PathBuf::from("UK"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![
                make_test_airport(
                    "EGLL",
                    "London Heathrow",
                    51.47,
                    -0.45,
                    12000,
                    SurfaceType::Hard,
                ), // London
                make_test_airport("EGLC", "London City", 51.50, 0.05, 4000, SurfaceType::Hard), // London City
                make_test_airport("EGKB", "Biggin Hill", 51.32, 0.03, 2500, SurfaceType::Soft), // Biggin Hill
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("Europe UK".to_string()),
        });
        manager.packs.push(SceneryPack {
            name: "France Pack".to_string(),
            path: PathBuf::from("France"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![
                make_test_airport(
                    "LFPG",
                    "Paris Charles de Gaulle",
                    49.00,
                    2.55,
                    12000,
                    SurfaceType::Hard,
                ), // Paris
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("Europe France".to_string()),
        });
        manager.packs.push(SceneryPack {
            name: "US Pack".to_string(),
            path: PathBuf::from("US"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![
                make_test_airport(
                    "KJFK",
                    "Kennedy Intl",
                    40.64,
                    -73.78,
                    12000,
                    SurfaceType::Hard,
                ), // New York
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("North America".to_string()),
        });

        let aircraft = vec![
            make_test_aircraft("Boeing 747", vec!["Heavy", "Jet"]),
            make_test_aircraft("Cessna 172", vec!["GA", "Prop"]),
        ];

        // 1. Heavy Jet Flight (Should pick KJFK)
        let plan = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from EGLL to KJFK using Boeing 747",
        )
        .unwrap();
        assert_eq!(plan.origin.id, "EGLL");
        assert_eq!(plan.destination.id, "KJFK");
        assert_eq!(plan.aircraft.name, "Boeing 747");

        // 2. Short Hop with Cessna (EGLL -> LFPG is ~180nm)
        let plan_short = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from EGLL to LFPG using Cessna",
        )
        .unwrap();
        assert_eq!(plan_short.destination.id, "LFPG");

        // 3. Guardrail Check: 747 into small airport?
        let res = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from EGLL to EGKB using Boeing 747",
        );
        assert!(res.is_err()); // Guardrail stopped it

        // 4. Ignore Guardrails (Should pass)
        let plan_wild = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from EGLL to EGKB using Boeing 747 ignore guardrails",
        )
        .unwrap();
        assert_eq!(plan_wild.destination.id, "EGKB");

        // 4. Region Filter
        // Prompt: "Flight from UK to Paris using Cessna"
        let plan_eu = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from UK to LFPG using Cessna",
        )
        .unwrap();
        assert!(
            plan_eu.origin.id == "EGLL"
                || plan_eu.origin.id == "EGLC"
                || plan_eu.origin.id == "EGKB"
        );
        assert_eq!(plan_eu.destination.id, "LFPG");
    }

    #[test]
    fn test_export_xml() {
        let plan = x_adox_core::flight_gen::FlightPlan {
            origin: make_test_airport(
                "EGLL",
                "London Heathrow",
                51.0,
                0.0,
                10000,
                SurfaceType::Hard,
            ),
            destination: make_test_airport(
                "LFPG",
                "Paris CDG",
                49.0,
                2.0,
                10000,
                SurfaceType::Hard,
            ),
            aircraft: make_test_aircraft("B737", vec!["Jet"]),
            distance_nm: 200,
            duration_minutes: 45,
            route_description: "generated".to_string(),
        };

        let xml = export_lnmpln(&plan);
        assert!(xml.contains("<Ident>EGLL</Ident>"));
        assert!(xml.contains("<Ident>LFPG</Ident>"));
    }
    #[test]
    fn test_generate_flight_name_match() {
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        manager.packs.push(SceneryPack {
            name: "Global Airports".to_string(),
            path: PathBuf::from("Global Airports"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![
                Airport {
                    id: "KLAX".to_string(),
                    name: "Los Angeles Intl".to_string(),
                    airport_type: AirportType::Land,
                    lat: Some(33.94),
                    lon: Some(-118.40),
                    proj_x: None,
                    proj_y: None,
                    max_runway_length: Some(4000),
                    surface_type: Some(SurfaceType::Hard),
                },
                Airport {
                    id: "KSFO".to_string(),
                    name: "San Francisco Intl".to_string(),
                    airport_type: AirportType::Land,
                    lat: Some(37.62),
                    lon: Some(-122.37),
                    proj_x: None,
                    proj_y: None,
                    max_runway_length: Some(3500),
                    surface_type: Some(SurfaceType::Hard),
                },
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("Global".to_string()),
        });

        let aircraft = vec![make_test_aircraft("Boeing 747", vec!["Heavy", "Jet"])];

        // Prompt using names
        let plan = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from Los Angeles to San Francisco using Boeing 747",
        )
        .unwrap();

        assert_eq!(plan.origin.id, "KLAX");
        assert_eq!(plan.destination.id, "KSFO");
    }

    #[test]
    fn test_generate_flight_ranking_and_multiword() {
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        manager.packs.push(SceneryPack {
            name: "Great Britain".to_string(),
            path: PathBuf::from("UK"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "EGLL",
                "London Heathrow",
                51.47,
                -0.45,
                12000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("United Kingdom".to_string()),
        });

        manager.packs.push(SceneryPack {
            name: "US Pack".to_string(),
            path: PathBuf::from("US"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "KGON",
                "Groton New London",
                41.33,
                -72.05,
                5000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("USA".to_string()),
        });

        let aircraft = vec![make_test_aircraft("Boeing 747", vec!["Heavy", "Jet"])];

        // 1. Ranking Check: "London"
        // "London Heathrow" starts with "London" (Score 80)
        // "Groton New London" contains "London" (Score 60)
        // Should pick EGLL
        for _ in 0..5 {
            let plan = generate_flight(
                &manager.packs,
                &aircraft,
                "Flight from London to KGON using Boeing 747",
            )
            .unwrap();
            assert_eq!(
                plan.origin.id, "EGLL",
                "Should prioritize London Heathrow for input 'London'"
            );
        }

        // 2. Multi-word Token Check: "London United Kingdom"
        // "London" in Heathrow name. "United Kingdom" in Region.
        // Should match EGLL.
        let plan_complex = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from London United Kingdom to KGON using Boeing 747",
        )
        .unwrap();
        assert_eq!(plan_complex.origin.id, "EGLL");

        // 3. Abbreviation Check: "London UK"
        // "UK" is NOT in "United Kingdom" by substring. Should fail currently.
        let plan_abbr = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from London UK to KGON using Boeing 747",
        );
        assert!(plan_abbr.is_ok(), "Should match 'UK' to 'United Kingdom'");
    }

    #[test]
    fn test_uk_gb_distinction() {
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        // 1. Northern Ireland Pack
        manager.packs.push(SceneryPack {
            name: "NI Scenery".to_string(),
            path: PathBuf::from("NI"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "EGAA",
                "Belfast Intl",
                54.65,
                -6.21,
                9000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("Northern Ireland".to_string()),
        });

        // 2. Dummy Destination
        manager.packs.push(SceneryPack {
            name: "US Pack".to_string(),
            path: PathBuf::from("US"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "KJFK",
                "Kennedy Intl",
                40.64,
                -73.78,
                12000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("USA".to_string()),
        });

        let aircraft = vec![make_test_aircraft("Boeing 747", vec!["Heavy", "Jet"])];

        // Case A: "Flight from UK to KJFK..." -> Should find EGAA (Northern Ireland is in UK)
        let plan_uk = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from UK to KJFK using Boeing 747",
        );
        assert!(plan_uk.is_ok(), "UK should include Northern Ireland");
        assert_eq!(plan_uk.unwrap().origin.id, "EGAA");

        // Case B: "Flight from GB to KJFK..." -> Should NOT find EGAA (Northern Ireland is NOT in GB)
        let plan_gb = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from GB to KJFK using Boeing 747",
        );
        assert!(plan_gb.is_err(), "GB should exclude Northern Ireland");
    }

    #[test]
    fn test_british_isles_grouping() {
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        // 1. Ireland (Republic)
        manager.packs.push(SceneryPack {
            name: "Ireland Scenery".to_string(),
            path: PathBuf::from("IE"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "EIDW",
                "Dublin Intl",
                53.42,
                -6.27,
                9000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("Ireland".to_string()),
        });

        // 2. UK (Great Britain)
        manager.packs.push(SceneryPack {
            name: "UK Scenery".to_string(),
            path: PathBuf::from("UK"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "EGLL",
                "London Heathrow",
                51.47,
                -0.45,
                12000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("United Kingdom".to_string()),
        });

        // 3. Isle of Man
        manager.packs.push(SceneryPack {
            name: "IOM Scenery".to_string(),
            path: PathBuf::from("IOM"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "EGNS",
                "Isle of Man",
                54.08,
                -4.63,
                6000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("Isle of Man".to_string()),
        });

        // 4. Dummy Destination
        manager.packs.push(SceneryPack {
            name: "France Pack".to_string(),
            path: PathBuf::from("France"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "LFPG",
                "Paris CDG",
                49.00,
                2.55,
                12000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("France".to_string()),
        });

        let aircraft = vec![make_test_aircraft("Boeing 747", vec!["Heavy", "Jet"])];

        // "Flight from British Isles to France"
        // Should find EITHER EIDW (Ireland) or EGLL (UK). Both are in British Isles.
        // We run multiple times to ensure no error is thrown and it picks valid origins.
        for _ in 0..5 {
            let plan = generate_flight(
                &manager.packs,
                &aircraft,
                "Flight from British Isles to France using Boeing 747 ignore guardrails",
            );
            assert!(plan.is_ok(), "British Isles should match Ireland or UK");
            let origin_id = plan.unwrap().origin.id;
            assert!(
                origin_id == "EIDW" || origin_id == "EGLL" || origin_id == "EGNS",
                "Origin {} not in British Isles",
                origin_id
            );
        }
    }
}
