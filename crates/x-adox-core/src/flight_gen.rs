use crate::apt_dat::{Airport, AirportType, AptDatParser, SurfaceType};
use crate::discovery::{AddonType, DiscoveredAddon};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::OnceLock;
use x_adox_bitnet::flight_prompt::{
    AircraftConstraint, DurationKeyword, FlightPrompt, LocationConstraint, SurfaceKeyword,
    TypeKeyword,
};
use x_adox_bitnet::geo::RegionIndex;
use x_adox_bitnet::HeuristicsConfig;

/// London area bounds (lat 51–52, lon -1 to 0.6). Excludes e.g. Great Yarmouth (52.6°N, 1.7°E).
/// Used when region is UK:London so origin/dest are always restricted to London even if region index is stale.
fn in_bounds_london(lat: f64, lon: f64) -> bool {
    lat >= 51.0 && lat <= 52.0 && lon >= -1.0 && lon <= 0.6
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
    pub icao_map: std::collections::HashMap<&'a str, &'a Airport>,
    pub name_map: std::collections::HashMap<String, Vec<usize>>, // Lowercase name -> indices in airports
    pub search_names: Vec<String>, // Parallel to airports, pre-lowercased
}

impl<'a> AirportPool<'a> {
    pub fn new(source: &'a [Airport]) -> Self {
        let airports: Vec<&Airport> = source.iter().collect();
        let mut icao_map = std::collections::HashMap::with_capacity(airports.len());
        let mut name_map = std::collections::HashMap::with_capacity(airports.len());
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
    for result in rdr.records() {
        if let Ok(record) = result {
            if record.len() >= 2 {
                let ident = record[0].trim().to_string();
                let title = record[1].trim().to_string();
                if !ident.is_empty() && !title.is_empty() {
                    map.insert(ident, title);
                }
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
    serde_json::to_writer_pretty(file, data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
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
}

use crate::scenery::SceneryPack;

// Helpers with Prompt Context
fn estimate_speed(a: &DiscoveredAddon, prompt: &FlightPrompt) -> u32 {
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
    } else if tags_joined.contains("helicopter") || tags_joined.contains("helo") {
        100
    } else if tags_joined.contains("seaplane") || tags_joined.contains("float") {
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
) -> Result<FlightPlan, String> {
    generate_flight_with_pool(packs, aircraft_list, prompt_str, base_airports, prefs, None)
}

/// A high-performance version of [generate_flight] that can use a pre-computed airport pool.
pub fn generate_flight_with_pool(
    packs: &[SceneryPack],
    aircraft_list: &[DiscoveredAddon],
    prompt_str: &str,
    base_airports: Option<&[Airport]>,
    prefs: Option<&HeuristicsConfig>,
    precomputed_pool: Option<&AirportPool>,
) -> Result<FlightPlan, String> {
    let prompt = FlightPrompt::parse(prompt_str);
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
    let speed_kts = estimate_speed(selected_aircraft, &prompt);
    // Surface preference comes from keywords only — not aircraft type
    let req_surface: Option<SurfaceType> = match prompt.keywords.surface {
        Some(SurfaceKeyword::Soft) => Some(SurfaceType::Soft),
        Some(SurfaceKeyword::Hard) => Some(SurfaceType::Hard),
        None => None,
    };
    log::debug!(
        "[flight_gen] selected_aircraft='{}' tags={:?} req_surface={:?}",
        selected_aircraft.name,
        selected_aircraft.tags,
        req_surface
    );

    // 2. Select Origin
    let is_b314 = selected_aircraft.name.contains("Boeing 314");

    // Combined pool: pack airports + base layer merged. For B314, exclude base and non-Sealanes packs.
    let pack_iter = packs.iter().filter(|p| {
        if is_b314
            && (prompt.origin.is_none() || matches!(prompt.origin, Some(LocationConstraint::Any)))
        {
            p.path.to_string_lossy().contains("B314 Sealanes")
        } else {
            true
        }
    });
    let base_slice = if is_b314 {
        &[] as &[Airport]
    } else {
        base_airports.unwrap_or(&[])
    };

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
        let mut pool: BTreeMap<String, Airport> = BTreeMap::new();

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
    let icao_index_owned: std::collections::HashMap<&str, &Airport>;
    let icao_index = if let Some(p) = precomputed_pool {
        &p.icao_map
    } else {
        icao_index_owned = all_airports.iter().map(|a| (a.id.as_str(), *a)).collect();
        &icao_index_owned
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
                    if !prompt.ignore_guardrails {
                        if !check_safety_constraints(apt, selected_aircraft, req_surface) {
                            return false;
                        }
                    }
                    apt.lat.is_some() && apt.lon.is_some()
                })
                .map(|a| *a)
                .collect()
        }
        Some(LocationConstraint::AirportName(ref name)) => score_airports_by_name(
            &all_airports,
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
        if let Some(LocationConstraint::Region(ref r)) = &prompt.origin {
            seed_origin_fallback = get_seed_airports_for_region(r);
            if !seed_origin_fallback.is_empty() {
                candidate_origins = seed_origin_fallback.iter().collect();
            }
        } else if let Some(LocationConstraint::NearCity { lat, lon, .. }) = &prompt.origin {
            // No pack airports near the city — derive region from coordinates and use seeds.
            for region in region_index.find_regions(*lat, *lon) {
                let seeds = get_seed_airports_for_region(&region.id);
                if !seeds.is_empty() {
                    seed_origin_fallback = seeds;
                    candidate_origins = seed_origin_fallback.iter().collect();
                    break;
                }
            }
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
        if let (Some(ref last), Some(LocationConstraint::Region(ref dest_r))) = (
            prefs.and_then(|c| c.flight_last_success.as_ref()),
            &prompt.destination,
        ) {
            if last.origin_region == *region_id
                && last.dest_region == *dest_r
                && !last.origin_icao.is_empty()
            {
                if !preferred_icaos.contains(&last.origin_icao) {
                    preferred_icaos.insert(0, last.origin_icao.clone());
                }
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
        } else {
            // No keyword constraint: wide-open random discovery.
            // Keywords (short/medium/long/haul) and duration_minutes are the
            // intended controls — aircraft type no longer sets distance limits.
            (10.0, 5000.0)
        };

        // Explicit Endpoint Check (Relax distance logic if both ends are specific)
        let endpoints_explicit = match (&prompt.origin, &prompt.destination) {
            (Some(o), Some(d)) => {
                !matches!(o, LocationConstraint::Any) && !matches!(d, LocationConstraint::Any)
            }
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
                        if !prompt.ignore_guardrails {
                            if !check_safety_constraints(apt, selected_aircraft, req_surface) {
                                return false;
                            }
                        }
                        apt.lat.is_some() && apt.lon.is_some()
                    })
                    .copied()
                    .collect()
            }
            Some(LocationConstraint::AirportName(ref name)) => score_airports_by_name(
                &all_airports,
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
            if let Some(LocationConstraint::Region(ref r)) = &prompt.destination {
                seed_dest_fallback = get_seed_airports_for_region(r);
                if !seed_dest_fallback.is_empty() {
                    candidate_dests = seed_dest_fallback.iter().collect();
                }
            } else if let Some(LocationConstraint::NearCity { lat, lon, .. }) =
                &prompt.destination
            {
                // No pack airports near the city — derive region and use seeds.
                for region in region_index.find_regions(*lat, *lon) {
                    let seeds = get_seed_airports_for_region(&region.id);
                    if !seeds.is_empty() {
                        seed_dest_fallback = seeds;
                        candidate_dests = seed_dest_fallback.iter().collect();
                        break;
                    }
                }
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
            let bbox_max = if endpoints_explicit { 20000.0 } else { max_dist };
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
                    if let (Some(ref last), Some(LocationConstraint::Region(ref orig_r))) = (
                        prefs.and_then(|c| c.flight_last_success.as_ref()),
                        &prompt.origin,
                    ) {
                        if last.dest_region == *region_id
                            && last.origin_region == *orig_r
                            && !last.dest_icao.is_empty()
                        {
                            if !icaos.contains(&last.dest_icao) {
                                icaos.insert(0, last.dest_icao.clone());
                            }
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
    // Keyword surface preference (grass/paved keywords, or bush → soft)
    if let Some(req_surf) = req_surface {
        if let Some(surf) = apt.surface_type {
            match req_surf {
                SurfaceType::Soft if surf != SurfaceType::Soft => return false,
                SurfaceType::Hard if surf != SurfaceType::Hard => return false,
                _ => {}
            }
        }
    }
    true
}

fn score_airports_by_name<'a>(
    airports: &[&'a Airport],
    search_names: Option<&[String]>,
    name_map: Option<&std::collections::HashMap<String, Vec<usize>>>,
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

/// Seed airports used only when the pool (scenery packs + base layer) has no candidates for
/// that region. Global coverage comes from the base layer (Resources + Global Scenery apt.dat);
/// we seed only a few high-traffic regions so prompts like "London to Paris" still work without
/// scenery. Parent fallback applies for sub-regions (e.g. US:SoCal → US seeds).
fn get_seed_airports_for_region(region_id: &str) -> Vec<Airport> {
    let direct = match region_id {
        // GB has no seeds: it excludes Northern Ireland; do not fall back to UK seeds.
        "GB" => return Vec::new(),
        "UK:London" => vec![
            seed_airport("EGLL", "London Heathrow", 51.4700, -0.4543),
            seed_airport("EGKK", "London Gatwick", 51.1481, -0.1903),
            seed_airport("EGGW", "London Luton", 51.8747, -0.3683),
            seed_airport("EGSS", "London Stansted", 51.8849, 0.2346),
            seed_airport("EGLC", "London City", 51.5053, 0.0553),
            seed_airport("EGKB", "London Biggin Hill", 51.3308, 0.0325),
            seed_airport("EGWU", "London Northolt", 51.5530, -0.4182),
            seed_airport("EGLF", "Farnborough", 51.2758, -0.7763),
        ],
        "UK" | "BI" => vec![
            seed_airport("EGLL", "London Heathrow", 51.4700, -0.4543),
            seed_airport("EGKK", "London Gatwick", 51.1481, -0.1903),
            seed_airport("EGGW", "London Luton", 51.8747, -0.3683),
            seed_airport("EGSS", "London Stansted", 51.8849, 0.2346),
            seed_airport("EGLC", "London City", 51.5053, 0.0553),
            seed_airport("EGKB", "London Biggin Hill", 51.3308, 0.0325),
            seed_airport("EGWU", "London Northolt", 51.5530, -0.4182),
            seed_airport("EGLF", "Farnborough", 51.2758, -0.7763),
            seed_airport("EGCC", "Manchester", 53.3537, -2.2750),
            seed_airport("EGBB", "Birmingham", 52.4539, -1.7480),
            seed_airport("EGPH", "Edinburgh", 55.9500, -3.3725),
            seed_airport("EGGP", "Liverpool", 53.3336, -2.8497),
        ],
        "IT" => vec![
            seed_airport("LIRF", "Rome Fiumicino", 41.8003, 12.2389),
            seed_airport("LIML", "Milan Malpensa", 45.6301, 8.7281),
            seed_airport("LIPE", "Bologna", 44.5354, 11.2887),
            seed_airport("LIBD", "Bari", 41.1389, 16.7606),
        ],
        "FR" => vec![
            seed_airport("LFPG", "Paris Charles de Gaulle", 49.0097, 2.5478),
            seed_airport("LFLL", "Lyon", 45.7256, 5.0811),
            seed_airport("LFML", "Marseille", 43.4393, 5.2214),
        ],
        "DE" => vec![
            seed_airport("EDDF", "Frankfurt", 50.0379, 8.5622),
            seed_airport("EDDM", "Munich", 48.3538, 11.7751),
            seed_airport("EDDK", "Cologne Bonn", 50.8659, 7.1427),
        ],
        "ES" => vec![
            seed_airport("LEMD", "Madrid Barajas", 40.4983, -3.5676),
            seed_airport("LEBL", "Barcelona El Prat", 41.2971, 2.0785),
        ],
        "US:AK" => vec![
            seed_airport("PANC", "Anchorage Ted Stevens", 61.1743, -149.9962),
            seed_airport("PAFA", "Fairbanks Intl", 64.8151, -147.8561),
            seed_airport("PAJN", "Juneau Intl", 58.3549, -134.5762),
            seed_airport("PABT", "Bettles", 66.9139, -151.5291), // Bush flavor
        ],
        "US:HI" => vec![
            seed_airport("PHNL", "Honolulu Intl", 21.3187, -157.9225),
            seed_airport("PHOG", "Kahului", 20.8986, -156.4305),
        ],
        "US" => vec![
            seed_airport("KJFK", "New York JFK", 40.6398, -73.7789),
            seed_airport("KLAX", "Los Angeles", 33.9425, -118.4081),
            seed_airport("KORD", "Chicago O'Hare", 41.9786, -87.9047),
            seed_airport("KATL", "Atlanta", 33.6367, -84.4281),
        ],
        "MX" => vec![
            seed_airport("MMMX", "Mexico City Benito Juarez", 19.4363, -99.0721),
            seed_airport("MMUN", "Cancun", 21.0365, -86.8770),
            seed_airport("MMMD", "Monterrey", 25.7785, -100.1070),
            seed_airport("MMGL", "Guadalajara Miguel Hidalgo", 20.5218, -103.3107),
        ],
        "CA" => vec![
            seed_airport("CYVR", "Vancouver", 49.1967, -123.1815),
            seed_airport("CYYZ", "Toronto Pearson", 43.6777, -79.6248),
            seed_airport("CYUL", "Montreal", 45.4706, -73.7408),
        ],
        "ZA" => vec![
            seed_airport("FALE", "King Shaka Durban", -29.6144, 31.1197),
            seed_airport("FAOR", "Johannesburg O.R. Tambo", -26.1367, 28.2411),
            seed_airport("FACT", "Cape Town Intl", -33.9715, 18.6021),
        ],
        "KE" => vec![
            seed_airport("HKJK", "Nairobi Jomo Kenyatta", -1.3192, 36.9275),
            seed_airport("HKMO", "Mombasa Moi", -4.0348, 39.5943),
            seed_airport("HKLU", "Lamu Manda", -2.2717, 40.9131),
            seed_airport("HKML", "Malindi", -3.2293, 40.1017),
        ],
        "IE" => vec![
            seed_airport("EIDW", "Dublin", 53.4263, -6.2499),
            seed_airport("EICK", "Cork", 51.8413, -8.4911),
        ],
        "BE" => vec![seed_airport("EBBR", "Brussels", 50.9014, 4.4844)],
        "NL" => vec![seed_airport("EHAM", "Amsterdam Schiphol", 52.3086, 4.7639)],
        "CH" => vec![seed_airport("LSZH", "Zurich", 47.4647, 8.5492)],
        "AT" => vec![seed_airport("LOWW", "Vienna", 48.1103, 16.5697)],
        // Africa (new)
        "TZ" => vec![
            seed_airport("HTDA", "Dar es Salaam Julius Nyerere", -6.8781, 39.2026),
            seed_airport("HTZA", "Zanzibar Abeid Amani Karume", -6.2220, 39.2249),
            seed_airport("HTKJ", "Kilimanjaro Intl", -3.4294, 37.0745),
        ],
        "ET" => vec![seed_airport("HAAB", "Addis Ababa Bole", 8.9779, 38.7993)],
        "NG" => vec![
            seed_airport("DNMM", "Lagos Murtala Muhammed", 6.5774, 3.3211),
            seed_airport("DNAA", "Abuja Nnamdi Azikiwe", 9.0068, 7.2632),
        ],
        "MA" => vec![
            seed_airport("GMMN", "Casablanca Mohammed V", 33.3675, -7.5898),
            seed_airport("GMMX", "Marrakech Menara", 31.6069, -8.0363),
        ],
        "EG" => vec![seed_airport("HECA", "Cairo Intl", 30.1219, 31.4056)],
        // Asia-Pacific
        "JP" => vec![
            seed_airport("RJAA", "Tokyo Narita", 35.7653, 140.3856),
            seed_airport("RJTT", "Tokyo Haneda", 35.5494, 139.7798),
            seed_airport("RJBB", "Osaka Kansai", 34.4347, 135.2440),
            seed_airport("RJOO", "Osaka Itami", 34.7855, 135.4380),
            seed_airport("RJCC", "Sapporo New Chitose", 42.7752, 141.6922),
            seed_airport("RJFF", "Fukuoka", 33.5858, 130.4511),
            seed_airport("ROAH", "Okinawa Naha", 26.1958, 127.6461),
        ],
        "KR" => vec![
            seed_airport("RKSI", "Seoul Incheon", 37.4691, 126.4510),
            seed_airport("RKSS", "Seoul Gimpo", 37.5583, 126.7908),
            seed_airport("RKPK", "Busan Gimhae", 35.1795, 128.9382),
        ],
        "CN" => vec![
            seed_airport("ZBAA", "Beijing Capital", 40.0799, 116.6031),
            seed_airport("ZBAD", "Beijing Daxing", 39.5093, 116.4105),
            seed_airport("ZSPD", "Shanghai Pudong", 31.1434, 121.8052),
            seed_airport("ZSSS", "Shanghai Hongqiao", 31.1979, 121.3362),
            seed_airport("ZGGG", "Guangzhou Baiyun", 23.3924, 113.2990),
            seed_airport("ZUCK", "Chongqing Jiangbei", 29.7192, 106.6417),
            seed_airport("ZUUU", "Chengdu Shuangliu", 30.5783, 103.9472),
            seed_airport("ZLXY", "Xi'an Xianyang", 34.4471, 108.7516),
        ],
        "IN" => vec![
            seed_airport("VABB", "Mumbai Chhatrapati Shivaji", 19.0887, 72.8679),
            seed_airport("VIDP", "Delhi Indira Gandhi", 28.5665, 77.1031),
            seed_airport("VOMM", "Chennai", 12.9900, 80.1693),
            seed_airport("VOBL", "Bangalore Kempegowda", 13.1979, 77.7063),
            seed_airport("VECC", "Kolkata Netaji Subhas", 22.6527, 88.4463),
            seed_airport("VAAH", "Ahmedabad Sardar Vallabhbhai Patel", 23.0772, 72.6347),
        ],
        "AU" => vec![
            seed_airport("YSSY", "Sydney Kingsford Smith", -33.9461, 151.1772),
            seed_airport("YMML", "Melbourne Tullamarine", -37.6733, 144.8430),
            seed_airport("YBBN", "Brisbane", -27.3842, 153.1175),
            seed_airport("YPPH", "Perth", -31.9403, 115.9669),
            seed_airport("YPAD", "Adelaide", -34.9450, 138.5308),
            seed_airport("YBCS", "Cairns", -16.8858, 145.7553),
            seed_airport("YPDN", "Darwin", -12.4147, 130.8766),
            seed_airport("YMHB", "Hobart", -42.8361, 147.5103),
        ],
        "ID" => vec![
            seed_airport("WADD", "Bali Ngurah Rai", -8.7482, 115.1670),
            seed_airport("WIII", "Jakarta Soekarno-Hatta", -6.1256, 106.6559),
            seed_airport("WBSB", "Bandar Seri Begawan", 4.9442, 114.9283),
            seed_airport("WAAA", "Makassar Sultan Hasanuddin", -5.0616, 119.5540),
        ],
        "TH" => vec![
            seed_airport("VTBS", "Bangkok Suvarnabhumi", 13.6811, 100.7477),
            seed_airport("VTBD", "Bangkok Don Mueang", 13.9126, 100.6072),
            seed_airport("VTSP", "Phuket", 8.1132, 98.3169),
            seed_airport("VTCC", "Chiang Mai", 18.7667, 98.9626),
        ],
        "VN" => vec![
            seed_airport("VVTS", "Ho Chi Minh City Tan Son Nhat", 10.8188, 106.6519),
            seed_airport("VVNB", "Hanoi Noi Bai", 21.2212, 105.8072),
            seed_airport("VVDN", "Da Nang", 16.0439, 108.1993),
        ],
        "SG" => vec![seed_airport("WSSS", "Singapore Changi", 1.3502, 103.9940)],
        "MY" => vec![
            seed_airport("WMKK", "Kuala Lumpur Intl", 2.7456, 101.7099),
            seed_airport("WMKC", "Kuala Lumpur City (Subang)", 3.1308, 101.5494),
        ],
        "PH" => vec![
            seed_airport("RPLL", "Manila Ninoy Aquino", 14.5086, 121.0198),
            seed_airport("RPVM", "Cebu Mactan", 10.3097, 123.9792),
        ],
        "HK" => vec![seed_airport("VHHH", "Hong Kong Intl", 22.3080, 113.9185)],
        "TW" => vec![seed_airport("RCTP", "Taipei Taoyuan", 25.0777, 121.2325)],
        "QA" => vec![seed_airport("OTHH", "Doha Hamad", 25.2731, 51.6081)],
        "AE" => vec![
            seed_airport("OMDB", "Dubai Intl", 25.2528, 55.3644),
            seed_airport("OMAA", "Abu Dhabi", 24.4328, 54.6511),
        ],
        "SA" => vec![
            seed_airport("OERK", "Riyadh King Khalid", 24.9576, 46.6988),
            seed_airport("OEJN", "Jeddah King Abdulaziz", 21.6796, 39.1565),
        ],
        "TR" => vec![
            seed_airport("LTFM", "Istanbul", 41.2753, 28.7519),
            seed_airport("LTAI", "Antalya", 36.8988, 30.7992),
        ],
        "GR" => vec![
            seed_airport("LGAV", "Athens Eleftherios Venizelos", 37.9364, 23.9445),
            seed_airport("LGRP", "Rhodes Diagoras", 36.4054, 28.0862),
        ],
        "PT" => vec![
            seed_airport("LPPT", "Lisbon Humberto Delgado", 38.7756, -9.1354),
            seed_airport("LPPR", "Porto Francisco Sa Carneiro", 41.2481, -8.6814),
        ],
        "CU" => vec![seed_airport("MUHA", "Havana Jose Marti", 22.9892, -82.4091)],
        "PA" => vec![seed_airport("MPTO", "Panama City Tocumen", 9.0714, -79.3835)],
        "CR" => vec![seed_airport("MROC", "San Jose Juan Santamaria", 9.9939, -84.2088)],
        "LK" => vec![seed_airport("VCBI", "Colombo Bandaranaike", 7.1808, 79.8841)],
        "NP" => vec![seed_airport("VNKT", "Kathmandu Tribhuvan", 27.6966, 85.3591)],
        "PK" => vec![
            seed_airport("OPKC", "Karachi Jinnah", 24.9065, 67.1608),
            seed_airport("OPLA", "Lahore Allama Iqbal", 31.5216, 74.4036),
            seed_airport("OPRN", "Islamabad New", 33.6167, 73.0997),
        ],
        "BD" => vec![seed_airport("VGHS", "Dhaka Hazrat Shahjalal", 23.8433, 90.3978)],
        "MM" => vec![seed_airport("VYYY", "Yangon", 16.9073, 96.1332)],
        "NZ" => vec![
            seed_airport("NZAA", "Auckland", -37.0082, 174.7917),
            seed_airport("NZWN", "Wellington", -41.3272, 174.8053),
            seed_airport("NZQN", "Queenstown", -45.0211, 168.7392),
            seed_airport("NZCH", "Christchurch", -43.4894, 172.5322),
        ],
        "FJ" => vec![seed_airport("NFFN", "Nadi", -17.7554, 177.4431)],
        // South America
        "BR" => vec![
            seed_airport("SBGR", "Sao Paulo Guarulhos", -23.4356, -46.4731),
            seed_airport("SBGL", "Rio de Janeiro Galeao", -22.8099, -43.2505),
            seed_airport("SBSV", "Salvador", -12.9086, -38.3225),
            seed_airport("SBFZ", "Fortaleza", -3.7763, -38.5326),
            seed_airport("SBPA", "Porto Alegre", -29.9944, -51.1714),
            seed_airport("SBMN", "Manaus Eduardo Gomes", -3.0386, -60.0497),
        ],
        "AR" => vec![
            seed_airport("SAEZ", "Buenos Aires Ezeiza", -34.8222, -58.5358),
            seed_airport("SABE", "Buenos Aires Aeroparque", -34.5592, -58.4156),
        ],
        "CO" => vec![seed_airport("SKBO", "Bogota El Dorado", 4.7016, -74.1469)],
        "PE" => vec![seed_airport("SPJC", "Lima Jorge Chavez", -12.0219, -77.1143)],
        "CL" => vec![seed_airport("SCEL", "Santiago Arturo Merino Benitez", -33.3930, -70.7858)],
        "VE" => vec![seed_airport("SVMI", "Caracas Simon Bolivar", 10.6031, -66.9906)],
        "EC" => vec![seed_airport("SEQU", "Quito Mariscal Sucre", -0.1292, -78.3576)],
        "UY" => vec![seed_airport("SUMU", "Montevideo Carrasco", -34.8384, -56.0308)],
        "PY" => vec![seed_airport("SGAS", "Asuncion Silvio Pettirossi", -25.2400, -57.5197)],
        "BO" => vec![seed_airport("SLLP", "La Paz El Alto", -16.5133, -68.1922)],
        // Africa extras
        "GH" => vec![seed_airport("DGAA", "Accra Kotoka", 5.6052, -0.1668)],
        "SN" => vec![seed_airport("GOBD", "Dakar Blaise Diagne", 14.6706, -17.1025)],
        "TN" => vec![seed_airport("DTTA", "Tunis Carthage", 36.8510, 10.2272)],
        "LY" => vec![seed_airport("HLLT", "Tripoli Mitiga", 32.8942, 13.2760)],
        "SD" => vec![seed_airport("HSSS", "Khartoum", 15.5895, 32.5532)],
        "UG" => vec![seed_airport("HUEN", "Entebbe", 0.0424, 32.4435)],
        "RW" => vec![seed_airport("HRYR", "Kigali", -1.9686, 30.1395)],
        "ZM" => vec![seed_airport("FLKK", "Lusaka Kenneth Kaunda", -15.3308, 28.4526)],
        "ZW" => vec![seed_airport("FVHA", "Harare", -17.9318, 31.0928)],
        "MG" => vec![seed_airport("FMMI", "Antananarivo Ivato", -18.7969, 47.4788)],
        // Central America / Caribbean extras
        "JM" => vec![seed_airport("MKJP", "Kingston Norman Manley", 17.9357, -76.7875)],
        "BS" => vec![seed_airport("MYNN", "Nassau Lynden Pindling", 25.0390, -77.4662)],
        "HT" => vec![seed_airport("MTPP", "Port-au-Prince Toussaint", 18.5800, -72.2926)],
        "DO" => vec![seed_airport("MDSD", "Santo Domingo Las Americas", 18.4297, -69.6689)],
        "GT" => vec![seed_airport("MGGT", "Guatemala City La Aurora", 14.5832, -90.5275)],
        "SV" => vec![seed_airport("MSSS", "San Salvador", 13.4409, -89.0557)],
        "HN" => vec![seed_airport("MHLM", "San Pedro Sula", 15.4527, -87.9236)],
        "NI" => vec![seed_airport("MNMG", "Managua", 12.1415, -86.1682)],
        // Europe extras
        "PL" => vec![
            seed_airport("EPWA", "Warsaw Chopin", 52.1657, 20.9671),
            seed_airport("EPKK", "Krakow John Paul II", 50.0778, 19.7848),
        ],
        "CZ" => vec![seed_airport("LKPR", "Prague Vaclav Havel", 50.1008, 14.2600)],
        "HU" => vec![seed_airport("LHBP", "Budapest Ferenc Liszt", 47.4390, 19.2611)],
        "RO" => vec![seed_airport("LROP", "Bucharest Henri Coanda", 44.5711, 26.0850)],
        "BG" => vec![seed_airport("LBSF", "Sofia", 42.6967, 23.4114)],
        "RS" => vec![seed_airport("LYBE", "Belgrade Nikola Tesla", 44.8184, 20.3091)],
        "HR" => vec![
            seed_airport("LDZA", "Zagreb", 45.7429, 16.0688),
            seed_airport("LDDU", "Dubrovnik", 42.5614, 18.2682),
            seed_airport("LDSP", "Split", 43.5390, 16.2980),
        ],
        "LV" => vec![seed_airport("EVRA", "Riga", 56.9236, 23.9711)],
        "EE" => vec![seed_airport("EETN", "Tallinn Lennart Meri", 59.4133, 24.8328)],
        "LT" => vec![seed_airport("EYVI", "Vilnius", 54.6341, 25.2858)],
        "SK" => vec![seed_airport("LZIB", "Bratislava", 48.1702, 17.2127)],
        "LU" => vec![seed_airport("ELLX", "Luxembourg Findel", 49.6234, 6.2044)],
        "IS" => vec![seed_airport("BIRK", "Reykjavik", 64.1300, -21.9406)],
        "NO" => vec![seed_airport("ENGM", "Oslo Gardermoen", 60.1939, 11.1004)],
        "SE" => vec![seed_airport("ESSA", "Stockholm Arlanda", 59.6519, 17.9186)],
        "DK" => vec![seed_airport("EKCH", "Copenhagen Kastrup", 55.6180, 12.6561)],
        "FI" => vec![seed_airport("EFHK", "Helsinki Vantaa", 60.3172, 24.9633)],
        "IL" => vec![seed_airport("LLBG", "Tel Aviv Ben Gurion", 32.0114, 34.8867)],
        "JO" => vec![seed_airport("OJAI", "Amman Queen Alia", 31.7226, 35.9932)],
        "LB" => vec![seed_airport("OLBA", "Beirut Rafic Hariri", 33.8209, 35.4883)],
        "KW" => vec![seed_airport("OKBK", "Kuwait Intl", 29.2266, 47.9689)],
        "OM" => vec![seed_airport("OOMS", "Muscat", 23.5933, 58.2844)],
        "IQ" => vec![seed_airport("ORBI", "Baghdad", 33.2626, 44.2346)],
        "IR" => vec![seed_airport("OIIE", "Tehran Imam Khomeini", 35.4161, 51.1522)],
        _ => Vec::new(),
    };
    if !direct.is_empty() {
        return direct;
    }
    // Parent fallback: US:SoCal, US:OR, US:NorCal, etc. -> US
    if region_id.contains(':') {
        let parent = region_id.split(':').next().unwrap_or(region_id);
        return get_seed_airports_for_region(parent);
    }
    direct
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
pub fn export_fms_11(plan: &FlightPlan) -> String {
    format!(
        "I\n1100 Version\nCYCLE 1709\nADEP {}\nADES {}\nNUMENR 0\n",
        plan.origin.id, plan.destination.id
    )
}

pub fn export_fms_12(plan: &FlightPlan) -> String {
    // XP12 FMS usually just uses same 1100 version or 3? FMS 3 is old. 1100 is standard.
    // Let's stick to 1100 unless we find otherwise.
    export_fms_11(plan)
}

pub fn export_lnmpln(plan: &FlightPlan) -> String {
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
      <ProgramVersion>2.4.0</ProgramVersion>
      <Documentation>{}</Documentation>
    </Header>
    <SimData>XPlane12</SimData>
    <NavData Cycle="1801">NAVIGRAPH</NavData>
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
    // Cessna / GA
    if name_lower.contains("cessna 172")
        || name_lower.contains("c172")
        || name_upper.contains("C172")
    {
        return "C172".to_string();
    }
    if name_lower.contains("cessna 208") || name_lower.contains("caravan") {
        return "C208".to_string();
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
        let result = generate_flight(&[pack], &[aircraft], prompt, None, None);

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
        assert!(ctx.origin.points_nearby.len() >= 1);
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
