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

//! The per-race base-rate substrate (design Part 9.9, Part 20, Part 40; the race half of the
//! R-EVIDENCE `implication_weights` and `absence_windows` derivations).
//!
//! Three of the evidence engine's numbers were once authored per trace kind: how much weight a
//! death-implying trace carries, and how long an absence runs before a being is presumed dead.
//! Both are functions of a race's own biology, not of a human forensic table. A hive race whose
//! members die and are recycled constantly finds a corpse unremarkable; a long-lived, highly
//! visible race treats a brief absence as alarming. So the weight and the window must derive from
//! per-race data, and this registry is where that data lives: each race's natural mortality curve
//! (the same shape the demography tier evaluates), how visible its members are, and how much
//! faster or slower its remains decay than the trace kind's own rate.
//!
//! This is a NEW registry beside [`crate::race::Race`], sibling to the value substrate (Part 21),
//! the homeostatic-axis substrate, and the trace-kind substrate ([`crate::trace`]). It authors no
//! outcome: the derivation functions in [`crate::trace`] and [`crate::absence`] read these fields
//! and never branch on a concrete [`RaceId`] (Principle 9). Deliberately it does NOT touch the
//! running world's [`crate::world::World`] mortality path (`set_mortality_hazard` /
//! `apply_mortality`); wiring the two mortality sources into one is a named follow-on, kept
//! separate so this substrate stays additive and standalone. The reserved inputs (each race's
//! `visibility` and `decay_multiplier`, and the shape of its `natural_mortality`) live as typed
//! data on these records, surfaced for the owner as the labelled dev fixture below rather than
//! fabricated into a manifest entry.

use std::collections::BTreeMap;

use civsim_core::Fixed;

use civsim_foundation::decision::Curve;
use civsim_foundation::value::RaceId;

/// One race's base rates: the biology the evidence derivations read.
///
/// Every field is per-race data (Principle 11); the mechanisms that consume it are fixed Rust. A
/// race differs only in this record, so two races reach different implication weights and absence
/// windows through one function rather than a per-race code branch.
#[derive(Clone, Debug)]
pub struct RaceBaseRates {
    /// Which race these rates describe.
    pub race: RaceId,
    /// The race's natural (background) mortality hazard as a per-cadence death probability against
    /// age, the same [`Curve`] shape the demography tier evaluates (design Part 20, R-AGING). It is
    /// the base rate `P(cause true anyway)` a death-implying trace is weighed against: a race that
    /// dies of natural causes constantly finds a corpse less surprising, so its traces carry less
    /// weight of evidence. RESERVED (shape). Basis: the race's own life table, the demographic unit
    /// used elsewhere, never a human forensic anchor.
    pub natural_mortality: Curve,
    /// How perceptible a member of this race is, a `0..1` visibility. RESERVED. Basis: the fraction
    /// of a member's presence an ordinary observer registers, from the race's size, mode of life,
    /// and how it disperses; it scales the absence window (a less visible race is presumed dead
    /// later, since its absence is harder to confirm), never an authored per-race window.
    pub visibility: Fixed,
    /// A multiplier on how fast this race's remains decay relative to a trace kind's own
    /// decomposition rate. RESERVED. Basis: the race's tissue chemistry against the trace kind's
    /// baseline (a race whose remains break down faster carries a multiplier above one, a durable
    /// one below), the per-race scaling the organic-salience derivation applies, not a per-race
    /// decay table.
    pub decay_multiplier: Fixed,
}

/// The set of per-race base rates a world runs, keyed by [`RaceId`] in canonical (ascending) order
/// so any walk is reproducible (R-CANON-WALK) and the registry has one representation for one
/// membership. Data-defined and extensible: a new race is covered the moment it registers its row.
#[derive(Clone, Debug, Default)]
pub struct RaceBaseRateRegistry(BTreeMap<RaceId, RaceBaseRates>);

