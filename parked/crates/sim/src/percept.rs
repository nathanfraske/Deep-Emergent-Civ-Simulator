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

//! The perceived-feature substrate: a data-defined registry of the raw environmental features a
//! being senses on the cell it stands on (harm-learning arc slice a; Principles 8, 9, 10, 11).
//!
//! This is the sensory sibling of the homeostatic and affordance registries
//! ([`crate::homeostasis`]). A being does not read a dose threshold, a hazard label, or a race id; it
//! senses the raw amount of each declared substance class underfoot, exactly as the thermoreceptor
//! senses the raw temperature deviation. The set of sensible classes is DATA keyed by the biology
//! floor's class strings (`bio.salinity`, a nutrient class, whatever a world declares), so a new
//! percept is a data edit and never an enum change or a hardcoded `{salinity}` slot (Principle 11).
//! The registry is EMPTY by default, so a world that declares no percepts grows the controller not at
//! all and its run stays bit-identical: the feature percept is opt-in, the emergent-anatomy pattern.
//!
//! The percept feeds two consumers. As a raw scalar per class it is a controller input (slice a, the
//! feature block of [`crate::controller::ControllerLayout`]), which the evolved weights may learn to
//! act on. Quantized to a coarse grid it is the stable KEY a per-feature belief subject is minted
//! from (slice b, the associative learner): the being correlates felt harm with a quantized feature
//! level, so "this ground harms me" emerges from correlation over a bounded-acuity percept, never a
//! god's-eye continuous read (Principle 10). Everything here is integer, fixed-point, and draws no
//! randomness, a pure function of the cell's composition and the registry (Principle 3).

use civsim_core::Fixed;

use crate::edibility::{Composition, ToleranceRegistry};

/// A feature-percept id, minted through the registry (extensible, never a closed enum). The numeric
/// value is the percept's slot in the controller's feature input block, in registry order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PerceptId(pub u16);

/// One perceivable feature class as data: the biology-floor class string a being senses underfoot.
/// Membership is the floor's data and grows with it (Principle 11). The class carries no valence: the
/// being senses how much salinity is present, not that salinity is a toxin, so its meaning is learned
/// by correlation with felt outcome rather than authored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerceptDef {
    /// The percept id.
    pub id: PerceptId,
    /// The biology-floor class string this percept reads (`bio.salinity`, a nutrient class, ...).
    pub class: String,
}

/// The set of feature classes a world's beings can perceive on the cell they stand on, data-defined
/// and extensible. EMPTY by default, so a world that declares no percepts leaves the controller layout
/// and every run hash unchanged (the feature percept is opt-in).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PerceptRegistry {
    percepts: Vec<PerceptDef>,
}

impl PerceptRegistry {
    /// An empty registry: no feature is sensed, so the controller grows no feature block and the run
    /// is bit-identical to a world without the percept substrate. The default and the opt-out.
    pub fn empty() -> PerceptRegistry {
        PerceptRegistry {
            percepts: Vec::new(),
        }
    }

    /// A registry over an explicit ordered list of biology-floor class strings, ids assigned by
    /// position (0, 1, ...). The order is the canonical feature-block order; a world declares the
    /// classes its beings can sense as data.
    pub fn from_classes(classes: &[&str]) -> PerceptRegistry {
        let percepts = classes
            .iter()
            .enumerate()
            .map(|(i, &class)| PerceptDef {
                id: PerceptId(i as u16),
                class: class.to_string(),
            })
            .collect();
        PerceptRegistry { percepts }
    }

    /// A labelled DEVELOPMENT FIXTURE: a world that senses the salt flat's `bio.salinity`, the toxin
    /// the base-liveliness harm law already reads. Not owner data; the minimum a harm-learning run
    /// needs so a being can correlate felt harm with the ground it stands on.
    pub fn dev_salinity() -> PerceptRegistry {
        PerceptRegistry::from_classes(&[crate::physiology::SALINITY])
    }

    /// A registry over the toxin classes a world declares its beings tolerate (its harm-relevant
    /// substances), derived from the tolerance registry (harm-learning arc slice b). A being perceives
    /// the substance classes its physiology responds to, so the percept vocabulary grows with the
    /// world's declared toxins as data (Principle 11), never a hardcoded `{salinity}` slot. The ids
    /// follow the tolerance-registry order, so a world's biochemistry stays part of its reproducible
    /// identity.
    pub fn from_tolerances(tolerances: &ToleranceRegistry) -> PerceptRegistry {
        let classes: Vec<&str> = tolerances
            .classes
            .iter()
            .map(|c| c.class.as_str())
            .collect();
        PerceptRegistry::from_classes(&classes)
    }

    /// The percepts in canonical id order.
    pub fn percepts(&self) -> &[PerceptDef] {
        &self.percepts
    }

    /// The number of feature channels (the width the controller's feature input block adds).
    pub fn len(&self) -> usize {
        self.percepts.len()
    }

    /// Whether the registry declares no percepts (the opt-out: the controller grows no feature block).
    pub fn is_empty(&self) -> bool {
        self.percepts.is_empty()
    }

