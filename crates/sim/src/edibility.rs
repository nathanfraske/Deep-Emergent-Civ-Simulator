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

//! Edibility as a living relation (design Part 25.13, R-BIOSPHERE).
//!
//! Edibility is measured, never stored: whether a tissue is food, poison, or medicine, and
//! to whom, is the tuple the resolved biology floor ([`civsim_physics::laws`], R-PHYS-BIO)
//! returns, read against a consumer's physiology. This module wires the two halves the floor
//! contracts: an organism's per-tissue [`Composition`] (its supply over the nutrient classes
//! and its dose over the toxin classes) and a consumer's [`Physiology`] (its requirement over
//! the nutrient classes and its tolerance and integer-Hill exponent over the toxin classes),
//! then calls the floor laws. The same organism is food to one consumer, poison to another,
//! and inert to a third because the law reads two different vectors, not because the organism
//! carries a verdict.
//!
//! The organism composition is drawn at genesis by a stick-breaking walk over the nutrient
//! simplex ([`Composition::genesis`]), so it sums to one exactly and its heritable payload is
//! the stick-breaking coordinates (the axis-keyed composition genome channel of Part 25), and
//! the draw is keyed on the species and `Phase::GENESIS`, so a world's biochemistry is part
//! of its reproducible identity. The consumer physiology is per-race data. The
//! gain-versus-danger valuation is not here: it lives in the agent layer (Parts 8, 20), so
//! the reported tuple carries no risk attitude. The relational medicinal credit and the
//! biphasic hormesis curve are reserved refinements of the floor and are not baked in.

use civsim_core::{DrawKey, Fixed, Phase};
use civsim_physics::laws::{self, Edibility};

/// The caps the floor laws need, reserved with their basis in the floor (the per-class and
/// aggregate harm ceilings and the margin cap). Passed in rather than baked so the mechanism
/// carries no fabricated constant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FloorCaps {
    pub harm_cap: Fixed,
    pub total_harm_cap: Fixed,
    pub margin_cap: Fixed,
}

impl FloorCaps {
    /// A labelled DEVELOPMENT FIXTURE, not owner values.
    pub fn dev_default() -> FloorCaps {
        FloorCaps {
            harm_cap: Fixed::ONE,
            total_harm_cap: Fixed::from_int(4),
            margin_cap: Fixed::from_int(8),
        }
    }
}

/// An organism tissue's composition: the supply of each nutrient class (a mass-fraction
/// simplex summing to one) and the dose of each toxin class present. The membership is the
/// floor's axis registry; the vectors are the organism's data.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Composition {
    /// Supply per nutrient class, a simplex over `[0, ONE]` summing to one.
    pub nutrients: Vec<Fixed>,
    /// Dose per toxin class, in the floor's per-class dose scale.
    pub toxins: Vec<Fixed>,
}

impl Composition {
    /// Draw a genesis composition for a species: the nutrient simplex is a stick-breaking
    /// walk (remainder starts at one, each class takes a heritable fraction of the remainder,
    /// the last class takes the rest), so it sums to one exactly with no divide and no product
    /// exceeding the remainder, and the toxin doses are independent draws in `[0, ONE]`. Keyed
    /// on the species and `Phase::GENESIS`, so the composition is reproducible.
    pub fn genesis(
        seed: u64,
        species_id: u64,
        nutrient_classes: usize,
        toxin_classes: usize,
    ) -> Composition {
        let rng = DrawKey::entity(species_id, 0, Phase::GENESIS).rng(seed);
        let mut nutrients = Vec::with_capacity(nutrient_classes);
        let mut remainder = Fixed::ONE;
        for c in 0..nutrient_classes {
            if c + 1 == nutrient_classes {
                nutrients.push(remainder); // the last class takes the rest
            } else {
                let u = rng.unit_fixed(c as u64);
                // child = remainder * u, both in [0, ONE] so the product cannot exceed the
                // remainder; then subtract (ordered, provably >= 0).
                let child = remainder.checked_mul(u).unwrap_or(Fixed::ZERO);
                nutrients.push(child);
                remainder -= child;
            }
        }
        let mut toxins = Vec::with_capacity(toxin_classes);
        for t in 0..toxin_classes {
            toxins.push(rng.unit_fixed((nutrient_classes + t) as u64));
        }
        Composition { nutrients, toxins }
    }

    /// The exact sum of the nutrient simplex (should be one by construction).
    pub fn nutrient_total(&self) -> Fixed {
        Fixed::saturating_sum(self.nutrients.iter().copied())
    }
}

/// A consumer's physiology over the floor's relation kinds: the requirement for each nutrient
/// class, and per toxin class a tolerance (`None` is not-applicable, distinct from a present
/// zero of maximal sensitivity) and the integer-Hill exponent. Per-race heritable data.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Physiology {
    pub requirements: Vec<Fixed>,
    pub tolerances: Vec<Option<Fixed>>,
    pub hill: Vec<u8>,
}

