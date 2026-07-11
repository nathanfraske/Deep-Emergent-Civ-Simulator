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

//! The perceivable-feature substrate: the data-defined, OPEN registry of the emitter optical properties a
//! perceiver can sense on a being-signal beyond its single strength scalar (creature-selection step 2b, the
//! percept kind-feature FLOOR arc; Principles 8, 9, 10, 11).
//!
//! Step 2b's frame-blind (section-11 smoke to CLEAR, then a 6/6 unanimous section-10 blind panel) settled the
//! shape of the emitter feature. The strength-alone being-signal cannot separate two emitters whose bodies
//! share a temperature and an emission coefficient, so a mind-less creature receiving predator and mate as
//! one magnitude through one freely-signed weight cannot evolve to flee one and approach the other. The cure
//! is to let the perceived signal carry a FEATURE, and the panel's unanimous constraint on that feature is
//! this registry's whole reason to exist:
//!
//! - Each perceivable channel reads exactly ONE floor optical axis of the emitter's surface material, a
//!   DIRECT read ([`crate::physiology::surface_optical_axis`]), never a composite of several axes folded by an
//!   authored weighting. A composite computed upstream of the perceiver's discrimination is a value in the
//!   path of world content that is neither a floor axis nor world data (the value-authoring line), and a
//!   composite hand-tuned to separate one covering material from another is a de-facto material-kind
//!   classifier with no label (the Principle 8 template violation). So combination across axes is FORBIDDEN
//!   here and deferred entirely to selection over the perceiver's per-axis, per-bucket weights: to make more
//!   than one axis matter, a world declares more single-axis channels, never a folded signature. A genuine
//!   multi-axis quantity is admissible only if it is authored as a physics-floor LAW (as the floor's
//!   refractive-contrast law already is), so its output is a floor derivation rather than a perception-layer
//!   recipe.
//! - The read keys on the being's OWN surface material and is ABSENT (zero) when that material declares no
//!   value for the axis, or when the being has no covering-row at all, so a coveringless, plasma, or field
//!   being carries no feature on that channel rather than a synthesized default: the alien is a data row
//!   (admit-the-alien), exactly the graceful-absence convention [`crate::physiology::surface_optical_axis`]
//!   already follows.
//! - The membership is DATA keyed by the floor's optical axis ids, so a new perceivable property is a data
//!   edit, never an enum change or a hardcoded slot (Principle 11). The registry is EMPTY by default, so a
//!   world that declares no perceivable features grows the controller not at all and its run stays
//!   bit-identical: the perceivable feature is opt-in, the emergent-anatomy pattern the biology
//!   ([`crate::percept::PerceptRegistry`]) and material ([`crate::material_percept::MaterialPerceptRegistry`])
//!   percepts established.
//!
//! Which specific optical axis a world arms as the FLEEING carrier is the one genuine owner/physics-floor
//! choice this arc surfaces (a physics-floor ADDITION under Principle 9: a spectral emissivity band, or a
//! reflectance axis once an incident-illumination floor exists), so this substrate is deliberately
//! PARAMETERIZED on "a perceivable optical floor axis" and reads whichever axis ids a world declares. The
//! per-channel value here is the raw emitter surface datum; the perceiver's just-noticeable-difference
//! discrimination of it into buckets, and the heritable freely-signed per-bucket response, are the downstream
//! wire (a following slice). Everything here is integer, fixed-point, and draws no randomness, a pure function
//! of the emitter's body plan and the registry (Principle 3).

use civsim_core::Fixed;

use crate::anatomy::{BodyPlan, BodyPlanRegistry};
use crate::physiology::surface_optical_axis;

/// A perceivable-feature id, minted through the registry (extensible, never a closed enum). The numeric value
/// is the channel's slot in the perceivable-feature order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PerceivableFeatureId(pub u16);

/// One perceivable emitter optical property as data: the ONE floor optical axis id a channel reads off the
/// emitter's surface material. Membership is the floor's data and grows with it (Principle 11). Exactly one
/// axis, never a composite: the axis id names a single chem/optics floor axis (`opt.emissivity`,
/// `opt.refractive_index`, `opt.albedo`, a spectral-emissivity band, whatever a world declares), read directly
/// with no cross-axis fold, so the perceiver's response to it must EMERGE from selection over its per-bucket
/// weights rather than being authored by a signature recipe here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerceivableFeatureDef {
    /// The channel id (its slot in canonical order).
    pub id: PerceivableFeatureId,
    /// The single chem/optics floor optical axis this channel reads off the emitter's surface material.
    pub axis: String,
}

/// The set of emitter optical properties a world's perceivers can sense on a being-signal, data-defined and
/// extensible. EMPTY by default, so a world that declares no perceivable features leaves the controller layout
/// and every run hash unchanged (the perceivable feature is opt-in). Each channel reads exactly one floor
/// optical axis (no composite), the value-authoring-line-clean form step 2b's frame-blind settled.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PerceivableFeatureRegistry {
    channels: Vec<PerceivableFeatureDef>,
}

impl PerceivableFeatureRegistry {
    /// An empty registry: no emitter optical property is sensed, so the controller grows no being-feature block
    /// and the run is bit-identical to a world without the perceivable-feature substrate. The default and the
    /// opt-out.
    pub fn empty() -> PerceivableFeatureRegistry {
        PerceivableFeatureRegistry {
            channels: Vec::new(),
        }
    }

