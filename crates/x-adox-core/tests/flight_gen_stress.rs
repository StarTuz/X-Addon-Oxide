//! Randomized stress test for flight generation.
//!
//! Simulates a human rapidly typing diverse flight prompts
//! (cities, regions, ICAO codes, keywords) and hammering Regenerate.
//!
//! Run:  cargo test -p x-adox-core --test flight_gen_stress -- --nocapture
//! Seed: set STRESS_SEED=<u64> for reproducible failures.

use std::path::PathBuf;
use x_adox_bitnet::flight_prompt::{AircraftConstraint, FlightPrompt, LocationConstraint};
use x_adox_core::apt_dat::{Airport, AirportType, SurfaceType};
use x_adox_core::discovery::{AcfVariant, AddonType, DiscoveredAddon};
use x_adox_core::flight_gen::{generate_flight, generate_flight_from_prompt, AirportPool};
use x_adox_core::scenery::{SceneryCategory, SceneryDescriptor, SceneryPack, SceneryPackType};

// ─── Mock helpers ────────────────────────────────────────────────────

fn apt(id: &str, name: &str, lat: f64, lon: f64, len: u32, surf: SurfaceType) -> Airport {
    Airport {
        id: id.to_string(),
        name: name.to_string(),
        airport_type: AirportType::Land,
        lat: Some(lat),
        lon: Some(lon),
        proj_x: None,
        proj_y: None,
        max_runway_length: Some(len),
        surface_type: Some(surf),
    }
}

fn heli(id: &str, name: &str, lat: f64, lon: f64) -> Airport {
    Airport {
        id: id.to_string(),
        name: name.to_string(),
        airport_type: AirportType::Heliport,
        lat: Some(lat),
        lon: Some(lon),
        proj_x: None,
        proj_y: None,
        max_runway_length: None,
        surface_type: Some(SurfaceType::Hard),
    }
}

fn seaplane(id: &str, name: &str, lat: f64, lon: f64) -> Airport {
    Airport {
        id: id.to_string(),
        name: name.to_string(),
        airport_type: AirportType::Seaplane,
        lat: Some(lat),
        lon: Some(lon),
        proj_x: None,
        proj_y: None,
        max_runway_length: Some(0),
        surface_type: Some(SurfaceType::Water),
    }
}

/// Airport with NO runway data (simulates base-layer gaps like OKBK)
fn apt_no_rwy(id: &str, name: &str, lat: f64, lon: f64) -> Airport {
    Airport {
        id: id.to_string(),
        name: name.to_string(),
        airport_type: AirportType::Land,
        lat: Some(lat),
        lon: Some(lon),
        proj_x: None,
        proj_y: None,
        max_runway_length: None,
        surface_type: None,
    }
}

fn acft(name: &str, tags: &[&str]) -> DiscoveredAddon {
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
        tags: tags.iter().map(|s| s.to_string()).collect(),
        is_laminar_default: false,
    }
}

fn pack(name: &str, airports: Vec<Airport>) -> SceneryPack {
    SceneryPack {
        name: name.to_string(),
        path: PathBuf::from(format!("Custom Scenery/{}", name)),
        raw_path: None,
        status: SceneryPackType::Active,
        category: SceneryCategory::GlobalAirport,
        airports,
        tiles: Vec::new(),
        tags: Vec::new(),
        descriptor: SceneryDescriptor::default(),
        region: None,
    }
}

// ─── Simple xorshift RNG (no extra dep) ──────────────────────────────

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0xDEAD_BEEF } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        let idx = (self.next_u64() as usize) % slice.len();
        &slice[idx]
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.next_u64() % 100 < pct
    }
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() & 0xFFFFFFFFFFFF) as f64 / (0xFFFFFFFFFFFFu64 as f64)
    }
}

// ─── Prompt building blocks ──────────────────────────────────────────

