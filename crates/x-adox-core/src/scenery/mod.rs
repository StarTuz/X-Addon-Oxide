pub mod classifier;
pub mod ini_handler;
pub mod sorter;
pub mod validator;

use crate::apt_dat::{Airport, AptDatParser};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub enum SceneryPackType {
    Active,
    Disabled,
    DuplicateHidden, // To be written as a comment with a special note
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, Hash)]
pub enum SceneryCategory {
    #[default]
    Unknown, // Fallback
    CustomAirport,    // Level 1: Score 100
    OrbxAirport,      // Level 2: Score 95
    GlobalAirport,    // Level 3: Score 90
    Landmark,         // Level 4: Score 88
    RegionalOverlay,  // Level 5: Score 85
    RegionalFluff,    // Level 7/8 in new scale? No wait, spec says 80.
    AirportOverlay,   // Level 6: Score 75
    LowImpactOverlay, // Level 7: Score 70
    AutoOrthoOverlay, // Level 8: Score 70 -> Wait spec is 70? No, AutoOrtho Overlay is 70 in user request?
    // User request:
    // Level 7: Dynamic/low-impact (Score 70)
    // Level 8: AutoOrtho overlay (Score 65 in main text, wait let me check artifact)
    // Artifact says: AutoOrthoOverlay (70). Level 7 LowImpact is 70.
    // Spec says:
    // "Level 8: AutoOrtho corrections/overlays (score ~65)"
    // "Level 7: Dynamic/low-impact overlays (score ~70)"
    // So LowImpact > AutoOrtho.
    Library, // Level 11: Score 65??
    // User request spec says: "Level 11: Libraries (score ~50 — lowest)"
    // Wait, recent update says:
    // "65: Libraries ... 55: Ortho/Photo Base ... 30: Mesh"
    // So Library is 65.
    // Ortho is 55.
    // Mesh is 30.
    // Let's stick to the Plan's values which were derived from the final user prompt.
    // Plan:
    // RegionalFluff (80)
    // AirportOverlay (75)
    // AutoOrthoOverlay (70) -> Wait, user said 70 for AutoOrtho in one place?
    // Let's re-read user prompt "Version 5.0":
    // "70: AutoOrtho Corrections Match: yAutoOrtho_Overlays/"
    // "65: Libraries Match: *_Library"
    // "55: Ortho/Photo Base"
    // "30: Mesh"
    // So AutoOrthoOverlay is 70.
    // LowImpactOverlay? "80: Regional Fluff (Forests... Birds...)"
    // Wait, user text says: "80: Regional Fluff (Forests, Networks, Low-Impact)".
    // So "LowImpact" is part of Regional Fluff?
    // User text also says: "Level 7: Dynamic/low-impact overlays (score ~70) ... Birds ... Global_Forests".
    // But then list says "80: Regional Fluff ... Birds ... Global_Forests".
    // There is a conflict in the user prompt between the text numbers and the "Recap" list.
    // "80: Regional Fluff (Forests, Networks, Low-Impact) Match: simHeaven *_7-forests ... Birds ... Global_Forests"
    // This seems to merge Level 5 and 7?
    // No, "85: Regional Detail Layers".
    // "80: Regional Fluff".
    // "75: Airport-Specific Enhancements".
    // "70: AutoOrtho Corrections".
    // "65: Libraries".
    // "55: Ortho".
    // "30: Mesh".
    // This looks like the consistent set from the "Consensus-Backed Final Heuristics".
    // I will follow this specific numbered list.
    OrthoBase,    // Level 10: Score 55
    GlobalBase,   // Level 9: Score 60 (Demo Areas, etc.)
    SpecificMesh, // Level 11: Score 30 (Mesh)
    Mesh,
    Group, // Virtual group pack
}

