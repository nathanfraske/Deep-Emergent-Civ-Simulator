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
//! [`Scenario::resolve`] layers a scenario over the calibration manifest, the bulk lever: it maps
//! each dial to the manifest entry behind its direction (the base id for `real`, a `.high`/`.low`
//! sibling for a pushed dial) and surfaces which magnitudes the world still needs the owner to set
//! ([`ScenarioResolution::reserved_ids`]). It never reads a magnitude, so it never fabricates one:
//! a still-reserved dial stays reserved and is carried into the review queue. Instantiating a
//! runnable `World` from the resolved values is the next step, gated on those magnitudes being set
//! (the prime directive that the engine never fabricates a value). Magic intensity and several
//! change dials are posture tokens here and become reserved ids as their systems (Part 34 and the
//! change systems) are built.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::calibration::{CalibrationError, CalibrationManifest, ReservedValue};

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
    /// The ambient medium a world's life breathes and floats in, selected categorically by
    /// name (for example "water", "dense_toxic"): a physics `Substance`, resolved to the
    /// manifest medium profile `medium.{name}`. Unlike the field and thermal-band levers this
    /// is not a `real`/`high`/`low` dial, because a medium is a coherent bundle (water is dense
    /// and low in dissolved gas together) rather than independent axes. `None` is the default
    /// temperate air.
    #[serde(default)]
    pub medium: Option<String>,
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
    /// The direction each environmental lever is pushed, by reserved id: the temperature
    /// field (`field.*`) and the per-race thermal band (`physiology.thermal_*`) that make a
    /// world hot, cold, or temperate. Resolved by the same mechanism as `dials`; kept in its
    /// own block so a world's environment reads apart from its change-engine dials.
    #[serde(default)]
    pub environment: BTreeMap<String, Direction>,
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

    /// The direction this scenario pushes an environmental lever, or `None` if unset.
    pub fn environment_dial(&self, id: &str) -> Option<Direction> {
        self.environment.get(id).copied()
    }

    /// Resolve this scenario against a base calibration manifest: the bulk lever. Pull one
    /// scenario and its whole dial set resolves at once, each dial to the manifest entry behind
    /// its direction ([`dial_manifest_id`]). Every resolved id must exist in the manifest, so a
    /// dial the manifest cannot satisfy (a missing direction sibling, a typo) fails loud as a
    /// dangling reference rather than silently doing nothing. This never reads a magnitude and so
    /// never fails on a reserved value: a still-reserved dial is carried through and surfaced in
    /// the resolution's review queue ([`ScenarioResolution::reserved_ids`]), the correct output
    /// under the prime directive that the engine never fabricates a value.
    pub fn resolve(
        &self,
        manifest: &CalibrationManifest,
    ) -> Result<ScenarioResolution, CalibrationError> {
        let mut dials = Vec::with_capacity(self.dials.len() + self.environment.len());
        // The change-and-extremes dials and the environmental levers resolve by the same
        // mechanism (direction to manifest id); both are carried in the resolution's review
        // queue, so a world's environment is levered and calibration-gated exactly like its
        // change dials, and a dangling environment reference fails loud the same way.
        for (dial, direction) in self.dials.iter().chain(self.environment.iter()) {
            let manifest_id = dial_manifest_id(dial, *direction);
            let entry = manifest
                .get(&manifest_id)
                .ok_or_else(|| CalibrationError::Unknown(manifest_id.clone()))?
                .clone();
            dials.push(ResolvedDial {
                dial: dial.clone(),
                direction: *direction,
                manifest_id,
                entry,
            });
        }
        // The ambient medium is selected by name, so it resolves to the manifest profile
        // `medium.{name}` (a require_map bundle of axis values) rather than through a direction.
        // A world naming a medium the manifest has no profile for fails loud like a dangling dial.
        let medium = match &self.scenario.medium {
            Some(name) => {
                let manifest_id = format!("medium.{name}");
                let entry = manifest
                    .get(&manifest_id)
                    .ok_or_else(|| CalibrationError::Unknown(manifest_id.clone()))?
                    .clone();
                Some(ResolvedMedium {
                    name: name.clone(),
                    manifest_id,
                    entry,
                })
            }
            None => None,
        };
        Ok(ScenarioResolution {
            scenario: self.scenario.id.clone(),
            dials,
            medium,
        })
    }
}