const CITIES: &[&str] = &[
    "London",
    "Paris",
    "Tokyo",
    "New York",
    "Dubai",
    "Sydney",
    "Bangkok",
    "Singapore",
    "Rome",
    "Madrid",
    "Berlin",
    "Munich",
    "Istanbul",
    "Cairo",
    "Nairobi",
    "Johannesburg",
    "Mumbai",
    "Delhi",
    "Beijing",
    "Shanghai",
    "Seoul",
    "Osaka",
    "Toronto",
    "Chicago",
    "Miami",
    "Los Angeles",
    "San Francisco",
    "Seattle",
    "Denver",
    "Atlanta",
    "Dallas",
    "Lima",
    "Buenos Aires",
    "Rio de Janeiro",
    "Sao Paulo",
    "Bogota",
    "Santiago",
    "Hong Kong",
    "Taipei",
    "Manila",
    "Hanoi",
    "Jakarta",
    "Bali",
    "Auckland",
    "Melbourne",
    "Brisbane",
    "Perth",
    "Queenstown",
    "Casablanca",
    "Marrakech",
    "Cape Town",
    "Mombasa",
    "Lamu",
    "Kuwait",
    "Doha",
    "Abu Dhabi",
    "Riyadh",
    "Jeddah",
    "Amman",
    "Beirut",
    "Tel Aviv",
    "Kuala Lumpur",
];

const REGIONS: &[&str] = &[
    "UK",
    "France",
    "Germany",
    "Italy",
    "Spain",
    "Greece",
    "Japan",
    "Australia",
    "New Zealand",
    "Canada",
    "USA",
    "Brazil",
    "Kenya",
    "South Africa",
    "India",
    "China",
    "Europe",
    "Asia",
    "Africa",
    "North America",
    "South America",
    "NorCal",
    "SoCal",
    "England",
    "Scotland",
    "Ireland",
];

const ICAOS: &[&str] = &[
    "EGLL", "LFPG", "KJFK", "KLAX", "RJAA", "OMDB", "YSSY", "VTBS", "LIRF", "LEMD", "EDDF", "LTFM",
    "HECA", "HKJK", "FAOR", "VABB", "VIDP", "ZBAA", "ZSPD", "RKSI", "RJBB", "CYYZ", "KORD", "KMIA",
    "KSFO", "KSEA", "KDEN", "KATL", "KDFW", "SPJC", "SCEL", "SBGR", "NZAA", "OKBK", "OTHH", "OEJN",
];

const AIRCRAFT_TAGS: &[&str] = &[
    "Cessna",
    "Boeing",
    "Airbus",
    "helicopter",
    "bush",
    "737",
    "747",
    "A320",
    "King Air",
    "Pilatus",
];

const MODIFIERS: &[&str] = &[
    "",
    "short flight",
    "long haul",
    "medium flight",
    "ignore guardrails",
    "bush flight",
];

fn build_random_prompt(rng: &mut Rng) -> String {
    let style = rng.next_u64() % 7;
    let mut parts: Vec<String> = Vec::new();

    // Optional prefix
    if rng.chance(50) {
        parts.push("Flight".to_string());
    }

    match style {
        0 => {
            // City to City: "London to Paris"
            parts.push("from".to_string());
            parts.push(rng.pick(CITIES).to_string());
            parts.push("to".to_string());
            parts.push(rng.pick(CITIES).to_string());
        }
        1 => {
            // ICAO to ICAO: "EGLL to KJFK"
            parts.push("from".to_string());
            parts.push(rng.pick(ICAOS).to_string());
            parts.push("to".to_string());
            parts.push(rng.pick(ICAOS).to_string());
        }
        2 => {
            // Region to Region: "UK to France"
            parts.push("from".to_string());
            parts.push(rng.pick(REGIONS).to_string());
            parts.push("to".to_string());
            parts.push(rng.pick(REGIONS).to_string());
        }
        3 => {
            // City to Region: "London to Italy"
            parts.push("from".to_string());
            parts.push(rng.pick(CITIES).to_string());
            parts.push("to".to_string());
            parts.push(rng.pick(REGIONS).to_string());
        }
        4 => {
            // ICAO to City: "EGLL to Tokyo"
            parts.push("from".to_string());
            parts.push(rng.pick(ICAOS).to_string());
            parts.push("to".to_string());
            parts.push(rng.pick(CITIES).to_string());
        }
        5 => {
            // Destination only: "to Paris"
            parts.push("to".to_string());
            parts.push(rng.pick(CITIES).to_string());
        }
        _ => {
            // Just a city name: "Dubai"
            parts.push(rng.pick(CITIES).to_string());
        }
    }

    // Optional aircraft
    if rng.chance(40) {
        parts.push("using".to_string());
        parts.push(rng.pick(AIRCRAFT_TAGS).to_string());
    }

    // Optional modifier
    let modifier = rng.pick(MODIFIERS);
    if !modifier.is_empty() && rng.chance(30) {
        parts.push(modifier.to_string());
    }

    parts.join(" ")
}

