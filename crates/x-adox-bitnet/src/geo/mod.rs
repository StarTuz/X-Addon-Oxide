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

pub struct RegionIndex {
    regions: Vec<Region>,
}

impl RegionIndex {
    pub fn new() -> Self {
        Self {
            regions: data::get_all_regions(),
        }
    }

    pub fn find_regions(&self, lat: f64, lon: f64) -> Vec<&Region> {
        self.regions
            .iter()
            .filter(|r| r.contains(lat, lon))
            .collect()
    }

    pub fn get_by_id(&self, id: &str) -> Option<&Region> {
        self.regions.iter().find(|r| r.id == id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Region> {
        let name_lower = name.to_lowercase();
        self.regions
            .iter()
            .find(|r| r.name.to_lowercase() == name_lower)
    }

    /// Fuzzy search for a region by name/alias
    pub fn search(&self, query: &str) -> Option<&Region> {
        let q = query.to_lowercase();
        // 1. Exact ID match
        if let Some(r) = self.regions.iter().find(|r| r.id.to_lowercase() == q) {
            return Some(r);
        }
        // 2. Exact Name match
        if let Some(r) = self.regions.iter().find(|r| r.name.to_lowercase() == q) {
            return Some(r);
        }
        // 3. Substring match (preference to shorter names / root regions?)
        // For now just return first containing match
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
}
