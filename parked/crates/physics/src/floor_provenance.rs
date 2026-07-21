// Copyright 2026 Nathan M. Fraske
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Frozen provenance view for the retired combined physics floor.

use serde::Deserialize;

/// One legacy floor value entry's provenance grade.
#[derive(Debug, Clone, Deserialize)]
pub struct FloorGrade {
    pub id: String,
    pub grade: String,
    #[serde(default)]
    pub derived_from: Vec<String>,
    #[serde(default)]
    pub derive_first_defect: bool,
    #[serde(default)]
    pub unsettled: bool,
    #[serde(default)]
    pub sources: Vec<String>,
}

/// The frozen combined grade register used by parked consumers.
#[derive(Debug, Clone, Deserialize)]
pub struct FloorProvenance {
    #[serde(default, rename = "grade")]
    pub grades: Vec<FloorGrade>,
}

impl FloorProvenance {
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        toml::from_str(s).map_err(|e| e.to_string())
    }

    pub fn embedded() -> Result<Self, String> {
        Self::from_toml_str(include_str!("../data/legacy_floor_provenance.toml"))
    }

    pub fn grade(&self, id: &str) -> Option<&FloorGrade> {
        self.grades.iter().find(|g| g.id == id)
    }

    pub fn authoring_surface(&self) -> Vec<&str> {
        self.grades
            .iter()
            .filter(|g| g.grade == "closure" || g.grade == "authored")
            .map(|g| g.id.as_str())
            .collect()
    }

    pub fn derive_first_defects(&self) -> Vec<&str> {
        self.grades
            .iter()
            .filter(|g| g.derive_first_defect)
            .map(|g| g.id.as_str())
            .collect()
    }
}
