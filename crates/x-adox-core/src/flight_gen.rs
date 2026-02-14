use crate::apt_dat::{Airport, AptDatParser, AirportType, SurfaceType};
use crate::discovery::{AddonType, DiscoveredAddon};
use rand::seq::SliceRandom;
use std::path::Path;
use x_adox_bitnet::flight_prompt::{
    AircraftConstraint, DurationKeyword, FlightPrompt, LocationConstraint, SurfaceKeyword,
    TypeKeyword,
};

use x_adox_bitnet::geo::RegionIndex;

/// Loads the default airport database from X-Plane root (Option B: guaranteed base layer).
/// Tries Resources/default scenery/default apt dat first, then Global Scenery/Global Airports.
/// Returns combined, deduplicated list (by ICAO id). Used by flight gen when INI packs lack coverage.
pub fn load_base_airports(xplane_root: &Path) -> Vec<Airport> {
    let mut all = Vec::new();

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
            all.extend(airports);
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
                if !all.iter().any(|x| x.id.eq_ignore_ascii_case(&a.id)) {
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

#[derive(Debug, Clone)]
pub struct FlightPlan {
    pub origin: Airport,
    pub destination: Airport,
    pub aircraft: DiscoveredAddon,
    pub distance_nm: u32,
    pub duration_minutes: u32,
    pub route_description: String,
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

fn estimate_runway_reqs(a: &DiscoveredAddon, prompt: &FlightPrompt) -> (u32, SurfaceType) {
    // Keyword Override: Surface
    let forced_surface = match prompt.keywords.surface {
        Some(SurfaceKeyword::Soft) => Some(SurfaceType::Soft),
        Some(SurfaceKeyword::Hard) => Some(SurfaceType::Hard),
        None => None,
    };

    let tags_joined = a.tags.join(" ").to_lowercase();
    if tags_joined.contains("heavy") || tags_joined.contains("airliner") {
        (1500, forced_surface.unwrap_or(SurfaceType::Hard))
    } else if tags_joined.contains("jet") {
        // Relaxed for "Bush" jets (rare but possible in sims)
        if let Some(TypeKeyword::Bush) = prompt.keywords.flight_type {
            (600, forced_surface.unwrap_or(SurfaceType::Hard))
        } else {
            (800, forced_surface.unwrap_or(SurfaceType::Hard))
        }
    } else if tags_joined.contains("seaplane") || tags_joined.contains("amphibian") {
        (0, SurfaceType::Water)
    } else if tags_joined.contains("helicopter") {
        (0, SurfaceType::Soft)
    } else {
        // GA / Bush
        if let Some(TypeKeyword::Bush) = prompt.keywords.flight_type {
            (300, forced_surface.unwrap_or(SurfaceType::Soft))
        } else {
            (500, forced_surface.unwrap_or(SurfaceType::Soft))
        }
    }
}

pub fn generate_flight(
    packs: &[SceneryPack],
    aircraft_list: &[DiscoveredAddon],
    prompt_str: &str,
    base_airports: Option<&[Airport]>,
) -> Result<FlightPlan, String> {
    let prompt = FlightPrompt::parse(prompt_str);
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

    // Determine Aircraft Capabilities with Keyword Overrides
    let speed_kts = estimate_speed(selected_aircraft, &prompt);
    let (min_rwy, req_surface) = estimate_runway_reqs(selected_aircraft, &prompt);

    // 2. Select Origin
    let is_b314 = selected_aircraft.name.contains("Boeing 314");

    // Combined pool: pack airports + base layer (Option B). For B314, exclude base and non-Sealanes packs.
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
    let all_airports: Vec<&Airport> = pack_iter
        .flat_map(|p| p.airports.iter())
        .chain(base_slice.iter())
        .collect();
    log::debug!("[flight_gen] airport pool: {} from packs + base", all_airports.len());

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

                    if let Some(r) = region_obj {
                        if let (Some(lat), Some(lon)) = (apt.lat, apt.lon) {
                            if !r.contains(lat, lon) {
                                return false;
                            }
                        }
                    }
                    if !prompt.ignore_guardrails {
                        if !check_safety_constraints(apt, selected_aircraft, min_rwy, req_surface) {
                            return false;
                        }
                    }
                    apt.lat.is_some() && apt.lon.is_some()
                })
                .map(|a| *a)
                .collect()
        }
        Some(LocationConstraint::AirportName(ref name)) => {
            score_airports_by_name(
                &all_airports,
                name,
                prompt.ignore_guardrails,
                selected_aircraft,
                min_rwy,
                req_surface,
            )
        }
        Some(LocationConstraint::ICAO(ref code)) => all_airports
            .iter()
            .filter(|a| a.id.eq_ignore_ascii_case(code))
            .copied()
            .collect(),
        _ => {
            // Wildcard origin: Filter by constraints
            all_airports
                .iter()
                .filter(|a| {
                    if !prompt.ignore_guardrails {
                        check_safety_constraints(a, selected_aircraft, min_rwy, req_surface)
                    } else {
                        true
                    }
                })
                .copied()
                .collect()
        }
    };

    // Fallback: use embedded seed airports when no pack has data for this region
    #[allow(unused_assignments)]
    let mut seed_origin_fallback: Vec<Airport> = Vec::new();
    if candidate_origins.is_empty() {
        if let Some(LocationConstraint::Region(ref r)) = &prompt.origin {
            seed_origin_fallback = get_seed_airports_for_region(r);
            if !seed_origin_fallback.is_empty() {
                candidate_origins = seed_origin_fallback.iter().collect();
            }
        }
    }

    if candidate_origins.is_empty() {
        log::debug!("[flight_gen] No departure candidates (origin={:?})", prompt.origin);
        return Err("No suitable departure airport found.".to_string());
    }

    candidate_origins.shuffle(&mut rng);
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
        } else {
            // Fallback to Aircraft Defaults
            if prompt.ignore_guardrails {
                (0.0, 20000.0)
            } else if is_glider(selected_aircraft) {
                (5.0, 60.0)
            } else if is_heavy(selected_aircraft) {
                (200.0, 8000.0)
            } else if is_jet(selected_aircraft) {
                // FIXED: Lower minimum to 50nm but allow override if endpoints explicit
                (50.0, 3000.0)
            } else if is_heli(selected_aircraft) {
                (5.0, 200.0)
            } else {
                (30.0, 500.0)
            }
        };

        // Explicit Endpoint Check (Fix for "London to London")
        let endpoints_explicit = matches!(
            (&prompt.origin, &prompt.destination),
            (
                Some(LocationConstraint::AirportName(_)),
                Some(LocationConstraint::AirportName(_))
            ) | (
                Some(LocationConstraint::ICAO(_)),
                Some(LocationConstraint::ICAO(_))
            ) | (
                Some(LocationConstraint::ICAO(_)),
                Some(LocationConstraint::AirportName(_))
            ) | (
                Some(LocationConstraint::AirportName(_)),
                Some(LocationConstraint::ICAO(_))
            )
        );

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
                        if let Some(r) = region_obj {
                            if let (Some(lat), Some(lon)) = (apt.lat, apt.lon) {
                                if !r.contains(lat, lon) {
                                    return false;
                                }
                            }
                        }
                        if !prompt.ignore_guardrails {
                            if !check_safety_constraints(
                                apt,
                                selected_aircraft,
                                min_rwy,
                                req_surface,
                            ) {
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
                name,
                prompt.ignore_guardrails,
                selected_aircraft,
                min_rwy,
                req_surface,
            ),
            Some(LocationConstraint::ICAO(ref code)) => all_airports
                .iter()
                .filter(|a| a.id.eq_ignore_ascii_case(code))
                .copied()
                .collect(),
            _ => all_airports.clone(),
        };

        // Fallback: use embedded seed airports when no pack has dests for this region
        let mut candidate_dests = candidate_dests;
        if candidate_dests.is_empty() {
            if let Some(LocationConstraint::Region(ref r)) = &prompt.destination {
                seed_dest_fallback = get_seed_airports_for_region(r);
                if !seed_dest_fallback.is_empty() {
                    candidate_dests = seed_dest_fallback.iter().collect();
                }
            }
        }

        let valid_dests: Vec<&Airport> = candidate_dests
            .into_iter()
            .filter(|dest| {
                if dest.id == origin.id {
                    return false;
                }
                // Safety Check
                if !prompt.ignore_guardrails {
                    // Check safety but relax if keyword override present (e.g. Bush)
                    // Or if destination is explicit, we might assume user knows best?
                    // For now, respect runway limits unless Ignore Guardrails
                    if !check_safety_constraints(dest, selected_aircraft, min_rwy, req_surface) {
                        return false;
                    }
                }

                if let (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) =
                    (origin.lat, origin.lon, dest.lat, dest.lon)
                {
                    let dist = haversine_nm(lat1, lon1, lat2, lon2);
                    if endpoints_explicit {
                        // Allow very short flights if explicit
                        dist > 2.0 && dist <= 20000.0
                    } else {
                        dist >= min_dist && dist <= max_dist
                    }
                } else {
                    false
                }
            })
            .collect();

        if !valid_dests.is_empty() {
            let destination = *valid_dests.choose(&mut rng).unwrap();
            let dist = haversine_nm(
                origin.lat.unwrap(),
                origin.lon.unwrap(),
                destination.lat.unwrap(),
                destination.lon.unwrap(),
            );
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
            });
        }
    }
    log::debug!(
        "[flight_gen] No destination found after {} origin attempts",
        max_attempts
    );
    log::debug!(
        "[flight_gen] No destination found after {} origin attempts",
        max_attempts
    );
    Err("No suitable destination found.".to_string())
}