    /// A registry over an explicit ordered list of floor optical axis ids, ids assigned by position
    /// (0, 1, ...). The order is the canonical channel order; a world declares the single-axis properties its
    /// perceivers can sense as data. Each entry is ONE axis: to sense several, list several, never fold them.
    pub fn from_axes(axes: &[&str]) -> PerceivableFeatureRegistry {
        let channels = axes
            .iter()
            .enumerate()
            .map(|(i, &axis)| PerceivableFeatureDef {
                id: PerceivableFeatureId(i as u16),
                axis: axis.to_string(),
            })
            .collect();
        PerceivableFeatureRegistry { channels }
    }

    /// The channels in canonical id order.
    pub fn channels(&self) -> &[PerceivableFeatureDef] {
        &self.channels
    }

    /// The number of perceivable-feature channels.
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// Whether the registry declares no channels (the opt-out: the controller grows no being-feature block).
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    /// The raw emitter feature vector for the emitter whose body plan is `plan`: its surface value on each
    /// declared channel's floor optical axis, in canonical channel order, resolved against the body-plan
    /// registry `bodyplan`. Each entry is a DIRECT single-axis read ([`surface_optical_axis`]), ZERO when the
    /// emitter's surface declares no value for that axis or has no covering-row (the absence convention: the
    /// feature is simply absent, never a synthesized default, so the alien is a data row). No cross-channel
    /// fold: the vector is the per-axis reads side by side, and any combination is left to the perceiver's
    /// per-bucket weights and selection. Pure and RNG-free.
    pub fn read_emitter(&self, plan: &BodyPlan, bodyplan: &BodyPlanRegistry) -> Vec<Fixed> {
        self.channels
            .iter()
            .map(|c| surface_optical_axis(plan, bodyplan, &c.axis))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{BodyPlanRegistry, KindDef, Part, Temperament};

    /// A body-plan registry with one covering carrying two distinct optical axes, and a body plan wearing it.
    fn emitter_with_optics() -> (BodyPlanRegistry, BodyPlan) {
        let mut reg = BodyPlanRegistry::dev_default();
        let cov = reg.coverings.len() as u16;
        let mut material = std::collections::BTreeMap::new();
        material.insert("opt.emissivity".to_string(), Fixed::from_ratio(9, 10));
        material.insert("opt.refractive_index".to_string(), Fixed::from_ratio(3, 2));
        reg.coverings.push(KindDef {
            id: cov,
            name: "test-hide".to_string(),
            fantasy: false,
            geometry: std::collections::BTreeMap::new(),
            material,
        });
        let half = Fixed::from_ratio(1, 2);
        let plan = BodyPlan {
            body_mass: Fixed::ONE,
            encephalization: half,
            diet_breadth: half,
            weapons: vec![],
            covering: Part {
                kind: cov,
                development: Fixed::ONE,
            },
            senses: vec![],
            locomotion: vec![1],
            organs: vec![],
            temperament: Temperament {
                boldness: half,
                exploration: half,
                activity: half,
                sociability: half,
                aggression: Fixed::from_ratio(1, 4),
            },
        };
        (reg, plan)
    }

    #[test]
    fn an_empty_registry_senses_nothing_and_stays_the_opt_out() {
        let reg = PerceivableFeatureRegistry::empty();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        // Even for an emitter carrying optical axes, an empty registry produces an empty feature vector, so the
        // controller grows no block and the run is bit-identical (the opt-in default).
        let (bodyplan, plan) = emitter_with_optics();
        assert!(reg.read_emitter(&plan, &bodyplan).is_empty());
    }

    #[test]
    fn it_reads_each_declared_axis_directly_in_canonical_order_no_composite() {
        let (bodyplan, plan) = emitter_with_optics();
        let reg =
            PerceivableFeatureRegistry::from_axes(&["opt.emissivity", "opt.refractive_index"]);
        assert_eq!(reg.len(), 2);
        // The vector follows registry (declaration) order, each entry the raw direct read of that ONE axis:
        // emissivity 0.9 then refractive index 1.5, side by side, never folded into one signature scalar.
        assert_eq!(
            reg.read_emitter(&plan, &bodyplan),
            vec![Fixed::from_ratio(9, 10), Fixed::from_ratio(3, 2)]
        );
    }

    #[test]
    fn an_axis_the_surface_does_not_declare_reads_zero_and_admits_the_alien() {
        let (bodyplan, plan) = emitter_with_optics();
        // A channel on an axis the covering does not carry reads ZERO (the feature is absent, not defaulted).
        let reg = PerceivableFeatureRegistry::from_axes(&["opt.emissivity", "opt.albedo"]);
        assert_eq!(
            reg.read_emitter(&plan, &bodyplan),
            vec![Fixed::from_ratio(9, 10), Fixed::ZERO]
        );
        // An emitter with no covering-row (an alien surface) reads ZERO on every channel: it carries no
        // feature rather than a rewrite, so it stays a data row.
        let mut alien = plan.clone();
        alien.covering = Part {
            kind: 60000,
            development: Fixed::ONE,
        };
        assert_eq!(
            reg.read_emitter(&alien, &bodyplan),
            vec![Fixed::ZERO, Fixed::ZERO]
        );
    }
}
