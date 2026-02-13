use crate::apt_dat::{Airport, SurfaceType};
use crate::discovery::{AddonType, DiscoveredAddon};
use rand::seq::SliceRandom;
use x_adox_bitnet::flight_prompt::{AircraftConstraint, FlightPrompt, LocationConstraint};

/// A lat/lon bounding box for geographic region matching.
#[derive(Debug, Clone, Copy)]
struct GeoBounds {
    lat_min: f64,
    lat_max: f64,
    lon_min: f64,
    lon_max: f64,
}

impl GeoBounds {
    fn contains(&self, lat: f64, lon: f64) -> bool {
        lat >= self.lat_min && lat <= self.lat_max && lon >= self.lon_min && lon <= self.lon_max
    }
}

/// Maps a canonical region name (from `try_as_region()`) to lat/lon bounding boxes.
/// Returns multiple boxes where needed (e.g., US = CONUS + Alaska + Hawaii).
fn region_bounds(name: &str) -> Option<Vec<GeoBounds>> {
    let b = |lat_min, lat_max, lon_min, lon_max| GeoBounds {
        lat_min,
        lat_max,
        lon_min,
        lon_max,
    };

    let boxes = match name {
        // Countries (approximate bounding boxes)
        "France" => vec![b(41.0, 51.5, -5.5, 8.3)],
        "Germany" => vec![b(47.0, 55.5, 5.5, 15.5)],
        "Spain" => vec![b(35.5, 44.0, -10.0, 4.5)],
        "Italy" => vec![b(35.5, 47.5, 6.5, 19.0)],
        "Japan" => vec![b(24.0, 46.0, 122.0, 154.0)],
        "Australia" => vec![b(-45.0, -10.0, 112.0, 155.0)],
        "Canada" => vec![b(41.0, 84.0, -141.0, -52.0)],
        "Brazil" => vec![b(-34.0, 6.0, -74.0, -34.0)],
        "India" => vec![b(6.0, 36.0, 68.0, 98.0)],
        "China" => vec![b(18.0, 54.0, 73.0, 135.0)],
        "Mexico" => vec![b(14.0, 33.0, -118.0, -86.0)],
        "Ireland" => vec![b(51.0, 55.5, -11.0, -5.5)],
        "Scotland" => vec![b(54.5, 61.0, -8.0, -0.5)],
        "England" => vec![b(49.5, 56.0, -6.5, 2.0)],
        "Wales" => vec![b(51.3, 53.5, -5.5, -2.5)],
        "Greece" => vec![b(34.5, 42.0, 19.0, 30.0)],
        "Turkey" => vec![b(35.5, 42.5, 25.5, 45.0)],
        "Thailand" => vec![b(5.5, 21.0, 97.0, 106.0)],
        "Portugal" => vec![b(36.5, 42.5, -10.0, -6.0)],
        "Netherlands" => vec![b(50.5, 54.0, 3.0, 7.5)],
        "Sweden" => vec![b(55.0, 69.5, 10.5, 24.5)],
        "Norway" => vec![b(57.5, 71.5, 4.0, 31.5)],
        "Finland" => vec![b(59.5, 70.5, 19.0, 32.0)],
        "Denmark" => vec![b(54.5, 58.0, 7.5, 15.5)],
        "Iceland" => vec![b(63.0, 67.0, -25.0, -13.0)],
        "Switzerland" => vec![b(45.5, 48.0, 5.5, 10.5)],
        "Austria" => vec![b(46.0, 49.5, 9.0, 17.5)],
        "Poland" => vec![b(49.0, 55.0, 14.0, 24.5)],
        "Belgium" => vec![b(49.5, 51.5, 2.5, 6.5)],
        "Czech Republic" => vec![b(48.5, 51.5, 12.0, 19.0)],
        "Romania" => vec![b(43.5, 48.5, 20.0, 30.5)],
        "Hungary" => vec![b(45.5, 49.0, 16.0, 23.0)],
        "Croatia" => vec![b(42.0, 46.5, 13.0, 19.5)],
        "Serbia" => vec![b(42.0, 46.5, 18.5, 23.0)],
        "Bulgaria" => vec![b(41.0, 44.5, 22.0, 29.0)],
        "South Korea" => vec![b(33.0, 39.0, 124.0, 132.0)],
        "Taiwan" => vec![b(21.5, 25.5, 119.5, 122.5)],
        "Philippines" => vec![b(4.5, 21.0, 116.0, 127.0)],
        "Indonesia" => vec![b(-11.0, 6.0, 95.0, 141.0)],
        "Malaysia" => vec![b(0.5, 7.5, 99.0, 119.5)],
        "Vietnam" => vec![b(8.0, 23.5, 102.0, 110.0)],
        "Singapore" => vec![b(1.1, 1.5, 103.5, 104.1)],
        "South Africa" => vec![b(-35.0, -22.0, 16.0, 33.0)],
        "Egypt" => vec![b(22.0, 32.0, 24.5, 37.0)],
        "Morocco" => vec![b(27.5, 36.0, -13.5, -1.0)],
        "Kenya" => vec![b(-5.0, 5.5, 33.5, 42.0)],
        "Nigeria" => vec![b(4.0, 14.0, 2.5, 15.0)],
        "Colombia" => vec![b(-4.5, 13.5, -79.0, -66.5)],
        "Argentina" => vec![b(-55.0, -21.5, -74.0, -53.5)],
        "Chile" => vec![b(-56.0, -17.5, -76.0, -66.5)],
        "Russia" => vec![b(41.0, 82.0, 19.0, 180.0)],
        "Ukraine" => vec![b(44.0, 53.0, 22.0, 41.0)],
        "Pakistan" => vec![b(23.5, 37.5, 60.5, 77.5)],

        // Abbreviation targets
        "United Kingdom" => vec![b(49.5, 61.0, -8.5, 2.0)],
        // Two boxes: mainland excludes NI longitude; Scottish islands box starts north of Belfast
        "Great Britain" => vec![b(49.5, 61.0, -5.8, 2.0), b(55.0, 61.0, -8.5, -5.8)],
        "United States" => vec![b(24.0, 50.0, -125.0, -66.0), b(51.0, 72.0, -170.0, -130.0)],
        "United Arab Emirates" => vec![b(22.5, 26.5, 51.0, 56.5)],
        "New Zealand" => vec![b(-47.5, -34.0, 165.5, 179.0)],

        // Geographic groups
        "British Isles" => vec![b(49.5, 61.0, -11.0, 2.0)],
        "Scandinavia" => vec![b(54.5, 71.5, 4.0, 32.0)],
        "Caribbean" => vec![b(10.0, 27.0, -85.0, -59.0)],
        "Mediterranean" => vec![b(30.0, 46.0, -6.0, 37.0)],
        "Benelux" => vec![b(49.5, 54.0, 2.5, 7.5)],
        "Southeast Asia" => vec![b(-11.0, 24.0, 92.0, 141.0)],
        "Middle East" => vec![b(12.0, 42.0, 24.0, 63.0)],
        "Central America" => vec![b(7.0, 18.5, -92.0, -77.0)],
        "Balkans" => vec![b(39.0, 47.0, 13.0, 30.0)],

        // US regions
        "US:SoCal" => vec![b(32.0, 35.5, -121.0, -114.5)],
        "US:NorCal" => vec![b(35.5, 42.5, -125.0, -119.5)],
        "US:PNW" => vec![b(42.0, 49.0, -125.0, -116.5)],
        "US:Northeast" => vec![b(40.0, 47.5, -80.0, -66.5)],
        "US:Midwest" => vec![b(36.0, 49.0, -104.0, -80.0)],
        "US:Southeast" => vec![b(24.5, 37.0, -92.0, -75.0)],
        "US:Texas" => vec![b(25.5, 36.5, -107.0, -93.0)],
        "US:Florida" => vec![b(24.5, 31.0, -88.0, -79.5)],
        "US:Hawaii" => vec![b(18.5, 22.5, -161.0, -154.5)],
        "US:Alaska" => vec![b(51.0, 72.0, -170.0, -130.0)],

        // Continents
        "Europe" => vec![b(34.0, 72.0, -25.0, 45.0)],
        "Africa" => vec![b(-35.0, 38.0, -18.0, 52.0)],
        "Asia" => vec![b(-11.0, 82.0, 25.0, 180.0)],
        "North America" => vec![b(7.0, 84.0, -170.0, -52.0)],
        "South America" => vec![b(-56.0, 13.5, -82.0, -34.0)],
        "Oceania" => vec![b(-47.5, 0.0, 110.0, 180.0)],

        _ => return None,
    };
    Some(boxes)
}

