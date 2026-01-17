use crate::scenery::SceneryCategory;
use std::path::Path;

pub struct Classifier;

impl Classifier {
    // Fast native check
    pub fn classify_heuristic(path: &Path, name: &str) -> SceneryCategory {
        // Normalize name
        let name_lower = name.to_lowercase();
        let name_clean = name.trim_matches(|c| c == '*' || c == ' ');

        // 1. Explicit System Packs (Global Airports)
        if name_clean.eq_ignore_ascii_case("global airports")
            || name_clean.eq_ignore_ascii_case("global_airports")
        {
            return SceneryCategory::GlobalAirport;
        }

        // 2. Check for specific files (High signal)
        if path.join("Earth nav data/earth_wed.xml").exists() || path.join("earth.wed.xml").exists()
        {
            return SceneryCategory::EarthAirports;
        }

        let has_library = path.join("library.txt").exists();
        if has_library {
            // Some airports also have library.txt, but usually they also have earth_wed.
            // If NO earth_wed but YES library.txt -> likely a library.
            return SceneryCategory::Library;
        }

        // 3. Naming Conventions (Strong signals)

        // Ortho
        if name_lower.starts_with("zortho")
            || name_lower.starts_with("yortho")
            || name_lower.contains("ortho4xp")
        {
            return SceneryCategory::Ortho;
        }

        // Mesh
        if name_lower.starts_with("zzz")
            || name_lower.contains("mesh")
            || name_lower.contains("uhd")
        {
            return SceneryCategory::Mesh;
        }

        // Libraries
        if name_lower.contains("library")
            || name_lower.contains("opensceneryx")
            || name_lower.contains("lib")
        {
            return SceneryCategory::Library;
        }

        // Overlays / SimHeaven
        if name_lower.contains("simheaven")
            || name_lower.contains("w2xp")
            || name_lower.contains("overlay")
            || name_lower.contains("landmarks")
        {
            return SceneryCategory::Overlay;
        }

        // Airports
        if name_lower.contains("airport")
            || name_lower.contains("apt")
            || name_lower.starts_with("katl")
            || name_lower.starts_with("k") && name_lower.len() == 4
        {
            return SceneryCategory::EarthAirports;
        }

        // Orbx TrueEarth
        if name_lower.contains("orbx") {
            if name_lower.contains("overlay") {
                return SceneryCategory::Overlay;
            }
            if name_lower.contains("ortho") {
                return SceneryCategory::Ortho;
            }
            if name_lower.contains("mesh") {
                return SceneryCategory::Mesh;
            }
            if name_lower.contains("airport") {
                return SceneryCategory::EarthAirports;
            }
            return SceneryCategory::Overlay;
        }

        SceneryCategory::Unknown
    }
}