impl RaceBaseRateRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        RaceBaseRateRegistry(BTreeMap::new())
    }

    /// Insert or replace a race's row, keyed by its own id, so the store stays canonical.
    pub fn insert(&mut self, rates: RaceBaseRates) {
        self.0.insert(rates.race, rates);
    }

    /// The base rates for a race, if registered.
    pub fn get(&self, race: RaceId) -> Option<&RaceBaseRates> {
        self.0.get(&race)
    }

    /// Iterate the rows in canonical (ascending race id) order.
    pub fn iter(&self) -> impl Iterator<Item = (&RaceId, &RaceBaseRates)> {
        self.0.iter()
    }

    /// A labelled DEVELOPMENT FIXTURE, not owner values, so the derivations run and can be tested
    /// now. Two contrasting races: a visible, ordinarily-mortal, ordinarily-decaying one, and a
    /// less visible, higher-background-mortality, faster-decaying one. The contrast is a fixture to
    /// exercise the non-steering swaps, never a fabricated calibration; the real rates are the
    /// owner's per-race data.
    pub fn dev_default() -> RaceBaseRateRegistry {
        let mut reg = RaceBaseRateRegistry::new();
        reg.insert(RaceBaseRates {
            race: DEV_LONGLIVED,
            natural_mortality: dev_low_hazard(),
            visibility: Fixed::from_ratio(3, 4),
            decay_multiplier: Fixed::ONE,
        });
        reg.insert(RaceBaseRates {
            race: DEV_SHORTLIVED,
            natural_mortality: dev_high_hazard(),
            visibility: Fixed::from_ratio(1, 4),
            decay_multiplier: Fixed::from_ratio(3, 2),
        });
        reg
    }
}

/// A rising, low-background hazard for the dev fixture: a long-lived, ordinarily-mortal race.
fn dev_low_hazard() -> Curve {
    Curve::new([
        (Fixed::from_int(0), Fixed::from_ratio(1, 100)),
        (Fixed::from_int(50), Fixed::from_ratio(1, 20)),
        (Fixed::from_int(80), Fixed::from_ratio(1, 2)),
        (Fixed::from_int(120), Fixed::ONE),
    ])
}

/// A rising, high-background hazard for the dev fixture: a short-lived race that dies often.
fn dev_high_hazard() -> Curve {
    Curve::new([
        (Fixed::from_int(0), Fixed::from_ratio(1, 5)),
        (Fixed::from_int(20), Fixed::from_ratio(1, 2)),
        (Fixed::from_int(40), Fixed::ONE),
    ])
}

/// The long-lived dev-fixture race (a leaf id, not special-cased in any mechanism).
pub const DEV_LONGLIVED: RaceId = RaceId(0);
/// The short-lived dev-fixture race.
pub const DEV_SHORTLIVED: RaceId = RaceId(1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_keyed_and_canonical() {
        let reg = RaceBaseRateRegistry::dev_default();
        assert!(reg.get(DEV_LONGLIVED).is_some());
        assert!(reg.get(DEV_SHORTLIVED).is_some());
        assert!(reg.get(RaceId(99)).is_none());
        // Canonical ascending-id order regardless of insertion order.
        let ids: Vec<u32> = reg.iter().map(|(id, _)| id.0).collect();
        assert_eq!(
            ids,
            vec![0, 1],
            "the store walks in ascending race id order"
        );
    }

    #[test]
    fn a_later_insert_replaces_a_row_keyed_by_id() {
        let mut reg = RaceBaseRateRegistry::new();
        reg.insert(RaceBaseRates {
            race: DEV_LONGLIVED,
            natural_mortality: dev_low_hazard(),
            visibility: Fixed::from_ratio(1, 2),
            decay_multiplier: Fixed::ONE,
        });
        reg.insert(RaceBaseRates {
            race: DEV_LONGLIVED,
            natural_mortality: dev_low_hazard(),
            visibility: Fixed::from_ratio(1, 4),
            decay_multiplier: Fixed::ONE,
        });
        assert_eq!(
            reg.get(DEV_LONGLIVED).unwrap().visibility,
            Fixed::from_ratio(1, 4)
        );
        assert_eq!(reg.iter().count(), 1, "one row per race id");
    }
}
