use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

static ALIAS_INDEX: OnceLock<HashMap<String, LocationConstraint>> = OnceLock::new();

#[derive(Deserialize)]
struct RawAlias {
    #[serde(rename = "type")]
    alias_type: String,
    value: serde_json::Value,
}

/// Words that introduce an aircraft clause (e.g. "on an MD-80").
const AIRCRAFT_CONNECTORS: &[&str] = &[
    "using",
    "in",
    "with",
    "on",
    "aboard",
    "taking",
    "flying",
    "piloting",
    "operating",
];

/// Words that terminate a location capture group.
/// Superset of AIRCRAFT_CONNECTORS plus directional/temporal words.
const LOCATION_TERMINATORS: &[&str] = &[
    "using",
    "in",
    "with",
    "on",
    "aboard",
    "taking",
    "flying",
    "piloting",
    "operating",
    "for",
    "via",
    "during",
    "at",
    "about",
    "around",
    "between",
    // Directional verbs — prevent "from F70 heading south" from eating the direction
    "heading",
    "going",
    "headed",
    "bound",
    // -bound compound words — "from EGLL northbound" must stop before "northbound"
    "northbound",
    "southbound",
    "eastbound",
    "westbound",
    "northeastbound",
    "southeastbound",
    "southwestbound",
    "northwestbound",
];

const WEATHER_TIME_WORDS: &[&str] = &[
    // Weather
    "vfr",
    "ifr",
    "storm",
    "rain",
    "snow",
    "fog",
    "mist",
    "haze",
    "clear",
    "sunny",
    "cloudy",
    "overcast",
    "gusty",
    "windy",
    "breezy",
    "calm",
    "turbulent",
    "turbulence",
    "stormy",
    "thunder",
    "thunderstorm",
    "lightning",
    "severe",
    "drizzle",
    "showers",
    "blizzard",
    "ice",
    // Time
    "night",
    "dark",
    "midnight",
    "dawn",
    "sunrise",
    "morning",
    "dusk",
    "sunset",
    "evening",
    "twilight",
    "day",
    "daytime",
    "daylight",
    "afternoon",
    "noon",
    // Compound/modifier forms
    "vfr conditions",
    "ifr conditions",
    "a storm",
    "the rain",
    "the dark",
    "the night",
    "heavy snow",
    "instrument",
    "visual",
    "clear skies",
    "bad weather",
    "good weather",
    "gusty conditions",
    "gusty winds",
    "windy conditions",
    "calm conditions",
    "stormy conditions",
    "clear weather",
];

/// Geographic feature words that, when captured by the aircraft regex, indicate the
/// text is a destination clause rather than an aircraft name.
/// E.g. "flying in the mountains" → destination, not aircraft="mountains".
/// Uses word-level matching (split_whitespace) to avoid partial-word false positives
/// like "hill" matching inside "churchill".
const GEOGRAPHIC_FEATURE_WORDS: &[&str] = &[
    "mountain",
    "mountains",
    "highlands",
    "highland",
    "hills",
    "hill",
    "fjord",
    "fjords",
    "valley",
    "valleys",
    "desert",
    "jungle",
    "rainforest",
    "forest",
    "coast",
    "coastal",
    "coastline",
    "plateau",
    "plains",
    "prairie",
    "savanna",
    "savannah",
    "steppe",
    "tundra",
    "glacier",
    "glaciers",
    "volcanic",
    "volcano",
    "canyon",
    "canyons",
    "outback",
    "wilderness",
    "archipelago",
    "islands",
    "isles",
];

fn build_alternation(words: &[&str]) -> String {
    words
        .iter()
        .map(|w| format!(r"\b{}\b", w))
        .collect::<Vec<_>>()
        .join("|")
}

