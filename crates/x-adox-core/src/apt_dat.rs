use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AirportType {
    Land,
    Seaplane,
    Heliport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Airport {
    pub id: String,
    pub name: String,
    pub airport_type: AirportType,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

pub struct AptDatParser;

#[derive(Error, Debug)]
pub enum AptDatError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

impl AptDatParser {
    /// Parses an apt.dat file and returns a list of airports.
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Vec<Airport>, AptDatError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::parse(reader)
    }

    pub fn parse<R: BufRead>(reader: R) -> Result<Vec<Airport>, AptDatError> {
        let mut airports = Vec::new();

        let mut current_airport: Option<AirportBuilder> = None;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let code_str = line.split_whitespace().next();

            match code_str {
                Some("1") | Some("16") | Some("17") => {
                    // Start of new airport. Push previous if exists.
                    if let Some(builder) = current_airport.take() {
                        airports.push(builder.build());
                    }

                    let apt_type = match code_str {
                        Some("1") => AirportType::Land,
                        Some("16") => AirportType::Seaplane,
                        Some("17") => AirportType::Heliport,
                        _ => AirportType::Land,
                    };

                    current_airport = parse_airport_header(line, apt_type);
                }
                Some("100") => {
                    // Runway
                    if let Some(ref mut builder) = current_airport {
                        parse_runway(line, builder);
                    }
                }
                Some("102") => {
                    // Helipad
                    if let Some(ref mut builder) = current_airport {
                        parse_helipad(line, builder);
                    }
                }
                Some("99") => {
                    // Explicit end of file usually, but we handle via stream end too.
                }
                _ => {}
            }
        }

        // Push last one
        if let Some(builder) = current_airport {
            airports.push(builder.build());
        }

        Ok(airports)
    }
}

struct AirportBuilder {
    id: String,
    name: String,
    airport_type: AirportType,
    lats: Vec<f64>,
    lons: Vec<f64>,
}

impl AirportBuilder {
    fn build(self) -> Airport {
        let (lat, lon) = if !self.lats.is_empty() {
            let avg_lat: f64 = self.lats.iter().sum::<f64>() / self.lats.len() as f64;
            let avg_lon: f64 = self.lons.iter().sum::<f64>() / self.lons.len() as f64;
            (Some(avg_lat), Some(avg_lon))
        } else {
            (None, None)
        };

        Airport {
            id: self.id,
            name: self.name,
            airport_type: self.airport_type,
            lat,
            lon,
        }
    }
}

fn parse_airport_header(line: &str, apt_type: AirportType) -> Option<AirportBuilder> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let id = parts[4].to_string();
    let name = parts[5..].join(" ");

    Some(AirportBuilder {
        id,
        name,
        airport_type: apt_type,
        lats: Vec::new(),
        lons: Vec::new(),
    })
}

fn parse_runway(line: &str, builder: &mut AirportBuilder) {
    // 100 width surf shoulder smooth center_lights edge_lights dist_remaining
    // lat1 lon1 lat2 lon2 ...
    // Indices in parts (0-based):
    // 0=100, 1..7 (properties)
    // 8=lat1, 9=lon1, 10=lat2, 11=lon2 (approx, depending on format version 1000)
    // Specifically 1000 spec:
    // 0:100, 1:width, 2:surf, 3:shoulder, 4:smooth, 5:center, 6:edge, 7:dist
    // 8:REIL1, 9:VASI1, 10:REIL2, 11:VASI2
    // 12:H1, 13:H2, 14:HE1, 15:HE2 (Threshold displacements?)
    // WAIT. 850 spec vs 1000 spec.
    // Row 100 in 1000 spec:
    // ...
    // 9: lat1, 10: lon1, 18: lat2, 19: lon2 ? No.

    // Let's assume standard modern apt.dat (1000/1050/1100).
    // The columns are:
    // 0: 100
    // ...
    // 8: Runway 1 Number (e.g. 35L)
    // 9: Latitude 1
    // 10: Longitude 1
    // ...
    // 17: Runway 2 Number (e.g. 17R)
    // 18: Latitude 2
    // 19: Longitude 2

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 20 {
        return;
    } // Safety check

    // End 1
    if let (Ok(lat), Ok(lon)) = (parts[9].parse::<f64>(), parts[10].parse::<f64>()) {
        builder.lats.push(lat);
        builder.lons.push(lon);
    }

    // End 2
    if let (Ok(lat), Ok(lon)) = (parts[18].parse::<f64>(), parts[19].parse::<f64>()) {
        builder.lats.push(lat);
        builder.lons.push(lon);
    }
}

fn parse_helipad(line: &str, builder: &mut AirportBuilder) {
    // 102 implementation
    // 102 H1 47.44166200 -122.31219600 0.00 30.48 30.48 1 0 0 0.00 0 0 2
    // 0: 102
    // 1: Designator
    // 2: Lat
    // 3: Lon
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return;
    }

    if let (Ok(lat), Ok(lon)) = (parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
        builder.lats.push(lat);
        builder.lons.push(lon);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_airports() {
        let data = "\
I
1000 Version
1 433 0 0 KBOS General Edward Lawrence Logan Intl
100 60.96 1 2 0.25 1 3 0 09 42.35824967 -071.01833215 0 0 3 0 1 1 27 42.36533800 -070.99120668 0 0 3 0 1 1
16 0 0 0 W01 fake seaplane base
17 50 0 0 H123 fake heliport
102 H1 42.000000 -71.000000 0 0 0 0 0 0 0 0 0 0
";
        let cursor = Cursor::new(data);
        let airports = AptDatParser::parse(cursor).unwrap();

        assert_eq!(airports.len(), 3);

        let kbos = &airports[0];
        assert_eq!(kbos.id, "KBOS");
        assert!(kbos.lat.is_some());
        assert!(kbos.lon.is_some());

        // Approximate check
        // Lat1: 42.358..., Lat2: 42.365...
        // Avg ~ 42.36
        assert!(kbos.lat.unwrap() > 42.3);

        let h123 = &airports[2];
        assert_eq!(h123.id, "H123");
        assert_eq!(h123.lat, Some(42.0));
    }
}
