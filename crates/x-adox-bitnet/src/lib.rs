use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
pub mod parser;

#[derive(Debug, Clone, Default)]
pub struct PredictContext {
    pub region_focus: Option<String>,
}

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
    #[serde(default)]
    pub overrides: std::collections::HashMap<String, u8>,
}

impl Default for HeuristicsConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                Rule {
                    name: "Major Airports (PANC, etc.)".to_string(),
                    keywords: vec!["panc".to_string(), "anchorage".to_string()],
                    score: 10,
                    is_exclusion: false,
                },
                Rule {
                    name: "SimHeaven / X-World".to_string(),
                    keywords: vec![
                        "simheaven".to_string(),
                        "x-world".to_string(),
                        "w2xp".to_string(),
                    ],
                    score: 15, // Above Global Airports (20)
                    is_exclusion: false,
                },
                Rule {
                    name: "AutoOrtho Overlays".to_string(),
                    keywords: vec!["yautoortho".to_string(), "y_autoortho".to_string()],
                    score: 42,
                    is_exclusion: false,
                },
                Rule {
                    name: "AutoOrtho Base".to_string(),
                    keywords: vec!["z_autoortho".to_string(), "z_ao_".to_string()],
                    score: 95,
                    is_exclusion: false,
                },
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
                    score: 25,
                    is_exclusion: false,
                },
                Rule {
                    name: "Orbx B / TrueEarth".to_string(),
                    keywords: vec!["orbx_b".to_string(), "trueearth_overlay".to_string()],
                    score: 35,
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
                    score: 31,
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
            overrides: std::collections::HashMap::new(),
        }
    }
}

