use crate::apt_dat::{Airport, AirportType, AptDatParser, SurfaceType};
use crate::discovery::{AddonType, DiscoveredAddon};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::path::Path;
use std::sync::OnceLock;
use x_adox_bitnet::flight_prompt::{
    AircraftConstraint, DurationKeyword, FlightPrompt, LocationConstraint, SurfaceKeyword,
    TimeKeyword, TypeKeyword,
};
use x_adox_bitnet::geo::RegionIndex;
use x_adox_bitnet::HeuristicsConfig;

/// London area bounds (lat 51–52, lon -1 to 0.6). Excludes e.g. Great Yarmouth (52.6°N, 1.7°E).
/// Used when region is UK:London so origin/dest are always restricted to London even if region index is stale.
fn in_bounds_london(lat: f64, lon: f64) -> bool {
    (51.0..=52.0).contains(&lat) && (-1.0..=0.6).contains(&lon)
}

/// Loads the default airport database from X-Plane root (Option B: guaranteed base layer).
/// Tries Resources/default scenery/default apt dat first, then Global Scenery/Global Airports.
/// Returns combined, deduplicated list (by ICAO id). Used by flight gen when INI packs lack coverage.
pub fn load_base_airports(xplane_root: &Path) -> Vec<Airport> {
    let mut all = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    let resources_apt = xplane_root
        .join("Resources")
        .join("default scenery")
        .join("default apt dat")
        .join("Earth nav data")
        .join("apt.dat");
    if resources_apt.exists() {
        if let Ok(airports) = AptDatParser::parse_file(&resources_apt) {
            log::info!(
                "[flight_gen] Base layer: loaded {} airports from Resources",
                airports.len()
            );
            for a in airports {
                if seen_ids.insert(a.id.to_uppercase()) {
                    all.push(a);
                }
            }
        }
    }

    let global_apt = xplane_root
        .join("Global Scenery")
        .join("Global Airports")
        .join("Earth nav data")
        .join("apt.dat");
    if global_apt.exists() {
        if let Ok(airports) = AptDatParser::parse_file(&global_apt) {
            log::info!(
                "[flight_gen] Base layer: loaded {} airports from Global Scenery",
                airports.len()
            );
            let before = all.len();
            for a in airports {
                if seen_ids.insert(a.id.to_uppercase()) {
                    all.push(a);
                }
            }
            if all.len() > before {
                log::info!(
                    "[flight_gen] Base layer: added {} from Global Scenery (deduped)",
                    all.len() - before
                );
            }
        }
    }

    all.sort_by(|a, b| a.id.cmp(&b.id));
    all.dedup_by(|a, b| a.id.eq_ignore_ascii_case(&b.id));
    all
}

/// Pre-computed airport pool for high-performance lookups.
pub struct AirportPool<'a> {
    pub airports: Vec<&'a Airport>,
    pub icao_map: HashMap<&'a str, &'a Airport>,
    pub name_map: HashMap<String, Vec<usize>>, // Lowercase name -> indices in airports
    pub search_names: Vec<String>,             // Parallel to airports, pre-lowercased
}

impl<'a> AirportPool<'a> {
    pub fn new(source: &'a [Airport]) -> Self {
        let airports: Vec<&Airport> = source.iter().collect();
        let mut icao_map = HashMap::with_capacity(airports.len());
        let mut name_map = HashMap::with_capacity(airports.len());
        let mut search_names = Vec::with_capacity(airports.len());
        for (i, apt) in airports.iter().enumerate() {
            icao_map.insert(apt.id.as_str(), *apt);
            let lower = apt.name.to_lowercase();
            name_map
                .entry(lower.clone())
                .or_insert_with(Vec::new)
                .push(i);
            search_names.push(lower);
        }
        Self {
            airports,
            icao_map,
            name_map,
            search_names,
        }
    }
}

/// A point of interest within ~10 nm of an airport (landmark, event, etc.).
#[derive(Debug, Clone)]
pub struct PointOfInterest {
    pub name: String,
    pub kind: String,
    pub snippet: String,
    /// Distance from airport in nautical miles, if known.
    pub distance_nm: Option<f64>,
}

/// History and flavor for one airport (snippet + nearby POIs).
#[derive(Debug, Clone)]
pub struct AirportContext {
    pub icao: String,
    pub snippet: String,
    pub points_nearby: Vec<PointOfInterest>,
}

/// Combined context for origin and destination (Phase 1: optional on FlightPlan).
#[derive(Debug, Clone)]
pub struct FlightContext {
    pub origin: AirportContext,
    pub destination: AirportContext,
}

/// File format for one airport in flight_context.json and cache (POIs have lat/lon; we filter by 10 nm).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AirportContextFile {
    pub snippet: String,
    #[serde(default)]
    pub points_nearby: Vec<PoiFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoiFile {
    pub name: String,
    #[serde(default)]
    pub kind: String,
    pub snippet: String,
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub score: i32,
}

/// Radius in nautical miles for "points nearby" (Phase 2).
const POI_RADIUS_NM: f64 = 10.0;

/// Subdir under config for Phase 2b API/LLM cache (one file per ICAO).
pub const FLIGHT_CONTEXT_CACHE_DIR: &str = "flight_context_cache";

/// Bundled default airport context (Option B). Embedded at compile time; load order uses this first.
fn get_bundled_flight_context_raw() -> &'static str {
    include_str!("../data/flight_context_bundle.json")
}

/// Returns the bundled default flight context map (ICAO → AirportContextFile). Use with load_flight_context_with_bundled.
pub fn get_bundled_flight_context() -> std::collections::BTreeMap<String, AirportContextFile> {
    serde_json::from_str(get_bundled_flight_context_raw()).unwrap_or_default()
}

/// Curated 10 nm POIs per ICAO (merged with bundle/cache at load time). Add entries here for historical/nearby points.
fn get_poi_overlay_raw() -> &'static str {
    include_str!("../data/flight_context_pois_overlay.json")
}

fn get_poi_overlay() -> std::collections::BTreeMap<String, Vec<PoiFile>> {
    serde_json::from_str(get_poi_overlay_raw()).unwrap_or_default()
}

/// Embedded OurAirports-derived CSV (ident,title). Parsed at runtime to build ICAO → Wikipedia map.
fn get_icao_to_wikipedia_csv_raw() -> &'static str {
    include_str!("../data/icao_to_wikipedia.csv")
}

static ICAO_TO_WIKIPEDIA: OnceLock<BTreeMap<String, String>> = OnceLock::new();

/// Parses the embedded CSV once and returns the map. Fallback: empty map on parse error.
fn parse_icao_to_wikipedia_csv() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let raw = get_icao_to_wikipedia_csv_raw();
    let mut rdr = csv::Reader::from_reader(Cursor::new(raw));
    for record in rdr.records().flatten() {
        if record.len() >= 2 {
            let ident = record[0].trim().to_string();
            let title = record[1].trim().to_string();
            if !ident.is_empty() && !title.is_empty() {
                map.insert(ident, title);
            }
        }
    }
    map
}

/// Returns ICAO → Wikipedia page title (underscore form) for the summary API. Built at runtime from embedded OurAirports-derived CSV.
pub fn get_icao_to_wikipedia() -> &'static BTreeMap<String, String> {
    ICAO_TO_WIKIPEDIA.get_or_init(parse_icao_to_wikipedia_csv)
}

/// Fallback (lat, lon) when an airport from the plan has no coordinates (e.g. from a pack without apt.dat). Ensures we can still fetch POIs.
fn fallback_coords(icao: &str) -> Option<(f64, f64)> {
    let (lat, lon) = match icao.to_uppercase().as_str() {
        "EGMC" => (51.5703, 0.6933),  // London Southend
        "EGLL" => (51.4700, -0.4543), // London Heathrow
        "LFPG" => (49.0097, 2.5478),  // Paris CDG
        "LIMB" => (45.5422, 9.2033),  // Bresso
        "LIPX" => (45.3953, 10.8885), // Verona Villafranca
        "LIBV" => (40.7678, 16.9333), // Gioia del Colle
        _ => return None,
    };
    Some((lat, lon))
}

/// Returns (lat, lon) for an airport: from the airport struct, or fallback for known ICAOs when missing.
pub fn airport_coords_for_poi_fetch(airport: &Airport) -> Option<(f64, f64)> {
    airport
        .lat
        .and_then(|lat| airport.lon.map(|lon| (lat, lon)))
        .or_else(|| fallback_coords(&airport.id))
}

/// Loads flight context from a curated JSON file (Phase 2a). Keys are ICAO codes; POIs (file + overlay) filtered to within ~10 nm.
/// Returns None if the file is missing or invalid.
pub fn load_flight_context(
    path: &Path,
    origin: &Airport,
    destination: &Airport,
) -> Option<FlightContext> {
    let data = std::fs::read_to_string(path).ok()?;
    let map: std::collections::BTreeMap<String, AirportContextFile> =
        serde_json::from_str(&data).ok()?;
    let overlay = get_poi_overlay();
    let origin_ctx = build_airport_context(
        origin,
        map.get(&*origin.id)
            .or_else(|| map.get(&origin.id.to_uppercase())),
        overlay
            .get(&*origin.id)
            .or_else(|| overlay.get(&origin.id.to_uppercase()))
            .map(Vec::as_slice),
        None,
    )?;
    let dest_ctx = build_airport_context(
        destination,
        map.get(&*destination.id)
            .or_else(|| map.get(&destination.id.to_uppercase())),
        overlay
            .get(&*destination.id)
            .or_else(|| overlay.get(&destination.id.to_uppercase()))
            .map(Vec::as_slice),
        None,
    )?;
    Some(FlightContext {
        origin: origin_ctx,
        destination: dest_ctx,
    })
}

/// Load order per airport: cache → bundled → config file. Option B: zero-effort default from bundle.
/// Optional dynamic_origin_pois / dynamic_dest_pois (e.g. from Wikipedia geosearch) are merged and filtered to 10 nm.
pub fn load_flight_context_with_bundled(
    bundled: &std::collections::BTreeMap<String, AirportContextFile>,
    config_path: &Path,
    cache_dir: &Path,
    origin: &Airport,
    destination: &Airport,
    dynamic_origin_pois: Option<Vec<PoiFile>>,
    dynamic_dest_pois: Option<Vec<PoiFile>>,
) -> Option<FlightContext> {
    let config_map: std::collections::BTreeMap<String, AirportContextFile> =
        std::fs::read_to_string(config_path)
            .ok()
            .and_then(|d| serde_json::from_str(&d).ok())
            .unwrap_or_default();
    let get_file = |icao: &str| {
        load_airport_context_from_cache(cache_dir, icao)
            .or_else(|| {
                bundled
                    .get(icao)
                    .or_else(|| bundled.get(&icao.to_uppercase()))
                    .cloned()
            })
            .or_else(|| {
                config_map
                    .get(icao)
                    .or_else(|| config_map.get(&icao.to_uppercase()))
                    .cloned()
            })
    };
    let origin_file = get_file(&origin.id);
    let dest_file = get_file(&destination.id);
    let overlay = get_poi_overlay();
    let origin_ctx = build_airport_context(
        origin,
        origin_file.as_ref(),
        overlay
            .get(&*origin.id)
            .or_else(|| overlay.get(&origin.id.to_uppercase()))
            .map(Vec::as_slice),
        dynamic_origin_pois.as_deref(),
    )?;
    let dest_ctx = build_airport_context(
        destination,
        dest_file.as_ref(),
        overlay
            .get(&*destination.id)
            .or_else(|| overlay.get(&destination.id.to_uppercase()))
            .map(Vec::as_slice),
        dynamic_dest_pois.as_deref(),
    )?;
    Some(FlightContext {
        origin: origin_ctx,
        destination: dest_ctx,
    })
}

