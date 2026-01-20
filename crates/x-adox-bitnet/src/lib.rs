use anyhow::Result;
use directories::ProjectDirs;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
pub mod parser;

#[derive(Debug, Clone, Default)]
pub struct PredictContext {
    pub region_focus: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct Rule {
    pub name: String,
    pub keywords: Vec<String>,
    pub score: u8,
    #[serde(default)]
    pub is_exclusion: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct HeuristicsConfig {
    pub rules: Vec<Rule>,
    pub fallback_score: u8,
    #[serde(default)]
    pub overrides: std::collections::BTreeMap<String, u8>,
    #[serde(default)]
    pub aircraft_overrides: std::collections::BTreeMap<String, Vec<String>>,
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
            overrides: std::collections::BTreeMap::new(),
            aircraft_overrides: std::collections::BTreeMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct BitNetModel {
    pub config: Arc<HeuristicsConfig>,
    config_path: PathBuf,
    regex_set: Option<RegexSet>,
}

impl Default for BitNetModel {
    fn default() -> Self {
        let config_path = Self::get_config_path();
        let config = Self::load_config(&config_path).unwrap_or_default();
        let regex_set = Self::build_regex_set(&config);
        Self {
            config: Arc::new(config),
            config_path,
            regex_set,
        }
    }
}

impl BitNetModel {
    pub fn new() -> Result<Self> {
        Ok(Self::default())
    }

    pub fn update_config(&mut self, config: HeuristicsConfig) {
        self.regex_set = Self::build_regex_set(&config);
        self.config = Arc::new(config);
    }

    pub fn refresh_regex_set(&mut self) {
        self.regex_set = Self::build_regex_set(&self.config);
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

    fn build_regex_set(config: &HeuristicsConfig) -> Option<RegexSet> {
        let mut patterns = Vec::new();
        for rule in &config.rules {
            for keyword in &rule.keywords {
                patterns.push(regex::escape(&keyword.to_lowercase()));
            }
        }
        if patterns.is_empty() {
            None
        } else {
            RegexSet::new(patterns).ok()
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self.config.as_ref())?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn reset_defaults(&mut self) -> Result<()> {
        self.config = Arc::new(HeuristicsConfig::default());
        self.regex_set = Self::build_regex_set(&self.config);
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

        if let Some(set) = &self.regex_set {
            if set.is_match(&name_lower) {
                let matches = set.matches(&name_lower);
                let mut current_idx = 0;
                for rule in &self.config.rules {
                    let end_idx = current_idx + rule.keywords.len();
                    if (current_idx..end_idx).any(|i| matches.matched(i)) {
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
                    current_idx = end_idx;
                }
            }
        } else {
            // Fallback to iterative matching if RegexSet is not built
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
        // 1. Check for manual overrides first
        if let Some(tags) = self.config.aircraft_overrides.get(name) {
            return tags.clone();
        }

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
        // Helper to check for keywords in a list
        let matches_any = |keywords: &[&str]| keywords.iter().any(|&k| name_lower.contains(k));

        let is_boeing = matches_any(&[
            "boeing", "b707", "b717", "b727", "b737", "b747", "b757", "b767", "b777", "b787",
            "707-", "717-", "727-", "737-", "747-", "757-", "767-", "777-", "787-", "b-17", "b-29",
            "b-52", "c-17", "f-15", "f/a-18", "apache", "chinook",
        ]);
        let is_airbus = matches_any(&[
            "airbus",
            "a300",
            "a310",
            "a318",
            "a319",
            "a320",
            "a321",
            "a330",
            "a340",
            "a350",
            "a380",
            "a220",
            "beluga",
            "a400m",
            "c295",
            "h125",
            "h135",
            "h145",
            "h160",
            "h175",
            "h225",
            "eurofighter",
            "typhoon",
        ]);
        let is_mcdonnell = matches_any(&[
            "mcdonnell",
            "douglas",
            "md-11",
            "md-8",
            "md-9",
            "dc-3",
            "dc-4",
            "dc-6",
            "dc-8",
            "dc-9",
            "dc-10",
            "kc-10",
            "a-4",
            "f-4",
            "phantom",
        ]);
        let is_bombardier = matches_any(&[
            "bombardier",
            "crj",
            "challenger",
            "global express",
            "global 5000",
            "global 6",
            "global 7",
            "global 8",
            "learjet",
            "dash 8",
            "q400",
        ]);
        let is_embraer = matches_any(&[
            "embraer", "erj", "e170", "e175", "e190", "e195", "phenom", "legacy", "praetor",
            "tucano", "emb-", "emb 110", "emb 120",
        ]);
        let is_lockheed = matches_any(&[
            "lockheed",
            "c-130",
            "hercules",
            "f-16",
            "f-22",
            "raptor",
            "f-35",
            "l-1011",
            "tristar",
            "constellation",
            "p-3",
            "orion",
            "u-2",
        ]);
        let is_cessna = matches_any(&[
            "cessna", "c150", "c152", "c172", "c182", "c206", "c208", "c210", "c310", "c340",
            "c402", "c404", "c421", "citation", "caravan",
        ]);
        let is_piper = matches_any(&[
            "piper", "pa-18", "pa-28", "pa-31", "pa-32", "pa-34", "pa-44", "pa-46", "archer",
            "warrior", "seneca", "seminole", "navajo", "cheyenne", "malibu", "meridian",
        ]);
        let is_beech = matches_any(&[
            "beech", "kingair", "king air", "baron", "bonanza", "be36", "be58", "be200", "be300",
            "be350", "be90", "be99", "be1900",
        ]);
        let is_gulfstream = matches_any(&[
            "gulfstream",
            " g-",
            "giv",
            "gv",
            "g450",
            "g550",
            "g650",
            "g700",
            "g800",
        ]);
        let is_de_havilland = matches_any(&[
            "de havilland",
            "dhc-2",
            "dhc-3",
            "dhc-6",
            "dhc-8",
            "beaver",
            "otter",
            "twin otter",
            "dash 8",
            "mosquito",
            "comet",
        ]);
        let is_fokker = matches_any(&["fokker", "f27", "f50", "f70", "f100"]);
        let is_tupolev = matches_any(&[
            "tupolev", "tu-134", "tu-154", "tu-204", "tu-214", "tu-160", "tu-95",
        ]);
        let is_ilyushin = matches_any(&["ilyushin", "il-18", "il-62", "il-76", "il-86", "il-96"]);
        let is_antonov = matches_any(&[
            "antonov", "an-2", "an-12", "an-24", "an-26", "an-30", "an-32", "an-72", "an-74",
            "an-124", "an-225",
        ]);

        let is_mooney = matches_any(&["mooney", "m20"]);
        let is_cirrus = matches_any(&["cirrus", "sr20", "sr22", "sf50"]);
        let is_diamond = matches_any(&["diamond", "da20", "da40", "da42", "da62"]);
        let is_socata = matches_any(&[
            "socata", "tbm700", "tbm850", "tbm900", "tbm910", "tbm930", "tbm940", "tbm960",
        ]);
        let is_robin = matches_any(&["robin", "dr400"]);
        let is_vans = matches_any(&[
            "van's", "rv-4", "rv-6", "rv-7", "rv-8", "rv-9", "rv-10", "rv-12", "rv-14",
        ]);
        let is_pilatus =
            matches_any(&["pilatus", "pc-6", "pc-7", "pc-9", "pc-12", "pc-21", "pc-24"]);
        let is_icon = matches_any(&["icon", " a5"]);
        let is_flight_design = matches_any(&["flight design", "ctsw", "ctls"]);

        // (Removed early pushes - moved to Step 7)

        // --- Step 2: Detection Pass 1: Broad Category (Special Roles) ---
        let is_helicopter = matches_any(&[
            "helicopter",
            "rotor",
            "bell",
            "aw139",
            "ec135",
            "bk117",
            "cabri",
            "sikorsky",
            "robinson",
            "r22",
            "r44",
            "r66",
            "guimbal",
            "eurocopter",
            "airbus helicopters",
            "as350",
            "h125",
            "h135",
            "h145",
        ]);
        let is_glider = matches_any(&[
            "glider",
            "sailplane",
            "ask21",
            "ls8",
            "discus",
            "schleicher",
            "schempp-hirth",
            "dg flugzeugbau",
        ]);

        let is_military = matches_any(&[
            "military",
            "fighter",
            "bomber",
            "tanker",
            "awacs",
            "trainer",
            "usaf",
            "navy",
            "royal air force",
            "luftwaffe",
            "f-4",
            "f-5",
            "f-8",
            "f-14",
            "f-15",
            "f-16",
            "f-18",
            "f-22",
            "f-35",
            "f-104",
            "f-117",
            "b-1b",
            "rockwell b-1",
            "b-2 spirit",
            "b-17",
            "b-24",
            "b-25",
            "b-29",
            "b-47",
            "b-52",
            "b-58",
            "c-5",
            "c-17",
            "c-47",
            "c-130",
            "kc-10",
            "kc-135",
            "douglas a-4",
            "mcdonnell a-4",
            "skyhawk a-4",
            "a-6",
            "a-10",
            "av-8b",
            "sr-71",
            "u-2",
            "t-6 texan",
            "t-33",
            "t-37",
            "t-38",
            "t-45",
            "t-6a",
            "mig-",
            "su-2",
            "su-3",
            "su-5",
            "tu-95",
            "tu-160",
            // "il-76", // Removed to favor Airliner/Cargo tagging (IL-76 is often used as freighter)
            // "an-12", // Removed to favor Airliner/Cargo
            "an-124",
            "an-225",
            "spitfire",
            "hurricane",
            "mustang",
            "corsair",
            "zero",
            "messerschmitt",
            "fw190",
            "bf109",
            "me262",
            "eurofighter",
            "typhoon",
            "tornado",
            "rafale",
            "mirage",
            "gripen",
            "viggen",
            "draken",
            "vulcan",
            "victor",
            "valiant",
            "harrier",
            "hawk t1",
            "hawk t2",
            "bae hawk",
            "apache",
            "chinook",
            "blackhawk",
            "cobra",
            "hind",
            "mi-8",
            "mi-24",
        ]);

        // --- Step 3: Detection Pass 2: Propulsion ---
        let is_known_jet_model = matches_any(&[
            // Airliners
            "b707",
            "b717",
            "b727",
            "b737",
            "b747",
            "b757",
            "b767",
            "b777",
            "b787",
            "a300",
            "a310",
            "a318",
            "a319",
            "a320",
            "a321",
            "a330",
            "a340",
            "a350",
            "a380",
            "dc-8",
            "dc-9",
            "dc-10",
            "md-11",
            "md-80",
            "md-90",
            "md-95",
            "crj",
            "erj",
            "e170",
            "e175",
            "e190",
            "e195",
            "f70",
            "f100",
            "baebb",
            "rj85",
            "rj100",
            "tu-134",
            "tu-154",
            "tu-204",
            "il-62",
            "il-76",
            "il-86",
            "il-96",
            "yak-40",
            "yak-42",
            "concorde",
            "trident",
            "comet",
            "caravelle",
            "mercure",
            "vfw-614",
            "l-1011",
            "tristar",
            // BizJets
            "citation",
            "lear",
            "learjet",
            "gulfstream",
            "falcon",
            "challenger",
            "global express",
            "global 5000",
            "phenom",
            "premier",
            "hawker",
            "hondajet",
            "cirrus sf50",
            "eclipse 500",
            "mustang",
            // Military Jets provided in is_military check mostly cover this, but explicitly:
            "f-15",
            "f-16",
            "f-18",
            "f-22",
            "f-35",
            "b-1b",
            "b-2",
            "b-52",
            "sr-71",
            "me262",
            "vulcan",
            "tornado",
        ]);

        let is_jet = name_lower.contains("jet") || is_known_jet_model;

        let is_known_turboprop_model = matches_any(&[
            "king air",
            "kingair",
            "b1900",
            "b200",
            "b350",
            "c90",
            "pc-12",
            "pc-6",
            "pc-7",
            "pc-9",
            "pc-21",
            "tbm",
            "kodiak",
            "caravan",
            "c208",
            "dash 8",
            "q400",
            "dhc-6",
            "twin otter",
            "dhc-2",
            "beaver",
            "otter", // DHC-2 Beaver is technically piston usually but DHC-2T exists. Regular Beaver is Piston.
            "atr",
            "f27",
            "f50",
            "l-188",
            "electra",
            "c-130",
            "hercules",
            "an-12",
            "an-24",
            "an-26",
            "an-32",
            "il-18",
            "tu-95",
            "sf340",
            "s2000",
            "js41",
            "js31",
            "mu-2",
            "commander",
            "cheyenne",
            "merlin",
            "metro",
        ]);

        let is_turboprop = name_lower.contains("turboprop") || is_known_turboprop_model;

        // Redefine Prop to be explicit about pistons if possible, or fallback
        // These are strictly PISTON props (usually)
        let is_known_piston = matches_any(&[
            "c150",
            "c152",
            "c172",
            "c182",
            "c206",
            "c210",
            "c310",
            "pa-18",
            "pa-28",
            "pa-32",
            "pa-34",
            "pa-44",
            "archer",
            "warrior",
            "seneca",
            "seminole",
            "bonanza",
            "baron",
            "mooney",
            "sr20",
            "sr22",
            "da20",
            "da40",
            "da62",
            "dr400",
            "tb10",
            "tb20",
            "rv-",
            "cub",
            "scout",
            "decathlon",
            "dc-3",
            "dc-4",
            "dc-6",
            "c-47",
            "constellation",
            "p-51",
            "spitfire",
            "b-17",
        ]);

        // --- Step 4: Detection Pass 3: Operational Role ---
        let is_airliner = is_boeing
            || is_airbus
            || is_lockheed
            || is_mcdonnell
            || is_fokker
            || is_bombardier
            || is_embraer
            || is_tupolev
            || is_ilyushin
            || is_antonov
            || matches_any(&[
                "atr",
                "dash 8",
                "q400",
                "crj",
                "erj",
                "saab",
                "bae",
                "concorde",
                "trident",
                "comet",
                "caravelle",
                "mercure",
                "lufthansa",
                "air france",
                "british airways",
                "delta",
                "united",
                "american",
                "klm",
                "airliner",
                "airways",
                "express",
                "cargo",
                "freight",
            ]);

        let is_bizjet = matches_any(&[
            "citation",
            "lear",
            "gulfstream",
            "challenger",
            "global",
            "falcon",
            "hondajet",
            "phenom",
            "hawker",
            "bizjet",
            "business jet",
        ]);

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
                // If not jet/turboprop, assume Prop for military (WW2, Trainers)
                tags.push("Prop".to_string());
            }
        } else {
            // Operational Role tagging: Airliner vs General Aviation
            // Strict exclusion: If it's an Airliner model, it is NOT GA.

            // BizJets are GA per user rules, but we tag them as BizJet + GA

            let likely_airliner = is_airliner || (is_jet && !is_bizjet);

            if likely_airliner {
                tags.push("Airliner".to_string());
                if is_jet {
                    tags.push("Jet".to_string());
                } else if is_turboprop {
                    tags.push("Turboprop".to_string());
                } else if is_known_piston {
                    tags.push("Prop".to_string()); // DC-3/DC-6 etc
                } else {
                    // Default Airliner to Jet if unknown? Or Prop?
                    // Most unknown airliners in sim context are likely jets or turboprops.
                    // Let's stick to explicitly detected propulsion or generic fallback
                    if name_lower.contains("prop") {
                        tags.push("Prop".to_string());
                    } else {
                        tags.push("Jet".to_string());
                    } // Fallback for "Airbus" -> Jet
                }
            } else {
                // General Aviation

                // If strictly unidentified (no matches at all), NO TAGS (as per "Unknown" removal)
                // BUT we need to check if we matched *anything* to call it GA.
                // We shouldn't default to GA if we just don't know what it is.

                let is_positively_ga = is_cessna
                    || is_piper
                    || is_beech
                    || is_mooney
                    || is_cirrus
                    || is_diamond
                    || is_socata
                    || is_robin
                    || is_vans
                    || is_pilatus
                    || is_bizjet
                    || is_known_piston
                    || is_known_turboprop_model;

                if is_positively_ga {
                    tags.push("General Aviation".to_string());
                    if is_bizjet {
                        tags.push("Business Jet".to_string());
                        tags.push("Jet".to_string());
                    } else if is_jet {
                        // Cirrus Jet etc
                        tags.push("Jet".to_string());
                    } else if is_turboprop {
                        tags.push("Turboprop".to_string());
                    } else {
                        tags.push("Prop".to_string());
                    }
                } else {
                    // Truly unidentified.
                    // Just try to apply propulsion if we know it (e.g. from "jet" keyword)
                    if is_jet {
                        tags.push("Jet".to_string());
                    } else if is_turboprop {
                        tags.push("Turboprop".to_string());
                    } else if name_lower.contains("prop") {
                        tags.push("Prop".to_string());
                    }
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

        // --- Step 7: Apply Manufacturer Tags (Consolidated) ---
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
        if is_lockheed {
            tags.push("Lockheed".to_string());
        }
        if is_de_havilland {
            tags.push("De Havilland".to_string());
        }
        if is_socata {
            tags.push("Socata".to_string());
        }
        if is_pilatus {
            tags.push("Pilatus".to_string());
        }
        if is_fokker {
            tags.push("Fokker".to_string());
        }
        if is_gulfstream {
            tags.push("Gulfstream".to_string());
        }
        if is_tupolev {
            tags.push("Tupolev".to_string());
        }
        if is_ilyushin {
            tags.push("Ilyushin".to_string());
        }
        if is_antonov {
            tags.push("Antonov".to_string());
        }
        if is_icon {
            tags.push("Icon".to_string());
        }
        if is_flight_design {
            tags.push("Flight Design".to_string());
        }
        if is_robin {
            tags.push("Robin".to_string());
        }
        if is_vans {
            tags.push("Van's".to_string());
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
            config: Arc::new(HeuristicsConfig::default()),
            config_path: PathBuf::from("test_heuristics.json"),
            regex_set: None,
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
            config: Arc::new(HeuristicsConfig::default()),
            config_path: PathBuf::from("test_heuristics.json"),
            regex_set: None,
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
    fn test_predict_tags_historical_prop_airliner() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Douglas DC-6 Cloudmaster", Path::new("test"));
        assert!(tags.contains(&"McDonnell Douglas".to_string()));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Prop".to_string()));
        assert!(!tags.contains(&"Jet".to_string()));
        assert!(!tags.contains(&"General Aviation".to_string()));
    }

    #[test]
    fn test_predict_tags_historical_bomber() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Boeing B-17 Flying Fortress", Path::new("test"));
        assert!(tags.contains(&"Boeing".to_string()));
        assert!(tags.contains(&"Military".to_string()));
        assert!(tags.contains(&"Prop".to_string()));
        assert!(!tags.contains(&"Jet".to_string()));
    }

    #[test]
    fn test_predict_tags_modern_bomber() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Boeing B-52 Stratofortress", Path::new("test"));
        assert!(tags.contains(&"Boeing".to_string()));
        assert!(tags.contains(&"Military".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
    }

    #[test]
    fn test_predict_tags_regional_turboprop() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Bombardier Dash 8 Q400", Path::new("test"));
        assert!(tags.contains(&"Bombardier".to_string()));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Turboprop".to_string()));
        assert!(!tags.contains(&"Prop".to_string()));
    }

    #[test]
    fn test_predict_tags_new_manufacturer_lockheed() {
        let model = BitNetModel::default();
        // L-1011 TriStar is a jet airliner
        let tags = model.predict_aircraft_tags("Lockheed L-1011 TriStar", Path::new("test"));
        assert!(tags.contains(&"Lockheed".to_string()));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
    }

    #[test]
    fn test_predict_tags_helicopter_specific() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Airbus H135", Path::new("test"));
        assert!(tags.contains(&"Airbus".to_string()));
        assert!(tags.contains(&"Helicopter".to_string()));
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

    #[test]
    fn test_predict_tags_boeing_707() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("Boeing 707", Path::new("test"));
        assert!(tags.contains(&"Boeing".to_string()));
        assert!(tags.contains(&"Airliner".to_string()));
        assert!(tags.contains(&"Jet".to_string()));
    }

    #[test]
    fn test_predict_tags_standalone_707() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("707-320C", Path::new("test"));
        assert!(tags.contains(&"Boeing".to_string()));
        assert!(tags.contains(&"Airliner".to_string()));
    }

    #[test]
    fn test_predict_tags_airbus_standalone_320() {
        let model = BitNetModel::default();
        let tags = model.predict_aircraft_tags("A320neo", Path::new("test"));
        assert!(tags.contains(&"Airbus".to_string()));
        assert!(tags.contains(&"Airliner".to_string()));
    }

    #[test]
    fn test_predict_tags_manual_override() {
        let mut model = BitNetModel {
            config: Arc::new(HeuristicsConfig::default()),
            config_path: PathBuf::from("test_heuristics_override.json"),
            regex_set: None,
        };

        // Before override
        let tags = model.predict_aircraft_tags("Boeing 737", Path::new("test"));
        assert!(tags.contains(&"Boeing".to_string()));
        assert!(tags.contains(&"Airliner".to_string()));

        // Set override
        Arc::make_mut(&mut model.config).aircraft_overrides.insert(
            "Boeing 737".to_string(),
            vec!["Military".to_string(), "Jet".to_string()],
        );

        // After override
        let tags_after = model.predict_aircraft_tags("Boeing 737", Path::new("test"));
        assert_eq!(tags_after.len(), 2);
        assert!(tags_after.contains(&"Military".to_string()));
        assert!(tags_after.contains(&"Jet".to_string()));
        assert!(!tags_after.contains(&"Boeing".to_string()));
        assert!(!tags_after.contains(&"Airliner".to_string()));
    }
}
