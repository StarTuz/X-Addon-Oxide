use anyhow::Result;

#[derive(Default)]
pub struct BitNetModel {
    // In a real implementation, this would hold the weights,
    // but now it holds the distilled logic.
}

impl BitNetModel {
    pub fn new() -> Result<Self> {
        Ok(Self::default())
    }

    /// Predicts the scenery priority score (0-100) based on the pack name and path.
    /// Lower score = higher priority.
    pub fn predict(&self, name: &str, _path: &std::path::Path) -> u8 {
        let name_lower = name.to_lowercase();
        let name_clean = name.trim_matches(|c| c == '*' || c == ' ' || c == '_');

        // Helper boolean flags for exclusion logic
        let has_airport_keyword = name_lower.contains("airport")
            || name_lower.contains("apt")
            || name_lower.contains("airfield")
            || name_lower.contains("heliport")
            || name_lower.contains("seaplane");

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
            || name_lower.contains("orbx");

        // Simple ICAO detector: look for 4-letter uppercase words
        let has_icao = name.split(|c: char| !c.is_alphanumeric()).any(|word| {
            word.len() == 4 && word.chars().all(|c| c.is_alphabetic() && c.is_uppercase())
        });

        let is_custom_airport = has_airport_keyword
            || is_major_dev
            || has_icao
            || name_lower.contains("panc")
            || name_lower.contains("anchorage")
            || (name_lower.contains("scenery_pack") && name_lower.len() < 35);

        // --- Tweak 1: Misplaced overlays/meshes (Score 61 = absolute bottom) ---
        if (name_lower.contains("overlay")
            || name_lower.contains("mesh")
            || name_lower.contains("ktex")
            || name_lower.contains("ortho"))
            && !is_custom_airport
        {
            return 61; // End of Meshes & Orthos section
        }

        // --- 1. MESH (Score 60) ---
        if name_lower.contains("mesh")
            || name_lower.contains("uhd")
            || name_lower.contains("terrain")
            || name_lower.starts_with("zzz")
        {
            return 60;
        }

        // --- 2. ORTHO / PHOTO (Score 50) ---
        if name_lower.contains("ortho")
            || name_lower.contains("photoscenery")
            || name_lower.starts_with("yortho")
        {
            return 50;
        }

        // --- 3. LIBRARIES (Score 45) ---
        if name_lower.contains("library")
            || name_lower.contains("lib")
            || name_lower.contains("opensceneryx")
            || name_lower.contains("r2_library")
            || name_lower.contains("bs2001")
            || name_lower.contains("3dnl")
            || name_lower.contains("misterx")
            || name_lower.contains("zdp")
            || name_lower.contains("sam")
            || name_lower.contains("aep") && name_lower.len() < 10
            || name_lower.contains("assets")
        {
            return 45;
        }

        // --- 4. OVERLAYS & LANDMARKS (Generic) (Score 40) ---
        if name_lower.contains("landmarks")
            || name_lower.contains("global_forests")
            || name_lower.contains("vfr") && !name_lower.contains("simheaven")
            || name_lower.contains("landmark") && !name_lower.contains("x-plane")
        {
            return 40;
        }

        // --- 5. ORBX B OVERLAYS (Score 37) ---
        // Moved higher (right after simHeaven) for better fidelity in GB-heavy setups
        // --- 5. ORBX B OVERLAYS (Score 35) ---
        // Moved higher for better fidelity in GB-heavy setups
        if name_lower.contains("orbx_b") || name_lower.contains("trueearth_overlay") {
            return 35;
        }

        // --- 6. SIMHEAVEN SLOTS AND BIRDS ---
        // Group all simHeaven X-World together (Score 31) to allow alphabetical continent grouping
        if name_lower.contains("simheaven")
            || name_lower.contains("x-world")
            || name_lower.contains("w2xp")
        {
            return 31;
        }

        // Tweak 2: Birds_Library at top of birds block (32), others at 33
        if name_lower.contains("birds_library") {
            return 32; // Immediately after simHeaven block (31)
        }

        if name_lower.contains("birds")
            || name_lower.contains("birdofprey")
            || name_lower.contains("crow")
            || name_lower.contains("goose")
            || name_lower.contains("pigeon")
            || name_lower.contains("seagulls")
        {
            return 33;
        }

        // --- 7. ORBX A CUSTOM (Score 25) ---
        // Not airports, but custom/landmarks components of Orbx A packs
        if name_lower.contains("orbx_a") && name_lower.contains("custom") {
            return 25;
        }

        // --- 8. X-PLANE DEFAULT AIRPORTS (Score 22) ---
        // Moved below Global Airports to let defaults handle them
        if name_lower.contains("x-plane airports") {
            return 22;
        }

        // --- 9. GLOBAL AIRPORTS (Score 20) ---
        // Precedence: *GLOBAL_AIRPORTS* (20) comes BEFORE individual packs (22)
        if name_clean.eq_ignore_ascii_case("global airports")
            || name_clean.eq_ignore_ascii_case("global_airports")
            || name_lower.contains("x-plane landmarks")
        {
            return 20;
        }

        // --- 10. CUSTOM AIRPORTS (Score 10) ---
        if is_custom_airport && !name_lower.contains("overlay") {
            return 10;
        }

        // Fallback
        if name_lower.starts_with('z') || name_lower.starts_with('y') {
            return 50;
        }

        40
    }
}
