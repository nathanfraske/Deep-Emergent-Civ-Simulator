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

//! The per-substance material-percept substrate: a data-defined registry of the material substances a
//! being senses in the cell it stands on (the lifetime/demography keystone, pillar 2, physical-trace
//! persistence, trace slice C; Principles 8, 9, 10, 11).
//!
//! This is the [`crate::material::MaterialField`] sibling of the biology-class feature percept
//! ([`crate::percept::PerceptRegistry`]). The biology percept senses the raw amount of each declared
//! [`crate::edibility::Composition`] class; this one senses the raw amount of each declared MATERIAL
//! substance in the cell's [`crate::material::SubstanceMix`] (`spent_hull`, `granite`, `oilseed`, whatever
//! a world declares). It exists because the two feature paths are disjoint: the biology percept never
//! reads the material field, and the affordance percept ([`crate::affordance_percept`]) reads the material
//! field but exposes only DERIVED kernels (a single aggregate `FracturePotential`), so a being cannot tell
//! granite from a fresh oilseed from a spent hull. This percept restores that per-substance granularity.
//!
//! The channel is the substance's OPAQUE physical signature, its raw per-cell volume, and nothing else: no
//! label that means food, hull, or eating-happened-here (Principle 9). A being senses how much of a
//! substance is underfoot exactly as the thermoreceptor senses the raw temperature deviation, and its
//! MEANING ("standing where this residue lies pays off") is EARNED by correlating the signature with felt
//! reward, the same associative loop the harm learner runs over a biology feature. Nothing is handed: this
//! is the percept the physical trace is re-earned through, never a store a conclusion is drawn from.
//!
//! The membership is DATA keyed by the material floor's substance ids, so a new sensible substance is a
//! data edit, never an enum change or a hardcoded slot (Principle 11). The registry is EMPTY by default, so
//! a world that declares no material percepts grows the controller not at all and its run stays
//! bit-identical: the material percept is opt-in, the emergent-anatomy pattern the biology percept
//! established. Everything here is integer, fixed-point, and draws no randomness, a pure function of the
//! cell's substance mixture and the registry (Principle 3).

use civsim_core::Fixed;

use crate::material::SubstanceMix;

/// A material-percept id, minted through the registry (extensible, never a closed enum). The numeric value
/// is the percept's slot in the controller's material-feature input block, in registry order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MaterialPerceptId(pub u16);

/// One perceivable material substance as data: the material-floor substance id a being senses in the cell's
/// matter. Membership is the floor's data and grows with it (Principle 11). The substance id carries no
/// valence: the being senses how much `spent_hull` is present, not that a spent hull means a technique paid
/// off here, so its meaning is learned by correlation with felt reward rather than authored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialPerceptDef {
    /// The percept id.
    pub id: MaterialPerceptId,
    /// The material-floor substance id this percept reads (`spent_hull`, `granite`, `oilseed`, ...).
    pub substance: String,
}

/// The set of material substances a world's beings can sense in the cell they stand on, data-defined and
/// extensible. EMPTY by default, so a world that declares no material percepts leaves the controller layout
/// and every run hash unchanged (the material percept is opt-in).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialPerceptRegistry {
    percepts: Vec<MaterialPerceptDef>,
}

impl MaterialPerceptRegistry {
    /// An empty registry: no substance is sensed, so the controller grows no material-feature block and the
    /// run is bit-identical to a world without the material-percept substrate. The default and the opt-out.
    pub fn empty() -> MaterialPerceptRegistry {
        MaterialPerceptRegistry {
            percepts: Vec::new(),
        }
    }

    /// A registry over an explicit ordered list of material-floor substance ids, ids assigned by position
    /// (0, 1, ...). The order is the canonical material-feature-block order; a world declares the substances
    /// its beings can sense as data.
    pub fn from_substances(substances: &[&str]) -> MaterialPerceptRegistry {
        let percepts = substances
            .iter()
            .enumerate()
            .map(|(i, &substance)| MaterialPerceptDef {
                id: MaterialPerceptId(i as u16),
                substance: substance.to_string(),
            })
            .collect();
        MaterialPerceptRegistry { percepts }
    }

