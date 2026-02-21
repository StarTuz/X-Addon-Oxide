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
            None,
            None,
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
            None,
            None,
        )
        .unwrap();
        assert_eq!(plan_short.destination.id, "LFPG");

        // 3. 747 into small soft airport — no runway/surface guardrails anymore,
        //    users control this via keywords and can swap aircraft after export.
        let plan_small = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from EGLL to EGKB using Boeing 747",
            None,
            None,
        )
        .unwrap();
        assert_eq!(plan_small.destination.id, "EGKB");

        // 4. Ignore Guardrails still works (bypasses even type checks)
        let plan_wild = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from EGLL to EGKB using Boeing 747 ignore guardrails",
            None,
            None,
        )
        .unwrap();
        assert_eq!(plan_wild.destination.id, "EGKB");

        // 4. Region Filter
        // Prompt: "Flight from UK to Paris using Cessna"
        let plan_eu = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from UK to LFPG using Cessna",
            None,
            None,
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
    fn test_heavy_aircraft_avoids_helipads() {
        // Tagless aircraft defaults to 500m/Soft, which allows HELIPADS if not filtered by type!
        let aircraft = vec![make_test_aircraft("Caravelle", vec![])]; // No tags

        let packs = vec![SceneryPack {
            path: PathBuf::from("Custom Scenery/TestPack"),
            name: "TestPack".to_string(),
            airports: vec![
                // Invalid Heliport - Should be ignored for Jet
                Airport {
                    id: "HOSP".to_string(),
                    name: "General Hospital".to_string(),
                    airport_type: AirportType::Heliport,
                    lat: Some(51.5),
                    lon: Some(-0.12),
                    proj_x: None,
                    proj_y: None,
                    max_runway_length: None,               // Tiny
                    surface_type: Some(SurfaceType::Hard), // Concrete pad
                },
            ],
            region: None,
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
        }];

        // Manager wrapper not needed for direct call, just pass packs

        // 1. Explicitly ask for any airport with Caravelle
        // Should NOT pick HOSP
        for _ in 0..10 {
            let plan = generate_flight(&packs, &aircraft, "Flight using Caravelle", None, None);
            assert!(
                plan.is_err(),
                "Caravelle (tagless) should NOT find HOSP heliport (Fixed)"
            );
        }
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
            origin_region_id: None,
            dest_region_id: None,
            duration_minutes: 45,
            route_description: "generated".to_string(),
            context: None,
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
            None,
            None,
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
                None,
                None,
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
            None,
            None,
        )
        .unwrap();
        assert_eq!(plan_complex.origin.id, "EGLL");

        // 3. Abbreviation Check: "London UK"
        // "UK" is NOT in "United Kingdom" by substring. Should fail currently.
        let plan_abbr = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from London UK to KGON using Boeing 747",
            None,
            None,
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
            None,
            None,
        );
        assert!(plan_uk.is_ok(), "UK should include Northern Ireland");
        assert_eq!(plan_uk.unwrap().origin.id, "EGAA");

        // Case B: "Flight from GB to KJFK..." -> Should NOT find EGAA (Northern Ireland is NOT in GB)
        let plan_gb = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from GB to KJFK using Boeing 747",
            None,
            None,
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
                None,
                None,
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

    #[test]
    fn test_region_to_region_flight() {
        // France → Germany using bounding-box region matching
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        manager.packs.push(SceneryPack {
            name: "Europe Scenery".to_string(),
            path: PathBuf::from("EU"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![
                // Paris CDG — inside France bounding box (41-51.5N, -5.5-10E)
                make_test_airport(
                    "LFPG",
                    "Paris Charles de Gaulle",
                    49.00,
                    2.55,
                    12000,
                    SurfaceType::Hard,
                ),
                // Munich — inside Germany bounding box (47-55.5N, 5.5-15.5E)
                // and clearly outside France bounding box
                make_test_airport(
                    "EDDM",
                    "Munich Intl",
                    48.35,
                    11.78,
                    12000,
                    SurfaceType::Hard,
                ),
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("Europe".to_string()),
        });

        let aircraft = vec![
            make_test_aircraft("Boeing 747", vec!["Heavy", "Jet"]),
            make_test_aircraft("Cessna 172", vec!["GA", "Prop"]),
        ];

        // "France" and "Germany" should now parse as Region constraints
        // and match airports by coordinate bounding boxes
        for _ in 0..5 {
            let plan = generate_flight(
                &manager.packs,
                &aircraft,
                "Flight from France to Germany using Cessna 172",
                None,
                None,
            );
            assert!(
                plan.is_ok(),
                "France → Germany should find airports: {:?}",
                plan.err()
            );
            let p = plan.unwrap();
            assert_eq!(p.origin.id, "LFPG", "Origin should be in France");
            assert_eq!(p.destination.id, "EDDM", "Destination should be in Germany");
        }
    }

    #[test]
    fn test_us_region_flight() {
        // SoCal → NorCal using bounding-box matching
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        manager.packs.push(SceneryPack {
            name: "US Global Airports".to_string(),
            path: PathBuf::from("US"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![
                // KLAX — inside SoCal box (32-35.5N, -121--114.5W)
                make_test_airport(
                    "KLAX",
                    "Los Angeles Intl",
                    33.94,
                    -118.40,
                    4000,
                    SurfaceType::Hard,
                ),
                // KSFO — inside NorCal box (35.5-42.5N, -125--119.5W)
                make_test_airport(
                    "KSFO",
                    "San Francisco Intl",
                    37.62,
                    -122.37,
                    3500,
                    SurfaceType::Hard,
                ),
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("North America".to_string()),
        });

        let aircraft = vec![make_test_aircraft("Cessna 172", vec!["GA", "Prop"])];

        for _ in 0..5 {
            let plan = generate_flight(
                &manager.packs,
                &aircraft,
                "Flight from Socal to Norcal",
                None,
                None,
            );
            assert!(plan.is_ok(), "SoCal → NorCal should work: {:?}", plan.err());
            let p = plan.unwrap();
            assert_eq!(p.origin.id, "KLAX");
            assert_eq!(p.destination.id, "KSFO");
        }
    }

    #[test]
    fn test_explicit_endpoints_bypass_distance_filter() {
        // Two airports ~300nm apart with a GA aircraft (default 30-500nm).
        // This should work even though previously it would fail for very long distances.
        // We'll test with airports ~600nm apart which would exceed GA default of 500nm.
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        manager.packs.push(SceneryPack {
            name: "US Airports".to_string(),
            path: PathBuf::from("US"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![
                // KLAX (Los Angeles) and KSEA (Seattle) are ~960nm apart
                make_test_airport(
                    "KLAX",
                    "Los Angeles Intl",
                    33.94,
                    -118.40,
                    4000,
                    SurfaceType::Hard,
                ),
                make_test_airport(
                    "KSEA",
                    "Seattle Tacoma Intl",
                    47.45,
                    -122.31,
                    3500,
                    SurfaceType::Hard,
                ),
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("North America".to_string()),
        });

        let aircraft = vec![make_test_aircraft("Cessna 172", vec!["GA", "Prop"])];

        // Using airport names (AirportName constraint) — both explicit
        let plan = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from Los Angeles to Seattle",
            None,
            None,
        );
        assert!(
            plan.is_ok(),
            "Explicit name-to-name should bypass distance filter: {:?}",
            plan.err()
        );
        let p = plan.unwrap();
        assert_eq!(p.origin.id, "KLAX");
        assert_eq!(p.destination.id, "KSEA");

        // Using ICAO codes — both explicit
        let plan2 = generate_flight(
            &manager.packs,
            &aircraft,
            "Flight from KLAX to KSEA",
            None,
            None,
        );
        assert!(
            plan2.is_ok(),
            "Explicit ICAO-to-ICAO should bypass distance filter: {:?}",
            plan2.err()
        );
    }

    #[test]
    fn test_boeing_314_seaplane_restriction() {
        // Pack with Sealanes
        let sealanes_pack = SceneryPack {
            name: "B314 Sealanes".to_string(),
            path: PathBuf::from("/xplane/x-plane/X-Plane12/Custom Scenery/B314 Sealanes/"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::CustomAirport,
            airports: vec![
                Airport {
                    id: "WATER1".to_string(),
                    name: "Sea Base 1".to_string(),
                    airport_type: AirportType::Seaplane,
                    lat: Some(45.0),
                    lon: Some(-123.0),
                    proj_x: None,
                    proj_y: None,
                    max_runway_length: Some(0),
                    surface_type: Some(SurfaceType::Water),
                },
                Airport {
                    id: "WATER2".to_string(),
                    name: "Sea Base 2".to_string(),
                    airport_type: AirportType::Seaplane,
                    lat: Some(46.0),
                    lon: Some(-123.0),
                    proj_x: None,
                    proj_y: None,
                    max_runway_length: Some(0),
                    surface_type: Some(SurfaceType::Water),
                },
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("US:Oregon".to_string()),
        };
        // Standard pack
        let standard_pack = SceneryPack {
            name: "KSEA".to_string(),
            path: PathBuf::from("/xplane/X-Plane 12/Custom Scenery/KSEA/"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "KSEA",
                "Seattle",
                47.45,
                -122.3,
                10000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("US:Washington".to_string()),
        };

        let packs = vec![sealanes_pack, standard_pack];
        let aircraft = vec![make_test_aircraft("Boeing 314", vec!["seaplane", "Prop"])];

        // Random flight should use Sea Base
        let plan = generate_flight(&packs, &aircraft, "random flight", None, None);
        assert!(
            plan.is_ok(),
            "Should generate flight for B314: {:?}",
            plan.err()
        );
        let p = plan.unwrap();
        assert!(p.origin.id == "WATER1" || p.origin.id == "WATER2");
        assert!(p.destination.id == "WATER1" || p.destination.id == "WATER2");
    }

    #[test]
    fn test_oregon_region_matching() {
        let packs = vec![SceneryPack {
            name: "Oregon Airports".to_string(),
            path: PathBuf::from("OR"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::CustomAirport,
            airports: vec![
                make_test_airport("KPDX", "Portland", 45.5, -122.6, 10000, SurfaceType::Hard),
                make_test_airport("KEUG", "Eugene", 44.1, -123.2, 8000, SurfaceType::Hard),
            ],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("US:Oregon".to_string()),
        }];
        let aircraft = vec![make_test_aircraft("Cessna 172", vec!["GA"])];

        let plan = generate_flight(
            &packs,
            &aircraft,
            "Flight from Oregon to anywhere",
            None,
            None,
        );
        assert!(
            plan.is_ok(),
            "Oregon region should be recognized: {:?}",
            plan.err()
        );
        let p = plan.unwrap();
        assert!(p.origin.id == "KPDX" || p.origin.id == "KEUG");
    }

    #[test]
    fn test_flight_prefix_regex_fix() {
        let packs = vec![
            SceneryPack {
                name: "KLAX".to_string(),
                path: PathBuf::from("KLAX"),
                raw_path: None,
                status: SceneryPackType::Active,
                category: SceneryCategory::CustomAirport,
                airports: vec![make_test_airport(
                    "KLAX",
                    "LAX",
                    33.9,
                    -118.4,
                    12000,
                    SurfaceType::Hard,
                )],
                tiles: vec![],
                tags: vec![],
                descriptor: SceneryDescriptor::default(),
                region: Some("US:SoCal".to_string()),
            },
            SceneryPack {
                name: "EGLL".to_string(),
                path: PathBuf::from("EGLL"),
                raw_path: None,
                status: SceneryPackType::Active,
                category: SceneryCategory::CustomAirport,
                airports: vec![make_test_airport(
                    "EGLL",
                    "Heathrow",
                    51.4,
                    -0.4,
                    12000,
                    SurfaceType::Hard,
                )],
                tiles: vec![],
                tags: vec![],
                descriptor: SceneryDescriptor::default(),
                region: Some("United Kingdom".to_string()),
            },
        ];
        let aircraft = vec![make_test_aircraft("Boeing 747", vec!["jet", "heavy"])];

        let plan = generate_flight(&packs, &aircraft, "Flight KLAX to EGLL", None, None);
        assert!(
            plan.is_ok(),
            "Should handle 'Flight' prefix without 'from': {:?}",
            plan.err()
        );
        let p = plan.unwrap();
        assert_eq!(p.origin.id, "KLAX");
        assert_eq!(p.destination.id, "EGLL");
    }
    #[test]
    fn test_unknown_runway_length_not_filtered() {
        // After the guardrail simplification, runway length is no longer enforced.
        // Any aircraft (GA or heavy) can use an airport with unknown runway data —
        // the user controls appropriateness via keywords and can swap aircraft after export.
        let packs = vec![SceneryPack {
            path: PathBuf::from("Custom Scenery/TestPack"),
            name: "TestPack".to_string(),
            airports: vec![
                Airport {
                    id: "UNKN".to_string(),
                    name: "Unknown Runway Strip".to_string(),
                    airport_type: AirportType::Land,
                    lat: Some(51.5),
                    lon: Some(-0.12),
                    proj_x: None,
                    proj_y: None,
                    max_runway_length: None,
                    surface_type: Some(SurfaceType::Soft),
                },
                make_test_airport("EGLL", "Heathrow", 52.5, -0.45, 12000, SurfaceType::Hard),
            ],
            region: None,
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
        }];

        // Both GA and heavy aircraft should freely use UNKN (no runway length filter)
        for tags in [vec!["GA", "Prop"], vec!["Heavy", "Jet"]] {
            let aircraft = vec![make_test_aircraft("Test Aircraft", tags)];
            let mut found_unkn = false;
            for _ in 0..20 {
                let plan =
                    generate_flight(&packs, &aircraft, "Flight from UNKN to any", None, None);
                if let Ok(p) = plan {
                    if p.origin.id == "UNKN" {
                        found_unkn = true;
                        break;
                    }
                }
            }
            assert!(
                found_unkn,
                "Aircraft should be able to depart from UNKN regardless of runway data"
            );
        }
    }
    #[test]
    fn test_alaska_flight_generation() {
        // "F70 to Alaska"
        // F70 = French Valley (CA)
        // Alaska = Region US:AK
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        manager.packs.push(SceneryPack {
            name: "California".to_string(),
            path: PathBuf::from("CA"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "F70",
                "French Valley",
                33.57,
                -117.13,
                4600,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("US:SoCal".to_string()),
        });
        manager.packs.push(SceneryPack {
            name: "Alaska".to_string(),
            path: PathBuf::from("AK"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "PANC",
                "Anchorage Intl",
                61.17,
                -149.99,
                10000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("US:AK".to_string()),
        });

        // Need long range aircraft. F70->PANC is ~1800nm.
        let aircraft = vec![make_test_aircraft("Boeing 747", vec!["Heavy", "Jet"])];

        // "F70 to Alaska" -> "Alaska" matches Region US:AK via RegionIndex (or alias)
        // This test proves that if parsing works, flight gen works.
        let plan = generate_flight(&manager.packs, &aircraft, "F70 to Alaska", None, None);
        assert!(
            plan.is_ok(),
            "Should find PANC in Alaska from F70: {:?}",
            plan.err()
        );
        let p = plan.unwrap();
        assert_eq!(p.destination.id, "PANC");
    }

    #[test]
    fn test_alaska_flight_generation_cessna() {
        let mut manager = SceneryManager::new(PathBuf::from("/tmp/scenery_packs.ini"));
        manager.packs.push(SceneryPack {
            name: "California".to_string(),
            path: PathBuf::from("CA"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "F70",
                "French Valley",
                33.57,
                -117.13,
                4600,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("US:SoCal".to_string()),
        });
        manager.packs.push(SceneryPack {
            name: "Alaska".to_string(),
            path: PathBuf::from("AK"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: vec![make_test_airport(
                "PANC",
                "Anchorage",
                61.17,
                -149.99,
                10000,
                SurfaceType::Hard,
            )],
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: Some("US:AK".to_string()),
        });

        // Cessna 172 (GA, Prop) -> Default range 30-500nm, but should RELAX because endpoints are explicit
        let aircraft = vec![make_test_aircraft("Cessna 172", vec!["GA", "Prop"])];

        let plan = generate_flight(&manager.packs, &aircraft, "F70 to Alaska", None, None);
        assert!(
            plan.is_ok(),
            "Should find PANC (Anchorage) from F70 even with Cessna, because destination is explicit: {:?}",
            plan.err()
        );
        let p = plan.unwrap();
        assert_eq!(p.destination.id, "PANC");
    }

    /// When the destination is a NearCity, a helipad that is the geometrically closest airport
    /// must NOT be selected for a non-helicopter aircraft.  Regression for the bug where
    /// valid_dests only applied type-safety for ICAO destinations, allowing helipads through
    /// for NearCity / Region / AirportName targets.
    #[test]
    fn test_nearcity_dest_does_not_pick_helipad() {
        let helipad = Airport {
            id: "HOSP".to_string(),
            name: "City Hospital Helipad".to_string(),
            airport_type: AirportType::Heliport,
            lat: Some(51.50),
            lon: Some(-0.12),
            proj_x: None,
            proj_y: None,
            max_runway_length: None,
            surface_type: Some(SurfaceType::Hard),
        };
        let real_airport = make_test_airport(
            "EGLL",
            "London Heathrow",
            51.47,
            -0.45,
            3900,
            SurfaceType::Hard,
        );
        let origin = make_test_airport(
            "EHAM",
            "Amsterdam Schiphol",
            52.31,
            4.76,
            3800,
            SurfaceType::Hard,
        );

        let pack = SceneryPack {
            path: PathBuf::from("Custom Scenery/Test"),
            name: "Test".to_string(),
            airports: vec![helipad, real_airport, origin],
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            tiles: vec![],
            tags: vec![],
            descriptor: SceneryDescriptor::default(),
            region: None,
        };

        // Use "Jet" only (not "Airliner") — "Airliner" triggers is_heavy → min_dist=200nm,
        // but EHAM→EGLL is only ~199nm. With just "Jet", min_dist=50nm so the route passes.
        let aircraft = vec![make_test_aircraft("Boeing 737", vec!["Jet"])];

        // "to London" uses the TO_RE path → destination = NearCity("London"), origin = wildcard.
        for _ in 0..20 {
            let plan = generate_flight(&[pack.clone()], &aircraft, "to London", None, None);
            assert!(plan.is_ok(), "Should find EGLL, not fail: {:?}", plan.err());
            let p = plan.unwrap();
            assert_ne!(
                p.destination.id, "HOSP",
                "Helipad must NOT be picked as destination for a Jet (NearCity bug)"
            );
            assert_eq!(
                p.destination.id, "EGLL",
                "Should pick EGLL, not the helipad"
            );
        }
    }
}
