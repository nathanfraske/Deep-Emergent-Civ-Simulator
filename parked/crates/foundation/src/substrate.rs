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

//! Data-driven substrate definitions and their loader (runbook section 2; design
//! Principle 11, Parts 21, 33, 36, 40).
//!
//! The resolved substrates (the value substrate, the semantic substrate, the
//! institution-function substrate, and their siblings) share a shape: the mechanism
//! is fixed Rust, the membership is data and grows with the world. The runbook says
//! the schema, the loader, and the round-trip tests are buildable now, while the
//! content (the specific axes, leaves, templates) stays data and is inert until the
//! owner provides it.
//!
//! This module is that loader in its general form: a substrate is a set of named
//! definition groups, each a list of identified entries, loaded from TOML and
//! round-tripping without loss. It carries no hardcoded axis, so it is not a closed
//! enum standing where world content should emerge (Principle 4). A concrete
//! substrate (values, drives, semantic primitives) is an instance of this shape,
//! filled from data.

use serde::{Deserialize, Serialize};

/// One named entry in a substrate group, for example a value axis or a drive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Stable identifier within its group.
    pub id: String,
    /// A short description of what the entry means.
    #[serde(default)]
    pub description: String,
}

/// A named group of entries, for example `value_axes` or `drives`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    /// The group name.
    pub name: String,
    /// The entries, in file order.
    #[serde(default)]
    pub entries: Vec<Entry>,
}

/// A data-driven substrate: any number of named groups, each a list of entries.
/// The set of groups is open, so a world can define axes the engine's authors never
/// enumerated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Substrate {
    /// The groups defined in this substrate.
    #[serde(default)]
    pub groups: Vec<Group>,
}

impl Substrate {
    /// An empty substrate.
    pub fn new() -> Self {
        Substrate::default()
    }

    /// Parse a substrate from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        toml::from_str(s).map_err(|e| e.to_string())
    }

    /// Serialize a substrate to TOML text.
    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string(self).map_err(|e| e.to_string())
    }

    /// Load a substrate from a file path.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::from_toml_str(&text)
    }

    /// Find a group by name.
    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.name == name)
    }

    /// The entry ids of a group, in order.
    pub fn entry_ids(&self, group: &str) -> Vec<&str> {
        self.group(group)
            .map(|g| g.entries.iter().map(|e| e.id.as_str()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Substrate {
        Substrate {
            groups: vec![
                Group {
                    name: "value_axes".to_string(),
                    entries: vec![
                        Entry {
                            id: "harm_care".to_string(),
                            description: "weighting of harm against care".to_string(),
                        },
                        Entry {
                            id: "liberty_authority".to_string(),
                            description: String::new(),
                        },
                    ],
                },
                Group {
                    name: "drives".to_string(),
                    entries: vec![Entry {
                        id: "hunger".to_string(),
                        description: "the need to eat".to_string(),
                    }],
                },
            ],
        }
    }

    #[test]
    fn round_trips_through_toml_without_loss() {
        let original = sample();
        let text = original.to_toml_string().unwrap();
        let reloaded = Substrate::from_toml_str(&text).unwrap();
        assert_eq!(original, reloaded, "substrate survived a TOML round trip");
    }

    #[test]
    fn queries_groups_and_entries() {
        let s = sample();
        assert_eq!(
            s.entry_ids("value_axes"),
            vec!["harm_care", "liberty_authority"]
        );
        assert_eq!(s.entry_ids("drives"), vec!["hunger"]);
        assert_eq!(s.entry_ids("absent"), Vec::<&str>::new());
        assert!(s.group("value_axes").is_some());
    }

    #[test]
    fn parses_authored_toml() {
        let toml_text = r#"
[[groups]]
name = "value_axes"
[[groups.entries]]
id = "harm_care"
description = "weighting of harm against care"
[[groups.entries]]
id = "loyalty_betrayal"
"#;
        let s = Substrate::from_toml_str(toml_text).unwrap();
        assert_eq!(
            s.entry_ids("value_axes"),
            vec!["harm_care", "loyalty_betrayal"]
        );
    }
}