    /// The percepts in canonical id order.
    pub fn percepts(&self) -> &[MaterialPerceptDef] {
        &self.percepts
    }

    /// The number of material-feature channels (the width the controller's material-feature input block
    /// adds).
    pub fn len(&self) -> usize {
        self.percepts.len()
    }

    /// Whether the registry declares no percepts (the opt-out: the controller grows no material-feature
    /// block).
    pub fn is_empty(&self) -> bool {
        self.percepts.is_empty()
    }

    /// The raw sensed material-signature vector for the cell whose substance mixture is `cell`: the volume of
    /// each declared substance present, in canonical id order. A void cell (`None`) or a cell holding none of
    /// a substance reads zero for that channel, the substrate absence convention. This is the controller
    /// material-feature input: a raw physical read of the matter underfoot, no threshold and no label
    /// (Principle 9), the signature a being re-earns the trace belief from.
    pub fn perceive(&self, cell: Option<&SubstanceMix>) -> Vec<Fixed> {
        self.percepts
            .iter()
            .map(|p| {
                cell.map(|mix| mix.volume(&p.substance))
                    .unwrap_or(Fixed::ZERO)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(pairs: &[(&str, i32)]) -> SubstanceMix {
        let mut mix = SubstanceMix::new();
        for &(substance, volume) in pairs {
            mix.add(substance, Fixed::from_int(volume));
        }
        mix
    }

    #[test]
    fn an_empty_registry_senses_nothing_and_stays_the_opt_out() {
        let reg = MaterialPerceptRegistry::empty();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        // Even over a cell full of matter, an empty registry produces an empty feature vector, so the
        // controller grows no block and the run is bit-identical (the opt-in default).
        let mix = cell(&[("spent_hull", 5), ("granite", 3)]);
        assert!(reg.perceive(Some(&mix)).is_empty());
        assert!(reg.perceive(None).is_empty());
    }

    #[test]
    fn it_reads_each_declared_substance_amount_in_canonical_order() {
        let reg = MaterialPerceptRegistry::from_substances(&["spent_hull", "granite"]);
        assert_eq!(reg.len(), 2);
        let mix = cell(&[("granite", 3), ("spent_hull", 5)]);
        // The vector follows registry (declaration) order, not the mixture's storage order: hull first,
        // granite second, each the raw volume present.
        assert_eq!(
            reg.perceive(Some(&mix)),
            vec![Fixed::from_int(5), Fixed::from_int(3)]
        );
    }

    #[test]
    fn a_void_cell_and_an_absent_substance_read_zero() {
        let reg = MaterialPerceptRegistry::from_substances(&["spent_hull", "granite"]);
        // A void cell reads all-zero (the absence convention).
        assert_eq!(reg.perceive(None), vec![Fixed::ZERO, Fixed::ZERO]);
        // A cell holding only granite reads zero on the hull channel and the granite amount on the other.
        let mix = cell(&[("granite", 7)]);
        assert_eq!(
            reg.perceive(Some(&mix)),
            vec![Fixed::ZERO, Fixed::from_int(7)]
        );
    }

    #[test]
    fn distinct_substances_give_distinguishable_signatures() {
        // The granularity the gate's granite-beside-oilseed experiment surfaced: a spent hull and granite are
        // DIFFERENT sensed features, not one aggregate fracture channel, so a being can learn a reward on the
        // hull without it bleeding onto granite. A cell of pure hull and a cell of pure granite read as
        // orthogonal signatures.
        let reg = MaterialPerceptRegistry::from_substances(&["spent_hull", "granite", "oilseed"]);
        let hull = reg.perceive(Some(&cell(&[("spent_hull", 4)])));
        let rock = reg.perceive(Some(&cell(&[("granite", 4)])));
        assert_ne!(
            hull, rock,
            "the hull and the rock are distinct perceived features"
        );
        assert_eq!(hull, vec![Fixed::from_int(4), Fixed::ZERO, Fixed::ZERO]);
        assert_eq!(rock, vec![Fixed::ZERO, Fixed::from_int(4), Fixed::ZERO]);
    }
}
