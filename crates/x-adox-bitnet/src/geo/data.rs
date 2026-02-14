use super::Region;
use serde_json;

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