impl SceneryCategory {
    pub fn short_code(&self) -> &'static str {
        match self {
            SceneryCategory::Unknown => "UNK",
            SceneryCategory::CustomAirport => "APT",
            SceneryCategory::OrbxAirport => "ORX",
            SceneryCategory::GlobalAirport => "GLO",
            SceneryCategory::Landmark => "LMK",
            SceneryCategory::RegionalOverlay => "REG",
            SceneryCategory::RegionalFluff => "RFL",
            SceneryCategory::AirportOverlay => "AOV",
            SceneryCategory::LowImpactOverlay => "LOW",
            SceneryCategory::AutoOrthoOverlay => "AOO",
            SceneryCategory::Library => "LIB",
            SceneryCategory::OrthoBase => "ORT",
            SceneryCategory::GlobalBase => "GBS",
            SceneryCategory::SpecificMesh => "MSH",
            SceneryCategory::Mesh => "MSH",
            SceneryCategory::Group => "GRP",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub struct SceneryPack {
    pub name: String,
    pub path: PathBuf,
    pub status: SceneryPackType,
    pub category: SceneryCategory,
    pub airports: Vec<Airport>,
    pub tiles: Vec<(i32, i32)>, // SW corner (lat, lon)
    pub tags: Vec<String>,
}

impl SceneryPack {
    pub fn calculate_health_score(&self) -> u8 {
        let mut score: u8 = 0;

        // 1. Category Baseline (20 points)
        if self.category != SceneryCategory::Unknown {
            score += 20;
        }

        // 2. Content Evaluation (Max 70 points)
        let has_airports = !self.airports.is_empty();
        let has_tiles = !self.tiles.is_empty();

        match self.category {
            // System / Trusted types get high base scores immediately
            SceneryCategory::GlobalAirport
            | SceneryCategory::Library
            | SceneryCategory::GlobalBase => {
                score += 70;
            }
            // Scenery types that SHOULD have tiles logic
            SceneryCategory::OrthoBase
            | SceneryCategory::Mesh
            | SceneryCategory::SpecificMesh
            | SceneryCategory::RegionalOverlay
            | SceneryCategory::AutoOrthoOverlay
            | SceneryCategory::AirportOverlay
            | SceneryCategory::LowImpactOverlay => {
                if has_tiles {
                    score += 60;
                } else if has_airports {
                    // Fallback: categorized as Scenery but only has airports? Strange but ok.
                    score += 40;
                }
            }
            // Airport types
            SceneryCategory::CustomAirport | SceneryCategory::OrbxAirport => {
                if has_airports {
                    score += 60;
                } else if has_tiles {
                    // Orbx 'Custom' or 'POI' packs are often just POI buildings (tiles), not airports.
                    // Give them more credit if they have tiles but no airports.
                    if self.name.to_lowercase().contains("custom")
                        || self.name.to_lowercase().contains("poi")
                    {
                        score += 60;
                    } else {
                        score += 40;
                    }
                }
            }
            // Unknown / Group / Other
            _ => {
                // Generic scoring: Reward presence of anything
                if has_airports {
                    score += 35;
                }
                if has_tiles {
                    score += 35;
                }
            }
        }

        // 3. User Metadata (10 points)
        if !self.tags.is_empty() {
            score += 10;
        }

        // Penalties
        // If it's NOT a library/group/system and has NO content: Crash score
        if !has_airports
            && !has_tiles
            && self.category != SceneryCategory::Library
            && self.category != SceneryCategory::Group
            && self.category != SceneryCategory::GlobalAirport
        {
            return 10; // "Broken" / Empty folder
        }

        score.min(100)
    }
}

#[derive(Error, Debug)]
pub enum SceneryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error on line {0}: {1}")]
    Parse(usize, String),
}

pub struct SceneryManager {
    pub file_path: PathBuf,
    pub packs: Vec<SceneryPack>,
    pub group_manager: Option<crate::groups::GroupManager>,
}

impl SceneryManager {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            packs: Vec::new(),
            group_manager: None,
        }
    }

    pub fn set_bulk_states(&mut self, states: &std::collections::HashMap<String, bool>) {
        for pack in &mut self.packs {
            if let Some(&enabled) = states.get(&pack.name) {
                pack.status = if enabled {
                    SceneryPackType::Active
                } else {
                    SceneryPackType::Disabled
                };
            }
        }
    }

    pub fn find_conflicts(&self, pack_name: &str) -> Vec<String> {
        let target_pack = match self.packs.iter().find(|p| p.name == pack_name) {
            Some(p) => p,
            None => return Vec::new(),
        };

        if target_pack.tiles.is_empty() {
            return Vec::new();
        }

        let mut conflicts = Vec::new();
        for other in &self.packs {
            if other.name == pack_name || other.status != SceneryPackType::Active {
                continue;
            }

            for tile in &target_pack.tiles {
                if other.tiles.contains(tile) {
                    conflicts.push(other.name.clone());
                    break;
                }
            }
        }
        conflicts
    }

    pub fn load(&mut self) -> Result<(), SceneryError> {
        let custom_scenery_dir = self.file_path.parent().unwrap_or(&self.file_path);
        println!("[SceneryManager] Loading from INI: {:?}", self.file_path);
        println!(
            "[SceneryManager] Custom Scenery Dir: {:?}",
            custom_scenery_dir
        );

        // 1. Read existing INI entries
        let mut packs = match ini_handler::read_ini(&self.file_path, custom_scenery_dir) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("[SceneryManager] INI file not found, starting fresh");
                Vec::new()
            }
            Err(e) => return Err(SceneryError::Io(e)),
        };
        println!("[SceneryManager] Read {} packs from INI", packs.len());

        // De-duplicate INI entries just in case
        let initial_count = packs.len();
        let mut seen_keys = HashSet::new();
        packs.retain(|p| {
            let path_str = p.path.to_string_lossy();
            let key = if p.name == "*GLOBAL_AIRPORTS*"
                || path_str.ends_with("Global Airports")
                || path_str.ends_with("Global Airports/")
            {
                "VIRTUAL:GLOBAL_AIRPORTS".to_string()
            } else {
                path_str.to_string()
            };
            seen_keys.insert(key)
        });
        if packs.len() < initial_count {
            println!(
                "[SceneryManager] Removed {} duplicate INI entries",
                initial_count - packs.len()
            );
        }

        // 2. Scan Custom Scenery for new packs not yet in the INI
        let mut cache = crate::cache::DiscoveryCache::load();
        let discovered =
            crate::discovery::DiscoveryManager::scan_scenery(custom_scenery_dir, &mut cache);
        println!(
            "[SceneryManager] Discovered {} folders in Custom Scenery",
            discovered.len()
        );

        // 2b. Also scan Global Scenery ONLY for Global Airports
        let xplane_root = custom_scenery_dir.parent().unwrap_or(custom_scenery_dir);
        let global_airports_dir = xplane_root.join("Global Scenery").join("Global Airports");
        let global_discovered = if global_airports_dir.exists() {
            // Just simulate a discovery for this one specific folder
            vec![crate::discovery::DiscoveredAddon {
                name: "Global Airports".to_string(),
                path: global_airports_dir,
                addon_type: crate::discovery::AddonType::Scenery {
                    airports: Vec::new(),
                },
                is_enabled: true,
                tags: Vec::new(),
            }]
        } else {
            Vec::new()
        };
        println!(
            "[SceneryManager] Targeted Global Airports discovery: {}",
            !global_discovered.is_empty()
        );

        // Merge both discovery results
        let all_discovered: Vec<_> = discovered.into_iter().chain(global_discovered).collect();

        for disc in all_discovered {
            // Special Case: *GLOBAL_AIRPORTS* virtual tag matches physical "Global Airports" folder.
            // We must reconcile them to avoid double entries in the INI while allowing discovery
            // to work on the physical path.
            let is_global_airports_folder = disc.name == "Global Airports"
                || disc.path.to_string_lossy().ends_with("Global Airports");

            let existing_idx = packs.iter().position(|p| {
                if p.path == disc.path {
                    return true;
                }
                if is_global_airports_folder && p.name == "*GLOBAL_AIRPORTS*" {
                    return true;
                }
                false
            });

            if let Some(idx) = existing_idx {
                // If it's the virtual tag, update its path to the physical one so discovery works.
                if packs[idx].name == "*GLOBAL_AIRPORTS*" && is_global_airports_folder {
                    packs[idx].path = disc.path;
                }
            } else {
                println!("[SceneryManager] Adding NEW discovered pack: {}", disc.name);

                // Prepend new discovery (X-Plane style)
                let new_pack = SceneryPack {
                    name: disc.name,
                    path: disc.path,
                    status: SceneryPackType::Active,
                    category: SceneryCategory::Unknown,
                    airports: Vec::new(),
                    tiles: Vec::new(),
                    tags: Vec::new(),
                };
                packs.insert(0, new_pack);
            }
        }

        use rayon::prelude::*;

        let cache_ref = &cache;
        let processed_results: Vec<(SceneryPack, Option<crate::cache::CacheEntry>)> = packs
            .into_par_iter()
            .map(|mut pack| {
                // Apply heuristic classification (now parallelized)
                pack.category = classifier::Classifier::classify_heuristic(&pack.path, &pack.name);

                // 2. Discover details with cache check
                let (airports, tiles, cache_entry) = if let Some(entry) = cache_ref.get(&pack.path)
                {
                    (entry.airports.clone(), entry.tiles.clone(), None)
                } else {
                    let airports = discover_airports_in_pack(&pack.path);
                    let tiles = discover_tiles_in_pack(&pack.path);

                    let mtime = std::fs::metadata(&pack.path)
                        .and_then(|m| m.modified())
                        .map(|m| m.into())
                        .unwrap_or_else(|_| chrono::Utc::now());

                    (
                        airports.clone(),
                        tiles.clone(),
                        Some(crate::cache::CacheEntry {
                            mtime,
                            addons: Vec::new(),
                            airports,
                            tiles,
                        }),
                    )
                };

                pack.airports = airports;
                pack.tiles = tiles;

                // 3. Post-Discovery Promotion
                // If we FOUND actual airports, this is a Custom Airport (Score 100)
                // UNLESS it's already a 'System' category like GlobalAirport or Library.
                if !pack.airports.is_empty() {
                    match pack.category {
                        SceneryCategory::GlobalAirport
                        | SceneryCategory::Library
                        | SceneryCategory::GlobalBase => {
                            // Keep existing system category
                        }
                        _ => {
                            // Promote to Custom Airport
                            pack.category = SceneryCategory::CustomAirport;
                        }
                    }
                } else if pack.category == SceneryCategory::Unknown && !pack.tiles.is_empty() {
                    // If it has tiles but no airports, it's likely a regional enhancement or ortho
                    if pack.name.to_lowercase().contains("ortho") {
                        pack.category = SceneryCategory::OrthoBase;
                    } else {
                        pack.category = SceneryCategory::RegionalOverlay;
                    }
                }

                // 4. Final Healing (Centralized)
                pack.category = classifier::Classifier::heal_classification(
                    pack.category,
                    !pack.airports.is_empty(),
                    !pack.tiles.is_empty(),
                );

                (pack, cache_entry)
            })
            .collect();

        let mut final_packs = Vec::with_capacity(processed_results.len());
        for (pack, cache_update) in processed_results {
            if let Some(entry) = cache_update {
                if !entry.airports.is_empty() {
                    println!(
                        "[SceneryManager] Pack '{}' initialized with {} airports",
                        pack.name,
                        entry.airports.len()
                    );
                }
                cache.entries.insert(pack.path.clone(), entry);
            }

            if !pack.tiles.is_empty() || !pack.airports.is_empty() {
                // Keep the verbosity similar but avoid spamming too much in parallel
            }
            final_packs.push(pack);
        }
        let mut packs = final_packs;

        // 3. Load Tags
        let xplane_root = custom_scenery_dir.parent().unwrap_or(custom_scenery_dir);
        let group_mgr = crate::groups::GroupManager::new(xplane_root);
        if let Ok(collection) = group_mgr.load() {
            for pack in &mut packs {
                if let Some(tags) = collection.pack_tags.get(&pack.name) {
                    pack.tags = tags.clone();
                }
            }
        }

        self.group_manager = Some(group_mgr);
        self.packs = packs;

        // 4. Save cache AFTER all discovery (prevents partial/missing saves)
        let _ = cache.save();

        Ok(())
    }

    pub fn enable_pack(&mut self, name: &str) {
        if let Some(pack) = self.packs.iter_mut().find(|p| p.name == name) {
            pack.status = SceneryPackType::Active;
        }
    }

    pub fn disable_pack(&mut self, name: &str) {
        if let Some(pack) = self.packs.iter_mut().find(|p| p.name == name) {
            pack.status = SceneryPackType::Disabled;
        }
    }

    pub fn add_tag(&mut self, pack_name: &str, tag: &str) -> anyhow::Result<()> {
        if let Some(pack) = self.packs.iter_mut().find(|p| p.name == pack_name) {
            if !pack.tags.contains(&tag.to_string()) {
                pack.tags.push(tag.to_string());
                self.save_tags()?;
            }
        }
        Ok(())
    }

    pub fn remove_tag(&mut self, pack_name: &str, tag: &str) -> anyhow::Result<()> {
        if let Some(pack) = self.packs.iter_mut().find(|p| p.name == pack_name) {
            if let Some(pos) = pack.tags.iter().position(|t| t == tag) {
                pack.tags.remove(pos);
                self.save_tags()?;
            }
        }
        Ok(())
    }

    pub fn save_tags(&self) -> anyhow::Result<()> {
        if let Some(mgr) = &self.group_manager {
            let mut map = std::collections::HashMap::new();
            for p in &self.packs {
                if !p.tags.is_empty() {
                    map.insert(p.name.clone(), p.tags.clone());
                }
            }
            mgr.save(&crate::groups::GroupCollection { pack_tags: map })?;
        }
        Ok(())
    }

    pub fn sorted_for_ui(&self) -> Vec<SceneryPack> {
        let mut ui_packs = self.packs.clone();
        sorter::sort_packs(
            &mut ui_packs,
            None,
            &x_adox_bitnet::PredictContext::default(),
        );
        ui_packs
    }

    pub fn sort(
        &mut self,
        model: Option<&x_adox_bitnet::BitNetModel>,
        context: &x_adox_bitnet::PredictContext,
    ) {
        Self::handle_duplicates(&mut self.packs);
        sorter::sort_packs(&mut self.packs, model, context);
    }

    pub fn validate_sort(&self) -> validator::ValidationReport {
        validator::SceneryValidator::validate(&self.packs)
    }

    pub fn simulate_sort(
        &self,
        model: &x_adox_bitnet::BitNetModel,
        context: &x_adox_bitnet::PredictContext,
    ) -> (Vec<SceneryPack>, validator::ValidationReport) {
        let mut simulated_packs = self.packs.clone();
        Self::handle_duplicates(&mut simulated_packs);
        sorter::sort_packs(&mut simulated_packs, Some(model), context);
        let report = validator::SceneryValidator::validate(&simulated_packs);
        (simulated_packs, report)
    }

    pub fn save(&self, model: Option<&x_adox_bitnet::BitNetModel>) -> Result<(), SceneryError> {
        self.perform_backup()?;
        ini_handler::write_ini(&self.file_path, &self.packs, model)?;
        Ok(())
    }

    /// Performs a rotating timestamped backup of scenery_packs.ini in a dedicated folder.
    fn perform_backup(&self) -> Result<(), SceneryError> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let backup_dir = crate::get_config_root().join("backups");

        // 1. Ensure backup directory exists
        if !backup_dir.exists() {
            std::fs::create_dir_all(&backup_dir)?;
        }

        // 2. Create new timestamped backup
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S%.3f").to_string();
        let backup_path = backup_dir.join(format!("scenery_packs.ini.{}", timestamp));
        std::fs::copy(&self.file_path, &backup_path)?;

        // 3. Cleanup old backups (keep last 10)
        self.cleanup_old_backups(&backup_dir)?;

        Ok(())
    }

    /// Keeps only the most recent 10 backups in the specified directory.
    fn cleanup_old_backups(&self, backup_dir: &std::path::Path) -> Result<(), SceneryError> {
        let mut backups = Vec::new();

        if let Ok(entries) = std::fs::read_dir(backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(metadata) = path.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            backups.push((path, modified));
                        }
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        backups.sort_by_key(|&(_, modified)| modified);

        // Remove oldest if more than 10
        if backups.len() > 10 {
            let to_remove = backups.len() - 10;
            for i in 0..to_remove {
                let _ = std::fs::remove_file(&backups[i].0);
            }
        }

        Ok(())
    }

    /// Detects duplicates based on normalized names and handles them by disabling older versions.
    fn handle_duplicates(packs: &mut Vec<SceneryPack>) {
        use std::collections::HashMap;

        // Group indices by clean name
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, pack) in packs.iter().enumerate() {
            let clean = clean_name(&pack.name);
            if clean.len() < 3 || clean == "customscenery" {
                continue;
            }
            groups.entry(clean).or_default().push(i);
        }

        let mut to_disable = Vec::new();
        let mut win_to_losers: HashMap<usize, Vec<usize>> = HashMap::new();

        for (name, indices) in groups {
            if indices.len() > 1 {
                println!(
                    "[SceneryManager] Found duplicates for '{}': {:?}",
                    name, indices
                );

                let mut best_idx = indices[0];
                let mut best_ver = extract_version(&packs[best_idx].name);
                let mut best_time = get_modified_time(&packs[best_idx].path);

                for &idx in &indices[1..] {
                    let ver = extract_version(&packs[idx].name);
                    let time = get_modified_time(&packs[idx].path);

                    let mut replace = false;
                    if let (Some(v1), Some(v2)) = (&ver, &best_ver) {
                        if v1 > v2 {
                            replace = true;
                        }
                    } else if ver.is_some() && best_ver.is_none() {
                        replace = true;
                    } else if ver.is_none() && best_ver.is_none() && time > best_time {
                        replace = true;
                    }

                    if replace {
                        best_idx = idx;
                        best_ver = ver;
                        best_time = time;
                    }
                }

                // Register losers
                let mut losers = Vec::new();
                for &idx in &indices {
                    if idx != best_idx {
                        packs[idx].status = SceneryPackType::DuplicateHidden;
                        to_disable.push(idx);
                        losers.push(idx);
                    }
                }
                if !losers.is_empty() {
                    win_to_losers.insert(best_idx, losers);
                }
            }
        }

        if to_disable.is_empty() {
            return;
        }

        // Reordering: LOSERS must be placed immediately after their WINNER.
        // We do this by creating a new vector.
        let mut new_packs = Vec::with_capacity(packs.len());
        let mut handled = std::collections::HashSet::new();

        for i in 0..packs.len() {
            if handled.contains(&i) {
                continue;
            }

            // If this is a winner, add it and its losers
            if let Some(losers) = win_to_losers.get(&i) {
                new_packs.push(packs[i].clone());
                handled.insert(i);
                for &l_idx in losers {
                    if !handled.contains(&l_idx) {
                        new_packs.push(packs[l_idx].clone());
                        handled.insert(l_idx);
                    }
                }
            } else if !to_disable.contains(&i) {
                // Regular pack
                new_packs.push(packs[i].clone());
                handled.insert(i);
            }
        }

        // Catch any remaining to_disable that were skipped because their winner was a loser of another group (cascading)
        // or other edge cases.
        for i in 0..packs.len() {
            if !handled.contains(&i) {
                new_packs.push(packs[i].clone());
            }
        }

        *packs = new_packs;
    }
}