/// The calibration-manifest id a dial resolves to under a direction: the base id for `real`, and
/// the `.high`/`.low` sibling for a pushed dial. The magnitude behind that id stays the owner's
/// reserved value; this is the wiring from a scenario's direction token to a manifest entry, never
/// a magnitude. A dial pushed the same direction by two worlds resolves to the same sibling, which
/// is the [`Direction`] abstraction: `high` is one reserved end, shared, not a per-world number.
pub fn dial_manifest_id(dial: &str, direction: Direction) -> String {
    match direction {
        Direction::Real => dial.to_string(),
        Direction::High => format!("{dial}.high"),
        Direction::Low => format!("{dial}.low"),
    }
}

/// One dial resolved against the manifest: the scenario dial id, the direction the scenario pushes
/// it, the manifest id that direction resolves to, and the entry found there (set or reserved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDial {
    /// The scenario dial id (the base, for example `genome.mutation_rates`).
    pub dial: String,
    /// The direction this scenario pushes it.
    pub direction: Direction,
    /// The manifest id the direction resolves to (the base id, or a `.high`/`.low` sibling).
    pub manifest_id: String,
    /// The manifest entry found at `manifest_id`, set or still reserved.
    pub entry: ReservedValue,
}

/// The manifest profile id of the default temperate-air ambient medium, the medium a scenario that
/// names none defaults to ([`ScenarioMeta::medium`] is `None`). A label pointer to the owner's
/// reserved `medium.air` physics profile, never a magnitude: the world-build path derives an
/// air-default world's field diffusion from air's real k/rho/c exactly as it derives a water world's
/// from water's, so no world reads a free diffusion scalar (design Part 5.4/5.5).
pub const DEFAULT_MEDIUM_ID: &str = "medium.air";

/// A scenario's ambient medium, resolved to its manifest profile: the categorical sibling of
/// [`ResolvedDial`], since a medium is selected by name rather than pushed by a direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMedium {
    /// The medium name the scenario selected (for example "water").
    pub name: String,
    /// The manifest id it resolves to (`medium.{name}`, a `require_map` profile of axis values).
    pub manifest_id: String,
    /// The manifest entry found at `manifest_id`, set or still reserved.
    pub entry: ReservedValue,
}

/// A scenario resolved against a base manifest: the whole override set the scenario pulls, with
/// each dial mapped to its manifest entry. The caller reads the set dials and takes the reserved
/// ones to the owner ([`reserved_ids`](Self::reserved_ids)), the per-scenario review queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioResolution {
    /// The scenario id this resolution is for.
    pub scenario: String,
    /// Each dial the scenario pushes, resolved to its manifest entry, in dial-id order.
    pub dials: Vec<ResolvedDial>,
    /// The scenario's ambient medium, resolved to its manifest profile, or `None` for the
    /// default temperate air.
    pub medium: Option<ResolvedMedium>,
}

impl ScenarioResolution {
    /// The manifest ids for the `[dials]` this scenario pushes that the owner has not set yet: the
    /// per-scenario dial review queue. This scopes to the change-and-extremes dials only; a world's
    /// magic-intensity and race postures ([`MagicPosture`], [`RacePosture`]) are not manifest-backed
    /// yet and are resolved separately once Part 34 lands, so an empty queue here means the dials are
    /// set, not that a magical world is fully calibrated.
    pub fn reserved_ids(&self) -> Vec<&str> {
        self.dials
            .iter()
            .filter(|d| !d.entry.is_set())
            .map(|d| d.manifest_id.as_str())
            .chain(
                self.medium
                    .iter()
                    .filter(|m| !m.entry.is_set())
                    .map(|m| m.manifest_id.as_str()),
            )
            .collect()
    }

    /// Whether every `[dials]` entry this scenario pushes has a set magnitude. This covers the
    /// change-and-extremes dials only, not the magic-intensity or race postures (not manifest-backed
    /// until Part 34), so it is necessary but not sufficient for a magical world to run under
    /// [`Profile::Calibrated`](crate::calibration::Profile); a grounded world (Mirror) with no magic
    /// postures is fully calibrated when this holds.
    pub fn is_fully_set(&self) -> bool {
        self.dials.iter().all(|d| d.entry.is_set())
            && self.medium.as_ref().is_none_or(|m| m.entry.is_set())
    }

