use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum PropType {
    RecipCarb,     // 0
    RecipInjected, // 1
    FreeTurbine,   // 2
    Electric,      // 3
    LoBypassJet,   // 4
    HiBypassJet,   // 5
    Rocket,        // 6
    TipRockets,    // 7
    FixedTurbine,  // 8
    Unknown,
}

impl From<i32> for PropType {
    fn from(val: i32) -> Self {
        match val {
            0 => PropType::RecipCarb,
            1 => PropType::RecipInjected,
            2 => PropType::FreeTurbine,
            3 => PropType::Electric,
            4 => PropType::LoBypassJet,
            5 => PropType::HiBypassJet,
            6 => PropType::Rocket,
            7 => PropType::TipRockets,
            8 => PropType::FixedTurbine,
            _ => PropType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AcfData {
    pub description: String,
    pub author: String,
    pub studio: String,
    pub prop_type: Option<PropType>,
}

/// Scans the directory for a .acf file and parses it.
pub fn parse_acf_in_dir(dir: &Path) -> Result<AcfData> {
    if !dir.exists() || !dir.is_dir() {
        return Err(anyhow::anyhow!("Path is not a valid directory: {:?}", dir));
    }

    let mut acf_file = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "acf" {
                    acf_file = Some(path);
                    break;
                }
            }
        }
    }

    if let Some(path) = acf_file {
        parse_acf(&path)
    } else {
        Err(anyhow::anyhow!(
            "No .acf file found in directory: {:?}",
            dir
        ))
    }
}

pub fn parse_acf(path: &Path) -> Result<AcfData> {
    let file = File::open(path).with_context(|| format!("Failed to open ACF file: {:?}", path))?;
    let reader = BufReader::new(file);
    let mut data = AcfData::default();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Check for engine type with or without "acf/" prefix
        // Matches: "P acf/_engn/0/_type" OR "P _engn/0/_type"
        if trimmed.ends_with("_engn/0/_type") || trimmed.contains("_engn/0/_type ") {
            // Extract value part
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if let Some(val_str) = parts.last() {
                println!(
                    "[ACF Parser] Found engine type line: '{}' -> value: '{}'",
                    trimmed, val_str
                );

                // Try parsing as integer first
                if let Ok(val) = val_str.parse::<i32>() {
                    let prop_type = PropType::from(val);
                    println!(
                        "[ACF Parser] Parsed int engine type: {:?} (val: {})",
                        prop_type, val
                    );
                    data.prop_type = Some(prop_type);
                } else {
                    // Try parsing as string keyword (legacy/alt format)
                    let prop_type = match *val_str {
                        "JET" | "JET_1SPOOL" | "JET_2SPOOL" => Some(PropType::LoBypassJet), // Map all jets to valid enum
                        "PROP" | "RCP_CRB" | "RCP_INJ" => Some(PropType::RecipCarb), // Map pistons to Prop
                        "TURB" | "TRB_FRE" | "TRB_FIX" => Some(PropType::FreeTurbine), // Map turboprops
                        "ELE" => Some(PropType::Electric),
                        _ => None,
                    };

                    if let Some(pt) = prop_type {
                        println!(
                            "[ACF Parser] Parsed string engine type: {:?} (val: {})",
                            pt, val_str
                        );
                        data.prop_type = Some(pt);
                    } else {
                        println!(
                            "[ACF Parser] FAILED to parse engine type from: '{}'",
                            val_str
                        );
                    }
                }
            }
        } else if trimmed.starts_with("P acf/_descrip") {
            data.description = trimmed
                .trim_start_matches("P acf/_descrip")
                .trim()
                .to_string();
        } else if trimmed.starts_with("P acf/_author") {
            data.author = trimmed
                .trim_start_matches("P acf/_author")
                .trim()
                .to_string();
        } else if trimmed.starts_with("P acf/_studio") {
            data.studio = trimmed
                .trim_start_matches("P acf/_studio")
                .trim()
                .to_string();
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_acf_jet() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "I")?;
        writeln!(file, "1000 Version")?;
        writeln!(file, "P acf/_descrip E-3 Sentry AWACS")?;
        writeln!(file, "P acf/_author Boeing")?;
        writeln!(file, "P acf/_engn/0/_type 4")?; // 4=LoBypassJet

        let data = parse_acf(file.path())?;
        assert_eq!(data.description, "E-3 Sentry AWACS");
        assert_eq!(data.author, "Boeing");
        assert_eq!(data.prop_type, Some(PropType::LoBypassJet));
        Ok(())
    }

    #[test]
    fn test_parse_acf_piston() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "P acf/_engn/0/_type 0")?; // 0=RecipCarb
        let data = parse_acf(file.path())?;
        assert_eq!(data.prop_type, Some(PropType::RecipCarb));
        Ok(())
    }

    #[test]
    fn test_parse_acf_legacy_f4() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        // Missing "acf/" prefix and using string value "JET_2SPOOL"
        writeln!(file, "P _engn/0/_type JET_2SPOOL")?;
        let data = parse_acf(file.path())?;
        assert_eq!(data.prop_type, Some(PropType::LoBypassJet));
        Ok(())
    }
}
