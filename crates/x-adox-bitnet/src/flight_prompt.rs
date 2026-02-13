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
        }
    }
}

impl FlightPrompt {
    pub fn parse(input: &str) -> Self {
        let mut prompt = FlightPrompt::default();
        // 1. Check for "ignore guardrails"
        let mut input_lower = input.to_lowercase();
        if input_lower.contains("ignore guardrails") {
            prompt.ignore_guardrails = true;
            input_lower = input_lower.replace("ignore guardrails", "");
        }

        // 2. Parse Origin and Destination
        // Patterns: "from X to Y", "flight from X to Y", "X to Y"
        // We look for "(flight from |from )?(capture) to (capture)"
        static LOC_RE: OnceLock<Regex> = OnceLock::new();
        let loc_re = LOC_RE.get_or_init(|| {
            Regex::new(
                r"(?:flight\s+from\s+|from\s+|^flight\s+)?(.+?)\s+to\s+(.+?)(\s+using|\s+in|\s+with|\s+for|$)",
            )
            .unwrap()
        });

        if let Some(caps) = loc_re.captures(&input_lower) {
            let origin_str = caps[1].trim();
            let dest_str = caps[2].trim();

            prompt.origin = Some(parse_location(origin_str));
            prompt.destination = Some(parse_location(dest_str));
        }

        // 3. Parse Aircraft
        // Patterns: "using [ACF]", "in a [ACF]", "with a [ACF]"
        static ACF_RE: OnceLock<Regex> = OnceLock::new();
        let acf_re = ACF_RE.get_or_init(|| {
            Regex::new(r"(?:using|in|with)(?:\s+a|\s+an)?\s+(.+?)(\s+for|\s+from|$)").unwrap()
        });

        if let Some(caps) = acf_re.captures(&input_lower) {
            let acf_str = caps[1].trim();
            if !acf_str.is_empty() {
                prompt.aircraft = Some(AircraftConstraint::Tag(acf_str.to_string()));
            }
        }

        // 4. Parse Duration
        // Patterns: "for 1 hour", "for 45 mins", "1 hour flight"
        // Regex to capture number + unit
        static TIME_RE: OnceLock<Regex> = OnceLock::new();
        let time_re = TIME_RE
            .get_or_init(|| Regex::new(r"(?:for\s+)?(\d+)\s*(hour|hr|minute|min|m)s?").unwrap());

        if let Some(caps) = time_re.captures(&input_lower) {
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
        // Assume ICAO if 4 letters
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

/// Attempts to recognize a string as a geographic region (country, US sub-region, continent, etc.).
/// Returns `Some(Region(...))` with a canonical name if recognized, `None` otherwise.
fn try_as_region(s: &str) -> Option<LocationConstraint> {
    // Strip leading "the "
    let s = s.strip_prefix("the ").unwrap_or(s).trim();

    // Use the global region index for lookup
    let index = crate::geo::RegionIndex::new();

    if let Some(region) = index.search(s) {
        return Some(LocationConstraint::Region(region.id.to_string()));
    }

    // Fallback for nicknames not in the core index yet
    // (Though most should be in data.rs now)
    match s.to_lowercase().as_str() {
        "british isles" => Some(LocationConstraint::Region("BI".to_string())),
        "ireland" | "eire" => Some(LocationConstraint::Region("IE".to_string())),
        "uk" | "united kingdom" => Some(LocationConstraint::Region("UK".to_string())),
        "gb" | "great britain" => Some(LocationConstraint::Region("GB".to_string())),
        "usa" | "us" | "united states" => Some(LocationConstraint::Region("US".to_string())),
        "socal" | "southern california" => Some(LocationConstraint::Region("US:SoCal".to_string())),
        "norcal" | "northern california" => {
            Some(LocationConstraint::Region("US:NorCal".to_string()))
        }
        "pnw" | "pacific northwest" => Some(LocationConstraint::Region("US:OR".to_string())), // Approximation to Oregon for now
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_from() {
        let p = FlightPrompt::parse("London to Paris");
        assert_eq!(
            p.origin,
            Some(LocationConstraint::AirportName("london".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::AirportName("paris".to_string()))
        );
    }

    #[test]
    fn test_parse_simple() {
        let p = FlightPrompt::parse("Flight from London to Paris");
        match p.origin {
            Some(LocationConstraint::AirportName(r)) => assert_eq!(r, "london"),
            _ => panic!("Bad origin"),
        }
        match p.destination {
            Some(LocationConstraint::AirportName(r)) => assert_eq!(r, "paris"),
            _ => panic!("Bad dest"),
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
    fn test_parse_article_stripped() {
        let p = FlightPrompt::parse("Flight from the British Isles to the Caribbean");
        assert_eq!(p.origin, Some(LocationConstraint::Region("BI".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::AirportName("caribbean".to_string()))
        );
    }

    #[test]
    fn test_parse_city_still_airport_name() {
        // "london" and "paris" are not in the region vocabulary — they stay AirportName
        let p = FlightPrompt::parse("Flight from London to Paris");
        assert_eq!(
            p.origin,
            Some(LocationConstraint::AirportName("london".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::AirportName("paris".to_string()))
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
}
