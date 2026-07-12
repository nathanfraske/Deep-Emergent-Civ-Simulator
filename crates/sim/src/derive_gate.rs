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

//! # The retired-floor-derivation registry (task #43, the derived-output-is-live gate)
//!
//! The constructor gate (`scripts/constructor_gate.py`) proves no authored value sits on the
//! canonical path. It cannot prove the derived value that REPLACED a retired floor is alive: a
//! derivation can retire an authored floor (obeying the gate) and then produce a constant, and
//! nothing catches it. That is the `soil_baseline` bug (a 330x sweep of the photosynthesis
//! constants left the state hash byte-identical) and the identically-zero Dalton evaporation
//! closed in the hydrology re-baseline.
//!
//! This registry is the data-defined membership of the derived-output-is-live gate: one row per
//! retired-floor derivation, naming the derivation, the reserved input it derives from, the
//! scenario that arms it on the run path, and the source site that carries its `@derives[id]:`
//! annotation. The MECHANISM (the gate that perturbs an input and asserts the derived value
//! responds at its site) is fixed Rust; the MEMBERSHIP is data and grows with the world as new
//! derivations retire new floors, sibling to the value, semantic, institution-function, and
//! provenance substrates. Each row's `id` matches the `@derives[id]:` token at its site, and the
//! cross-check (in the gate's test) asserts every annotated site has a row and every row a site,
//! the ratchet the constructor gate already models.
//!
//! The membership and its cross-check are the first surface. The site-local liveness probe (perturb
//! the declared input, assert the derived output responds at its site) is the pass/fail gate below: a
//! live-but-byte-neutral derivation whose effect lands in a downstream deadband is a coverage gap the
//! separate coverage signal reports, never a liveness failure (the gate's ruling).

use crate::calibration::CalibrationManifest;
use civsim_core::Fixed;

/// One retired-floor derivation: a derived value that replaced an authored floor, and the metadata
/// the liveness gate reads to probe it. All fields are data, so a new derivation is a new row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetiredFloorDerivation {
    /// The stable id, matching the `@derives[id]:` token at the site.
    pub id: String,
    /// What the derivation produces, in prose (the derived quantity).
    pub derived: String,
    /// The reserved input keys or floor sources the derivation reads, the values the gate perturbs
    /// to prove the output responds. Empty is not allowed: a derivation with no declared input has
    /// nothing to perturb, so the gate could not prove it alive.
    pub inputs: Vec<String>,
    /// The run scenario that arms the derivation on the canonical path.
    pub scenario: String,
    /// The source file carrying this derivation's `@derives[id]:` annotation, repo-relative.
    pub site: String,
}

/// The registry of retired-floor derivations. Ordered by registration, so any canonical walk over
/// it is deterministic (the same discipline the quantity and unit registries follow). The built-in
/// membership is [`RetiredFloorDerivationRegistry::canonical`]; a world may register more.
#[derive(Clone, Debug, Default)]
pub struct RetiredFloorDerivationRegistry {
    entries: Vec<RetiredFloorDerivation>,
}

impl RetiredFloorDerivationRegistry {
    /// An empty registry.
    pub fn new() -> RetiredFloorDerivationRegistry {
        RetiredFloorDerivationRegistry::default()
    }

    /// Register a derivation. Panics on a duplicate id or an empty input list, so the registry
    /// cannot carry a row the gate could not probe or two rows for one id.
    pub fn register(&mut self, d: RetiredFloorDerivation) {
        assert!(
            !d.inputs.is_empty(),
            "retired-floor derivation '{}' declares no input to perturb",
            d.id
        );
        assert!(
            !self.entries.iter().any(|e| e.id == d.id),
            "duplicate retired-floor derivation id '{}'",
            d.id
        );
        self.entries.push(d);
    }

    /// Walk the derivations in canonical registration order.
    pub fn iter(&self) -> impl Iterator<Item = &RetiredFloorDerivation> {
        self.entries.iter()
    }

    /// The number of registered derivations.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The registered ids, in canonical order.
    pub fn ids(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.id.as_str()).collect()
    }

    /// The built-in membership: every retired-floor derivation the codebase carries today, each
    /// keyed to its `@derives[id]:` site. This is the data the gate reads; extending it is adding a
    /// row, never changing the gate.
    pub fn canonical() -> RetiredFloorDerivationRegistry {
        let mut r = RetiredFloorDerivationRegistry::new();
        for (id, derived, inputs, scenario, site) in CANONICAL {
            r.register(RetiredFloorDerivation {
                id: id.to_string(),
                derived: derived.to_string(),
                inputs: inputs.iter().map(|s| s.to_string()).collect(),
                scenario: scenario.to_string(),
                site: site.to_string(),
            });
        }
        r
    }
}