fn build_industrial_struct(rng: &mut Rng) -> FlightPrompt {
    let mut prompt = FlightPrompt::default();
    match rng.next_u64() % 4 {
        0 => {
            // From Sxxxx To Sxxxx
            let o = format!("S{:04X}", rng.next_u64() % 0xFFFF);
            let d = format!("S{:04X}", rng.next_u64() % 0xFFFF);
            prompt.origin = Some(LocationConstraint::ICAO(o));
            prompt.destination = Some(LocationConstraint::ICAO(d));
        }
        1 => {
            // Industrial N to Industrial M
            let o = format!("Industrial {}", rng.next_u64() % 44000);
            let d = format!("Industrial {}", rng.next_u64() % 44000);
            prompt.origin = Some(LocationConstraint::AirportName(o));
            prompt.destination = Some(LocationConstraint::AirportName(d));
        }
        2 => {
            // To Sxxxx (from Any)
            let d = format!("S{:04X}", rng.next_u64() % 0xFFFF);
            prompt.destination = Some(LocationConstraint::ICAO(d));
            prompt.origin = None; // Any
        }
        3 => {
            // Industrial N (Origin?, Dest Any?)
            let n = format!("Industrial {}", rng.next_u64() % 44000);
            prompt.origin = Some(LocationConstraint::AirportName(n));
            prompt.destination = None; // Any
        }
        _ => unreachable!(),
    }
    // Random aircraft constraint
    if rng.chance(50) {
        prompt.aircraft = Some(AircraftConstraint::Tag("Jet".to_string()));
    }
    prompt
}

// ─── World builder ───────────────────────────────────────────────────

