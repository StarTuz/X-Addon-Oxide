use std::path::PathBuf;
use x_adox_bitnet::{BitNetModel, PredictContext};

#[test]
fn test_lfpg_classification() {
    let model = BitNetModel::new().unwrap();
    let ctx = PredictContext {
        has_airports: true, // Simulate discovery finding airports inside
        has_tiles: false,
        region_focus: None,
    };

    let cases = vec![
        ("LFPG - Paris Charles de Gaulle", 10),
        ("Aerosoft LFPG", 10),
        ("Paris Charles De Gaulle", 10),
        ("CDG Paris", 10),
        ("Tax2gate-Paris", 10),
        ("JustSim-LFPG", 10),
        ("LFPG_Scenery_Pack", 10),
        ("Charles De Gaulle", 10), // Expected Fail if heuristic is weak
        ("CDG_Airport", 10),
        ("Roissy-en-France", 10),
    ];

    for (name, expected) in cases {
        let (score, rule) = model.predict_with_rule_name(name, &PathBuf::from(name), &ctx);
        println!("Name: '{}' -> Score: {}, Rule: '{}'", name, score, rule);

        if expected < 30 {
            assert_eq!(
                score, expected,
                "Name '{}' expected score {}",
                name, expected
            );
        }

        // If score >= 25, it enters the "Low Priority" zone where exclusion issue happens.
        // We warn but don't fail, to see all output.
        if score > 25 {
            println!("  [WARN] Priority too low! Expected <= 25");
        }
    }
}

#[test]
fn test_lfpg_classification_no_discovery() {
    // Scenario: Discovery failed to find apt.dat or user hasn't scanned yet
    let model = BitNetModel::new().unwrap();
    let ctx = PredictContext {
        has_airports: false,
        has_tiles: false,
        region_focus: None,
    };

    let cases = vec![
        ("LFPG - Paris Charles de Gaulle", 10),
        ("Aerosoft LFPG", 10),
        ("Paris Charles De Gaulle", 10),
        ("Charles De Gaulle", 10),
    ];

    println!("\n--- NO DISCOVERY SCENARIO ---");
    for (name, _expected) in cases {
        let (score, rule) = model.predict_with_rule_name(name, &PathBuf::from(name), &ctx);
        println!("Name: '{}' -> Score: {}, Rule: '{}'", name, score, rule);

        if score > 25 {
            println!("  [WARN] Priority too low! Expected <= 25");
        }
    }
}
