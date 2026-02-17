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
    Soft, // Grass, Dirt
    Hard, // Paved, Tarmac
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
        if clean_input.contains("short") || clean_input.contains("hop") {
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
        {
            prompt.keywords.surface = Some(SurfaceKeyword::Soft);
        } else if clean_input.contains("paved")
            || clean_input.contains("tarmac")
            || clean_input.contains("concrete")
        {
            prompt.keywords.surface = Some(SurfaceKeyword::Hard);
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
                r"(?:flight\s+from\s+|from\s+|^flight\s+)?(.+?)\s+to\s+(.+?)(\s+using|\s+in|\s+with|\s+for|$)",
            )
            .unwrap()
        });

        if let Some(caps) = loc_re.captures(&clean_input) {
            let origin_str = caps[1].trim();
            let dest_str = caps[2].trim();

            prompt.origin = Some(parse_location(origin_str));
            prompt.destination = Some(parse_location(dest_str));
        } else {
            // Fallback: Check for destination-only prompt "to X" or "flight to X"
            static TO_RE: OnceLock<Regex> = OnceLock::new();
            let to_re = TO_RE.get_or_init(|| {
                Regex::new(r"(?:^flight\s+to\s+|^to\s+)(.+?)(\s+using|\s+in|\s+with|\s+for|$)")
                    .unwrap()
            });
            if let Some(caps) = to_re.captures(&clean_input) {
                let dest_str = caps[1].trim();
                prompt.destination = Some(parse_location(dest_str));
            } else {
                // Fallback: "from X" without "to Y" (e.g. "2 hour flight from UK") — constrain origin only
                static FROM_RE: OnceLock<Regex> = OnceLock::new();
                let from_re =
                    FROM_RE.get_or_init(|| Regex::new(r"from\s+([a-zA-Z0-9\s,]+)").unwrap());
                if let Some(caps) = from_re.captures(&clean_input) {
                    let raw = caps[1].trim();
                    // Strip trailing keywords so "from UK for 2 hours" yields "UK"
                    let origin_str = raw
                        .find(" for ")
                        .or_else(|| raw.find(" using "))
                        .or_else(|| raw.find(" in "))
                        .or_else(|| raw.find(" with "))
                        .map(|i| &raw[..i])
                        .unwrap_or(raw)
                        .trim();
                    if !origin_str.is_empty() {
                        prompt.origin = Some(parse_location(origin_str));
                    }
                }
            }
        }

        // 4. Parse Aircraft
        static ACF_RE: OnceLock<Regex> = OnceLock::new();
        let acf_re = ACF_RE.get_or_init(|| {
            Regex::new(r"(?:using|in|with)(?:\s+a|\s+an)?\s+(.+?)(\s+for|\s+from|$)").unwrap()
        });

        if let Some(caps) = acf_re.captures(&clean_input) {
            let acf_str = caps[1].trim();
            if !acf_str.is_empty() {
                prompt.aircraft = Some(AircraftConstraint::Tag(acf_str.to_string()));
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
        // Check region/city aliases BEFORE the 4-letter ICAO heuristic.
        // This prevents city names like "Lamu" or "Lima" from being
        // misidentified as ICAO codes.
        region
    } else if s.len() == 4 && s.chars().all(|c| c.is_alphabetic()) {
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

/// Attempts to recognize a string as a geographic region.
fn try_as_region(s: &str) -> Option<LocationConstraint> {
    let s = s.strip_prefix("the ").unwrap_or(s).trim();
    let index = crate::geo::RegionIndex::new();

    if let Some(region) = index.search(s) {
        return Some(LocationConstraint::Region(region.id.to_string()));
    }

    // Fallback aliases (countries/regions and major cities → region for reliable flight gen)
    // "City Country" patterns avoid ambiguous name match (e.g. Rome Georgia vs Rome Italy)
    let key = normalize_for_region_match(s);
    match key.as_str() {
        // British Isles & UK
        "british isles" => Some(LocationConstraint::Region("BI".to_string())),
        "ireland" | "eire" => Some(LocationConstraint::Region("IE".to_string())),
        "uk" | "united kingdom" => Some(LocationConstraint::Region("UK".to_string())),
        "gb" | "great britain" => Some(LocationConstraint::Region("GB".to_string())),
        "london" | "london uk" => Some(LocationConstraint::Region("UK:London".to_string())),
        "england" | "scotland" | "wales" => Some(LocationConstraint::Region("UK".to_string())),
        // Europe — countries
        "italy" => Some(LocationConstraint::Region("IT".to_string())),
        "rome italy" | "rome, italy" => Some(LocationConstraint::Region("IT".to_string())),
        "france" => Some(LocationConstraint::Region("FR".to_string())),
        "paris france" | "paris, france" => Some(LocationConstraint::Region("FR".to_string())),
        "germany" => Some(LocationConstraint::Region("DE".to_string())),
        "spain" => Some(LocationConstraint::Region("ES".to_string())),
        // Europe — cities
        "amsterdam" => Some(LocationConstraint::Region("NL".to_string())),
        "zurich" | "geneva" => Some(LocationConstraint::Region("CH".to_string())),
        "vienna" | "wien" => Some(LocationConstraint::Region("AT".to_string())),
        "brussels" => Some(LocationConstraint::Region("BE".to_string())),
        "istanbul" => Some(LocationConstraint::Region("TR".to_string())),
        "lisbon" | "porto" => Some(LocationConstraint::Region("PT".to_string())),
        "athens" => Some(LocationConstraint::Region("GR".to_string())),
        "oslo" => Some(LocationConstraint::Region("NO".to_string())),
        "stockholm" => Some(LocationConstraint::Region("SE".to_string())),
        "copenhagen" => Some(LocationConstraint::Region("DK".to_string())),
        "helsinki" => Some(LocationConstraint::Region("FI".to_string())),
        "reykjavik" => Some(LocationConstraint::Region("IS".to_string())),
        "warsaw" | "krakow" => Some(LocationConstraint::Region("PL".to_string())),
        "prague" => Some(LocationConstraint::Region("CZ".to_string())),
        // North America
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
        // Geographic features
        "alps" => Some(LocationConstraint::Region("Alps".to_string())),
        "rockies" => Some(LocationConstraint::Region("Rockies".to_string())),
        "caribbean" => Some(LocationConstraint::Region("Caribbean".to_string())),
        // Africa — countries
        "south africa" => Some(LocationConstraint::Region("ZA".to_string())),
        "kenya" => Some(LocationConstraint::Region("KE".to_string())),
        "egypt" => Some(LocationConstraint::Region("EG".to_string())),
        "tanzania" => Some(LocationConstraint::Region("TZ".to_string())),
        "ethiopia" => Some(LocationConstraint::Region("ET".to_string())),
        "nigeria" => Some(LocationConstraint::Region("NG".to_string())),
        "morocco" => Some(LocationConstraint::Region("MA".to_string())),
        // Africa — cities
        "nairobi" => Some(LocationConstraint::Region("KE".to_string())),
        "mombasa" => Some(LocationConstraint::Region("KE".to_string())),
        "lamu" => Some(LocationConstraint::Region("KE".to_string())),
        "malindi" => Some(LocationConstraint::Region("KE".to_string())),
        "johannesburg" | "joburg" => Some(LocationConstraint::Region("ZA".to_string())),
        "cape town" => Some(LocationConstraint::Region("ZA".to_string())),
        "durban" => Some(LocationConstraint::Region("ZA".to_string())),
        "cairo" => Some(LocationConstraint::Region("EG".to_string())),
        "addis ababa" | "addis" => Some(LocationConstraint::Region("ET".to_string())),
        "lagos" => Some(LocationConstraint::Region("NG".to_string())),
        "abuja" => Some(LocationConstraint::Region("NG".to_string())),
        "dar es salaam" | "dar" => Some(LocationConstraint::Region("TZ".to_string())),
        "zanzibar" => Some(LocationConstraint::Region("TZ".to_string())),
        "kilimanjaro" => Some(LocationConstraint::Region("TZ".to_string())),
        "marrakech" | "casablanca" => Some(LocationConstraint::Region("MA".to_string())),
        // Asia — cities
        "tokyo" => Some(LocationConstraint::Region("JP".to_string())),
        "osaka" => Some(LocationConstraint::Region("JP".to_string())),
        "bangkok" => Some(LocationConstraint::Region("TH".to_string())),
        "singapore" => Some(LocationConstraint::Region("SG".to_string())),
        "hong kong" => Some(LocationConstraint::Region("HK".to_string())),
        "taipei" => Some(LocationConstraint::Region("TW".to_string())),
        "seoul" => Some(LocationConstraint::Region("KR".to_string())),
        "beijing" | "shanghai" | "guangzhou" => Some(LocationConstraint::Region("CN".to_string())),
        "mumbai" | "delhi" | "bangalore" | "chennai" | "kolkata" => {
            Some(LocationConstraint::Region("IN".to_string()))
        }
        "dubai" | "abu dhabi" => Some(LocationConstraint::Region("UAE".to_string())),
        "doha" => Some(LocationConstraint::Region("QA".to_string())),
        "tel aviv" | "jerusalem" => Some(LocationConstraint::Region("IL".to_string())),
        "kuala lumpur" => Some(LocationConstraint::Region("MY".to_string())),
        "manila" => Some(LocationConstraint::Region("PH".to_string())),
        "hanoi" | "ho chi minh" | "saigon" => Some(LocationConstraint::Region("VN".to_string())),
        "bali" | "jakarta" => Some(LocationConstraint::Region("ID".to_string())),
        // South America — countries & cities
        "brazil" => Some(LocationConstraint::Region("BR".to_string())),
        "argentina" => Some(LocationConstraint::Region("AR".to_string())),
        "colombia" => Some(LocationConstraint::Region("CO".to_string())),
        "peru" => Some(LocationConstraint::Region("PE".to_string())),
        "chile" => Some(LocationConstraint::Region("CL".to_string())),
        "rio" | "rio de janeiro" | "sao paulo" => {
            Some(LocationConstraint::Region("BR".to_string()))
        }
        "buenos aires" => Some(LocationConstraint::Region("AR".to_string())),
        "bogota" => Some(LocationConstraint::Region("CO".to_string())),
        "lima" => Some(LocationConstraint::Region("PE".to_string())),
        "santiago" => Some(LocationConstraint::Region("CL".to_string())),
        // Oceania — cities
        "sydney" | "melbourne" | "brisbane" | "perth" => {
            Some(LocationConstraint::Region("AU".to_string()))
        }
        "auckland" | "wellington" | "queenstown" => {
            Some(LocationConstraint::Region("NZ".to_string()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_from() {
        let p = FlightPrompt::parse("London to Paris");
        // London maps to UK:London (London area only, not all UK); Paris stays name-based
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("UK:London".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::AirportName("paris".to_string()))
        );
    }

    #[test]
    fn test_parse_simple() {
        let p = FlightPrompt::parse("Flight from London to Paris");
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("UK:London".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::AirportName("paris".to_string()))
        );
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
    fn test_parse_city_maps_to_name() {
        // London maps to UK:London (London area only); Paris stays name-based
        let p = FlightPrompt::parse("Flight from London to Paris");
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("UK:London".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::AirportName("paris".to_string()))
        );
    }

    #[test]
    fn test_parse_london_uk_to_region() {
        let p = FlightPrompt::parse("Flight from London UK to Germany");
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("UK:London".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("DE".to_string()))
        );
    }

    #[test]
    fn test_parse_london_to_italy() {
        let p = FlightPrompt::parse("Flight from London to Italy");
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("UK:London".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("IT".to_string()))
        );
    }

    #[test]
    fn test_parse_rome_italy_as_region() {
        // "Rome Italy" must resolve to Region(IT), not AirportName, to avoid matching Rome GA (KRMG)
        let p = FlightPrompt::parse("Flight from EGMC to Rome Italy");
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGMC".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("IT".to_string())),
            "Rome Italy should map to Italy region, not airport name"
        );
    }

    #[test]
    fn test_parse_rome_comma_italy_as_region() {
        let p = FlightPrompt::parse("Flight from London to Rome, Italy");
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("IT".to_string()))
        );
    }

    #[test]
    fn test_parse_paris_france_as_region() {
        let p = FlightPrompt::parse("Flight from EGLL to Paris France");
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("FR".to_string())),
            "Paris France should map to France region"
        );
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
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("KE".to_string())),
            "Nairobi should resolve to Kenya region"
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("KE".to_string())),
            "Lamu should resolve to Kenya region, not ICAO 'LAMU'"
        );
    }

    #[test]
    fn test_parse_nairobi_to_mombasa() {
        let p = FlightPrompt::parse("Nairobi to Mombasa");
        assert_eq!(p.origin, Some(LocationConstraint::Region("KE".to_string())),);
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("KE".to_string())),
        );
    }

    #[test]
    fn test_parse_tokyo_to_bangkok() {
        let p = FlightPrompt::parse("Tokyo to Bangkok");
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("JP".to_string())),
            "Tokyo should resolve to Japan region"
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("TH".to_string())),
            "Bangkok should resolve to Thailand region"
        );
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
}