/// Check if an airport's coordinates fall within any of the given bounding boxes.
fn airport_in_bounds(airport: &Airport, bounds: &[GeoBounds]) -> bool {
    if let (Some(lat), Some(lon)) = (airport.lat, airport.lon) {
        bounds.iter().any(|b| b.contains(lat, lon))
    } else {
        false
    }
}

/// Check if a pack has any airport within the given bounding boxes,
/// falling back to text matching on pack name/region if no airports have coordinates.
fn pack_matches_region(pack: &SceneryPack, region: &str, bounds: &[GeoBounds]) -> bool {
    // First try coordinate-based matching
    let has_coords = pack
        .airports
        .iter()
        .any(|a| a.lat.is_some() && a.lon.is_some());
    if has_coords {
        return pack.airports.iter().any(|a| airport_in_bounds(a, bounds));
    }
    // Fallback: text match on pack name/region
    let region_lower = region.to_lowercase();
    let pack_region = pack.get_region().to_lowercase();
    let pack_name = pack.name.to_lowercase();
    pack_region.contains(&region_lower) || pack_name.contains(&region_lower)
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

pub fn generate_flight(
    packs: &[SceneryPack],
    aircraft_list: &[DiscoveredAddon],
    prompt_str: &str,
) -> Result<FlightPlan, String> {
    let prompt = FlightPrompt::parse(prompt_str);
    let mut rng = rand::thread_rng();

    // 1. Select Aircraft
    let suitable_aircraft: Vec<&DiscoveredAddon> = aircraft_list
        .iter()
        .filter(|a| {
            if let AddonType::Aircraft { .. } = a.addon_type {
                if let Some(AircraftConstraint::Tag(ref tag)) = prompt.aircraft {
                    // Fuzzy tag match
                    let tag_lower = tag.to_lowercase();
                    let matches = a.tags.iter().any(|t| t.to_lowercase().contains(&tag_lower))
                        || a.name.to_lowercase().contains(&tag_lower);
                    if !matches {}
                    matches
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

    // Determine Aircraft Capabilities (Heuristics)
    let speed_kts = estimate_speed(selected_aircraft);
    let (min_rwy, req_surface) = estimate_runway_reqs(selected_aircraft);

    // 2. Select Origin
    // Collect all valid airports from scenery
    // Flattening all packs
    let all_airports: Vec<&Airport> = packs.iter().flat_map(|p| p.airports.iter()).collect();

    // Refined Origin Selection (Group by Pack for Region check)
    let candidate_origins: Vec<&Airport> =
        if let Some(LocationConstraint::Region(ref region)) = prompt.origin {
            // Use bounding box matching if available, with text fallback
            let bounds = region_bounds(region);
            packs
                .iter()
                .filter(|p| {
                    if let Some(ref bb) = bounds {
                        pack_matches_region(p, region, bb)
                    } else {
                        // No bounding box data — fall back to text matching
                        let region_lower = region.to_lowercase();
                        p.get_region().to_lowercase().contains(&region_lower)
                            || p.name.to_lowercase().contains(&region_lower)
                    }
                })
                .flat_map(|p| p.airports.iter())
                .filter(|apt| {
                    // If we have bounds, also filter individual airports by coordinates
                    if let Some(ref bb) = bounds {
                        if !airport_in_bounds(apt, bb) {
                            return false;
                        }
                    }
                    // Guardrails
                    if !prompt.ignore_guardrails {
                        if let Some(surf) = apt.surface_type {
                            if req_surface == SurfaceType::Water && surf != SurfaceType::Water {
                                return false;
                            }
                            if req_surface == SurfaceType::Hard && surf != SurfaceType::Hard {
                                return false;
                            }
                        }
                        if let Some(len) = apt.max_runway_length {
                            if (len as u32) < min_rwy {
                                return false;
                            }
                        }
                    }
                    apt.lat.is_some() && apt.lon.is_some()
                })
                .collect()
        } else if let Some(LocationConstraint::AirportName(ref name)) = prompt.origin {
            let name_lower = name.to_lowercase();
            // Match Pack Region OR Pack Name OR Airport Name OR Airport ID
            // Scoring System:
            // 100 = Exact ID Match
            // 90 = Exact Name Match
            // 80 = Starts With Name
            // 60 = Contains Name
            // 50 = Token Match (All words present in Airport Name OR Pack Name/Region)
            // 40 = Region Match
            // 0 = No Match
            let mut candidates: Vec<(&Airport, u8)> = packs
                .iter()
                .flat_map(|p| {
                    let pack_region = p.get_region().to_lowercase();
                    let pack_name = p.name.to_lowercase();
                    let pack_matches =
                        pack_region.contains(&name_lower) || pack_name.contains(&name_lower);
                    let base_score = if pack_matches { 40 } else { 0 };
                    p.airports
                        .iter()
                        .map(move |a| (a, base_score, pack_name.clone(), pack_region.clone()))
                })
                .map(|(a, base_score, p_name, p_region)| {
                    let a_name = a.name.to_lowercase();
                    let a_id = a.id.to_lowercase();

                    let score = if a_id == name_lower {
                        100
                    } else if a_name == name_lower {
                        90
                    } else if a_name.starts_with(&name_lower) {
                        80
                    } else if a_name.contains(&name_lower) {
                        60
                    } else if name_lower.contains("british isles")
                        && is_british_isles_region(&p_region)
                    {
                        50
                    } else {
                        // Token match check
                        let tokens: Vec<&str> = name_lower.split_whitespace().collect();
                        if !tokens.is_empty()
                            && tokens.iter().all(|t| {
                                a_name.contains(t)
                                    || p_contains_token(t, &p_name)
                                    || p_contains_token(t, &p_region)
                            })
                        {
                            50
                        } else {
                            base_score
                        }
                    };
                    (a, score)
                })
                .filter(|(a, score)| {
                    *score > 0 && {
                        // Guardrails
                        if !prompt.ignore_guardrails {
                            if let Some(surf) = a.surface_type {
                                if req_surface == SurfaceType::Water && surf != SurfaceType::Water {
                                    return false;
                                }
                                if req_surface == SurfaceType::Hard && surf != SurfaceType::Hard {
                                    return false;
                                }
                            }
                            if let Some(len) = a.max_runway_length {
                                if (len as u32) < min_rwy {
                                    return false;
                                }
                            }
                        }
                        a.lat.is_some() && a.lon.is_some()
                    }
                })
                .collect();

            // Sort by score descending
            candidates.sort_by(|a, b| b.1.cmp(&a.1));

            // Tiered Selection: Pick only from the highest score group
            // This ensures we don't pick "Groton New London" (60) when "London Heathrow" (80) is available.
            let top_count = (candidates.len() / 4).max(5).min(candidates.len());
            if let Some(first) = candidates.first() {
                let max_score = first.1;
                candidates
                    .into_iter()
                    .take_while(|(_, score)| *score == max_score)
                    .map(|(a, _)| a)
                    .take(top_count)
                    .collect()
            } else {
                vec![]
            }
        } else if let Some(LocationConstraint::ICAO(ref code)) = prompt.origin {
            all_airports
                .iter()
                .filter(|a| a.id == *code)
                .copied()
                .collect()
        } else {
            // Any, matching guardrails
            all_airports
                .iter()
                .filter(|img| {
                    if !prompt.ignore_guardrails {
                        if let Some(surf) = img.surface_type {
                            if req_surface == SurfaceType::Water && surf != SurfaceType::Water {
                                return false;
                            }
                            if req_surface == SurfaceType::Hard && surf != SurfaceType::Hard {
                                return false;
                            }
                        }
                        if let Some(len) = img.max_runway_length {
                            if (len as u32) < min_rwy {
                                return false;
                            }
                        }
                    }
                    img.lat.is_some() && img.lon.is_some()
                })
                .copied()
                .collect()
        };

    if candidate_origins.is_empty() {
        log::warn!("No suitable departure airport found for prompt: '{}'. Filters: Region/Name match failed.", prompt_str);
        return Err("No suitable departure airport found.".to_string());
    }

    let origin = *candidate_origins.choose(&mut rng).unwrap();

    // 3. Select Destination
    // Calculate Target Distance
    // When both endpoints are explicit (ICAO or AirportName), the user specified
    // exactly where they want to fly — don't filter by aircraft-type distance defaults.
    let both_explicit = matches!(
        (&prompt.origin, &prompt.destination),
        (
            Some(LocationConstraint::ICAO(_)) | Some(LocationConstraint::AirportName(_)),
            Some(LocationConstraint::ICAO(_)) | Some(LocationConstraint::AirportName(_))
        )
    );

    let (min_dist, max_dist) = if let Some(mins) = prompt.duration_minutes {
        let dist = speed_kts as f64 * (mins as f64 / 60.0);
        (dist * 0.8, dist * 1.2)
    } else if prompt.ignore_guardrails || both_explicit {
        (0.0, 20000.0)
    } else {
        // Default range based on aircraft type
        if is_heavy(selected_aircraft) {
            (200.0, 8000.0)
        } else if is_heli(selected_aircraft) {
            (5.0, 200.0)
        } else {
            (30.0, 500.0)
        }
    };

    let candidate_dests = if let Some(LocationConstraint::Region(ref region)) = prompt.destination {
        let bounds = region_bounds(region);
        packs
            .iter()
            .filter(|p| {
                if let Some(ref bb) = bounds {
                    pack_matches_region(p, region, bb)
                } else {
                    let region_lower = region.to_lowercase();
                    p.get_region().to_lowercase().contains(&region_lower)
                        || p.name.to_lowercase().contains(&region_lower)
                }
            })
            .flat_map(|p| p.airports.iter())
            .filter(|apt| {
                if let Some(ref bb) = bounds {
                    airport_in_bounds(apt, bb)
                } else {
                    true
                }
            })
            .collect::<Vec<&Airport>>()
    } else if let Some(LocationConstraint::AirportName(ref name)) = prompt.destination {
        let name_lower = name.to_lowercase();
        let mut candidates: Vec<(&Airport, u8)> = packs
            .iter()
            .flat_map(|p| {
                let pack_region = p.get_region().to_lowercase();
                let pack_name = p.name.to_lowercase();
                let pack_matches =
                    pack_region.contains(&name_lower) || pack_name.contains(&name_lower);
                let base_score = if pack_matches { 40 } else { 0 };
                p.airports
                    .iter()
                    .map(move |a| (a, base_score, pack_name.clone(), pack_region.clone()))
            })
            .map(|(a, base_score, p_name, p_region)| {
                let a_name = a.name.to_lowercase();
                let a_id = a.id.to_lowercase();

                let score = if a_id == name_lower {
                    100
                } else if a_name == name_lower {
                    90
                } else if a_name.starts_with(&name_lower) {
                    80
                } else if a_name.contains(&name_lower) {
                    60
                } else if name_lower.contains("british isles")
                    && is_british_isles_region(&p_region)
                {
                    50
                } else {
                    // Token match check
                    let tokens: Vec<&str> = name_lower.split_whitespace().collect();
                    if !tokens.is_empty()
                        && tokens.iter().all(|t| {
                            a_name.contains(t)
                                || p_contains_token(t, &p_name)
                                || p_contains_token(t, &p_region)
                        })
                    {
                        50
                    } else {
                        base_score
                    }
                };
                (a, score)
            })
            .filter(|(_a, score)| *score > 0)
            // Existing logic didn't seem to apply guardrails tightly to destination in the AirportName branch before,
            // but let's stick to simple filtering for now.
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        // Tiered Selection
        if let Some(first) = candidates.first() {
            let max_score = first.1;
            candidates
                .into_iter()
                .take_while(|(_, score)| *score == max_score)
                .map(|(a, _)| a)
                .collect()
        } else {
            vec![]
        }
    } else if let Some(LocationConstraint::ICAO(ref code)) = prompt.destination {
        packs
            .iter()
            .flat_map(|p| p.airports.iter())
            .filter(|a| a.id.eq_ignore_ascii_case(code))
            .collect()
    } else {
        all_airports.clone()
    };

    let valid_dests: Vec<&Airport> = candidate_dests
        .into_iter()
        .filter(|dest| {
            if dest.id == origin.id {
                return false;
            }

            // Guardrails
            if !prompt.ignore_guardrails {
                if let Some(surf) = dest.surface_type {
                    if req_surface == SurfaceType::Water && surf != SurfaceType::Water {
                        return false;
                    }
                    if req_surface == SurfaceType::Hard && surf != SurfaceType::Hard {
                        return false;
                    }
                }
                if let Some(len) = dest.max_runway_length {
                    if (len as u32) < min_rwy {
                        return false;
                    }
                }
            }

            if let (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) =
                (origin.lat, origin.lon, dest.lat, dest.lon)
            {
                let dist = haversine_nm(lat1, lon1, lat2, lon2);
                if dist < min_dist as f64 || dist > max_dist as f64 {
                    // println!(
                    //     "DEBUG: Distance check failed for {}: dist={} min={} max={}",
                    //     dest.id, dist, min_dist, max_dist
                    // );
                } else {
                    // println!(
                    //     "DEBUG: Distance check passed for {}: dist={}",
                    //     dest.id, dist
                    // );
                }
                dist >= min_dist as f64 && dist <= max_dist as f64
            } else {
                false
            }
        })
        .collect();

    if valid_dests.is_empty() {
        log::warn!(
            "No suitable destination found for origin {} within range {}-{}. Prompt: '{}'.",
            origin.id,
            min_dist,
            max_dist,
            prompt_str
        );
        return Err("No suitable destination found within range.".to_string());
    }

    let destination = *valid_dests.choose(&mut rng).unwrap();
    let dist = haversine_nm(
        origin.lat.unwrap(),
        origin.lon.unwrap(),
        destination.lat.unwrap(),
        destination.lon.unwrap(),
    );

    Ok(FlightPlan {
        origin: origin.clone(),
        destination: destination.clone(),
        aircraft: selected_aircraft.clone(),
        distance_nm: dist as u32,
        duration_minutes: (dist / (speed_kts as f64) * 60.0) as u32,
        route_description: if prompt.ignore_guardrails {
            "(Guardrails Ignored)".to_string()
        } else {
            "generated".to_string()
        },
    })
}

// Helpers

fn estimate_speed(a: &DiscoveredAddon) -> u32 {
    let tags_joined = a.tags.join(" ").to_lowercase();
    if tags_joined.contains("jet") || tags_joined.contains("airliner") {
        450
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

fn estimate_runway_reqs(a: &DiscoveredAddon) -> (u32, SurfaceType) {
    let tags_joined = a.tags.join(" ").to_lowercase();
    if tags_joined.contains("heavy") || tags_joined.contains("airliner") {
        (1500, SurfaceType::Hard)
    } else if tags_joined.contains("jet") {
        (1000, SurfaceType::Hard)
    } else if tags_joined.contains("seaplane") || tags_joined.contains("amphibian") {
        (0, SurfaceType::Water)
    } else if tags_joined.contains("helicopter") {
        (0, SurfaceType::Soft) // Any
    } else {
        (500, SurfaceType::Soft) // Any
    }
}

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
        "socal" => {
            text.contains("southern california")
                || text.contains("los angeles")
                || text.contains("san diego")
        }
        "norcal" => {
            text.contains("northern california")
                || text.contains("san francisco")
                || text.contains("sacramento")
        }
        "pnw" => {
            text.contains("pacific northwest")
                || text.contains("washington")
                || text.contains("oregon")
                || text.contains("seattle")
        }
        _ => false,
    }
}

/// Returns true if the region string refers to a British Isles sub-region.
fn is_british_isles_region(region: &str) -> bool {
    region.contains("united kingdom")
        || region.contains("great britain")
        || region.contains("ireland")
        || region.contains("northern ireland")
        || region.contains("scotland")
        || region.contains("wales")
        || region.contains("england")
        || region.contains("isle of man")
        || region.contains("hebrides")
        || region.contains("shetland")
        || region.contains("orkney")
        || region.contains("channel islands")
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
