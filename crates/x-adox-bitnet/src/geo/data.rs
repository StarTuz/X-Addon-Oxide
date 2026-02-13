// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use super::{BoundingBox, Region};

pub fn get_all_regions() -> Vec<Region> {
    let mut regions = Vec::new();

    // --- Continents ---
    regions.push(Region {
        id: "EU",
        name: "Europe",
        bounds: vec![BoundingBox::new(34.0, 72.0, -25.0, 45.0)],
        parent_id: None,
    });
    regions.push(Region {
        id: "NA",
        name: "North America",
        bounds: vec![BoundingBox::new(15.0, 85.0, -170.0, -50.0)],
        parent_id: None,
    });
    regions.push(Region {
        id: "SA",
        name: "South America",
        bounds: vec![BoundingBox::new(-60.0, 15.0, -90.0, -30.0)],
        parent_id: None,
    });
    regions.push(Region {
        id: "AF",
        name: "Africa",
        bounds: vec![BoundingBox::new(-40.0, 40.0, -20.0, 55.0)],
        parent_id: None,
    });
    regions.push(Region {
        id: "AS",
        name: "Asia",
        bounds: vec![BoundingBox::new(-10.0, 80.0, 25.0, 180.0)],
        parent_id: None,
    });
    regions.push(Region {
        id: "OC",
        name: "Oceania",
        bounds: vec![BoundingBox::new(-50.0, 0.0, 110.0, 180.0)],
        parent_id: None,
    });

    // --- US States (Selected Major) ---
    // Source: Approximate bounding boxes
    regions.push(Region {
        id: "US",
        name: "United States",
        bounds: vec![
            BoundingBox::new(24.0, 50.0, -125.0, -66.0),  // CONUS
            BoundingBox::new(51.0, 72.0, -170.0, -130.0), // Alaska
            BoundingBox::new(18.0, 23.0, -161.0, -154.0), // Hawaii
        ],
        parent_id: Some("NA"),
    });

    regions.push(Region {
        id: "US:WA",
        name: "Washington",
        bounds: vec![BoundingBox::new(45.5, 49.0, -125.0, -117.0)],
        parent_id: Some("US"),
    });
    regions.push(Region {
        id: "US:OR",
        name: "Oregon",
        bounds: vec![BoundingBox::new(42.0, 46.5, -125.0, -116.5)],
        parent_id: Some("US"),
    });
    regions.push(Region {
        id: "US:CA",
        name: "California",
        bounds: vec![BoundingBox::new(32.5, 42.0, -124.5, -114.0)],
        parent_id: Some("US"),
    });
    regions.push(Region {
        id: "US:SoCal",
        name: "Southern California", // Nickname alias
        bounds: vec![BoundingBox::new(32.5, 36.0, -121.0, -114.0)],
        parent_id: Some("US:CA"),
    });
    regions.push(Region {
        id: "US:NorCal",
        name: "Northern California", // Nickname alias
        bounds: vec![BoundingBox::new(36.0, 42.0, -124.5, -118.0)],
        parent_id: Some("US:CA"),
    });
    regions.push(Region {
        id: "US:TX",
        name: "Texas",
        bounds: vec![BoundingBox::new(25.8, 36.5, -106.6, -93.5)],
        parent_id: Some("US"),
    });
    regions.push(Region {
        id: "US:FL",
        name: "Florida",
        bounds: vec![BoundingBox::new(24.5, 31.0, -87.6, -80.0)],
        parent_id: Some("US"),
    });
    regions.push(Region {
        id: "US:NY",
        name: "New York",
        bounds: vec![BoundingBox::new(40.5, 45.0, -79.8, -71.8)],
        parent_id: Some("US"),
    });
    regions.push(Region {
        id: "US:HI",
        name: "Hawaii",
        bounds: vec![BoundingBox::new(18.5, 22.5, -161.0, -154.5)],
        parent_id: Some("US"),
    });
    regions.push(Region {
        id: "US:AK",
        name: "Alaska",
        bounds: vec![BoundingBox::new(51.0, 72.0, -170.0, -130.0)],
        parent_id: Some("US"),
    });

    // --- Major European Countries ---
    regions.push(Region {
        id: "UK",
        name: "United Kingdom",
        bounds: vec![BoundingBox::new(49.9, 59.4, -8.2, 1.8)],
        parent_id: Some("EU"),
    });
    regions.push(Region {
        id: "GB",
        name: "Great Britain",
        // Exclude Northern Ireland (approx -8.2 to -5.5)
        bounds: vec![BoundingBox::new(49.9, 59.4, -5.5, 1.8)],
        parent_id: Some("UK"),
    });
    regions.push(Region {
        id: "FR",
        name: "France",
        bounds: vec![BoundingBox::new(41.3, 51.1, -5.2, 9.6)],
        parent_id: Some("EU"),
    });
    regions.push(Region {
        id: "DE",
        name: "Germany",
        bounds: vec![BoundingBox::new(47.2, 55.1, 5.8, 15.1)],
        parent_id: Some("EU"),
    });
    regions.push(Region {
        id: "IT",
        name: "Italy",
        bounds: vec![BoundingBox::new(36.6, 47.1, 6.6, 18.6)],
        parent_id: Some("EU"),
    });
    regions.push(Region {
        id: "ES",
        name: "Spain",
        bounds: vec![BoundingBox::new(36.0, 43.8, -9.4, 3.4)],
        parent_id: Some("EU"),
    });
    regions.push(Region {
        id: "IE",
        name: "Ireland",
        bounds: vec![BoundingBox::new(51.4, 55.4, -10.5, -6.0)],
        parent_id: Some("EU"), // Or BI if we want BI to be a parent, but usually EU is the parent in this schema
    });
    regions.push(Region {
        id: "BI",
        name: "British Isles",
        bounds: vec![BoundingBox::new(49.9, 59.4, -10.5, 1.8)],
        parent_id: Some("EU"),
    });

    // --- Other Major Countries ---
    regions.push(Region {
        id: "CA",
        name: "Canada",
        bounds: vec![BoundingBox::new(41.7, 83.1, -141.0, -52.6)],
        parent_id: Some("NA"),
    });
    regions.push(Region {
        id: "AU",
        name: "Australia",
        bounds: vec![BoundingBox::new(-44.0, -10.0, 112.0, 154.0)],
        parent_id: Some("OC"),
    });
    regions.push(Region {
        id: "JP",
        name: "Japan",
        bounds: vec![BoundingBox::new(24.0, 46.0, 122.0, 146.0)],
        parent_id: Some("AS"),
    });
    regions.push(Region {
        id: "CN",
        name: "China",
        bounds: vec![BoundingBox::new(18.0, 54.0, 73.0, 135.0)],
        parent_id: Some("AS"),
    });
    regions.push(Region {
        id: "BR",
        name: "Brazil",
        bounds: vec![BoundingBox::new(-34.0, 5.5, -74.0, -34.0)],
        parent_id: Some("SA"),
    });

    regions
}