fn build_world() -> (Vec<SceneryPack>, Vec<DiscoveredAddon>) {
    let airports = vec![
        // ── Europe ──
        apt(
            "EGLL",
            "London Heathrow",
            51.47,
            -0.45,
            3900,
            SurfaceType::Hard,
        ),
        apt(
            "EGLC",
            "London City",
            51.505,
            0.055,
            1508,
            SurfaceType::Hard,
        ),
        apt(
            "EGKK",
            "London Gatwick",
            51.15,
            -0.19,
            3316,
            SurfaceType::Hard,
        ),
        apt(
            "EGSS",
            "London Stansted",
            51.885,
            0.235,
            3048,
            SurfaceType::Hard,
        ),
        apt(
            "LFPG",
            "Paris Charles de Gaulle",
            49.00,
            2.55,
            4200,
            SurfaceType::Hard,
        ),
        apt("LFPO", "Paris Orly", 48.73, 2.37, 3650, SurfaceType::Hard),
        apt(
            "EDDF",
            "Frankfurt Main",
            50.03,
            8.57,
            4000,
            SurfaceType::Hard,
        ),
        apt("EDDM", "Munich Intl", 48.35, 11.78, 4000, SurfaceType::Hard),
        apt(
            "LIRF",
            "Rome Fiumicino",
            41.80,
            12.24,
            3900,
            SurfaceType::Hard,
        ),
        apt(
            "LEMD",
            "Madrid Barajas",
            40.47,
            -3.57,
            4349,
            SurfaceType::Hard,
        ),
        apt("LGAV", "Athens Intl", 37.94, 23.94, 4000, SurfaceType::Hard),
        apt(
            "LTFM",
            "Istanbul Airport",
            41.26,
            28.74,
            4100,
            SurfaceType::Hard,
        ),
        apt(
            "EDDB",
            "Berlin Brandenburg",
            52.36,
            13.51,
            4000,
            SurfaceType::Hard,
        ),
        apt("LOWW", "Vienna Intl", 48.11, 16.57, 3600, SurfaceType::Hard),
        apt(
            "EHAM",
            "Amsterdam Schiphol",
            52.31,
            4.76,
            3800,
            SurfaceType::Hard,
        ),
        apt("EGPH", "Edinburgh", 55.95, -3.37, 2525, SurfaceType::Hard),
        apt("EIDW", "Dublin Intl", 53.42, -6.27, 2637, SurfaceType::Hard),
        // ── Middle East ──
        apt("OMDB", "Dubai Intl", 25.25, 55.36, 4000, SurfaceType::Hard),
        apt(
            "OMAA",
            "Abu Dhabi Intl",
            24.43,
            54.65,
            4100,
            SurfaceType::Hard,
        ),
        apt_no_rwy("OKBK", "Kuwait Intl", 29.23, 47.97),
        apt(
            "OTHH",
            "Doha Hamad Intl",
            25.26,
            51.61,
            4850,
            SurfaceType::Hard,
        ),
        apt(
            "OEJN",
            "Jeddah King Abdulaziz",
            21.68,
            39.16,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "OERK",
            "Riyadh King Khalid",
            24.96,
            46.70,
            4205,
            SurfaceType::Hard,
        ),
        apt(
            "LLBG",
            "Tel Aviv Ben Gurion",
            32.01,
            34.89,
            3657,
            SurfaceType::Hard,
        ),
        apt(
            "OLBA",
            "Beirut Rafic Hariri",
            33.82,
            35.49,
            3395,
            SurfaceType::Hard,
        ),
        apt(
            "OJAI",
            "Amman Queen Alia",
            31.72,
            35.99,
            3660,
            SurfaceType::Hard,
        ),
        apt("OOMS", "Muscat Intl", 23.59, 58.28, 4000, SurfaceType::Hard),
        // ── Africa ──
        apt("HECA", "Cairo Intl", 30.12, 31.41, 3300, SurfaceType::Hard),
        apt(
            "HKJK",
            "Nairobi Jomo Kenyatta",
            -1.32,
            36.93,
            4117,
            SurfaceType::Hard,
        ),
        apt(
            "HKMO",
            "Mombasa Moi Intl",
            -4.03,
            39.60,
            3350,
            SurfaceType::Hard,
        ),
        apt("HKLU", "Lamu Manda", -2.25, 40.91, 1050, SurfaceType::Soft),
        apt(
            "FAOR",
            "Johannesburg OR Tambo",
            -26.13,
            28.24,
            4418,
            SurfaceType::Hard,
        ),
        apt(
            "FACT",
            "Cape Town Intl",
            -33.97,
            18.60,
            3201,
            SurfaceType::Hard,
        ),
        apt(
            "GMMN",
            "Casablanca Mohammed V",
            33.37,
            -7.59,
            3720,
            SurfaceType::Hard,
        ),
        apt(
            "GMMX",
            "Marrakech Menara",
            31.61,
            -8.04,
            3100,
            SurfaceType::Hard,
        ),
        // ── Asia ──
        apt(
            "RJAA",
            "Tokyo Narita",
            35.76,
            140.39,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "RJTT",
            "Tokyo Haneda",
            35.55,
            139.78,
            3360,
            SurfaceType::Hard,
        ),
        apt(
            "RJBB",
            "Osaka Kansai",
            34.43,
            135.24,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "VTBS",
            "Bangkok Suvarnabhumi",
            13.69,
            100.75,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "WSSS",
            "Singapore Changi",
            1.35,
            103.99,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "VHHH",
            "Hong Kong Intl",
            22.31,
            113.91,
            3800,
            SurfaceType::Hard,
        ),
        apt(
            "RCTP",
            "Taipei Taoyuan",
            25.08,
            121.23,
            3660,
            SurfaceType::Hard,
        ),
        apt(
            "RKSI",
            "Seoul Incheon",
            37.46,
            126.44,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "ZBAA",
            "Beijing Capital",
            40.08,
            116.59,
            3800,
            SurfaceType::Hard,
        ),
        apt(
            "ZSPD",
            "Shanghai Pudong",
            31.14,
            121.81,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "ZGGG",
            "Guangzhou Baiyun",
            23.39,
            113.30,
            3800,
            SurfaceType::Hard,
        ),
        apt(
            "VABB",
            "Mumbai Chhatrapati Shivaji",
            19.09,
            72.87,
            3660,
            SurfaceType::Hard,
        ),
        apt(
            "VIDP",
            "Delhi Indira Gandhi",
            28.57,
            77.10,
            4430,
            SurfaceType::Hard,
        ),
        apt(
            "VOBL",
            "Bangalore Kempegowda",
            13.20,
            77.71,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "VOMM",
            "Chennai Intl",
            12.99,
            80.17,
            3658,
            SurfaceType::Hard,
        ),
        apt(
            "VECC",
            "Kolkata Netaji Subhas",
            22.65,
            88.45,
            3627,
            SurfaceType::Hard,
        ),
        apt(
            "WMKK",
            "Kuala Lumpur Intl",
            2.74,
            101.70,
            4050,
            SurfaceType::Hard,
        ),
        apt(
            "RPLL",
            "Manila Ninoy Aquino",
            14.51,
            121.02,
            3737,
            SurfaceType::Hard,
        ),
        apt(
            "VVNB",
            "Hanoi Noi Bai",
            21.22,
            105.81,
            3800,
            SurfaceType::Hard,
        ),
        apt(
            "WIII",
            "Jakarta Soekarno-Hatta",
            -6.13,
            106.66,
            3660,
            SurfaceType::Hard,
        ),
        apt(
            "WADD",
            "Bali Ngurah Rai",
            -8.75,
            115.17,
            3000,
            SurfaceType::Hard,
        ),
        // ── North America ──
        apt(
            "KJFK",
            "New York Kennedy Intl",
            40.64,
            -73.78,
            4423,
            SurfaceType::Hard,
        ),
        apt(
            "KLAX",
            "Los Angeles Intl",
            33.94,
            -118.40,
            3685,
            SurfaceType::Hard,
        ),
        apt(
            "KSFO",
            "San Francisco Intl",
            37.62,
            -122.37,
            3618,
            SurfaceType::Hard,
        ),
        apt(
            "KORD",
            "Chicago O'Hare Intl",
            41.97,
            -87.91,
            3963,
            SurfaceType::Hard,
        ),
        apt(
            "KATL",
            "Atlanta Hartsfield-Jackson",
            33.64,
            -84.43,
            3624,
            SurfaceType::Hard,
        ),
        apt("KMIA", "Miami Intl", 25.79, -80.29, 3962, SurfaceType::Hard),
        apt(
            "KSEA",
            "Seattle Tacoma Intl",
            47.45,
            -122.31,
            3408,
            SurfaceType::Hard,
        ),
        apt(
            "KDEN",
            "Denver Intl",
            39.86,
            -104.67,
            4877,
            SurfaceType::Hard,
        ),
        apt(
            "KDFW",
            "Dallas Fort Worth Intl",
            32.90,
            -97.04,
            4084,
            SurfaceType::Hard,
        ),
        apt(
            "CYYZ",
            "Toronto Pearson Intl",
            43.68,
            -79.63,
            3389,
            SurfaceType::Hard,
        ),
        // ── South America ──
        apt(
            "SBGR",
            "Sao Paulo Guarulhos",
            -23.44,
            -46.47,
            3700,
            SurfaceType::Hard,
        ),
        apt(
            "SBGL",
            "Rio de Janeiro Galeao",
            -22.81,
            -43.25,
            4000,
            SurfaceType::Hard,
        ),
        apt(
            "SAEZ",
            "Buenos Aires Ezeiza",
            -34.82,
            -58.54,
            3300,
            SurfaceType::Hard,
        ),
        apt(
            "SKBO",
            "Bogota El Dorado",
            4.70,
            -74.15,
            3800,
            SurfaceType::Hard,
        ),
        apt(
            "SPJC",
            "Lima Jorge Chavez",
            -12.02,
            -77.11,
            3507,
            SurfaceType::Hard,
        ),
        apt(
            "SCEL",
            "Santiago Arturo Merino",
            -33.39,
            -70.79,
            3748,
            SurfaceType::Hard,
        ),
        // ── Oceania ──
        apt(
            "YSSY",
            "Sydney Kingsford Smith",
            -33.95,
            151.18,
            3962,
            SurfaceType::Hard,
        ),
        apt(
            "YMML",
            "Melbourne Tullamarine",
            -37.67,
            144.84,
            3657,
            SurfaceType::Hard,
        ),
        apt(
            "YBBN",
            "Brisbane Airport",
            -27.38,
            153.12,
            3560,
            SurfaceType::Hard,
        ),
        apt(
            "YPPH",
            "Perth Airport",
            -31.94,
            115.97,
            3444,
            SurfaceType::Hard,
        ),
        apt(
            "NZAA",
            "Auckland Airport",
            -37.01,
            174.79,
            3635,
            SurfaceType::Hard,
        ),
        apt(
            "NZWN",
            "Wellington Airport",
            -41.33,
            174.81,
            1936,
            SurfaceType::Hard,
        ),
        apt(
            "NZQN",
            "Queenstown Airport",
            -45.02,
            168.74,
            1676,
            SurfaceType::Hard,
        ),
        // ── Bush strips (short, soft) ──
        apt(
            "0AK2",
            "Bush Strip Alaska",
            62.0,
            -150.0,
            600,
            SurfaceType::Soft,
        ),
        apt(
            "FZBO",
            "Bush Strip Congo",
            -4.3,
            20.6,
            550,
            SurfaceType::Soft,
        ),
        // ── Heliports ──
        heli("HELI1", "London Heliport", 51.47, -0.18),
        heli("HELI2", "Manhattan Heliport", 40.70, -74.01),
        // ── Seaplane bases ──
        seaplane("SEA1", "Puget Sound Seaplane", 47.6, -122.4),
    ];

    let packs = vec![pack("Global Airports", airports)];

    let fleet = vec![
        acft("Cessna 172", &["General Aviation", "Prop"]),
        acft("Cessna 208 Caravan", &["General Aviation", "Turboprop"]),
        acft("King Air 350", &["Turboprop"]),
        acft("Pilatus PC-12", &["Turboprop", "Bush"]),
        acft("Boeing 737-800", &["Jet", "Airliner"]),
        acft("Boeing 747-400", &["Jet", "Heavy", "Airliner"]),
        acft("Airbus A320", &["Jet", "Airliner"]),
        acft("CH47 Chinook", &["Helicopter"]),
        acft("Bell 407", &["Helicopter"]),
    ];

    (packs, fleet)
}

