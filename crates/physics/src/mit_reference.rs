// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The metal-insulator-transition (MIT) reference set (`crates/physics/data/correlation_mit_reference.toml`).
//!
//! The observed metal-versus-insulator classification of the rock-salt divalent 3d monoxides, MEASURED `[M]`
//! (the observed conductivity class, refutable without the sim). The materials correlation classifier calibrates
//! its `U/W` separation threshold against this set: the derived `U/W` must ORDER the set (insulators above
//! metals), and the separation margin is the honesty number. The analog of the ionic lattice-energy validation
//! set for D1. No consumer in any pinned run path yet (byte-neutral).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the MIT reference set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MitReferenceError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A row carries no citation (every classification is real-with-source).
    MissingSource(String),
    /// A row's `observed_class` is neither `insulator` nor `metal`.
    BadClass(String),
}

impl fmt::Display for MitReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MitReferenceError::Parse(m) => write!(f, "mit-reference parse error: {m}"),
            MitReferenceError::MissingSource(m) => {
                write!(f, "mit-reference row without citation: {m}")
            }
            MitReferenceError::BadClass(m) => write!(f, "mit-reference bad observed_class: {m}"),
        }
    }
}

impl std::error::Error for MitReferenceError {}

/// The observed conductivity class of a reference material (the MEASURED classification the classifier calibrates
/// against). The `window` cases (materials at a metal-insulator transition) are deliberately NOT in the
/// reference set: they are the escalation the classifier must produce, not a class it is calibrated on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedClass {
    /// A Mott or charge-transfer insulator (localized d electrons).
    Insulator,
    /// A correlated metal (itinerant d electrons).
    Metal,
}

/// One reference material: its composition and its observed conductivity class.
#[derive(Debug, Clone)]
pub struct MitReferenceMaterial {
    /// The material name (diagnostics only).
    pub name: String,
    /// The composition (element, count).
    pub composition: Vec<(String, u32)>,
    /// The observed conductivity class (the `[M]` datum).
    pub observed_class: ObservedClass,
}

/// The MIT reference set: the observed classifications the correlation classifier calibrates its threshold on.
#[derive(Debug, Clone, Default)]
pub struct MitReference {
    materials: Vec<MitReferenceMaterial>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ReferenceFile {
    #[serde(default)]
    reference: Vec<ReferenceDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ReferenceDef {
    name: String,
    #[serde(default)]
    composition: BTreeMap<String, u32>,
    #[serde(default)]
    observed_class: String,
    #[serde(default)]
    source: String,
}

impl MitReference {
    /// Load the MIT reference set from a TOML string. Every row must carry a citation and a valid class.
    pub fn from_toml_str(s: &str) -> Result<Self, MitReferenceError> {
        let file: ReferenceFile =
            toml::from_str(s).map_err(|e| MitReferenceError::Parse(e.to_string()))?;
        let mut materials = Vec::new();
        for r in file.reference {
            if r.source.trim().is_empty() {
                return Err(MitReferenceError::MissingSource(r.name.clone()));
            }
            let observed_class = match r.observed_class.trim() {
                "insulator" => ObservedClass::Insulator,
                "metal" => ObservedClass::Metal,
                other => return Err(MitReferenceError::BadClass(format!("{}: {other}", r.name))),
            };
            materials.push(MitReferenceMaterial {
                name: r.name,
                composition: r.composition.into_iter().collect(),
                observed_class,
            });
        }
        Ok(MitReference { materials })
    }

    /// The embedded standard reference set (`data/correlation_mit_reference.toml`).
    pub fn standard() -> Result<Self, MitReferenceError> {
        Self::from_toml_str(include_str!("../data/correlation_mit_reference.toml"))
    }

    /// The reference materials.
    pub fn materials(&self) -> &[MitReferenceMaterial] {
        &self.materials
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_set_is_the_rock_salt_monoxides() {
        let r = MitReference::standard().expect("the MIT reference set loads");
        assert_eq!(
            r.materials().len(),
            6,
            "the seed is the 6 rock-salt divalent monoxides"
        );
        let insulators = r
            .materials()
            .iter()
            .filter(|m| m.observed_class == ObservedClass::Insulator)
            .count();
        let metals = r
            .materials()
            .iter()
            .filter(|m| m.observed_class == ObservedClass::Metal)
            .count();
        assert_eq!(insulators, 4, "NiO/CoO/FeO/MnO are the insulators");
        assert_eq!(metals, 2, "TiO/VO are the metals");
    }

    #[test]
    fn a_missing_citation_is_rejected() {
        let bad = r#"
[[reference]]
name = "test oxide"
composition = { Ni = 1, O = 1 }
observed_class = "insulator"
source = ""
"#;
        assert!(matches!(
            MitReference::from_toml_str(bad),
            Err(MitReferenceError::MissingSource(_))
        ));
    }

    #[test]
    fn a_bad_class_is_rejected() {
        let bad = r#"
[[reference]]
name = "test oxide"
composition = { Ni = 1, O = 1 }
observed_class = "superconductor"
source = "test"
"#;
        assert!(matches!(
            MitReference::from_toml_str(bad),
            Err(MitReferenceError::BadClass(_))
        ));
    }
}
