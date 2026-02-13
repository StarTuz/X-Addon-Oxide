// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum AirportType {
    Land,
    Seaplane,
    Heliport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SurfaceType {
    Hard,  // Asphalt, Concrete
    Soft,  // Grass, Dirt, Gravel
    Water, // Water
}

impl Default for SurfaceType {
    fn default() -> Self {
        Self::Soft
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Airport {
    pub id: String,
    pub name: String,
    pub airport_type: AirportType,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub proj_x: Option<f32>,            // Normalized Mercator X (0.0 to 1.0)
    pub proj_y: Option<f32>,            // Normalized Mercator Y (0.0 to 1.0)
    pub max_runway_length: Option<u32>, // in meters
    pub surface_type: Option<SurfaceType>,
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
                "1302" => {
                    // Metadata (Datum, city, etc.)
                    if let Some(ref mut builder) = current_airport {
                        parse_metadata(line, builder);
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
    datum_lat: Option<f64>,
    datum_lon: Option<f64>,
    max_rwy_len: f64,
    primary_surface: SurfaceType,
}

impl AirportBuilder {
    fn build(self) -> Airport {
        let (lat, lon, proj_x, proj_y) =
            if let (Some(d_lat), Some(d_lon)) = (self.datum_lat, self.datum_lon) {
                // Priority 1: Use the explicit datum coordinates (most precise)
                let px = (d_lon + 180.0) / 360.0;
                let lat_rad = d_lat.to_radians();
                let py =
                    (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0;

                (Some(d_lat), Some(d_lon), Some(px as f32), Some(py as f32))
            } else if !self.lats.is_empty() {
                // Priority 2: Standard averaging of runways
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
            max_runway_length: if self.max_rwy_len > 0.0 {
                Some(self.max_rwy_len as u32)
            } else {
                None
            },
            surface_type: if self.max_rwy_len > 0.0 {
                Some(self.primary_surface)
            } else {
                None
            },
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
        datum_lat: None,
        datum_lon: None,
        max_rwy_len: 0.0,
        primary_surface: SurfaceType::Soft,
    })
}

fn parse_metadata(line: &str, builder: &mut AirportBuilder) {
    let mut parts = line.split_whitespace();
    parts.next(); // 1302
    if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
        match key {
            "datum_lat" => {
                if let Ok(lat) = val.parse::<f64>() {
                    builder.datum_lat = Some(lat);
                }
            }
            "datum_lon" => {
                if let Ok(lon) = val.parse::<f64>() {
                    builder.datum_lon = Some(lon);
                }
            }
            _ => {}
        }
    }
}

fn parse_runway(line: &str, builder: &mut AirportBuilder) {
    let mut parts = line.split_whitespace();

    // 100 width surface ...
    parts.next(); // 100
    let _width = parts.next();
    let surface_code = parts.next().unwrap_or("1"); // Default to asphalt if missing

    // Map surface code
    // 1=Asphalt, 2=Concrete -> Hard
    // 3=Turf, 4=Dirt, 5=Gravel -> Soft
    // 13=Water -> Water
    // 14=Snow, 15=Transparent -> Soft/Hard? Treat 15 as Hard (usually overlays).
    let surface = match surface_code {
        "1" | "2" | "15" => SurfaceType::Hard,
        "13" => SurfaceType::Water,
        _ => SurfaceType::Soft,
    };

    // Skip to lat/lon 1 (index 9 in 0-indexed split, we consumed 3)
    // 3:shoulder, 4:smooth, 5:center, 6:edge, 7:dist, 8:rwy1
    for _ in 0..6 {
        parts.next();
    }

    let mut lat1 = 0.0;
    let mut lon1 = 0.0;
    if let (Some(lat_s), Some(lon_s)) = (parts.next(), parts.next()) {
        if let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>()) {
            lat1 = lat;
            lon1 = lon;
            builder.lats.push(lat);
            builder.lons.push(lon);
        }
    }

    // Skip to lat/lon 2
    // 11: rwy2_thld, 12: rwy2_vasi, 13: rwy2_reil, 14: rwy2_mark, 15: rwy2_stop, 16: rwy2_blast, 17: rwy2_nm
    for _ in 0..7 {
        parts.next();
    }

    let mut lat2 = 0.0;
    let mut lon2 = 0.0;
    if let (Some(lat_s), Some(lon_s)) = (parts.next(), parts.next()) {
        if let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>()) {
            lat2 = lat;
            lon2 = lon;
            builder.lats.push(lat);
            builder.lons.push(lon);
        }
    }

    // Calculate length
    let length = haversine_dist(lat1, lon1, lat2, lon2);
    if length > builder.max_rwy_len {
        builder.max_rwy_len = length;
        // If this is the longest runway, assume its surface is the airport's primary surface
        builder.primary_surface = surface;
    }
}

// Simple Haversine distance in meters
fn haversine_dist(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371000.0; // Earth radius in meters
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
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

        // Runway 09/27 length check
        // Coords: (42.358, -71.018) to (42.365, -70.991)
        // Distance roughly 2.3km = 2300m
        assert!(kbos.max_runway_length.is_some());
        let len = kbos.max_runway_length.unwrap();
        assert!(len > 2000 && len < 4000);
        assert_eq!(kbos.surface_type, Some(SurfaceType::Hard));

        let h123 = &airports[2];
        assert_eq!(h123.id, "H123");
        assert_eq!(h123.lat, Some(42.0));
        // Helipad parser doesn't set runway length yet, so it should be None
        assert!(h123.max_runway_length.is_none());
    }
}