fn get_modified_time(path: &Path) -> u64 {
    if let Ok(metadata) = std::fs::metadata(path) {
        if let Ok(time) = metadata.modified() {
            return time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }
    0
}

fn clean_name(name: &str) -> String {
    use std::sync::OnceLock;
    static RE_XP: OnceLock<regex::Regex> = OnceLock::new();
    static RE_VER: OnceLock<regex::Regex> = OnceLock::new();
    static RE_COPY: OnceLock<regex::Regex> = OnceLock::new();

    // Strict Name Cleaning:
    // - Strips: _v1.2, _XP12, _XP11, (space)v2
    // - Preserves: 100m, 300ft, 4K, UHD, standalone numbers
    let name_lower = name.to_lowercase();

    // Remove XP suffixes first
    let re_xp = RE_XP.get_or_init(|| regex::Regex::new(r"(?i)[-_ ]?xp\d*").unwrap());
    let no_xp = re_xp.replace_all(&name_lower, "");

    // Remove strict version patterns:
    // Matches: v1.2, v2, _v1, -v2.5.0
    // Does NOT match: 100m, 300ft, 400
    let re_ver = RE_VER.get_or_init(|| regex::Regex::new(r"(?i)[-_ ]?v\d+(\.\d+)*").unwrap());
    let no_ver = re_ver.replace_all(&no_xp, "");

    // Remove OS copy suffixes: (1), (2), etc.
    let re_copy = RE_COPY.get_or_init(|| regex::Regex::new(r"\s+\(\d+\)$").unwrap());
    let no_copy = re_copy.replace_all(&no_ver, "");

    // Final trim
    no_copy.trim().replace(['_', ' '], "").to_string()
}

fn extract_version(name: &str) -> Option<String> {
    use std::sync::OnceLock;
    static RE_V: OnceLock<regex::Regex> = OnceLock::new();
    static RE_DOT: OnceLock<regex::Regex> = OnceLock::new();

    // Robust Version Parsing:
    // - Requires 'v' prefix OR invalid chars around it to be a version
    // - Matches: v1.2, 1.0.5, 2.0
    // - Does NOT match: 100 (meters), 4000 (pixels)

    // 1. Explicit 'v' prefix (strongest signal)
    let re_v = RE_V.get_or_init(|| regex::Regex::new(r"(?i)v(\d+(\.\d+)*)").unwrap());
    if let Some(cap) = re_v.captures(name) {
        return Some(cap[1].to_string());
    }

    // 2. SemVer pattern (x.y.z) - requires at least one dot
    let re_dot = RE_DOT.get_or_init(|| regex::Regex::new(r"(\d+\.\d+(\.\d+)*)").unwrap());
    if let Some(cap) = re_dot.captures(name) {
        return Some(cap[1].to_string());
    }

    None
}

/// Recursively find all directories within a pack that look like actual scenery roots
/// (containing 'Earth nav data', 'library.txt', or 'earth.wed.xml').
fn find_pack_roots(pack_path: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // 1. Check the pack path itself first
    if is_scenery_root(pack_path) {
        roots.push(pack_path.to_path_buf());
    }

    // 2. Check one and two levels deeper (common for nested/wrapped packs)
    if let Ok(entries) = std::fs::read_dir(pack_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if is_scenery_root(&path) {
                    roots.push(path.clone());
                } else {
                    // One more level deep (e.g. Pack/Sub/Data)
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.is_dir() && is_scenery_root(&sub_path) {
                                roots.push(sub_path);
                            }
                        }
                    }
                }
            }
        }
    }

    // If no specific roots found but it's a valid directory, at least return the base
    // so heuristic classification still works.
    if roots.is_empty() {
        roots.push(pack_path.to_path_buf());
    }

    roots.sort();
    roots.dedup();
    roots
}

