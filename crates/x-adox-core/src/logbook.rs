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

impl LogbookEntry {
    pub fn to_log_line(&self) -> String {
        let date_str = self
            .date
            .map(|d| d.format("%y%m%d").to_string())
            .unwrap_or_else(|| "     0".to_string());
        format!(
            "2 {:>6} {:>7} {:>7}   {:>1}   {:>3.2}   {:>3.2}   {:>3.2}   {:>3.2}  {:>6}  {}",
            date_str,
            self.dep_airport,
            self.arr_airport,
            self.landings,
            self.total_duration,
            self.cross_country,
            self.ifr_time,
            self.night_time,
            self.tail_number,
            self.aircraft_type
        )
    }
}

pub struct LogbookParser;

impl LogbookParser {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Vec<LogbookEntry>> {
        let file = File::open(path).context("Failed to open logbook file")?;
        let reader = BufReader::new(file);
        Self::parse(reader)
    }

    pub fn save_file<P: AsRef<Path>>(path: P, entries: &[LogbookEntry]) -> Result<()> {
        use std::io::Write;

        // Always create a backup first
        let bak_path = path.as_ref().with_extension("bak");
        if path.as_ref().exists() {
            std::fs::copy(&path, &bak_path).context("Failed to create logbook backup")?;
        }

        let mut file = File::create(path).context("Failed to create logbook file")?;

        // Write header
        writeln!(file, "I")?;
        writeln!(file, "1 Version")?;

        for entry in entries {
            writeln!(file, "{}", entry.to_log_line())?;
        }

        // Write EOF marker
        writeln!(file, "99")?;

        Ok(())
    }

    pub fn parse<R: BufRead>(reader: R) -> Result<Vec<LogbookEntry>> {
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('I') || trimmed == "99" {
                continue;
            }

            // Version line often starts with a version number like "1 Version" or "1100 version"
            if trimmed.contains("Version") || trimmed.contains("version") {
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

    #[test]
    fn test_roundtrip() {
        let entry = LogbookEntry {
            date: NaiveDate::from_ymd_opt(2024, 4, 1),
            dep_airport: "KBOS".to_string(),
            arr_airport: "KJFK".to_string(),
            landings: 1,
            total_duration: 1.25,
            cross_country: 1.25,
            ifr_time: 0.0,
            night_time: 0.0,
            tail_number: "N123AB".to_string(),
            aircraft_type: "B738".to_string(),
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("log.txt");

        LogbookParser::save_file(&file_path, &[entry.clone()]).unwrap();
        let loaded = LogbookParser::parse_file(&file_path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].dep_airport, entry.dep_airport);
        assert_eq!(loaded[0].arr_airport, entry.arr_airport);
        assert_eq!(loaded[0].total_duration, entry.total_duration);
        assert_eq!(loaded[0].tail_number, entry.tail_number);
        assert_eq!(loaded[0].aircraft_type, entry.aircraft_type);
    }
}