/// Loads one airport's context from the cache (Phase 2b). File format: `{ "snippet": "", "points_nearby": [] }`.
pub fn load_airport_context_from_cache(cache_dir: &Path, icao: &str) -> Option<AirportContextFile> {
    let path = cache_dir.join(format!("{}.json", icao.to_uppercase()));
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Writes one airport's context to the cache (Phase 2b). Creates cache_dir if needed.
pub fn save_airport_context_to_cache(
    cache_dir: &Path,
    icao: &str,
    data: &AirportContextFile,
) -> std::io::Result<()> {
    let _ = std::fs::create_dir_all(cache_dir);
    let path = cache_dir.join(format!("{}.json", icao.to_uppercase()));
    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, data).map_err(std::io::Error::other)
}

/// Builds FlightContext from cache (per-ICAO files) and optionally curated map. Cache takes precedence per airport.
/// POIs are filtered to within 10 nm. Returns None if either airport has no lat/lon.
pub fn load_flight_context_with_cache(
    cache_dir: &Path,
    curated_path: &Path,
    origin: &Airport,
    destination: &Airport,
) -> Option<FlightContext> {
    let curated_map: std::collections::BTreeMap<String, AirportContextFile> =
        std::fs::read_to_string(curated_path)
            .ok()
            .and_then(|d| serde_json::from_str(&d).ok())
            .unwrap_or_default();
    let origin_file = load_airport_context_from_cache(cache_dir, &origin.id).or_else(|| {
        curated_map
            .get(&*origin.id)
            .or_else(|| curated_map.get(&origin.id.to_uppercase()))
            .cloned()
    });
    let dest_file = load_airport_context_from_cache(cache_dir, &destination.id).or_else(|| {
        curated_map
            .get(&*destination.id)
            .or_else(|| curated_map.get(&destination.id.to_uppercase()))
            .cloned()
    });
    let overlay = get_poi_overlay();
    let origin_ctx = build_airport_context(
        origin,
        origin_file.as_ref(),
        overlay
            .get(&*origin.id)
            .or_else(|| overlay.get(&origin.id.to_uppercase()))
            .map(Vec::as_slice),
        None,
    )?;
    let dest_ctx = build_airport_context(
        destination,
        dest_file.as_ref(),
        overlay
            .get(&*destination.id)
            .or_else(|| overlay.get(&destination.id.to_uppercase()))
            .map(Vec::as_slice),
        None,
    )?;
    Some(FlightContext {
        origin: origin_ctx,
        destination: dest_ctx,
    })
}

/// Merges file snippet/POIs with overlay POIs and optional dynamic POIs (e.g. Wikipedia geosearch); filters all to within POI_RADIUS_NM (10 nm).
fn build_airport_context(
    airport: &Airport,
    file: Option<&AirportContextFile>,
    overlay_pois: Option<&[PoiFile]>,
    dynamic_pois: Option<&[PoiFile]>,
) -> Option<AirportContext> {
    let (apt_lat, apt_lon) = airport
        .lat
        .and_then(|lat| airport.lon.map(|lon| (lat, lon)))
        .or_else(|| fallback_coords(&airport.id))?;
    let snippet = file.map(|f| f.snippet.as_str()).unwrap_or("").to_string();
    let from_file = file.iter().flat_map(|f| f.points_nearby.iter());
    let from_overlay = overlay_pois.iter().flat_map(|s| s.iter());
    let from_dynamic = dynamic_pois.unwrap_or(&[]).iter();
    // Don't show the airport's own Wikipedia article as a landmark (redundant with History).
    let is_airport_self = |name: &str| {
        name.eq_ignore_ascii_case(airport.name.as_str())
            || name.eq_ignore_ascii_case(&format!("{} Airport", airport.name))
    };
    let points_nearby: Vec<PointOfInterest> = from_file
        .chain(from_overlay)
        .chain(from_dynamic)
        .filter_map(|p| {
            if is_airport_self(&p.name) {
                return None;
            }
            let dist_nm = haversine_nm(apt_lat, apt_lon, p.lat, p.lon);
            if dist_nm <= POI_RADIUS_NM {
                Some(PointOfInterest {
                    name: p.name.clone(),
                    kind: p.kind.clone(),
                    snippet: p.snippet.clone(),
                    distance_nm: Some(dist_nm),
                })
            } else {
                None
            }
        })
        .collect();
    Some(AirportContext {
        icao: airport.id.clone(),
        snippet,
        points_nearby,
    })
}

// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FlightPlan {
    pub origin: Airport,
    pub destination: Airport,
    pub aircraft: DiscoveredAddon,
    pub distance_nm: u32,
    pub duration_minutes: u32,
    pub route_description: String,
    /// When origin was resolved from a region (e.g. "from Kenya"), the region id for UI/prefs.
    pub origin_region_id: Option<String>,
    /// When destination was resolved from a region, the region id for UI/prefs.
    pub dest_region_id: Option<String>,
    /// Optional history & flavor for origin/destination (3.2); None until Phase 2 loads data.
    pub context: Option<FlightContext>,
    /// Optional requested time of day (e.g., Night, Dusk).
    pub time: Option<x_adox_bitnet::flight_prompt::TimeKeyword>,
    /// Optional requested weather (e.g., Storm, Clear).
    pub weather: Option<x_adox_bitnet::flight_prompt::WeatherKeyword>,
    /// True only when `weather` was confirmed via live METAR data.
    /// False means weather is an unverified user preference — don't display as fact.
    pub weather_confirmed: bool,
}

use crate::scenery::SceneryPack;

// Helpers with Prompt Context
fn estimate_speed(a: &DiscoveredAddon, prompt: &FlightPrompt) -> u32 {
    // JSON aircraft rule speed override takes highest priority.
    if let Some(kts) = prompt.aircraft_speed_kts {
        return kts;
    }
    // Keyword override: Bush planes are slow
    if let Some(TypeKeyword::Bush) = prompt.keywords.flight_type {
        return 100;
    }

    let tags_joined = a.tags.join(" ").to_lowercase();
    if tags_joined.contains("heavy") || tags_joined.contains("airliner") {
        450
    } else if tags_joined.contains("jet") {
        350
    } else if tags_joined.contains("turboprop") {
        250
    } else if tags_joined.contains("helicopter")
        || tags_joined.contains("helo")
        || tags_joined.contains("seaplane")
        || tags_joined.contains("float")
    {
        100
    } else {
        120
    }
}

/// Orchestrates flight generation based on scenery packs, available aircraft, and a natural language prompt.
pub fn generate_flight(
    packs: &[SceneryPack],
    aircraft_list: &[DiscoveredAddon],
    prompt_str: &str,
    base_airports: Option<&[Airport]>,
    prefs: Option<&HeuristicsConfig>,
    nlp_rules: Option<&x_adox_bitnet::NLPRulesConfig>,
) -> Result<FlightPlan, String> {
    generate_flight_with_pool(
        packs,
        aircraft_list,
        prompt_str,
        base_airports,
        prefs,
        nlp_rules,
        None,
    )
}

/// A high-performance version of [generate_flight] that can use a pre-computed airport pool.
pub fn generate_flight_with_pool(
    packs: &[SceneryPack],
    aircraft_list: &[DiscoveredAddon],
    prompt_str: &str,
    base_airports: Option<&[Airport]>,
    prefs: Option<&HeuristicsConfig>,
    nlp_rules: Option<&x_adox_bitnet::NLPRulesConfig>,
    precomputed_pool: Option<&AirportPool>,
) -> Result<FlightPlan, String> {
    static DEFAULT_NLP: std::sync::OnceLock<x_adox_bitnet::NLPRulesConfig> =
        std::sync::OnceLock::new();
    let default_rules = DEFAULT_NLP.get_or_init(x_adox_bitnet::NLPRulesConfig::default);
    let rules = nlp_rules.unwrap_or(default_rules);
    let prompt = FlightPrompt::parse(prompt_str, rules);
    generate_flight_from_prompt(
        packs,
        aircraft_list,
        &prompt,
        base_airports,
        prefs,
        precomputed_pool,
    )
}