fn contains_phrase(text: &str, phrase: &str) -> bool {
    // Use match_indices so we never manually advance a byte offset into a multi-byte
    // codepoint. The boundary check uses as_bytes() only for the bytes immediately
    // outside the match, both of which are guaranteed to be on char boundaries because
    // str::find / match_indices always return valid char-boundary offsets.
    for (actual_idx, matched) in text.match_indices(phrase) {
        let end_idx = actual_idx + matched.len();
        let prev_ok = actual_idx == 0 || !text.as_bytes()[actual_idx - 1].is_ascii_alphabetic();
        let next_ok = end_idx == text.len() || !text.as_bytes()[end_idx].is_ascii_alphabetic();
        if prev_ok && next_ok {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FlightPrompt {
    pub origin: Option<LocationConstraint>,
    pub destination: Option<LocationConstraint>,
    pub aircraft: Option<AircraftConstraint>,
    pub duration_minutes: Option<u32>,
    pub ignore_guardrails: bool,
    pub keywords: FlightKeywords,
    /// Soft distance floor from a matched aircraft rule (nm). Overridden by duration keywords.
    #[serde(default)]
    pub aircraft_min_dist: Option<f64>,
    /// Soft distance cap from a matched aircraft rule (nm). Overridden by duration keywords.
    #[serde(default)]
    pub aircraft_max_dist: Option<f64>,
    /// Cruise speed override from a matched aircraft rule (kts). Overrides heuristic estimate.
    #[serde(default)]
    pub aircraft_speed_kts: Option<u32>,
    /// Optional bearing constraint from origin (degrees). E.g. "north" → (315, 45).
    #[serde(default)]
    pub direction_bearing: Option<(f64, f64)>,
    /// User-specified minimum distance (nm). E.g. "between 100 and 200 nm" → 100.
    #[serde(default)]
    pub user_min_dist_nm: Option<f64>,
    /// User-specified maximum distance (nm). E.g. "within 50 nm" → 50.
    #[serde(default)]
    pub user_max_dist_nm: Option<f64>,
    /// Exact engine count constraint. E.g. "twin engine" -> 2.
    #[serde(default)]
    pub num_engines: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FlightKeywords {
    pub duration: Option<DurationKeyword>,
    pub surface: Option<SurfaceKeyword>,
    pub flight_type: Option<TypeKeyword>,
    pub time: Option<TimeKeyword>,
    pub weather: Option<WeatherKeyword>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeKeyword {
    Dawn,
    Day,
    Dusk,
    Night,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WeatherKeyword {
    Clear,
    Cloudy,
    Storm,
    Rain,
    Snow,
    Fog,
    Gusty,
    Calm,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DurationKeyword {
    Short,  // < 45m / < 200nm
    Medium, // 45m - 2h / 200 - 800nm
    Long,   // > 2h / > 800nm
    Haul,   // > 4h / > 2000nm
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurfaceKeyword {
    Soft,  // Grass, Dirt, Unpaved
    Hard,  // Paved, Tarmac, Asphalt
    Water, // Seaplane bases, floatplanes
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeKeyword {
    Bush,     // Remote, short runway
    Regional, // Standard airports
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LocationConstraint {
    ICAO(String),
    Region(String),
    AirportName(String),
    /// Airport search near a named city center (lat/lon) within ~50 nm.
    NearCity {
        name: String,
        lat: f64,
        lon: f64,
    },
    Any,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AircraftConstraint {
    Tag(String),  // Matches tags like "jet", "cessna", "heavy"
    ICAO(String), // Matches ICAO aircraft types like "C172", "B788"
    Any,
}

impl std::fmt::Display for AircraftConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AircraftConstraint::Tag(tag) => write!(f, "{}", tag),
            AircraftConstraint::ICAO(icao) => write!(f, "ICAO:{}", icao),
            AircraftConstraint::Any => write!(f, "Any"),
        }
    }
}

/// Returns true if `s` contains any CJK Unified Ideographs (U+4E00–U+9FFF).
fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

/// Preprocesses Chinese input by replacing Chinese directional/modifier phrases
/// with their English equivalents so the English NLP pipeline can parse them.
/// Returns a new `String`; if no CJK characters are detected the input is returned
/// unchanged (cheap clone of a short &str).
fn preprocess_chinese(input: &str) -> String {
    if !has_cjk(input) {
        return input.to_string();
    }
    let mut s = input.to_string();

    // ── Directional markers ──────────────────────────────────────────────────
    // Apply longer/more-specific phrases first to avoid partial-match collisions.
    s = s.replace("东北", "northeast");
    s = s.replace("西北", "northwest");
    s = s.replace("东南", "southeast");
    s = s.replace("西南", "southwest");
    s = s.replace("向北", "north");
    s = s.replace("向南", "south");
    s = s.replace("向东", "east");
    s = s.replace("向西", "west");
    s = s.replace("飞往", " to ");
    s = s.replace("飞去", " to ");
    s = s.replace("飞向", " to ");
    s = s.replace("前往", " to ");
    s = s.replace("从", "from ");
    s = s.replace("到", " to "); // single-char; applied after multi-char compounds

    // ── Duration ─────────────────────────────────────────────────────────────
    s = s.replace("超长途", "long haul");
    s = s.replace("洲际", "long haul");
    s = s.replace("跨洋", "long haul");
    s = s.replace("跨大西洋", "long haul");
    s = s.replace("跨太平洋", "long haul");
    s = s.replace("长途", "long");
    s = s.replace("远程", "long");
    s = s.replace("长程", "long");
    s = s.replace("中途", "medium");
    s = s.replace("中程", "medium");
    s = s.replace("短途", "short");
    s = s.replace("短程", "short");
    s = s.replace("近距", "short");

    // ── Surface ──────────────────────────────────────────────────────────────
    // Seaplane phrases before generic "water" to avoid partial replacement.
    s = s.replace("水上飞机", "seaplane");
    s = s.replace("水飞机", "seaplane");
    s = s.replace("浮筒飞机", "seaplane");
    s = s.replace("水陆两用", "seaplane");
    s = s.replace("草地跑道", "grass");
    s = s.replace("草坪", "grass");
    s = s.replace("土跑道", "grass");
    s = s.replace("泥土", "grass");
    s = s.replace("碎石", "grass");
    s = s.replace("砂砾", "grass");
    s = s.replace("未铺设", "grass");
    s = s.replace("未铺装", "grass");
    s = s.replace("柏油", "tarmac");
    s = s.replace("混凝土", "tarmac");
    s = s.replace("沥青", "tarmac");
    s = s.replace("硬地跑道", "tarmac");
    s = s.replace("铺装", "tarmac");
    s = s.replace("水上", "water");
    s = s.replace("水面", "water");

    // ── Weather ──────────────────────────────────────────────────────────────
    s = s.replace("晴天", "clear");
    s = s.replace("晴朗", "clear");
    s = s.replace("晴空万里", "clear");
    s = s.replace("多云", "cloudy");
    s = s.replace("阴天", "cloudy");
    s = s.replace("暴风雨", "storm");
    s = s.replace("雷暴", "storm");
    s = s.replace("风暴", "storm");
    // Heavy/light rain — longer phrases first so "暴雨" isn't partially caught by later "雨" rules.
    s = s.replace("暴雨", "storm"); // torrential → storm intensity
    s = s.replace("大雨", "rain");
    s = s.replace("小雨", "rain");
    s = s.replace("阵风", "gusty");
    s = s.replace("大风", "gusty");
    s = s.replace("强风", "gusty");
    s = s.replace("平静", "calm");
    s = s.replace("无风", "calm");
    s = s.replace("下雪", "snow");
    s = s.replace("暴雪", "snow");
    s = s.replace("冰雪", "snow");
    s = s.replace("下雨", "rain");
    s = s.replace("阵雨", "rain");
    s = s.replace("降雨", "rain");
    s = s.replace("大雾", "fog");
    s = s.replace("薄雾", "fog");
    s = s.replace("霾", "fog");

    // ── Time ─────────────────────────────────────────────────────────────────
    s = s.replace("黎明", "dawn");
    s = s.replace("日出", "dawn");
    s = s.replace("清晨", "dawn");
    s = s.replace("拂晓", "dawn");
    s = s.replace("黄昏", "dusk");
    s = s.replace("日落", "dusk");
    s = s.replace("傍晚", "dusk");
    s = s.replace("凌晨", "night"); // early morning hours (0–4 am) = night from a flight-context view
    s = s.replace("午夜", "night");
    s = s.replace("深夜", "night");
    s = s.replace("夜间", "night");
    s = s.replace("夜晚", "night");
    s = s.replace("晚上", "night");
    s = s.replace("白天", "day");
    s = s.replace("上午", "day");
    s = s.replace("下午", "day");
    s = s.replace("正午", "day");

    // ── Flight type ──────────────────────────────────────────────────────────
    s = s.replace("丛林飞行", "bush");
    s = s.replace("偏远地区", "bush");
    s = s.replace("越野", "bush");
    s = s.replace("野外", "bush");

    // ── Aircraft type hints ──────────────────────────────────────────────────
    s = s.replace("直升机", "helicopter");
    s = s.replace("旋翼机", "helicopter");
    s = s.replace("波音", "boeing");
    s = s.replace("空客", "airbus");
    // Longer phrase first so "涡轮螺旋桨" doesn't get partially replaced by "涡桨".
    s = s.replace("涡轮螺旋桨", "turboprop");
    s = s.replace("涡桨", "turboprop");
    s = s.replace("喷气式", "jet");
    s = s.replace("喷气机", "jet");

    // ── Grammatical particles ────────────────────────────────────────────────
    // "在" (at/in/located-at) often appears between aircraft and time context,
    // e.g. "A320在凌晨" (A320 at dawn).  Map to " at " so the ACF_RE \bat\b
    // terminator can cleanly cut off time context from the aircraft token.
    s = s.replace("在", " at ");
    // Other high-frequency particles that carry no NLP value in this context.
    s = s.replace("的", " "); // possessive/attributive
    s = s.replace("了", " "); // perfective marker

    // ── Vehicle connector ────────────────────────────────────────────────────
    s = s.replace("搭乘", " in a "); // board / travel on (common for flights)
    s = s.replace("乘坐", " in a ");
    s = s.replace("使用", " in a ");
    s = s.replace("驾驶", " in a ");

    // ── Generic flight / trip noise words ───────────────────────────────────
    s = s.replace("飞行", " flight ");
    s = s.replace("航班", " flight ");
    s = s.replace("航程", " ");

    // ── CJK ↔ ASCII spacing pass ─────────────────────────────────────────────
    // After keyword substitution, remaining CJK characters (city names) may be
    // directly adjacent to converted ASCII keywords (e.g. "成都short").
    // Insert a space at every CJK↔ASCII transition so the location regex can
    // cleanly tokenise city names from trailing English keywords.
    let chars: Vec<char> = s.chars().collect();
    let mut spaced = String::with_capacity(s.len() + 16);
    for (i, &ch) in chars.iter().enumerate() {
        let is_cjk = ('\u{4E00}'..='\u{9FFF}').contains(&ch);
        if i > 0 {
            let prev = chars[i - 1];
            let prev_cjk = ('\u{4E00}'..='\u{9FFF}').contains(&prev);
            let prev_space = prev == ' ';
            if is_cjk != prev_cjk && !prev_space && ch != ' ' {
                spaced.push(' ');
            }
        }
        spaced.push(ch);
    }

    spaced.trim().to_string()
}

impl FlightPrompt {
    pub fn parse(input: &str, rules: &crate::NLPRulesConfig) -> Self {
        let mut prompt = FlightPrompt::default();
        let preprocessed = preprocess_chinese(input.trim());
        let input_lower = preprocessed.trim().to_lowercase();

        // 1. Check for "ignore guardrails"
        let mut clean_input = input_lower.clone();
        if clean_input.contains("ignore guardrails") {
            prompt.ignore_guardrails = true;
            clean_input = clean_input.replace("ignore guardrails", "");
        }

        // Helper: sort rule slice by priority descending, return sorted references.
        fn sorted_rules(rules: &[crate::NLPRule]) -> Vec<&crate::NLPRule> {
            let mut v: Vec<&crate::NLPRule> = rules.iter().collect();
            v.sort_by(|a, b| b.priority.cmp(&a.priority));
            v
        }

        // 2. Parse Keywords (Global search)
        // Duration — JSON rules checked first (priority-sorted), then hardcoded fallback.
        let mut duration_matched = false;
        for rule in sorted_rules(&rules.duration_rules) {
            if rule
                .keywords
                .iter()
                .any(|k| clean_input.contains(k.to_lowercase().as_str()))
            {
                let mapped = match rule.mapped_value.to_lowercase().as_str() {
                    "short" | "hop" | "quick" | "sprint" => DurationKeyword::Short,
                    "medium" | "mid" => DurationKeyword::Medium,
                    "haul" | "long haul" | "ultra long" | "intercontinental" => {
                        DurationKeyword::Haul
                    }
                    _ => DurationKeyword::Long, // "long" and anything else
                };
                prompt.keywords.duration = Some(mapped);
                duration_matched = true;
                break;
            }
        }
        if !duration_matched {
            if clean_input.contains("short")
                || clean_input.contains("hop")
                || clean_input.contains("quick")
            {
                prompt.keywords.duration = Some(DurationKeyword::Short);
            } else if clean_input.contains("medium") {
                prompt.keywords.duration = Some(DurationKeyword::Medium);
            } else if clean_input.contains("long haul")
                || clean_input.contains("ultra long")
                || clean_input.contains("transatlantic")
                || clean_input.contains("transpacific")
                || clean_input.contains("transcontinental")
            {
                prompt.keywords.duration = Some(DurationKeyword::Haul);
            } else if clean_input.contains("long") {
                prompt.keywords.duration = Some(DurationKeyword::Long);
            }
        }

        // Surface — JSON rules checked first, then hardcoded fallback.
        let mut surface_matched = false;
        for rule in sorted_rules(&rules.surface_rules) {
            if rule
                .keywords
                .iter()
                .any(|k| clean_input.contains(k.to_lowercase().as_str()))
            {
                let mapped = match rule.mapped_value.to_lowercase().as_str() {
                    "hard" | "paved" | "tarmac" | "asphalt" => SurfaceKeyword::Hard,
                    "water" | "seaplane" | "float" => SurfaceKeyword::Water,
                    _ => SurfaceKeyword::Soft, // "soft" and anything else
                };
                prompt.keywords.surface = Some(mapped);
                surface_matched = true;
                break;
            }
        }
        if !surface_matched {
            if clean_input.contains("grass")
                || clean_input.contains("dirt")
                || clean_input.contains("gravel")
                || clean_input.contains("strip")
                || clean_input.contains("unpaved")
            {
                prompt.keywords.surface = Some(SurfaceKeyword::Soft);
            } else if clean_input.contains("paved")
                || clean_input.contains("tarmac")
                || clean_input.contains("concrete")
                || clean_input.contains("asphalt")
            {
                prompt.keywords.surface = Some(SurfaceKeyword::Hard);
            } else if clean_input.contains("water")
                || clean_input.contains("seaplane")
                || clean_input.contains("floatplane")
                || clean_input.contains("amphibian")
            {
                prompt.keywords.surface = Some(SurfaceKeyword::Water);
            }
        }

        // Type — JSON rules checked first, then hardcoded fallback.
        let mut type_matched = false;
        for rule in sorted_rules(&rules.flight_type_rules) {
            if rule
                .keywords
                .iter()
                .any(|k| clean_input.contains(k.to_lowercase().as_str()))
            {
                let mapped = match rule.mapped_value.to_lowercase().as_str() {
                    "bush" | "backcountry" | "remote" | "stol" => TypeKeyword::Bush,
                    _ => TypeKeyword::Regional, // "regional" and anything else
                };
                prompt.keywords.flight_type = Some(mapped.clone());
                // Bush implies soft surface if not already set
                if mapped == TypeKeyword::Bush && prompt.keywords.surface.is_none() {
                    prompt.keywords.surface = Some(SurfaceKeyword::Soft);
                }
                type_matched = true;
                break;
            }
        }
        if !type_matched {
            if clean_input.contains("bush") || clean_input.contains("backcountry") {
                prompt.keywords.flight_type = Some(TypeKeyword::Bush);
                if prompt.keywords.surface.is_none() {
                    prompt.keywords.surface = Some(SurfaceKeyword::Soft);
                }
            } else if clean_input.contains("regional") {
                prompt.keywords.flight_type = Some(TypeKeyword::Regional);
            }
        }

        // Time — JSON rules checked first (priority-sorted), then hardcoded fallback.
        let mut time_matched = false;
        for rule in sorted_rules(&rules.time_rules) {
            if rule
                .keywords
                .iter()
                .any(|k| contains_phrase(&clean_input, &k.to_lowercase()))
            {
                let mapped = match rule.mapped_value.to_lowercase().as_str() {
                    "dawn" | "sunrise" | "morning" | "golden hour" | "golden" => TimeKeyword::Dawn,
                    "dusk" | "sunset" | "evening" | "twilight" | "civil twilight" => {
                        TimeKeyword::Dusk
                    }
                    "night" | "midnight" | "dark" | "night flight" | "moonlight" | "late night" => {
                        TimeKeyword::Night
                    }
                    _ => TimeKeyword::Day,
                };
                prompt.keywords.time = Some(mapped);
                time_matched = true;
                break;
            }
        }

        if !time_matched {
            if contains_phrase(&clean_input, "dawn")
                || contains_phrase(&clean_input, "sunrise")
                || contains_phrase(&clean_input, "morning")
                || contains_phrase(&clean_input, "golden hour")
                || contains_phrase(&clean_input, "golden")
            {
                prompt.keywords.time = Some(TimeKeyword::Dawn);
            } else if contains_phrase(&clean_input, "day")
                || contains_phrase(&clean_input, "daytime")
                || contains_phrase(&clean_input, "daylight")
                || contains_phrase(&clean_input, "afternoon")
                || contains_phrase(&clean_input, "noon")
            {
                prompt.keywords.time = Some(TimeKeyword::Day);
            } else if contains_phrase(&clean_input, "dusk")
                || contains_phrase(&clean_input, "sunset")
                || contains_phrase(&clean_input, "evening")
                || contains_phrase(&clean_input, "twilight")
            {
                prompt.keywords.time = Some(TimeKeyword::Dusk);
            } else if contains_phrase(&clean_input, "night")
                || contains_phrase(&clean_input, "midnight")
                || contains_phrase(&clean_input, "dark")
            {
                prompt.keywords.time = Some(TimeKeyword::Night);
            }
        }

        // Weather — JSON rules checked first (priority-sorted), then hardcoded fallback.
        let mut weather_matched = false;
        for rule in sorted_rules(&rules.weather_rules) {
            if rule
                .keywords
                .iter()
                .any(|k| contains_phrase(&clean_input, &k.to_lowercase()))
            {
                let mapped = match rule.mapped_value.to_lowercase().as_str() {
                    "clear" | "sunny" | "fair" | "vfr" | "visual" | "clear vfr" | "cavok"
                    | "cavu" | "clear skies" | "blue sky" | "easy" | "relax" | "scenic" => {
                        WeatherKeyword::Clear
                    }
                    "cloudy" | "overcast" | "clouds" | "mvfr" | "marginal" | "scattered"
                    | "few clouds" | "broken" => WeatherKeyword::Cloudy,
                    "storm" | "thunder" | "thunderstorm" | "severe" | "lifr" | "low ifr"
                    | "challenge" | "hard mode" => WeatherKeyword::Storm,
                    "gusty" | "windy" | "breezy" | "turbulent" | "gusts" => WeatherKeyword::Gusty,
                    "calm" | "still" | "smooth" | "no wind" | "light winds" | "glassy" => {
                        WeatherKeyword::Calm
                    }
                    "snow" | "blizzard" | "ice" | "wintry" | "winter" | "frozen" | "snowy"
                    | "icy" => WeatherKeyword::Snow,
                    "rain" | "showers" | "wet" => WeatherKeyword::Rain,
                    "fog" | "mist" | "haze" | "ifr" | "instrument" | "smoky" => WeatherKeyword::Fog,
                    _ => WeatherKeyword::Clear,
                };
                prompt.keywords.weather = Some(mapped);
                weather_matched = true;
                break;
            }
        }

        if !weather_matched {
            if contains_phrase(&clean_input, "clear")
                || contains_phrase(&clean_input, "sunny")
                || contains_phrase(&clean_input, "fair")
                || contains_phrase(&clean_input, "vfr")
            {
                prompt.keywords.weather = Some(WeatherKeyword::Clear);
            } else if contains_phrase(&clean_input, "cloudy")
                || contains_phrase(&clean_input, "overcast")
                || contains_phrase(&clean_input, "clouds")
            {
                prompt.keywords.weather = Some(WeatherKeyword::Cloudy);
            } else if contains_phrase(&clean_input, "storm")
                || contains_phrase(&clean_input, "thunder")
                || contains_phrase(&clean_input, "thunderstorm")
                || contains_phrase(&clean_input, "lightning")
                || contains_phrase(&clean_input, "severe")
            {
                prompt.keywords.weather = Some(WeatherKeyword::Storm);
            } else if contains_phrase(&clean_input, "gusty")
                || contains_phrase(&clean_input, "windy")
                || contains_phrase(&clean_input, "breezy")
                || contains_phrase(&clean_input, "turbulent")
                || contains_phrase(&clean_input, "gusts")
            {
                prompt.keywords.weather = Some(WeatherKeyword::Gusty);
            } else if contains_phrase(&clean_input, "calm")
                || contains_phrase(&clean_input, "still")
                || contains_phrase(&clean_input, "smooth")
                || contains_phrase(&clean_input, "light winds")
                || contains_phrase(&clean_input, "glassy")
            {
                prompt.keywords.weather = Some(WeatherKeyword::Calm);
            } else if contains_phrase(&clean_input, "snow")
                || contains_phrase(&clean_input, "blizzard")
                || contains_phrase(&clean_input, "ice")
            {
                prompt.keywords.weather = Some(WeatherKeyword::Snow);
            } else if contains_phrase(&clean_input, "rain")
                || contains_phrase(&clean_input, "showers")
                || contains_phrase(&clean_input, "drizzle")
                || contains_phrase(&clean_input, "wet")
            {
                prompt.keywords.weather = Some(WeatherKeyword::Rain);
            } else if contains_phrase(&clean_input, "fog")
                || contains_phrase(&clean_input, "mist")
                || contains_phrase(&clean_input, "haze")
                || contains_phrase(&clean_input, "ifr")
                || contains_phrase(&clean_input, "low vis")
            {
                prompt.keywords.weather = Some(WeatherKeyword::Fog);
            }
        }

        // 2b. Parse "direction of LOCATION" → origin=LOCATION, direction=bearing.
        // e.g. "fly north of EGMC", "northeast of London".
        // Must run before step 3 so the location is consumed and not double-parsed.
        static DIR_OF_RE: OnceLock<Regex> = OnceLock::new();
        let dir_of_re = DIR_OF_RE.get_or_init(|| {
            Regex::new(r"(?i)\b(north(?:east|west)?|south(?:east|west)?|east|west)\s+of\s+([a-zA-Z0-9][a-zA-Z0-9\s,]{0,30}?)(?:\s+(to|using|in|with|on|aboard|taking|flying|for|via|at|about)(?:\s|$)|$)").unwrap()
        });
        if prompt.origin.is_none() {
            if let Some(caps) = dir_of_re.captures(&clean_input) {
                let dir_word = caps[1].to_lowercase();
                let loc_str = caps[2].trim();
                let bearing = match dir_word.as_str() {
                    "north" => Some((315.0, 45.0)),
                    "northeast" => Some((22.5, 67.5)),
                    "east" => Some((45.0, 135.0)),
                    "southeast" => Some((112.5, 157.5)),
                    "south" => Some((135.0, 225.0)),
                    "southwest" => Some((202.5, 247.5)),
                    "west" => Some((225.0, 315.0)),
                    "northwest" => Some((292.5, 337.5)),
                    _ => None,
                };
                if bearing.is_some() {
                    prompt.direction_bearing = bearing;
                    prompt.origin = Some(parse_location(loc_str));
                    // Remove the matched portion; if a terminator keyword (e.g. "to") was
                    // consumed as group 3, put it back so step 3 can parse "to Paris" etc.
                    let terminator = caps.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
                    let full_match = caps[0].trim().to_string();
                    clean_input = clean_input.replacen(&full_match, "", 1);
                    clean_input = clean_input.trim().to_string();
                    if !terminator.is_empty() {
                        clean_input = format!("{} {}", terminator.trim(), clean_input);
                    }
                }
            }
        }

        // 2c. Pre-extract distance constraints from the full clean_input BEFORE step 3
        // strips location text.  Distance text embedded in dest_str (e.g. "a destination
        // within 70km or 90nm") is consumed by the LOC_RE strip and invisible to step 7.
        // Running here ensures those constraints are captured; step 7 is a no-op override
        // when the text is already gone.
        //
        // Unit aliases supported: nm, nmi, km, kilometer(s), kilometre(s), mi, miles,
        // nautical miles.  "within N unit or M unit" takes the larger (most permissive) bound.
        {
            const U: &str = r"(?:nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)";
            static PRE_DIST_OR_RE: OnceLock<Regex> = OnceLock::new();
            let pre_dist_or_re = PRE_DIST_OR_RE.get_or_init(|| {
                Regex::new(&format!(
                    r"(?i)(?:within|under|less than|max(?:imum)?)\s+(\d+(?:\.\d+)?)\s*({})\s+or\s+(\d+(?:\.\d+)?)\s*({})",
                    U, U
                ))
                .unwrap()
            });
            static PRE_DIST_BETWEEN_RE: OnceLock<Regex> = OnceLock::new();
            let pre_dist_between_re = PRE_DIST_BETWEEN_RE.get_or_init(|| {
                Regex::new(&format!(
                    r"(?i)between\s+(\d+(?:\.\d+)?)\s+and\s+(\d+(?:\.\d+)?)\s*({})",
                    U
                ))
                .unwrap()
            });
            static PRE_DIST_WITHIN_RE: OnceLock<Regex> = OnceLock::new();
            let pre_dist_within_re = PRE_DIST_WITHIN_RE.get_or_init(|| {
                Regex::new(&format!(
                    r"(?i)(?:within|under|less than|max(?:imum)?)\s+(\d+(?:\.\d+)?)\s*({})",
                    U
                ))
                .unwrap()
            });
            static PRE_DIST_ATLEAST_RE: OnceLock<Regex> = OnceLock::new();
            let pre_dist_atleast_re = PRE_DIST_ATLEAST_RE.get_or_init(|| {
                Regex::new(&format!(
                    r"(?i)(?:at least|over|more than|min(?:imum)?)\s+(\d+(?:\.\d+)?)\s*({})",
                    U
                ))
                .unwrap()
            });
            static PRE_DIST_BARE_RE: OnceLock<Regex> = OnceLock::new();
            let pre_dist_bare_re = PRE_DIST_BARE_RE.get_or_init(|| {
                Regex::new(&format!(r"(?i)\b(\d+(?:\.\d+)?)\s*({})\b", U)).unwrap()
            });

            if let Some(caps) = pre_dist_or_re.captures(&clean_input) {
                if let (Ok(a), Ok(b)) = (caps[1].parse::<f64>(), caps[3].parse::<f64>()) {
                    prompt.user_max_dist_nm =
                        Some(to_nm_unit(a, &caps[2]).max(to_nm_unit(b, &caps[4])));
                }
            } else if let Some(caps) = pre_dist_between_re.captures(&clean_input) {
                if let (Ok(a), Ok(b)) = (caps[1].parse::<f64>(), caps[2].parse::<f64>()) {
                    prompt.user_min_dist_nm = Some(to_nm_unit(a, &caps[3]));
                    prompt.user_max_dist_nm = Some(to_nm_unit(b, &caps[3]));
                }
            } else if let Some(caps) = pre_dist_within_re.captures(&clean_input) {
                if let Ok(v) = caps[1].parse::<f64>() {
                    prompt.user_max_dist_nm = Some(to_nm_unit(v, &caps[2]));
                }
            } else if let Some(caps) = pre_dist_atleast_re.captures(&clean_input) {
                if let Ok(v) = caps[1].parse::<f64>() {
                    prompt.user_min_dist_nm = Some(to_nm_unit(v, &caps[2]));
                }
            } else if let Some(caps) = pre_dist_bare_re.captures(&clean_input) {
                if let Ok(v) = caps[1].parse::<f64>() {
                    let nm = to_nm_unit(v, &caps[2]);
                    prompt.user_min_dist_nm = Some((nm * 0.7).max(2.0));
                    prompt.user_max_dist_nm = Some(nm * 1.3);
                }
            }
        }

        // 3. Parse Origin and Destination
        // Patterns: "from X to Y", "flight from X to Y", "X to Y"
        // Suffix terminators include "via" so "Paris via Brussels" → dest="paris".
        static LOC_RE: OnceLock<Regex> = OnceLock::new();
        let loc_re = LOC_RE.get_or_init(|| {
            let terms = build_alternation(LOCATION_TERMINATORS);
            Regex::new(&format!(
                r"(?:flight\s+from\s+|\bfrom\s+|^flight\s+)?(.+?)\s+\bto\b\s+(.+?)(\s+(?:{})|$)",
                terms
            ))
            .unwrap()
        });

        if let Some(caps) = loc_re.captures(&clean_input) {
            let origin_str = caps[1].trim();
            let dest_str = caps[2].trim();

            // Detect noise origins: words like "flight", "quick flight", "a hop" that
            // appear before "to" but are not locations. The LOC_RE can match these when
            // the optional "flight from" prefix is absent — e.g. "flight to Germany"
            // captures origin="flight", dest="germany" instead of dest-only.
            let is_noise_origin = origin_str == "flight"
                || origin_str.ends_with(" flight")
                || origin_str == "hop"
                || origin_str.ends_with(" hop")
                || origin_str == "trip"
                || origin_str.ends_with(" trip")
                || origin_str == "run"
                || origin_str.ends_with(" run")
                || origin_str == "journey"
                || origin_str.ends_with(" journey")
                || origin_str == "a"
                || origin_str == "the"
                // "fly", "flying", "heading", "going", "headed", "bound" appear when
                // someone writes "fly to X" and LOC_RE picks up "fly" as origin.
                || origin_str == "fly"
                || origin_str == "flying"
                || origin_str == "heading"
                || origin_str == "going"
                || origin_str == "headed"
                || origin_str == "bound"
                || origin_str == "on"
                || origin_str == "aboard"
                || origin_str == "taking"
                || origin_str == "at"
                || origin_str == "during";

            // Resolve destination: strip embedded distance text, check for noise-only strings,
            // then strip descriptor noise so the underlying location can be resolved.
            // E.g. "a destination within 70km or 90nm" → Any
            //      "any airport in France within 500nm"  → Region("FR")
            //      "paris within 500nm"                  → NearCity("Paris")
            let dest_no_dist = strip_distance_text(dest_str);
            let dest_loc = if is_noise_dest(dest_str) {
                LocationConstraint::Any
            } else {
                let dest_clean = clean_location_noise(&dest_no_dist);
                if !dest_clean.is_empty() {
                    parse_location(&dest_clean)
                } else {
                    parse_location(dest_str)
                }
            };

            if is_noise_origin {
                // Treat as destination-only (same as "flight to X" / "to X" path)
                prompt.destination = Some(dest_loc);
            } else {
                // Strip descriptor noise from origin too (e.g. "high altitude departure airport
                // in Europe" → "europe").
                let origin_clean = clean_location_noise(origin_str);
                let origin_loc = if !origin_clean.is_empty() {
                    parse_location(&origin_clean)
                } else {
                    parse_location(origin_str)
                };
                prompt.origin = Some(origin_loc);
                prompt.destination = Some(dest_loc);
            }
            // Remove the matched location text so its words (e.g. "South" in "South Korea")
            // don't trigger keywords later (like cardinal directions).
            // We preserve the terminator (caps[3]) because it may also be an aircraft connector
            // (e.g. "on" in "EGLL to LIRF on an MD-80").
            if let Some(m) = caps.get(3) {
                // Only strip from the start of the full match to the start of the terminator.
                let strip = &clean_input[caps.get(0).unwrap().start()..m.start()];
                clean_input = clean_input.replacen(strip, " ", 1);
            } else {
                clean_input = clean_input.replace(&caps[0], " ");
            }
        } else {
            // Fallback: Check for destination-only prompt.
            // Handles: "to X", "flight to X", "fly to X", "heading to X",
            // "going to X", "headed to X", "bound for X".
            static TO_RE: OnceLock<Regex> = OnceLock::new();
            let to_re = TO_RE.get_or_init(|| {
                let terms = build_alternation(LOCATION_TERMINATORS);
                Regex::new(&format!(
                    r"(?:^(?:flight|fly|flying|heading|going|headed)\s+to\s+|^to\s+|^bound\s+for\s+)(.+?)(\s+(?:{})|$)",
                    terms
                ))
                .unwrap()
            });
            if let Some(caps) = to_re.captures(&clean_input) {
                let dest_str = caps[1].trim();
                prompt.destination = Some(parse_location(dest_str));
                // Remove the matched location structure
                clean_input = clean_input.replace(&caps[0], " ");
            } else {
                // Fallback: "from X" without "to Y" (e.g. "2 hour flight from UK") — constrain origin only
                static FROM_RE: OnceLock<Regex> = OnceLock::new();
                let from_re =
                    FROM_RE.get_or_init(|| Regex::new(r"\bfrom\b\s+([a-zA-Z0-9\s,]+)").unwrap());
                if let Some(caps) = from_re.captures(&clean_input) {
                    let raw = caps[1].trim();
                    // Strip trailing keywords so "from UK for 2 hours" → "UK",
                    // "from KSFO southbound" → "ksfo" (terminator may be at end of string,
                    // so check both " term " in middle and " term" at end).
                    let origin_str = LOCATION_TERMINATORS
                        .iter()
                        .fold(raw, |acc, &term| {
                            let phrase_mid = format!(" {} ", term);
                            let phrase_end = format!(" {}", term);
                            if let Some(i) = acc.find(&phrase_mid) {
                                &acc[..i]
                            } else if acc.ends_with(&phrase_end) {
                                &acc[..acc.len() - phrase_end.len()]
                            } else {
                                acc
                            }
                        })
                        .trim();
                    if !origin_str.is_empty() {
                        prompt.origin = Some(parse_location(origin_str));
                        // Only strip "from {origin_str}" — NOT the full greedy caps[0].
                        // If a terminator cut the raw capture short (e.g. "f70 heading south"
                        // → origin_str="f70"), the remainder ("heading south") must stay in
                        // clean_input so direction/keyword parsing still fires.
                        if origin_str == raw {
                            clean_input = clean_input.replace(&caps[0], " ");
                        } else {
                            let to_remove = format!("from {}", origin_str);
                            clean_input = clean_input.replacen(&to_remove, " ", 1);
                        }
                    }
                }

                // Final fallback: bare region/city name with no directional keyword.
                // e.g. "Washington State", "Alaska", "London" — treat as destination.
                if prompt.origin.is_none() && prompt.destination.is_none() {
                    let candidate = normalize_for_region_match(&clean_input);
                    if let Some(region) = try_as_region(&candidate) {
                        prompt.destination = Some(region);
                    }
                }
            }
        }

        // 4. Parse Aircraft
        // Strip known modifier phrases so "using Boeing long haul" captures "boeing" not "boeing long haul".
        // We strip multi-word modifiers only — single words like "long"/"short" are left alone to avoid
        // false-positive on aircraft names that contain them (e.g. "Rutan Long-EZ").
        let acf_input = clean_input
            .replace("short flight", "")
            .replace("short hop", "")
            .replace("long haul", "")
            .replace("long flight", "")
            .replace("medium flight", "")
            .replace("bush flight", "")
            .replace("bush trip", "")
            .replace("backcountry flight", "")
            .replace("backcountry trip", "")
            .replace("quick trip", "")
            .replace("ignore guardrails", "");

        let mut acf_matched = false;

        // 1. Check Custom Dictionary First (Global Search, priority-sorted)
        for rule in sorted_rules(&rules.aircraft_rules) {
            if rule
                .keywords
                .iter()
                .any(|k| contains_phrase(&clean_input, &k.to_lowercase()))
            {
                prompt.aircraft = Some(AircraftConstraint::Tag(rule.mapped_value.clone()));
                prompt.aircraft_min_dist = rule.min_distance_nm.map(|v| v as f64);
                prompt.aircraft_max_dist = rule.max_distance_nm.map(|v| v as f64);
                prompt.aircraft_speed_kts = rule.speed_kts;
                acf_matched = true;
                break;
            }
        }

        // 2. If no custom rule matched, use Regex for generic/explicit aircraft
        if !acf_matched {
            static ACF_RE: OnceLock<Regex> = OnceLock::new();
            let acf_re = ACF_RE.get_or_init(|| {
                let connectors = build_alternation(AIRCRAFT_CONNECTORS);
                Regex::new(&format!(
                    r"(?:{})\b(?:\s+a|\s+an)?\s+(.+?)(\s+\bat\b|\s+\bfor\b|\s+\bfrom\b|\s+\blanding\b|\s+\barriving\b|\s+\bdeparting\b|$)",
                    connectors
                ))
                .unwrap()
            });

            if let Some(caps) = acf_re.captures(&acf_input) {
                let mut acf_str = caps[1].trim().to_string();

                // Safety net: strip any residual CJK characters that may have slipped
                // through preprocessing (aircraft names are always ASCII).
                if has_cjk(&acf_str) {
                    let cleaned: String = acf_str
                        .chars()
                        .filter(|c| !('\u{4E00}'..='\u{9FFF}').contains(c))
                        .collect();
                    let cleaned = cleaned.trim().to_string();
                    if !cleaned.is_empty() {
                        acf_str = cleaned;
                    }
                }

                let acf_lower = acf_str.to_lowercase();

                let is_weather_false_positive = WEATHER_TIME_WORDS.iter().any(|&w| acf_lower == w);

                // Check if the captured text is a geographic feature (destination clue, not aircraft).
                // Use word-level matching to avoid partial matches (e.g. "hill" inside "churchill").
                let is_geo_false_positive = acf_lower
                    .split_whitespace()
                    .any(|word| GEOGRAPHIC_FEATURE_WORDS.contains(&word));

                if is_geo_false_positive {
                    // The aircraft connector introduced a geographic destination clause.
                    // Redirect to destination parsing (e.g. "in the mountains in europe" → EU).
                    if prompt.destination.is_none() {
                        let cleaned = clean_location_noise(&acf_str);
                        let loc_str = if cleaned.is_empty() { &acf_str } else { &cleaned };
                        if !is_noise_dest(loc_str) {
                            prompt.destination = Some(parse_location(loc_str));
                        }
                    }
                } else if !is_weather_false_positive {
                    if !acf_str.is_empty() {
                        let acf_upper = acf_str.to_uppercase();
                        // 3. Detect ICAO Aircraft Type Codes (e.g. C172, B738, F16)
                        // Typically 2-4 chars, alphanumeric, starts with a letter.
                        if (acf_upper.len() >= 2 && acf_upper.len() <= 4)
                            && acf_upper
                                .chars()
                                .next()
                                .map_or(false, |c| c.is_ascii_alphabetic())
                            && acf_upper.chars().all(|c| c.is_ascii_alphanumeric())
                            && !acf_lower.contains("jet")
                            && !acf_lower.contains("prop")
                            && !acf_lower.contains("heli")
                        {
                            prompt.aircraft = Some(AircraftConstraint::ICAO(acf_upper));
                        } else {
                            // Normalize common conversational variants into standardized tags
                            // matching the BitNet classifier's taxonomy.
                            if acf_lower.contains("airliner")
                                || acf_lower.contains("commercial")
                                || acf_lower.contains("passenger")
                                || acf_lower.contains("heavy")
                                || (acf_lower.contains("jet") && !acf_lower.contains("biz"))
                            {
                                acf_str = "Airliner".to_string();
                            } else if acf_lower.contains("biz jet")
                                || acf_lower.contains("bizjet")
                                || acf_lower.contains("business")
                                || acf_lower.contains("corporate")
                                || acf_lower.contains("private jet")
                            {
                                acf_str = "Business Jet".to_string();
                            } else if acf_lower == "ga"
                                || acf_lower.contains("general aviation")
                                || acf_lower.contains("small plane")
                                || acf_lower.contains("light aircraft")
                                || acf_lower.contains("propeller")
                                || acf_lower.contains("piston")
                                || acf_lower.contains("civilian")
                                || acf_lower.contains("puddle")
                                || acf_lower.contains("tail")
                                || acf_lower.contains("float")
                                || acf_lower.contains("sea")
                            {
                                acf_str = "General Aviation".to_string();
                            } else if acf_lower.contains("glass")
                                || acf_lower.contains("g1000")
                                || acf_lower.contains("modern panel")
                            {
                                acf_str = "G1000".to_string();
                            } else if acf_lower.contains("steam") || acf_lower.contains("analog") {
                                acf_str = "Analog".to_string();
                            } else if acf_lower.contains("warbird")
                                || acf_lower.contains("wwii")
                                || acf_lower.contains("fighter")
                                || acf_lower.contains("military")
                                || acf_lower.contains("combat")
                                || acf_lower.contains("bomber")
                            {
                                acf_str = "Military".to_string();
                            } else if acf_lower.contains("cargo")
                                || acf_lower.contains("freight")
                                || acf_lower.contains("transport")
                            {
                                acf_str = "Cargo".to_string();
                            } else if acf_lower.contains("heli")
                                || acf_lower.contains("chopper")
                                || acf_lower.contains("rotor")
                            {
                                acf_str = "Helicopter".to_string();
                            } else if acf_lower.contains("glider")
                                || acf_lower.contains("sailplane")
                            {
                                acf_str = "Glider".to_string();
                            } else if acf_lower.contains("turboprop")
                                || acf_lower.contains("turbo prop")
                                || acf_lower.contains("twin engine")
                                || acf_lower.contains("twin-engine")
                                || acf_lower.contains("single engine")
                                || acf_lower.contains("single-engine")
                            {
                                acf_str = "General Aviation".to_string();
                            }

                            if !acf_str.is_empty() {
                                prompt.aircraft = Some(AircraftConstraint::Tag(acf_str));
                            }
                        }
                    }
                }
            }
        }

        // 3. Parse Engine Count (Global Search)
        if acf_input.contains("single engine") || acf_input.contains("single-engine") {
            prompt.num_engines = Some(1);
        } else if acf_input.contains("twin engine")
            || acf_input.contains("twin-engine")
            || acf_input.contains("twin jet")
            || acf_input.contains("twin-jet")
            || acf_input.contains("bi-jet")
        {
            prompt.num_engines = Some(2);
        } else if acf_input.contains("tri engine")
            || acf_input.contains("tri-engine")
            || acf_input.contains("tri jet")
            || acf_input.contains("tri-jet")
            || acf_input.contains("three engine")
        {
            prompt.num_engines = Some(3);
        } else if acf_input.contains("quad engine")
            || acf_input.contains("quad-engine")
            || acf_input.contains("quad jet")
            || acf_input.contains("quad-jet")
            || acf_input.contains("four engine")
        {
            prompt.num_engines = Some(4);
        }

        // 5. Parse Explicit Duration (Overrides keyword if present)
        static TIME_RE: OnceLock<Regex> = OnceLock::new();
        let time_re = TIME_RE
            .get_or_init(|| Regex::new(r"(?:for\s+)?\b(\d+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|a|an)\s+(hour|hr|minute|min|m)s?\b").unwrap());

        if let Some(caps) = time_re.captures(&clean_input) {
            let val_str = &caps[1];
            let val = match val_str {
                "one" | "a" | "an" => 1,
                "two" => 2,
                "three" => 3,
                "four" => 4,
                "five" => 5,
                "six" => 6,
                "seven" => 7,
                "eight" => 8,
                "nine" => 9,
                "ten" => 10,
                "eleven" => 11,
                "twelve" => 12,
                _ => val_str.parse::<u32>().unwrap_or(1),
            };

            if let Some(unit) = caps.get(2) {
                let minutes = match unit.as_str() {
                    "hour" | "hr" => val * 60,
                    _ => val,
                };
                prompt.duration_minutes = Some(minutes);
            }
        }

        // 6. Parse Cardinal Direction
        // Patterns: "north", "northeast", "heading east", "fly south"
        static DIR_RE: OnceLock<Regex> = OnceLock::new();
        let dir_re = DIR_RE.get_or_init(|| {
            // Prefixes: heading/fly/flying/go/head + optional "to the"/"towards the"
            // Suffixes: bare word, -bound (northbound), -ward/-wards (northward/northwards)
            Regex::new(r"\b(?:heading\s+|fly(?:ing)?\s+|go(?:ing)?\s+|head(?:ing)?\s+)?(?:towards?\s+the\s+|to\s+the\s+)?(north(?:east|west)?(?:bound|wards?)?|south(?:east|west)?(?:bound|wards?)?|east(?:bound|wards?)?|west(?:bound|wards?)?)\b").unwrap()
        });
        if let Some(caps) = dir_re.captures(&clean_input) {
            // Strip -bound / -ward / -wards suffixes to normalise "northbound" → "north"
            let raw_dir = caps[1].to_lowercase();
            let dir = raw_dir
                .trim_end_matches("wards")
                .trim_end_matches("ward")
                .trim_end_matches("bound");
            let bearing = match dir {
                "north" => Some((315.0, 45.0)),
                "northeast" => Some((22.5, 67.5)),
                "east" => Some((45.0, 135.0)),
                "southeast" => Some((112.5, 157.5)),
                "south" => Some((135.0, 225.0)),
                "southwest" => Some((202.5, 247.5)),
                "west" => Some((225.0, 315.0)),
                "northwest" => Some((292.5, 337.5)),
                _ => None,
            };
            prompt.direction_bearing = bearing;
        }

        // 7. Parse Distance Range (also handles leftovers after step 2c pre-extraction).
        // Unit aliases: nm, nmi, km, kilometer(s), kilometre(s), mi, miles, nautical miles.
        // "within N or M unit" takes the larger (more permissive) bound.
        static DIST_OR_RE: OnceLock<Regex> = OnceLock::new();
        let dist_or_re = DIST_OR_RE.get_or_init(|| {
            Regex::new(r"(?i)(?:within|under|less than|max(?:imum)?)\s+(\d+(?:\.\d+)?)\s*(nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)\s+or\s+(\d+(?:\.\d+)?)\s*(nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)").unwrap()
        });
        static DIST_BETWEEN_RE: OnceLock<Regex> = OnceLock::new();
        let dist_between_re = DIST_BETWEEN_RE.get_or_init(|| {
            Regex::new(r"(?i)between\s+(\d+(?:\.\d+)?)\s+and\s+(\d+(?:\.\d+)?)\s*(nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)").unwrap()
        });
        static DIST_WITHIN_RE: OnceLock<Regex> = OnceLock::new();
        let dist_within_re = DIST_WITHIN_RE.get_or_init(|| {
            Regex::new(r"(?i)(?:within|under|less than|max(?:imum)?)\s+(\d+(?:\.\d+)?)\s*(nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)").unwrap()
        });
        static DIST_ATLEAST_RE: OnceLock<Regex> = OnceLock::new();
        let dist_atleast_re = DIST_ATLEAST_RE.get_or_init(|| {
            Regex::new(r"(?i)(?:at least|over|more than|min(?:imum)?)\s+(\d+(?:\.\d+)?)\s*(nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)").unwrap()
        });

        fn to_nm(val: f64, unit: &str) -> f64 {
            to_nm_unit(val, unit)
        }

        // Bare number+unit with no qualifier: "70nm", "200km", "50 NM".
        // Treat as approximate target distance — set a ±30% window so the generator
        // has room to find airports (e.g. "70nm" → 49–91nm).
        static DIST_BARE_RE: OnceLock<Regex> = OnceLock::new();
        let dist_bare_re = DIST_BARE_RE.get_or_init(|| {
            Regex::new(r"(?i)\b(\d+(?:\.\d+)?)\s*(nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)\b").unwrap()
        });

        if let Some(caps) = dist_or_re.captures(&clean_input) {
            if let (Ok(a), Ok(b)) = (caps[1].parse::<f64>(), caps[3].parse::<f64>()) {
                prompt.user_max_dist_nm =
                    Some(to_nm(a, &caps[2]).max(to_nm(b, &caps[4])));
            }
        } else if let Some(caps) = dist_between_re.captures(&clean_input) {
            if let (Ok(a), Ok(b)) = (caps[1].parse::<f64>(), caps[2].parse::<f64>()) {
                let unit = &caps[3];
                prompt.user_min_dist_nm = Some(to_nm(a, unit));
                prompt.user_max_dist_nm = Some(to_nm(b, unit));
            }
        } else if let Some(caps) = dist_within_re.captures(&clean_input) {
            if let Ok(v) = caps[1].parse::<f64>() {
                prompt.user_max_dist_nm = Some(to_nm(v, &caps[2]));
            }
        } else if let Some(caps) = dist_atleast_re.captures(&clean_input) {
            if let Ok(v) = caps[1].parse::<f64>() {
                prompt.user_min_dist_nm = Some(to_nm(v, &caps[2]));
            }
        } else if let Some(caps) = dist_bare_re.captures(&clean_input) {
            if let Ok(v) = caps[1].parse::<f64>() {
                let nm = to_nm(v, &caps[2]);
                prompt.user_min_dist_nm = Some((nm * 0.7).max(2.0));
                prompt.user_max_dist_nm = Some(nm * 1.3);
            }
        }

        prompt
    }
}

/// Converts a distance value to nautical miles given the unit string captured by a distance regex.
fn to_nm_unit(val: f64, unit: &str) -> f64 {
    let u = unit.to_lowercase();
    if u.starts_with("km") || u.starts_with("kilo") {
        val / 1.852
    } else if u.starts_with("mi") {
        val / 1.151
    } else {
        val // nm, nmi, nautical miles
    }
}

/// Strips common airport-descriptor phrases from a location string so the underlying
/// geographic name can be resolved.  E.g. "high altitude departure airport in Europe" → "europe".
/// Returns a lowercase string; `parse_location` handles case-insensitive matching.
fn clean_location_noise(s: &str) -> String {
    let lower = s.to_lowercase();

    // Strategy 1: extract the geographic name that follows "in/at/near".
    // E.g. "high altitude departure airport in Europe" → "europe"
    //      "any airport near Tokyo"                   → "tokyo"
    // Word boundaries prevent matching "in" inside words like "international".
    static PREP_RE: OnceLock<Regex> = OnceLock::new();
    let prep_re = PREP_RE.get_or_init(|| Regex::new(r"(?i)\b(?:in|at|near)\s+(.+)$").unwrap());
    if let Some(caps) = prep_re.captures(&lower) {
        let loc = caps[1].trim();
        if !loc.is_empty() {
            return loc.to_string();
        }
    }

    // Strategy 2: strip well-defined compound descriptors + standalone airport-type words.
    // Only phrases that are very unlikely to appear in actual geographic names.
    // NOTE: Do NOT strip single words like "mountain", "coastal", "regional" etc.
    // as substring replacement would corrupt place names (e.g. "mountains" → "s").
    const COMPOUND_NOISE: &[&str] = &[
        "high-altitude airport",
        "low-altitude airport",
        "high altitude airport",
        "low altitude airport",
        "international airport",
        "domestic airport",
        "regional airport",
        "local airport",
        "departure airport",
        "arrival airport",
        "destination airport",
        "origin airport",
        "high-altitude",
        "low-altitude",
        "high altitude",
        "low altitude",
        "departure",
        "arrival",
    ];
    const STANDALONE: &[&str] = &["airport", "airfield", "airstrip", "aerodrome"];

    let mut r = lower;
    for w in COMPOUND_NOISE {
        r = r.replace(*w, " ");
    }
    for w in STANDALONE {
        r = r.replace(*w, " ");
    }
    // Collapse whitespace and strip leading prepositions/articles.
    let r = r.split_whitespace().collect::<Vec<_>>().join(" ");
    let r = r.as_str();
    let r = r
        .strip_prefix("in ")
        .or_else(|| r.strip_prefix("at "))
        .or_else(|| r.strip_prefix("near "))
        .or_else(|| r.strip_prefix("the "))
        .or_else(|| r.strip_prefix("a "))
        .or_else(|| r.strip_prefix("an "))
        .unwrap_or(r);
    r.trim().to_string()
}

/// Strips distance constraint phrases from `s` and returns the cleaned string.
/// E.g. "paris within 500nm" → "paris", "europe within 70km or 90nm" → "europe".
fn strip_distance_text(s: &str) -> String {
    static DIST_STRIP_RE: OnceLock<Regex> = OnceLock::new();
    let re = DIST_STRIP_RE.get_or_init(|| {
        Regex::new(
            r"(?i)\s*(?:between\s+\d+(?:\.\d+)?\s+and\s+\d+(?:\.\d+)?\s*(?:nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)|(?:within|under|less than|max(?:imum)?|at least|over|more than)\s+\d+(?:\.\d+)?\s*(?:nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?)(?:\s+or\s+\d+(?:\.\d+)?\s*(?:nmi|nm|km|kilomet(?:re|er)s?|mi(?:les?)?|nautical miles?))?)",
        )
        .unwrap()
    });
    re.replace_all(s, "").trim().to_string()
}

/// Returns true if `s`, after stripping any embedded distance constraint text,
/// is a noise destination like "a destination", "anywhere", "any airport", etc.
fn is_noise_dest(s: &str) -> bool {
    let stripped = strip_distance_text(s);
    matches!(
        stripped.trim().to_lowercase().as_str(),
        "" | "a destination"
            | "any destination"
            | "some destination"
            | "a location"
            | "any location"
            | "any airport"
            | "somewhere"
            | "anywhere"
            | "any place"
            | "anyplace"
            | "a"
            | "any"
            | "the"
            | "destination"
    )
}

fn parse_location(s: &str) -> LocationConstraint {
    let s = s.strip_prefix("the ").unwrap_or(s).trim();
    if s == "here" || s == "current location" {
        LocationConstraint::Region("Here".to_string())
    } else if s == "anywhere" || s == "any" || s == "random" {
        LocationConstraint::Any
    } else if let Some(region) = try_as_region(s) {
        // Check region/city aliases BEFORE the ICAO heuristic.
        // This prevents city names like "Lamu" or "Lima" from being
        // misidentified as ICAO codes.
        region
    } else if has_cjk(s) {
        // When Chinese preprocessing has left a city name followed by English
        // keywords (e.g. "成都 short flight rain 天"), the location regex may
        // capture the full suffix.  Try matching just the leading CJK segment.
        let cjk_prefix: String = s.chars().take_while(|c| !c.is_ascii_alphabetic()).collect();
        let cjk_prefix = cjk_prefix.trim();
        if !cjk_prefix.is_empty() {
            if let Some(region) = try_as_region(cjk_prefix) {
                return region;
            }
        }
        LocationConstraint::AirportName(s.to_string())
    } else if (s.len() >= 4 && s.len() <= 7) && s.chars().all(|c| c.is_alphanumeric()) {
        // Real ICAO codes are 4 characters (EGLL, KJFK, RJAA, …).
        // Allow up to 7 to cover IATA/domestic variants, but 3-char codes like
        // "F70" are FAA facility IDs and are better handled as AirportName so
        // that flight_gen's name-scoring can match them by ICAO id.
        LocationConstraint::ICAO(s.to_uppercase())
    } else {
        // Fallback: if the string contains spaces, the location regex may have
        // captured trailing aircraft/keyword text (e.g. "lirf md80").
        // Try the first whitespace-delimited token as an ICAO code.
        if let Some(first) = s.split_whitespace().next() {
            if first.len() == 4 && first.chars().all(|c| c.is_ascii_alphanumeric()) {
                return LocationConstraint::ICAO(first.to_uppercase());
            }
        }
        LocationConstraint::AirportName(s.to_string())
    }
}

/// Normalizes location string for alias match: lowercase, collapse whitespace, strip "the ", replace comma with space.
fn normalize_for_region_match(s: &str) -> String {
    let s = s.strip_prefix("the ").unwrap_or(s).trim();
    s.replace(',', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Attempts to recognize a string as a geographic region or city.
fn try_as_region(s: &str) -> Option<LocationConstraint> {
    let s = s.strip_prefix("the ").unwrap_or(s).trim();

    // 1. Check explicit alias table FIRST (includes NearCity entries for cities).
    //    This takes priority over RegionIndex to avoid stale region matches
    //    (e.g. "London" matching UK:London region instead of NearCity).
    let key = normalize_for_region_match(s);
    let index = ALIAS_INDEX.get_or_init(|| {
        let raw: HashMap<String, RawAlias> =
            serde_json::from_str(include_str!("geo/location_aliases.json"))
                .expect("location_aliases.json must be valid");

        raw.into_iter()
            .map(|(k, v)| {
                let key = normalize_for_region_match(&k);
                let constraint = match v.alias_type.as_str() {
                    "Region" => LocationConstraint::Region(v.value.as_str().unwrap().to_string()),
                    "NearCity" => {
                        let obj = v.value.as_object().unwrap();
                        LocationConstraint::NearCity {
                            name: obj.get("name").unwrap().as_str().unwrap().to_string(),
                            lat: obj.get("lat").unwrap().as_f64().unwrap(),
                            lon: obj.get("lon").unwrap().as_f64().unwrap(),
                        }
                    }
                    _ => unreachable!("invalid alias type in json"),
                };
                (key, constraint)
            })
            .collect()
    });

    if let Some(constraint) = index.get(key.as_str()) {
        return Some(constraint.clone());
    }

    // 2. Fallback: check RegionIndex for geographic regions not in the explicit table
    let index = crate::geo::RegionIndex::new();
    if let Some(region) = index.search(s) {
        return Some(LocationConstraint::Region(region.id.to_string()));
    }
    None
}

/// Validates an `NLPRulesConfig` for semantic correctness beyond JSON syntax.
/// Returns a list of human-readable error strings; empty means the config is valid.
/// Aircraft rules are not validated (mapped_value is a free-form discovery tag).
pub fn validate_nlp_config(config: &crate::NLPRulesConfig) -> Vec<String> {
    let mut errors = Vec::new();

    let valid_time: &[&str] = &[
        "dawn",
        "sunrise",
        "morning",
        "golden hour",
        "golden",
        "day",
        "daytime",
        "daylight",
        "afternoon",
        "noon",
        "dusk",
        "sunset",
        "evening",
        "twilight",
        "civil twilight",
        "night",
        "midnight",
        "dark",
        "night flight",
        "moonlight",
        "late night",
    ];
    let valid_weather: &[&str] = &[
        "clear",
        "sunny",
        "fair",
        "vfr",
        "visual",
        "clear vfr",
        "cavok",
        "cavu",
        "clear skies",
        "blue sky",
        "easy",
        "relax",
        "scenic",
        "cloudy",
        "overcast",
        "clouds",
        "mvfr",
        "marginal",
        "scattered",
        "few clouds",
        "broken",
        "storm",
        "thunder",
        "thunderstorm",
        "severe",
        "lifr",
        "low ifr",
        "challenge",
        "hard mode",
        "gusty",
        "windy",
        "breezy",
        "turbulent",
        "gusts",
        "calm",
        "still",
        "smooth",
        "no wind",
        "light winds",
        "glassy",
        "snow",
        "blizzard",
        "ice",
        "wintry",
        "winter",
        "frozen",
        "snowy",
        "icy",
        "rain",
        "showers",
        "wet",
        "fog",
        "mist",
        "haze",
        "ifr",
        "instrument",
        "smoky",
    ];
    let valid_surface: &[&str] = &[
        "soft", "grass", "dirt", "gravel", "strip", "unpaved", "hard", "paved", "tarmac",
        "concrete", "asphalt", "water", "seaplane", "float",
    ];
    let valid_type: &[&str] = &[
        "bush",
        "backcountry",
        "remote",
        "stol",
        "regional",
        "commuter",
    ];
    let valid_duration: &[&str] = &[
        "short",
        "hop",
        "quick",
        "sprint",
        "medium",
        "mid",
        "long",
        "long range",
        "haul",
        "long haul",
        "ultra long",
        "intercontinental",
        "transatlantic",
        "transpacific",
        "transcontinental",
    ];

    fn check_category(
        rules: &[crate::NLPRule],
        category: &str,
        valid: &[&str],
        errors: &mut Vec<String>,
    ) {
        for (i, rule) in rules.iter().enumerate() {
            if rule.mapped_value.trim().is_empty() {
                errors.push(format!(
                    "{}[{}] \"{}\": mapped_value cannot be empty.",
                    category, i, rule.name
                ));
            } else if !valid.contains(&rule.mapped_value.to_lowercase().trim()) {
                errors.push(format!(
                    "{}[{}] \"{}\": \"{}\" is not a recognized value. Valid options: {}",
                    category,
                    i,
                    rule.name,
                    rule.mapped_value,
                    valid.join(", ")
                ));
            }
        }
    }

    check_category(&config.time_rules, "time_rules", valid_time, &mut errors);
    check_category(
        &config.weather_rules,
        "weather_rules",
        valid_weather,
        &mut errors,
    );
    check_category(
        &config.surface_rules,
        "surface_rules",
        valid_surface,
        &mut errors,
    );
    check_category(
        &config.flight_type_rules,
        "flight_type_rules",
        valid_type,
        &mut errors,
    );
    check_category(
        &config.duration_rules,
        "duration_rules",
        valid_duration,
        &mut errors,
    );

    // Aircraft rules: only check non-empty mapped_value
    for (i, rule) in config.aircraft_rules.iter().enumerate() {
        if rule.mapped_value.trim().is_empty() {
            errors.push(format!(
                "aircraft_rules[{}] \"{}\": mapped_value cannot be empty.",
                i, rule.name
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_from() {
        let p = FlightPrompt::parse("London to Paris", &crate::NLPRulesConfig::default());
        // London maps to NearCity; Paris also maps to NearCity
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            other => panic!("Paris should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_simple() {
        let p = FlightPrompt::parse(
            "Flight from London to Paris",
            &crate::NLPRulesConfig::default(),
        );
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            other => panic!("Paris should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_full() {
        let p = FlightPrompt::parse(
            "Flight from EGLL to KJFK using a Boeing 747 for 7 hours ignore guardrails",
            &crate::NLPRulesConfig::default(),
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
        let p = FlightPrompt::parse("Just fly for 45 mins", &crate::NLPRulesConfig::default());
        assert_eq!(p.duration_minutes, Some(45));
    }

    #[test]
    fn test_parse_country_as_region() {
        let p = FlightPrompt::parse(
            "Flight from France to Germany",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.origin, Some(LocationConstraint::Region("FR".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("DE".to_string()))
        );
    }

    #[test]
    fn test_parse_us_nickname_as_region() {
        let p = FlightPrompt::parse(
            "Flight from Socal to Norcal",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("US:SoCal".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("US:NorCal".to_string()))
        );
    }

    #[test]
    fn test_parse_abbreviation_as_region() {
        let p = FlightPrompt::parse("Flight from UK to USA", &crate::NLPRulesConfig::default());
        assert_eq!(p.origin, Some(LocationConstraint::Region("UK".to_string())));
        // "usa" is 3 chars, not 4, so not ICAO
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("US".to_string()))
        );
    }

    #[test]
    fn test_parse_from_uk_only() {
        // "from X" without "to Y" must constrain origin so flight is from UK, not e.g. Algeria/France
        let p = FlightPrompt::parse("2 hour flight from UK", &crate::NLPRulesConfig::default());
        assert_eq!(p.origin, Some(LocationConstraint::Region("UK".to_string())));
        assert_eq!(p.destination, None);
        let p2 = FlightPrompt::parse("flight from UK", &crate::NLPRulesConfig::default());
        assert_eq!(
            p2.origin,
            Some(LocationConstraint::Region("UK".to_string()))
        );
        assert_eq!(p2.destination, None);
    }

    #[test]
    fn test_parse_article_stripped() {
        let p = FlightPrompt::parse(
            "Flight from the British Isles to the Caribbean",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.origin, Some(LocationConstraint::Region("BI".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("Caribbean".to_string()))
        );
    }

    #[test]
    fn test_parse_city_maps_to_nearcity() {
        // London and Paris both map to NearCity now
        let p = FlightPrompt::parse(
            "Flight from London to Paris",
            &crate::NLPRulesConfig::default(),
        );
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            other => panic!("Paris should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_london_uk_to_region() {
        let p = FlightPrompt::parse(
            "Flight from London UK to Germany",
            &crate::NLPRulesConfig::default(),
        );
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London UK should be NearCity, got {:?}", other),
        }
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("DE".to_string()))
        );
    }

    #[test]
    fn test_parse_london_to_italy() {
        let p = FlightPrompt::parse(
            "Flight from London to Italy",
            &crate::NLPRulesConfig::default(),
        );
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "London"),
            other => panic!("London should be NearCity, got {:?}", other),
        }
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("IT".to_string()))
        );
    }

    #[test]
    fn test_parse_rome_italy_as_nearcity() {
        // "Rome Italy" now maps to NearCity(Rome) — coordinates (41.9°N, 12.5°E) disambiguate from Rome GA
        let p = FlightPrompt::parse(
            "Flight from EGMC to Rome Italy",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGMC".to_string())));
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Rome"),
            other => panic!("Rome Italy should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_rome_comma_italy_as_nearcity() {
        let p = FlightPrompt::parse(
            "Flight from London to Rome, Italy",
            &crate::NLPRulesConfig::default(),
        );
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Rome"),
            other => panic!("Rome, Italy should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_paris_france_as_nearcity() {
        let p = FlightPrompt::parse(
            "Flight from EGLL to Paris France",
            &crate::NLPRulesConfig::default(),
        );
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            other => panic!("Paris France should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_icao_still_icao() {
        let p = FlightPrompt::parse(
            "Flight from EGLL to LFPG",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::ICAO("LFPG".to_string()))
        );
    }
    #[test]
    fn test_parse_f70_to_alaska() {
        let p = FlightPrompt::parse("F70 to Alaska", &crate::NLPRulesConfig::default());
        // F70 is 3 chars, so parsed as Name, not ICAO (valid behavior as flight_gen handles ID match in name search)
        match p.origin {
            Some(LocationConstraint::AirportName(ref s)) if s == "f70" => {}
            _ => panic!("Origin should be AirportName(f70), got {:?}", p.origin),
        }
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("US:AK".to_string()))
        );
    }

    #[test]
    fn test_parse_nairobi_to_lamu() {
        let p = FlightPrompt::parse("Nairobi to Lamu", &crate::NLPRulesConfig::default());
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Nairobi"),
            other => panic!("Nairobi should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Lamu"),
            other => panic!("Lamu should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_nairobi_to_mombasa() {
        let p = FlightPrompt::parse("Nairobi to Mombasa", &crate::NLPRulesConfig::default());
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Nairobi"),
            other => panic!("Nairobi should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Mombasa"),
            other => panic!("Mombasa should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_tokyo_to_bangkok() {
        let p = FlightPrompt::parse("Tokyo to Bangkok", &crate::NLPRulesConfig::default());
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Tokyo"),
            other => panic!("Tokyo should be NearCity, got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Bangkok"),
            other => panic!("Bangkok should be NearCity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_icao_still_works_after_reorder() {
        // Ensure the parse_location reorder didn't break real ICAO codes
        let p = FlightPrompt::parse(
            "Flight from EGLL to KJFK",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::ICAO("KJFK".to_string()))
        );
    }

    /// "washington" alone must resolve to Washington State (US:WA), NOT Washington D.C.
    /// Bare "washington" is ambiguous; the Pacific Northwest state is the more useful
    /// flight-gen target. Use "washington dc" or "dc" to get the capital.
    #[test]
    fn test_parse_washington_resolves_to_wa_state() {
        let p = FlightPrompt::parse("F70 to Washington", &crate::NLPRulesConfig::default());
        // F70 is 3 chars → AirportName
        assert!(
            matches!(&p.origin, Some(LocationConstraint::AirportName(s)) if s == "f70"),
            "Origin should be AirportName(f70), got {:?}",
            p.origin
        );
        // "Washington" must be Washington State, not D.C.
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("US:WA".to_string())),
            "Bare 'washington' should resolve to US:WA, got {:?}",
            p.destination
        );
    }

    /// "washington state" must resolve to Region(US:WA).
    #[test]
    fn test_parse_washington_state_explicit() {
        let p = FlightPrompt::parse("Washington State", &crate::NLPRulesConfig::default());
        assert_eq!(
            p.destination.or(p.origin.clone()),
            Some(LocationConstraint::Region("US:WA".to_string())),
            "'Washington State' should be Region(US:WA)"
        );
    }

    /// "washington dc" and "dc" must still resolve to the capital NearCity.
    #[test]
    fn test_parse_washington_dc_still_works() {
        let p = FlightPrompt::parse("fly to washington dc", &crate::NLPRulesConfig::default());
        assert!(
            matches!(&p.destination, Some(LocationConstraint::NearCity { name, .. }) if name == "Washington DC"),
            "washington dc should still be NearCity(Washington DC), got {:?}",
            p.destination
        );
        let p2 = FlightPrompt::parse("fly to dc", &crate::NLPRulesConfig::default());
        assert!(
            matches!(&p2.destination, Some(LocationConstraint::NearCity { name, .. }) if name == "Washington DC"),
            "dc should still be NearCity(Washington DC), got {:?}",
            p2.destination
        );
    }

    #[test]
    fn test_parse_civilian_airliner_with_landing() {
        let p = FlightPrompt::parse(
            "Flight from Scotland to Italy using civilian airliner landing at civilian airport",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(
            p.origin,
            Some(LocationConstraint::Region("UK:Scotland".to_string()))
        );
        assert_eq!(
            p.destination,
            Some(LocationConstraint::Region("IT".to_string()))
        );
        match p.aircraft {
            Some(AircraftConstraint::Tag(t)) => {
                assert_eq!(t, "Airliner", "Should normalize to Airliner")
            }
            _ => panic!(
                "Aircraft should be mapped to Airliner, got {:?}",
                p.aircraft
            ),
        }
    }

    #[test]
    fn test_parse_time_and_weather() {
        let p = FlightPrompt::parse(
            "Night flight from London to Paris in a storm",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.keywords.time, Some(TimeKeyword::Night));
        assert_eq!(p.keywords.weather, Some(WeatherKeyword::Storm));

        let p2 = FlightPrompt::parse(
            "Morning departure to KJFK in clear skies",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p2.keywords.time, Some(TimeKeyword::Dawn));
        assert_eq!(p2.keywords.weather, Some(WeatherKeyword::Clear));

        let p3 = FlightPrompt::parse(
            "fly in heavy snow at dusk",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p3.keywords.time, Some(TimeKeyword::Dusk));
        assert_eq!(p3.keywords.weather, Some(WeatherKeyword::Snow));
    }

    #[test]
    fn test_parse_thunderstorm() {
        let p = FlightPrompt::parse(
            "Flight to KSFO in a thunderstorm",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.keywords.weather, Some(WeatherKeyword::Storm));
    }

    #[test]
    fn test_parse_vfr_ifr() {
        let p1 = FlightPrompt::parse("VFR flight to KLAX", &crate::NLPRulesConfig::default());
        let p2 = FlightPrompt::parse("IFR flight to KJFK", &crate::NLPRulesConfig::default());

        assert_eq!(p1.keywords.weather, Some(WeatherKeyword::Clear));
        assert_eq!(p2.keywords.weather, Some(WeatherKeyword::Fog));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Chinese NLP tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_chinese_from_to() {
        // "从北京到上海" — "from Beijing to Shanghai"
        let p = FlightPrompt::parse("从北京到上海", &crate::NLPRulesConfig::default());
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Beijing"),
            other => panic!("Origin should be NearCity(Beijing), got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Shanghai"),
            other => panic!("Destination should be NearCity(Shanghai), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chinese_city_chars() {
        // Bare Chinese city name should resolve to NearCity
        let p = FlightPrompt::parse("北京", &crate::NLPRulesConfig::default());
        let loc = p.destination.or(p.origin);
        match loc {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Beijing"),
            other => panic!("北京 should be NearCity(Beijing), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chinese_new_city() {
        // 成都 — new city added in this PR
        let p = FlightPrompt::parse("成都", &crate::NLPRulesConfig::default());
        let loc = p.destination.or(p.origin);
        match loc {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Chengdu"),
            other => panic!("成都 should be NearCity(Chengdu), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chinese_to_only() {
        // "飞往广州" — destination only, no origin
        let p = FlightPrompt::parse("飞往广州", &crate::NLPRulesConfig::default());
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Guangzhou"),
            other => panic!("Destination should be NearCity(Guangzhou), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chinese_duration() {
        let p = FlightPrompt::parse("短途飞行", &crate::NLPRulesConfig::default());
        assert_eq!(
            p.keywords.duration,
            Some(DurationKeyword::Short),
            "短途飞行 should produce DurationKeyword::Short"
        );
    }

    #[test]
    fn test_parse_chinese_weather() {
        let p = FlightPrompt::parse("下雨天飞行", &crate::NLPRulesConfig::default());
        assert_eq!(
            p.keywords.weather,
            Some(WeatherKeyword::Rain),
            "下雨 should produce WeatherKeyword::Rain"
        );
    }

    #[test]
    fn test_parse_chinese_aircraft() {
        let p = FlightPrompt::parse("驾驶直升机", &crate::NLPRulesConfig::default());
        match &p.aircraft {
            Some(AircraftConstraint::Tag(t)) => assert!(
                t.to_lowercase().contains("helicopter"),
                "Aircraft tag should contain 'helicopter', got {:?}",
                t
            ),
            other => panic!("Aircraft should be Tag(helicopter), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chinese_country() {
        let p = FlightPrompt::parse("中国", &crate::NLPRulesConfig::default());
        let loc = p.destination.or(p.origin);
        assert_eq!(
            loc,
            Some(LocationConstraint::Region("CN".to_string())),
            "中国 should be Region(CN)"
        );
    }

    #[test]
    fn test_parse_chinese_full_prompt() {
        // "从北京到成都短途飞行下雨天" — origin, dest, duration, weather all parsed
        let p = FlightPrompt::parse(
            "从北京到成都短途飞行下雨天",
            &crate::NLPRulesConfig::default(),
        );
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Beijing"),
            other => panic!("Origin should be NearCity(Beijing), got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Chengdu"),
            other => panic!("Destination should be NearCity(Chengdu), got {:?}", other),
        }
        assert_eq!(
            p.keywords.duration,
            Some(DurationKeyword::Short),
            "Should detect Short duration"
        );
        assert_eq!(
            p.keywords.weather,
            Some(WeatherKeyword::Rain),
            "Should detect Rain weather"
        );
    }

    #[test]
    fn test_parse_chinese_mixed_language_a320() {
        // Mixed Chinese + ASCII: "从北京到上海短途飞行下雨天使用A320在凌晨"
        // Includes "在" particle + time word after aircraft — must not bleed into tag.
        let p = FlightPrompt::parse(
            "从北京到上海短途飞行下雨天使用A320在凌晨",
            &crate::NLPRulesConfig::default(),
        );
        match &p.origin {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Beijing"),
            other => panic!("Origin should be NearCity(Beijing), got {:?}", other),
        }
        match &p.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Shanghai"),
            other => panic!("Destination should be NearCity(Shanghai), got {:?}", other),
        }
        assert_eq!(
            p.keywords.duration,
            Some(DurationKeyword::Short),
            "Should detect Short duration"
        );
        assert_eq!(
            p.keywords.weather,
            Some(WeatherKeyword::Rain),
            "Should detect Rain weather"
        );
        match &p.aircraft {
            Some(AircraftConstraint::Tag(t)) => assert!(
                t.to_lowercase().contains("a320"),
                "Aircraft tag should contain 'a320', got {:?}",
                t
            ),
            Some(AircraftConstraint::ICAO(t)) => assert!(
                t.to_uppercase().contains("A320"),
                "Aircraft ICAO should contain 'A320', got {:?}",
                t
            ),
            other => panic!("Aircraft should be Tag or ICAO containing a320, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chinese_heavy_rain_is_storm() {
        // 暴雨 (torrential rain) → storm intensity
        let p = FlightPrompt::parse("暴雨飞行", &crate::NLPRulesConfig::default());
        assert_eq!(
            p.keywords.weather,
            Some(WeatherKeyword::Storm),
            "暴雨 should produce WeatherKeyword::Storm"
        );
    }

    #[test]
    fn test_parse_bare_distance() {
        let cfg = crate::NLPRulesConfig::default();

        // Bare "70NM" → ±30% window (49–91nm)
        let p = FlightPrompt::parse("flight from EGMC heading south 70NM", &cfg);
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGMC".to_string())));
        assert!(p.direction_bearing.is_some(), "south bearing should be set");
        let min = p.user_min_dist_nm.expect("min dist should be set");
        let max = p.user_max_dist_nm.expect("max dist should be set");
        assert!((min - 49.0).abs() < 1.0, "min ~49nm, got {min}");
        assert!((max - 91.0).abs() < 1.0, "max ~91nm, got {max}");

        // Bare km → converted
        let p2 = FlightPrompt::parse("fly east 100km", &cfg);
        let min2 = p2.user_min_dist_nm.expect("min dist km");
        let max2 = p2.user_max_dist_nm.expect("max dist km");
        // 100km = 53.99nm; ±30% → ~37.8nm–70.2nm
        assert!(min2 > 35.0 && min2 < 40.0, "min ~37.8nm, got {min2}");
        assert!(max2 > 68.0 && max2 < 73.0, "max ~70.2nm, got {max2}");

        // Qualified patterns are not affected by bare fallback
        let p3 = FlightPrompt::parse("within 50nm", &cfg);
        assert_eq!(p3.user_min_dist_nm, None);
        assert!((p3.user_max_dist_nm.unwrap() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_direction_variants() {
        let cfg = crate::NLPRulesConfig::default();

        // -bound
        let p = FlightPrompt::parse("northbound from EGLL", &cfg);
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!((min_b - 315.0).abs() < 0.1 && (max_b - 45.0).abs() < 0.1, "northbound → north");

        let p = FlightPrompt::parse("from KSFO southbound", &cfg);
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("KSFO".to_string())));
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!((min_b - 135.0).abs() < 0.1 && (max_b - 225.0).abs() < 0.1, "southbound → south");

        let p = FlightPrompt::parse("eastbound from KJFK", &cfg);
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("KJFK".to_string())));
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!((min_b - 45.0).abs() < 0.1 && (max_b - 135.0).abs() < 0.1, "eastbound → east");

        // -ward/-wards
        let p = FlightPrompt::parse("fly northward from EGLL", &cfg);
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!((min_b - 315.0).abs() < 0.1 && (max_b - 45.0).abs() < 0.1, "northward → north");

        let p = FlightPrompt::parse("westwards from LIRF", &cfg);
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!((min_b - 225.0).abs() < 0.1 && (max_b - 315.0).abs() < 0.1, "westwards → west");

        // "towards the"
        let p = FlightPrompt::parse("flying towards the south", &cfg);
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!((min_b - 135.0).abs() < 0.1 && (max_b - 225.0).abs() < 0.1, "towards the south");

        // "flying" as prefix
        let p = FlightPrompt::parse("flying east from EGLL", &cfg);
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!((min_b - 45.0).abs() < 0.1 && (max_b - 135.0).abs() < 0.1, "flying east");
    }

    #[test]
    fn test_parse_from_x_heading_direction() {
        // "flight from F70 heading south" — origin must be F70, bearing must be south.
        // Bug: "heading" was not a LOCATION_TERMINATOR so origin_str was "f70 heading south",
        // and the full from_re match consumed "heading south" before direction parsing ran.
        let p = FlightPrompt::parse("flight from F70 heading south", &crate::NLPRulesConfig::default());
        assert!(
            matches!(&p.origin, Some(LocationConstraint::AirportName(n)) if n.to_uppercase() == "F70"),
            "origin should be AirportName(F70), got {:?}", p.origin
        );
        assert!(p.direction_bearing.is_some(), "bearing should be set");
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!(
            (min_b - 135.0).abs() < 0.1 && (max_b - 225.0).abs() < 0.1,
            "south bearing expected, got ({min_b}, {max_b})"
        );

        // "from EGLL going east" — similar pattern
        let p2 = FlightPrompt::parse("from EGLL going east", &crate::NLPRulesConfig::default());
        assert_eq!(p2.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert!(p2.direction_bearing.is_some(), "bearing should be set");
        let (min_b, max_b) = p2.direction_bearing.unwrap();
        assert!((min_b - 45.0).abs() < 0.1 && (max_b - 135.0).abs() < 0.1, "east bearing expected");
    }

    #[test]
    fn test_parse_direction_of_location() {
        // "fly north of EGMC" → origin=EGMC, direction=north
        let p = FlightPrompt::parse("fly north of EGMC", &crate::NLPRulesConfig::default());
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGMC".to_string())));
        assert!(p.direction_bearing.is_some(), "should have direction bearing");
        let (min_b, max_b) = p.direction_bearing.unwrap();
        assert!((min_b - 315.0).abs() < 0.1 && (max_b - 45.0).abs() < 0.1, "north bearing expected");

        // "northeast of London" → origin=NearCity(London), direction=northeast
        let p2 = FlightPrompt::parse("northeast of London", &crate::NLPRulesConfig::default());
        assert!(
            matches!(&p2.origin, Some(LocationConstraint::NearCity { name, .. }) if name == "London"),
            "origin should be NearCity(London), got {:?}", p2.origin
        );
        let (min_b, max_b) = p2.direction_bearing.unwrap();
        assert!((min_b - 22.5).abs() < 0.1 && (max_b - 67.5).abs() < 0.1, "northeast bearing expected");

        // "north of EGMC to Paris" → origin=EGMC, dest=Paris (NearCity), direction=north
        let p3 = FlightPrompt::parse("north of EGMC to Paris", &crate::NLPRulesConfig::default());
        assert_eq!(p3.origin, Some(LocationConstraint::ICAO("EGMC".to_string())));
        assert!(
            matches!(&p3.destination, Some(LocationConstraint::NearCity { name, .. }) if name == "Paris"),
            "dest should be NearCity(Paris), got {:?}", p3.destination
        );
        assert!(p3.direction_bearing.is_some(), "should have direction bearing");
    }

    #[test]
    fn test_parse_chinese_aircraft_tag_clean() {
        // "使用A320在凌晨" — the "在" particle and "凌晨" (→ "night") must NOT bleed into
        // the aircraft tag; tag should be exactly "a320".
        let p = FlightPrompt::parse("使用A320在凌晨", &crate::NLPRulesConfig::default());
        match &p.aircraft {
            Some(AircraftConstraint::Tag(t)) => {
                assert!(
                    t.to_lowercase() == "a320",
                    "Aircraft tag should be 'a320', got {:?}",
                    t
                );
            }
            Some(AircraftConstraint::ICAO(t)) => {
                assert!(
                    t.to_uppercase() == "A320",
                    "Aircraft ICAO should be 'A320', got {:?}",
                    t
                );
            }
            other => panic!("Expected Tag(a320) or ICAO(A320), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chinese_late_night_is_night() {
        // 凌晨 (early hours before dawn) → Night
        let p = FlightPrompt::parse("凌晨飞行", &crate::NLPRulesConfig::default());
        assert_eq!(
            p.keywords.time,
            Some(TimeKeyword::Night),
            "凌晨 should produce TimeKeyword::Night"
        );
    }

    #[test]
    fn test_parse_on_an_md80() {
        // "on an MD-80" should parse LIRF as destination and MD-80 as aircraft
        let p = FlightPrompt::parse(
            "Flight from EGLL to LIRF on an MD-80",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::ICAO("LIRF".to_string()))
        );
        match p.aircraft {
            Some(AircraftConstraint::Tag(ref t)) => {
                assert!(
                    t.to_lowercase().contains("md-80") || t.to_lowercase().contains("md80"),
                    "Aircraft tag should contain 'md-80', got {:?}",
                    t
                );
            }
            other => panic!("Expected Tag containing 'md-80', got {:?}", other),
        }
    }

    #[test]
    fn test_parse_lirf_md80_no_connector() {
        // "LIRF MD80" without a connector word should still parse LIRF as dest.
        let p = FlightPrompt::parse(
            "Flight from EGLL to LIRF MD80",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert_eq!(
            p.destination,
            Some(LocationConstraint::ICAO("LIRF".to_string()))
        );
    }

    #[test]
    fn test_parse_fly_to_on_a_737() {
        // "fly to Alaska on a 737" — dest-only with "on" connector
        let p = FlightPrompt::parse("fly to Alaska on a 737", &crate::NLPRulesConfig::default());
        assert!(
            matches!(p.destination, Some(LocationConstraint::Region(ref r)) if r == "US:AK"),
            "Destination should be Alaska region (US:AK), got {:?}",
            p.destination
        );
        match p.aircraft {
            Some(AircraftConstraint::Tag(ref t)) => {
                assert!(
                    t.contains("737"),
                    "Aircraft tag should contain '737', got {:?}",
                    t
                );
            }
            other => panic!("Expected Tag containing '737', got {:?}", other),
        }
    }

    #[test]
    fn test_parse_new_connectors_and_terminators() {
        // "aboard" and "at" are new
        let p = FlightPrompt::parse(
            "Flight aboard a 737 at dusk",
            &crate::NLPRulesConfig::default(),
        );
        match &p.aircraft {
            Some(AircraftConstraint::Tag(t)) => assert!(t.contains("737")),
            _ => panic!("Expected 737, got {:?}", p.aircraft),
        }
        assert_eq!(p.keywords.time, Some(TimeKeyword::Dusk));

        // "via" and "during" are new terminators
        let p2 = FlightPrompt::parse(
            "EGLL to Paris via London during a storm",
            &crate::NLPRulesConfig::default(),
        );
        assert_eq!(
            p2.origin,
            Some(LocationConstraint::ICAO("EGLL".to_string()))
        );
        match &p2.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Paris"),
            _ => panic!("Expected NearCity(Paris), got {:?}", p2.destination),
        }
        assert_eq!(p2.keywords.weather, Some(WeatherKeyword::Storm));
    }

    #[test]
    fn test_weather_as_aircraft_false_positive() {
        // "vfr conditions" should NOT be parsed as an aircraft name
        let p = FlightPrompt::parse("fly in vfr conditions", &crate::NLPRulesConfig::default());
        assert_eq!(p.keywords.weather, Some(WeatherKeyword::Clear));
        assert!(
            p.aircraft.is_none(),
            "Weather term should not be parsed as aircraft"
        );

        let p2 = FlightPrompt::parse("flight in a storm", &crate::NLPRulesConfig::default());
        assert_eq!(p2.keywords.weather, Some(WeatherKeyword::Storm));
        assert!(
            p2.aircraft.is_none(),
            "Storm should not be parsed as aircraft"
        );
    }

    #[test]
    fn test_external_aliases_from_json() {
        // Test a few specific entries that were moved to JSON
        let p1 = FlightPrompt::parse("Flight to british isles", &crate::NLPRulesConfig::default());
        let p2 = FlightPrompt::parse("Flight to nyc", &crate::NLPRulesConfig::default());
        let p3 = FlightPrompt::parse("Flight to 成都", &crate::NLPRulesConfig::default());

        assert_eq!(
            p1.destination,
            Some(LocationConstraint::Region("BI".to_string()))
        );
        match &p2.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "New York"),
            _ => panic!("Expected NearCity(New York), got {:?}", p2.destination),
        }
        match &p3.destination {
            Some(LocationConstraint::NearCity { name, .. }) => assert_eq!(name, "Chengdu"),
            _ => panic!("Expected NearCity(Chengdu), got {:?}", p3.destination),
        }
    }

    #[test]
    fn test_parse_km_and_nmi_units() {
        let cfg = crate::NLPRulesConfig::default();

        // "kilometers" alias
        let p = FlightPrompt::parse("within 100 kilometers", &cfg);
        let max = p.user_max_dist_nm.expect("within 100 kilometers should set max");
        assert!((max - 53.99).abs() < 0.5, "100km ≈ 53.99nm, got {max}");

        // "kilometre" (British spelling)
        let p2 = FlightPrompt::parse("within 200 kilometres", &cfg);
        let max2 = p2.user_max_dist_nm.expect("within 200 kilometres should set max");
        assert!((max2 - 107.99).abs() < 0.5, "200km ≈ 107.99nm, got {max2}");

        // "nmi" alias
        let p3 = FlightPrompt::parse("within 50 nmi", &cfg);
        let max3 = p3.user_max_dist_nm.expect("within 50 nmi should set max");
        assert!((max3 - 50.0).abs() < 0.1, "50nmi = 50nm, got {max3}");

        // "at least N km"
        let p4 = FlightPrompt::parse("at least 300 km", &cfg);
        let min4 = p4.user_min_dist_nm.expect("at least 300km should set min");
        assert!((min4 - 161.99).abs() < 0.5, "300km ≈ 162nm, got {min4}");
    }

    #[test]
    fn test_parse_dual_unit_distance() {
        let cfg = crate::NLPRulesConfig::default();

        // "within 70KM or 90NM" — take the larger bound (90nm)
        let p = FlightPrompt::parse("flight to Germany within 70km or 90nm", &cfg);
        let max = p.user_max_dist_nm.expect("dual-unit should set max");
        // 70km = 37.8nm; 90nm = 90nm → max should be 90nm
        assert!((max - 90.0).abs() < 0.5, "should take larger bound 90nm, got {max}");

        // "within 200nm or 100km" — 200nm > 100km(54nm) → take 200nm
        let p2 = FlightPrompt::parse("within 200nm or 100km", &cfg);
        let max2 = p2.user_max_dist_nm.expect("dual-unit should set max");
        assert!((max2 - 200.0).abs() < 0.5, "should take 200nm, got {max2}");
    }

    #[test]
    fn test_parse_noise_destination() {
        let cfg = crate::NLPRulesConfig::default();

        // "a destination" → Any
        let p = FlightPrompt::parse("from EGLL to a destination", &cfg);
        assert_eq!(p.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert_eq!(p.destination, Some(LocationConstraint::Any));

        // "a destination within 70km or 90nm" → Any + distance constraint
        let p2 = FlightPrompt::parse("from EGLL to a destination within 70km or 90nm", &cfg);
        assert_eq!(p2.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert_eq!(p2.destination, Some(LocationConstraint::Any));
        let max2 = p2.user_max_dist_nm.expect("distance should be set");
        assert!((max2 - 90.0).abs() < 0.5, "should take larger bound 90nm, got {max2}");

        // "anywhere" → Any
        let p3 = FlightPrompt::parse("from KSFO to anywhere", &cfg);
        assert_eq!(p3.destination, Some(LocationConstraint::Any));
    }

    #[test]
    fn test_parse_location_descriptor_noise() {
        let cfg = crate::NLPRulesConfig::default();

        // "high altitude departure airport in Europe" → origin should resolve to Europe
        let p = FlightPrompt::parse(
            "high altitude departure airport in Europe to a destination within 70km or 90nm",
            &cfg,
        );
        assert!(
            matches!(&p.origin, Some(LocationConstraint::Region(r)) if r == "EU"),
            "origin should be EU region, got {:?}",
            p.origin
        );
        assert_eq!(p.destination, Some(LocationConstraint::Any));
        let max = p.user_max_dist_nm.expect("distance should be set");
        assert!((max - 90.0).abs() < 0.5, "max dist should be 90nm (larger bound), got {max}");

        // "international airport in France" as origin
        let p2 = FlightPrompt::parse("from international airport in France to EGLL", &cfg);
        assert!(
            matches!(&p2.origin, Some(LocationConstraint::Region(r)) if r == "FR"),
            "origin should be FR region, got {:?}",
            p2.origin
        );
        assert_eq!(p2.destination, Some(LocationConstraint::ICAO("EGLL".to_string())));

        // Distance embedded in dest_str, but actual location is Paris
        let p3 = FlightPrompt::parse("from EGLL to paris within 500nm", &cfg);
        assert_eq!(p3.origin, Some(LocationConstraint::ICAO("EGLL".to_string())));
        assert!(
            matches!(&p3.destination, Some(LocationConstraint::NearCity { name, .. }) if name == "Paris"),
            "dest should be NearCity(Paris), got {:?}",
            p3.destination
        );
        let max3 = p3.user_max_dist_nm.expect("500nm should be set");
        assert!((max3 - 500.0).abs() < 0.5, "max should be 500nm, got {max3}");
    }
}
