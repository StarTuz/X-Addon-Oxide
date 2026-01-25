use crate::scenery::SceneryCategory;
use std::path::Path;

pub struct Classifier;

impl Classifier {
    // Fast native check
    pub fn classify_heuristic(_path: &Path, name: &str) -> SceneryCategory {
        let name_lower = name.to_lowercase();

        // 1. Mesh/Foundation (Level 11 - Score 30)
        if name_lower.contains("mesh")
            || name_lower.starts_with("zzz")
            || name_lower.contains("uhd")
        {
            return SceneryCategory::Mesh;
        }

        // 2. Libraries (Level 8 - Score 65)
        // Consolidated block: SAM, MisterX, CDB, 3D_people, Birds_Library etc.
        // MUST be checked before brand brand matches.
        if name_lower.contains("library")
            || name_lower.contains("lib")
            || name_lower.contains("zdp")
            || name_lower.contains("3d_people")
            || name_lower.contains("aa_sam")
            || name_lower.contains("sam_library")
            || name_lower.contains("misterx")
            || name_lower.contains("opensceneryx")
            || name_lower.contains("worldjetways")
        {
            if !name_lower.contains("vegetation_library") {
                return SceneryCategory::Library;
            }
        }

        // 3. Global Airports (Level 3 - Score 90)
        if name_lower.contains("global_airports")
            || name_lower.contains("global scenery/global airports")
            || name_lower == "global airports"
            || name_lower.contains("*global_airports*")
        {
            return SceneryCategory::GlobalAirport;
        }

        // 4. Orbx TrueEarth Customs/Airports (Level 2 - Score 95)
        if name_lower.starts_with("orbx_a_") {
            return SceneryCategory::OrbxAirport;
        }

        // 5. City/Landmark Overlays (Level 4 - Score 88)
        if name_lower.contains("x-plane landmarks -") {
            return SceneryCategory::Landmark;
        }

        // 6. Regional Detail Layers (Level 5 - Score 85)
        if name_lower.contains("simheaven_x-world")
            || name_lower.contains("vegetation_library")
            || (name_lower.starts_with("orbx_b_") || name_lower.starts_with("orbx_c_"))
                && name_lower.contains("overlay")
        {
            return SceneryCategory::RegionalOverlay;
        }

        // 7. Regional Fluff (Level 6 - Score 80)
        if name_lower.contains("forests")
            || name_lower.contains("network")
            || name_lower.contains("birds")
            || name_lower.contains("seagulls")
            || name_lower.contains("sealanes")
            || name_lower.contains("global_forests_v2")
        {
            return SceneryCategory::RegionalFluff;
        }

        // 8. Airport-Specific Enhancements & Overlays (Level 7 - Score 75)
        // Grouping Y KTEX Overlay and other specific enhancements here.
        if name_lower.contains("overlay")
            || name_lower.contains("followme")
            || name_lower.contains("groundservice")
            || name_lower.contains("airportvehicles")
            || name_lower.contains("aep")
            || name_lower.contains("static")
        {
            return SceneryCategory::AirportOverlay;
        }

        // 9. AutoOrtho Corrections (Level 8 - Score 70)
        if name_lower.contains("yautoortho_overlays") {
            return SceneryCategory::AutoOrthoOverlay;
        }

        // 10. Custom Airports (Level 1 - Score 100)
        // Add DarkBlue, and verify it's not a generic overlay/library already caught.
        if has_icao_pattern(&name)
            || name_lower.contains("fly2high")
            || name_lower.contains("aerosoft")
            || name_lower.contains("flytampa")
            || name_lower.contains("nimbus")
            || name_lower.contains("justsim")
            || name_lower.contains("skyline")
            || name_lower.contains("boundless")
            || name_lower.contains("axonos")
            || name_lower.contains("taimodels")
            || name_lower.contains("x-scenery")
            || name_lower.contains("darkblue")
            || name_lower.contains("panc---anchorage")
            || name_lower.contains("airport")
            || name_lower.contains("airfield")
            || name_lower.contains("heliport")
        {
            return SceneryCategory::CustomAirport;
        }

        // 11. Global Base Scenery (Level 9 - Score 60)
        // Demo Areas, X-Plane 12 Global Scenery
        if name_lower.contains("demo areas") || name_lower.contains("global scenery") {
            return SceneryCategory::GlobalBase;
        }

        // 12. Ortho/Photo Base (Level 10 - Score 55)
        if (name_lower.contains("orbx_c_") || name_lower.contains("orbx_d_"))
            && name_lower.contains("ortho")
            || name_lower.contains("z_ao_")
            || name_lower.contains("z_autoortho")
            || name_lower.contains("ortho4xp")
            || name_lower.starts_with("z_")
            || name_lower.starts_with("y_")
        {
            return SceneryCategory::OrthoBase;
        }

        SceneryCategory::Unknown
    }
}

// Helper: matches 4-letter ICAO codes (e.g., KLAX, EGLL) roughly
// Must be simplistic to rely on string matching as requested.
fn has_icao_pattern(name: &str) -> bool {
    // Basic heuristics: 4 uppercase letters, possibly surrounded by [_- ]
    use std::sync::OnceLock;
    static RE_ICAO: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE_ICAO.get_or_init(|| regex::Regex::new(r"(^|[^A-Z])[A-Z]{4}([^A-Z]|$)").unwrap());
    re.is_match(name)
}
