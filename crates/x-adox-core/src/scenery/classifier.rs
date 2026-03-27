// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use crate::scenery::SceneryCategory;
use std::path::Path;
use x_adox_bitnet::{BitNetModel, PredictContext};

pub struct Classifier;

impl Classifier {
    /// Integrated classification using BitNet. 
    /// This is the primary entry point for all scenery classification.
    pub fn classify(
        name: &str,
        path: &Path,
        context: &PredictContext,
        model: &BitNetModel,
    ) -> SceneryCategory {
        let (_, category, _) = model.predict_with_rule_name(name, path, context);
        category
    }

    /// Legacy fallback classification if no model is available.
    /// Uses a default BitNet model which contains all standard heuristics.
    pub fn classify_default(name: &str, context: &PredictContext) -> SceneryCategory {
        let model = BitNetModel::default();
        let (_, category, _) = model.predict_with_rule_name(name, Path::new(name), context);
        category
    }

    /// Optimized "name-only" heuristic for fast UI checks.
    pub fn classify_by_name_only(name: &str) -> SceneryCategory {
        let model = BitNetModel::default();
        let context = PredictContext::default();
        let (_, category, _) = model.predict_with_rule_name(name, Path::new(name), &context);
        category
    }
}