// Re-usable constraint checker
fn check_safety_constraints(
    apt: &Airport,
    aircraft: &DiscoveredAddon,
    min_rwy: u32,
    req_surf: SurfaceType,
) -> bool {
    let is_heli = is_heli(aircraft);
    let is_seaplane = is_seaplane(aircraft);

    match apt.airport_type {
        AirportType::Heliport => {
            if !is_heli {
                return false;
            }
        }
        AirportType::Seaplane => {
            if !is_seaplane {
                return false;
            }
        }
        AirportType::Land => {
            if is_seaplane && apt.surface_type != Some(SurfaceType::Water) {
                return false;
            }
        }
    }

    if let Some(surf) = apt.surface_type {
        if req_surf == SurfaceType::Water && surf != SurfaceType::Water {
            return false;
        }
        if req_surf == SurfaceType::Hard && surf != SurfaceType::Hard {
            return false;
        }
    }

    if let Some(len) = apt.max_runway_length {
        if (len as u32) < min_rwy {
            return false;
        }
    } else if min_rwy > 500 {
        return false;
    }
    true
}

fn score_airports_by_name<'a>(
    airports: &[&'a Airport],
    search_str: &str,
    ignore_guardrails: bool,
    selected_aircraft: &DiscoveredAddon,
    min_rwy: u32,
    req_surface: SurfaceType,
) -> Vec<&'a Airport> {
    let search_lower = search_str.to_lowercase();
    let mut scored: Vec<(i32, &'a Airport)> = airports
        .iter()
        .copied()
        .filter(|apt| {
            if !ignore_guardrails {
                check_safety_constraints(apt, selected_aircraft, min_rwy, req_surface)
            } else {
                true
            }
        })
        .map(|apt| {
            let name_lower = apt.name.to_lowercase();
            let id_lower = apt.id.to_lowercase();
            let mut score = 0;

            if id_lower == search_lower {
                score += 1000;
            } else if id_lower.contains(&search_lower) {
                score += 500;
            }

            if name_lower == search_lower {
                score += 800;
            } else if name_lower.contains(&search_lower) {
                score += 300;
            }

            // Accuracy Boost: If search_str contains a region token (e.g. "Paris FR" or "London UK")
            // check if the airport's ICAO matches that region's prefix.
            for token in search_lower.split_whitespace() {
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
    use super::*;

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

fn is_jet(a: &DiscoveredAddon) -> bool {
    a.tags.iter().any(|t| t.to_lowercase().contains("jet"))
}

fn is_heavy(a: &DiscoveredAddon) -> bool {
    a.tags
        .iter()
        .any(|t| t.to_lowercase().contains("heavy") || t.to_lowercase().contains("airliner"))
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

fn is_glider(a: &DiscoveredAddon) -> bool {
    a.tags.iter().any(|t| t.to_lowercase().contains("glider"))
        || a.name.to_lowercase().contains("glider")
        || a.name.to_lowercase().contains("ask 21")
        || a.name.to_lowercase().contains("ask21")
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
        // Middle East & Africa
        "IL" => Some(vec!["LL"]),
        "EG" => Some(vec!["HE"]),
        "ZA" => Some(vec!["FA"]),
        "KE" => Some(vec!["HK"]),
        "UAE" => Some(vec!["OM"]),
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
        "UK" | "BI" => vec![
            seed_airport("EGLL", "London Heathrow", 51.4700, -0.4543),
            seed_airport("EGKK", "London Gatwick", 51.1481, -0.1903),
            seed_airport("EGCC", "Manchester", 53.3537, -2.2750),
            seed_airport("EGGW", "London Luton", 51.8747, -0.3683),
            seed_airport("EGSS", "London Stansted", 51.8849, 0.2346),
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
        "US" => vec![
            seed_airport("KJFK", "New York JFK", 40.6398, -73.7789),
            seed_airport("KLAX", "Los Angeles", 33.9425, -118.4081),
            seed_airport("KORD", "Chicago O'Hare", 41.9786, -87.9047),
            seed_airport("KATL", "Atlanta", 33.6367, -84.4281),
        ],
        "MX" => vec![
            seed_airport("MMMX", "Mexico City", 19.4363, -99.0721),
            seed_airport("MMUN", "Cancun", 21.0365, -86.8770),
            seed_airport("MMMD", "Monterrey", 25.7785, -100.1070),
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
        ],
        "IE" => vec![
            seed_airport("EIDW", "Dublin", 53.4263, -6.2499),
            seed_airport("EICK", "Cork", 51.8413, -8.4911),
        ],
        "BE" => vec![seed_airport("EBBR", "Brussels", 50.9014, 4.4844)],
        "NL" => vec![seed_airport("EHAM", "Amsterdam Schiphol", 52.3086, 4.7639)],
        "CH" => vec![seed_airport("LSZH", "Zurich", 47.4647, 8.5492)],
        "AT" => vec![seed_airport("LOWW", "Vienna", 48.1103, 16.5697)],
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

pub fn export_simbrief(plan: &FlightPlan) -> String {
    format!(
        "https://www.simbrief.com/system/dispatch.php?dep={}&dest={}&type={}",
        plan.origin.id,
        plan.destination.id,
        plan.aircraft
            .tags
            .iter()
            .find(|t| t.len() == 4)
            .unwrap_or(&"C172".to_string()) // Try to find ICAO tag logic? or just C172 default
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
    fn test_jet_runway_estimates() {
        let aircraft = make_addon("Learjet 35", vec!["General Aviation", "Jet"]);
        let prompt = FlightPrompt::default();
        let speed = estimate_speed(&aircraft, &prompt);
        assert_eq!(speed, 350, "Light jets should have 350kts speed");

        let (rwy, surface) = estimate_runway_reqs(&aircraft, &prompt);
        assert_eq!(rwy, 800, "Light jets should need 800m runway");
        assert_eq!(surface, SurfaceType::Hard);
    }

    #[test]
    fn test_bush_keyword_override() {
        let aircraft = make_addon("Cessna 208", vec!["General Aviation", "Turboprop"]);
        let mut prompt = FlightPrompt::default();
        prompt.keywords.flight_type = Some(TypeKeyword::Bush);

        let speed = estimate_speed(&aircraft, &prompt);
        // Bush override makes it slower
        assert_eq!(speed, 100);

        let (rwy, surface) = estimate_runway_reqs(&aircraft, &prompt);
        // Bush override lowers runway requirement
        assert_eq!(rwy, 300);
        assert_eq!(surface, SurfaceType::Soft);
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
        let result = generate_flight(&[pack], &[aircraft], prompt, None);

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
}