    /// The manifest profile id of this scenario's ambient medium: the resolved medium's profile id
    /// (`medium.{name}`), or the default temperate-air profile ([`DEFAULT_MEDIUM_ID`]) when the
    /// scenario names no medium. A scenario that selects no medium is the documented default temperate
    /// air ([`ScenarioMeta::medium`] is `None`), and air is a real physics profile with its own thermal
    /// axes rather than a fabricated field default, so the world-build path reads this to derive the
    /// field's diffusion coefficient from the medium's physics (`k/(rho*c)`) for every world, the
    /// medium-named ones and the air-default ones alike. This is a label pointer to a manifest profile,
    /// never a magnitude: the profile behind it stays the owner's reserved value.
    pub fn medium_manifest_id(&self) -> &str {
        self.medium
            .as_ref()
            .map_or(DEFAULT_MEDIUM_ID, |m| m.manifest_id.as_str())
    }

    /// The manifest id a given base dial resolves to under this scenario, or `None` if the scenario
    /// does not push that dial.
    pub fn manifest_id(&self, dial: &str) -> Option<&str> {
        self.dials
            .iter()
            .find(|d| d.dial == dial)
            .map(|d| d.manifest_id.as_str())
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

    const RESOLVE_MANIFEST: &str = r#"
[[reserved]]
id = "genome.mutation_rates"
basis = "b"
status = "reserved"
value = ""
unit = "x"
source = "s"
[[reserved]]
id = "genome.mutation_rates.high"
basis = "b"
status = "set"
value = "0.01"
unit = "x"
source = "s"
[[reserved]]
id = "genome.effective_population_size"
basis = "b"
status = "set"
value = "200"
unit = "individuals"
source = "s"
[[reserved]]
id = "genome.effective_population_size.low"
basis = "b"
status = "reserved"
value = ""
unit = "individuals"
source = "s"
"#;

    #[test]
    fn a_direction_resolves_to_the_base_or_a_sibling_id() {
        assert_eq!(
            dial_manifest_id("genome.mutation_rates", Direction::Real),
            "genome.mutation_rates"
        );
        assert_eq!(
            dial_manifest_id("genome.mutation_rates", Direction::High),
            "genome.mutation_rates.high"
        );
        assert_eq!(
            dial_manifest_id("genome.effective_population_size", Direction::Low),
            "genome.effective_population_size.low"
        );
    }

    #[test]
    fn resolving_maps_each_dial_to_its_manifest_entry_and_queues_the_reserved_ones() {
        let manifest = CalibrationManifest::from_toml_str(RESOLVE_MANIFEST).unwrap();
        let scenario = Scenario::from_toml_str(
            r#"
[scenario]
id = "probe"
name = "Probe"

[dials]
"genome.mutation_rates" = "high"
"genome.effective_population_size" = "low"
"#,
        )
        .unwrap();
        let r = scenario.resolve(&manifest).unwrap();
        // The high dial resolves to its set sibling; the low dial to its reserved sibling.
        assert_eq!(
            r.manifest_id("genome.mutation_rates"),
            Some("genome.mutation_rates.high")
        );
        assert_eq!(
            r.manifest_id("genome.effective_population_size"),
            Some("genome.effective_population_size.low")
        );
        // The reserved sibling is the per-scenario review queue; the set one is not in it.
        assert_eq!(
            r.reserved_ids(),
            vec!["genome.effective_population_size.low"]
        );
        assert!(!r.is_fully_set(), "one dial is still reserved");
    }

    #[test]
    fn a_dial_with_no_manifest_sibling_fails_loud_rather_than_doing_nothing() {
        let manifest = CalibrationManifest::from_toml_str(RESOLVE_MANIFEST).unwrap();
        // The manifest has no `genome.mutation_step.high` sibling, so pushing it high is a dangling
        // reference the resolver must reject, not silently drop.
        let scenario = Scenario::from_toml_str(
            r#"
[scenario]
id = "probe"
name = "Probe"

[dials]
"genome.mutation_step" = "high"
"#,
        )
        .unwrap();
        assert_eq!(
            scenario.resolve(&manifest).unwrap_err(),
            CalibrationError::Unknown("genome.mutation_step.high".to_string())
        );
    }

    #[test]
    fn every_canonical_scenario_resolves_against_the_real_manifest() {
        // The bulk lever's completeness guarantee: every dial each world pushes maps to a real
        // manifest entry (a base id or a direction sibling), so pulling a scenario lever surfaces a
        // defined reserved value for every dial rather than a dangling reference. This keeps
        // scenarios/*.toml and calibration/reserved.toml in step. Covers the four canonical worlds
        // and the three new variants (Venus, Europa, Crucible); Europa's placid low dials resolve to
        // the reserved .low genome and drift siblings surfaced for it.
        let manifest = CalibrationManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../calibration/reserved.toml"
        ))
        .unwrap();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scenarios/");
        for world in [
            "mirror",
            "tempest",
            "arcanum",
            "confluence",
            "venus",
            "europa",
            "crucible",
        ] {
            let scenario = Scenario::load(format!("{dir}{world}.toml")).unwrap();
            let r = scenario
                .resolve(&manifest)
                .unwrap_or_else(|e| panic!("{world} has a dangling dial: {e}"));
            assert_eq!(
                r.dials.len(),
                scenario.dials.len() + scenario.environment.len(),
                "{world} resolves every dial and environment lever it pushes"
            );
        }
        // Tempest cranks change, so its direction siblings are the stress-world ends. As the owner
        // graduates them through the reserved-values worksheet the review queue shrinks, so this
        // asserts the MECHANISM rather than a fixed membership, and survives calibration: the review
        // queue surfaces exactly the dials Tempest pushes whose resolved manifest entry is still
        // unset, and is_fully_set is precisely that queue being empty.
        let tempest = Scenario::load(format!("{dir}tempest.toml")).unwrap();
        let queue = tempest.resolve(&manifest).unwrap();
        let reserved = queue.reserved_ids();
        for &id in &reserved {
            assert!(
                !manifest.get(id).unwrap().is_set(),
                "the review queue must surface only unset dials, but {id} is set"
            );
        }
        assert_eq!(
            queue.is_fully_set(),
            reserved.is_empty(),
            "a scenario is fully set exactly when its review queue is empty"
        );
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

