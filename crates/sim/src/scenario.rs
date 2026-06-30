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

//! The scenario loader: read a starting scenario (a test world) from data
//! (`docs/working/TEST_WORLDS.md`, the files in `scenarios/`).
//!
//! A scenario is a profile over the engine's dials, not a separate engine: the same
//! deterministic rules run in every one, and a scenario file sets only what differs (the race
//! posture, the magical physics, and the direction each change-and-extremes dial is pushed).
//! This module parses such a file into a typed [`Scenario`] and exposes its postures and dial
//! directions.
//!
//! A dial direction is a token ([`Direction`]), not a magnitude: `real` (a plausible
//! real-world analogue, the baseline), `high` (cranked toward volatility), or `low` (damped).
//! Resolving a direction to a concrete value needs the reserved magnitudes the owner sets in
//! `calibration/reserved.toml`, so this loader is the parse-and-represent step; layering a
//! scenario's directions over the manifest into a runnable `World` is the next step, gated on
//! those magnitudes being set (the prime directive that the engine never fabricates a value).
//! Magic intensity and several change dials are posture tokens here and become reserved ids as
//! their systems (Part 34 and the change systems) are built.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The direction a scenario pushes a dial: a token, not a magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// A plausible real-world analogue, the baseline.
    Real,
    /// Cranked toward volatility.
    High,
    /// Damped.
    Low,
}

/// The identity and one-line character of a scenario.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ScenarioMeta {
    /// The scenario's stable id (for example "mirror").
    pub id: String,
    /// The scenario's display name.
    pub name: String,
    /// A one-line summary.
    #[serde(default)]
    pub summary: String,
    /// Grounding posture: "real" or "minimal".
    #[serde(default)]
    pub grounding: String,
}

/// The seeded sentient-race posture (design Part 20), as owner-given categorical choices.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct RacePosture {
    /// How many races (for example "few", "several", "many").
    #[serde(default)]
    pub count: String,
    /// How distinct they are (for example "moderate", "high").
    #[serde(default)]
    pub diversity: String,
    /// Whether a mix of magically disposed and non-magical peoples coexist.
    #[serde(default)]
    pub magical_mix: bool,
}

/// The magical-physics posture (design Part 34). `laws` is whether a `MagicLaws` is installed;
/// the intensity fields are posture tokens now and become reserved ids as Part 34 is built.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct MagicPosture {
    /// Whether a `MagicLaws` is installed at all.
    #[serde(default)]
    pub laws: bool,
    /// How potent magic is (a posture token; becomes a reserved id with Part 34).
    #[serde(default)]
    pub potency: Option<String>,
    /// How cheap working magic is.
    #[serde(default)]
    pub cost: Option<String>,
    /// How loose magic's limits are.
    #[serde(default)]
    pub limit_looseness: Option<String>,
    /// What fraction of races carry the imbued magic-affinity channel.
    #[serde(default)]
    pub affinity_fraction: Option<String>,
    /// How strongly affinity is weighted in the genome.
    #[serde(default)]
    pub affinity_weight: Option<String>,
}

/// A parsed scenario: the data-of-record for a test world.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Scenario {
    /// The scenario identity.
    pub scenario: ScenarioMeta,
    /// The race posture.
    #[serde(default)]
    pub races: RacePosture,
    /// The magic posture.
    #[serde(default)]
    pub magic: MagicPosture,
    /// The direction each change-and-extremes dial is pushed, by reserved dial id.
    #[serde(default)]
    pub dials: BTreeMap<String, Direction>,
}

impl Scenario {
    /// Parse a scenario from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Scenario, ScenarioError> {
        toml::from_str(s).map_err(|e| ScenarioError::Parse(e.to_string()))
    }

    /// Load a scenario from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Scenario, ScenarioError> {
        let text = std::fs::read_to_string(path).map_err(|e| ScenarioError::Io(e.to_string()))?;
        Scenario::from_toml_str(&text)
    }

    /// The direction this scenario pushes a dial, or `None` if the scenario does not set it.
    pub fn dial(&self, id: &str) -> Option<Direction> {
        self.dials.get(id).copied()
    }
}

/// A scenario load or parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioError {
    /// The TOML did not parse or did not match the schema.
    Parse(String),
    /// The file could not be read.
    Io(String),
}

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioError::Parse(e) => write!(f, "scenario parse error: {e}"),
            ScenarioError::Io(e) => write!(f, "scenario read error: {e}"),
        }
    }
}

impl std::error::Error for ScenarioError {}

#[cfg(test)]
mod tests {
    use super::*;

    const MIRROR: &str = r#"
[scenario]
id = "mirror"
name = "Mirror"
summary = "grounded baseline"
grounding = "real"

[races]
count = "few"
diversity = "moderate"

[magic]
laws = false
affinity_fraction = "none"

[dials]
"genome.mutation_rates" = "real"
"genome.effective_population_size" = "real"
"#;

    #[test]
    fn a_scenario_parses_its_posture_and_dials() {
        let s = Scenario::from_toml_str(MIRROR).unwrap();
        assert_eq!(s.scenario.id, "mirror");
        assert_eq!(s.scenario.name, "Mirror");
        assert_eq!(s.races.count, "few");
        assert!(!s.magic.laws);
        assert_eq!(s.dial("genome.mutation_rates"), Some(Direction::Real));
        assert_eq!(s.dial("nonexistent.dial"), None);
    }

    #[test]
    fn an_unknown_direction_token_is_a_parse_error() {
        let bad = r#"
[scenario]
id = "x"
name = "X"

[dials]
"genome.mutation_rates" = "extreme"
"#;
        assert!(matches!(
            Scenario::from_toml_str(bad),
            Err(ScenarioError::Parse(_))
        ));
    }

    #[test]
    fn the_four_canonical_scenario_files_load_and_read_as_expected() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scenarios/");
        let mirror = Scenario::load(format!("{dir}mirror.toml")).unwrap();
        assert!(!mirror.magic.laws, "Mirror has no magic");
        assert_eq!(mirror.dial("genome.mutation_rates"), Some(Direction::Real));

        let tempest = Scenario::load(format!("{dir}tempest.toml")).unwrap();
        assert!(!tempest.magic.laws, "Tempest has no magic");
        assert_eq!(
            tempest.dial("genome.mutation_rates"),
            Some(Direction::High),
            "Tempest cranks change high"
        );
        assert_eq!(
            tempest.dial("genome.effective_population_size"),
            Some(Direction::Low),
            "a small Ne drifts harder"
        );

        let arcanum = Scenario::load(format!("{dir}arcanum.toml")).unwrap();
        assert!(arcanum.magic.laws, "Arcanum is magical");
        assert_eq!(arcanum.scenario.grounding, "minimal");

        let confluence = Scenario::load(format!("{dir}confluence.toml")).unwrap();
        assert!(confluence.magic.laws, "Confluence has magic");
        assert!(
            confluence.races.magical_mix,
            "Confluence mixes magical and not"
        );
        assert_eq!(confluence.scenario.grounding, "real");
    }
}
