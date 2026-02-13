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
                r"(?:flight\s+from\s+|from\s+)?(.+?)\s+to\s+(.+?)(\s+using|\s+in|\s+with|\s+for|$)",
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
    let s = s.trim();
    if s.len() == 4 && s.chars().all(|c| c.is_alphabetic()) {
        // Assume ICAO if 4 letters
        LocationConstraint::ICAO(s.to_uppercase())
    } else if s == "here" || s == "current location" {
        // Handling for "here" requires map context, so maybe return a special variant or Region("Here")
        // For now, let's treat it as a Region specific string that the generator handles
        LocationConstraint::Region("Here".to_string())
    } else if s == "anywhere" || s == "random" {
        LocationConstraint::Any
    } else {
        // Treat as generic search (Region OR Airport Name will be handled by generator)
        // We'll use AirportName as the carrier for "Arbitrary String" which logic should broaden.
        LocationConstraint::AirportName(s.to_string())
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
}