    #[test]
    fn the_environment_block_levers_the_field_and_thermal_band() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scenarios/");
        let manifest = CalibrationManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../calibration/reserved.toml"
        ))
        .unwrap();

        // Venus is levered hot: a dense diffusive field, fast body coupling, and a
        // heat-shifted, widened thermal band.
        let venus = Scenario::load(format!("{dir}venus.toml")).unwrap();
        assert_eq!(
            venus.environment_dial("field.body_exchange"),
            Some(Direction::High),
            "the dense hot medium couples the body fast"
        );
        assert_eq!(
            venus.environment_dial("physiology.thermal_setpoint"),
            Some(Direction::High),
            "a heat-shifted thermophile core"
        );

        // Europa is levered cold and buffered: near-static relaxation under ice, a cold
        // narrow-banded specialist.
        let europa = Scenario::load(format!("{dir}europa.toml")).unwrap();
        assert_eq!(
            europa.environment_dial("field.relaxation"),
            Some(Direction::Low),
            "near-static under lightless ice"
        );
        assert_eq!(
            europa.environment_dial("physiology.thermal_half_band"),
            Some(Direction::Low),
            "a narrow band in a stable cold ocean"
        );

        // Both environments resolve against the real manifest: every environment lever maps
        // to a defined (reserved) entry, so the two worlds are levered and calibration-gated,
        // and the resolution surfaces the environment magnitudes in its review queue.
        let r = europa.resolve(&manifest).unwrap();
        assert_eq!(r.dials.len(), europa.dials.len() + europa.environment.len());
        let reserved = r.reserved_ids();
        assert!(
            reserved.contains(&"physiology.thermal_setpoint.low"),
            "Europa's cold set point is surfaced as a reserved environment magnitude"
        );

        // The four canonical worlds carry no environment block: temperate is the unlevered
        // baseline, so their resolution is unchanged.
        let mirror = Scenario::load(format!("{dir}mirror.toml")).unwrap();
        assert!(
            mirror.environment.is_empty(),
            "Mirror is the temperate baseline"
        );
    }

    #[test]
    fn the_medium_selection_resolves_to_a_manifest_profile() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scenarios/");
        let manifest = CalibrationManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../calibration/reserved.toml"
        ))
        .unwrap();

        // Europa breathes cold water; the medium resolves to its manifest profile, which is
        // reserved (surfaced for the owner), so it is carried in the review queue.
        let europa = Scenario::load(format!("{dir}europa.toml")).unwrap();
        assert_eq!(europa.scenario.medium.as_deref(), Some("water"));
        let r = europa.resolve(&manifest).unwrap();
        let med = r.medium.as_ref().expect("Europa selects a medium");
        assert_eq!(med.manifest_id, "medium.water");
        assert!(r.reserved_ids().contains(&"medium.water"));

        // Venus breathes a dense toxic atmosphere.
        let venus = Scenario::load(format!("{dir}venus.toml")).unwrap();
        assert_eq!(
            venus
                .resolve(&manifest)
                .unwrap()
                .medium
                .unwrap()
                .manifest_id,
            "medium.dense_toxic"
        );

        // The canonical worlds name no medium: temperate air is the default.
        let mirror = Scenario::load(format!("{dir}mirror.toml")).unwrap();
        assert!(mirror.scenario.medium.is_none(), "Mirror defaults to air");
        assert!(mirror.resolve(&manifest).unwrap().medium.is_none());

        // A world naming a medium the manifest has no profile for fails loud, like a dangling
        // dial, rather than silently defaulting.
        let bogus =
            Scenario::from_toml_str("[scenario]\nid = \"b\"\nname = \"B\"\nmedium = \"plasma\"\n")
                .unwrap();
        assert!(bogus.resolve(&manifest).is_err());
    }

    #[test]
    fn the_three_new_scenario_files_load_and_read_as_expected() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scenarios/");

        // Venus: super-hot toxic world, a thin thread of magic, harsh selection on small pools.
        let venus = Scenario::load(format!("{dir}venus.toml")).unwrap();
        assert!(venus.magic.laws, "Venus installs a thin MagicLaws");
        assert!(venus.races.magical_mix, "Venus mixes magical and not");
        assert_eq!(
            venus.dial("genome.selection_scaling"),
            Some(Direction::High),
            "a lethal world selects hard"
        );
        assert_eq!(
            venus.dial("genome.effective_population_size"),
            Some(Direction::Low),
            "small cloud-deck pools drift"
        );

        // Europa: placid ocean world, no magic, low drift, long pre-dawn.
        let europa = Scenario::load(format!("{dir}europa.toml")).unwrap();
        assert!(!europa.magic.laws, "Europa has no magic");
        assert_eq!(
            europa.dial("genome.mutation_rates"),
            Some(Direction::Low),
            "a buffered ocean drifts slowly"
        );
        assert_eq!(
            europa.dial("biosphere.predawn_generations"),
            Some(Direction::High),
            "a vent ecology needs deep time to radiate"
        );

        // Crucible: war as the emergent equilibrium of scarcity and divergence.
        let crucible = Scenario::load(format!("{dir}crucible.toml")).unwrap();
        assert!(crucible.magic.laws, "Crucible carries a scarce war-magic");
        assert_eq!(crucible.races.count, "many", "many peoples, few zones");
        assert_eq!(
            crucible.dial("value_metric.conflict_coefficient"),
            Some(Direction::High),
            "scarce contested range earns a high conflict coefficient"
        );
        assert_eq!(
            crucible.dial("genome.speciation_distance"),
            Some(Direction::Low),
            "fast radiation into many distinct peoples"
        );
        // The mutation clock stays real: the extremity is social and environmental, not a churned
        // genome, which is what distinguishes Crucible from Tempest.
        assert_eq!(
            crucible.dial("genome.mutation_rates"),
            Some(Direction::Real),
            "Crucible does not crank the mutation clock"
        );
    }
}
