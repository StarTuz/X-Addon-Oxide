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
        // --- 1. MESH (Score 60) ---
        let name_lower = name.to_lowercase();
        if name_lower.contains("mesh")
            || name_lower.contains("uhd")
            || name_lower.contains("terrain")
            || name_lower.starts_with("zzz")
        {
            return 60;
        }

        // --- 2. ORTHO (Score 50) ---
        if name_lower.contains("ortho")
            || name_lower.contains("photoscenery")
            || name_lower.starts_with("yortho")
        {
            return 50;
        }

        // --- 3. BIRDS (Score 48) ---
        if name_lower.contains("birds") {
            return 48;
        }

        // --- 4. LIBRARIES (Score 45) ---
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

        // --- 5. ORBX B OVERLAYS (Score 42) ---
        if name_lower.contains("orbx_b") || name_lower.contains("trueearth_overlay") {
            return 42;
        }

        // --- 6. LANDMARKS / GLOBAL FORESTS (Score 40) ---
        if name_lower.contains("landmarks")
            || name_lower.contains("global_forests")
            || name_lower.contains("landmark") && !name_lower.contains("x-plane")
        {
            return 40;
        }

        // --- 7. SIMHEAVEN / W2XP (Score 31-36) ---
        if name_lower.contains("simheaven")
            || name_lower.contains("x-world")
            || name_lower.contains("w2xp")
        {
            if name_lower.contains("-1-vfr") || name_lower.contains("-2-regions") {
                return 31;
            }
            if name_lower.contains("-3-details")
                || name_lower.contains("-4-extras")
                || name_lower.contains("-5-footprints")
            {
                return 32;
            }
            if name_lower.contains("-7-forests") || name_lower.contains("vegetation") {
                return 34;
            }
            if name_lower.contains("-8-network") || name_lower.contains("-6-scenery") {
                return 36;
            }
            return 35; // Default for other simheaven
        }

        // --- 8. ORBX A CUSTOM (Score 25) ---
        // Not airports, but custom/landmarks components of Orbx A packs
        if name_lower.contains("orbx_a") && name_lower.contains("custom") {
            return 25;
        }

        // --- 9. GLOBAL AIRPORTS (Score 20) ---
        let name_clean = name.trim_matches(|c| c == '*' || c == ' ' || c == '_');
        if name_clean.eq_ignore_ascii_case("global airports")
            || name_clean.eq_ignore_ascii_case("global_airports")
            || name_lower.contains("x-plane airports")
            || name_lower.contains("x-plane landmarks")
        {
            return 20;
        }

        // --- 10. CUSTOM AIRPORTS (Score 10) ---
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

        let is_specific_panc = name_lower.contains("panc") || name_lower.contains("anchorage");

        // Simple ICAO detector: look for 4-letter uppercase words
        let has_icao = name.split(|c: char| !c.is_alphanumeric()).any(|word| {
            word.len() == 4 && word.chars().all(|c| c.is_alphabetic() && c.is_uppercase())
        });

        if has_airport_keyword
            || is_major_dev
            || is_specific_panc
            || has_icao
            || (name_lower.contains("scenery_pack") && name_lower.len() < 35)
        {
            return 10;
        }

        // Fallback
        if name_lower.starts_with('z') || name_lower.starts_with('y') {
            return 50;
        }

        40
    }
}
