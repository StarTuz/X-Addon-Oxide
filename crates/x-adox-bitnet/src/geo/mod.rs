// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

pub mod data;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}

impl BoundingBox {
    pub fn new(min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> Self {
        Self {
            min_lat,
            max_lat,
            min_lon,
            max_lon,
        }
    }

    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        lat >= self.min_lat && lat <= self.max_lat && lon >= self.min_lon && lon <= self.max_lon
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub bounds: Vec<BoundingBox>,
    pub parent_id: Option<String>,
}

impl Region {
    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        self.bounds.iter().any(|b| b.contains(lat, lon))
    }
}

use std::sync::Arc;

// Module-level static so that the index HashMaps (built once) are shared
// across all RegionIndex instances and across both lookup sites
// (generate_flight_inner and flight_prompt parser).
static REGION_CACHE: crate::geo::data::CachedRegions = crate::geo::data::CachedRegions::new();

pub struct RegionIndex {
    regions: Arc<Vec<Region>>,
}

impl RegionIndex {
    pub fn new() -> Self {
        Self {
            regions: Arc::clone(REGION_CACHE.get_arc()),
        }
    }

    pub fn find_regions(&self, lat: f64, lon: f64) -> Vec<&Region> {
        self.regions
            .iter()
            .filter(|r| r.contains(lat, lon))
            .collect()
    }

    /// O(1) lookup by region ID (case-insensitive).
    pub fn get_by_id(&self, id: &str) -> Option<&Region> {
        let id_lower = id.to_lowercase();
        REGION_CACHE
            .get_id_index()
            .get(&id_lower)
            .map(|&i| &self.regions[i])
    }

    /// O(1) lookup by region name (case-insensitive exact match).
    pub fn get_by_name(&self, name: &str) -> Option<&Region> {
        let name_lower = name.to_lowercase();
        REGION_CACHE
            .get_name_index()
            .get(&name_lower)
            .map(|&i| &self.regions[i])
    }

    /// Fuzzy search for a region by name/alias.
    /// Steps 1 and 2 are O(1) via the pre-built index; step 3 falls back to O(n).
    pub fn search(&self, query: &str) -> Option<&Region> {
        let q = query.to_lowercase();
        // 1. Exact ID match — O(1)
        if let Some(&i) = REGION_CACHE.get_id_index().get(&q) {
            return Some(&self.regions[i]);
        }
        // 2. Exact Name match — O(1)
        if let Some(&i) = REGION_CACHE.get_name_index().get(&q) {
            return Some(&self.regions[i]);
        }
        // 3. Substring match — O(n), only reached for non-exact queries
        self.regions
            .iter()
            .find(|r| r.name.to_lowercase().contains(&q))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_alaska() {
        let index = RegionIndex::new();
        // Should find "Alaska" by name match
        let r = index.search("Alaska");
        assert!(r.is_some(), "Should find Alaska by name");
        assert_eq!(r.unwrap().id, "US:AK");
    }

    #[test]
    fn test_search_washington_bare() {
        let index = RegionIndex::new();
        // Bare "washington" should resolve to US:WA by exact name match
        let r = index.search("washington");
        assert!(r.is_some(), "Should find Washington State by exact name");
        assert_eq!(r.unwrap().id, "US:WA");
    }
}
