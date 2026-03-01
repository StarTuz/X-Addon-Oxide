use super::Region;
use serde_json;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

pub struct CachedRegions {
    inner: OnceLock<Arc<Vec<Region>>>,
    /// lowercase id → index into the regions Vec
    id_index: OnceLock<HashMap<String, usize>>,
    /// lowercase name → index into the regions Vec
    name_index: OnceLock<HashMap<String, usize>>,
}

impl Default for CachedRegions {
    fn default() -> Self {
        Self::new()
    }
}

impl CachedRegions {
    pub const fn new() -> Self {
        Self {
            inner: OnceLock::new(),
            id_index: OnceLock::new(),
            name_index: OnceLock::new(),
        }
    }

    pub fn get_arc(&self) -> &Arc<Vec<Region>> {
        self.inner.get_or_init(|| {
            Arc::new(
                serde_json::from_str(include_str!("regions.json"))
                    .expect("Failed to parse regions.json"),
            )
        })
    }

    pub fn get(&self) -> &Vec<Region> {
        self.get_arc().as_ref()
    }

    /// Returns a HashMap of lowercase region ID → index (built once on first call).
    pub fn get_id_index(&self) -> &HashMap<String, usize> {
        self.id_index.get_or_init(|| {
            self.get()
                .iter()
                .enumerate()
                .map(|(i, r)| (r.id.to_lowercase(), i))
                .collect()
        })
    }

    /// Returns a HashMap of lowercase region name → index (built once on first call).
    pub fn get_name_index(&self) -> &HashMap<String, usize> {
        self.name_index.get_or_init(|| {
            self.get()
                .iter()
                .enumerate()
                .map(|(i, r)| (r.name.to_lowercase(), i))
                .collect()
        })
    }
}

pub fn get_all_regions() -> Vec<Region> {
    serde_json::from_str(include_str!("regions.json")).expect("Failed to parse regions.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_loading() {
        let regions = get_all_regions();
        assert!(!regions.is_empty(), "Regions list should not be empty");

        // specific checks
        let us = regions
            .iter()
            .find(|r| r.id == "US")
            .expect("US region missing");
        assert_eq!(us.name, "United States");

        let nl = regions
            .iter()
            .find(|r| r.id == "NL")
            .expect("Netherlands missing");
        assert_eq!(nl.name, "Netherlands");
        assert_eq!(nl.parent_id, Some("EU".to_string()));

        let alps = regions
            .iter()
            .find(|r| r.id == "Alps")
            .expect("Alps missing");
        assert!(!alps.bounds.is_empty());

        let unk = regions.iter().find(|r| r.id == "ZZZ");
        assert!(unk.is_none());
    }
}
