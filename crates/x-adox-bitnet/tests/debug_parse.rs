use x_adox_bitnet::flight_prompt::FlightPrompt;

#[test]
fn debug_vfr() {
    let p = FlightPrompt::parse(
        "Flight in VFR conditions",
        &x_adox_bitnet::NLPRulesConfig::default(),
    );
    println!("VFR conditions: {:#?}", p);
    let p2 = FlightPrompt::parse(
        "Flight during storm",
        &x_adox_bitnet::NLPRulesConfig::default(),
    );
    println!("storm: {:#?}", p2);
    let p3 = FlightPrompt::parse("Flight in IFR", &x_adox_bitnet::NLPRulesConfig::default());
    println!("IFR: {:#?}", p3);
    let p4 = FlightPrompt::parse(
        "Flight VFR during bad weather",
        &x_adox_bitnet::NLPRulesConfig::default(),
    );
    println!("VFR bad weather: {:#?}", p4);
    panic!("Show me output");
}
