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
    if s.len() == 4 && s.chars().all(|c| c.is_alphabetic()) {
        LocationConstraint::ICAO(s.to_uppercase())
    } else if s == "here" || s == "current location" {
        LocationConstraint::Region("Here".to_string())
    } else if s == "anywhere" || s == "any" || s == "random" {
        LocationConstraint::Any
    } else if let Some(region) = try_as_region(s) {
        region
    } else {
        LocationConstraint::AirportName(s.to_string())
    }
}

/// Attempts to recognize a string as a geographic region.
fn try_as_region(s: &str) -> Option<LocationConstraint> {
    let s = s.strip_prefix("the ").unwrap_or(s).trim();
    let index = crate::geo::RegionIndex::new();

    if let Some(region) = index.search(s) {
        return Some(LocationConstraint::Region(region.id.to_string()));
    }

    // Fallback aliases (countries/regions and major cities → region for reliable flight gen)
    match s.to_lowercase().as_str() {
        "british isles" => Some(LocationConstraint::Region("BI".to_string())),
        "ireland" | "eire" => Some(LocationConstraint::Region("IE".to_string())),
        "uk" | "united kingdom" => Some(LocationConstraint::Region("UK".to_string())),
        "gb" | "great britain" => Some(LocationConstraint::Region("GB".to_string())),
        "london" | "london uk" | "london united kingdom" => Some(LocationConstraint::Region("UK".to_string())),
        "england" | "scotland" | "wales" => Some(LocationConstraint::Region("UK".to_string())),
        "italy" | "rome" | "milan" => Some(LocationConstraint::Region("IT".to_string())),
        "france" | "paris" => Some(LocationConstraint::Region("FR".to_string())),
        "germany" | "berlin" | "frankfurt" => Some(LocationConstraint::Region("DE".to_string())),
        "spain" | "madrid" | "barcelona" => Some(LocationConstraint::Region("ES".to_string())),
        "usa" | "us" | "united states" => Some(LocationConstraint::Region("US".to_string())),
        "canada" => Some(LocationConstraint::Region("CA".to_string())),
        "mexico" => Some(LocationConstraint::Region("MX".to_string())),
        "socal" | "southern california" => Some(LocationConstraint::Region("US:SoCal".to_string())),
        "riverside county" | "riverside" => Some(LocationConstraint::Region("US:SoCal".to_string())),
        "norcal" | "northern california" => {
            Some(LocationConstraint::Region("US:NorCal".to_string()))
        }
        "oregon" => Some(LocationConstraint::Region("US:OR".to_string())),
        "pnw" | "pacific northwest" => Some(LocationConstraint::Region("US:OR".to_string())),
        "alps" => Some(LocationConstraint::Region("Alps".to_string())),
        "rockies" => Some(LocationConstraint::Region("Rockies".to_string())),
        "caribbean" => Some(LocationConstraint::Region("Caribbean".to_string())),
        "south africa" => Some(LocationConstraint::Region("ZA".to_string())),
        "kenya" => Some(LocationConstraint::Region("KE".to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_from() {
        let p = FlightPrompt::parse("London to Paris");
        // London and Paris map to UK/FR regions for reliable flight gen
        assert_eq!(p.origin, Some(LocationConstraint::Region("UK".to_string())));
        assert_eq!(p.destination, Some(LocationConstraint::Region("FR".to_string())));
    }

    #[test]
    fn test_parse_simple() {
        let p = FlightPrompt::parse("Flight from London to Paris");
        assert_eq!(p.origin, Some(LocationConstraint::Region("UK".to_string())));
        assert_eq!(p.destination, Some(LocationConstraint::Region("FR".to_string())));
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
    fn test_parse_article_stripped() {
        let p = FlightPrompt::parse("Flight from the British Isles to the Caribbean");
        assert_eq!(p.origin, Some(LocationConstraint::Region("BI".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("Caribbean".to_string()))
        );
    }

    #[test]
    fn test_parse_city_maps_to_region() {
        // London/Paris map to UK/FR for reliable region-based flight gen
        let p = FlightPrompt::parse("Flight from London to Paris");
        assert_eq!(p.origin, Some(LocationConstraint::Region("UK".to_string())));
        assert_eq!(p.destination, Some(LocationConstraint::Region("FR".to_string())));
    }

    #[test]
    fn test_parse_london_to_italy() {
        let p = FlightPrompt::parse("Flight from London to Italy");
        assert_eq!(p.origin, Some(LocationConstraint::Region("UK".to_string())));
        assert_eq!(p.destination, Some(LocationConstraint::Region("IT".to_string())));
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
}