#[derive(Clone)]
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
    pub fn predict(&self, name: &str, _path: &Path, context: &PredictContext) -> u8 {
        // 1. Check for manual overrides first (Sticky Sort)
        if let Some(&score) = self.config.overrides.get(name) {
            return score;
        }

        let name_lower = name.to_lowercase();

        // DEBUG: Print current rules count
        if name.contains("autoortho") {
            println!(
                "[BitNet] Debug: {} rules loaded. First rule: {}",
                self.config.rules.len(),
                self.config.rules[0].name
            );
        }

        // Detection for common airport patterns (still somewhat hardcoded as a base logic)
        let has_airport_keyword = name_lower.contains("airport")
            || name_lower.contains("apt")
            || name_lower.contains("airfield")
            || name_lower.contains("heliport")
            || name_lower.contains("seaplane")
            || name_lower.contains("anchorage")
            || name_lower.contains("panc");

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
            || name_lower.contains("orbx")
            || name_lower.contains("x-scenery");

        let has_icao = name.split(|c: char| !c.is_alphanumeric()).any(|word| {
            word.len() == 4
                && word.chars().all(|c| c.is_alphabetic())
                && (word.chars().all(|c| c.is_uppercase()) || name_lower.starts_with(word))
        });

        let is_airport = has_airport_keyword || is_major_dev || has_icao;

        let mut score = None;

        for rule in &self.config.rules {
            let matches = rule.keywords.iter().any(|k| name_lower.contains(k));
            if matches {
                if rule.is_exclusion {
                    if !is_airport {
                        score = Some(rule.score);
                        break;
                    }
                } else {
                    score = Some(rule.score);
                    break;
                }
            }
        }

        let mut final_score = if let Some(s) = score {
            s
        } else if is_airport && !name_lower.contains("overlay") {
            10
        } else if name_lower.starts_with('z') || name_lower.starts_with('y') {
            50
        } else {
            self.config.fallback_score
        };

        // Pro Mode: Region Biasing
        if let Some(focus) = &context.region_focus {
            if name_lower.contains(&focus.to_lowercase()) {
                // Boost priority by 1 (lower score) if it matches the focus region
                final_score = final_score.saturating_sub(1);
            }
        }

        final_score
    }
    /// Predicts aircraft tags based on name and path.
    pub fn predict_aircraft_tags(&self, name: &str, path: &Path) -> Vec<String> {
        let mut tags = Vec::new();
        let mut text_to_check = name.to_lowercase();

        // Scan folder for .acf files to get more context
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let filename = entry.file_name().to_string_lossy().to_lowercase();
                if filename.ends_with(".acf") {
                    text_to_check.push(' ');
                    text_to_check.push_str(&filename);
                }
            }
        }

        let name_lower = text_to_check;

        // --- Step 0: Try Native Parsing ---
        // Attempt to parse the .acf file to get definitive data
        let parsed_data = parser::parse_acf_in_dir(path).ok();

        if parsed_data.is_some() {
            println!("[BitNet] Successfully parsed ACF for path: {:?}", path);
        }

        let mut definitive_prop_type = None;
        if let Some(data) = &parsed_data {
            // Use description/author for keywords if available
            if !data.description.is_empty() {
                // Append description to text check for better context
                // (e.g. if desc says "USAF Bomber", we catch it)
            }

            if let Some(pt) = &data.prop_type {
                definitive_prop_type = Some(pt.clone());
            }
        }

        // --- Step 1: Detect Manufacturers & Core Brands ---
        let is_boeing = name_lower.contains("boeing") || name_lower.contains("b7");
        let is_airbus = name_lower.contains("airbus") || name_lower.contains("a3");
        let is_cessna = name_lower.contains("cessna")
            || name_lower.contains("c172")
            || name_lower.contains("c152")
            || name_lower.contains("c208");
        let is_piper = name_lower.contains("piper")
            || name_lower.contains("pa-")
            || name_lower.contains("archer")
            || name_lower.contains("warrior");
        let is_beech = name_lower.contains("beech")
            || name_lower.contains("kingair")
            || name_lower.contains("baron")
            || name_lower.contains("bonanza");
        let is_mooney = name_lower.contains("mooney");
        let is_cirrus = name_lower.contains("cirrus")
            || name_lower.contains("sr22")
            || name_lower.contains("sf50");
        let is_diamond = name_lower.contains("diamond")
            || name_lower.contains("da40")
            || name_lower.contains("da42")
            || name_lower.contains("da62");
        let is_embraer = name_lower.contains("embraer")
            || name_lower.contains("erj")
            || name_lower.contains("e1")
            || name_lower.contains("e2")
            || name_lower.contains("phenom");
        let is_bombardier = name_lower.contains("bombardier")
            || name_lower.contains("crj")
            || name_lower.contains("challenger")
            || name_lower.contains("global");
        let is_mcdonnell = name_lower.contains("mcdonnell")
            || name_lower.contains("douglas")
            || name_lower.contains("md-")
            || name_lower.contains("dc-");
        let is_gulfstream = name_lower.contains("gulfstream")
            || name_lower.contains("g-")
            || name_lower.contains("g550")
            || name_lower.contains("g650");

        let is_socata = name_lower.contains("socata") || name_lower.contains("tbm");
        let is_robin = name_lower.contains("robin") || name_lower.contains("dr400");
        let is_vans = name_lower.contains("van's") || name_lower.contains("rv-");
        let is_pilatus = name_lower.contains("pilatus")
            || name_lower.contains("pc-")
            || name_lower.contains("pc12")
            || name_lower.contains("pc24");
        let is_icon = name_lower.contains("icon") && name_lower.contains("a5");
        let is_flight_design = name_lower.contains("flight design")
            || name_lower.contains("ctsw")
            || name_lower.contains("ctls");

        if is_boeing {
            tags.push("Boeing".to_string());
        }
        if is_airbus {
            tags.push("Airbus".to_string());
        }
        if is_cessna {
            tags.push("Cessna".to_string());
        }
        if is_piper {
            tags.push("Piper".to_string());
        }
        if is_beech {
            tags.push("Beechcraft".to_string());
        }
        if is_mooney {
            tags.push("Mooney".to_string());
        }
        if is_diamond {
            tags.push("Diamond".to_string());
        }
        if is_cirrus {
            tags.push("Cirrus".to_string());
        }
        if is_embraer {
            tags.push("Embraer".to_string());
        }
        if is_bombardier {
            tags.push("Bombardier".to_string());
        }
        if is_mcdonnell {
            tags.push("McDonnell Douglas".to_string());
        }

        // --- Step 2: Detection Pass 1: Broad Category (Special Roles) ---
        let is_helicopter = name_lower.contains("helicopter")
            || name_lower.contains("rotor")
            || name_lower.contains("bell")
            || name_lower.contains("aw139")
            || name_lower.contains("ec135")
            || name_lower.contains("bk117")
            || name_lower.contains("cabri")
            || name_lower.contains("sikorsky");
        let is_glider = name_lower.contains("glider")
            || name_lower.contains("ask21")
            || name_lower.contains("ls8")
            || name_lower.contains("discus");
        let is_military = name_lower.contains("military")
            || name_lower.contains("fighter")
            || name_lower.contains("bomber")
            || name_lower.contains("tanker")
            || name_lower.contains("awacs")
            || name_lower.contains("f-")
            || name_lower.contains("mig-")
            || name_lower.contains("su-")
            || name_lower.contains("spitfire")
            || name_lower.contains("mustang")
            || name_lower.contains("p-51")
            || name_lower.contains("t-6")
            || name_lower.contains("j-")
            || name_lower.contains("vulcan")
            || name_lower.contains("mirage")
            || name_lower.contains("rafale")
            || name_lower.contains("eurofighter")
            || name_lower.contains("typhoon")
            || name_lower.contains("tornado")
            || name_lower.contains("harrier");

        // --- Step 3: Detection Pass 2: Propulsion ---
        let is_jet = name_lower.contains("jet")
            || name_lower.contains("citation")
            || name_lower.contains("lear")
            || is_boeing
            || is_airbus
            || is_gulfstream
            || name_lower.contains("crj")
            || name_lower.contains("erj")
            || name_lower.contains("fokker")
            || name_lower.contains("tupolev")
            || name_lower.contains("il-")
            || name_lower.contains("concorde")
            || name_lower.contains("trident")
            || name_lower.contains("comet")
            || name_lower.contains("caravelle")
            || name_lower.contains("md-")
            || name_lower.contains("dc-")
            || name_lower.contains("f-1")
            || name_lower.contains("f-2")
            || name_lower.contains("f-3")
            || name_lower.contains("f-8")
            || name_lower.contains("f-22")
            || name_lower.contains("mig-")
            || name_lower.contains("su-")
            || name_lower.contains("vulcan")
            || name_lower.contains("mirage")
            || name_lower.contains("rafale")
            || name_lower.contains("eurofighter")
            || name_lower.contains("typhoon")
            || name_lower.contains("tornado")
            || name_lower.contains("harrier");
        let is_turboprop = name_lower.contains("turboprop")
            || name_lower.contains("kingair")
            || name_lower.contains("king air")
            || name_lower.contains("tbm")
            || name_lower.contains("pc12")
            || name_lower.contains("pc-12")
            || name_lower.contains("q400")
            || name_lower.contains("dhc-")
            || name_lower.contains("atr")
            || name_lower.contains("caravan")
            || name_lower.contains("c208")
            || name_lower.contains("kodiak")
            || name_lower.contains("ansel")
            || name_lower.contains("an-")
            || name_lower.contains("t-6")
            || name_lower.contains("t6");

        // --- Step 4: Detection Pass 3: Operational Role ---
        let is_airliner = is_boeing
            || is_airbus
            || name_lower.contains("md-")
            || name_lower.contains("dc-10")
            || name_lower.contains("dc-8")
            || name_lower.contains("dc-9")
            || name_lower.contains("fokker")
            || name_lower.contains("q400")
            || name_lower.contains("atr")
            || name_lower.contains("crj")
            || name_lower.contains("e17")
            || name_lower.contains("e19")
            || name_lower.contains("ba-")
            || name_lower.contains("lufthansa")
            || name_lower.contains("air france")
            || name_lower.contains("delta")
            || name_lower.contains("united")
            || name_lower.contains("southwest")
            || name_lower.contains("ryanair")
            || name_lower.contains("emirates")
            || name_lower.contains("concorde")
            || name_lower.contains("trident")
            || name_lower.contains("comet")
            || name_lower.contains("caravelle")
            || name_lower.contains("tupolev")
            || name_lower.contains("tu-")
            || name_lower.contains("ilyushin")
            || name_lower.contains("il-")
            || name_lower.contains("yak-")
            || name_lower.contains("antonov")
            || name_lower.contains("an-")
            || (name_lower.contains("sukhoi") && name_lower.contains("superjet"))
            || name_lower.contains("bac")
            || name_lower.contains("vickers")
            // Strict Commercial Keywords
            || name_lower.contains("cargo")
            || name_lower.contains("transport")
            || name_lower.contains("freight")
            || name_lower.contains("express")
            || name_lower.contains("airways")
            || name_lower.contains("airlines")
            || name_lower.contains("trans")
            || name_lower.contains("international");

        let is_bizjet = name_lower.contains("citation")
            || name_lower.contains("lear")
            || name_lower.contains("gulfstream")
            || name_lower.contains("challenger")
            || name_lower.contains("global")
            || name_lower.contains("falcon")
            || name_lower.contains("hondajet")
            || name_lower.contains("phenom");

        // --- Step 5: Assign Final Tags ---
        if is_helicopter {
            tags.push("Helicopter".to_string());
        } else if is_glider {
            tags.push("Glider".to_string());
        } else if is_military {
            tags.push("Military".to_string());
            if is_jet {
                tags.push("Jet".to_string());
            } else if is_turboprop {
                tags.push("Turboprop".to_string());
            } else {
                tags.push("Prop".to_string());
            }
        } else {
            // Operational Role tagging: Airliner vs General Aviation
            // Note: Per user request, Business Jets are part of General Aviation.

            // STRICT EXCLUSION: If it looks like a jet but isn't a BizJet, assume Airliner/Commercial.
            // This prevents "Unknown Jets" (which are usually airliners) from falling into GA.
            let likely_airliner = is_airliner || (is_jet && !is_bizjet);

            // Define "Positively GA" to avoid fallback for unknowns
            let is_positively_ga = is_cessna
                || is_piper
                || is_beech
                || is_mooney
                || is_diamond
                || is_cirrus
                || is_socata
                || is_robin
                || is_vans
                || is_pilatus
                || is_icon
                || is_flight_design
                || is_bizjet;

            if likely_airliner {
                tags.push("Airliner".to_string());
                if is_jet {
                    tags.push("Jet".to_string());
                } else if is_turboprop {
                    tags.push("Turboprop".to_string());
                } else if !is_airliner && !is_jet {
                    // Prop airliner
                    tags.push("Prop".to_string());
                } else {
                    tags.push("Prop".to_string());
                }
            } else if is_positively_ga {
                // Default to General Aviation for known GA types
                tags.push("General Aviation".to_string());

                if is_bizjet {
                    tags.push("Business Jet".to_string());
                }

                if is_jet {
                    tags.push("Jet".to_string());
                } else if is_turboprop {
                    tags.push("Turboprop".to_string());
                } else {
                    tags.push("Prop".to_string());
                }
            } else {
                // No specific category identified (not Airliner, Military, or known GA)
                // Do NOT tag as "Unknown" or "General Aviation". Just define propulsion.

                if is_jet {
                    tags.push("Jet".to_string());
                } else if is_turboprop {
                    tags.push("Turboprop".to_string());
                } else {
                    tags.push("Prop".to_string());
                }
            }
        }

        // --- Step 6: Apply Definitive Parser Overrides ---
        if let Some(pt) = definitive_prop_type {
            // Remove guessed propulsion tags first
            tags.retain(|t| t != "Jet" && t != "Prop" && t != "Turboprop" && t != "Electric");

            match pt {
                parser::PropType::LoBypassJet | parser::PropType::HiBypassJet => {
                    tags.push("Jet".to_string());
                }
                parser::PropType::FreeTurbine | parser::PropType::FixedTurbine => {
                    tags.push("Turboprop".to_string());
                }
                parser::PropType::RecipCarb | parser::PropType::RecipInjected => {
                    tags.push("Prop".to_string());
                }
                parser::PropType::Electric => {
                    tags.push("Prop".to_string()); // Treat electric as prop key for filtering for now
                }
                parser::PropType::Rocket | parser::PropType::TipRockets => {
                    tags.push("Jet".to_string()); // Rockets are closer to jets? Or distinct? Defaults to Jet for now.
                }
                _ => {}
            }

            // If checking for "Unknown" tags, we might want to resolve it?
            // If parsed successfully, is it still "Unknown"?
            // Maybe we trust the parser's logic for "Airliner" vs "GA" based on description?
            // For now, let's just fix the propulsion.
        }

        // Additional Context Tags
        if name_lower.contains("seaplane")
            || name_lower.contains("float")
            || name_lower.contains("amphibian")
        {
            tags.push("Seaplane".to_string());
        }

        tags.sort();
        tags.dedup();
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predict_panc() {
        let model = BitNetModel {
            config: HeuristicsConfig::default(),
            config_path: PathBuf::from("test_heuristics.json"),
        };
        let score = model.predict(
            "panc---anchorage-v2.0.2",
            Path::new("test"),
            &PredictContext::default(),
        );
        assert_eq!(
            score, 10,
            "PANC should be recognized as a high-priority airport"
        );
    }

    #[test]
    fn test_predict_simheaven_consistency() {
        let model = BitNetModel {
            config: HeuristicsConfig::default(),
            config_path: PathBuf::from("test_heuristics.json"),
        };
        let score1 = model.predict(
            "simHeaven_X-World_America-1-vfr",
            Path::new("test"),
            &PredictContext::default(),
        );
        let score2 = model.predict(
            "simHeaven_X-World_Europe-8-network",
            Path::new("test"),
            &PredictContext::default(),
        );
        assert_eq!(score1, 15);
        assert_eq!(score2, 15);
        assert_eq!(
            score1, score2,
            "SimHeaven layers should have the same score to allow alphabetical continent grouping"
        );
    }

    #[test]
    fn test_predict_tags_airliner() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Airbus A319 Lufthansa", Path::new("test"));
        assert!(tags.contains(&"Airbus".to_string()));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_bizjet() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Cessna Citation CJ4", Path::new("test"));
        assert!(tags.contains(&"Cessna".to_string()));
        assert!(tags.contains(&"Business Jet".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
        assert!(tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_ga_piston() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Cessna 172 Skyhawk", Path::new("test"));
        assert!(tags.contains(&"Cessna".to_string()));
        assert!(tags.contains(&"General Aviation".to_string()));
        assert!(tags.contains(&"Prop".to_string()));
        assert!(!tags.contains(&"Turboprop".to_string()));
    }

    #[test]
    fn test_predict_tags_ga_turboprop() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Beechcraft King Air 350", Path::new("test"));
        assert!(tags.contains(&"Beechcraft".to_string()));
        assert!(tags.contains(&"General Aviation".to_string()));
        assert!(tags.contains(&"Turboprop".to_string()));
    }

    #[test]
    fn test_predict_tags_military_jet() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("F-16 Fighting Falcon", Path::new("test"));
        assert!(tags.contains(&"Military".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
    }

    #[test]
    fn test_predict_tags_concorde() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("CONCORDE_FXP", Path::new("test"));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
        assert!(!tags.contains(&"Prop".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_trident() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Trident_2E", Path::new("test"));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
        assert!(!tags.contains(&"Prop".to_string()));
    }

    #[test]
    fn test_predict_tags_bizjet_is_ga() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Cessna Citation CJ4", Path::new("test"));
        assert!(tags.contains(&"General Aviation".to_string()));
        assert!(tags.contains(&"Business Jet".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
    }

    #[test]
    fn test_predict_tags_generic_cargo_jet() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Generic Cargo Jet", Path::new("test"));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_airways_express() {
        // Even if we don't know the plane, "Airways" implies commercial service
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Fake Airways Express", Path::new("test"));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_unknown_jet_safety_net() {
        // An unknown jet should default to Airliner, NOT GA
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Mystery Jet 2000", Path::new("test"));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_unknown_prop() {
        // A truly unknown prop should have NO category (not GA, not Unknown), just Prop
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Mystery Machine Prop", Path::new("test"));
        assert!(!tags.contains(&"Unknown".to_string()));
        assert!(tags.contains(&"Prop".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_vulcan_bomber() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Avro Vulcan B2", Path::new("test"));
        assert!(tags.contains(&"Military".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_il76_cargo() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Ilyushin Il-76", Path::new("test"));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_pc12_ga() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Pilatus PC-12", Path::new("test"));
        assert!(tags.contains(&"General Aviation".to_string()));
        assert!(tags.contains(&"Turboprop".to_string()));
    }

    #[test]
    fn test_predict_tags_with_acf_parsing() -> Result<()> {
        use std::io::Write;
        use tempfile::tempdir;

        // 1. Setup temp dir and acf file
        let dir = tempdir()?;
        let acf_path = dir.path().join("test.acf");
        let mut file = std::fs::File::create(&acf_path)?;

        // Write ACF header forcing it to be a Jet (1)
        writeln!(file, "I")?;
        writeln!(file, "1000 Version")?;
        writeln!(file, "P acf/_descrip Forced Jet Type")?;
        writeln!(file, "P acf/_engn/0/_type 4")?; // 4 = LoBypassJet

        // 2. Predict using a name that would normally default to Prop or Unknown
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Unknown Aircraft", dir.path());

        // 3. Verify that the parser override worked
        assert!(
            tags.contains(&"Jet".to_string()),
            "Should be detected as Jet from ACF"
        );
        assert!(!tags.contains(&"Prop".to_string()), "Should NOT be Prop");

        Ok(())
    }
}
