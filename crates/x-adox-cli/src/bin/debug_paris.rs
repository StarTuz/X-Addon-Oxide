use x_adox_core::discovery::{AddonType, DiscoveredAddon};

fn main() {
    let x_root =
        std::env::var("XPLANE_ROOT").unwrap_or_else(|_| "/xplane/x-plane/X-Plane12".to_string());
    let apt_dat_path = std::path::PathBuf::from(&x_root)
        .join("Global Scenery")
        .join("Global Airports")
        .join("Earth nav data")
        .join("apt.dat");

    let airports = x_adox_core::apt_dat::AptDatParser::parse_file(&apt_dat_path).unwrap();

    let lfpg = airports.iter().find(|a| a.id == "LFPG").unwrap();
    println!(
        "LFPG details: max_len: {:?} ft (expected ~13740), surf: {:?}, type: {:?}",
        lfpg.max_runway_length, lfpg.surface_type, lfpg.airport_type
    );
    println!("LFPG width: {:?} ft", lfpg.max_runway_width);

    let _aircraft = DiscoveredAddon {
        name: "ToLissA321".to_string(),
        addon_type: AddonType::Aircraft {
            variants: vec![x_adox_core::discovery::AcfVariant {
                name: "A321".to_string(),
                file_name: "a321.acf".to_string(),
                icao_type: Some("A321".to_string()),
                num_engines: Some(2),
                vne_kts: Some(350),
                mtow_kg: Some(80000),
                min_rwy_len: Some(5000),
                rwy_req_pave: Some(2),
                is_enabled: true,
            }],
            livery_names: vec![],
            livery_count: 0,
        },
        path: std::path::PathBuf::from("/"),
        tags: vec![],
        is_laminar_default: false,
        is_enabled: true,
    };

    // Verify length is now > 5000
    if let Some(len) = lfpg.max_runway_length {
        if len >= 5000 {
            println!(
                "SUCCESS: LFPG runway length {}ft satisfies A321 requirement 5000ft",
                len
            );
        } else {
            println!(
                "FAILURE: LFPG runway length {}ft still less than 5000ft",
                len
            );
        }
    }
}
