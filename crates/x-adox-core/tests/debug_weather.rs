use x_adox_core::weather::WeatherEngine;

#[test]
fn debug_weather_map_errors() {
    let engine = WeatherEngine::new();
    let map = engine.get_global_weather_map().expect("Failed map");
    println!("Parsed {} stations", map.len());
    panic!("Show me output");
}

           
       