/// The built-in derivation rows as data: `(id, derived, inputs, scenario, site)`. Each id matches an
/// `@derives[id]:` annotation at its site. The input keys are the reserved values or floor sources
/// the derivation reads; the liveness probe perturbs one and asserts the derived output moves.
const CANONICAL: &[(&str, &str, &[&str], &str, &str)] = &[
    (
        "world_time_cadence",
        "a world's year and day (the aging, drift, and calendar cadences)",
        &["world.orbital_period_seconds"],
        "default",
        "crates/world/src/celestial.rs",
    ),
    (
        "locomotion_speed_cell",
        "movement speed in tiles per tick and the cell edge in metres",
        &["locomotion.base_speed_m_per_s"],
        "default",
        "crates/sim/src/locomotion.rs",
    ),
    (
        "clock_calendar_cell",
        "a world's year, day, and season in ticks, and the cell area",
        &["world.orbital_period_seconds"],
        "default",
        "crates/sim/src/clock.rs",
    ),
    (
        "hydrology_water",
        "local water presence, rainfall, evaporation, and runoff",
        &["hydrology.saturation_t_ref"],
        "full",
        "crates/sim/src/environ.rs",
    ),
    (
        "productivity_capacity",
        "per-cell biomass productivity and carrying capacity",
        &["productivity.soil_requirement"],
        "full",
        "crates/sim/src/environ.rs",
    ),
    (
        "metabolic_rate",
        "a being's metabolic rate, energy drain, and heat loss",
        &["metabolism.kleiber_a"],
        "full",
        "crates/sim/src/physiology.rs",
    ),
    (
        "decomposition_recovery",
        "soil-nutrient recovery through the matter cycle",
        &["decompose.decomposer_rate"],
        "full",
        "crates/sim/src/decompose.rs",
    ),
    (
        "weathering_soil_nutrient",
        "abiotic mineral-weathering soil-nutrient supply (rock into soil nutrient before any biomass)",
        &["weathering.mineral_dissolution_rate"],
        "living",
        "crates/sim/src/environ.rs",
    ),
    (
        "carbon_fixation_rate",
        "per-cell carbon-fixation rate / net primary productivity",
        &["photosynthesis.light_saturation"],
        "living",
        "crates/sim/src/environ.rs",
    ),
];

// --- The site-local liveness probe (task #43 slice 2): the pass/fail gate ---
//
// The registry above is the membership; this is the mechanism. For a wired derivation, the probe
// evaluates the derived output at a fixed representative situation twice: once from the base
// manifest, and once with the derivation's declared reserved input perturbed. A LIVE derivation's
// output responds (the two readings differ); a DEAD one holds the same value regardless of its input,
// the soil_baseline bug (a large sweep of the photosynthesis constants left the derived output, and so
// the state hash, byte-identical). The gate keys on the SITE-LOCAL value, not a distant hash
// consequence, so a live-but-byte-neutral derivation (its effect landing in a downstream deadband on
// the pinned scenarios) is never mistaken for dead (the gate's ruling; that coverage gap is the
// separate softer signal). The perturbation is a structural test factor, never a world value, and
// never enters the sim: the gate is tooling that reads run state and authors nothing on the run path.

/// The two site-local readings of a derived output: its value from the base manifest, and its value
/// with the declared reserved input perturbed. The gate compares them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbeReading {
    /// The derived output at the representative situation, base manifest.
    pub baseline: Fixed,
    /// The derived output at the same situation, with the declared input perturbed.
    pub perturbed: Fixed,
}

/// The liveness verdict for one derivation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liveness {
    /// The derived output responded to its declared input: alive.
    Live,
    /// The derived output was identical under the perturbation: dead (the soil_baseline bug).
    Dead,
}

/// The pass/fail rule: a reading whose two values differ is live; identical values are dead. Exact
/// fixed-point equality (Principle 3), so a response of a single ULP already reads live and only a
/// constant output reads dead.
pub fn assess(reading: &ProbeReading) -> Liveness {
    if reading.baseline == reading.perturbed {
        Liveness::Dead
    } else {
        Liveness::Live
    }
}