fn build_industrial_world(count: usize, seed: u64) -> (Vec<SceneryPack>, Vec<DiscoveredAddon>) {
    eprintln!("Building industrial world with {} airports...", count);
    let mut rng = Rng::new(seed);
    let mut airports = Vec::with_capacity(count);

    for i in 0..count {
        if i > 0 && i % 10_000 == 0 {
            eprintln!("  ...generated {} airports", i);
        }
        let (lat, lon) = if rng.chance(60) {
            // High density: Europe / North America
            (
                30.0 + rng.next_f64() * 30.0,
                -125.0 + rng.next_f64() * 170.0,
            )
        } else {
            // Global sparse distribution
            (
                -60.0 + rng.next_f64() * 140.0,
                -180.0 + rng.next_f64() * 360.0,
            )
        };

        let surf = if rng.chance(60) {
            SurfaceType::Hard
        } else {
            SurfaceType::Soft
        };

        let len = 500 + (rng.next_u64() % 4500) as u32;
        let id = format!("S{:04X}", i % 0xFFFF);
        airports.push(apt(&id, &format!("Industrial {}", i), lat, lon, len, surf));
    }

    let packs = vec![pack("Industrial Global", airports)];
    let (_, fleet) = build_world(); // Reuse the same fleet
    (packs, fleet)
}

