use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
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
    pub proj_x: Option<f32>, // Normalized Mercator X (0.0 to 1.0)
    pub proj_y: Option<f32>, // Normalized Mercator Y (0.0 to 1.0)
}

impl std::hash::Hash for Airport {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.name.hash(state);
        self.airport_type.hash(state);
        if let Some(lat) = self.lat {
            lat.to_bits().hash(state);
        }
        if let Some(lon) = self.lon {
            lon.to_bits().hash(state);
        }
    }
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

    pub fn parse<R: BufRead>(mut reader: R) -> Result<Vec<Airport>, AptDatError> {
        let mut airports = Vec::with_capacity(1000); // Pre-allocate some space
        let mut line_buf = String::with_capacity(256);
        let mut current_airport: Option<AirportBuilder> = None;

        loop {
            line_buf.clear();
            let bytes_read = reader.read_line(&mut line_buf)?;
            if bytes_read == 0 {
                // End of file: Push last airport
                if let Some(builder) = current_airport.take() {
                    airports.push(builder.build());
                }
                break;
            }

            let line = line_buf.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let code_str = match parts.next() {
                Some(s) => s,
                None => continue,
            };
            match code_str {
                "1" | "16" | "17" => {
                    // Start of new airport. Push previous if exists.
                    if let Some(builder) = current_airport.take() {
                        airports.push(builder.build());
                    }

                    let apt_type = match code_str {
                        "1" => AirportType::Land,
                        "16" => AirportType::Seaplane,
                        "17" => AirportType::Heliport,
                        _ => AirportType::Land,
                    };

                    current_airport = parse_airport_header(line, apt_type);
                }
                "100" | "101" => {
                    // Runway or Water Runway
                    if let Some(ref mut builder) = current_airport {
                        parse_runway(line, builder);
                    }
                }
                "102" => {
                    // Helipad
                    if let Some(ref mut builder) = current_airport {
                        parse_helipad(line, builder);
                    }
                }
                "99" => {
                    // Explicit end of file
                    if let Some(builder) = current_airport.take() {
                        airports.push(builder.build());
                    }
                    break;
                }
                _ => {}
            }
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
        let (lat, lon, proj_x, proj_y) = if !self.lats.is_empty() {
            let avg_lat: f64 = self.lats.iter().sum::<f64>() / self.lats.len() as f64;
            let avg_lon: f64 = self.lons.iter().sum::<f64>() / self.lons.len() as f64;

            // Calculate normalized Mercator coordinates (0.0 to 1.0)
            let px = (avg_lon + 180.0) / 360.0;
            let lat_rad = avg_lat.to_radians();
            let py =
                (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0;

            (
                Some(avg_lat),
                Some(avg_lon),
                Some(px as f32),
                Some(py as f32),
            )
        } else {
            (None, None, None, None)
        };

        Airport {
            id: self.id,
            name: self.name,
            airport_type: self.airport_type,
            lat,
            lon,
            proj_x,
            proj_y,
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
        lats: Vec::with_capacity(4),
        lons: Vec::with_capacity(4),
    })
}

fn parse_runway(line: &str, builder: &mut AirportBuilder) {
    let mut parts = line.split_whitespace();
    // Skip first 9 parts to get to lat/lon 1
    // 0:100, 1:width, 2:surf, 3:shoulder, 4:smooth, 5:center, 6:edge, 7:dist, 8:rwy1
    for _ in 0..9 {
        parts.next();
    }

    if let (Some(lat_s), Some(lon_s)) = (parts.next(), parts.next()) {
        if let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>()) {
            builder.lats.push(lat);
            builder.lons.push(lon);
        }
    }

    // Skip to next lat/lon
    // 11: rwy2_thld, 12: rwy2_vasi, 13: rwy2_reil, 14: rwy2_mark, 15: rwy2_stop, 16: rwy2_blast, 17: rwy2_nm
    for _ in 0..7 {
        parts.next();
    }

    if let (Some(lat_s), Some(lon_s)) = (parts.next(), parts.next()) {
        if let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>()) {
            builder.lats.push(lat);
            builder.lons.push(lon);
        }
    }
}

fn parse_helipad(line: &str, builder: &mut AirportBuilder) {
    let mut parts = line.split_whitespace();
    parts.next(); // 102
    parts.next(); // Designator

    if let (Some(lat_s), Some(lon_s)) = (parts.next(), parts.next()) {
        if let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>()) {
            builder.lats.push(lat);
            builder.lons.push(lon);
        }
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