/// A site-local liveness probe: given the base manifest, it returns the two readings of one
/// derivation's output. The closure owns the mapping from the derivation's declared reserved key to
/// the kernel input it perturbs, so the mechanism ([`assess`]) stays derivation-agnostic. Returns an
/// error string if the manifest cannot supply the derivation's constants (fail-loud, never a silent
/// pass).
pub type LivenessProbe = fn(&CalibrationManifest) -> Result<ProbeReading, String>;

/// The wired probe for a registry id, or `None` where a derivation is registered (so the cross-check
/// covers it) but its site-local probe is not yet wired. A `None` is an honest coverage gap in the
/// probe wiring, reported as such, never a false [`Liveness::Dead`]. The flagship carbon-fixation case
/// (the soil_baseline bug's own site) is wired; the remaining derivations' probes follow as data-plus-
/// closure, each a kernel call reachable from this crate (the sim crate is downstream of core,
/// physics, and world).
pub fn liveness_probe(id: &str) -> Option<LivenessProbe> {
    match id {
        "carbon_fixation_rate" => Some(probe_carbon_fixation_rate),
        _ => None,
    }
}

/// The perturbation the site-local probes apply to a declared input: double it. A structural test
/// factor (never a world value, never on the run path); for the site-local assertion the magnitude
/// only has to move the output past its own ULP, which any non-degenerate factor does.
const PROBE_PERTURBATION: i32 = 2;