fn is_scenery_root(path: &Path) -> bool {
    let signals = [
        "earth nav data",
        "library.txt",
        "apt.dat",
        "earth.wed.xml",
        "earth.wed.bak.xml",
        "mars nav data",
    ];
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if signals.iter().any(|&s| s == name) {
                return true;
            }
        }
    }
    false
}

fn discover_tiles_in_pack(pack_path: &Path) -> Vec<(i32, i32)> {
    let mut tiles = Vec::new();
    let nav_data_dirs = ["Earth nav data", "Mars nav data"];
    let pack_path_str = pack_path.to_string_lossy().to_lowercase();

    // Clutter filter
    let excluded_keywords = [
        "resources",
        "default scenery",
        "opensceneryx",
        "world2xplane",
    ];

    let is_global_airports =
        pack_path_str.contains("global airports") || pack_path_str.contains("global_airports");

    if !is_global_airports {
        for keyword in excluded_keywords {
            if pack_path_str.contains(keyword) {
                return tiles;
            }
        }
    }

    // Find real roots (might be nested)
    let roots = find_pack_roots(pack_path);

    for root in roots {
        // Search for nav data folders case-insensitively within each root
        let real_nav_path = if let Ok(entries) = std::fs::read_dir(&root) {
            entries.flatten().find_map(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if nav_data_dirs.iter().any(|&d| d.to_lowercase() == name) {
                    Some(e.path())
                } else {
                    None
                }
            })
        } else {
            None
        };

        if let Some(nav_path) = real_nav_path {
            // Search for folders like +40-090
            if let Ok(entries) = std::fs::read_dir(nav_path) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let folder_path = entry.path();

                        // Scan inside the folder for .dsf files (e.g., +41-088.dsf)
                        if let Ok(file_entries) = std::fs::read_dir(folder_path) {
                            for file_entry in file_entries.flatten() {
                                let file_name =
                                    file_entry.file_name().to_string_lossy().to_string();
                                if file_name.to_lowercase().ends_with(".dsf") {
                                    if let Some(tile) = parse_tile_name(&file_name) {
                                        tiles.push(tile);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    tiles.sort();
    tiles.dedup();

    // Filter massive regional packs (max 100 tiles for the map)
    if tiles.len() > 100 {
        return Vec::new();
    }

    tiles
}

fn parse_tile_name(name: &str) -> Option<(i32, i32)> {
    // Strips .dsf if present
    let name = name.strip_suffix(".dsf").unwrap_or(name);

    // Format: +40-120 or -10+030
    if name.len() < 6 {
        return None;
    }

    let lat_str = &name[0..3];
    let lon_str = &name[3..];

    let lat = lat_str.parse::<i32>().ok()?;
    let lon = lon_str.parse::<i32>().ok()?;

    Some((lat, lon))
}

fn discover_airports_in_pack(pack_path: &Path) -> Vec<Airport> {
    let mut all_airports = Vec::new();
    let roots = find_pack_roots(pack_path);

    for root in roots {
        // 1. Find "Earth nav data" case-insensitively
        let real_nav_path = if let Ok(entries) = std::fs::read_dir(&root) {
            entries.flatten().find_map(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if name == "earth nav data" {
                    Some(e.path())
                } else {
                    None
                }
            })
        } else {
            None
        };

        let mut apt_path = None;

        if let Some(nav_path) = real_nav_path {
            // 2. Find "apt.dat" case-insensitively inside "Earth nav data"
            if let Ok(entries) = std::fs::read_dir(&nav_path) {
                apt_path = entries.flatten().find_map(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    if name == "apt.dat" {
                        Some(e.path())
                    } else {
                        None
                    }
                });
            }
        }

        // 3. Fallback: Search for "apt.dat" in the ROOT of the pack
        // (Common for some custom scenery packs)
        if apt_path.is_none() {
            if let Ok(entries) = std::fs::read_dir(&root) {
                apt_path = entries.flatten().find_map(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    if name == "apt.dat" {
                        Some(e.path())
                    } else {
                        None
                    }
                });
            }
        }

        if let Some(apt_path) = apt_path {
            match AptDatParser::parse_file(&apt_path) {
                Ok(airports) => all_airports.extend(airports),
                Err(e) => {
                    println!(
                        "[SceneryManager] ERROR parsing {}: {}",
                        apt_path.display(),
                        e
                    );
                }
            }
        }
    }

    all_airports.sort_by(|a, b| a.id.cmp(&b.id));
    all_airports.dedup_by(|a, b| a.id == b.id);
    all_airports
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_discover_tiles_dsf() {
        let dir = tempdir().unwrap();
        let pack_path = dir.path();
        let nav_path = pack_path.join("Earth nav data");
        let grid_path = nav_path.join("+30-010");
        std::fs::create_dir_all(&grid_path).unwrap();

        // Faro DSF
        std::fs::write(grid_path.join("+37-008.dsf"), "").unwrap();
        // Another tile in the same grid
        std::fs::write(grid_path.join("+38-009.dsf"), "").unwrap();

        let tiles = discover_tiles_in_pack(pack_path);
        assert_eq!(tiles.len(), 2);
        assert_eq!(tiles[0], (37, -8));
        assert_eq!(tiles[1], (38, -9));
    }

    #[test]
    fn test_parse_scenery_ini() {
        // Create an isolated directory for the test
        let dir = tempdir().unwrap();
        let ini_path = dir.path().join("scenery_packs.ini");

        let mut file = std::fs::File::create(&ini_path).unwrap();
        writeln!(file, "I").unwrap();
        writeln!(file, "1000 Version").unwrap();
        writeln!(file, "SCENERY").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "SCENERY_PACK Custom Scenery/simHeaven_X-Europe/").unwrap();
        writeln!(file, "SCENERY_PACK_DISABLED Custom Scenery/Orbx_NorCal/").unwrap();

        let mut manager = SceneryManager::new(ini_path);
        // Ensure no other folders exist in this temp dir to confuse discovery
        manager.load().expect("Failed to load ini");

        assert_eq!(manager.packs.len(), 2);
        assert_eq!(manager.packs[0].name, "simHeaven_X-Europe");
        assert_eq!(manager.packs[1].name, "Orbx_NorCal");
    }

    #[test]
    fn test_save_scenery_ini() {
        let dir = tempdir().unwrap();
        let ini_path = dir.path().join("scenery_packs.ini");

        // Ensure parent dir exists (it does from tempdir)
        let mut manager = SceneryManager::new(ini_path);

        manager.packs.push(SceneryPack {
            name: "TestPack".to_string(),
            path: PathBuf::from("Custom Scenery/TestPack/"),
            status: SceneryPackType::Active,
            category: SceneryCategory::default(),
            airports: Vec::new(),
            tiles: Vec::new(),
            tags: Vec::new(),
        });

        manager.save(None).expect("Failed to save");

        let mut verify_manager = SceneryManager::new(manager.file_path.clone());
        verify_manager.load().expect("Failed to reload");

        assert_eq!(verify_manager.packs.len(), 1);
        assert_eq!(verify_manager.packs[0].name, "TestPack");
    }
    #[test]
    fn test_strict_duplicate_detection_logic() {
        // Case 1: Same name, different versions (Should match)
        let n1 = clean_name("Airport_v1.0");
        let n2 = clean_name("Airport v2");
        assert_eq!(n1, "airport");
        assert_eq!(n2, "airport"); // Match!

        // Case 2: Content Attributes (Should NOT match)
        let n3 = clean_name("Goose100m");
        let n4 = clean_name("Goose300m");
        assert_eq!(n3, "goose100m");
        assert_eq!(n4, "goose300m"); // Different!

        // Case 3: XP Platform suffixes (Should match)
        let n5 = clean_name("Orbx_NorCal_XP11");
        let n6 = clean_name("Orbx_NorCal_XP12");
        assert_eq!(n5, "orbxnorcal");
        assert_eq!(n6, "orbxnorcal"); // Match!

        // Case 4: OS Copy Suffix (e.g. " (1)")
        let n7 = clean_name("Fly2High - KTUL (Tulsa International Airport) XP12");
        let n8 = clean_name("Fly2High - KTUL (Tulsa International Airport) XP12 (1)");
        assert_eq!(n7, n8); // Should match
    }

    #[test]
    fn test_duplicate_placement_logic() {
        let mut packs = vec![
            SceneryPack {
                name: "Alpha_Airport".to_string(),
                path: PathBuf::from("A"),
                status: SceneryPackType::Active,
                category: SceneryCategory::default(),
                airports: Vec::new(),
                tiles: Vec::new(),
                tags: Vec::new(),
            },
            SceneryPack {
                name: "Bravo_Airport".to_string(),
                path: PathBuf::from("B"),
                status: SceneryPackType::Active,
                category: SceneryCategory::default(),
                airports: Vec::new(),
                tiles: Vec::new(),
                tags: Vec::new(),
            },
            SceneryPack {
                name: "Alpha_Airport (1)".to_string(),
                path: PathBuf::from("A1"),
                status: SceneryPackType::Active,
                category: SceneryCategory::default(),
                airports: Vec::new(),
                tiles: Vec::new(),
                tags: Vec::new(),
            },
        ];

        SceneryManager::handle_duplicates(&mut packs);

        assert_eq!(packs.len(), 3);
        assert_eq!(packs[0].name, "Alpha_Airport");
        assert_eq!(packs[0].status, SceneryPackType::Active);

        // LOSER should be at index 1 now (immediately after winner A)
        assert_eq!(packs[1].name, "Alpha_Airport (1)");
        assert_eq!(packs[1].status, SceneryPackType::DuplicateHidden);

        // B should be pushed to index 2
        assert_eq!(packs[2].name, "Bravo_Airport");
    }

    #[test]
    fn test_version_extraction() {
        assert_eq!(extract_version("Test_v1.2"), Some("1.2".to_string()));
        assert_eq!(extract_version("Test v2.0"), Some("2.0".to_string()));
        assert_eq!(extract_version("Test 1.0.5"), Some("1.0.5".to_string()));

        // Negative cases (Plain numbers are NOT versions)
        assert_eq!(extract_version("Test 100m"), None);
        assert_eq!(extract_version("Test 4000"), None);
    }

    #[test]
    fn test_scenery_backup_retention() {
        let dir = tempdir().unwrap();
        // ISOLATION: Redirect config root to temp dir to avoid destroying user backups
        std::env::set_var("X_ADOX_CONFIG_DIR", dir.path());

        let ini_path = dir.path().join("scenery_packs.ini");
        let backup_dir = dir.path().join("backups");

        // 1. Initial save (no backup because no file yet)
        let manager = SceneryManager::new(ini_path.clone());
        manager.save(None).expect("Save failed");
        assert!(ini_path.exists());

        // 2. Second save (should create .xam_backups and first backup)
        manager.save(None).expect("Save failed");
        assert!(backup_dir.exists());
        let entries: Vec<_> = std::fs::read_dir(&backup_dir).unwrap().flatten().collect();
        assert_eq!(entries.len(), 1);

        // 3. Save 12 more times (total 14 saves, but only 13 backups since first save didn't backup)
        // Retention is 10, so we should end up with exactly 10 backups.
        for _ in 0..12 {
            // Sleep briefly to ensure unique modification times if OS precision is low,
            // though our code uses modification time and filesystem precision might vary.
            std::thread::sleep(std::time::Duration::from_millis(10));
            manager.save(None).expect("Save failed");
        }

        let entries: Vec<_> = std::fs::read_dir(&backup_dir).unwrap().flatten().collect();
        // Retention caps at 10, so valid count is 10.
        assert_eq!(entries.len(), 10);
    }

    #[test]
    fn test_is_subset() {
        use super::validator::is_subset;

        // Exact match
        assert!(is_subset(&[(10, 20)], &[(10, 20)]));
        // Subset
        assert!(is_subset(&[(10, 20)], &[(10, 20), (11, 21)]));
        // Multi-subset
        assert!(is_subset(
            &[(10, 20), (12, 22)],
            &[(10, 20), (11, 21), (12, 22)]
        ));

        // Not subset (missing some)
        assert!(!is_subset(
            &[(10, 20), (13, 23)],
            &[(10, 20), (11, 21), (12, 22)]
        ));
        // Not subset (completely different)
        assert!(!is_subset(&[(50, 50)], &[(10, 20)]));
        // Empty small is always subset
        assert!(is_subset(&[], &[(10, 20)]));
    }
}
