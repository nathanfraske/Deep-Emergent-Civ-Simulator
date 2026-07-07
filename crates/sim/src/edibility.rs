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

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::{DrawKey, Fixed, Phase};
use civsim_physics::laws::{self, Edibility};

use crate::genome::{Channel, GeneSet, Genome, ToleranceAxisId};
use crate::homeostasis::HomeostaticRegistry;
use crate::physiology::SALINITY;

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

/// An organism tissue's composition: the supply of each nutrient class (a mass-fraction simplex
/// summing to one) and the dose of each toxin class present, each keyed by its biology-floor axis
/// id. This is the same string-keyed, sorted-walk composition-over-the-floor shape
/// [`crate::anatomy::TissueComposition`] and `civsim_physics::Substance::vector` use, so the class
/// vocabulary is the floor's DATA and grows with it at zero code cost (Principle 11): a nutrient or
/// toxin class is a data edit, never an enum change. A class the tissue bears none of is simply
/// absent (reads as zero, the substrate absence convention).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Composition {
    /// Supply per nutrient class, keyed by biology-floor axis id; a simplex over `[0, ONE]`.
    pub nutrients: BTreeMap<String, Fixed>,
    /// Dose per toxin class, keyed by biology-floor axis id, in the floor's per-class dose scale.
    pub toxins: BTreeMap<String, Fixed>,
}