/// Measure the edibility of a composition to a consumer through the floor laws: net nutrition
/// by the Liebig minimum of per-class satisfaction, net harm by the per-class integer-Hill
/// dose response, and the aggregate safety margin. A measured tuple, no stored verdict.
pub fn assess(comp: &Composition, phys: &Physiology, caps: &FloorCaps) -> Edibility {
    // Nutrition classes: (supply, requirement, no synthesis residual in the interim).
    let nutrition_classes: Vec<(Fixed, Fixed, Option<Fixed>)> = comp
        .nutrients
        .iter()
        .zip(phys.requirements.iter())
        .map(|(&s, &r)| (s, r, None))
        .collect();
    let net_nutrition = laws::net_nutrition(&nutrition_classes);

    // Harm classes: (dose, tolerance, hill exponent).
    let harm_classes: Vec<(Fixed, Option<Fixed>, u8)> = comp
        .toxins
        .iter()
        .enumerate()
        .map(|(i, &d)| {
            let tol = phys.tolerances.get(i).copied().flatten();
            let n = phys.hill.get(i).copied().unwrap_or(1);
            (d, tol, n)
        })
        .collect();
    let net_harm = laws::net_harm(&harm_classes, caps.harm_cap, caps.total_harm_cap);

    // Aggregate dose and tolerance for the safety margin (an applicable tolerance only).
    let dose_aggregate = Fixed::saturating_sum(comp.toxins.iter().copied());
    let tolerance_aggregate = Fixed::saturating_sum(harm_classes.iter().filter_map(|&(_, t, _)| t));
    laws::edibility(
        net_nutrition,
        net_harm,
        tolerance_aggregate,
        dose_aggregate,
        caps.margin_cap,
    )
}

/// A read-time band over the measured tuple, for display and for the agent layer to key its
/// valuation on. This is a label read from the tuple, never an input to a law or a stored
/// property of the organism.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    Food,
    Poison,
    Inert,
}

/// Read a coarse verdict from a measured tuple against a nutrition floor and a harm ceiling.
/// The bands are the reader's, not the organism's; a different reader may band differently.
pub fn verdict(e: &Edibility, nutrition_floor: Fixed, harm_ceiling: Fixed) -> Verdict {
    if e.net_harm >= harm_ceiling {
        Verdict::Poison
    } else if e.net_nutrition >= nutrition_floor {
        Verdict::Food
    } else {
        Verdict::Inert
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    #[test]
    fn a_genesis_composition_is_a_simplex() {
        let c = Composition::genesis(0xED1B, 3, 5, 3);
        assert_eq!(c.nutrients.len(), 5);
        assert_eq!(c.toxins.len(), 3);
        // The nutrient simplex sums to one within fixed-point tolerance.
        assert!((c.nutrient_total() - Fixed::ONE).abs() <= Fixed::from_ratio(1, 100000));
        // Deterministic.
        assert_eq!(c, Composition::genesis(0xED1B, 3, 5, 3));
        assert_ne!(
            c,
            Composition::genesis(0xED1B, 4, 5, 3),
            "a different species differs"
        );
    }

    #[test]
    fn the_same_organism_is_food_to_one_and_poison_to_another() {
        let caps = FloorCaps::dev_default();
        // An organism rich in nutrients and carrying one toxin class at a moderate dose.
        let comp = Composition {
            nutrients: vec![f(5, 10), f(5, 10)],
            toxins: vec![f(6, 10)],
        };
        // Consumer A: needs are met and it tolerates the toxin well.
        let a = Physiology {
            requirements: vec![f(3, 10), f(3, 10)],
            tolerances: vec![Some(f(9, 10))],
            hill: vec![2],
        };
        // Consumer B: same needs, but almost no tolerance for the toxin.
        let b = Physiology {
            requirements: vec![f(3, 10), f(3, 10)],
            tolerances: vec![Some(f(1, 100))],
            hill: vec![2],
        };
        let ea = assess(&comp, &a, &caps);
        let eb = assess(&comp, &b, &caps);
        assert!(
            eb.net_harm > ea.net_harm,
            "the low-tolerance consumer takes more harm"
        );
        assert_eq!(
            verdict(&ea, f(1, 10), f(1, 2)),
            Verdict::Food,
            "food to the tolerant one"
        );
        assert_eq!(
            verdict(&eb, f(1, 10), f(1, 2)),
            Verdict::Poison,
            "poison to the sensitive one"
        );
    }

    #[test]
    fn a_not_applicable_tolerance_takes_no_harm() {
        let caps = FloorCaps::dev_default();
        let comp = Composition {
            nutrients: vec![Fixed::ONE],
            toxins: vec![f(9, 10)],
        };
        // No tolerance entry for the toxin class: not-applicable, so zero harm (distinct from
        // a present zero tolerance).
        let phys = Physiology {
            requirements: vec![f(5, 10)],
            tolerances: vec![None],
            hill: vec![2],
        };
        let e = assess(&comp, &phys, &caps);
        assert_eq!(
            e.net_harm,
            Fixed::ZERO,
            "a not-applicable toxin class does no harm"
        );
    }
}