/// Core generation logic accepting a pre-parsed [FlightPrompt] struct.
/// Use this for maximum throughput (skips regex parsing).
pub fn generate_flight_from_prompt(
    packs: &[SceneryPack],
    aircraft_list: &[DiscoveredAddon],
    prompt: &FlightPrompt,
    base_airports: Option<&[Airport]>,
    prefs: Option<&HeuristicsConfig>,
    precomputed_pool: Option<&AirportPool>,
) -> Result<FlightPlan, String> {
    log::debug!(
        "[flight_gen] origin={:?} dest={:?}",
        prompt.origin,
        prompt.destination
    );

    let mut rng = rand::thread_rng();
    let region_index = RegionIndex::new();

    // 1. Select Aircraft
    let suitable_aircraft: Vec<&DiscoveredAddon> = aircraft_list
        .iter()
        .filter(|a| {
            if let AddonType::Aircraft { .. } = a.addon_type {
                if let Some(AircraftConstraint::Tag(ref tag)) = prompt.aircraft {
                    let tag_lower = tag.to_lowercase();
                    a.tags.iter().any(|t| t.to_lowercase().contains(&tag_lower))
                        || a.name.to_lowercase().contains(&tag_lower)
                } else {
                    true
                }
            } else {
                false
            }
        })
        .collect();

    if suitable_aircraft.is_empty() {
        return Err("No matching aircraft found.".to_string());
    }
    let selected_aircraft = *suitable_aircraft.choose(&mut rng).unwrap();

    // Speed for duration-based distance calculation
    let speed_kts = estimate_speed(selected_aircraft, prompt);
    // Surface preference comes from keywords only — not aircraft type
    let req_surface: Option<SurfaceType> = match prompt.keywords.surface {
        Some(SurfaceKeyword::Soft) => Some(SurfaceType::Soft),
        Some(SurfaceKeyword::Hard) => Some(SurfaceType::Hard),
        Some(SurfaceKeyword::Water) => Some(SurfaceType::Water),
        None => None,
    };
    log::debug!(
        "[flight_gen] selected_aircraft='{}' tags={:?} req_surface={:?}",
        selected_aircraft.name,
        selected_aircraft.tags,
        req_surface
    );

    // 2. Select Origin

    // Combined pool: pack airports + base layer merged.
    let pack_iter = packs.iter();
    let base_slice = base_airports.unwrap_or(&[]);

    // 1. Build total airport pool (packs + base layer merged)
    // FAST PATH: If no packs to merge and only base layer is provided, avoid the expensive BTreeMap + cloning.
    let all_airports_owned: Vec<Airport>;
    let all_airports_ref_owned: Vec<&Airport>;
    let all_airports: &[&Airport] = if let Some(p) = precomputed_pool {
        &p.airports
    } else if packs.is_empty() {
        all_airports_ref_owned = base_slice.iter().collect();
        &all_airports_ref_owned
    } else {
        let total_hint = base_slice.len() + packs.iter().map(|p| p.airports.len()).sum::<usize>();
        let mut pool: HashMap<String, Airport> = HashMap::with_capacity(total_hint);

        // Add base layer first (baseline data)
        for apt in base_slice {
            pool.insert(apt.id.to_uppercase(), apt.clone());
        }

        // Add pack airports (higher priority for name/metadata, merge runways)
        for pack in pack_iter {
            for apt in &pack.airports {
                let key = apt.id.to_uppercase();
                if let Some(existing) = pool.get_mut(&key) {
                    // Name: Pack priority
                    existing.name = apt.name.clone();

                    // Lat/Lon: Pack priority if present
                    if apt.lat.is_some() {
                        existing.lat = apt.lat;
                        existing.lon = apt.lon;
                        existing.proj_x = apt.proj_x;
                        existing.proj_y = apt.proj_y;
                    }

                    // Max Runway Length: Always take the LONGEST found across all sources
                    let p_len = apt.max_runway_length.unwrap_or(0);
                    let e_len = existing.max_runway_length.unwrap_or(0);
                    if p_len > e_len || existing.max_runway_length.is_none() {
                        existing.max_runway_length = apt.max_runway_length;
                        existing.surface_type = apt.surface_type;
                    } else if p_len == e_len && existing.surface_type.is_none() {
                        existing.surface_type = apt.surface_type;
                    }
                } else {
                    pool.insert(key, apt.clone());
                }
            }
        }
        all_airports_owned = pool.into_values().collect();
        all_airports_ref_owned = all_airports_owned.iter().collect();
        &all_airports_ref_owned
    };

    log::debug!(
        "[flight_gen] merged airport pool: {} unique ICAOs",
        all_airports.len()
    );

    // 1b. Build ICAO index for O(1) lookups
    let icao_index_owned: HashMap<&str, &Airport>;
    let icao_index = if let Some(p) = precomputed_pool {
        &p.icao_map
    } else {
        icao_index_owned = all_airports.iter().map(|a| (a.id.as_str(), *a)).collect();
        &icao_index_owned
    };

    // 1c. Apply Live Real-World Filters (Solar Time & METAR)
    // weather_map: Some(map) if on-disk cache exists and parsed successfully,
    // None if no weather keyword or cache is absent/stale.
    // NOTE: We intentionally do NOT call fetch_live_metars() here — blocking a
    // 30-second network download inside synchronous generation would stall callers
    // and break offline/CI environments. Callers that want fresh data should pre-fetch
    // (e.g. the GUI does this before opening the flight generator tab).
    let weather_map = if prompt.keywords.weather.is_some() {
        let engine = crate::weather::WeatherEngine::new();
        engine.get_global_weather_map().ok()
    } else {
        None
    };
    // True only when we actually have METAR data to filter against (non-empty map).
    let metar_available = weather_map.as_ref().map(|m| !m.is_empty()).unwrap_or(false);

    let filtered_airports_owned: Vec<&Airport>;
    let all_airports: &[&Airport] = if prompt.keywords.time.is_some()
        || prompt.keywords.weather.is_some()
    {
        let utc_now = chrono::Utc::now();
        let before = all_airports.len();
        filtered_airports_owned = all_airports
            .iter()
            .filter(|apt| {
                // Solar Time Filter — soft: airports without lon are kept.
                if let Some(req_time) = &prompt.keywords.time {
                    if let Some(lon) = apt.lon {
                        let solar_time = calculate_solar_time(lon, utc_now);
                        if solar_time != *req_time {
                            return false;
                        }
                    }
                    // No lon → can't determine time → keep airport.
                }
                // Weather Filter — soft: only exclude when the airport IS in the
                // METAR map and its actual weather explicitly doesn't match.
                // Airports not in the map (no METAR station) are kept.
                // If the map is empty (fetch failed), skip the filter entirely.
                if let Some(req_wx) = &prompt.keywords.weather {
                    if metar_available {
                        if let Some(map) = &weather_map {
                            if let Some(apt_wx) = map.get(apt.id.as_str()) {
                                if apt_wx != req_wx {
                                    return false;
                                }
                            }
                            // Airport not in METAR map → keep (no data ≠ wrong weather).
                        }
                    }
                    // metar_available=false → fetch failed → skip weather filter.
                    let _ = req_wx; // suppress unused warning
                }
                true
            })
            .copied()
            .collect();

        let after = filtered_airports_owned.len();
        log::debug!(
                "[flight_gen] Real-World Filters: {} → {} airports (time={}, weather={}, metar_data={})",
                before,
                after,
                prompt.keywords.time.is_some(),
                prompt.keywords.weather.is_some(),
                metar_available,
            );

        // Fallback: if filters eliminated every airport, discard them and use
        // the full pool so generation doesn't fail for a transient/time reason.
        if filtered_airports_owned.is_empty() && before > 0 {
            log::warn!(
                    "[flight_gen] Real-World Filters produced 0 airports; ignoring time/weather constraints and using full pool."
                );
            all_airports
        } else {
            &filtered_airports_owned
        }
    } else {
        all_airports
    };

    // Refined Origin Selection
    let mut candidate_origins: Vec<&Airport> = match prompt.origin {
        Some(LocationConstraint::Region(ref region_id)) => {
            let region_obj = region_index.get_by_id(region_id);
            let prefixes = icao_prefixes_for_region(region_id);

            all_airports
                .iter()
                .filter(|apt| {
                    // Accuracy: Check ICAO prefix if region has one
                    if let Some(ref pfxs) = prefixes {
                        if !pfxs.iter().any(|pfx| apt.id.starts_with(pfx)) {
                            return false;
                        }
                    }

                    // Bounds: use region from index, or hardcoded London bounds for UK:London (avoids Great Yarmouth etc. if index is stale)
                    if region_id == "UK:London" {
                        if let (Some(lat), Some(lon)) = (apt.lat, apt.lon) {
                            if !in_bounds_london(lat, lon) {
                                return false;
                            }
                        }
                    } else if let Some(r) = region_obj {
                        if let (Some(lat), Some(lon)) = (apt.lat, apt.lon) {
                            if !r.contains(lat, lon) {
                                return false;
                            }
                        }
                    }
                    if !prompt.ignore_guardrails
                        && !check_safety_constraints(apt, selected_aircraft, req_surface)
                    {
                        return false;
                    }
                    apt.lat.is_some() && apt.lon.is_some()
                })
                .copied()
                .collect()
        }
        Some(LocationConstraint::AirportName(ref name)) => score_airports_by_name(
            all_airports,
            precomputed_pool.map(|p| p.search_names.as_slice()),
            precomputed_pool.map(|p| &p.name_map),
            name,
            prompt.ignore_guardrails,
            selected_aircraft,
            req_surface,
        ),
        Some(LocationConstraint::NearCity { lat, lon, .. }) => {
            let mut nearby: Vec<(&Airport, f64)> = all_airports
                .iter()
                .filter_map(|apt| {
                    if let (Some(alat), Some(alon)) = (apt.lat, apt.lon) {
                        // Spatial Pruning: Bounding box check (+/- 1.0 deg is roughly 60nm)
                        if (alat - lat).abs() > 1.0 || (alon - lon).abs() > 1.5 {
                            return None;
                        }

                        let dist = haversine_nm(lat, lon, alat, alon);
                        if dist <= 50.0
                            && (prompt.ignore_guardrails
                                || check_safety_constraints(apt, selected_aircraft, req_surface))
                        {
                            Some((*apt, dist))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            nearby.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            nearby.into_iter().map(|(apt, _)| apt).collect()
        }
        Some(LocationConstraint::ICAO(ref code)) => {
            // Priority: Index lookup (case-insensitive keys would be better, but we can try exact and then fallback)
            let mut results = Vec::new();
            if let Some(apt) = icao_index.get(code.as_str()) {
                results.push(*apt);
            } else {
                // FALLBACK: case-insensitive scan (if code is not uppercase)
                let code_upper = code.to_uppercase();
                if let Some(apt) = icao_index.get(code_upper.as_str()) {
                    results.push(*apt);
                }
            }
            results
        }
        _ => {
            // Wildcard origin: Filter by type compatibility only
            all_airports
                .iter()
                .filter(|a| {
                    if !prompt.ignore_guardrails {
                        check_safety_constraints(a, selected_aircraft, req_surface)
                    } else {
                        true
                    }
                })
                .copied()
                .collect()
        }
    };

    // Fallback: use embedded seed airports when no pack has data for this region/city.
    #[allow(unused_assignments)]
    let mut seed_origin_fallback: Vec<Airport> = Vec::new();
    if candidate_origins.is_empty() {
        seed_origin_fallback = seeds_for_constraint(&prompt.origin, &region_index);
        if !seed_origin_fallback.is_empty() {
            candidate_origins = seed_origin_fallback.iter().collect();
        }
    }

    if candidate_origins.is_empty() {
        log::debug!(
            "[flight_gen] No departure candidates (origin={:?}) all_airports={}",
            prompt.origin,
            all_airports.len()
        );
        return Err("No suitable departure airport found.".to_string());
    } else {
        log::debug!(
            "[flight_gen] Found {} departure candidates for {:?}",
            candidate_origins.len(),
            prompt.origin
        );
    }

    // Apply flight preferences: preferred origin ICAOs first
    if let Some(LocationConstraint::Region(ref region_id)) = &prompt.origin {
        let mut preferred_icaos: Vec<String> = prefs
            .and_then(|c| c.flight_origin_prefs.get(region_id).cloned())
            .unwrap_or_default();
        if let (Some(last), Some(LocationConstraint::Region(ref dest_r))) = (
            prefs.and_then(|c| c.flight_last_success.as_ref()),
            &prompt.destination,
        ) {
            if last.origin_region == *region_id
                && last.dest_region == *dest_r
                && !last.origin_icao.is_empty()
                && !preferred_icaos.contains(&last.origin_icao)
            {
                preferred_icaos.insert(0, last.origin_icao.clone());
            }
        }
        if !preferred_icaos.is_empty() {
            let preferred_set: std::collections::HashSet<&str> =
                preferred_icaos.iter().map(|s| s.as_str()).collect();
            let mut preferred: Vec<&Airport> = candidate_origins
                .iter()
                .filter(|a| preferred_set.contains(a.id.as_str()))
                .copied()
                .collect();
            let mut rest: Vec<&Airport> = candidate_origins
                .iter()
                .filter(|a| !preferred_set.contains(a.id.as_str()))
                .copied()
                .collect();
            rest.shuffle(&mut rng);
            preferred.shuffle(&mut rng);
            candidate_origins = preferred;
            candidate_origins.extend(rest);
        } else {
            candidate_origins.shuffle(&mut rng);
        }
    } else if let Some(LocationConstraint::AirportName(_)) = &prompt.origin {
        // FIXED: Do NOT shuffle if search was by name. Preserve the scoring from score_airports_by_name.
    } else if let Some(LocationConstraint::NearCity { .. }) = &prompt.origin {
        // NearCity: already sorted by proximity, don't shuffle.
    } else {
        candidate_origins.shuffle(&mut rng);
    }
    let max_attempts = 20;
    #[allow(unused_assignments)]
    let mut seed_dest_fallback: Vec<Airport> = Vec::new();

    for origin in candidate_origins.iter().take(max_attempts) {
        // 3. Select Destination

        // Keyword-Driven Range Logic
        let (min_dist, max_dist) = if let Some(mins) = prompt.duration_minutes {
            let dist = speed_kts as f64 * (mins as f64 / 60.0);
            (dist * 0.8, dist * 1.2)
        } else if let Some(dkw) = &prompt.keywords.duration {
            match dkw {
                DurationKeyword::Short => (10.0, 200.0),
                DurationKeyword::Medium => (200.0, 800.0),
                DurationKeyword::Long => (800.0, 2500.0),
                DurationKeyword::Haul => (2500.0, 12000.0),
            }
        } else if prompt.ignore_guardrails {
            (0.0, 20000.0)
        } else if prompt.aircraft_min_dist.is_some() || prompt.aircraft_max_dist.is_some() {
            // Aircraft rule supplied a soft distance envelope.
            // Keyword duration (short/long/haul) takes priority (handled above);
            // this only applies when no duration keyword was given.
            let lo = prompt.aircraft_min_dist.unwrap_or(10.0);
            let hi = prompt.aircraft_max_dist.unwrap_or(8000.0);
            (lo, hi)
        } else {
            // No keyword constraint: wide-open random discovery.
            // Keywords (short/medium/long/haul) and duration_minutes are the
            // intended controls — aircraft type no longer sets distance limits.
            // 8000nm covers most intercontinental routes (LA→UK ~5400nm, NY→Tokyo ~6760nm).
            // Ultra-long-haul (LA→Australia ~9400nm) requires "long haul" keyword.
            (10.0, 8000.0)
        };

        // Explicit Endpoint Check (Relax distance logic when both ends are "point" constraints)
        // ICAO and NearCity are point types (user named a specific place), so we relax range.
        // Region is an "area" type — keep constraints so random picks stay geographically sensible.
        let is_point = |c: &LocationConstraint| {
            matches!(
                c,
                LocationConstraint::ICAO(_) | LocationConstraint::NearCity { .. }
            )
        };
        let endpoints_explicit = match (&prompt.origin, &prompt.destination) {
            (Some(o), Some(d)) => is_point(o) && is_point(d),
            _ => false,
        };

        // Destination selection (same combined pool)
        let candidate_dests: Vec<&Airport> = match prompt.destination {
            Some(LocationConstraint::Region(ref region_id)) => {
                let region_obj = region_index.get_by_id(region_id);
                let prefixes = icao_prefixes_for_region(region_id);

                all_airports
                    .iter()
                    .filter(|apt| {
                        if let Some(ref pfxs) = prefixes {
                            if !pfxs.iter().any(|pfx| apt.id.starts_with(pfx)) {
                                return false;
                            }
                        }
                        if region_id == "UK:London" {
                            if let (Some(lat), Some(lon)) = (apt.lat, apt.lon) {
                                if !in_bounds_london(lat, lon) {
                                    return false;
                                }
                            }
                        } else if let Some(r) = region_obj {
                            if let (Some(lat), Some(lon)) = (apt.lat, apt.lon) {
                                if !r.contains(lat, lon) {
                                    return false;
                                }
                            }
                        }
                        if !prompt.ignore_guardrails
                            && !check_safety_constraints(apt, selected_aircraft, req_surface)
                        {
                            return false;
                        }
                        apt.lat.is_some() && apt.lon.is_some()
                    })
                    .copied()
                    .collect()
            }
            Some(LocationConstraint::AirportName(ref name)) => score_airports_by_name(
                all_airports,
                precomputed_pool.map(|p| p.search_names.as_slice()),
                precomputed_pool.map(|p| &p.name_map),
                name,
                true, // skip safety: user explicitly named this destination
                selected_aircraft,
                req_surface,
            ),
            Some(LocationConstraint::NearCity { lat, lon, .. }) => {
                // User explicitly named this city — skip safety constraints
                // during candidate generation (they are handled in valid_dests).
                let mut nearby: Vec<(&Airport, f64)> = all_airports
                    .iter()
                    .filter_map(|apt| {
                        if let (Some(alat), Some(alon)) = (apt.lat, apt.lon) {
                            // Spatial Pruning: Bounding box check (+/- 1.0 deg is roughly 60nm)
                            if (alat - lat).abs() > 1.0 || (alon - lon).abs() > 1.5 {
                                return None;
                            }

                            let dist = haversine_nm(lat, lon, alat, alon);
                            if dist <= 50.0 {
                                Some((*apt, dist))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                nearby.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                nearby.into_iter().map(|(apt, _)| apt).collect()
            }
            Some(LocationConstraint::ICAO(ref code)) => {
                let mut results = Vec::new();
                if let Some(apt) = icao_index.get(code.as_str()) {
                    results.push(*apt);
                } else {
                    let code_upper = code.to_uppercase();
                    if let Some(apt) = icao_index.get(code_upper.as_str()) {
                        results.push(*apt);
                    }
                }
                results
            }
            _ => all_airports.to_vec(),
        };

        // Fallback: use embedded seed airports when no pack has dests for this region
        let mut candidate_dests = candidate_dests;
        if candidate_dests.is_empty() {
            seed_dest_fallback = seeds_for_constraint(&prompt.destination, &region_index);
            if !seed_dest_fallback.is_empty() {
                candidate_dests = seed_dest_fallback.iter().collect();
            }
        }

        let candidate_dests_count = candidate_dests.len();
        log::debug!(
            "[flight_gen] origin='{}' found {} destination candidates for {:?}",
            origin.id,
            candidate_dests_count,
            prompt.destination
        );
        if candidate_dests_count == 0 {
            log::debug!(
                "[flight_gen] origin='{}': candidate_dests is empty",
                origin.id
            );
        }

        // Pre-calculate spatial bounds for pruning if origin has coordinates.
        // When both endpoints are explicit (user named origin+dest), relax the bbox to the
        // maximum possible distance so that distant-but-valid destinations aren't pruned
        // before the haversine check can apply the `endpoints_explicit` relaxation.
        let spatial_bounds = if let (Some(lat1), Some(lon1)) = (origin.lat, origin.lon) {
            let lat_rad = lat1.to_radians();
            let cos_lat = lat_rad.cos().abs().max(0.1);
            let bbox_max = if endpoints_explicit {
                20000.0
            } else {
                max_dist
            };
            let dlat = (bbox_max / 60.0) + 0.1;
            let dlon = (bbox_max / (60.0 * cos_lat)) + 0.1;
            Some((lat1, lon1, dlat, dlon))
        } else {
            None
        };

        let valid_dests: Vec<&Airport> = candidate_dests
            .into_iter()
            .filter(|dest| {
                // Exclude same airport as origin (by pointer or ICAO — needed when
                // origin/dest come from different seed-fallback Vec<Airport> allocations).
                if std::ptr::eq(dest, origin) || dest.id.eq_ignore_ascii_case(&origin.id) {
                    return false;
                }
                if let Some((lat1, lon1, dlat_lim, dlon_lim)) = spatial_bounds {
                    if let (Some(lat2), Some(lon2)) = (dest.lat, dest.lon) {
                        // SPATIAL PRUNING: Bounding box check before Haversine.
                        // Use shortest longitude arc to handle date-line crossings correctly.
                        let dlon_abs = (lon2 - lon1).abs();
                        let dlon_wrap = dlon_abs.min(360.0 - dlon_abs);
                        if (lat2 - lat1).abs() > dlat_lim || dlon_wrap > dlon_lim {
                            return false;
                        }

                        let dist = haversine_nm(lat1, lon1, lat2, lon2);
                        let result = if endpoints_explicit {
                            // Allow very short flights if explicit
                            dist > 2.0 && dist <= 20000.0
                        } else {
                            dist >= min_dist && dist <= max_dist
                        };
                        if !result {
                            return false;
                        }
                    }
                }

                // Type safety + keyword surface preference.
                if !prompt.ignore_guardrails
                    && !check_safety_constraints(dest, selected_aircraft, req_surface)
                {
                    return false;
                }
                true
            })
            .collect();
        log::debug!(
            "[flight_gen] origin='{}': candidate_dests={}, valid_dests={}",
            origin.id,
            candidate_dests_count,
            valid_dests.len()
        );

        if !valid_dests.is_empty() {
            let preferred_dest_icaos: Vec<String> = match &prompt.destination {
                Some(LocationConstraint::Region(ref region_id)) => {
                    let mut icaos = prefs
                        .and_then(|c| c.flight_dest_prefs.get(region_id).cloned())
                        .unwrap_or_default();
                    if let (Some(last), Some(LocationConstraint::Region(ref orig_r))) = (
                        prefs.and_then(|c| c.flight_last_success.as_ref()),
                        &prompt.origin,
                    ) {
                        if last.dest_region == *region_id
                            && last.origin_region == *orig_r
                            && !last.dest_icao.is_empty()
                            && !icaos.contains(&last.dest_icao)
                        {
                            icaos.insert(0, last.dest_icao.clone());
                        }
                    }
                    icaos
                }
                _ => Vec::new(),
            };
            let destination = if let Some(LocationConstraint::AirportName(_)) = &prompt.destination
            {
                // FIXED: Preserve scoring for name-based destination search.
                // Pick the first one (highest score) from the valid list.
                *valid_dests.first().unwrap()
            } else if let Some(LocationConstraint::NearCity { .. }) = &prompt.destination {
                // NearCity: pick closest (already sorted by proximity in candidate list).
                *valid_dests.first().unwrap()
            } else if preferred_dest_icaos.is_empty() {
                *valid_dests.choose(&mut rng).unwrap()
            } else {
                let preferred_set: std::collections::HashSet<&str> =
                    preferred_dest_icaos.iter().map(|s| s.as_str()).collect();
                let preferred: Vec<&Airport> = valid_dests
                    .iter()
                    .filter(|a| preferred_set.contains(a.id.as_str()))
                    .copied()
                    .collect();
                if preferred.is_empty() {
                    *valid_dests.choose(&mut rng).unwrap()
                } else {
                    *preferred.choose(&mut rng).unwrap()
                }
            };
            let dist = haversine_nm(
                origin.lat.unwrap(),
                origin.lon.unwrap(),
                destination.lat.unwrap(),
                destination.lon.unwrap(),
            );
            let origin_region_id = prompt.origin.as_ref().and_then(|o| {
                if let LocationConstraint::Region(r) = o {
                    Some(r.clone())
                } else {
                    None
                }
            });
            let dest_region_id = prompt.destination.as_ref().and_then(|d| {
                if let LocationConstraint::Region(r) = d {
                    Some(r.clone())
                } else {
                    None
                }
            });
            return Ok(FlightPlan {
                origin: (*origin).clone(),
                destination: destination.clone(),
                aircraft: selected_aircraft.clone(),
                distance_nm: dist as u32,
                duration_minutes: (dist / (speed_kts as f64) * 60.0) as u32,
                route_description: if prompt.ignore_guardrails {
                    "(Guardrails Ignored)".to_string()
                } else {
                    "generated".to_string()
                },
                origin_region_id,
                dest_region_id,
                context: None,
                time: prompt.keywords.time.clone(),
                weather: prompt.keywords.weather.clone(),
                // Only mark confirmed if this specific origin airport has a METAR entry
                // that matches the requested condition. A non-empty global dataset is not
                // sufficient — small/remote airports are often absent from METAR feeds.
                weather_confirmed: weather_map
                    .as_ref()
                    .and_then(|map| map.get(origin.id.as_str()))
                    .map(|actual_wx| {
                        prompt
                            .keywords
                            .weather
                            .as_ref()
                            .map(|req| actual_wx == req)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false),
            });
        }
    }
    log::debug!(
        "[flight_gen] No destination found after {} origin attempts",
        max_attempts
    );
    Err("No suitable destination found.".to_string())
}

// Type-compatibility and keyword surface check.
// Runway length and aircraft-type surface requirements have been removed — this is a
// random flight generator where users control distance via keywords and can swap
// aircraft after export. Only genuine type mismatches (helipads, seaplane bases) and
// explicit keyword surface preferences are enforced.
fn check_safety_constraints(
    apt: &Airport,
    aircraft: &DiscoveredAddon,
    req_surface: Option<SurfaceType>,
) -> bool {
    // Type: helipad → helicopters only; seaplane base → seaplanes only
    match apt.airport_type {
        AirportType::Heliport => return is_heli(aircraft),
        AirportType::Seaplane => return is_seaplane(aircraft),
        AirportType::Land => {
            // Floatplanes/seaplanes need water surface
            if is_seaplane(aircraft) && apt.surface_type != Some(SurfaceType::Water) {
                return false;
            }
        }
    }
    // Keyword surface preference (grass/paved/water keywords, or bush → soft)
    if let Some(req_surf) = req_surface {
        match req_surf {
            SurfaceType::Water => {
                // "seaplane"/"water" keyword: require seaplane base or water surface
                if apt.airport_type != AirportType::Seaplane
                    && apt.surface_type != Some(SurfaceType::Water)
                {
                    return false;
                }
            }
            _ => {
                if let Some(surf) = apt.surface_type {
                    match req_surf {
                        SurfaceType::Soft if surf != SurfaceType::Soft => return false,
                        SurfaceType::Hard if surf != SurfaceType::Hard => return false,
                        _ => {}
                    }
                }
            }
        }
    }
    true
}

fn score_airports_by_name<'a>(
    airports: &[&'a Airport],
    search_names: Option<&[String]>,
    name_map: Option<&HashMap<String, Vec<usize>>>,
    search_str: &str,
    ignore_guardrails: bool,
    selected_aircraft: &DiscoveredAddon,
    req_surface: Option<SurfaceType>,
) -> Vec<&'a Airport> {
    let search_lower = search_str.to_lowercase();
    let search_tokens: Vec<&str> = search_lower.split_whitespace().collect();

    // FAST PATH: Exact Name Match via name_map
    if let (Some(nm), Some(_sn)) = (name_map, search_names) {
        if let Some(indices) = nm.get(&search_lower) {
            let mut results = Vec::new();
            for &idx in indices {
                let apt = airports[idx];
                if ignore_guardrails
                    || check_safety_constraints(apt, selected_aircraft, req_surface)
                {
                    results.push(apt);
                }
            }
            if !results.is_empty() {
                return results;
            }
        }
    }

    let mut scored: Vec<(i32, &'a Airport)> = airports
        .iter()
        .enumerate()
        .map(|(i, &apt)| (i, apt))
        .filter(|(_, apt)| {
            if !ignore_guardrails {
                check_safety_constraints(apt, selected_aircraft, req_surface)
            } else {
                true
            }
        })
        .map(|(idx, apt)| {
            let mut score = 0;

            // ID Match (Case-insensitive, no allocation)
            if apt.id.eq_ignore_ascii_case(&search_lower) {
                score += 1000;
            } else if apt.id.len() < 16 && apt.id.to_lowercase().contains(&search_lower) {
                // We still do one lowercase for ID if it's a substring match, but ID is short.
                score += 500;
            }

            if apt.name.eq_ignore_ascii_case(&search_lower) {
                score += 800;
            } else {
                let name_lower_owned: String;
                let name_lower: &str = if let Some(sn) = search_names {
                    &sn[idx]
                } else {
                    name_lower_owned = apt.name.to_lowercase();
                    &name_lower_owned
                };

                if name_lower.contains(&search_lower) {
                    score += 300;
                }

                // Token-based matching
                for token in &search_tokens {
                    if name_lower.contains(token) {
                        score += 200;
                        if name_lower.split_whitespace().any(|w| w == *token) {
                            score += 300;
                        }
                    }
                }
            }

            // Accuracy Boost: If search_str contains a region token
            for token in &search_tokens {
                if token.len() >= 2 {
                    if let Some(region_id) = try_map_token_to_region_id(token) {
                        if let Some(prefixes) = icao_prefixes_for_region(region_id) {
                            if prefixes.iter().any(|pfx| apt.id.starts_with(pfx)) {
                                score += 200;
                            }
                        }
                    }
                }
            }

            (score, apt)
        })
        .filter(|(s, _)| *s > 0)
        .collect();

    scored.sort_by_key(|(s, _)| -*s);
    scored.into_iter().map(|(_, apt)| apt).collect()
}

fn try_map_token_to_region_id(token: &str) -> Option<&'static str> {
    match token {
        "uk" | "gb" | "britain" | "england" => Some("UK"),
        "us" | "usa" | "america" => Some("US"),
        "fr" | "france" => Some("FR"),
        "it" | "italy" => Some("IT"),
        "de" | "germany" => Some("DE"),
        "es" | "spain" => Some("ES"),
        "ch" | "switzerland" | "swiss" => Some("CH"),
        "at" | "austria" => Some("AT"),
        _ => None,
    }
}

#[cfg(test)]
mod legacy_tests {

    fn p_contains_token(token: &str, text: &str) -> bool {
        if text.contains(token) {
            return true;
        }
        // Handle common abbreviations
        match token {
            "uk" => {
                text.contains("united kingdom")
                    || text.contains("great britain")
                    || text.contains("england")
                    || text.contains("scotland")
                    || text.contains("wales")
                    || text.contains("northern ireland")
            }
            "gb" => {
                text.contains("great britain")
                    || text.contains("england")
                    || text.contains("scotland")
                    || text.contains("wales")
            }
            "usa" | "us" => text.contains("united states") || text.contains("america"),
            "uae" => text.contains("united arab emirates"),
            "nz" => text.contains("new zealand"),
            _ => false,
        }
    }

    fn is_british_isles_region(region: &str) -> bool {
        let r_lower = region.to_lowercase();
        p_contains_token("uk", &r_lower)
            || p_contains_token("gb", &r_lower)
            || r_lower.contains("ireland")
            || r_lower.contains("isle of man")
            || r_lower.contains("channel islands")
    }

    #[test]
    fn test_british_isles_matching() {
        assert!(is_british_isles_region("Great Britain"));
        assert!(is_british_isles_region("UK"));
        assert!(is_british_isles_region("United Kingdom"));
        assert!(is_british_isles_region("Scotland"));
    }
}

fn is_heli(a: &DiscoveredAddon) -> bool {
    a.tags
        .iter()
        .any(|t| t.to_lowercase().contains("helicopter"))
}

fn is_seaplane(a: &DiscoveredAddon) -> bool {
    a.tags.iter().any(|t| {
        let t_lower = t.to_lowercase();
        t_lower.contains("seaplane") || t_lower.contains("amphibian") || t_lower.contains("float")
    })
}

/// ICAO location prefix(es) per region. Used to restrict origin/destination to the correct
/// country (e.g. "Mexico" → MM only, so US airports in the same bounding box are excluded).
/// Parent fallback applies for sub-regions (US:SoCal → US → K). Continent ids (EU, NA, AS, …)
/// are not listed; they have no single prefix.
fn icao_prefixes_for_region(region_id: &str) -> Option<Vec<&'static str>> {
    let direct = match region_id {
        // Europe
        "IT" => Some(vec!["LI"]),
        "FR" => Some(vec!["LF"]),
        "UK" | "GB" | "BI" => Some(vec!["EG"]),
        "DE" => Some(vec!["ED", "ET"]),
        "ES" => Some(vec!["LE"]),
        "CH" => Some(vec!["LS"]),
        "AT" => Some(vec!["LO"]),
        "PT" => Some(vec!["LP"]),
        "GR" => Some(vec!["LG"]),
        "BE" => Some(vec!["EB"]),
        "NL" => Some(vec!["EH"]),
        "LU" => Some(vec!["EL"]),
        "IE" => Some(vec!["EI"]),
        "NO" => Some(vec!["EN"]),
        "SE" => Some(vec!["ES"]),
        "FI" => Some(vec!["EF"]),
        "DK" => Some(vec!["EK"]),
        "IS" => Some(vec!["BI"]),
        "PL" => Some(vec!["EP"]),
        "CZ" => Some(vec!["LK"]),
        "TR" => Some(vec!["LT"]),
        "UA" => Some(vec!["UK"]), // Ukrainian ICAO prefix (UKBB=Kyiv, UKLL=Lviv, etc.)
        // Americas
        "US:AK" | "US:HI" => Some(vec!["P"]), // Alaska (PA..) & Hawaii (PH..)
        "US" => Some(vec!["K"]),
        "CA" => Some(vec!["C"]),
        "MX" => Some(vec!["MM"]),
        "BR" => Some(vec!["SB"]),
        // Asia–Pacific
        "JP" => Some(vec!["RJ"]),
        "CN" => Some(vec!["ZB", "ZG", "ZH", "ZM", "ZU"]),
        "KR" => Some(vec!["RK"]),
        "IN" => Some(vec!["VE", "VO"]),
        "TH" => Some(vec!["VT"]),
        "VN" => Some(vec!["VV"]),
        "ID" => Some(vec!["WI"]),
        "AU" => Some(vec!["Y"]),
        "SG" => Some(vec!["WS"]),
        "MY" => Some(vec!["WM"]),
        "PH" => Some(vec!["RP"]),
        "HK" => Some(vec!["VH"]),
        "TW" => Some(vec!["RC"]),
        "NZ" => Some(vec!["NZ"]),
        // Middle East & Africa
        "IL" => Some(vec!["LL"]),
        "EG" => Some(vec!["HE"]),
        "ZA" => Some(vec!["FA"]),
        "KE" => Some(vec!["HK"]),
        "TZ" => Some(vec!["HT"]),
        "ET" => Some(vec!["HA"]),
        "NG" => Some(vec!["DN"]),
        "MA" => Some(vec!["GM"]),
        "UAE" => Some(vec!["OM"]),
        "QA" => Some(vec!["OT"]),
        // South America
        "AR" => Some(vec!["SA"]),
        "CO" => Some(vec!["SK"]),
        "PE" => Some(vec!["SP"]),
        "CL" => Some(vec!["SC"]),
        // Eastern Europe
        "AL" => Some(vec!["LA"]), // Albania
        "BA" => Some(vec!["LQ"]), // Bosnia-Herzegovina
        "BG" => Some(vec!["LB"]), // Bulgaria
        "BY" => Some(vec!["UM"]), // Belarus
        "EE" => Some(vec!["EE"]), // Estonia
        "HR" => Some(vec!["LD"]), // Croatia
        "HU" => Some(vec!["LH"]), // Hungary
        "LT" => Some(vec!["EY"]), // Lithuania
        "LV" => Some(vec!["EV"]), // Latvia
        "MD" => Some(vec!["LU"]), // Moldova
        "ME" => Some(vec!["LY"]), // Montenegro (LY shared with Serbia)
        "MK" => Some(vec!["LW"]), // North Macedonia
        "RO" => Some(vec!["LR"]), // Romania
        "RS" => Some(vec!["LY"]), // Serbia
        "SI" => Some(vec!["LJ"]), // Slovenia
        "SK" => Some(vec!["LZ"]), // Slovakia
        // Middle East
        "BH" => Some(vec!["OB"]),  // Bahrain
        "IQ" => Some(vec!["OR"]),  // Iraq
        "IR" => Some(vec!["OI"]),  // Iran
        "JO" => Some(vec!["OJ"]),  // Jordan
        "KW" => Some(vec!["OK"]),  // Kuwait
        "LB" => Some(vec!["OL"]),  // Lebanon
        "OM" => Some(vec!["OO"]),  // Oman
        "SAU" => Some(vec!["OE"]), // Saudi Arabia
        // South & Southeast Asia
        "BD" => Some(vec!["VG"]), // Bangladesh
        "KH" => Some(vec!["VD"]), // Cambodia
        "LA" => Some(vec!["VL"]), // Laos
        "LK" => Some(vec!["VC"]), // Sri Lanka
        "MM" => Some(vec!["VY"]), // Myanmar
        "MN" => Some(vec!["ZM"]), // Mongolia (ZMUB=Ulaanbaatar)
        "NP" => Some(vec!["VN"]), // Nepal
        "PG" => Some(vec!["AY"]), // Papua New Guinea
        "PK" => Some(vec!["OP"]), // Pakistan
        // Africa
        "AO" => Some(vec!["FN"]), // Angola
        "CM" => Some(vec!["FK"]), // Cameroon
        "FJ" => Some(vec!["NF"]), // Fiji
        "GH" => Some(vec!["DG"]), // Ghana
        "LY" => Some(vec!["HL"]), // Libya
        "MG" => Some(vec!["FM"]), // Madagascar
        "MZ" => Some(vec!["FQ"]), // Mozambique
        "RW" => Some(vec!["HR"]), // Rwanda
        "SD" => Some(vec!["HS"]), // Sudan
        "SN" => Some(vec!["GO"]), // Senegal
        "TN" => Some(vec!["DT"]), // Tunisia
        "UG" => Some(vec!["HU"]), // Uganda
        "ZM" => Some(vec!["FL"]), // Zambia
        "ZW" => Some(vec!["FV"]), // Zimbabwe
        // Latin America & Caribbean
        "BO" => Some(vec!["SL"]), // Bolivia
        "BS" => Some(vec!["MY"]), // Bahamas
        "CR" => Some(vec!["MR"]), // Costa Rica
        "CU" => Some(vec!["MU"]), // Cuba
        "DO" => Some(vec!["MD"]), // Dominican Republic
        "EC" => Some(vec!["SE"]), // Ecuador
        "GT" => Some(vec!["MG"]), // Guatemala
        "HN" => Some(vec!["MH"]), // Honduras
        "HT" => Some(vec!["MT"]), // Haiti
        "JM" => Some(vec!["MK"]), // Jamaica
        "NI" => Some(vec!["MN"]), // Nicaragua
        "PA" => Some(vec!["MP"]), // Panama
        "PY" => Some(vec!["SG"]), // Paraguay
        "SV" => Some(vec!["MS"]), // El Salvador
        "UY" => Some(vec!["SU"]), // Uruguay
        "VE" => Some(vec!["SV"]), // Venezuela
        _ => None,
    };
    if direct.is_some() {
        return direct;
    }
    // Parent fallback: US:SoCal, US:OR, etc. -> US
    if region_id.contains(':') {
        let parent = region_id.split(':').next().unwrap_or(region_id);
        return icao_prefixes_for_region(parent);
    }
    None
}

/// Build a minimal Airport for seed data (used when no pack has airports for a region).
fn seed_airport(id: &str, name: &str, lat: f64, lon: f64) -> Airport {
    Airport {
        id: id.to_string(),
        name: name.to_string(),
        airport_type: AirportType::Land,
        lat: Some(lat),
        lon: Some(lon),
        proj_x: None,
        proj_y: None,
        max_runway_length: Some(2000),
        surface_type: Some(SurfaceType::Hard),
    }
}

/// Returns seed airports for a Region or NearCity constraint, empty Vec otherwise.
/// Centralises the fallback pattern used for both origin and destination selection.
fn seeds_for_constraint(
    constraint: &Option<LocationConstraint>,
    region_index: &RegionIndex,
) -> Vec<Airport> {
    match constraint {
        Some(LocationConstraint::Region(r)) => get_seed_airports_for_region(r),
        Some(LocationConstraint::NearCity { lat, lon, .. }) => {
            for region in region_index.find_regions(*lat, *lon) {
                let seeds = get_seed_airports_for_region(&region.id);
                if !seeds.is_empty() {
                    return seeds;
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Seed airports used only when the pool (scenery packs + base layer) has no candidates for
/// that region. Global coverage comes from the base layer (Resources + Global Scenery apt.dat);
/// we seed only a few high-traffic regions so prompts like "London to Paris" still work without
/// scenery. Parent fallback applies for sub-regions (e.g. US:SoCal → US seeds).
fn get_seed_airports_for_region(region_id: &str) -> Vec<Airport> {
    #[derive(serde::Deserialize)]
    struct SeedEntry {
        id: String,
        name: String,
        lat: f64,
        lon: f64,
    }

    static SEEDS: OnceLock<HashMap<String, Vec<SeedEntry>>> = OnceLock::new();
    let map = SEEDS.get_or_init(|| {
        let json = include_str!("data/seed_airports.json");
        serde_json::from_str(json).expect("seed_airports.json is malformed")
    });

    // GB has no seeds: it excludes Northern Ireland; do not fall back to UK seeds.
    if region_id == "GB" {
        return Vec::new();
    }

    if let Some(entries) = map.get(region_id) {
        return entries
            .iter()
            .map(|e| seed_airport(&e.id, &e.name, e.lat, e.lon))
            .collect();
    }

    // Parent fallback: US:SoCal, US:OR, US:NorCal, etc. -> US
    if region_id.contains(':') {
        let parent = region_id.split(':').next().unwrap_or(region_id);
        return get_seed_airports_for_region(parent);
    }

    Vec::new()
}

fn haversine_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r_nm = 3440.06;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r_nm * c
}

// Exporters

/// Reads the first two lines of an X-Plane navdata `.dat` file and extracts
/// the AIRAC cycle string (e.g. "2512") from a header like:
/// `1100 Version - data cycle 2512, build 20241114, ...`
fn read_cycle_from_dat(path: &std::path::Path) -> Option<String> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    // Line 1 is the encoding indicator ("I" or "A") — skip it.
    reader.read_line(&mut line).ok()?;
    line.clear();
    // Line 2 contains the version/cycle info.
    reader.read_line(&mut line).ok()?;
    let tag = "data cycle ";
    let idx = line.find(tag)?;
    let rest = &line[idx + tag.len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let cycle = &rest[..end];
    if cycle.len() == 4 {
        Some(cycle.to_string())
    } else {
        None
    }
}

/// Attempts to read the AIRAC cycle from X-Plane's installed navdata.
/// Checks (in priority order):
///   1. Custom Data/earth_nav.dat  — Navigraph or other custom navdata
///   2. Resources/default data/earth_nav.dat — stock X-Plane navdata
///   3. Global Scenery/Global Airports/Earth nav data/apt.dat — apt.dat header
///
/// Returns `None` if none of the files contain a parseable cycle.
pub fn detect_xplane_airac_cycle(xplane_root: &std::path::Path) -> Option<String> {
    let candidates = [
        xplane_root.join("Custom Data").join("earth_nav.dat"),
        xplane_root
            .join("Resources")
            .join("default data")
            .join("earth_nav.dat"),
        xplane_root
            .join("Global Scenery")
            .join("Global Airports")
            .join("Earth nav data")
            .join("apt.dat"),
    ];
    for path in &candidates {
        if let Some(cycle) = read_cycle_from_dat(path) {
            return Some(cycle);
        }
    }
    None
}

/// Resolves the AIRAC cycle to embed in exports.
/// Prefers the cycle found in X-Plane's installed navdata; falls back to the
/// current real-world cycle computed from the system clock.
fn resolve_airac_cycle(xplane_root: Option<&std::path::Path>) -> String {
    if let Some(root) = xplane_root {
        if let Some(cycle) = detect_xplane_airac_cycle(root) {
            return cycle;
        }
    }
    // Fallback: compute from system clock.
    // AIRAC 2501 started 2025-01-23 00:00:00 UTC.
    use std::time::{SystemTime, UNIX_EPOCH};
    const REF_UNIX: u64 = 1737590400;
    const CYCLE_SECS: u64 = 28 * 24 * 3600;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(REF_UNIX);
    if now < REF_UNIX {
        return "2501".to_string();
    }
    let n = (now - REF_UNIX) / CYCLE_SECS;
    let year = 25u64 + n / 13;
    let cycle = (n % 13) + 1;
    format!("{:02}{:02}", year, cycle)
}

pub fn export_fms_11(plan: &FlightPlan, xplane_root: Option<&std::path::Path>) -> String {
    format!(
        "I\n1100 Version\nCYCLE {}\nADEP {}\nADES {}\nNUMENR 0\n",
        resolve_airac_cycle(xplane_root),
        plan.origin.id,
        plan.destination.id
    )
}

pub fn export_fms_12(plan: &FlightPlan, xplane_root: Option<&std::path::Path>) -> String {
    // XP12 uses the same 1100-format file as XP11.
    export_fms_11(plan, xplane_root)
}

pub fn export_lnmpln(plan: &FlightPlan, xplane_root: Option<&std::path::Path>) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<LittleNavmap xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="http://www.littlenavmap.org/schema/lnmpln.xsd">
  <Flightplan>
    <Header>
      <FlightplanType>VFR</FlightplanType>
      <CruisingAlt>5000</CruisingAlt>
      <CreationDate>{}</CreationDate>
      <FileVersion>1.0</FileVersion>
      <ProgramName>X-Addon-Oxide</ProgramName>
      <ProgramVersion>2.4.4</ProgramVersion>
      <Documentation>{}</Documentation>
    </Header>
    <SimData>XPlane12</SimData>
    <NavData Cycle="{}">NAVIGRAPH</NavData>
    <Waypoints>
      <Waypoint>
        <Name>{}</Name>
        <Ident>{}</Ident>
        <Type>AIRPORT</Type>
        <Pos Lon="{}" Lat="{}" Alt="0.00"/>
      </Waypoint>
      <Waypoint>
        <Name>{}</Name>
        <Ident>{}</Ident>
        <Type>AIRPORT</Type>
        <Pos Lon="{}" Lat="{}" Alt="0.00"/>
      </Waypoint>
    </Waypoints>
  </Flightplan>
</LittleNavmap>"#,
        chrono::Local::now().to_rfc3339(),
        plan.route_description,
        resolve_airac_cycle(xplane_root),
        plan.origin.name,
        plan.origin.id,
        plan.origin.lon.unwrap_or_default(),
        plan.origin.lat.unwrap_or_default(),
        plan.destination.name,
        plan.destination.id,
        plan.destination.lon.unwrap_or_default(),
        plan.destination.lat.unwrap_or_default()
    )
}

/// Derives a SimBrief ICAO aircraft type from addon name and tags (e.g. A320, B738, C172).
fn simbrief_aircraft_type(aircraft: &crate::discovery::DiscoveredAddon) -> String {
    // 1. Tags: look for a 4-char ICAO-like code (letter + 3 alphanumeric)
    for t in &aircraft.tags {
        let t = t.trim().to_uppercase();
        if t.len() == 4
            && t.chars()
                .next()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
            && t.chars().skip(1).all(|c| c.is_ascii_alphanumeric())
        {
            return t;
        }
    }
    // 2. Name: common patterns (ToLiss A320, Boeing 737-800, Cessna 172, etc.)
    let name_upper = aircraft.name.to_uppercase();
    let name_lower = aircraft.name.to_lowercase();
    // Airbus A3xx
    if name_upper.contains("A320") || name_lower.contains("a320") {
        return "A320".to_string();
    }
    if name_upper.contains("A321") || name_lower.contains("a321") {
        return "A321".to_string();
    }
    if name_upper.contains("A319") || name_lower.contains("a319") {
        return "A319".to_string();
    }
    if name_upper.contains("A318") || name_lower.contains("a318") {
        return "A318".to_string();
    }
    if name_upper.contains("A330") || name_lower.contains("a330") {
        return "A333".to_string();
    }
    if name_upper.contains("A340") || name_lower.contains("a340") {
        return "A346".to_string();
    }
    if name_upper.contains("A350") || name_lower.contains("a350") {
        return "A359".to_string();
    }
    if name_upper.contains("A380") || name_lower.contains("a380") {
        return "A388".to_string();
    }
    // Boeing 7xx
    if name_upper.contains("737-800")
        || name_upper.contains("737 800")
        || name_lower.contains("737-800")
    {
        return "B738".to_string();
    }
    if name_upper.contains("737-900") || name_upper.contains("737 900") {
        return "B739".to_string();
    }
    if name_upper.contains("737-700") || name_upper.contains("737 700") {
        return "B737".to_string();
    }
    if name_upper.contains("747") || name_lower.contains("747") {
        return "B748".to_string();
    }
    if name_upper.contains("757") || name_lower.contains("757") {
        return "B752".to_string();
    }
    if name_upper.contains("767") || name_lower.contains("767") {
        return "B763".to_string();
    }
    if name_upper.contains("777") || name_lower.contains("777") {
        return "B77W".to_string();
    }
    if name_upper.contains("787") || name_lower.contains("787") {
        return "B788".to_string();
    }
    // McDonnell Douglas
    if name_upper.contains("MD-11F") || name_upper.contains("MD11F") {
        return "MD1F".to_string();
    }
    if name_upper.contains("MD-11") || name_upper.contains("MD11") {
        return "MD11".to_string();
    }
    if name_upper.contains("MD-82") || name_upper.contains("MD82") {
        return "MD82".to_string();
    }
    if name_upper.contains("MD-80") || name_upper.contains("MD80") {
        return "MD82".to_string(); // MD82 is generally a safe default for MD80 series in simbrief
    }
    if name_upper.contains("MD-88") || name_upper.contains("MD88") {
        return "MD88".to_string();
    }
    // Airbus Other
    if name_upper.contains("A300") || name_lower.contains("a300") {
        return "A306".to_string();
    }
    if name_upper.contains("A310") || name_lower.contains("a310") {
        return "A310".to_string();
    }
    // Boeing Other
    if name_upper.contains("717") || name_lower.contains("717") {
        return "B712".to_string();
    }
    if name_upper.contains("727") || name_lower.contains("727") {
        return "B722".to_string();
    }
    // Regional Jets (CRJ / ERJ / E-Jets)
    if name_upper.contains("CRJ-200")
        || name_upper.contains("CRJ 200")
        || name_upper.contains("CRJ200")
    {
        return "CRJ2".to_string();
    }
    if name_upper.contains("CRJ-700")
        || name_upper.contains("CRJ 700")
        || name_upper.contains("CRJ700")
    {
        return "CRJ7".to_string();
    }
    if name_upper.contains("CRJ-900")
        || name_upper.contains("CRJ 900")
        || name_upper.contains("CRJ900")
    {
        return "CRJ9".to_string();
    }
    if name_upper.contains("E170")
        || name_upper.contains("E-170")
        || name_upper.contains("EMBRAER 170")
    {
        return "E170".to_string();
    }
    if name_upper.contains("E175")
        || name_upper.contains("E-175")
        || name_upper.contains("EMBRAER 175")
    {
        return "E175".to_string();
    }
    if name_upper.contains("E190")
        || name_upper.contains("E-190")
        || name_upper.contains("EMBRAER 190")
    {
        return "E190".to_string();
    }
    if name_upper.contains("E195")
        || name_upper.contains("E-195")
        || name_upper.contains("EMBRAER 195")
    {
        return "E195".to_string();
    }
    if name_upper.contains("ERJ-135")
        || name_upper.contains("ERJ 135")
        || name_upper.contains("ERJ135")
    {
        return "E135".to_string();
    }
    if name_upper.contains("ERJ-140")
        || name_upper.contains("ERJ 140")
        || name_upper.contains("ERJ140")
    {
        return "E140".to_string();
    }
    if name_upper.contains("ERJ-145")
        || name_upper.contains("ERJ 145")
        || name_upper.contains("ERJ145")
    {
        return "E145".to_string();
    }
    // Regional Turboprops
    if name_upper.contains("Q400") || name_upper.contains("DASH 8") || name_upper.contains("DH8D") {
        return "DH8D".to_string();
    }
    if name_upper.contains("ATR 72")
        || name_upper.contains("ATR-72")
        || name_upper.contains("ATR72")
    {
        return "AT72".to_string();
    }
    if name_upper.contains("ATR 42")
        || name_upper.contains("ATR-42")
        || name_upper.contains("ATR42")
    {
        return "AT42".to_string();
    }
    // Business Jets / High End
    if name_upper.contains("CHALLENGER 650")
        || name_upper.contains("CL650")
        || name_upper.contains("CL-650")
    {
        return "CL60".to_string();
    }
    if name_upper.contains("CHALLENGER") || name_upper.contains("CL6") {
        return "CL60".to_string();
    }
    if name_upper.contains("CITATION X") || name_upper.contains("C750") {
        return "C750".to_string();
    }
    if name_upper.contains("CITATION MUSTANG") || name_upper.contains("C510") {
        return "C510".to_string();
    }
    if name_upper.contains("TBM") || name_upper.contains("TBM900") || name_upper.contains("TBM-9") {
        return "TBM9".to_string();
    }
    if name_upper.contains("CONCORDE") || name_upper.contains("CONC") {
        return "CONC".to_string();
    }
    // GA Twins / High Performance
    if name_upper.contains("KING AIR") || name_upper.contains("B350") || name_upper.contains("350I")
    {
        return "B350".to_string();
    }
    if name_upper.contains("BARON") || name_upper.contains("BE58") {
        return "BE58".to_string();
    }
    if name_upper.contains("SR22") || name_upper.contains("CIRRUS") {
        return "SR22".to_string();
    }
    // Cessna / GA Basic
    if name_lower.contains("cessna 172")
        || name_lower.contains("c172")
        || name_upper.contains("C172")
    {
        return "C172".to_string();
    }
    if name_lower.contains("cessna 208") || name_lower.contains("caravan") {
        return "C208".to_string();
    }
    if name_upper.contains("C152") || name_lower.contains("cessna 152") {
        return "C152".to_string();
    }
    if name_upper.contains("PA-28")
        || name_upper.contains("PIPER ARCHER")
        || name_upper.contains("PIPER CHEROKEE")
    {
        return "P28A".to_string();
    }
    // Default when nothing matches
    "C172".to_string()
}

/// Builds SimBrief dispatch URL. Uses `orig` (not `dep`) and `dest` per SimBrief API; aircraft type derived from name/tags.
pub fn export_simbrief(plan: &FlightPlan) -> String {
    let ac_type = simbrief_aircraft_type(&plan.aircraft);
    format!(
        "https://www.simbrief.com/system/dispatch.php?orig={}&dest={}&type={}",
        plan.origin.id, plan.destination.id, ac_type
    )
}
/// Dynamically calculates the current local solar phase based on an airport's longitude.
pub fn calculate_solar_time(lon: f64, utc_now: chrono::DateTime<chrono::Utc>) -> TimeKeyword {
    use chrono::{Duration, Timelike};

    // Earth rotates ~15 degrees per hour.
    let offset_hours = lon / 15.0;

    // Process integer minutes directly to bypass float resolution drops
    let offset_dur = Duration::minutes((offset_hours * 60.0) as i64);
    let local_time = utc_now + offset_dur;
    let hour = local_time.hour();

    match hour {
        5..=7 => TimeKeyword::Dawn,
        8..=17 => TimeKeyword::Day,
        18..=19 => TimeKeyword::Dusk,
        _ => TimeKeyword::Night,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_addon(name: &str, tags: Vec<&str>) -> DiscoveredAddon {
        DiscoveredAddon {
            path: PathBuf::from(format!("/test/{}", name)),
            name: name.to_string(),
            addon_type: AddonType::Aircraft {
                variants: vec![],
                livery_count: 0,
                livery_names: vec![],
            },
            tags: tags.into_iter().map(|t| t.to_string()).collect(),
            is_enabled: true,
            is_laminar_default: false,
        }
    }

    #[test]
    fn test_airport_coords_for_poi_fetch() {
        use crate::apt_dat::{Airport, AirportType};
        // With coords: use them
        let apt = Airport {
            id: "EGMC".to_string(),
            name: "Southend".to_string(),
            airport_type: AirportType::Land,
            lat: Some(51.57),
            lon: Some(0.69),
            proj_x: None,
            proj_y: None,
            max_runway_length: None,
            surface_type: None,
        };
        let c = airport_coords_for_poi_fetch(&apt);
        assert_eq!(c, Some((51.57, 0.69)));
        // No coords but known ICAO: fallback
        let apt_no_coords = Airport {
            id: "EGMC".to_string(),
            name: "Southend".to_string(),
            airport_type: AirportType::Land,
            lat: None,
            lon: None,
            proj_x: None,
            proj_y: None,
            max_runway_length: None,
            surface_type: None,
        };
        let c2 = airport_coords_for_poi_fetch(&apt_no_coords);
        assert_eq!(c2, Some((51.5703, 0.6933)));

        // Unknown ICAO, no coords: None
        let apt_unknown = Airport {
            id: "XXXX".to_string(),
            name: "Unknown".to_string(),
            airport_type: AirportType::Land,
            lat: None,
            lon: None,
            proj_x: None,
            proj_y: None,
            max_runway_length: None,
            surface_type: None,
        };
        assert!(airport_coords_for_poi_fetch(&apt_unknown).is_none());
    }

    #[test]
    fn test_jet_speed_estimate() {
        let aircraft = make_addon("Learjet 35", vec!["General Aviation", "Jet"]);
        let prompt = FlightPrompt::default();
        let speed = estimate_speed(&aircraft, &prompt);
        assert_eq!(speed, 350, "Light jets should have 350kts speed");
    }

    #[test]
    fn test_bush_speed_override() {
        let aircraft = make_addon("Cessna 208", vec!["General Aviation", "Turboprop"]);
        let mut prompt = FlightPrompt::default();
        prompt.keywords.flight_type = Some(TypeKeyword::Bush);
        let speed = estimate_speed(&aircraft, &prompt);
        assert_eq!(speed, 100, "Bush keyword slows the aircraft speed");
    }

    fn create_test_airport(id: &str, lat: f64, lon: f64) -> Airport {
        use crate::apt_dat::{AirportType, SurfaceType};
        Airport {
            id: id.to_string(),
            name: format!("Airport {}", id),
            airport_type: AirportType::Land,
            lat: Some(lat),
            lon: Some(lon),
            proj_x: None,
            proj_y: None,
            max_runway_length: Some(2000),
            surface_type: Some(SurfaceType::Hard),
        }
    }

    fn create_test_pack() -> SceneryPack {
        use crate::scenery::{SceneryCategory, SceneryDescriptor, SceneryPackType};
        SceneryPack {
            name: "Global Airports".to_string(),
            path: std::path::PathBuf::from("Custom Scenery/Global Airports"),
            raw_path: None,
            status: SceneryPackType::Active,
            category: SceneryCategory::GlobalAirport,
            airports: Vec::new(),
            tiles: Vec::new(),
            tags: Vec::new(),
            descriptor: SceneryDescriptor::default(),
            region: None,
        }
    }

    #[test]
    fn test_region_selection_by_bounds() {
        // 1. Create a "Global Airports" pack
        let mut pack = create_test_pack();

        // 2. Add Swiss Airports
        let lszh = create_test_airport("LSZH", 47.458, 8.555); // Zurich
        let lsgg = create_test_airport("LSGG", 46.230, 6.110); // Geneva

        // 3. Add Italian Airport
        let lirf = create_test_airport("LIRF", 41.800, 12.238);

        pack.airports.push(lszh);
        pack.airports.push(lsgg);
        pack.airports.push(lirf);

        // 4. Create aircraft
        let aircraft = make_addon("Cessna 172", vec!["General Aviation"]);

        // 5. Prompt for "Switzerland"
        // The parser should map "Switzerland" -> Region("CH")
        // flight_gen should filter based on bounds, picking LSZH and ignoring LIRF
        let prompt = "Flight from Switzerland to Switzerland using a Cessna";

        // This test simulates the logic inside generate_flight's origin selection
        // We can call generate_flight directly
        let result = generate_flight(&[pack], &[aircraft], prompt, None, None, None);

        // Assertions
        if let Ok(plan) = result {
            let valid_origins = ["LSZH", "LSGG"];
            assert!(
                valid_origins.contains(&plan.origin.id.as_str()),
                "Origin should be Swiss (LSZH/LSGG), got {}",
                plan.origin.id
            );
            assert!(
                valid_origins.contains(&plan.destination.id.as_str()),
                "Destination should be Swiss (LSZH/LSGG), got {}",
                plan.destination.id
            );
            assert_ne!(plan.origin.id, "LIRF", "Should NOT pick Rome");
            assert_ne!(plan.destination.id, "LIRF", "Should NOT pick Rome");
        } else {
            panic!("Flight generation failed: {:?}", result.err());
        }
    }

    #[test]
    fn test_simbrief_url_orig_dest_type() {
        // SimBrief expects orig= (not dep=), dest=, and type= (ICAO aircraft)
        let a320 = make_addon("ToLissA320_V1p0", vec!["Airliner", "Jet"]);
        let plan = FlightPlan {
            origin: create_test_airport("EGMC", 51.57, 0.70),
            destination: create_test_airport("LIRF", 41.80, 12.24),
            aircraft: a320,
            distance_nm: 753,
            duration_minutes: 100,
            route_description: "Direct".to_string(),
            origin_region_id: Some("UK:London".to_string()),
            dest_region_id: Some("IT".to_string()),
            context: None,
            time: None,
            weather: None,
            weather_confirmed: false,
        };
        let url = export_simbrief(&plan);
        assert!(
            url.contains("orig=EGMC"),
            "SimBrief URL must use orig= for departure: {}",
            url
        );
        assert!(
            url.contains("dest=LIRF"),
            "SimBrief URL must use dest= for destination: {}",
            url
        );
        assert!(
            url.contains("type=A320"),
            "SimBrief URL must use A320 for ToLiss A320: {}",
            url
        );
        assert!(
            !url.contains("dep="),
            "SimBrief does not use dep= parameter: {}",
            url
        );
    }

    #[test]
    fn test_load_flight_context_from_json() {
        let json = r#"{
            "EGLL": {
                "snippet": "Heathrow is the largest UK airport.",
                "points_nearby": [
                    {"name": "Windsor", "kind": "landmark", "snippet": "Castle.", "lat": 51.48, "lon": -0.60}
                ]
            },
            "LIRF": {
                "snippet": "Rome Fiumicino.",
                "points_nearby": []
            }
        }"#;
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(tmp.path(), json).expect("write");
        let origin = create_test_airport("EGLL", 51.47, -0.45);
        let dest = create_test_airport("LIRF", 41.80, 12.24);
        let ctx = load_flight_context(tmp.path(), &origin, &dest).expect("load");
        assert_eq!(ctx.origin.icao, "EGLL");
        assert!(ctx.origin.snippet.contains("Heathrow"));
        // File has 1 POI (Windsor); overlay adds 1 (Windsor Castle) for EGLL → 2 total
        assert!(!ctx.origin.points_nearby.is_empty());
        assert!(ctx.origin.points_nearby.iter().any(|p| p.name == "Windsor"));
        assert!(ctx
            .origin
            .points_nearby
            .iter()
            .any(|p| p.name == "Windsor Castle"));

        // Option B: bundled + load order (cache → bundled → config)
        let bundled = get_bundled_flight_context();
        assert!(
            bundled.len() >= 50,
            "bundle should have at least 50 airports (Phase 1.1), got {}",
            bundled.len()
        );
        assert!(
            bundled.contains_key("EGLL") && bundled.contains_key("LIRF"),
            "bundle should contain EGLL and LIRF"
        );
        let cache_dir = tempfile::tempdir().expect("tempdir");
        let config_path = tempfile::NamedTempFile::new().expect("temp config");
        std::fs::write(config_path.path(), "{}").expect("empty config");
        let ctx2 = load_flight_context_with_bundled(
            &bundled,
            config_path.path(),
            cache_dir.path(),
            &origin,
            &dest,
            None,
            None,
        )
        .expect("load with bundle");
        assert_eq!(ctx2.origin.icao, "EGLL");
        assert!(ctx2.origin.snippet.contains("Heathrow") || ctx2.origin.snippet.contains("London"));
        assert_eq!(
            get_icao_to_wikipedia().get("EGLL").map(String::as_str),
            Some("Heathrow_Airport")
        );
        let windsor_poi = ctx
            .origin
            .points_nearby
            .iter()
            .find(|p| p.name == "Windsor")
            .expect("Windsor from file");
        assert!(windsor_poi.distance_nm.unwrap() <= 10.0);
        assert_eq!(ctx.destination.icao, "LIRF");
        assert!(ctx.destination.snippet.contains("Fiumicino"));
        // LIRF gets overlay POI (Ostia Antica)
        assert!(ctx
            .destination
            .points_nearby
            .iter()
            .any(|p| p.name == "Ostia Antica"));
    }
}