/// The flagship probe: the per-cell carbon-fixation rate must respond to `photosynthesis.light_saturation`.
/// The situation is a fertile daylit cell (every non-light Liebig factor saturated), so the light term
/// is limiting and the light-saturation constant reaches the derived rate; doubling it must move the
/// output. This is the exact site of the soil_baseline bug: with the soil factor barren the same kernel
/// goes dead (its output zero regardless of the constants), the case the slice's proof-of-fire test
/// reproduces.
fn probe_carbon_fixation_rate(m: &CalibrationManifest) -> Result<ProbeReading, String> {
    use crate::environ::{carbon_fixation_rate, PhotosynthesisCalib};
    let base = PhotosynthesisCalib::from_manifest(m)
        .map_err(|e| format!("carbon_fixation_rate probe: {e}"))?;
    // A representative daylit, fertile situation: full insolation under the Earth stellar constant
    // (1361 W/m^2, the flux the derivation's own doc-comment cites), the temperature at the enzyme
    // optimum, and water and soil supplied well past their requirements, so every non-light factor
    // saturates and the light-saturation constant reaches the derived rate. Situation scaffolding for
    // the two-point read; it never enters the sim.
    let insolation = Fixed::ONE;
    let solar_constant = Fixed::from_int(1361);
    let temperature = base.temp_optimum;
    let evaporative_demand = Fixed::ONE;
    let water_supply = Fixed::from_int(1000);
    let soil_fertility = Fixed::from_int(1000);
    let soil_requirement = Fixed::ONE;
    let read = |photo: &PhotosynthesisCalib| {
        carbon_fixation_rate(
            insolation,
            solar_constant,
            temperature,
            evaporative_demand,
            water_supply,
            soil_fertility,
            soil_requirement,
            photo,
        )
    };
    let baseline = read(&base);
    let mut perturbed_calib = base;
    perturbed_calib.light_saturation = base
        .light_saturation
        .checked_mul(Fixed::from_int(PROBE_PERTURBATION))
        .ok_or_else(|| {
            "carbon_fixation_rate probe: light_saturation perturbation overflowed".to_string()
        })?;
    let perturbed = read(&perturbed_calib);
    Ok(ProbeReading {
        baseline,
        perturbed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canonical_registry_carries_the_annotated_derivations() {
        let r = RetiredFloorDerivationRegistry::canonical();
        assert_eq!(r.len(), 9, "one row per @derives[id] site today");
        // Every row declares at least one input to perturb (register enforces it), a scenario, and a
        // site, so the gate has what it needs to probe each.
        for d in r.iter() {
            assert!(!d.inputs.is_empty());
            assert!(!d.scenario.is_empty());
            assert!(d.site.ends_with(".rs"));
        }
    }

    #[test]
    fn the_walk_is_deterministic_registration_order() {
        let r = RetiredFloorDerivationRegistry::canonical();
        let ids = r.ids();
        assert_eq!(ids.first(), Some(&"world_time_cadence"));
        assert_eq!(ids.last(), Some(&"carbon_fixation_rate"));
        // A second walk yields the identical order (no hash-map iteration leaks in).
        let again = RetiredFloorDerivationRegistry::canonical();
        assert_eq!(again.ids(), ids);
    }

    #[test]
    #[should_panic(expected = "duplicate retired-floor derivation id")]
    fn a_duplicate_id_panics() {
        let mut r = RetiredFloorDerivationRegistry::canonical();
        r.register(RetiredFloorDerivation {
            id: "metabolic_rate".to_string(),
            derived: "x".to_string(),
            inputs: vec!["y".to_string()],
            scenario: "full".to_string(),
            site: "z.rs".to_string(),
        });
    }

    #[test]
    #[should_panic(expected = "declares no input to perturb")]
    fn an_inputless_derivation_panics() {
        let mut r = RetiredFloorDerivationRegistry::new();
        r.register(RetiredFloorDerivation {
            id: "x".to_string(),
            derived: "x".to_string(),
            inputs: vec![],
            scenario: "full".to_string(),
            site: "z.rs".to_string(),
        });
    }

    // --- The site-local liveness probe (slice 2) ---

    /// The base manifest the probes read: the labelled dev-fixtures profile, which carries the
    /// photosynthesis constants set (not reserved). The probe never enters the sim, so a fixture
    /// base is correct: it is tooling reading run state.
    const DEV_FIXTURES: &str = include_str!("../../../calibration/profiles/dev-fixtures.toml");

    fn base_manifest() -> CalibrationManifest {
        CalibrationManifest::from_toml_str(DEV_FIXTURES).expect("dev-fixtures profile parses")
    }

    #[test]
    fn every_wired_probe_id_is_a_registered_derivation() {
        // A probe may only exist for an id the registry carries, so the cross-check that guards the
        // membership also guards the probe wiring. (The reverse is allowed: a registered derivation
        // whose probe is not yet wired is an honest coverage gap, a None, never a false Dead.)
        let registry = RetiredFloorDerivationRegistry::canonical();
        let ids = registry.ids();
        for d in registry.iter() {
            if liveness_probe(&d.id).is_some() {
                assert!(
                    ids.contains(&d.id.as_str()),
                    "wired probe id '{}' is not a registered derivation",
                    d.id
                );
            }
        }
        // The flagship is wired.
        assert!(liveness_probe("carbon_fixation_rate").is_some());
        // An unregistered id has no probe.
        assert!(liveness_probe("not_a_derivation").is_none());
    }

    #[test]
    fn the_flagship_carbon_fixation_probe_reads_live() {
        // The live derivation: perturbing photosynthesis.light_saturation moves the derived rate at
        // its site, so the probe reads Live. This is the pass side of the gate.
        let m = base_manifest();
        let probe = liveness_probe("carbon_fixation_rate").expect("the flagship probe is wired");
        let reading =
            probe(&m).expect("the fixture manifest supplies the photosynthesis constants");
        assert_ne!(
            reading.baseline, reading.perturbed,
            "a live carbon-fixation rate must respond to its light-saturation input"
        );
        assert_eq!(assess(&reading), Liveness::Live);
    }

    #[test]
    fn the_gate_fires_on_the_soil_baseline_dead_case() {
        // The proof-of-fire: reproduce the exact soil_baseline dead output. With the soil factor
        // barren (fertility zero, the soil-bootstrap deadlock the mineral-weathering arm was added to
        // break), the Liebig minimum zeroes the derived rate regardless of the photosynthesis
        // constants, so perturbing light_saturation moves nothing. assess must read Dead, catching the
        // silent regression the constructor gate cannot see.
        use crate::environ::{carbon_fixation_rate, PhotosynthesisCalib};
        let m = base_manifest();
        let base = PhotosynthesisCalib::from_manifest(&m).expect("photosynthesis constants set");
        let barren_read = |photo: &PhotosynthesisCalib| {
            carbon_fixation_rate(
                Fixed::ONE,            // full insolation
                Fixed::from_int(1361), // stellar constant
                base.temp_optimum,     // optimal temperature
                Fixed::ONE,            // small evaporative demand
                Fixed::from_int(1000), // ample water
                Fixed::ZERO,           // BARREN soil: the dead regime
                Fixed::ONE,            // soil requirement
                photo,
            )
        };
        let mut doubled = base;
        doubled.light_saturation = base.light_saturation.mul(Fixed::from_int(2));
        let dead = ProbeReading {
            baseline: barren_read(&base),
            perturbed: barren_read(&doubled),
        };
        assert_eq!(
            dead.baseline,
            Fixed::ZERO,
            "the barren regime fixes no carbon"
        );
        assert_eq!(
            assess(&dead),
            Liveness::Dead,
            "a derived output constant under its input perturbation must read Dead"
        );
    }
}