// ─── Stress test ─────────────────────────────────────────────────────

#[test]
fn stress_random_prompts() {
    let seed: u64 = std::env::var("STRESS_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            eprintln!("STRESS_SEED={}", t);
            t
        });
    let mut rng = Rng::new(seed);

    let (packs, fleet) = build_world();
    let iterations = 500;
    let mut ok_count = 0;
    let mut err_count = 0;
    let mut failures: Vec<(String, String)> = Vec::new();

    let start = std::time::Instant::now();

    for i in 0..iterations {
        let prompt = build_random_prompt(&mut rng);
        match generate_flight(&packs, &fleet, &prompt, None, None, None) {
            Ok(plan) => {
                ok_count += 1;
                // Sanity: origin != dest
                assert_ne!(
                    plan.origin.id, plan.destination.id,
                    "[{}] origin == dest for '{}': {}",
                    i, prompt, plan.origin.id
                );
                // Sanity: distance > 0
                assert!(
                    plan.distance_nm > 0,
                    "[{}] zero distance for '{}'",
                    i,
                    prompt
                );
            }
            Err(e) => {
                err_count += 1;
                failures.push((prompt, e));
            }
        }
    }

    let elapsed = start.elapsed();

    // Print comprehensive report
    eprintln!("\n═══════════════════════════════════════════════════════");
    eprintln!("  STRESS TEST REPORT  (seed={})", seed);
    eprintln!("═══════════════════════════════════════════════════════");
    eprintln!("  Iterations:   {}", iterations);
    eprintln!(
        "  Succeeded:    {} ({:.1}%)",
        ok_count,
        ok_count as f64 / iterations as f64 * 100.0
    );
    eprintln!(
        "  Failed:       {} ({:.1}%)",
        err_count,
        err_count as f64 / iterations as f64 * 100.0
    );
    eprintln!(
        "  Elapsed:      {:.2}s ({:.1} flights/sec)",
        elapsed.as_secs_f64(),
        iterations as f64 / elapsed.as_secs_f64()
    );
    eprintln!("───────────────────────────────────────────────────────");

    if !failures.is_empty() {
        // Group failures by error type
        let mut error_groups: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for (prompt, err) in &failures {
            error_groups
                .entry(err.clone())
                .or_default()
                .push(prompt.clone());
        }
        for (err, prompts) in &error_groups {
            eprintln!("\n  ERROR: {}", err);
            for p in prompts.iter().take(5) {
                eprintln!("    → \"{}\"", p);
            }
            if prompts.len() > 5 {
                eprintln!("    … and {} more", prompts.len() - 5);
            }
        }
        eprintln!("───────────────────────────────────────────────────────");
    }

    // The success rate should be at least 50%.
    // Some failures are expected (e.g., "from Bali to Bali" picks the same
    // airport for origin/dest, or very obscure combos with no coverage).
    let success_rate = ok_count as f64 / iterations as f64;
    assert!(
        success_rate >= 0.50,
        "Success rate {:.1}% is below 50% threshold. {} failures out of {}. Seed: {}",
        success_rate * 100.0,
        err_count,
        iterations,
        seed,
    );
}

