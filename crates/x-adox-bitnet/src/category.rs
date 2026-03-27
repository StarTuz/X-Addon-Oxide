// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
pub enum SceneryCategory {
    #[default]
    Unknown,
    CustomAirport,
    OrbxAirport,
    GlobalAirport,
    Landmark,
    RegionalOverlay,
    RegionalFluff,
    AirportOverlay,
    LowImpactOverlay,
    AutoOrthoOverlay,
    Library,
    OrthoBase,
    GlobalBase,
    SpecificMesh,
    Mesh,
    Group,
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

    /// Returns true if this category is compatible with the given BitNet score.
    pub fn is_compatible_with_score(&self, score: u8) -> bool {
        if matches!(
            self,
            SceneryCategory::Library
                | SceneryCategory::SpecificMesh
                | SceneryCategory::Unknown
                | SceneryCategory::Group
        ) {
            return true;
        }

        match self {
            SceneryCategory::AirportOverlay
            | SceneryCategory::RegionalOverlay
            | SceneryCategory::RegionalFluff
            | SceneryCategory::LowImpactOverlay
            | SceneryCategory::AutoOrthoOverlay => score < 50,
            SceneryCategory::OrthoBase | SceneryCategory::Mesh => score > 35,
            SceneryCategory::CustomAirport | SceneryCategory::OrbxAirport => score <= 13,
            SceneryCategory::GlobalAirport => score == 13,
            SceneryCategory::Landmark => (14..=16).contains(&score),
            SceneryCategory::GlobalBase => score >= 30,
            _ => true,
        }
    }

    /// Returns a "healed" score for this category if a contradiction is detected.
    pub fn heal_score(&self, current_score: u8) -> u8 {
        match self {
            SceneryCategory::CustomAirport | SceneryCategory::OrbxAirport => 11,
            SceneryCategory::GlobalAirport => 13,
            SceneryCategory::Landmark => 14,
            SceneryCategory::AirportOverlay
            | SceneryCategory::RegionalOverlay
            | SceneryCategory::RegionalFluff
            | SceneryCategory::LowImpactOverlay
            | SceneryCategory::AutoOrthoOverlay => 20,
            SceneryCategory::Library => current_score,
            SceneryCategory::GlobalBase => 60,
            SceneryCategory::OrthoBase => 55,
            SceneryCategory::Mesh | SceneryCategory::SpecificMesh => 60,
            SceneryCategory::Unknown | SceneryCategory::Group => current_score,
        }
    }
}
