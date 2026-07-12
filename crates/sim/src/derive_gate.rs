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
//! This slice carries the membership and the cross-check surface only. The site-local liveness
//! probe (perturb the input, assert the derived output responds) is the next slice; a live-but-
//! byte-neutral derivation whose effect lands in a downstream deadband is a coverage gap the
//! separate coverage signal reports, never a liveness failure (the gate's ruling).

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
}
