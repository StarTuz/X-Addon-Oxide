use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightPrompt {
    pub origin: Option<LocationConstraint>,
    pub destination: Option<LocationConstraint>,
    pub aircraft: Option<AircraftConstraint>,
    pub duration_minutes: Option<u32>,
    pub ignore_guardrails: bool,
    pub keywords: FlightKeywords,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FlightKeywords {
    pub duration: Option<DurationKeyword>,
    pub surface: Option<SurfaceKeyword>,
    pub flight_type: Option<TypeKeyword>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DurationKeyword {
    Short,  // < 45m / < 200nm
    Medium, // 45m - 2h / 200 - 800nm
    Long,   // > 2h / > 800nm
    Haul,   // > 4h / > 2000nm
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurfaceKeyword {
    Soft,  // Grass, Dirt, Unpaved
    Hard,  // Paved, Tarmac, Asphalt
    Water, // Seaplane bases, floatplanes
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeKeyword {
    Bush,     // Remote, short runway
    Regional, // Standard airports
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LocationConstraint {
    ICAO(String),
    Region(String),
    AirportName(String),
    /// Airport search near a named city center (lat/lon) within ~50 nm.
    NearCity {
        name: String,
        lat: f64,
        lon: f64,
    },
    Any,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AircraftConstraint {
    Tag(String), // Matches tags like "jet", "cessna", "heavy"
    Any,
}

impl Default for FlightPrompt {
    fn default() -> Self {
        Self {
            origin: None,
            destination: None,
            aircraft: None,
            duration_minutes: None,
            ignore_guardrails: false,
            keywords: FlightKeywords::default(),
        }
    }
}

impl FlightPrompt {
    pub fn parse(input: &str) -> Self {
        let mut prompt = FlightPrompt::default();
        let input_lower = input.to_lowercase();

        // 1. Check for "ignore guardrails"
        let mut clean_input = input_lower.clone();
        if clean_input.contains("ignore guardrails") {
            prompt.ignore_guardrails = true;
            clean_input = clean_input.replace("ignore guardrails", "");
        }

        // 2. Parse Keywords (Global search)
        // Duration
        if clean_input.contains("short")
            || clean_input.contains("hop")
            || clean_input.contains("quick")
        {
            prompt.keywords.duration = Some(DurationKeyword::Short);
        } else if clean_input.contains("medium") {
            prompt.keywords.duration = Some(DurationKeyword::Medium);
        } else if clean_input.contains("long haul") {
            prompt.keywords.duration = Some(DurationKeyword::Haul);
        } else if clean_input.contains("long") {
            prompt.keywords.duration = Some(DurationKeyword::Long);
        }

        // Surface
        if clean_input.contains("grass")
            || clean_input.contains("dirt")
            || clean_input.contains("gravel")
            || clean_input.contains("strip")
            || clean_input.contains("unpaved")
        {
            prompt.keywords.surface = Some(SurfaceKeyword::Soft);
        } else if clean_input.contains("paved")
            || clean_input.contains("tarmac")
            || clean_input.contains("concrete")
            || clean_input.contains("asphalt")
        {
            prompt.keywords.surface = Some(SurfaceKeyword::Hard);
        } else if clean_input.contains("water")
            || clean_input.contains("seaplane")
            || clean_input.contains("floatplane")
            || clean_input.contains("amphibian")
        {
            prompt.keywords.surface = Some(SurfaceKeyword::Water);
        }

        // Type
        if clean_input.contains("bush") || clean_input.contains("backcountry") {
            prompt.keywords.flight_type = Some(TypeKeyword::Bush);
            // Bush implies soft if not specified
            if prompt.keywords.surface.is_none() {
                prompt.keywords.surface = Some(SurfaceKeyword::Soft);
            }
        } else if clean_input.contains("regional") {
            prompt.keywords.flight_type = Some(TypeKeyword::Regional);
        }

        // 3. Parse Origin and Destination
        // Patterns: "from X to Y", "flight from X to Y", "X to Y"
        static LOC_RE: OnceLock<Regex> = OnceLock::new();
        let loc_re = LOC_RE.get_or_init(|| {
            Regex::new(
                r"(?:flight\s+from\s+|\bfrom\s+|^flight\s+)?(.+?)\s+\bto\b\s+(.+?)(\s+\busing\b|\s+\bin\b|\s+\bwith\b|\s+\bfor\b|$)",
            )
            .unwrap()
        });

        if let Some(caps) = loc_re.captures(&clean_input) {
            let origin_str = caps[1].trim();
            let dest_str = caps[2].trim();

            // Detect noise origins: words like "flight", "quick flight", "a hop" that
            // appear before "to" but are not locations. The LOC_RE can match these when
            // the optional "flight from" prefix is absent — e.g. "flight to Germany"
            // captures origin="flight", dest="germany" instead of dest-only.
            let is_noise_origin = origin_str == "flight"
                || origin_str.ends_with(" flight")
                || origin_str == "hop"
                || origin_str.ends_with(" hop")
                || origin_str == "a"
                || origin_str == "the";

            if is_noise_origin {
                // Treat as destination-only (same as "flight to X" / "to X" path)
                prompt.destination = Some(parse_location(dest_str));
            } else {
                prompt.origin = Some(parse_location(origin_str));
                prompt.destination = Some(parse_location(dest_str));
            }
        } else {
            // Fallback: Check for destination-only prompt "to X" or "flight to X"
            static TO_RE: OnceLock<Regex> = OnceLock::new();
            let to_re = TO_RE.get_or_init(|| {
                Regex::new(r"(?:^flight\s+to\s+|^to\s+)(.+?)(\s+\busing\b|\s+\bin\b|\s+\bwith\b|\s+\bfor\b|$)")
                    .unwrap()
            });
            if let Some(caps) = to_re.captures(&clean_input) {
                let dest_str = caps[1].trim();
                prompt.destination = Some(parse_location(dest_str));
            } else {
                // Fallback: "from X" without "to Y" (e.g. "2 hour flight from UK") — constrain origin only
                static FROM_RE: OnceLock<Regex> = OnceLock::new();
                let from_re =
                    FROM_RE.get_or_init(|| Regex::new(r"\bfrom\b\s+([a-zA-Z0-9\s,]+)").unwrap());
                if let Some(caps) = from_re.captures(&clean_input) {
                    let raw = caps[1].trim();
                    // Strip trailing keywords so "from UK for 2 hours" yields "UK"
                    let origin_str = raw
                        .find(" for ")
                        .or_else(|| raw.find(" using "))
                        .or_else(|| raw.find(" in "))
                        .or_else(|| raw.find(" with "))
                        .or_else(|| raw.find(" for\n")) // handle potential newlines from cleaned input
                        .map(|i| &raw[..i])
                        .unwrap_or(raw)
                        .trim();
                    if !origin_str.is_empty() {
                        prompt.origin = Some(parse_location(origin_str));
                    }
                }

                // Final fallback: bare region/city name with no directional keyword.
                // e.g. "Washington State", "Alaska", "London" — treat as destination.
                if prompt.origin.is_none() && prompt.destination.is_none() {
                    let candidate = normalize_for_region_match(&clean_input);
                    if let Some(region) = try_as_region(&candidate) {
                        prompt.destination = Some(region);
                    }
                }
            }
        }

        // 4. Parse Aircraft
        // Strip known modifier phrases so "using Boeing long haul" captures "boeing" not "boeing long haul".
        // We strip multi-word modifiers only — single words like "long"/"short" are left alone to avoid
        // false-positive on aircraft names that contain them (e.g. "Rutan Long-EZ").
        let acf_input = clean_input
            .replace("short flight", "")
            .replace("short hop", "")
            .replace("long haul", "")
            .replace("long flight", "")
            .replace("medium flight", "")
            .replace("bush flight", "")
            .replace("backcountry flight", "")
            .replace("ignore guardrails", "");

        static ACF_RE: OnceLock<Regex> = OnceLock::new();
        let acf_re = ACF_RE.get_or_init(|| {
            Regex::new(r"\b(?:using|in|with)\b(?:\s+a|\s+an)?\s+(.+?)(\s+\bfor\b|\s+\bfrom\b|\s+\blanding\b|\s+\barriving\b|\s+\bdeparting\b|$)")
                .unwrap()
        });

        if let Some(caps) = acf_re.captures(&acf_input) {
            let mut acf_str = caps[1].trim().to_string();

            // Normalize common conversational variants into standardized tags
            // matching the BitNet classifier's taxonomy.
            let acf_lower = acf_str.to_lowercase();
            if acf_lower.contains("airliner")
                || acf_lower.contains("commercial")
                || acf_lower.contains("passenger")
            {
                acf_str = "Airliner".to_string();
            } else if acf_lower.contains("biz jet")
                || acf_lower.contains("bizjet")
                || acf_lower.contains("business")
                || acf_lower.contains("corporate")
                || acf_lower.contains("private jet")
            {
                acf_str = "Business Jet".to_string();
            } else if acf_lower == "ga"
                || acf_lower.contains("general aviation")
                || acf_lower.contains("small plane")
                || acf_lower.contains("light aircraft")
                || acf_lower.contains("propeller")
                || acf_lower.contains("piston")
                || acf_lower.contains("civilian")
            {
                acf_str = "General Aviation".to_string();
            } else if acf_lower.contains("cargo")
                || acf_lower.contains("freight")
                || acf_lower.contains("transport")
            {
                acf_str = "Cargo".to_string();
            } else if acf_lower.contains("heli")
                || acf_lower.contains("chopper")
                || acf_lower.contains("rotor")
            {
                acf_str = "Helicopter".to_string();
            } else if acf_lower.contains("military")
                || acf_lower.contains("fighter")
                || acf_lower.contains("combat")
                || acf_lower.contains("bomber")
            {
                acf_str = "Military".to_string();
            } else if acf_lower.contains("glider") || acf_lower.contains("sailplane") {
                acf_str = "Glider".to_string();
            }

            if !acf_str.is_empty() {
                prompt.aircraft = Some(AircraftConstraint::Tag(acf_str));
            }
        }

        // 5. Parse Explicit Duration (Overrides keyword if present)
        static TIME_RE: OnceLock<Regex> = OnceLock::new();
        let time_re = TIME_RE
            .get_or_init(|| Regex::new(r"(?:for\s+)?(\d+)\s*(hour|hr|minute|min|m)s?").unwrap());

        if let Some(caps) = time_re.captures(&clean_input) {
            if let (Ok(val), Some(unit)) = (caps[1].parse::<u32>(), caps.get(2)) {
                let minutes = match unit.as_str() {
                    "hour" | "hr" => val * 60,
                    _ => val,
                };
                prompt.duration_minutes = Some(minutes);
            }
        }

        prompt
    }
}

fn parse_location(s: &str) -> LocationConstraint {
    let s = s.strip_prefix("the ").unwrap_or(s).trim();
    if s == "here" || s == "current location" {
        LocationConstraint::Region("Here".to_string())
    } else if s == "anywhere" || s == "any" || s == "random" {
        LocationConstraint::Any
    } else if let Some(region) = try_as_region(s) {
        // Check region/city aliases BEFORE the ICAO heuristic.
        // This prevents city names like "Lamu" or "Lima" from being
        // misidentified as ICAO codes.
        region
    } else if (s.len() >= 4 && s.len() <= 7) && s.chars().all(|c| c.is_alphanumeric()) {
        // Real ICAO codes are 4 characters (EGLL, KJFK, RJAA, …).
        // Allow up to 7 to cover IATA/domestic variants, but 3-char codes like
        // "F70" are FAA facility IDs and are better handled as AirportName so
        // that flight_gen's name-scoring can match them by ICAO id.
        LocationConstraint::ICAO(s.to_uppercase())
    } else {
        LocationConstraint::AirportName(s.to_string())
    }
}

/// Normalizes location string for alias match: lowercase, collapse whitespace, strip "the ", replace comma with space.
fn normalize_for_region_match(s: &str) -> String {
    let s = s.strip_prefix("the ").unwrap_or(s).trim();
    s.replace(',', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Attempts to recognize a string as a geographic region or city.
fn try_as_region(s: &str) -> Option<LocationConstraint> {
    let s = s.strip_prefix("the ").unwrap_or(s).trim();

    // 1. Check explicit alias table FIRST (includes NearCity entries for cities).
    //    This takes priority over RegionIndex to avoid stale region matches
    //    (e.g. "London" matching UK:London region instead of NearCity).
    let key = normalize_for_region_match(s);
    // Helper to build NearCity compactly
    let nc = |name: &str, lat: f64, lon: f64| -> Option<LocationConstraint> {
        Some(LocationConstraint::NearCity {
            name: name.to_string(),
            lat,
            lon,
        })
    };

    match key.as_str() {
        // British Isles & UK — regions
        "british isles" => Some(LocationConstraint::Region("BI".to_string())),
        "ireland" | "eire" => Some(LocationConstraint::Region("IE".to_string())),
        "uk" | "united kingdom" => Some(LocationConstraint::Region("UK".to_string())),
        "gb" | "great britain" => Some(LocationConstraint::Region("GB".to_string())),
        "england" => Some(LocationConstraint::Region("UK:England".to_string())),
        "scotland" => Some(LocationConstraint::Region("UK:Scotland".to_string())),
        "wales" => Some(LocationConstraint::Region("UK:Wales".to_string())),
        // Europe — countries
        "ukraine" | "ukr" => Some(LocationConstraint::Region("UA".to_string())),
        "italy" => Some(LocationConstraint::Region("IT".to_string())),
        "france" => Some(LocationConstraint::Region("FR".to_string())),
        "germany" => Some(LocationConstraint::Region("DE".to_string())),
        "spain" => Some(LocationConstraint::Region("ES".to_string())),
        // North America — regions
        "usa" | "us" | "united states" => Some(LocationConstraint::Region("US".to_string())),
        "canada" => Some(LocationConstraint::Region("CA".to_string())),
        "mexico" => Some(LocationConstraint::Region("MX".to_string())),
        "socal" | "southern california" => Some(LocationConstraint::Region("US:SoCal".to_string())),
        "riverside county" | "riverside" => {
            Some(LocationConstraint::Region("US:SoCal".to_string()))
        }
        "norcal" | "northern california" => {
            Some(LocationConstraint::Region("US:NorCal".to_string()))
        }
        "oregon" => Some(LocationConstraint::Region("US:OR".to_string())),
        "pnw" | "pacific northwest" => Some(LocationConstraint::Region("US:OR".to_string())),
        "alaska" => Some(LocationConstraint::Region("US:AK".to_string())),
        "hawaii" => Some(LocationConstraint::Region("US:HI".to_string())),
        // Geographic features
        "alps" => Some(LocationConstraint::Region("Alps".to_string())),
        "rockies" | "rocky mountains" => Some(LocationConstraint::Region("Rockies".to_string())),
        "caribbean" => Some(LocationConstraint::Region("Caribbean".to_string())),
        // Pacific Islands sub-regions
        "micronesia" => Some(LocationConstraint::Region(
            "PacIsles:Micronesia".to_string(),
        )),
        "melanesia" => Some(LocationConstraint::Region("PacIsles:Melanesia".to_string())),
        "polynesia" | "french polynesia" | "south pacific" => {
            Some(LocationConstraint::Region("PacIsles:Polynesia".to_string()))
        }
        // Africa — countries
        "south africa" => Some(LocationConstraint::Region("ZA".to_string())),
        "kenya" => Some(LocationConstraint::Region("KE".to_string())),
        "egypt" => Some(LocationConstraint::Region("EG".to_string())),
        "tanzania" => Some(LocationConstraint::Region("TZ".to_string())),
        "ethiopia" => Some(LocationConstraint::Region("ET".to_string())),
        "nigeria" => Some(LocationConstraint::Region("NG".to_string())),
        "morocco" => Some(LocationConstraint::Region("MA".to_string())),
        // South America — countries
        "brazil" => Some(LocationConstraint::Region("BR".to_string())),
        "argentina" => Some(LocationConstraint::Region("AR".to_string())),
        "colombia" => Some(LocationConstraint::Region("CO".to_string())),
        "peru" => Some(LocationConstraint::Region("PE".to_string())),
        "chile" => Some(LocationConstraint::Region("CL".to_string())),
        // Alternative / historical names that RegionIndex can't catch
        "burma" => Some(LocationConstraint::Region("MM".to_string())),
        "persia" => Some(LocationConstraint::Region("IR".to_string())),
        // Short forms / alternate spellings
        "czechia" | "czech republic" | "czech" => {
            Some(LocationConstraint::Region("CZ".to_string()))
        }
        "lao" | "lao pdr" => Some(LocationConstraint::Region("LA".to_string())),

        // ===================== CITIES → NearCity =====================
        // North America — US cities (many are 5-7 chars and would be mis-parsed as ICAO otherwise)
        "new york" | "new york city" | "nyc" => nc("New York", 40.7128, -74.0060),
        "los angeles" | "la" => nc("Los Angeles", 34.0522, -118.2437),
        "chicago" => nc("Chicago", 41.8781, -87.6298),
        "miami" => nc("Miami", 25.7617, -80.1918),
        "seattle" => nc("Seattle", 47.6062, -122.3321),
        "denver" => nc("Denver", 39.7392, -104.9903),
        "atlanta" => nc("Atlanta", 33.7490, -84.3880),
        "dallas" | "dallas fort worth" => nc("Dallas", 32.7767, -96.7970),
        "san francisco" | "sf" => nc("San Francisco", 37.7749, -122.4194),
        "boston" => nc("Boston", 42.3601, -71.0589),
        "toronto" => nc("Toronto", 43.7000, -79.4163),
        "vancouver" => nc("Vancouver", 49.2827, -123.1207),
        "montreal" => nc("Montreal", 45.5017, -73.5673),
        // Europe — cities
        "london" | "london uk" => nc("London", 51.5074, -0.1278),
        "rome" | "rome italy" | "rome, italy" => nc("Rome", 41.9028, 12.4964),
        "paris" | "paris france" | "paris, france" => nc("Paris", 48.8566, 2.3522),
        "amsterdam" => nc("Amsterdam", 52.3676, 4.9041),
        "zurich" | "zürich" => nc("Zurich", 47.3769, 8.5417),
        "geneva" => nc("Geneva", 46.2044, 6.1432),
        "vienna" | "wien" => nc("Vienna", 48.2082, 16.3738),
        "brussels" => nc("Brussels", 50.8503, 4.3517),
        "istanbul" => nc("Istanbul", 41.0082, 28.9784),
        "lisbon" => nc("Lisbon", 38.7223, -9.1393),
        "porto" => nc("Porto", 41.1579, -8.6291),
        "athens" => nc("Athens", 37.9838, 23.7275),
        "oslo" => nc("Oslo", 59.9139, 10.7522),
        "stockholm" => nc("Stockholm", 59.3293, 18.0686),
        "copenhagen" => nc("Copenhagen", 55.6761, 12.5683),
        "helsinki" => nc("Helsinki", 60.1699, 24.9384),
        "reykjavik" => nc("Reykjavik", 64.1466, -21.9426),
        "warsaw" => nc("Warsaw", 52.2297, 21.0122),
        "krakow" => nc("Krakow", 50.0647, 19.9450),
        "prague" => nc("Prague", 50.0755, 14.4378),
        "berlin" => nc("Berlin", 52.5200, 13.4050),
        "hamburg" => nc("Hamburg", 53.5500, 9.9937),
        "munich" | "münchen" => nc("Munich", 48.1351, 11.5820),
        "madrid" => nc("Madrid", 40.4168, -3.7038),
        "barcelona" => nc("Barcelona", 41.3874, 2.1686),
        // Africa — cities
        "nairobi" => nc("Nairobi", -1.2921, 36.8219),
        "mombasa" => nc("Mombasa", -4.0435, 39.6682),
        "lamu" => nc("Lamu", -2.2717, 40.9020),
        "malindi" => nc("Malindi", -3.2238, 40.1169),
        "johannesburg" | "joburg" => nc("Johannesburg", -26.2041, 28.0473),
        "cape town" => nc("Cape Town", -33.9249, 18.4241),
        "durban" => nc("Durban", -29.8587, 31.0218),
        "cairo" => nc("Cairo", 30.0444, 31.2357),
        "addis ababa" | "addis" => nc("Addis Ababa", 9.0192, 38.7525),
        "lagos" => nc("Lagos", 6.5244, 3.3792),
        "abuja" => nc("Abuja", 9.0579, 7.4951),
        "dar es salaam" | "dar" => nc("Dar es Salaam", -6.7924, 39.2083),
        "zanzibar" => nc("Zanzibar", -6.1659, 39.2026),
        "kilimanjaro" => nc("Kilimanjaro", -3.0674, 37.3556),
        "marrakech" => nc("Marrakech", 31.6295, -7.9811),
        "casablanca" => nc("Casablanca", 33.5731, -7.5898),
        // Asia — cities
        "tokyo" => nc("Tokyo", 35.6762, 139.6503),
        "osaka" => nc("Osaka", 34.6937, 135.5023),
        "bangkok" => nc("Bangkok", 13.7563, 100.5018),
        "singapore" => Some(LocationConstraint::Region("SG".to_string())),
        "hong kong" => Some(LocationConstraint::Region("HK".to_string())),
        "taipei" => nc("Taipei", 25.0330, 121.5654),
        "seoul" => nc("Seoul", 37.5665, 126.9780),
        "beijing" => nc("Beijing", 39.9042, 116.4074),
        "shanghai" => nc("Shanghai", 31.2304, 121.4737),
        "guangzhou" => nc("Guangzhou", 23.1291, 113.2644),
        "mumbai" => nc("Mumbai", 19.0760, 72.8777),
        "delhi" => nc("Delhi", 28.7041, 77.1025),
        "bangalore" => nc("Bangalore", 12.9716, 77.5946),
        "chennai" => nc("Chennai", 13.0827, 80.2707),
        "kolkata" => nc("Kolkata", 22.5726, 88.3639),
        "dubai" => nc("Dubai", 25.2048, 55.2708),
        "abu dhabi" => nc("Abu Dhabi", 24.4539, 54.3773),
        "doha" => nc("Doha", 25.2854, 51.5310),
        "kuwait" => Some(LocationConstraint::Region("KW".to_string())),
        "kuwait city" => nc("Kuwait City", 29.3759, 47.9774),
        "riyadh" => nc("Riyadh", 24.7136, 46.6753),
        "jeddah" => nc("Jeddah", 21.4858, 39.1925),
        "muscat" => nc("Muscat", 23.5880, 58.3829),
        "amman" => nc("Amman", 31.9454, 35.9284),
        "beirut" => nc("Beirut", 33.8938, 35.5018),
        "tel aviv" => nc("Tel Aviv", 32.0853, 34.7818),
        "jerusalem" => nc("Jerusalem", 31.7683, 35.2137),
        "kuala lumpur" => nc("Kuala Lumpur", 3.1390, 101.6869),
        "manila" => nc("Manila", 14.5995, 120.9842),
        "hanoi" => nc("Hanoi", 21.0278, 105.8342),
        "ho chi minh" | "saigon" => nc("Ho Chi Minh City", 10.8231, 106.6297),
        "bali" => nc("Bali", -8.3405, 115.0920),
        "jakarta" => nc("Jakarta", -6.2088, 106.8456),
        // South America — cities
        "rio" | "rio de janeiro" => nc("Rio de Janeiro", -22.9068, -43.1729),
        "sao paulo" => nc("São Paulo", -23.5505, -46.6333),
        "buenos aires" => nc("Buenos Aires", -34.6037, -58.3816),
        "bogota" => nc("Bogota", 4.7110, -74.0721),
        "lima" => nc("Lima", -12.0464, -77.0428),
        "santiago" => nc("Santiago", -33.4489, -70.6693),
        // Oceania — cities
        "sydney" => nc("Sydney", -33.8688, 151.2093),
        "melbourne" => nc("Melbourne", -37.8136, 144.9631),
        "brisbane" => nc("Brisbane", -27.4698, 153.0251),
        "perth" => nc("Perth", -31.9505, 115.8605),
        "auckland" => nc("Auckland", -36.8485, 174.7633),
        "wellington" => nc("Wellington", -41.2865, 174.7762),
        "queenstown" => nc("Queenstown", -45.0312, 168.6626),
        "christchurch" => nc("Christchurch", -43.4899, 172.5369),
        "hobart" => nc("Hobart", -42.8821, 147.3272),
        "darwin" => nc("Darwin", -12.4634, 130.8456),
        "cairns" => nc("Cairns", -16.9186, 145.7781),
        "fiji" => Some(LocationConstraint::Region("FJ".to_string())),
        "nadi" => nc("Nadi", -17.7559, 177.4515),
        // US — more major cities
        "phoenix" => nc("Phoenix", 33.4484, -112.0740),
        "houston" => nc("Houston", 29.7604, -95.3698),
        "las vegas" => nc("Las Vegas", 36.1699, -115.1398),
        // "washington" alone falls through to RegionIndex → US:WA (Washington State).
        // Only explicit "washington dc" / "dc" phrases map to the capital.
        "washington dc" | "dc" => nc("Washington DC", 38.9072, -77.0369),
        "washington state" | "state of washington" | "wa state" => {
            Some(LocationConstraint::Region("US:WA".to_string()))
        }
        "philadelphia" | "philly" => nc("Philadelphia", 39.9526, -75.1652),
        "minneapolis" => nc("Minneapolis", 44.9778, -93.2650),
        "detroit" => nc("Detroit", 42.3314, -83.0458),
        "charlotte" => nc("Charlotte", 35.2271, -80.8431),
        "portland" => nc("Portland", 45.5051, -122.6750),
        "salt lake city" | "slc" => nc("Salt Lake City", 40.7608, -111.8910),
        "kansas city" => nc("Kansas City", 39.0997, -94.5786),
        "new orleans" => nc("New Orleans", 29.9511, -90.0715),
        "orlando" => nc("Orlando", 28.5383, -81.3792),
        "tampa" => nc("Tampa", 27.9506, -82.4572),
        "san diego" => nc("San Diego", 32.7157, -117.1611),
        "anchorage" => nc("Anchorage", 61.2181, -149.9003),
        "honolulu" => nc("Honolulu", 21.3069, -157.8583),
        // Europe — more cities
        "frankfurt" => nc("Frankfurt", 50.1109, 8.6821),
        "milan" | "milano" => nc("Milan", 45.4654, 9.1859),
        "edinburgh" => nc("Edinburgh", 55.9533, -3.1883),
        "manchester" => nc("Manchester", 53.4808, -2.2426),
        "birmingham" => nc("Birmingham", 52.4862, -1.8904),
        "lyon" => nc("Lyon", 45.7640, 4.8357),
        "marseille" => nc("Marseille", 43.2965, 5.3698),
        "nice" => nc("Nice", 43.7102, 7.2620),
        "naples" | "napoli" => nc("Naples", 40.8518, 14.2681),
        "florence" | "firenze" => nc("Florence", 43.7696, 11.2558),
        "venice" | "venezia" => nc("Venice", 45.4408, 12.3155),
        "seville" | "sevilla" => nc("Seville", 37.3891, -5.9845),
        "valencia" => nc("Valencia", 39.4699, -0.3763),
        "palma" | "mallorca" => nc("Palma", 39.5696, 2.6502),
        "tenerife" => nc("Tenerife", 28.2916, -16.6291),
        "budapest" => nc("Budapest", 47.4979, 19.0402),
        "bucharest" => nc("Bucharest", 44.4268, 26.1025),
        "sofia" => nc("Sofia", 42.6977, 23.3219),
        "belgrade" => nc("Belgrade", 44.7866, 20.4489),
        "zagreb" => nc("Zagreb", 45.8150, 15.9819),
        "dubrovnik" => nc("Dubrovnik", 42.6507, 18.0944),
        "split" => nc("Split", 43.5081, 16.4402),
        "riga" => nc("Riga", 56.9460, 24.1059),
        "tallinn" => nc("Tallinn", 59.4370, 24.7536),
        "vilnius" => nc("Vilnius", 54.6872, 25.2797),
        "bratislava" => nc("Bratislava", 48.1486, 17.1077),
        "luxembourg" => Some(LocationConstraint::Region("LU".to_string())),
        // Middle East / Africa — more cities
        "tehran" => nc("Tehran", 35.6892, 51.3890),
        "baghdad" => nc("Baghdad", 33.3152, 44.3661),
        "karachi" => nc("Karachi", 24.8607, 67.0011),
        "islamabad" => nc("Islamabad", 33.7294, 73.0931),
        "lahore" => nc("Lahore", 31.5204, 74.3587),
        "dhaka" => nc("Dhaka", 23.8103, 90.4125),
        "colombo" => nc("Colombo", 6.9271, 79.8612),
        "kathmandu" => nc("Kathmandu", 27.7172, 85.3240),
        "yangon" | "rangoon" => nc("Yangon", 16.8661, 96.1951),
        "accra" => nc("Accra", 5.6037, -0.1870),
        "dakar" => nc("Dakar", 14.7167, -17.4677),
        "tunis" => nc("Tunis", 36.8065, 10.1815),
        "tripoli" => nc("Tripoli", 32.9034, 13.1807),
        "khartoum" => nc("Khartoum", 15.5007, 32.5599),
        "kampala" => nc("Kampala", 0.3476, 32.5825),
        "kyiv" | "kiev" => nc("Kyiv", 50.4501, 30.5234),
        "lviv" | "lwow" => nc("Lviv", 49.8397, 24.0297),
        "odessa" | "odesa" => nc("Odessa", 46.4825, 30.7233),
        "kharkiv" | "kharkov" => nc("Kharkiv", 49.9935, 36.2304),
        "dnipro" | "dnipropetrovsk" => nc("Dnipro", 48.4647, 35.0462),
        "kigali" => nc("Kigali", -1.9441, 30.0619),
        "lusaka" => nc("Lusaka", -15.4167, 28.2833),
        "harare" => nc("Harare", -17.8252, 31.0335),
        "antananarivo" => nc("Antananarivo", -18.9137, 47.5361),
        // Latin America — more cities
        "mexico city" | "cdmx" => nc("Mexico City", 19.4326, -99.1332),
        "guadalajara" => nc("Guadalajara", 20.6597, -103.3496),
        "cancun" => nc("Cancun", 21.1619, -86.8515),
        "havana" | "la habana" => nc("Havana", 23.1136, -82.3666),
        "san juan" => nc("San Juan", 18.4655, -66.1057),
        "nassau" => nc("Nassau", 25.0480, -77.3554),
        "panama" => Some(LocationConstraint::Region("PA".to_string())),
        "panama city" => nc("Panama City", 8.9936, -79.5197),
        "san jose" => nc("San Jose", 9.9281, -84.0907),
        "quito" => nc("Quito", -0.1807, -78.4678),
        "caracas" => nc("Caracas", 10.4806, -66.9036),
        "montevideo" => nc("Montevideo", -34.9011, -56.1645),
        "asuncion" | "asunción" => nc("Asunción", -25.2867, -57.6470),
        "la paz" => nc("La Paz", -16.5000, -68.1500),
        _ => {
            // 2. Fallback: check RegionIndex for geographic regions not in the explicit table
            let index = crate::geo::RegionIndex::new();
            if let Some(region) = index.search(s) {
                return Some(LocationConstraint::Region(region.id.to_string()));
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_from() {
        let p = FlightPrompt::parse("London to Paris");
        // London maps to NearCity; Paris also maps to NearCity
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            other => panic!("Paris should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_simple() {
        let p = FlightPrompt::parse("Flight from London to Paris");
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            other => panic!("Paris should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_full() {
        let p = FlightPrompt::parse(
            "Flight from EGLL to KJFK using a Boeing 747 for 7 hours ignore guardrails",
        );

        match p.origin {
            Some(LocationConstraint::ICAO(code)) => assert_eq!(code, "EGLL"),
            _ => panic!("Bad origin"),
        }
        match p.destination {
            Some(LocationConstraint::ICAO(code)) => assert_eq!(code, "KJFK"),
            _ => panic!("Bad dest"),
        }
        match p.aircraft {
            Some(AircraftConstraint::Tag(t)) => assert!(t.contains("boeing 747")),
            _ => panic!("Bad aircraft"),
        }
        assert_eq!(p.duration_minutes, Some(420));
        assert!(p.ignore_guardrails);
    }

    #[test]
    fn test_parse_duration() {
        let p = FlightPrompt::parse("Just fly for 45 mins");
        assert_eq!(p.duration_minutes, Some(45));
    }

    #[test]
    fn test_parse_country_as_region() {
        let p = FlightPrompt::parse("Flight from France to Germany");
        assert_eq!(p.origin, Some(LocationConstraint::Region("FR".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("DE".to_string()))
        );
    }

    #[test]
    fn test_parse_us_nickname_as_region() {
        let p = FlightPrompt::parse("Flight from Socal to Norcal");
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("US:SoCal".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("US:NorCal".to_string()))
        );
    }

    #[test]
    fn test_parse_abbreviation_as_region() {
        let p = FlightPrompt::parse("Flight from UK to USA");
        assert_eq!(p.origin, Some(LocationConstraint::Region("UK".to_string())));
        // "usa" is 3 chars, not 4, so not ICAO
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("US".to_string()))
        );
    }

    #[test]
    fn test_parse_from_uk_only() {
        // "from X" without "to Y" must constrain origin so flight is from UK, not e.g. Algeria/France
        let p = FlightPrompt::parse("2 hour flight from UK");
        assert_eq!(p.origin, Some(LocationConstraint::Region("UK".to_string())));
        assert_eq!(p.destination, None);
        let p2 = FlightPrompt::parse("flight from UK");
        assert_eq!(
            p2.origin,
            Some(LocationConstraint::Region("UK".to_string()))
        );
        assert_eq!(p2.destination, None);
    }

    #[test]
    fn test_parse_article_stripped() {
        let p = FlightPrompt::parse("Flight from the British Isles to the Caribbean");
        assert_eq!(p.origin, Some(LocationConstraint::Region("BI".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("Caribbean".to_string()))
        );
    }

    #[test]
    fn test_parse_city_maps_to_nearcity() {
        // London and Paris both map to NearCity now
        let p = FlightPrompt::parse("Flight from London to Paris");
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            other => panic!("Paris should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_london_uk_to_region() {
        let p = FlightPrompt::parse("Flight from London UK to Germany");
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London UK should be NearCity, got {:?}", other),
        }
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("DE".to_string()))
        );
    }

    #[test]
    fn test_parse_london_to_italy() {
        let p = FlightPrompt::parse("Flight from London to Italy");
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London should be NearCity, got {:?}", other),
        }
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("IT".to_string()))
        );
    }

    #[test]
    fn test_parse_rome_italy_as_nearcity() {
        // "Rome Italy" now maps to NearCity(Rome) — coordinates (41.9°N, 12.5°E) disambiguate from Rome GA
        let p = FlightPrompt::parse("Flight from EGMC to Rome Italy");
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGMC".to_string())));
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Rome"),
            other => panic!("Rome Italy should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_rome_comma_italy_as_nearcity() {
        let p = FlightPrompt::parse("Flight from London to Rome, Italy");
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Rome"),
            other => panic!("Rome, Italy should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_paris_france_as_nearcity() {
        let p = FlightPrompt::parse("Flight from EGLL to Paris France");
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            other => panic!("Paris France should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_icao_still_icao() {
        let p = FlightPrompt::parse("Flight from EGLL to LFPG");
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::ICAO("LFPG".to_string()))
        );
    }
    #[test]
    fn test_parse_f70_to_alaska() {
        let p = FlightPrompt::parse("F70 to Alaska");
        // F70 is 3 chars, so parsed as Name, not ICAO (valid behavior as flight_gen handles ID match in name search)
        match p.origin {
            Some(LocationConstraint::AirportName(ref s)) if s == "f70" => {}
            _ => panic!("Origin should be AirportName(f70), got {:?}", p.origin),
        }
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("US:AK".to_string()))
        );
    }

    #[test]
    fn test_parse_nairobi_to_lamu() {
        let p = FlightPrompt::parse("Nairobi to Lamu");
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Nairobi"),
            other => panic!("Nairobi should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Lamu"),
            other => panic!("Lamu should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_nairobi_to_mombasa() {
        let p = FlightPrompt::parse("Nairobi to Mombasa");
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Nairobi"),
            other => panic!("Nairobi should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Mombasa"),
            other => panic!("Mombasa should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_tokyo_to_bangkok() {
        let p = FlightPrompt::parse("Tokyo to Bangkok");
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Tokyo"),
            other => panic!("Tokyo should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Bangkok"),
            other => panic!("Bangkok should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_icao_still_works_after_reorder() {
        // Ensure the parse_location reorder didn't break real ICAO codes
        let p = FlightPrompt::parse("Flight from EGLL to KJFK");
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::ICAO("KJFK".to_string()))
        );
    }

    /// "washington" alone must resolve to Washington State (US:WA), NOT Washington D.C.
    /// Bare "washington" is ambiguous; the Pacific Northwest state is the more useful
    /// flight-gen target. Use "washington dc" or "dc" to get the capital.
    #[test]
    fn test_parse_washington_resolves_to_wa_state() {
        let p = FlightPrompt::parse("F70 to Washington");
        // F70 is 3 chars → AirportName
        assert!(
            matches!(&p.origin, Some(LocationConstraint::AirportName(s)) if s == "f70"),
            "Origin should be AirportName(f70), got {:?}",
            p.origin
        );
        // "Washington" must be Washington State, not D.C.
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("US:WA".to_string())),
            "Bare 'washington' should resolve to US:WA, got {:?}",
            p.destination
        );
    }

    /// "washington state" must resolve to Region(US:WA).
    #[test]
    fn test_parse_washington_state_explicit() {
        let p = FlightPrompt::parse("Washington State");
        assert_eq!(
            p.destination.or(p.origin.clone()),
            Some(LocationConstraint::Region("US:WA".to_string())),
            "'Washington State' should be Region(US:WA)"
        );
    }

    /// "washington dc" and "dc" must still resolve to the capital NearCity.
    #[test]
    fn test_parse_washington_dc_still_works() {
        let p = FlightPrompt::parse("fly to washington dc");
        assert!(
            matches!(&p.destination, Some(LocationConstraint::NearCity { name, .. }) if name == "Washington DC"),
            "washington dc should still be NearCity(Washington DC), got {:?}",
            p.destination
        );
        let p2 = FlightPrompt::parse("fly to dc");
        assert!(
            matches!(&p2.destination, Some(LocationConstraint::NearCity { name, .. }) if name == "Washington DC"),
            "dc should still be NearCity(Washington DC), got {:?}",
            p2.destination
        );
    }

    #[test]
    fn test_parse_civilian_airliner_with_landing() {
        let p = FlightPrompt::parse(
            "Flight from Scotland to Italy using civilian airliner landing at civilian airport",
        );
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("UK:Scotland".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("IT".to_string()))
        );
        match p.aircraft {
            Some(AircraftConstraint::Tag(t)) => {
                assert_eq!(t, "Airliner", "Should normalize to Airliner")
            }
            _ => panic!(
                "Aircraft should be mapped to Airliner, got {:?}",
                p.aircraft
            ),
        }
    }
}