/// Simulates hitting "Regenerate" 50 times on the same prompt.
/// This catches the Kuwait bug: same prompt, different RNG seed → different aircraft → failure.
#[test]
fn stress_regenerate_same_prompt() {
    let (packs, fleet) = build_world();

    let prompts = &[
        "London to Kuwait",
        "London to Dubai",
        "Tokyo to Paris",
        "Nairobi to Lamu",
        "Sydney to Auckland",
        "from New York to London",
        "from EGLL to KJFK using Boeing",
        "from Dubai to Mumbai",
        "Chicago to Miami",
        "from KSFO to RJAA",
        "Berlin to Istanbul",
        "Cape Town to Cairo",
        "Lima to Bogota",
        "Seoul to Taipei",
        "from Doha to Singapore",
    ];

    let mut total_fails = 0;
    let regenerate_count = 50;

    for prompt in prompts {
        let mut fails = 0;
        for _ in 0..regenerate_count {
            if generate_flight(&packs, &fleet, prompt, None, None, None).is_err() {
                fails += 1;
            }
        }
        if fails > 0 {
            eprintln!(
                "REGEN FAIL: \"{}\" failed {}/{} times ({:.0}%)",
                prompt,
                fails,
                regenerate_count,
                fails as f64 / regenerate_count as f64 * 100.0,
            );
            total_fails += 1;
        }
    }

    assert_eq!(
        total_fails, 0,
        "{} prompts had regeneration failures (logged above)",
        total_fails,
    );
}

/// Specifically tests that airports with missing runway data don't block
/// explicit destination requests (the OKBK/Kuwait bug).
#[test]
fn stress_missing_runway_data_explicit_dest() {
    let airports = vec![
        apt(
            "EGLL",
            "London Heathrow",
            51.47,
            -0.45,
            3900,
            SurfaceType::Hard,
        ),
        apt_no_rwy("OKBK", "Kuwait Intl", 29.23, 47.97),
        apt_no_rwy("OIIE", "Tehran Imam Khomeini", 35.41, 51.15),
        apt_no_rwy("OPRN", "Islamabad Intl", 33.62, 72.83),
    ];

    let packs = vec![pack("Sparse", airports)];
    let fleet = vec![
        acft("Boeing 737-800", &["Jet", "Airliner"]),
        acft("Cessna 172", &["General Aviation"]),
        acft("CH47 Chinook", &["Helicopter"]),
    ];

    // Each should succeed regardless of which aircraft is randomly selected
    let explicit_prompts = &[
        "EGLL to OKBK",
        "EGLL to Kuwait",
        "London to Kuwait",
        "from EGLL to OIIE",
        "from London to OPRN",
    ];

    for prompt in explicit_prompts {
        for attempt in 0..20 {
            let result = generate_flight(&packs, &fleet, prompt, None, None, None);
            assert!(
                result.is_ok(),
                "Explicit dest prompt '{}' failed on attempt {}: {:?}",
                prompt,
                attempt,
                result.err(),
            );
        }
    }
}

