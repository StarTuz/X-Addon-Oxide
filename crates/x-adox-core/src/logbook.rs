use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogbookEntry {
    pub date: Option<NaiveDate>,
    pub dep_airport: String,
    pub arr_airport: String,
    pub landings: u32,
    pub total_duration: f64,
    pub cross_country: f64,
    pub ifr_time: f64,
    pub night_time: f64,
    pub tail_number: String,
    pub aircraft_type: String,
}

pub struct LogbookParser;

impl LogbookParser {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Vec<LogbookEntry>> {
        let file = File::open(path).context("Failed to open logbook file")?;
        let reader = BufReader::new(file);
        Self::parse(reader)
    }

    pub fn parse<R: BufRead>(reader: R) -> Result<Vec<LogbookEntry>> {
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('I') {
                continue;
            }

            // Split by whitespace
            let parts: Vec<&str> = trimmed.split_whitespace().collect();

            // Expected format:
            // 0: Entry Code (usually 2)
            // 1: Date (YYMMDD)
            // 2: Dep ICAO
            // 3: Arr ICAO
            // 4: Landings
            // 5: Total Time
            // 6: Cross Country
            // 7: IFR
            // 8: Night
            // 9: Tail (Optional in some versions, but let's assume it's there or handle length)
            // 10: Type (Optional)

            if parts.len() < 9 {
                continue;
            }

            if parts[0] != "2" {
                continue;
            }

            let date = NaiveDate::parse_from_str(parts[1], "%y%m%d").ok();
            let dep_airport = parts[2].to_string();
            let arr_airport = parts[3].to_string();
            let landings = parts[4].parse::<u32>().unwrap_or(0);
            let total_duration = parts[5].parse::<f64>().unwrap_or(0.0);
            let cross_country = parts[6].parse::<f64>().unwrap_or(0.0);
            let ifr_time = parts[7].parse::<f64>().unwrap_or(0.0);
            let night_time = parts[8].parse::<f64>().unwrap_or(0.0);

            // Tail number and type might be further out or joined
            let tail_number = parts.get(9).unwrap_or(&"").to_string();
            let aircraft_type = if parts.len() > 10 {
                parts[10..].join(" ")
            } else {
                "".to_string()
            };

            entries.push(LogbookEntry {
                date,
                dep_airport,
                arr_airport,
                landings,
                total_duration,
                cross_country,
                ifr_time,
                night_time,
                tail_number,
                aircraft_type,
            });
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use std::io::Cursor;

    #[test]
    fn test_parse_sample() {
        let data = "I\n1000 version\n2 240401 KBOS KJFK 1 1.25 1.25 0.0 0.0 N123AB B738\n2 240402 KJFK KLAX 1 5.5 5.5 1.0 2.0 N456CD B772";
        let cursor = Cursor::new(data);
        let entries = LogbookParser::parse(cursor).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].dep_airport, "KBOS");
        assert_eq!(entries[0].arr_airport, "KJFK");
        assert_eq!(entries[0].total_duration, 1.25);
        assert_eq!(entries[0].tail_number, "N123AB");
        assert_eq!(entries[0].aircraft_type, "B738");

        // Check date
        assert!(entries[0].date.is_some());
        assert_eq!(entries[0].date.unwrap().year(), 2024);
        assert_eq!(entries[0].date.unwrap().month(), 4);
        assert_eq!(entries[0].date.unwrap().day(), 1);
    }
}
