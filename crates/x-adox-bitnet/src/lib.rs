use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub name: String,
    pub keywords: Vec<String>,
    pub score: u8,
    #[serde(default)]
    pub is_exclusion: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeuristicsConfig {
    pub rules: Vec<Rule>,
    pub fallback_score: u8,
}

impl Default for HeuristicsConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                Rule {
                    name: "Exclusion Logic (Overlay/Mesh Tweaks)".to_string(),
                    keywords: vec![
                        "overlay".to_string(),
                        "mesh".to_string(),
                        "ktex".to_string(),
                        "ortho".to_string(),
                    ],
                    score: 61,
                    is_exclusion: true,
                },
                Rule {
                    name: "Mesh/Terrain".to_string(),
                    keywords: vec![
                        "mesh".to_string(),
                        "uhd".to_string(),
                        "terrain".to_string(),
                        "zzz".to_string(),
                    ],
                    score: 60,
                    is_exclusion: false,
                },
                Rule {
                    name: "Ortho/Photo".to_string(),
                    keywords: vec![
                        "ortho".to_string(),
                        "photoscenery".to_string(),
                        "yortho".to_string(),
                    ],
                    score: 50,
                    is_exclusion: false,
                },
                Rule {
                    name: "Libraries".to_string(),
                    keywords: vec![
                        "library".to_string(),
                        "lib".to_string(),
                        "opensceneryx".to_string(),
                        "r2_library".to_string(),
                        "bs2001".to_string(),
                        "3dnl".to_string(),
                        "misterx".to_string(),
                        "zdp".to_string(),
                        "sam".to_string(),
                        "assets".to_string(),
                    ],
                    score: 45,
                    is_exclusion: false,
                },
                Rule {
                    name: "Landmarks".to_string(),
                    keywords: vec![
                        "landmarks".to_string(),
                        "global_forests".to_string(),
                        "landmark".to_string(),
                    ],
                    score: 40,
                    is_exclusion: false,
                },
                Rule {
                    name: "Orbx B / TrueEarth".to_string(),
                    keywords: vec!["orbx_b".to_string(), "trueearth_overlay".to_string()],
                    score: 35,
                    is_exclusion: false,
                },
                Rule {
                    name: "SimHeaven / X-World".to_string(),
                    keywords: vec![
                        "simheaven".to_string(),
                        "x-world".to_string(),
                        "w2xp".to_string(),
                    ],
                    score: 31,
                    is_exclusion: false,
                },
                Rule {
                    name: "Birds".to_string(),
                    keywords: vec![
                        "birds".to_string(),
                        "birdofprey".to_string(),
                        "crow".to_string(),
                        "goose".to_string(),
                        "pigeon".to_string(),
                        "seagulls".to_string(),
                    ],
                    score: 33,
                    is_exclusion: false,
                },
                Rule {
                    name: "Orbx A Custom".to_string(),
                    keywords: vec!["orbx_a".to_string()],
                    score: 25,
                    is_exclusion: false,
                },
                Rule {
                    name: "Global Airports".to_string(),
                    keywords: vec![
                        "global airports".to_string(),
                        "global_airports".to_string(),
                        "x-plane landmarks".to_string(),
                    ],
                    score: 20,
                    is_exclusion: false,
                },
            ],
            fallback_score: 40,
        }
    }
}

pub struct BitNetModel {
    pub config: HeuristicsConfig,
    config_path: PathBuf,
}

impl Default for BitNetModel {
    fn default() -> Self {
        let config_path = Self::get_config_path();
        let config = Self::load_config(&config_path).unwrap_or_default();
        Self {
            config,
            config_path,
        }
    }
}

impl BitNetModel {
    pub fn new() -> Result<Self> {
        Ok(Self::default())
    }

    fn get_config_path() -> PathBuf {
        ProjectDirs::from("com", "x-adox", "X-Addon-Oxide")
            .map(|dirs| dirs.config_dir().join("heuristics.json"))
            .unwrap_or_else(|| PathBuf::from("heuristics.json"))
    }

    fn load_config(path: &Path) -> Result<HeuristicsConfig> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(HeuristicsConfig::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn reset_defaults(&mut self) -> Result<()> {
        self.config = HeuristicsConfig::default();
        self.save()
    }

    /// Predicts the scenery priority score (0-100) based on the pack name and path.
    /// Lower score = higher priority.
    pub fn predict(&self, name: &str, _path: &Path) -> u8 {
        let name_lower = name.to_lowercase();

        // Detection for common airport patterns (still somewhat hardcoded as a base logic)
        let has_airport_keyword = name_lower.contains("airport")
            || name_lower.contains("apt")
            || name_lower.contains("airfield")
            || name_lower.contains("heliport")
            || name_lower.contains("seaplane");

        let is_major_dev = name_lower.contains("aerosoft")
            || name_lower.contains("justsim")
            || name_lower.contains("flytampa")
            || name_lower.contains("boundless")
            || name_lower.contains("taimodels")
            || name_lower.contains("nimbus")
            || name_lower.contains("axonos")
            || name_lower.contains("skyline")
            || name_lower.contains("fly2high")
            || name_lower.contains("skyhigh")
            || name_lower.contains("orbx");

        let has_icao = name.split(|c: char| !c.is_alphanumeric()).any(|word| {
            word.len() == 4 && word.chars().all(|c| c.is_alphabetic() && c.is_uppercase())
        });

        let is_airport = has_airport_keyword || is_major_dev || has_icao;

        // Custom Airport check (hardcoded threshold for now, but will be rule-based below)
        if is_airport && !name_lower.contains("overlay") {
            // We'll check rules first, but if none match, airports are usually 10
        }

        for rule in &self.config.rules {
            let matches = rule.keywords.iter().any(|k| name_lower.contains(k));
            if matches {
                if rule.is_exclusion {
                    if !is_airport {
                        return rule.score;
                    }
                } else {
                    return rule.score;
                }
            }
        }

        // Final airport fallback if no rule caught it
        if is_airport && !name_lower.contains("overlay") {
            return 10;
        }

        // Fallback for Z-prefix or other
        if name_lower.starts_with('z') || name_lower.starts_with('y') {
            return 50;
        }

        self.config.fallback_score
    }
}