impl Composition {
    /// The supply on one nutrient class, named by its biology-floor axis id. An absent class reads
    /// as zero (mirrors [`crate::anatomy::TissueComposition::component`]).
    pub fn nutrient(&self, axis: &str) -> Fixed {
        self.nutrients.get(axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// The dose on one toxin class, named by its biology-floor axis id. An absent class reads as
    /// zero (the same absence convention).
    pub fn toxin(&self, axis: &str) -> Fixed {
        self.toxins.get(axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// Set (or update) a nutrient class's supply in place, reusing the entry and its already-interned key
    /// so a repeated per-tick write allocates the key string at most once (on the first touch of a fresh
    /// composition), never every call. The stored {class: value} content is what folds into the state
    /// hash, so this is byte-identical to rebuilding the map with the same content.
    pub fn set_nutrient(&mut self, class: &str, value: Fixed) {
        if let Some(v) = self.nutrients.get_mut(class) {
            *v = value;
        } else {
            self.nutrients.insert(class.to_string(), value);
        }
    }

    /// Set or clear a toxin class's dose in place. A positive dose sets it (reusing the key); a zero or
    /// negative dose removes the class, matching the absence convention a rebuilt composition follows when
    /// it omits an absent toxin, so the stored content and its hash are unchanged.
    pub fn set_toxin(&mut self, class: &str, dose: Fixed) {
        if dose > Fixed::ZERO {
            if let Some(v) = self.toxins.get_mut(class) {
                *v = dose;
            } else {
                self.toxins.insert(class.to_string(), dose);
            }
        } else {
            self.toxins.remove(class);
        }
    }

    /// The raw amount of a substance class present, whatever its valence: the nutrient supply plus the
    /// toxin dose of the class (a class lives in at most one map, so one term is zero). This is the
    /// valence-blind percept read the feature substrate uses ([`crate::percept`]): a being senses how
    /// much of a substance is underfoot, never that the floor filed it as food or poison, so its
    /// meaning is learned by correlation with felt outcome, not authored (Principle 8).
    pub fn sensed(&self, class: &str) -> Fixed {
        self.nutrient(class).saturating_add(self.toxin(class))
    }

    /// Draw a genesis composition for a species over an explicit ordered list of nutrient class ids
    /// and toxin class ids: the nutrient simplex is a stick-breaking walk (remainder starts at one,
    /// each class takes a heritable fraction of the remainder, the last class takes the rest), so it
    /// sums to one exactly with no divide and no product exceeding the remainder, and the toxin doses
    /// are independent draws in `[0, ONE]`. Keyed on the species and `Phase::GENESIS`, so the
    /// composition is reproducible. The class ids, not a bare count, are the membership (Principle
    /// 11): the floor's axis vocabulary is passed in as data, and the per-index draw order is
    /// preserved so a world's biochemistry stays part of its reproducible identity.
    pub fn genesis(
        seed: u64,
        species_id: u64,
        nutrient_axes: &[&str],
        toxin_axes: &[&str],
    ) -> Composition {
        let rng = DrawKey::entity(species_id, 0, Phase::GENESIS).rng(seed);
        let mut nutrients = BTreeMap::new();
        let mut remainder = Fixed::ONE;
        let n = nutrient_axes.len();
        for (c, &axis) in nutrient_axes.iter().enumerate() {
            let value = if c + 1 == n {
                remainder // the last class takes the rest
            } else {
                let u = rng.unit_fixed(c as u64);
                // child = remainder * u, both in [0, ONE] so the product cannot exceed the
                // remainder; then subtract (ordered, provably >= 0).
                let child = remainder.checked_mul(u).unwrap_or(Fixed::ZERO);
                remainder -= child;
                child
            };
            nutrients.insert(axis.to_string(), value);
        }
        let mut toxins = BTreeMap::new();
        for (t, &axis) in toxin_axes.iter().enumerate() {
            toxins.insert(axis.to_string(), rng.unit_fixed((n + t) as u64));
        }
        Composition { nutrients, toxins }
    }

    /// The exact sum of the nutrient simplex (should be one by construction).
    pub fn nutrient_total(&self) -> Fixed {
        Fixed::saturating_sum(self.nutrients.values().copied())
    }
}

/// A consumer's physiology over the floor's relation kinds, each keyed by biology-floor class id:
/// the requirement for each nutrient class it needs, the assimilation efficiency with which it
/// extracts each nutrient class, and per toxin class a tolerance (ABSENT is not-applicable, distinct
/// from a PRESENT zero of maximal sensitivity) and the integer-Hill exponent. Per-race heritable
/// data; the class vocabulary is the floor's, so it grows with the world (Principle 11).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Physiology {
    /// The requirement on each nutrient class the consumer needs; an absent class is not required
    /// (fully satisfied, never lowers the Liebig minimum).
    pub requirements: BTreeMap<String, Fixed>,
    /// The assimilation efficiency on each nutrient class, the fraction of supply the consumer can
    /// draw into its reserve; an absent class is not assimilated (zero, the absence convention).
    pub assimilation: BTreeMap<String, Fixed>,
    /// The tolerance on each toxin class; PRESENT is applicable (a present zero is maximal
    /// sensitivity), ABSENT is not-applicable (the class does this consumer no harm).
    pub tolerances: BTreeMap<String, Fixed>,
    /// The integer-Hill exponent on each toxin class; an absent class defaults to one.
    pub hill: BTreeMap<String, u8>,
}

impl Physiology {
    /// The requirement on a nutrient class, `None` when the consumer does not require it.
    pub fn requirement(&self, class: &str) -> Option<Fixed> {
        self.requirements.get(class).copied()
    }

    /// The assimilation efficiency on a nutrient class; an absent class reads as zero (not
    /// assimilated), the substrate absence convention.
    pub fn assimilation(&self, class: &str) -> Fixed {
        self.assimilation.get(class).copied().unwrap_or(Fixed::ZERO)
    }

    /// The tolerance on a toxin class, `None` (not-applicable) when the class is absent.
    pub fn tolerance(&self, class: &str) -> Option<Fixed> {
        self.tolerances.get(class).copied()
    }

    /// The integer-Hill exponent on a toxin class; an absent class defaults to one.
    pub fn hill_exp(&self, class: &str) -> u8 {
        self.hill.get(class).copied().unwrap_or(1)
    }

    /// A labelled DEVELOPMENT FIXTURE physiology derived from a homeostatic registry: for every axis
    /// backed by a biology-floor class, a unit requirement and unit assimilation on that class, and no
    /// toxin tolerances. Not owner values; a stand-in so a walker has a physiology to ingest through
    /// until per-race physiology data is authored. The assimilation datum is the labelled `Fixed::ONE`
    /// per class.
    pub fn dev_for_registry(reg: &HomeostaticRegistry) -> Physiology {
        let mut requirements = BTreeMap::new();
        let mut assimilation = BTreeMap::new();
        for axis in &reg.axes {
            if let Some(class) = &axis.backing_component {
                requirements.insert(class.clone(), Fixed::ONE);
                assimilation.insert(class.clone(), Fixed::ONE);
            }
        }
        Physiology {
            requirements,
            assimilation,
            tolerances: BTreeMap::new(),
            hill: BTreeMap::new(),
        }
    }

    /// Express a consumer physiology from a genome (base-level liveliness step 4): the nutrient
    /// requirement and assimilation over the homeostatic registry's backed classes exactly as
    /// [`Physiology::dev_for_registry`] builds them (the labelled unit dev fixtures), PLUS a heritable
    /// per-toxin-class TOLERANCE expressed from the genome through the [`ToleranceRegistry`], so a
    /// being's resistance to a salt flat or a dust haze is its inheritance the way its size and its
    /// behaviour are. Selection on the tolerance channel is what lets a lineage adapt to an
    /// environmental gradient rather than being excluded at a fixed dose (Principle 8: a graded dose;
    /// Principle 9: keyed off the floor class id and the genome, never a `RaceId`). Each class's
    /// tolerance is PRESENT (so the toxin applies; a low value is sensitive, a high value resistant, and
    /// the harm law reads a present zero as maximal sensitivity), clamped non-negative. The Hill exponent
    /// is the class's data constant (the dose-response shape), not the heritable selection target.
    pub fn express(
        reg: &HomeostaticRegistry,
        tolerances_reg: &ToleranceRegistry,
        genes: &GeneSet,
        genome: &Genome,
    ) -> Physiology {
        let mut phys = Physiology::dev_for_registry(reg);
        for tc in &tolerances_reg.classes {
            let value = genes
                .express(genome, Channel::Tolerance(tc.axis), Fixed::ZERO)
                .clamp(Fixed::ZERO, Fixed::MAX);
            phys.tolerances.insert(tc.class.clone(), value);
            phys.hill.insert(tc.class.clone(), tc.hill);
        }
        phys
    }
}

/// One toxin-tolerance class as data (base-level liveliness step 4): the floor toxin-class id, the gene
/// channel a being's heritable tolerance for it is expressed through, and the integer-Hill exponent (the
/// dose-response steepness). The magnitude is heritable and selected; the Hill exponent is the class's
/// data-constant curve shape.
#[derive(Clone, Debug)]
pub struct ToleranceClass {
    /// The floor toxin-class id (for example `bio.salinity`), the key a cell composition doses and the
    /// harm law reads.
    pub class: String,
    /// The gene channel this class's heritable tolerance is expressed through.
    pub axis: ToleranceAxisId,
    /// The integer-Hill exponent, the dose-response steepness for this class.
    pub hill: u8,
}

/// The world's toxin-tolerance registry (base-level liveliness step 4): a data-defined map from a floor
/// toxin-class id to the gene channel a being's heritable tolerance for it is expressed through, plus
/// the per-class Hill exponent. Sibling to the controller-parameter and composition-axis registries: the
/// mechanism (express a tolerance per class from the genome) is fixed Rust, the membership (which toxin
/// classes a world runs, and which gene channel and Hill each carries) is data (Principle 11), so a salt
/// flat, a dust haze, or an alien toxin is a data edit, keyed off the floor class id, never a `RaceId`
/// (Principle 9).
#[derive(Clone, Debug, Default)]
pub struct ToleranceRegistry {
    pub classes: Vec<ToleranceClass>,
}

impl ToleranceRegistry {
    /// A labelled DEVELOPMENT FIXTURE: a single salinity tolerance class keyed off `bio.salinity`,
    /// expressed through tolerance axis 0 with a Hill exponent of two (a moderately steep dose response),
    /// so a founding grazer carries a heritable salt tolerance and a lineage can adapt to a salt flat.
    pub fn dev_salinity() -> ToleranceRegistry {
        ToleranceRegistry {
            classes: vec![ToleranceClass {
                class: SALINITY.to_string(),
                axis: ToleranceAxisId(0),
                hill: 2,
            }],
        }
    }
}

/// Measure the edibility of a composition to a consumer through the floor laws: net nutrition
/// by the Liebig minimum of per-class satisfaction, net harm by the per-class integer-Hill
/// dose response, and the aggregate safety margin. A measured tuple, no stored verdict.
pub fn assess(comp: &Composition, phys: &Physiology, caps: &FloorCaps) -> Edibility {
    // Nutrition: walk the union of the classes the composition supplies and the classes the consumer
    // requires, so neither side's axis set is assumed to match the other's. Each class contributes
    // its Liebig satisfaction over (supply, assimilation, requirement); a class the consumer does not
    // require never lowers the minimum (`laws::satisfaction` returns ONE for a `None` requirement).
    // The requirement lands in the requirement slot (not the assimilation slot), so satisfaction
    // varies with supply as the floor contracts (`laws::net_nutrition`, R-PHYS-BIO).
    let mut nutrient_classes: BTreeSet<&str> = BTreeSet::new();
    nutrient_classes.extend(comp.nutrients.keys().map(String::as_str));
    nutrient_classes.extend(phys.requirements.keys().map(String::as_str));
    let nutrition_classes: Vec<(Fixed, Fixed, Option<Fixed>)> = nutrient_classes
        .iter()
        .map(|&class| {
            (
                comp.nutrient(class),
                phys.assimilation(class),
                phys.requirement(class),
            )
        })
        .collect();
    let net_nutrition = laws::net_nutrition(&nutrition_classes);

    // Harm: walk the union of the classes the composition doses and the classes the consumer carries
    // a tolerance or a Hill exponent for. (dose, tolerance, hill exponent) per class.
    let mut toxin_classes: BTreeSet<&str> = BTreeSet::new();
    toxin_classes.extend(comp.toxins.keys().map(String::as_str));
    toxin_classes.extend(phys.tolerances.keys().map(String::as_str));
    toxin_classes.extend(phys.hill.keys().map(String::as_str));
    let harm_classes: Vec<(Fixed, Option<Fixed>, u8)> = toxin_classes
        .iter()
        .map(|&class| {
            (
                comp.toxin(class),
                phys.tolerance(class),
                phys.hill_exp(class),
            )
        })
        .collect();
    let net_harm = laws::net_harm(&harm_classes, caps.harm_cap, caps.total_harm_cap);

    // Aggregate dose and tolerance for the safety margin over the SAME set of classes: only classes
    // the consumer carries a tolerance for (Some). A class the composition doses but the consumer has
    // no tolerance for contributes to neither aggregate, so an inert meal (a "toxin" the consumer
    // does not register) does not accrue an unbalanced dose against a zero tolerance. Summing every
    // dose while summing only paired tolerances made the margin read as if the whole dose were
    // untolerated.
    let (dose_aggregate, tolerance_aggregate) = harm_classes.iter().fold(
        (Fixed::ZERO, Fixed::ZERO),
        |(dose_acc, tol_acc), &(dose, tol, _)| match tol {
            Some(tolerance) => (
                dose_acc.saturating_add(dose),
                tol_acc.saturating_add(tolerance),
            ),
            None => (dose_acc, tol_acc),
        },
    );
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

    /// Build a composition from (class id, value) pairs (a test convenience).
    fn comp(nutrients: &[(&str, Fixed)], toxins: &[(&str, Fixed)]) -> Composition {
        Composition {
            nutrients: nutrients.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
            toxins: toxins.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
        }
    }

    /// Build a class-keyed map from (class id, value) pairs.
    fn m<V: Copy>(pairs: &[(&str, V)]) -> BTreeMap<String, V> {
        pairs.iter().map(|&(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn a_genesis_composition_is_a_simplex() {
        let nut = ["n0", "n1", "n2", "n3", "n4"];
        let tox = ["t0", "t1", "t2"];
        let c = Composition::genesis(0xED1B, 3, &nut, &tox);
        assert_eq!(c.nutrients.len(), 5);
        assert_eq!(c.toxins.len(), 3);
        // The nutrient simplex sums to one within fixed-point tolerance.
        assert!((c.nutrient_total() - Fixed::ONE).abs() <= Fixed::from_ratio(1, 100000));
        // Deterministic.
        assert_eq!(c, Composition::genesis(0xED1B, 3, &nut, &tox));
        assert_ne!(
            c,
            Composition::genesis(0xED1B, 4, &nut, &tox),
            "a different species differs"
        );
    }

    #[test]
    fn the_same_organism_is_food_to_one_and_poison_to_another() {
        let caps = FloorCaps::dev_default();
        // An organism rich in nutrients and carrying one toxin class at a moderate dose.
        let organism = comp(&[("n0", f(5, 10)), ("n1", f(5, 10))], &[("t0", f(6, 10))]);
        // Consumer A: needs are met (and assimilated) and it tolerates the toxin well.
        let a = Physiology {
            requirements: m(&[("n0", f(3, 10)), ("n1", f(3, 10))]),
            assimilation: m(&[("n0", Fixed::ONE), ("n1", Fixed::ONE)]),
            tolerances: m(&[("t0", f(9, 10))]),
            hill: m(&[("t0", 2u8)]),
        };
        // Consumer B: same needs, but almost no tolerance for the toxin.
        let b = Physiology {
            requirements: m(&[("n0", f(3, 10)), ("n1", f(3, 10))]),
            assimilation: m(&[("n0", Fixed::ONE), ("n1", Fixed::ONE)]),
            tolerances: m(&[("t0", f(1, 100))]),
            hill: m(&[("t0", 2u8)]),
        };
        let ea = assess(&organism, &a, &caps);
        let eb = assess(&organism, &b, &caps);
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
        let organism = comp(&[("n0", Fixed::ONE)], &[("t0", f(9, 10))]);
        // No tolerance entry for the toxin class: absent is not-applicable, so zero harm (distinct
        // from a present zero tolerance, which is maximal sensitivity).
        let phys = Physiology {
            requirements: m(&[("n0", f(5, 10))]),
            assimilation: m(&[("n0", Fixed::ONE)]),
            tolerances: BTreeMap::new(),
            hill: m(&[("t0", 2u8)]),
        };
        let e = assess(&organism, &phys, &caps);
        assert_eq!(
            e.net_harm,
            Fixed::ZERO,
            "a not-applicable toxin class does no harm"
        );
    }

    #[test]
    fn different_requirements_yield_different_net_nutrition() {
        // The bug-fix regression (R-PHYS-BIO): the requirement must land in laws::satisfaction's
        // requirement slot, not the assimilation slot. Two consumers with different requirements over
        // the SAME composition now measure different net nutrition; before the fix net_nutrition was
        // ONE every call (satisfaction saw a None requirement and returned ONE).
        let caps = FloorCaps::dev_default();
        let food = comp(&[("n0", f(2, 10))], &[]); // one nutrient class at a modest supply
        let lean = Physiology {
            requirements: m(&[("n0", f(4, 10))]),
            assimilation: m(&[("n0", Fixed::ONE)]),
            tolerances: BTreeMap::new(),
            hill: BTreeMap::new(),
        };
        let demanding = Physiology {
            requirements: m(&[("n0", f(8, 10))]),
            assimilation: m(&[("n0", Fixed::ONE)]),
            tolerances: BTreeMap::new(),
            hill: BTreeMap::new(),
        };
        let el = assess(&food, &lean, &caps);
        let ed = assess(&food, &demanding, &caps);
        assert!(
            el.net_nutrition > ed.net_nutrition,
            "the leaner requirement is better satisfied by the same supply"
        );
        assert_eq!(
            el.net_nutrition,
            f(1, 2),
            "0.2 supply over 0.4 requirement is half satisfied"
        );
        assert_eq!(
            ed.net_nutrition,
            f(1, 4),
            "0.2 supply over 0.8 requirement is a quarter satisfied"
        );
    }
}