#[test]
#[ignore = "throughput benchmark: run with `-- --ignored --nocapture`, takes ~35s"]
fn stress_industrial_massive() {
    let seed: u64 = 42;
    let (packs, mut fleet) = build_industrial_world(44000, seed);
    let mut rng = Rng::new(seed + 1);
    // Pre-filter fleet to avoid O(Fleet) work inside generate_flight
    fleet.retain(|a| matches!(a.addon_type, AddonType::Aircraft { .. }));
    let all_airports: Vec<Airport> = packs.iter().flat_map(|p| p.airports.clone()).collect();

    let iterations = 10_000;
    let mut ok_count = 0;
    let mut err_count = 0;
    let progress_step = 1000;
    eprintln!("World built. Starting {} iterations...", iterations);

    eprintln!("Building industrial pool (ICAO index)...");
    let pool = AirportPool::new(&all_airports);

    eprintln!(
        "\nStarting Industrial Massive Stress Test (44,000 airports, {} flights)...",
        iterations
    );
    let start = std::time::Instant::now();
    let mut last_checkpoint = start;

    for i in 1..=iterations {
        let prompt_struct = build_industrial_struct(&mut rng);
        match generate_flight_from_prompt(
            &[],
            &fleet,
            &prompt_struct,
            Some(&all_airports),
            None,
            Some(&pool),
        ) {
            Ok(_) => ok_count += 1,
            Err(_) => err_count += 1,
        }

        if i % progress_step == 0 {
            let now = std::time::Instant::now();
            let step_dur = now.duration_since(last_checkpoint).as_secs_f64();
            let total_dur = now.duration_since(start).as_secs_f64();
            eprintln!(
                "Progress: {:>6} / {} | OK: {:>6} | ERR: {:>4} | Speed: {:>7.1} fl/s (Step: {:.1}s)",
                i, iterations, ok_count, err_count,
                i as f64 / total_dur,
                step_dur
            );
            last_checkpoint = now;
        }
    }

    let elapsed = start.elapsed();
    eprintln!("\n═══════════════════════════════════════════════════════");
    eprintln!("  INDUSTRIAL MASSIVE REPORT");
    eprintln!("═══════════════════════════════════════════════════════");
    eprintln!("  Total Flights: {}", iterations);
    eprintln!(
        "  Succeeded:     {} ({:.1}%)",
        ok_count,
        ok_count as f64 / iterations as f64 * 100.0
    );
    eprintln!(
        "  Failed:        {} ({:.1}%)",
        err_count,
        err_count as f64 / iterations as f64 * 100.0
    );
    eprintln!("  Total Time:    {:.2}s", elapsed.as_secs_f64());
    eprintln!(
        "  Avg Throughput: {:.1} flights/sec",
        iterations as f64 / elapsed.as_secs_f64()
    );
    eprintln!("═══════════════════════════════════════════════════════\n");

    // ~49% success is structurally expected: 40% of airports are soft strips and
    // build_industrial_struct() adds a "Jet" constraint 50% of the time, which
    // can't land on soft/short strips. Assert a floor well below that to catch
    // catastrophic regressions (e.g., the pool breaking all lookups).
    let success_rate = ok_count as f64 / iterations as f64;
    assert!(
        success_rate >= 0.30,
        "Success rate {:.1}% is suspiciously low — AirportPool may be broken",
        success_rate * 100.0,
    );

    // The real invariant: throughput must not regress badly.
    // Release builds sustained ~295 fl/s; allow 5× headroom for debug/slow CI.
    let throughput = iterations as f64 / elapsed.as_secs_f64();
    assert!(
        throughput >= 50.0,
        "Throughput {:.1} fl/s is below the 50 fl/s floor — performance regressed",
        throughput,
    );
}
