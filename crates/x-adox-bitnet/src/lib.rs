use anyhow::Result;

pub struct BitNetModel {
    // In a real implementation, this would hold the weights,
    // but now it holds the distilled logic.
}

impl BitNetModel {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// Predicts the scenery priority score (0-100) based on the pack name and path.
    /// Lower score = higher priority.
    pub fn predict(&self, name: &str, _path: &std::path::Path) -> u8 {
        // Distilled Logic from xpllamasort (Prompt Rules)
        // Weight: Airport (10) < GlobalAirport (20) < Library (30) < Overlay (40) < Ortho (50) < Mesh (60)

        let name_lower = name.to_lowercase();
        let name_clean = name.trim_matches(|c| c == '*' || c == ' ');

        // 1. Ortho (Rule: "starts with z or y" or contains "ortho")
        if name_lower.starts_with("zortho")
            || name_lower.starts_with("yortho")
            || name_lower.contains("ortho")
        {
            return 50;
        }

        // 2. Mesh (Rule: "starts with zzz" or contains "mesh")
        if name_lower.starts_with("zzz")
            || name_lower.contains("mesh")
            || name_lower.contains("uhd")
        {
            return 60;
        }

        // 3. Global Airports (Explicit)
        if name_clean.eq_ignore_ascii_case("global airports")
            || name_clean.eq_ignore_ascii_case("global_airports")
        {
            return 20;
        }

        // 4. Library (Rule: "assets for other packs", "opensceneryx")
        if name_lower.contains("library")
            || name_lower.contains("opensceneryx")
            || name_lower.contains("lib")
        {
            return 30;
        }

        // 5. Overlay (Rule: "roads, buildings, trees", "simheaven")
        if name_lower.contains("simheaven")
            || name_lower.contains("overlay")
            || name_lower.contains("w2xp")
            || name_lower.contains("landmark")
        {
            return 40;
        }

        // 6. Airport (Rule: "adds a specific airport", "apt", ICAO codes)
        // Strong signal: defined by "Earth nav data/earth_wed.xml" file existence often, but name check here:
        if name_lower.contains("airport")
            || name_lower.contains("apt")
            || (name_lower.starts_with('k') && name_lower.len() == 4)
        {
            return 10;
        }

        // Default: Treat unknown as Overlay/Generic Scenery (Safe middle ground)
        40
    }
}