    /// The raw sensed feature vector for the cell whose composition is `comp`: the amount of each
    /// declared class present, in canonical id order. A cell with no composition (or none of a class)
    /// reads zero for that channel, the substrate absence convention. This is the controller feature
    /// input: a raw physical read of what is underfoot, no threshold and no label (Principle 9).
    pub fn perceive(&self, comp: Option<&Composition>) -> Vec<Fixed> {
        self.percepts
            .iter()
            .map(|p| comp.map(|c| c.sensed(&p.class)).unwrap_or(Fixed::ZERO))
            .collect()
    }
}

/// The discrete bucket a raw feature scalar falls in on the being's perceptual grid (floor
/// quantization: the largest multiple of `granularity` at or below `raw`). This is the stable key a
/// per-feature belief subject is minted from (slice b): two cells whose feature amount lands in the
/// same bucket are the same perceived kind, so a belief learned on one applies to the other, the
/// coarse-graining that lets a lineage generalise "salty ground harms me" past the exact dose. A
/// non-positive granularity is a misconfiguration (the reserved value is fail-loud in production) and
/// reads as bucket zero.
///
/// RESERVED: the feature granularity (the quantization step). Basis: the just-noticeable difference in
/// the sensed substance, the sensorium's per-class acuity/JND the perception subsystem already defines
/// (`civsim_foundation::sensorium`); coarse enough that ordinary spatial variation in a hazard reads as one kind,
/// fine enough to separate a harmful ground from a benign one. Surfaced for the owner, never
/// fabricated.
pub fn feature_bucket(raw: Fixed, granularity: Fixed) -> i64 {
    if granularity.to_bits() <= 0 {
        return 0;
    }
    raw.checked_div(granularity)
        .map(|q| q.to_int() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physiology::SALINITY;
    use std::collections::BTreeMap;

    fn comp_with(class: &str, dose: Fixed) -> Composition {
        let mut toxins = BTreeMap::new();
        toxins.insert(class.to_string(), dose);
        Composition {
            nutrients: BTreeMap::new(),
            toxins,
        }
    }

    #[test]
    fn empty_registry_perceives_nothing_and_grows_no_block() {
        let reg = PerceptRegistry::empty();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        // A cell with composition still yields a zero-width vector: no percept declared, nothing sensed.
        let comp = comp_with(SALINITY, Fixed::from_int(3));
        assert!(reg.perceive(Some(&comp)).is_empty());
    }

    #[test]
    fn a_declared_class_is_sensed_as_its_raw_amount_in_canonical_order() {
        let reg = PerceptRegistry::from_classes(&["a", "b"]);
        assert_eq!(reg.len(), 2);
        let mut toxins = BTreeMap::new();
        toxins.insert("b".to_string(), Fixed::from_int(5));
        let comp = Composition {
            nutrients: BTreeMap::new(),
            toxins,
        };
        let v = reg.perceive(Some(&comp));
        // Canonical order is the declared order: a (absent, zero), then b (five).
        assert_eq!(v, vec![Fixed::ZERO, Fixed::from_int(5)]);
    }

    #[test]
    fn a_cell_with_no_composition_reads_zero_for_every_channel() {
        let reg = PerceptRegistry::dev_salinity();
        assert_eq!(reg.perceive(None), vec![Fixed::ZERO]);
    }

    #[test]
    fn nutrient_and_toxin_of_the_same_class_are_both_sensed() {
        // A percept reads the substance amount regardless of whether the floor filed it as food or
        // poison: the being senses the substance, its valence is learned, not authored.
        let reg = PerceptRegistry::from_classes(&["x"]);
        let mut nutrients = BTreeMap::new();
        nutrients.insert("x".to_string(), Fixed::from_int(2));
        let comp = Composition {
            nutrients,
            toxins: BTreeMap::new(),
        };
        assert_eq!(reg.perceive(Some(&comp)), vec![Fixed::from_int(2)]);
    }

    #[test]
    fn quantization_floors_a_raw_amount_to_its_grid_bucket() {
        let g = Fixed::from_ratio(1, 4); // a quarter-unit grid
                                         // 0.9 falls in bucket 3 (0.75..1.0), 0.2 in bucket 0, 1.0 in bucket 4.
        assert_eq!(feature_bucket(Fixed::from_ratio(9, 10), g), 3);
        assert_eq!(feature_bucket(Fixed::from_ratio(2, 10), g), 0);
        assert_eq!(feature_bucket(Fixed::ONE, g), 4);
        // Two amounts in the same bucket are the same perceived kind (the coarse-graining).
        assert_eq!(
            feature_bucket(Fixed::from_ratio(76, 100), g),
            feature_bucket(Fixed::from_ratio(99, 100), g)
        );
    }

    #[test]
    fn a_nonpositive_granularity_reads_as_bucket_zero() {
        assert_eq!(feature_bucket(Fixed::from_int(5), Fixed::ZERO), 0);
    }
}
