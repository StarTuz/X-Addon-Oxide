use crate::apt_dat::{Airport, SurfaceType};
use crate::discovery::{AddonType, DiscoveredAddon};
use rand::seq::SliceRandom;
use x_adox_bitnet::flight_prompt::{AircraftConstraint, FlightPrompt, LocationConstraint};

use x_adox_bitnet::geo::RegionIndex;

/// Check if a pack has any airport within the given region.
/// Uses the global RegionIndex.
fn pack_matches_region(pack: &SceneryPack, region_id: &str, index: &RegionIndex) -> bool {
    let region = match index.get_by_id(region_id) {
        Some(r) => r,
        None => return false,
    };

    // First try coordinate-based matching
    let has_coords = pack
        .airports
        .iter()
        .any(|a| a.lat.is_some() && a.lon.is_some());
    if has_coords {
        return pack.airports.iter().any(|a| {
            if let (Some(lat), Some(lon)) = (a.lat, a.lon) {
                region.contains(lat, lon)
            } else {
                false
            }
        });
    }
    // Fallback: text match on pack name/region matching the Region name
    // (We match against the region's human name or ID)
    let region_name_lower = region.name.to_lowercase();
    let region_id_lower = region.id.to_lowercase();
    let pack_region = pack.get_region().to_lowercase();
    let pack_name = pack.name.to_lowercase();

    pack_region.contains(&region_name_lower)
        || pack_name.contains(&region_name_lower)
        || pack_region.contains(&region_id_lower)
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
    let region_index = RegionIndex::new();

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
    // Identify Boeing 314 specific restriction
    let is_b314 = selected_aircraft.name.contains("Boeing 314");

    // Collect all valid airports from scenery
    // Flattening all packs
    let all_airports: Vec<&Airport> = packs
        .iter()
        .filter(|p| {
            if is_b314
                && (prompt.origin.is_none()
                    || matches!(prompt.origin, Some(LocationConstraint::Any)))
            {
                p.path.to_string_lossy().contains("B314 Sealanes")
            } else {
                true
            }
        })
        .flat_map(|p| p.airports.iter())
        .collect();

    // Refined Origin Selection (Group by Pack for Region check)
    let candidate_origins: Vec<&Airport> = if let Some(LocationConstraint::Region(ref region_id)) =
        prompt.origin
    {
        // Look up region object
        let region_obj = region_index.get_by_id(region_id);

        packs
            .iter()
            .filter(|p| pack_matches_region(p, region_id, &region_index))
            .flat_map(|p| p.airports.iter())
            .filter(|apt| {
                // If we have region bounds, filter individual airports by coordinates
                if let Some(r) = region_obj {
                    if let (Some(lat), Some(lon)) = (apt.lat, apt.lon) {
                        if !r.contains(lat, lon) {
                            return false;
                        }
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
                } else if name_lower.contains("british isles") && is_british_isles_region(&p_region)
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

    let candidate_dests = if let Some(LocationConstraint::Region(ref region_id)) =
        prompt.destination
    {
        let region_obj = region_index.get_by_id(region_id);

        packs
            .iter()
            .filter(|p| pack_matches_region(p, region_id, &region_index))
            .flat_map(|p| p.airports.iter())
            .filter(|apt| {
                if let Some(r) = region_obj {
                    if let (Some(lat), Some(lon)) = (apt.lat, apt.lon) {
                        if !r.contains(lat, lon) {
                            return false;
                        }
                    }
                }
                true
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
                } else if name_lower.contains("british isles") && is_british_isles_region(&p_region)
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

            // Boeing 314 destination restriction: if destination is any/random,
            // ensure it comes from a Sealanes pack.
            if is_b314
                && (prompt.destination.is_none()
                    || matches!(prompt.destination, Some(LocationConstraint::Any)))
            {
                // Find which pack this dest belongs to
                let belongs_to_sealanes = packs.iter().any(|p| {
                    p.path.to_string_lossy().contains("B314 Sealanes")
                        && p.airports.iter().any(|a| a.id == dest.id)
                });
                if !belongs_to_sealanes {
                    return false;
                }
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
