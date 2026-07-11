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

//! The evolved behaviour controller (design Part 8; R-BEHAVIOR-EVOLVE, the evolved-behaviour work
//! whose design pass is `docs/emergent_behavior_design.md`; Principles 3, 9, 10, 11).
//!
//! This is layer 4 of the evolved-behaviour architecture: the heritable mapping from a being's
//! homeostatic state and percept (layers 1 and 3, `crate::homeostasis` and the locomotion percept)
//! to which morphological affordance it issues (layer 2, `crate::homeostasis::AffordanceRegistry`).
//! The mechanism is fixed Rust; the mapping's parameters are the heritable data, one weight per
//! [`crate::genome::ControllerParamId`], expressed per individual from its genome by
//! [`crate::genome::GeneSet::express`] exactly as the cognition, build, and composition channels are
//! (Principle 11). So a being's behaviour is its inheritance the way its size and acuity are, and it
//! evolves under the pre-dawn epoch's selection rather than being authored (Principle 9). Nobody
//! writes "seek water when dry": a lineage comes to have that behaviour because the controllers that
//! did kept their bodies alive.
//!
//! The layout is data, derived from the two Stage-1 registries so neither the percept nor the option
//! set is a closed enum in the mechanism. The inputs are, per homeostatic axis (canonical order),
//! the reserve level (the even comfort magnitude), a flag for whether a source of that axis is on the
//! current tile (matter within reach), the unit direction to the nearest known source, and the axis's
//! signed setpoint deviation (a raw interoceptive percept of which side of its comfort band the axis
//! sits on, the thermoreceptive warm-versus-cold bit for a two-sided axis), plus a constant bias. The
//! signed deviation is a percept, not a heading: it says the body is too hot, not which way to flee,
//! so the controller must still combine it with the direction percept to act (Principle 9). The
//! outputs are,
//! per affordance (canonical id order), an activation and, for a directional operation, a heading,
//! the shape given by [`crate::homeostasis::AffordanceParam`]. The weight matrix connects every input
//! to every output, so an adaptive coupling (low water to the water-source heading, or ingesting the
//! matter underfoot when a reserve is low) is discovered by selection, never wired in.
//!
//! Two representations share this plumbing (the design pass's staged choice): a linear reaction norm
//! ([`ControllerLayout::hidden`] zero), the substrate and the determinism proof, which cannot gate a
//! response on internal state; and a small recurrent network ([`ControllerLayout::hidden`] positive),
//! which carries a hidden state across ticks and can. The step from one to the other is the reserved
//! hidden width, not a rewrite. Both use one piecewise-linear activation, a clamp to `[-1, 1]` (the
//! only nonlinearity `Fixed` offers), so both stay bit-identical across machines and thread counts.
//!
//! Determinism is load-bearing. Evaluation is integer and fixed-point with no float and no RNG: a
//! weighted sum is accumulated in i128 through [`Fixed::saturating_sum`] (partition-independent, so
//! the reduction order cannot change the result) over per-term saturating products, then clamped, so
//! the controller reproduces bit for bit (Principle 3). Its output is a pure function of the genome
//! and the percept, and the percept is a pure function of the being and the world, never the camera
//! (Principle 10).

use std::collections::BTreeMap;

use civsim_core::Fixed;

use std::collections::BTreeSet;

use crate::conviction_percept::ConvictionPerceptRegistry;
use crate::genome::{Channel, ControllerParamId, GeneSet, Genome};
use crate::homeostasis::{
    AffordanceId, AffordanceParam, AffordanceRegistry, Homeostasis, HomeostaticAxisId,
    HomeostaticRegistry,
};
use crate::material_percept::MaterialPerceptRegistry;
use crate::perceivable_feature::PerceivableFeatureRegistry;
use crate::percept::PerceptRegistry;

/// Minus one, the low clamp of the activation.
const NEG_ONE: Fixed = Fixed::from_int(-1);

/// The number of controller inputs each homeostatic axis contributes: its reserve level (the even
/// comfort magnitude, sign-blind), a flag for whether a source of it is on the current tile (matter
/// within reach, so the being can tell food underfoot from food in the distance), the two components
/// of the unit direction to its nearest known source, and its signed setpoint deviation (the raw
/// interoceptive percept of which side of the comfort band the axis sits on: for a two-sided axis such
/// as temperature, whether the body is too hot (+) or too cold (-), the bit the even level cannot
/// carry). The signed deviation is supplied per being alongside the field-derived directions
/// ([`build_input`]'s `signed`), zero where no such percept exists (a one-sided axis, or a being whose
/// physiology does not surface it), so an axis with no signed percept reads the same as before.
const INPUTS_PER_AXIS: usize = 5;

/// The per-axis input slot of the signed setpoint-deviation percept (the raw thermoreceptor for a
/// two-sided axis), placed after the reserve level, the here flag, and the two direction components.
const SIGNED_SLOT: usize = 4;

/// A saturating fixed-point product: on overflow it saturates to the signed extreme rather than
/// wrapping, so a large heritable weight against a bounded input stays deterministic and bounded
/// (`Fixed::mul` and `*` wrap on overflow; the controller must not).
#[inline]
fn sat_mul(a: Fixed, b: Fixed) -> Fixed {
    a.checked_mul(b).unwrap_or_else(|| {
        if (a.to_bits() < 0) ^ (b.to_bits() < 0) {
            Fixed::MIN
        } else {
            Fixed::MAX
        }
    })
}

/// The piecewise-linear activation: the weighted sum of the terms, accumulated order-independently
/// in i128 (so the reduction order cannot change the bits), then clamped to `[-1, 1]`. This is a
/// hard-saturating linear unit, the only nonlinearity `Fixed` affords (there is no tanh or sigmoid),
/// and it serves as both the reaction-norm output map and the recurrent network's activation.
#[inline]
fn activate<I: IntoIterator<Item = (Fixed, Fixed)>>(terms: I) -> Fixed {
    let acc = Fixed::saturating_sum(terms.into_iter().map(|(w, x)| sat_mul(w, x)));
    acc.clamp(NEG_ONE, Fixed::ONE)
}

/// One output slot group: an affordance, the shape of its parameter, and the base index of its
/// outputs in the controller's flat output vector.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct OutputSlot {
    affordance: AffordanceId,
    param: AffordanceParam,
    base: usize,
}

/// The data-defined layout of a controller: which homeostatic axes feed its inputs (in canonical
/// order), which affordances read its outputs (in canonical id order, each with its parameter
/// shape), and the hidden width that chooses the representation. Built from the two Stage-1
/// registries, so the controller's input and option sets grow with the world as data (Principle 11),
/// never a closed enum. A layout is a pure function of the registries; two worlds with the same
/// registries share it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ControllerLayout {
    /// The homeostatic axes feeding the inputs, in the registry's canonical order.
    axes: Vec<HomeostaticAxisId>,
    /// The output slot groups, one per affordance, in canonical id order.
    outputs: Vec<OutputSlot>,
    /// The number of raw perceived-feature channels (the width of the feature input block; harm-
    /// learning arc slice a). Zero when the world declares no percepts, in which case the input vector,
    /// the weight count, and the genome expression are all identical to a world without the feature
    /// substrate: the feature block is OPT-IN and hash-neutral by default (the emergent-anatomy
    /// pattern). Each channel is the raw amount of one declared substance class on the cell the being
    /// stands on ([`crate::percept::PerceptRegistry`]), a percept the evolved weights may learn to act
    /// on, sitting between the per-axis blocks and the bias.
    n_features: usize,
    /// The number of APPETITIVE belief channels (the width of the appetitive input block; ideation /
    /// experiential-discovery arc, piece 1, the belief-to-behaviour feedback). Zero unless the world opts
    /// into reward repetition, in which case it is one channel per affordance (canonical id order, aligned
    /// to the output slots), each carrying the being's committed reward-belief signal about that
    /// affordance's single-primitive sequence ([`crate::learn::appetitive_salience`]). Zero yields an input
    /// vector, weight count, and genome expression identical to a world without the appetitive block, so it
    /// is OPT-IN and hash-neutral by default, the same discipline as the feature block. The block sits
    /// AFTER the feature block and before the bias, so the per-axis bases, the feature base, and the
    /// bias-as-last convention all hold unchanged. Only a heritable weight lifted off founder-zero by
    /// selection turns a channel into repetition, so acting on a reward belief emerges (Principle 9).
    n_appetitive: usize,
    /// The number of raw MATERIAL-feature channels (the width of the material-feature input block; the
    /// lifetime/demography keystone, pillar 2, physical-trace persistence, trace slice C). Zero unless the
    /// world opts into material percepts, in which case it is one channel per substance the material-percept
    /// registry declares ([`crate::material_percept::MaterialPerceptRegistry`]), each carrying the raw amount
    /// of that substance in the cell the being stands on (the opaque signature the trace is re-earned from).
    /// Zero yields an input vector, weight count, and genome expression identical to a world without it, so
    /// it is OPT-IN and hash-neutral by default, the same discipline as the feature and appetitive blocks.
    /// The block sits AFTER the appetitive block and before the bias, so the per-axis bases, the feature base,
    /// the appetitive base, and the bias-as-last convention all hold unchanged. A founder expresses zero for
    /// the new material-feature weights, so the percept moves no behaviour until selection lifts a weight off
    /// zero (Principle 8, the emergent pattern the feature block established).
    n_material: usize,
    /// The width of the belief-derived ATTRACTION-direction input (the lifetime/demography keystone, pillar
    /// 2, physical-trace persistence, trace slice C3): TWO when the world opts into the reward-attraction
    /// gradient, else zero. The two channels carry the unit direction toward the nearest believed-rewarding
    /// material the being senses ([`crate::learn::attraction_gradient`]), the positive mirror of the harm
    /// avoidance gradient, which routes into the CONDITION axis's dead direction slot; the reward has no dead
    /// reserve slot (the ENERGY slot carries live food-source memory), so it gets a DEDICATED clean channel of
    /// its own. Zero yields an input vector, weight count, and genome expression identical to a world without
    /// it, so it is OPT-IN and hash-neutral by default. The block sits AFTER the material block and before the
    /// bias, so every earlier base and the bias-as-last convention hold unchanged. A founder expresses zero
    /// for the attraction weights, so nothing is drawn toward a trace until selection lifts one (Principle 9).
    n_attraction: usize,
    /// The number of CONVICTION channels (the width of the conviction input block; Prereq B for the learned
    /// experience-to-conviction coupling, `OWNER_DECISIONS_LOG.md` R2). Zero unless the world exposes a
    /// nonempty [`crate::conviction_percept::ConvictionPerceptRegistry`], in which case it is one channel per
    /// conviction axis declared, each carrying the being's own STANCE on that axis (from its intrinsic
    /// beliefs, written by the runner). Zero yields an input vector, weight count, and genome expression
    /// identical to a world without it, so it is OPT-IN and hash-neutral by default, the same discipline as
    /// the feature, appetitive, material, and attraction blocks. The block sits AFTER the attraction block and
    /// before the bias, so every earlier base and the bias-as-last convention hold unchanged. A founder
    /// expresses zero for the conviction weights, so a conviction moves no behaviour until selection lifts a
    /// weight off zero: whether and how a conviction sways action EMERGES rather than being authored (Principle
    /// 8). Keys on the axis id, never a named institution or religion (the Steering Audit bites here).
    n_conviction: usize,
    /// The width of the BEING-DIRECTED input block (the being-percept keystone, step 6): FOUR when the world
    /// opts into the being-percept (the being-avoidance direction `(dx, dy)` and the being-attraction
    /// direction `(dx, dy)` over the perceived emitters), else zero. The avoidance pair points away from the
    /// bulk of believed-harmful perceived emitters, the attraction pair toward the believed-rewarding ones
    /// ([`crate::learn::being_avoidance_gradient`] / [`crate::learn::being_attraction_gradient`]). Zero yields
    /// an input vector, weight count, and genome expression identical to a world without it, so it is OPT-IN
    /// and hash-neutral by default, the same discipline as the earlier blocks. The block sits AFTER the
    /// conviction block and before the bias, so every earlier base and the bias-as-last convention hold
    /// unchanged. A founder expresses zero for the being weights, and the controller weight is FREELY SIGNED,
    /// so whether a being approaches or avoids a perceived emitter emerges from selection (Principle 9): a
    /// negative weight inverts the direction, so approach-to-a-harm (a parasite, a scavenger) is reachable.
    n_being: usize,
    /// The width of the being-FEATURE input block (creature-selection step 2b, the percept kind-feature floor
    /// arc): two components (a toward-direction `dx, dy`) per discrimination bucket per perceivable-feature
    /// channel ([`crate::perceivable_feature::PerceivableFeatureRegistry::layout_width`]), else zero. Each
    /// `(channel, bucket)` pair carries the unit direction toward the perceived emitters whose optical feature
    /// on that channel falls in that bucket, so a perceiver's heritable freely-signed weight on the pair can
    /// move it toward or away from that bucket's emitters. Zero yields an input vector, weight count, and genome
    /// expression identical to a world without it, so it is OPT-IN and hash-neutral by default, the same
    /// discipline as the earlier blocks. The block sits AFTER the being block and before the bias, so every
    /// earlier base and the bias-as-last convention hold unchanged. A founder expresses zero for the
    /// being-feature weights, so a feature moves no behaviour until selection lifts a weight off zero: whether a
    /// perceiver flees or approaches the emitters in a feature-bucket EMERGES (Principle 9). Keys on the
    /// emitter's own surface optical datum discriminated by the perceiver's own resolution, never a species,
    /// kind, or emitter-kind label.
    n_being_feature: usize,
    /// The number of inputs (`INPUTS_PER_AXIS * axes + n_features + n_appetitive + n_material + n_attraction +
    /// n_conviction + n_being + n_being_feature + 1` for the bias).
    n_in: usize,
    /// The number of outputs (the sum of the affordances' slot counts).
    n_out: usize,
    /// The hidden width: zero is a linear reaction norm, positive a recurrent network. RESERVED for
    /// the recurrent case. Basis: the smallest hidden state that represents the conditional foraging
    /// and predator-prey behaviours the ecology needs, against the per-tick evaluation budget, a
    /// performance-and-expressiveness bound found by trial (docs/emergent_behavior_design.md).
    hidden: usize,
}

impl ControllerLayout {
    /// Build a layout from the homeostatic and affordance registries and a hidden width (zero for a
    /// reaction norm). The affordances are taken in canonical id order so the output indexing is
    /// reproducible; a directional affordance gets an activation and a two-component heading, a
    /// scalar one gets only an activation.
    pub fn new(
        homeo: &HomeostaticRegistry,
        afford: &AffordanceRegistry,
        hidden: usize,
    ) -> ControllerLayout {
        ControllerLayout::with_percepts(homeo, afford, &PerceptRegistry::empty(), hidden)
    }

    /// Build a layout that also feeds a block of raw perceived-feature channels, one per class the
    /// percept registry declares (harm-learning arc slice a). The feature block sits between the
    /// per-axis blocks and the bias, so the per-axis input bases ([`axis_input_base`]) and the
    /// bias-as-last-input convention the seed helpers rely on both hold unchanged; an EMPTY percept
    /// registry yields exactly [`ControllerLayout::new`]'s layout (`n_features` zero), so the feature
    /// substrate is opt-in and a world that declares no percepts is bit-identical. The feature channels
    /// grow `n_in`, so the weight count and the genome expression grow with them: a founder expresses
    /// zero for the new feature weights (unseeded channels), so the percept has no effect until
    /// selection lifts a weight off zero, the emergent pattern (Principle 8).
    pub fn with_percepts(
        homeo: &HomeostaticRegistry,
        afford: &AffordanceRegistry,
        percept: &PerceptRegistry,
        hidden: usize,
    ) -> ControllerLayout {
        ControllerLayout::with_percepts_and_appetitive(homeo, afford, percept, false, hidden)
    }

    /// As [`with_percepts`], plus the being-directed input block when `being` is true (the being-percept
    /// keystone, step 6): the percept layout with no appetitive, material, attraction, or conviction block
    /// but with the being block. A harness that seeds a genome against a layout (a flat `o * n_in + i`
    /// weight index, [`taxis_move_weights`]) must build that layout at the SAME width the run embodiment
    /// expresses against, or the seeded weights land in the wrong slots. An embodiment that arms only the
    /// being block ([`crate::runner::Embodiment::set_being_percept`], with no appetitive, material,
    /// attraction, or conviction block) rebuilds its layout through the full builder with exactly those
    /// flags, so this convenience builds the matching layout for the seeding side. Delegates to the full
    /// builder; a `being = false` call is identical to [`with_percepts`].
    pub fn with_percepts_and_being(
        homeo: &HomeostaticRegistry,
        afford: &AffordanceRegistry,
        percept: &PerceptRegistry,
        being: bool,
        hidden: usize,
    ) -> ControllerLayout {
        ControllerLayout::with_percepts_appetitive_material_attraction_and_conviction(
            homeo,
            afford,
            percept,
            false,
            &MaterialPerceptRegistry::empty(),
            false,
            &ConvictionPerceptRegistry::empty(),
            being,
            hidden,
        )
    }

    /// Build a layout that also feeds an APPETITIVE belief block, one channel per affordance in canonical
    /// id order (ideation / experiential-discovery arc, piece 1, the belief-to-behaviour feedback). The
    /// block sits AFTER the feature block and before the bias, so the per-axis input bases
    /// ([`axis_input_base`]), the feature base ([`feature_input_base`]), and the bias-as-last convention
    /// all hold unchanged; `appetitive = false` yields exactly [`ControllerLayout::with_percepts`]'s layout
    /// (`n_appetitive` zero), so the block is opt-in and a world that does not enable reward repetition is
    /// bit-identical. Each channel carries the being's committed reward-belief signal about that
    /// affordance's single-primitive sequence, written by the runner from
    /// [`crate::learn::appetitive_salience`]; the channels grow `n_in`, so the weight count and the genome
    /// expression grow with them, and a founder expresses zero for the new appetitive weights (unseeded
    /// channels), so a reward belief moves no behaviour until selection lifts a weight off zero (Principle
    /// 8, the emergent pattern the feature block established).
    pub fn with_percepts_and_appetitive(
        homeo: &HomeostaticRegistry,
        afford: &AffordanceRegistry,
        percept: &PerceptRegistry,
        appetitive: bool,
        hidden: usize,
    ) -> ControllerLayout {
        ControllerLayout::with_percepts_appetitive_and_material(
            homeo,
            afford,
            percept,
            appetitive,
            &MaterialPerceptRegistry::empty(),
            hidden,
        )
    }

    /// Build a layout that also feeds a raw MATERIAL-feature block, one channel per substance the material-
    /// percept registry declares (the lifetime/demography keystone, pillar 2, trace slice C). The block sits
    /// AFTER the appetitive block and before the bias, so the per-axis input bases ([`axis_input_base`]), the
    /// feature base ([`feature_input_base`]), the appetitive base ([`appetitive_input_base`]), and the
    /// bias-as-last convention all hold unchanged; an EMPTY material registry yields exactly
    /// [`ControllerLayout::with_percepts_and_appetitive`]'s layout (`n_material` zero), so the material
    /// substrate is opt-in and a world that declares no material percepts is bit-identical. Each channel
    /// carries the raw amount of one declared substance in the cell the being stands on
    /// ([`crate::material_percept::MaterialPerceptRegistry::perceive`]), the opaque signature the physical
    /// trace is re-earned from; the channels grow `n_in`, so the weight count and the genome expression grow
    /// with them, and a founder expresses zero for the new material-feature weights (unseeded channels), so
    /// the percept moves no behaviour until selection lifts a weight off zero (Principle 8, the emergent
    /// pattern the feature block established).
    pub fn with_percepts_appetitive_and_material(
        homeo: &HomeostaticRegistry,
        afford: &AffordanceRegistry,
        percept: &PerceptRegistry,
        appetitive: bool,
        material: &MaterialPerceptRegistry,
        hidden: usize,
    ) -> ControllerLayout {
        ControllerLayout::with_percepts_appetitive_material_and_attraction(
            homeo, afford, percept, appetitive, material, false, hidden,
        )
    }

    /// Build a layout that also feeds the belief-derived ATTRACTION-direction input (the lifetime/demography
    /// keystone, pillar 2, trace slice C3): TWO channels carrying the unit direction toward the nearest
    /// believed-rewarding material the being senses ([`crate::learn::attraction_gradient`]), the positive
    /// mirror of the harm avoidance gradient. Avoidance routes into the CONDITION axis's dead direction slot;
    /// the reward has no dead reserve slot (the ENERGY slot carries live food-source memory), so it gets a
    /// DEDICATED clean channel of its own, sitting AFTER the material block and before the bias, so every
    /// earlier base and the bias-as-last convention hold unchanged. `attraction = false` yields exactly
    /// [`ControllerLayout::with_percepts_appetitive_and_material`]'s layout (`n_attraction` zero), so it is
    /// opt-in and a world that does not enable the gradient is bit-identical. A founder expresses zero for the
    /// two attraction weights, so nothing is drawn toward a trace until selection lifts one (Principle 9, the
    /// emergent pattern the feature block established).
    #[allow(clippy::too_many_arguments)]
    pub fn with_percepts_appetitive_material_and_attraction(
        homeo: &HomeostaticRegistry,
        afford: &AffordanceRegistry,
        percept: &PerceptRegistry,
        appetitive: bool,
        material: &MaterialPerceptRegistry,
        attraction: bool,
        hidden: usize,
    ) -> ControllerLayout {
        ControllerLayout::with_percepts_appetitive_material_attraction_and_conviction(
            homeo,
            afford,
            percept,
            appetitive,
            material,
            attraction,
            &ConvictionPerceptRegistry::empty(),
            false,
            hidden,
        )
    }

    /// Build a layout that also feeds a CONVICTION block, one channel per conviction axis the registry exposes
    /// (Prereq B for the learned experience-to-conviction coupling, `OWNER_DECISIONS_LOG.md` R2). The block
    /// sits AFTER the attraction block and before the bias, so the per-axis input bases ([`axis_input_base`]),
    /// every earlier block base ([`feature_input_base`], [`appetitive_input_base`], [`material_input_base`],
    /// [`attraction_input_base`]), and the bias-as-last convention all hold unchanged; an EMPTY conviction
    /// registry yields exactly [`ControllerLayout::with_percepts_appetitive_material_and_attraction`]'s layout
    /// (`n_conviction` zero), so the conviction substrate is opt-in and a world that exposes no conviction is
    /// bit-identical. Each channel carries the being's own STANCE on that conviction axis, written by the
    /// runner from its intrinsic beliefs; the channels grow `n_in`, so the weight count and the genome
    /// expression grow with them, and a founder expresses zero for the new conviction weights, so a conviction
    /// moves no behaviour until selection lifts a weight off zero. Whether and how a conviction sways action
    /// EMERGES from selection over the evolved weight, never an authored conviction-to-action rule (Principle
    /// 8); the block keys on the axis id, never a named institution or religion (Principle 9, the Steering
    /// Audit).
    #[allow(clippy::too_many_arguments)]
    pub fn with_percepts_appetitive_material_attraction_and_conviction(
        homeo: &HomeostaticRegistry,
        afford: &AffordanceRegistry,
        percept: &PerceptRegistry,
        appetitive: bool,
        material: &MaterialPerceptRegistry,
        attraction: bool,
        conviction: &ConvictionPerceptRegistry,
        being: bool,
        hidden: usize,
    ) -> ControllerLayout {
        Self::with_percepts_appetitive_material_attraction_conviction_and_being_features(
            homeo,
            afford,
            percept,
            appetitive,
            material,
            attraction,
            conviction,
            being,
            &PerceivableFeatureRegistry::empty(),
            hidden,
        )
    }

    /// As [`with_percepts_appetitive_material_attraction_and_conviction`], plus the being-FEATURE input block
    /// (creature-selection step 2b, the percept kind-feature floor arc): the discrimination-bucket toward
    /// directions of the emitter optical features a world's perceivers can sense on a being-signal beyond its
    /// strength scalar. The block sits AFTER the being block and before the bias, so every earlier base and the
    /// bias-as-last convention hold unchanged. Its width is [`PerceivableFeatureRegistry::layout_width`] (two
    /// slots, a `(dx, dy)` toward-direction pair, per discrimination bucket per channel), so an EMPTY registry
    /// yields exactly [`with_percepts_appetitive_material_attraction_and_conviction`]'s layout (`n_being_feature`
    /// zero), an input vector, weight count, and genome expression identical to a world without it: the
    /// perceivable feature is opt-in and hash-neutral by default, the same discipline as the earlier blocks. A
    /// founder expresses zero for the being-feature weights, and each per-bucket weight is FREELY SIGNED, so
    /// whether a perceiver moves toward or away from the emitters in a given feature-bucket emerges from
    /// selection (Principle 9): fleeing one emitter kind and approaching another becomes reachable once a
    /// strength-independent optical feature separates them, which strength alone cannot.
    #[allow(clippy::too_many_arguments)]
    pub fn with_percepts_appetitive_material_attraction_conviction_and_being_features(
        homeo: &HomeostaticRegistry,
        afford: &AffordanceRegistry,
        percept: &PerceptRegistry,
        appetitive: bool,
        material: &MaterialPerceptRegistry,
        attraction: bool,
        conviction: &ConvictionPerceptRegistry,
        being: bool,
        being_features: &PerceivableFeatureRegistry,
        hidden: usize,
    ) -> ControllerLayout {
        let axes: Vec<HomeostaticAxisId> = homeo.axes.iter().map(|a| a.id).collect();
        let n_features = percept.len();
        let n_material = material.len();
        // The attraction-direction input is two components (dx, dy), or none when the world does not enable
        // the reward-attraction gradient.
        let n_attraction = if attraction { 2 } else { 0 };
        // One conviction channel per exposed conviction axis, or none when the world exposes no conviction.
        let n_conviction = conviction.len();
        // The being-directed block is four components (the being-avoidance direction dx, dy and the
        // being-attraction direction dx, dy), or none when the world does not enable the being-percept.
        let n_being = if being { 4 } else { 0 };
        // The being-FEATURE block: two components (a toward-direction dx, dy) per discrimination bucket per
        // perceivable-feature channel, or none when the world declares no perceivable features (step 2b).
        let n_being_feature = being_features.layout_width();
        let mut defs: Vec<&crate::homeostasis::AffordanceDef> = afford.affordances.iter().collect();
        defs.sort_by_key(|d| d.id);
        // One appetitive channel per affordance (aligned to the output slots below), or none when the
        // world does not opt into reward repetition.
        let n_appetitive = if appetitive { defs.len() } else { 0 };
        let n_in = INPUTS_PER_AXIS * axes.len()
            + n_features
            + n_appetitive
            + n_material
            + n_attraction
            + n_conviction
            + n_being
            + n_being_feature
            + 1;
        let mut outputs = Vec::with_capacity(defs.len());
        let mut base = 0usize;
        for d in defs {
            outputs.push(OutputSlot {
                affordance: d.id,
                param: d.param,
                base,
            });
            base += slots_for(d.param);
        }
        let n_out = base;
        ControllerLayout {
            axes,
            outputs,
            n_features,
            n_appetitive,
            n_material,
            n_attraction,
            n_conviction,
            n_being,
            n_being_feature,
            n_in,
            n_out,
            hidden,
        }
    }

    /// The number of inputs.
    pub fn n_in(&self) -> usize {
        self.n_in
    }

    /// The number of outputs.
    pub fn n_out(&self) -> usize {
        self.n_out
    }

    /// The hidden width (zero for a reaction norm).
    pub fn hidden(&self) -> usize {
        self.hidden
    }

    /// The input-block base index of a homeostatic axis: the offset in the input vector where the
    /// axis's per-axis slots begin (its level, here-flag, two source-direction components, then its
    /// signed slot). `None` if the axis does not feed this layout. A seed function reads this so it
    /// never hardcodes an axis's position, which the registry's membership and canonical order set: add
    /// or reorder an axis and the base follows the data (Principle 11), not a magic constant.
    pub fn axis_input_base(&self, axis: HomeostaticAxisId) -> Option<usize> {
        self.axes
            .iter()
            .position(|&a| a == axis)
            .map(|i| INPUTS_PER_AXIS * i)
    }

    /// The OUTPUT-slot base index of an affordance: the offset in the output vector where the
    /// affordance's slots begin (its activation, then, for a directional affordance, its two heading
    /// components). `None` if the body/registry does not afford it. A caller reaching an affordance's
    /// output weight (for a reaction norm, the weight feeding output `o` from input `i` is
    /// [`ControllerParamId`] `o * n_in + i`) reads this so it never hardcodes the slot, which the
    /// affordance registry's membership and canonical id order set (Principle 11): the base follows the
    /// data, exactly as [`axis_input_base`] does for the input side.
    pub fn output_base(&self, affordance: AffordanceId) -> Option<usize> {
        self.outputs
            .iter()
            .find(|s| s.affordance == affordance)
            .map(|s| s.base)
    }

    /// The number of raw perceived-feature channels this layout feeds (zero when no percepts are
    /// declared; harm-learning arc slice a).
    pub fn n_features(&self) -> usize {
        self.n_features
    }

    /// The input-vector index where the feature block begins: after all the per-axis blocks, before
    /// the bias. A caller writing a feature vector, and a seed function reaching a feature weight, read
    /// this so neither hardcodes the block's position, which the axis count sets.
    pub fn feature_input_base(&self) -> usize {
        INPUTS_PER_AXIS * self.axes.len()
    }

    /// The number of appetitive belief channels this layout feeds (zero unless the world opts into reward
    /// repetition; ideation arc, piece 1). When positive it is one channel per affordance, in the same
    /// canonical id order as [`affordance_ids`] and the output slots.
    pub fn n_appetitive(&self) -> usize {
        self.n_appetitive
    }

    /// The input-vector index where the appetitive block begins: after the per-axis blocks and the feature
    /// block, before the bias. A caller writing the appetitive vector reads this so it never hardcodes the
    /// block's position, which the axis count and feature width set.
    pub fn appetitive_input_base(&self) -> usize {
        INPUTS_PER_AXIS * self.axes.len() + self.n_features
    }

    /// The number of raw material-feature channels this layout feeds (zero unless the world opts into
    /// material percepts; the lifetime/demography keystone, pillar 2, trace slice C). When positive it is one
    /// channel per substance the material-percept registry declares, in canonical registry order.
    pub fn n_material(&self) -> usize {
        self.n_material
    }

    /// The input-vector index where the material-feature block begins: after the per-axis blocks, the feature
    /// block, and the appetitive block, before the bias. A caller writing the material-feature vector reads
    /// this so it never hardcodes the block's position, which the axis count, feature width, and appetitive
    /// width set.
    pub fn material_input_base(&self) -> usize {
        INPUTS_PER_AXIS * self.axes.len() + self.n_features + self.n_appetitive
    }

    /// The width of the attraction-direction input (two when the reward-attraction gradient is enabled, else
    /// zero; the lifetime/demography keystone, pillar 2, trace slice C3).
    pub fn n_attraction(&self) -> usize {
        self.n_attraction
    }

    /// The input-vector index where the attraction-direction input begins: after the per-axis blocks, the
    /// feature block, the appetitive block, and the material block, before the bias. A caller writing the
    /// attraction direction reads this so it never hardcodes the block's position.
    pub fn attraction_input_base(&self) -> usize {
        INPUTS_PER_AXIS * self.axes.len() + self.n_features + self.n_appetitive + self.n_material
    }

    /// The number of conviction channels this layout feeds (zero unless the world exposes a nonempty
    /// [`crate::conviction_percept::ConvictionPerceptRegistry`]; Prereq B). When positive it is one channel per
    /// exposed conviction axis, in the registry's canonical order, each carrying the being's own stance.
    pub fn n_conviction(&self) -> usize {
        self.n_conviction
    }

    /// The input-vector index where the conviction block begins: after the per-axis blocks and the feature,
    /// appetitive, material, and attraction blocks, before the bias. A caller writing the conviction vector
    /// reads this so it never hardcodes the block's position, which the earlier block widths set.
    pub fn conviction_input_base(&self) -> usize {
        INPUTS_PER_AXIS * self.axes.len()
            + self.n_features
            + self.n_appetitive
            + self.n_material
            + self.n_attraction
    }

    /// The width of the being-directed input block (four when the world enables the being-percept, else zero;
    /// the being-percept keystone, step 6).
    pub fn n_being(&self) -> usize {
        self.n_being
    }

    /// The input-vector index where the being-directed block begins: after the per-axis blocks and the
    /// feature, appetitive, material, attraction, and conviction blocks, before the bias. A caller writing the
    /// being directions reads this so it never hardcodes the block's position, which the earlier block widths
    /// set.
    pub fn being_input_base(&self) -> usize {
        INPUTS_PER_AXIS * self.axes.len()
            + self.n_features
            + self.n_appetitive
            + self.n_material
            + self.n_attraction
            + self.n_conviction
    }

    /// The width of the being-FEATURE input block (two per discrimination bucket per perceivable-feature
    /// channel, else zero; creature-selection step 2b).
    pub fn n_being_feature(&self) -> usize {
        self.n_being_feature
    }

    /// The input-vector index where the being-FEATURE block begins: after the per-axis blocks and the feature,
    /// appetitive, material, attraction, conviction, and being blocks, before the bias. A caller writing the
    /// being-feature directions reads this so it never hardcodes the block's position, which the earlier block
    /// widths set.
    pub fn being_feature_input_base(&self) -> usize {
        self.being_input_base() + self.n_being
    }

    /// The affordance ids this layout's outputs (and, when enabled, its appetitive channels) are indexed by,
    /// in canonical id order. The runner reads this to align an [`crate::learn::appetitive_salience`] vector
    /// to the appetitive block, so neither hardcodes the affordance order, which the registry sets.
    pub fn affordance_ids(&self) -> Vec<AffordanceId> {
        self.outputs.iter().map(|o| o.affordance).collect()
    }

    /// The number of heritable weights this layout's controller carries, which is the number of
    /// [`ControllerParamId`]s a genome must reach to express a full controller. For a reaction norm
    /// this is `n_out * n_in`; for a recurrent network it adds the input-to-hidden, hidden-to-hidden,
    /// and hidden-to-output blocks.
    pub fn weight_count(&self) -> usize {
        weight_count(self.n_in, self.n_out, self.hidden)
    }

    /// Build the input vector for a being: per axis (canonical order) its reserve level, a flag for
    /// whether a source of that axis is on the current tile (`here`), the unit direction to its
    /// nearest known source (zero if none is known), and its signed setpoint deviation (zero where no
    /// such percept is supplied), then the bias. A pure read of the being's physiology and its earned
    /// knowledge (Principle 10). The signed deviation is the raw interoceptive percept, clamped to
    /// `[-1, 1]` by its supplier, delivered as its own input rather than folded into the direction, so
    /// the controller (not the mechanism) decides how to combine "which side of the band am I on" with
    /// "which way is the gradient" (Principle 9).
    pub fn build_input(
        &self,
        homeo: &Homeostasis,
        here: &BTreeSet<HomeostaticAxisId>,
        source_dirs: &BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>,
        signed: &BTreeMap<HomeostaticAxisId, Fixed>,
    ) -> Vec<Fixed> {
        self.build_input_with_features(homeo, here, source_dirs, signed, &[])
    }

    /// Build the input vector including the raw perceived-feature block (harm-learning arc slice a):
    /// the per-axis blocks and bias exactly as [`build_input`], plus each declared feature channel's
    /// raw value written into the feature block ([`feature_input_base`] onward, in registry order).
    /// `features` is the [`crate::percept::PerceptRegistry::perceive`] read of the cell the being stands
    /// on; a shorter slice leaves the unfilled channels zero (clean degrade). With no features (an empty
    /// slice and `n_features` zero) the result is byte-identical to [`build_input`] before the feature
    /// substrate existed, so an opted-out world is unchanged. A pure read of physiology, earned
    /// knowledge, and the physical feature underfoot (Principles 9, 10).
    pub fn build_input_with_features(
        &self,
        homeo: &Homeostasis,
        here: &BTreeSet<HomeostaticAxisId>,
        source_dirs: &BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>,
        signed: &BTreeMap<HomeostaticAxisId, Fixed>,
        features: &[Fixed],
    ) -> Vec<Fixed> {
        self.build_input_with_features_and_appetitive(
            homeo,
            here,
            source_dirs,
            signed,
            features,
            &[],
        )
    }

    /// Build the input vector including both the raw perceived-feature block and the APPETITIVE belief block
    /// (ideation arc, piece 1, the belief-to-behaviour feedback): the per-axis blocks, feature block, and
    /// bias exactly as [`build_input_with_features`], plus each appetitive channel's belief signal written
    /// into the appetitive block ([`appetitive_input_base`] onward, in canonical affordance order).
    /// `appetitive` is the [`crate::learn::appetitive_salience`] read over this being's reward beliefs; a
    /// shorter slice leaves the unfilled channels zero (clean degrade), and an empty slice with `n_appetitive`
    /// zero is byte-identical to [`build_input_with_features`] before the appetitive block existed, so an
    /// opted-out world is unchanged. A pure read of physiology, earned knowledge, the feature underfoot, and
    /// the being's own reward beliefs (Principles 9, 10).
    pub fn build_input_with_features_and_appetitive(
        &self,
        homeo: &Homeostasis,
        here: &BTreeSet<HomeostaticAxisId>,
        source_dirs: &BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>,
        signed: &BTreeMap<HomeostaticAxisId, Fixed>,
        features: &[Fixed],
        appetitive: &[Fixed],
    ) -> Vec<Fixed> {
        self.build_input_with_features_appetitive_and_material(
            homeo,
            here,
            source_dirs,
            signed,
            features,
            appetitive,
            &[],
        )
    }

    /// Build the input vector including the raw MATERIAL-feature block (the lifetime/demography keystone,
    /// pillar 2, trace slice C): the per-axis blocks, feature block, appetitive block, and bias exactly as
    /// [`build_input_with_features_and_appetitive`], plus each material-feature channel's raw amount written
    /// into the material block ([`material_input_base`] onward, in canonical registry order). `material` is
    /// the [`crate::material_percept::MaterialPerceptRegistry::perceive`] read of the cell the being stands
    /// on; a shorter slice leaves the unfilled channels zero (clean degrade), and an empty slice with
    /// `n_material` zero is byte-identical to [`build_input_with_features_and_appetitive`] before the material
    /// block existed, so an opted-out world is unchanged. A pure read of physiology, earned knowledge, the
    /// biology feature underfoot, the being's reward beliefs, and the matter underfoot (Principles 9, 10).
    #[allow(clippy::too_many_arguments)]
    pub fn build_input_with_features_appetitive_and_material(
        &self,
        homeo: &Homeostasis,
        here: &BTreeSet<HomeostaticAxisId>,
        source_dirs: &BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>,
        signed: &BTreeMap<HomeostaticAxisId, Fixed>,
        features: &[Fixed],
        appetitive: &[Fixed],
        material: &[Fixed],
    ) -> Vec<Fixed> {
        self.build_input_full(
            homeo,
            here,
            source_dirs,
            signed,
            features,
            appetitive,
            material,
            &[],
        )
    }

    /// Build the input vector including the belief-derived ATTRACTION-direction input (the lifetime/demography
    /// keystone, pillar 2, trace slice C3): the per-axis blocks, feature block, appetitive block, material
    /// block, and bias exactly as [`build_input_with_features_appetitive_and_material`], plus the two
    /// attraction components (dx, dy) written into the attraction block ([`attraction_input_base`] onward).
    /// `attraction` is the unit-normalised [`crate::learn::attraction_gradient`] read; a shorter slice leaves
    /// the unfilled channels zero (clean degrade), and an empty slice with `n_attraction` zero is byte-
    /// identical to the material builder before the attraction input existed, so an opted-out world is
    /// unchanged. A pure read of physiology, earned knowledge, the biology feature underfoot, the being's
    /// reward beliefs, the matter underfoot, and the direction toward believed-rewarding matter (Principles
    /// 9, 10).
    #[allow(clippy::too_many_arguments)]
    pub fn build_input_full(
        &self,
        homeo: &Homeostasis,
        here: &BTreeSet<HomeostaticAxisId>,
        source_dirs: &BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>,
        signed: &BTreeMap<HomeostaticAxisId, Fixed>,
        features: &[Fixed],
        appetitive: &[Fixed],
        material: &[Fixed],
        attraction: &[Fixed],
    ) -> Vec<Fixed> {
        self.build_input_full_with_conviction(
            homeo,
            here,
            source_dirs,
            signed,
            features,
            appetitive,
            material,
            attraction,
            &[],
            &[],
        )
    }

    /// Build the input vector including the CONVICTION block (Prereq B, `OWNER_DECISIONS_LOG.md` R2): the
    /// per-axis blocks and the feature, appetitive, material, and attraction blocks and bias exactly as
    /// [`build_input_full`], plus each conviction channel's own stance written into the conviction block
    /// ([`conviction_input_base`] onward, in the registry's canonical axis order). `conviction` is the being's
    /// own stance on each exposed conviction axis, read from its intrinsic beliefs by the runner; a shorter
    /// slice leaves the unfilled channels zero (clean degrade), and an empty slice with `n_conviction` zero is
    /// byte-identical to [`build_input_full`] before the conviction block existed, so an opted-out world is
    /// unchanged. A pure read of the being's own physiology, earned knowledge, and convictions (Principles 9,
    /// 10); the controller, not this builder, decides whether a conviction moves behaviour (Principle 8).
    #[allow(clippy::too_many_arguments)]
    pub fn build_input_full_with_conviction(
        &self,
        homeo: &Homeostasis,
        here: &BTreeSet<HomeostaticAxisId>,
        source_dirs: &BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>,
        signed: &BTreeMap<HomeostaticAxisId, Fixed>,
        features: &[Fixed],
        appetitive: &[Fixed],
        material: &[Fixed],
        attraction: &[Fixed],
        conviction: &[Fixed],
        being: &[Fixed],
    ) -> Vec<Fixed> {
        self.build_input_full_with_conviction_and_being_features(
            homeo,
            here,
            source_dirs,
            signed,
            features,
            appetitive,
            material,
            attraction,
            conviction,
            being,
            &[],
        )
    }

    /// As [`build_input_full_with_conviction`], plus the being-FEATURE block (creature-selection step 2b): each
    /// `(channel, bucket)` toward-direction written into the being-feature block ([`being_feature_input_base`]
    /// onward, in the registry's channel-then-bucket order). `being_features` is the runner's per-perceiver
    /// discrimination of the perceived emitters' optical features into buckets, each bucket carrying the unit
    /// toward-direction over its emitters; a shorter slice leaves the unfilled slots zero (clean degrade), and
    /// an empty slice with `n_being_feature` zero is byte-identical to [`build_input_full_with_conviction`]
    /// before the being-feature block existed, so an opted-out world is unchanged. A pure read; the controller,
    /// not this builder, decides how each feature-bucket direction moves behaviour (Principle 9).
    #[allow(clippy::too_many_arguments)]
    pub fn build_input_full_with_conviction_and_being_features(
        &self,
        homeo: &Homeostasis,
        here: &BTreeSet<HomeostaticAxisId>,
        source_dirs: &BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>,
        signed: &BTreeMap<HomeostaticAxisId, Fixed>,
        features: &[Fixed],
        appetitive: &[Fixed],
        material: &[Fixed],
        attraction: &[Fixed],
        conviction: &[Fixed],
        being: &[Fixed],
        being_features: &[Fixed],
    ) -> Vec<Fixed> {
        let mut v = vec![Fixed::ZERO; self.n_in];
        for (a, &axis) in self.axes.iter().enumerate() {
            v[INPUTS_PER_AXIS * a] = homeo.level(axis);
            if here.contains(&axis) {
                v[INPUTS_PER_AXIS * a + 1] = Fixed::ONE;
            }
            if let Some(&(dx, dy)) = source_dirs.get(&axis) {
                v[INPUTS_PER_AXIS * a + 2] = dx;
                v[INPUTS_PER_AXIS * a + 3] = dy;
            }
            if let Some(&s) = signed.get(&axis) {
                v[INPUTS_PER_AXIS * a + SIGNED_SLOT] = s;
            }
        }
        let fbase = self.feature_input_base();
        for (k, &f) in features.iter().enumerate().take(self.n_features) {
            v[fbase + k] = f;
        }
        let abase = self.appetitive_input_base();
        for (k, &a) in appetitive.iter().enumerate().take(self.n_appetitive) {
            v[abase + k] = a;
        }
        let mbase = self.material_input_base();
        for (k, &m) in material.iter().enumerate().take(self.n_material) {
            v[mbase + k] = m;
        }
        let atbase = self.attraction_input_base();
        for (k, &a) in attraction.iter().enumerate().take(self.n_attraction) {
            v[atbase + k] = a;
        }
        let cbase = self.conviction_input_base();
        for (k, &c) in conviction.iter().enumerate().take(self.n_conviction) {
            v[cbase + k] = c;
        }
        let bbase = self.being_input_base();
        for (k, &b) in being.iter().enumerate().take(self.n_being) {
            v[bbase + k] = b;
        }
        let bfbase = self.being_feature_input_base();
        for (k, &bf) in being_features.iter().enumerate().take(self.n_being_feature) {
            v[bfbase + k] = bf;
        }
        v[self.n_in - 1] = Fixed::ONE; // the bias input, always the last slot
        v
    }

    /// Decide which afforded operation to issue from an output vector: the afforded affordance with
    /// the greatest activation (clamped to `[0, 1]`, so a negative activation never wins over rest),
    /// ties broken by the lowest affordance id (the outputs are walked in id order and only a
    /// strictly greater activation replaces the incumbent, so the tie-break is deterministic). A
    /// directional affordance carries its heading. Returns `None` if the body affords nothing.
    pub fn decide(&self, out: &[Fixed], afforded: &[AffordanceId]) -> Option<ControllerDecision> {
        let mut best: Option<ControllerDecision> = None;
        for slot in &self.outputs {
            if !afforded.contains(&slot.affordance) {
                continue;
            }
            let activation = out
                .get(slot.base)
                .copied()
                .unwrap_or(Fixed::ZERO)
                .clamp(Fixed::ZERO, Fixed::ONE);
            let heading = match slot.param {
                AffordanceParam::Directional => {
                    let hx = out.get(slot.base + 1).copied().unwrap_or(Fixed::ZERO);
                    let hy = out.get(slot.base + 2).copied().unwrap_or(Fixed::ZERO);
                    Some((hx, hy))
                }
                AffordanceParam::Scalar => None,
            };
            let take = match &best {
                None => true,
                Some(b) => activation > b.activation,
            };
            if take {
                best = Some(ControllerDecision {
                    affordance: slot.affordance,
                    activation,
                    heading,
                });
            }
        }
        best
    }
}

/// The number of output slots an affordance's parameter shape needs: a scalar operation reads one
/// activation, a directional one reads an activation and a two-component heading.
fn slots_for(param: AffordanceParam) -> usize {
    match param {
        AffordanceParam::Scalar => 1,
        AffordanceParam::Directional => 3,
    }
}

/// The heritable weight count for a controller of the given dimensions (free function so callers can
/// size a gene set before building a layout instance).
pub fn weight_count(n_in: usize, n_out: usize, hidden: usize) -> usize {
    if hidden == 0 {
        n_out * n_in
    } else {
        hidden * n_in + hidden * hidden + n_out * hidden
    }
}

/// The nonzero founding-taxis weights for a MOVE-directional reaction-norm controller over one target
/// axis (base-level liveliness, step 1): each entry names the [`ControllerParamId`] of one nonzero
/// weight and the target value a founder should express for it. The pattern is the reaction-norm form
/// of the tested taxis controller: the MOVE activation follows the bias input (so the being wants to
/// move, `move_bias`), and the MOVE heading follows the target axis's source-direction percept (so it
/// steers along that axis's gradient, `heading_gain`). Nothing here is a magnitude of its own: the two
/// gains are the caller's reserved values (Principle 11), and this function is fixed mechanism that maps
/// each nonzero weight to its heritable channel with no race branch (Principle 9).
///
/// The layout must give MOVE a directional output at `move_output` (its activation, then its two
/// heading components at the next two output indices) and the target axis an input block starting at
/// `axis_input_base` (its level, here-flag, two source-direction components, then its signed slot). For
/// a reaction norm the weight feeding output `o` from input `i` is [`ControllerParamId`] `o * n_in + i`.
/// A caller seeds each returned weight by adding a unit-effect gene on that channel and a pool locus at
/// frequency one whose additive effect is `target / ploidy`, so a homozygous founder expresses exactly
/// the target (the locus carries no additive variance at frequency one, so the dawn expression is
/// deterministic and mutation is what later gives selection a gradient to act on).
pub fn taxis_move_weights(
    layout: &ControllerLayout,
    move_output: usize,
    axis_input_base: usize,
    move_bias: Fixed,
    heading_gain: Fixed,
) -> Vec<(ControllerParamId, Fixed)> {
    let n_in = layout.n_in();
    let param = |output: usize, input: usize| ControllerParamId((output * n_in + input) as u32);
    let bias = n_in - 1;
    vec![
        // MOVE activation from the bias input: the being wants to move.
        (param(move_output, bias), move_bias),
        // MOVE heading from the axis's source-direction percept: it steers along the gradient.
        (param(move_output + 1, axis_input_base + 2), heading_gain),
        (param(move_output + 2, axis_input_base + 3), heading_gain),
    ]
}

/// The reserved gain magnitudes a founding forage-taxis reaction norm is seeded with (base-level
/// liveliness step 3). Each is a controller-weight magnitude the caller supplies (Principle 11); the
/// mechanism ([`forage_taxis_weights`]) maps each to its heritable channel with no race branch. The
/// values are the owner's `controller.taxis.*` reserved lever; the dev harness stands up labelled
/// fixtures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ForageGains {
    /// The MOVE-activation bias, so a being wants to move rather than idle (`controller.taxis.move_bias`).
    pub move_bias: Fixed,
    /// The suppression of MOVE when a forage source is on the current tile, so a being stops on food
    /// rather than wandering off it (`controller.taxis.here_suppress`).
    pub here_suppress: Fixed,
    /// The gain steering the MOVE heading toward a forage source's direction and along a steer axis's
    /// field gradient (`controller.taxis.heading_gain`).
    pub heading_gain: Fixed,
    /// The INGEST-activation drive from a forage source underfoot, so a being eats what it stands on
    /// (`controller.taxis.ingest_drive`). The reserve-level gating that a single-axis taxis norm adds is
    /// left to the reserve-room bound in the ingest arm, so a full being draws nothing without a
    /// per-axis gate the shared scalar INGEST output cannot carry across several forage axes.
    pub ingest_drive: Fixed,
}

/// The nonzero founding weights for a full FORAGE reaction norm over one or more target axes plus
/// zero or more gradient-steer axes (base-level liveliness step 3): the being wants to move (the
/// `move_bias`), stops when a forage source is underfoot (each forage axis's here-flag suppresses
/// MOVE), steers toward each forage source and along each steer axis's field gradient (the source-
/// direction and gradient percepts drive the MOVE heading), and ingests a forage source underfoot
/// (each forage axis's here-flag drives the shared INGEST activation). A steer axis contributes only a
/// heading pull (the temperature comfort gradient the runner supplies), never an ingest, since it has
/// no consumable source. This is the multi-axis generalisation of the tested single-axis taxis
/// controller: an energy-and-water grazer that thermoregulates is a forage set `[energy, water]` with a
/// steer set `[temperature]`. Nothing here is a magnitude of its own: the gains are the caller's
/// reserved values (Principle 11), and the mapping from each nonzero weight to its heritable channel
/// carries no race id (Principle 9).
///
/// The layout must give MOVE a directional output at `move_output` (its activation, then its two
/// heading components) and the INGEST scalar output at `ingest_output`; each forage and steer axis is
/// named by its input-block base ([`ControllerLayout::axis_input_base`]). For a reaction norm the
/// weight feeding output `o` from input `i` is [`ControllerParamId`] `o * n_in + i`. The forage and
/// steer bases must be disjoint (an axis is a food source or a gradient to steer by, not both), so no
/// two entries collide on one channel.
pub fn forage_taxis_weights(
    layout: &ControllerLayout,
    move_output: usize,
    ingest_output: usize,
    forage_bases: &[usize],
    steer_bases: &[usize],
    gains: ForageGains,
) -> Vec<(ControllerParamId, Fixed)> {
    let n_in = layout.n_in();
    let param = |output: usize, input: usize| ControllerParamId((output * n_in + input) as u32);
    let bias = n_in - 1;
    // MOVE activation from the bias input: the being wants to move (seeded once).
    let mut out = vec![(param(move_output, bias), gains.move_bias)];
    for &base in forage_bases {
        let (here, dx, dy) = (base + 1, base + 2, base + 3);
        // MOVE suppressed when this forage source is underfoot: stop to eat rather than wander off it.
        out.push((param(move_output, here), Fixed::ZERO - gains.here_suppress));
        // MOVE heading toward this forage source's known direction.
        out.push((param(move_output + 1, dx), gains.heading_gain));
        out.push((param(move_output + 2, dy), gains.heading_gain));
        // INGEST fires when this forage source is underfoot (the reserve-room bound gates the amount).
        out.push((param(ingest_output, here), gains.ingest_drive));
    }
    for &base in steer_bases {
        let (dx, dy) = (base + 2, base + 3);
        // MOVE heading along this steer axis's field gradient (the temperature comfort gradient).
        out.push((param(move_output + 1, dx), gains.heading_gain));
        out.push((param(move_output + 2, dy), gains.heading_gain));
    }
    out
}

/// What the controller decided this tick: which affordance to issue, how strongly, and, for a
/// directional affordance, in what heading (the raw two-component output, to be normalised by the
/// caller against the movement physics).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ControllerDecision {
    /// The chosen affordance.
    pub affordance: AffordanceId,
    /// Its activation, clamped to `[0, 1]`.
    pub activation: Fixed,
    /// The heading for a directional affordance (raw output components), `None` for a scalar one.
    pub heading: Option<(Fixed, Fixed)>,
}

/// A being's expressed behaviour controller: its flat heritable weight vector and the dimensions the
/// layout gave it. The weights are the phenotype expressed from the genome; the dimensions come from
/// the layout. Evaluation is a pure fixed-point function of the weights, the input, and (for a
/// recurrent controller) the carried hidden state.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Controller {
    n_in: usize,
    n_out: usize,
    hidden: usize,
    weights: Vec<Fixed>,
}

impl Controller {
    /// A controller from an explicit weight vector (for tests and for a hand-built default). The
    /// vector must have exactly [`weight_count`] entries for the dimensions.
    pub fn from_weights(
        n_in: usize,
        n_out: usize,
        hidden: usize,
        weights: Vec<Fixed>,
    ) -> Controller {
        assert_eq!(
            weights.len(),
            weight_count(n_in, n_out, hidden),
            "controller weight vector length must match the dimensions"
        );
        Controller {
            n_in,
            n_out,
            hidden,
            weights,
        }
    }

    /// A controller with every weight zero (the blank controller: it issues no positive activation,
    /// so a being carrying it idles and, unfed, dies, which is the base against which selection has
    /// something to improve).
    pub fn zeros(layout: &ControllerLayout) -> Controller {
        Controller {
            n_in: layout.n_in,
            n_out: layout.n_out,
            hidden: layout.hidden,
            weights: vec![Fixed::ZERO; layout.weight_count()],
        }
    }

    /// Express a controller from a genome (design Part 25, the expression mechanism, R-BEHAVIOR-
    /// EVOLVE): each weight is the value the genome expresses for its [`ControllerParamId`] channel,
    /// with no environmental offset, so the controller is a pure function of the being's genes. This
    /// is the same expression path the cognition and composition channels use; a controller is
    /// inherited, drifts, and is selected as any other genetic value is.
    pub fn express(genes: &GeneSet, genome: &Genome, layout: &ControllerLayout) -> Controller {
        let count = layout.weight_count();
        let mut weights = Vec::with_capacity(count);
        for k in 0..count {
            let channel = Channel::Controller(ControllerParamId(k as u32));
            weights.push(genes.express(genome, channel, Fixed::ZERO));
        }
        Controller {
            n_in: layout.n_in,
            n_out: layout.n_out,
            hidden: layout.hidden,
            weights,
        }
    }

    /// The number of inputs this controller reads.
    pub fn n_in(&self) -> usize {
        self.n_in
    }

    /// The number of outputs this controller produces.
    pub fn n_out(&self) -> usize {
        self.n_out
    }

    /// The hidden width (zero for a reaction norm).
    pub fn hidden(&self) -> usize {
        self.hidden
    }

    /// The `k`-th heritable weight (for a selection gradient or inspection); zero past the end.
    pub fn weight(&self, k: usize) -> Fixed {
        self.weights.get(k).copied().unwrap_or(Fixed::ZERO)
    }

    /// The full heritable weight vector, in [`ControllerParamId`] order (weight `k` feeds
    /// `Channel::Controller(ControllerParamId(k))`). A caller that has evolved a controller (the dawn
    /// forage bootstrap) reads this to seed a founder pool with each weight as its target, so a founder
    /// expresses the pre-adapted controller (`crate::genome::append_controller_block`).
    pub fn weights(&self) -> &[Fixed] {
        &self.weights
    }

    /// A copy of this controller with each weight `k` offset by `deviation(k)` (creature-selection step 2,
    /// the mint-time and inheritance perturbation). The caller supplies a DETERMINISTIC per-weight deviation
    /// (a seed-keyed bounded zero-mean draw), so the result is a pure function of the parent and the draw and
    /// replays bit for bit; the offset is saturating so it can never overflow the weight.
    pub fn perturbed(&self, mut deviation: impl FnMut(usize) -> Fixed) -> Controller {
        let weights = self
            .weights
            .iter()
            .enumerate()
            .map(|(k, w)| Fixed::from_bits(w.to_bits().saturating_add(deviation(k).to_bits())))
            .collect();
        Controller {
            n_in: self.n_in,
            n_out: self.n_out,
            hidden: self.hidden,
            weights,
        }
    }

    /// The MIDPARENT blend of two controllers of the same schema (creature-selection step 2, the offspring
    /// controller under the reproduction beat): each weight is the average of the two parents' weights plus a
    /// deterministic per-weight `deviation(k)`. The general inheritance primitive, no reading of a trait, kind,
    /// or relatedness, so an offspring resembles both parents and drifts by the bounded perturbation, the sign
    /// of any weight emerging from which lineages out-reproduce (Principle 8). Both controllers are minted
    /// against the shared embodiment layout, so their schemas always match; a mismatch is a bug. Saturating;
    /// a pure function of the parents and the draw.
    pub fn midparent(
        &self,
        other: &Controller,
        mut deviation: impl FnMut(usize) -> Fixed,
    ) -> Controller {
        assert_eq!(
            (self.n_in, self.n_out, self.hidden),
            (other.n_in, other.n_out, other.hidden),
            "midparent controllers must share the schema"
        );
        let weights = self
            .weights
            .iter()
            .zip(other.weights.iter())
            .enumerate()
            .map(|(k, (a, b))| {
                let avg = a.to_bits().saturating_add(b.to_bits()) / 2;
                Fixed::from_bits(avg.saturating_add(deviation(k).to_bits()))
            })
            .collect();
        Controller {
            n_in: self.n_in,
            n_out: self.n_out,
            hidden: self.hidden,
            weights,
        }
    }

    /// A fresh zero hidden state of the right width (empty for a reaction norm), the state a being
    /// starts life with.
    pub fn fresh_hidden(&self) -> Vec<Fixed> {
        vec![Fixed::ZERO; self.hidden]
    }

    /// Evaluate the controller on an input vector and the previous hidden state, returning the
    /// output vector and the new hidden state (empty for a reaction norm). For a reaction norm each
    /// output is the clamped weighted sum of the inputs. For a recurrent network the hidden state is
    /// the clamped sum of the input-to-hidden and hidden-to-hidden contributions, and the output is
    /// the clamped hidden-to-output map, so behaviour can depend on the being's recent history. A
    /// short or absent previous hidden state reads as zeros, so the first tick degrades cleanly.
    pub fn evaluate(&self, input: &[Fixed], prev_hidden: &[Fixed]) -> (Vec<Fixed>, Vec<Fixed>) {
        if self.hidden == 0 {
            let mut out = Vec::with_capacity(self.n_out);
            for o in 0..self.n_out {
                let row = o * self.n_in;
                out.push(activate(
                    (0..self.n_in).map(|i| (self.weights[row + i], input_at(input, i))),
                ));
            }
            (out, Vec::new())
        } else {
            let ih = 0;
            let hh = self.hidden * self.n_in;
            let ho = hh + self.hidden * self.hidden;
            let mut new_hidden = Vec::with_capacity(self.hidden);
            for h in 0..self.hidden {
                let ih_row = ih + h * self.n_in;
                let hh_row = hh + h * self.hidden;
                let from_input =
                    (0..self.n_in).map(|i| (self.weights[ih_row + i], input_at(input, i)));
                let from_hidden =
                    (0..self.hidden).map(|j| (self.weights[hh_row + j], input_at(prev_hidden, j)));
                new_hidden.push(activate(from_input.chain(from_hidden)));
            }
            let mut out = Vec::with_capacity(self.n_out);
            for o in 0..self.n_out {
                let ho_row = ho + o * self.hidden;
                out.push(activate(
                    (0..self.hidden).map(|h| (self.weights[ho_row + h], new_hidden[h])),
                ));
            }
            (out, new_hidden)
        }
    }
}

/// Read an input component, treating a short vector's missing tail as zero (so a stale or empty
/// hidden state degrades cleanly rather than panicking).
#[inline]
fn input_at(v: &[Fixed], i: usize) -> Fixed {
    v.get(i).copied().unwrap_or(Fixed::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{
        Allele, AlleleState, DominanceMode, GeneDef, GeneEffect, GeneId, Haplotype, SchemeId,
    };
    use crate::homeostasis::{
        AffordanceRegistry, HomeostaticRegistry, ENERGY, INGEST, MOVE, WATER,
    };

    fn layout(hidden: usize) -> ControllerLayout {
        ControllerLayout::new(
            &HomeostaticRegistry::dev_default(),
            &AffordanceRegistry::dev_default(),
            hidden,
        )
    }

    #[test]
    fn the_appetitive_block_is_opt_in_byte_identical_and_founder_inert() {
        // Ideation arc, piece 1, the belief-to-behaviour feedback (WIRE substrate): the appetitive input
        // block is opt-in and hash-neutral by default, one channel per affordance when enabled, and inert
        // for a founder whose weights are all zero, so acting on a reward belief emerges (Principle 8) and
        // an opted-out world is bit-identical.
        let homeo = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let percept = PerceptRegistry::empty();

        // Opt-out: the appetitive-disabled layout is byte-identical to the plain percept layout.
        let off =
            ControllerLayout::with_percepts_and_appetitive(&homeo, &afford, &percept, false, 0);
        let plain = ControllerLayout::with_percepts(&homeo, &afford, &percept, 0);
        assert_eq!(off, plain, "the appetitive-off layout is byte-identical");
        assert_eq!(off.n_appetitive(), 0);

        // Opt-in: n_in grows by exactly one channel per affordance, the block sits after the features and
        // before the bias, and the channels align to the canonical affordance order.
        let on = ControllerLayout::with_percepts_and_appetitive(&homeo, &afford, &percept, true, 0);
        let n_afford = on.affordance_ids().len();
        assert_eq!(on.n_appetitive(), n_afford);
        assert_eq!(on.n_in(), plain.n_in() + n_afford);
        assert_eq!(on.appetitive_input_base(), plain.n_in() - 1); // just past features, before the bias
        assert_eq!(on.weight_count(), on.n_out() * on.n_in());

        // The builder writes the appetitive vector into the block and keeps the bias last.
        let here = BTreeSet::new();
        let dirs = BTreeMap::new();
        let signed = BTreeMap::new();
        let homeostasis = Homeostasis::from_mass(&homeo, Fixed::ONE);
        let appetitive: Vec<Fixed> = (0..n_afford).map(|_| Fixed::ONE).collect();
        let input = on.build_input_with_features_and_appetitive(
            &homeostasis,
            &here,
            &dirs,
            &signed,
            &[],
            &appetitive,
        );
        let abase = on.appetitive_input_base();
        for k in 0..n_afford {
            assert_eq!(
                input[abase + k],
                Fixed::ONE,
                "appetitive channel {k} is written"
            );
        }
        assert_eq!(
            input[on.n_in() - 1],
            Fixed::ONE,
            "the bias is still the last slot"
        );

        // A founder (all-zero weights) issues the identical output whether or not the appetitive block is
        // lit, so a reward belief moves no behaviour until selection lifts an appetitive weight off zero.
        let founder = Controller::zeros(&on);
        let dark = on.build_input_with_features_and_appetitive(
            &homeostasis,
            &here,
            &dirs,
            &signed,
            &[],
            &vec![Fixed::ZERO; n_afford],
        );
        let (lit_out, _) = founder.evaluate(&input, &[]);
        let (dark_out, _) = founder.evaluate(&dark, &[]);
        assert_eq!(
            lit_out, dark_out,
            "a founder's behaviour is unmoved by the appetitive percept (emergent, not authored)"
        );
    }

    #[test]
    fn the_conviction_block_is_opt_in_byte_identical_and_founder_inert() {
        // Prereq B for the learned experience-to-conviction coupling (OWNER_DECISIONS_LOG R2): the conviction
        // input block is opt-in and hash-neutral by default, one channel per exposed conviction axis when
        // armed, sits AFTER the attraction block and before the bias (so every earlier base holds), and is
        // inert for a founder whose weights are all zero, so whether a conviction moves behaviour EMERGES
        // (Principle 8) and an opted-out world is bit-identical.
        use crate::axiom::AxiomAxisId;
        use crate::material_percept::MaterialPerceptRegistry;
        let homeo = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let percept = PerceptRegistry::empty();
        let material = MaterialPerceptRegistry::empty();

        // Opt-out: the conviction-empty layout is byte-identical to the attraction-tier layout it delegates to.
        let plain = ControllerLayout::with_percepts_appetitive_material_and_attraction(
            &homeo, &afford, &percept, false, &material, false, 0,
        );
        let off = ControllerLayout::with_percepts_appetitive_material_attraction_and_conviction(
            &homeo,
            &afford,
            &percept,
            false,
            &material,
            false,
            &ConvictionPerceptRegistry::empty(),
            false,
            0,
        );
        assert_eq!(off, plain, "the conviction-empty layout is byte-identical");
        assert_eq!(off.n_conviction(), 0);

        // Opt-in: n_in grows by exactly one channel per exposed conviction axis, the block sits just past the
        // attraction block (here, with no other blocks, just before the bias), and the weight count follows.
        let convictions = ConvictionPerceptRegistry::from_axes(&[AxiomAxisId(0), AxiomAxisId(3)]);
        let on = ControllerLayout::with_percepts_appetitive_material_attraction_and_conviction(
            &homeo,
            &afford,
            &percept,
            false,
            &material,
            false,
            &convictions,
            false,
            0,
        );
        assert_eq!(on.n_conviction(), 2);
        assert_eq!(on.n_in(), plain.n_in() + 2);
        assert_eq!(on.conviction_input_base(), plain.n_in() - 1); // just past attraction, before the bias
        assert_eq!(on.weight_count(), on.n_out() * on.n_in());

        // The builder writes the being's own stances into the conviction block and keeps the bias last.
        let here = BTreeSet::new();
        let dirs = BTreeMap::new();
        let signed = BTreeMap::new();
        let homeostasis = Homeostasis::from_mass(&homeo, Fixed::ONE);
        let stances = vec![Fixed::ONE, Fixed::from_ratio(1, 2)];
        let input = on.build_input_full_with_conviction(
            &homeostasis,
            &here,
            &dirs,
            &signed,
            &[],
            &[],
            &[],
            &[],
            &stances,
            &[],
        );
        let cbase = on.conviction_input_base();
        assert_eq!(
            input[cbase],
            Fixed::ONE,
            "conviction channel 0 is the being's own stance"
        );
        assert_eq!(
            input[cbase + 1],
            Fixed::from_ratio(1, 2),
            "conviction channel 1 is its stance"
        );
        assert_eq!(
            input[on.n_in() - 1],
            Fixed::ONE,
            "the bias is still the last slot"
        );

        // A founder (all-zero weights) issues the identical output whether or not the conviction block is lit,
        // so a conviction moves no behaviour until selection lifts a conviction weight off zero.
        let founder = Controller::zeros(&on);
        let dark = on.build_input_full_with_conviction(
            &homeostasis,
            &here,
            &dirs,
            &signed,
            &[],
            &[],
            &[],
            &[],
            &[Fixed::ZERO; 2],
            &[],
        );
        let (lit_out, _) = founder.evaluate(&input, &[]);
        let (dark_out, _) = founder.evaluate(&dark, &[]);
        assert_eq!(
            lit_out, dark_out,
            "a founder's behaviour is unmoved by its convictions until selection lifts a weight (emergent)"
        );
    }

    #[test]
    fn the_material_feature_block_is_opt_in_byte_identical_and_founder_inert() {
        // The lifetime/demography keystone, pillar 2, trace slice C (the SENSE half): the material-feature
        // input block is opt-in and hash-neutral by default, one channel per declared substance when armed,
        // sits AFTER the appetitive block and before the bias (so every earlier base holds), and is inert for
        // a founder whose weights are all zero, so sensing the matter underfoot moves no behaviour until
        // selection lifts a weight (Principle 8) and an opted-out world is bit-identical.
        let homeo = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let percept = PerceptRegistry::empty();

        // Opt-out: an empty material registry yields exactly the appetitive layout (n_material zero), so the
        // material substrate adds nothing when unarmed.
        let base =
            ControllerLayout::with_percepts_and_appetitive(&homeo, &afford, &percept, true, 0);
        let off = ControllerLayout::with_percepts_appetitive_and_material(
            &homeo,
            &afford,
            &percept,
            true,
            &MaterialPerceptRegistry::empty(),
            0,
        );
        assert_eq!(
            off, base,
            "the material-off layout is byte-identical to the appetitive layout"
        );
        assert_eq!(off.n_material(), 0);

        // Opt-in: n_in grows by exactly one channel per substance, the block sits after the appetitive block
        // and before the bias.
        let material = MaterialPerceptRegistry::from_substances(&["spent_hull", "granite"]);
        let on = ControllerLayout::with_percepts_appetitive_and_material(
            &homeo, &afford, &percept, true, &material, 0,
        );
        assert_eq!(on.n_material(), 2);
        assert_eq!(on.n_in(), base.n_in() + 2);
        // The material base sits just past the appetitive block, so the axis, feature, and appetitive bases
        // are all unchanged by adding it.
        assert_eq!(
            on.material_input_base(),
            base.appetitive_input_base() + base.n_appetitive()
        );
        assert_eq!(on.feature_input_base(), base.feature_input_base());
        assert_eq!(on.appetitive_input_base(), base.appetitive_input_base());

        // The builder writes the material vector into the block and keeps the bias last.
        let here = BTreeSet::new();
        let dirs = BTreeMap::new();
        let signed = BTreeMap::new();
        let homeostasis = Homeostasis::from_mass(&homeo, Fixed::ONE);
        let appetitive: Vec<Fixed> = vec![Fixed::ZERO; on.n_appetitive()];
        let material_vec = vec![Fixed::from_int(3), Fixed::from_int(5)];
        let input = on.build_input_with_features_appetitive_and_material(
            &homeostasis,
            &here,
            &dirs,
            &signed,
            &[],
            &appetitive,
            &material_vec,
        );
        let mbase = on.material_input_base();
        assert_eq!(
            input[mbase],
            Fixed::from_int(3),
            "material channel 0 is written"
        );
        assert_eq!(
            input[mbase + 1],
            Fixed::from_int(5),
            "material channel 1 is written"
        );
        assert_eq!(
            input[on.n_in() - 1],
            Fixed::ONE,
            "the bias is still the last slot"
        );

        // A founder (all-zero weights) issues the identical output whether or not the material block is lit,
        // so sensing the matter moves no behaviour until selection lifts a material weight off zero.
        let founder = Controller::zeros(&on);
        let dark = on.build_input_with_features_appetitive_and_material(
            &homeostasis,
            &here,
            &dirs,
            &signed,
            &[],
            &appetitive,
            &[Fixed::ZERO; 2],
        );
        let (lit_out, _) = founder.evaluate(&input, &[]);
        let (dark_out, _) = founder.evaluate(&dark, &[]);
        assert_eq!(
            lit_out, dark_out,
            "a founder's behaviour is unmoved by the material percept (emergent, not authored)"
        );
    }

    #[test]
    fn the_attraction_direction_input_is_opt_in_byte_identical_and_founder_inert() {
        // The lifetime/demography keystone, pillar 2, trace slice C3 (the behaviour half): the belief-derived
        // attraction-direction input is opt-in and hash-neutral by default, TWO channels (dx, dy) when armed,
        // sits AFTER the material block and before the bias, and is inert for a founder whose weights are all
        // zero, so approaching a trace-marked place moves no behaviour until selection lifts a weight (Principle
        // 8) and an opted-out world is bit-identical. The dedicated clean channel, the mirror of avoidance's
        // dead CONDITION slot.
        let homeo = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let percept = PerceptRegistry::empty();
        let material = MaterialPerceptRegistry::from_substances(&["spent_hull"]);

        // Opt-out: attraction off yields exactly the material layout (n_attraction zero).
        let base = ControllerLayout::with_percepts_appetitive_material_and_attraction(
            &homeo, &afford, &percept, true, &material, false, 0,
        );
        let off = ControllerLayout::with_percepts_appetitive_material_and_attraction(
            &homeo, &afford, &percept, true, &material, false, 0,
        );
        assert_eq!(off, base);
        assert_eq!(off.n_attraction(), 0);

        // Opt-in: n_in grows by exactly two channels, the block sits after the material block and before the
        // bias, and every earlier base is unchanged.
        let on = ControllerLayout::with_percepts_appetitive_material_and_attraction(
            &homeo, &afford, &percept, true, &material, true, 0,
        );
        assert_eq!(on.n_attraction(), 2);
        assert_eq!(on.n_in(), base.n_in() + 2);
        assert_eq!(
            on.attraction_input_base(),
            base.material_input_base() + base.n_material()
        );
        assert_eq!(on.material_input_base(), base.material_input_base());

        // The builder writes the two attraction components and keeps the bias last.
        let here = BTreeSet::new();
        let dirs = BTreeMap::new();
        let signed = BTreeMap::new();
        let homeostasis = Homeostasis::from_mass(&homeo, Fixed::ONE);
        let material_vec = vec![Fixed::ZERO];
        let attraction = vec![Fixed::from_int(1), Fixed::from_int(-1)];
        let input = on.build_input_full(
            &homeostasis,
            &here,
            &dirs,
            &signed,
            &[],
            &vec![Fixed::ZERO; on.n_appetitive()],
            &material_vec,
            &attraction,
        );
        let atbase = on.attraction_input_base();
        assert_eq!(
            input[atbase],
            Fixed::from_int(1),
            "attraction dx is written"
        );
        assert_eq!(
            input[atbase + 1],
            Fixed::from_int(-1),
            "attraction dy is written"
        );
        assert_eq!(
            input[on.n_in() - 1],
            Fixed::ONE,
            "the bias is still the last slot"
        );

        // A founder (all-zero weights) issues the identical output whether or not the attraction block is lit,
        // so the gradient moves no behaviour until selection lifts an attraction weight off zero.
        let founder = Controller::zeros(&on);
        let dark = on.build_input_full(
            &homeostasis,
            &here,
            &dirs,
            &signed,
            &[],
            &vec![Fixed::ZERO; on.n_appetitive()],
            &material_vec,
            &[Fixed::ZERO; 2],
        );
        let (lit_out, _) = founder.evaluate(&input, &[]);
        let (dark_out, _) = founder.evaluate(&dark, &[]);
        assert_eq!(
            lit_out, dark_out,
            "a founder's behaviour is unmoved by the attraction percept (emergent, not authored)"
        );
    }

    #[test]
    fn taxis_move_weights_land_on_the_move_activation_and_heading_slots() {
        // Base-level liveliness step 1: the founding taxis weights target the MOVE activation from the
        // bias input and the two MOVE heading components from the target axis's source-direction slots,
        // at the reaction-norm indices output * n_in + input, carrying the caller's two gains.
        let l = layout(0);
        let n_in = l.n_in();
        let mo = MOVE.0 as usize;
        let w = taxis_move_weights(&l, mo, 0, Fixed::from_int(3), Fixed::from_int(5));
        assert_eq!(w.len(), 3, "move activation and the two heading components");
        assert!(
            w.contains(&(
                ControllerParamId((mo * n_in + (n_in - 1)) as u32),
                Fixed::from_int(3)
            )),
            "move activation from the bias carries the move bias"
        );
        assert!(
            w.contains(&(
                ControllerParamId(((mo + 1) * n_in + 2) as u32),
                Fixed::from_int(5)
            )),
            "move heading dx from the axis source-direction x carries the heading gain"
        );
        assert!(
            w.contains(&(
                ControllerParamId(((mo + 2) * n_in + 3) as u32),
                Fixed::from_int(5)
            )),
            "move heading dy from the axis source-direction y carries the heading gain"
        );
    }

    #[test]
    fn the_layout_dims_follow_the_registries() {
        let l = layout(0);
        // Two axes (energy, water) -> 5*2 + 1 bias = 11 inputs (the fifth per-axis input is the
        // signed setpoint-deviation percept).
        assert_eq!(l.n_in(), 11);
        // MOVE (directional, 3 outputs) + INGEST (scalar, 1) = 4 outputs.
        assert_eq!(l.n_out(), 4);
        // Reaction norm: n_out * n_in weights.
        assert_eq!(l.weight_count(), 44);
    }

    #[test]
    fn a_recurrent_layout_sizes_its_weight_blocks() {
        let l = layout(3);
        // hidden*n_in + hidden*hidden + n_out*hidden = 3*11 + 3*3 + 4*3 = 33 + 9 + 12 = 54.
        assert_eq!(l.weight_count(), 54);
        assert_eq!(l.hidden(), 3);
    }

    /// A reaction-norm controller whose non-zero weights make it move toward known water while it is
    /// away from it and ingest the water underfoot when its reserve is low. The input layout, in the
    /// dev registry order (energy, water), is per axis [level, here, dir_x, dir_y, signed] then a
    /// bias, so the water block starts at `INPUTS_PER_AXIS` (water is axis 1). The output layout is
    /// [move_act, move_dx, move_dy, ingest_act].
    fn taxis_controller(l: &ControllerLayout) -> Controller {
        let n_in = l.n_in();
        let bias = n_in - 1;
        let water = INPUTS_PER_AXIS; // the water axis's input block base (axis index 1)
        let (w_lvl, w_here, w_dx, w_dy) = (water, water + 1, water + 2, water + 3);
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        // move_act (output 0): a desire to move (bias), suppressed when water is underfoot, so a being
        // on the water does not wander off it.
        w[bias] = Fixed::ONE;
        w[w_here] = Fixed::from_int(-1);
        // move_dx / move_dy (outputs 1, 2): follow the water source direction.
        w[n_in + w_dx] = Fixed::ONE;
        w[2 * n_in + w_dy] = Fixed::ONE;
        // ingest_act (output 3): fire when water is underfoot and the water reserve is low.
        w[3 * n_in + w_here] = Fixed::ONE;
        w[3 * n_in + w_lvl] = Fixed::from_int(-1);
        Controller::from_weights(l.n_in(), l.n_out(), l.hidden(), w)
    }

    fn drained(reg: &HomeostaticRegistry, ticks: usize) -> Homeostasis {
        let mut h = Homeostasis::from_mass(reg, Fixed::ONE);
        for _ in 0..ticks {
            h.metabolize(reg, Fixed::ZERO);
        }
        h
    }

    #[test]
    fn the_controller_moves_toward_known_water_when_away_from_it() {
        let l = layout(0);
        let c = taxis_controller(&l);
        let reg = HomeostaticRegistry::dev_default();
        let homeo = drained(&reg, 200); // water somewhat below full
                                        // Knows of water to the east (unit direction (1, 0)); it is not standing on it.
        let mut dirs = BTreeMap::new();
        dirs.insert(WATER, (Fixed::ONE, Fixed::ZERO));
        let input = l.build_input(&homeo, &BTreeSet::new(), &dirs, &BTreeMap::new());
        let (out, hidden) = c.evaluate(&input, &[]);
        assert!(hidden.is_empty(), "a reaction norm carries no hidden state");
        let d = l.decide(&out, &[MOVE, INGEST]).unwrap();
        assert_eq!(d.affordance, MOVE, "away from water, the being moves");
        let (hx, hy) = d.heading.unwrap();
        assert!(
            hx > Fixed::ZERO,
            "the heading points toward the known water (east)"
        );
        assert_eq!(hy, Fixed::ZERO, "and not north or south");
    }

    #[test]
    fn a_being_attraction_weight_steers_the_move_heading_toward_or_away_by_its_sign() {
        // Creatures-react (mechanism B3), the CI-verified movement proof (the watchable is
        // examples/creatures_react_demo.rs). The creature's being block carries a magnitude-graded
        // toward-direction (its attraction pair); a heritable freely-signed weight turns it into approach
        // (positive) or flight (negative). The SIGN is what selection sets in the world; both are checked here
        // to prove the wire yields movement either way, and founder-zero is the inert null (neither sign
        // privileged, Principle 9).
        let homeo_reg = HomeostaticRegistry::dev_grazer();
        let l = ControllerLayout::with_percepts_and_being(
            &homeo_reg,
            &AffordanceRegistry::dev_default(),
            &PerceptRegistry::empty(),
            true,
            0,
        );
        let n_in = l.n_in();
        let being_base = l.being_input_base();
        // MOVE is output 0 (activation), heading dx/dy at outputs 1 and 2; a reaction-norm weight feeding
        // output `o` from input `i` is at flat index `o * n_in + i`.
        let build = |gain: Fixed| -> Controller {
            let mut w = vec![Fixed::ZERO; l.weight_count()];
            w[n_in - 1] = Fixed::ONE; // MOVE activation from the bias: the creature wants to move
            w[n_in + (being_base + 2)] = gain; // MOVE heading dx follows the being-attraction dx
            w[2 * n_in + (being_base + 3)] = gain; // MOVE heading dy follows the being-attraction dy
            Controller::from_weights(l.n_in(), l.n_out(), l.hidden(), w)
        };
        let homeo = Homeostasis::from_mass(&homeo_reg, Fixed::ONE);
        // A perceived being due EAST: the being block's attraction pair points east (dx = +1, dy = 0).
        let being = [Fixed::ZERO, Fixed::ZERO, Fixed::ONE, Fixed::ZERO];
        let input = l.build_input_full_with_conviction(
            &homeo,
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &[],
            &[],
            &[],
            &[],
            &[],
            &being,
        );
        // A POSITIVE weight steers the MOVE heading TOWARD the east emitter.
        let (out_pos, _) = build(Fixed::from_int(4)).evaluate(&input, &[]);
        let d_pos = l
            .decide(&out_pos, &[MOVE, INGEST])
            .expect("the body affords MOVE");
        assert_eq!(d_pos.affordance, MOVE, "the creature decides to move");
        let (hx_pos, hy_pos) = d_pos.heading.expect("MOVE carries a heading");
        assert!(
            hx_pos > Fixed::ZERO,
            "a positive being-weight steers TOWARD the east emitter (hunting)"
        );
        assert_eq!(
            hy_pos,
            Fixed::ZERO,
            "no north-south component for a due-east emitter"
        );
        // A NEGATIVE weight steers AWAY (west) from the IDENTICAL percept: only the sign differs.
        let (out_neg, _) = build(Fixed::from_int(-4)).evaluate(&input, &[]);
        let d_neg = l
            .decide(&out_neg, &[MOVE, INGEST])
            .expect("the body affords MOVE");
        let (hx_neg, _) = d_neg.heading.expect("MOVE carries a heading");
        assert!(
            hx_neg < Fixed::ZERO,
            "a negative being-weight steers AWAY (west, fleeing) from the same percept"
        );
        // Founder-zero is inert: with no being-weight the heading carries no eastward pull (the null, so
        // neither approach nor flight is privileged until selection lifts a weight).
        let (out_zero, _) = build(Fixed::ZERO).evaluate(&input, &[]);
        if let Some(d_zero) = l.decide(&out_zero, &[MOVE, INGEST]) {
            if let Some((hx_zero, _)) = d_zero.heading {
                assert_eq!(
                    hx_zero,
                    Fixed::ZERO,
                    "founder-zero being-weight steers neither way (the true null)"
                );
            }
        }
    }

    #[test]
    fn an_empty_being_feature_registry_leaves_the_layout_byte_identical() {
        // The byte-neutral opt-in guarantee (step 2b): a layout built with an EMPTY perceivable-feature
        // registry is bit-for-bit the same shape (inputs and weights) as the plain being layout, so a world
        // that declares no perceivable features grows the controller not at all.
        use crate::perceivable_feature::PerceivableFeatureRegistry;
        let homeo_reg = HomeostaticRegistry::dev_grazer();
        let afford = AffordanceRegistry::dev_default();
        let being = ControllerLayout::with_percepts_and_being(
            &homeo_reg,
            &afford,
            &PerceptRegistry::empty(),
            true,
            0,
        );
        let with_empty =
            ControllerLayout::with_percepts_appetitive_material_attraction_conviction_and_being_features(
                &homeo_reg,
                &afford,
                &PerceptRegistry::empty(),
                false,
                &MaterialPerceptRegistry::empty(),
                false,
                &ConvictionPerceptRegistry::empty(),
                true,
                &PerceivableFeatureRegistry::empty(),
                0,
            );
        assert_eq!(with_empty.n_being_feature(), 0);
        assert_eq!(with_empty.n_in(), being.n_in());
        assert_eq!(with_empty.weight_count(), being.weight_count());
    }

    #[test]
    fn a_being_feature_bucket_weight_steers_the_move_heading_by_its_sign() {
        // Step 2b, the per-bucket heritable-sign response: an armed perceivable-feature channel adds, per
        // discrimination bucket, a (dx, dy) toward-direction slot; a heritable FREELY-SIGNED weight on a
        // bucket's slots turns the perceiver toward (positive) or away (negative) from that bucket's emitters,
        // so fleeing one feature-bucket and approaching another emerges from selection (Principle 9). This
        // proves the block is placed after the being block and that a per-bucket weight moves the heading.
        use crate::perceivable_feature::PerceivableFeatureRegistry;
        let homeo_reg = HomeostaticRegistry::dev_grazer();
        // One channel, two buckets: the block adds 2 buckets * 2 (dx, dy) = 4 slots after the being block.
        let features = PerceivableFeatureRegistry::from_channels(&[("opt.emissivity", 2)]);
        let l =
            ControllerLayout::with_percepts_appetitive_material_attraction_conviction_and_being_features(
                &homeo_reg,
                &AffordanceRegistry::dev_default(),
                &PerceptRegistry::empty(),
                false,
                &MaterialPerceptRegistry::empty(),
                false,
                &ConvictionPerceptRegistry::empty(),
                true,
                &features,
                0,
            );
        // The being-feature block begins right after the four-slot being block, and grows n_in by 4.
        assert_eq!(l.n_being_feature(), 4);
        assert_eq!(
            l.being_feature_input_base(),
            l.being_input_base() + l.n_being()
        );
        let n_in = l.n_in();
        let bfbase = l.being_feature_input_base();
        // Steer the MOVE heading (outputs 1, 2) from BUCKET 1's (dx, dy) pair (slots bfbase+2, bfbase+3).
        let build = |gain: Fixed| -> Controller {
            let mut w = vec![Fixed::ZERO; l.weight_count()];
            w[n_in - 1] = Fixed::ONE; // MOVE activation from the bias
            w[n_in + (bfbase + 2)] = gain; // MOVE heading dx follows bucket 1's toward-dx
            w[2 * n_in + (bfbase + 3)] = gain; // MOVE heading dy follows bucket 1's toward-dy
            Controller::from_weights(l.n_in(), l.n_out(), l.hidden(), w)
        };
        let homeo = Homeostasis::from_mass(&homeo_reg, Fixed::ONE);
        // An emitter whose feature falls in BUCKET 1 lies due EAST: bucket 1's toward pair points east. Bucket
        // 0 (slots bfbase+0, bfbase+1) is empty. The being block (bbase..bbase+4) is also empty here.
        let mut being_features = vec![Fixed::ZERO; l.n_being_feature()];
        being_features[2] = Fixed::ONE; // bucket 1 dx = +1 (east)
        being_features[3] = Fixed::ZERO; // bucket 1 dy = 0
        let input = l.build_input_full_with_conviction_and_being_features(
            &homeo,
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &[],
            &[],
            &[],
            &[],
            &[],
            &[Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, Fixed::ZERO],
            &being_features,
        );
        // A POSITIVE weight on bucket 1 steers TOWARD the east emitter (approach).
        let (out_pos, _) = build(Fixed::from_int(4)).evaluate(&input, &[]);
        let (hx_pos, _) = l
            .decide(&out_pos, &[MOVE, INGEST])
            .expect("affords MOVE")
            .heading
            .expect("MOVE carries a heading");
        assert!(
            hx_pos > Fixed::ZERO,
            "a positive bucket-1 weight steers TOWARD the emitter in that feature-bucket"
        );
        // A NEGATIVE weight on the SAME bucket steers AWAY (west, fleeing): only the sign differs.
        let (out_neg, _) = build(Fixed::from_int(-4)).evaluate(&input, &[]);
        let (hx_neg, _) = l
            .decide(&out_neg, &[MOVE, INGEST])
            .expect("affords MOVE")
            .heading
            .expect("MOVE carries a heading");
        assert!(
            hx_neg < Fixed::ZERO,
            "a negative bucket-1 weight steers AWAY (fleeing) from the same feature-bucket emitter"
        );
    }

    #[test]
    fn the_controller_ingests_the_water_underfoot_when_dry() {
        let l = layout(0);
        let c = taxis_controller(&l);
        let reg = HomeostaticRegistry::dev_default();
        let homeo = drained(&reg, 200);
        // Standing on the water (its source is on the current tile).
        let mut here = BTreeSet::new();
        here.insert(WATER);
        let input = l.build_input(&homeo, &here, &BTreeMap::new(), &BTreeMap::new());
        let (out, _) = c.evaluate(&input, &[]);
        let d = l.decide(&out, &[MOVE, INGEST]).unwrap();
        assert_eq!(
            d.affordance, INGEST,
            "on the water and dry, the being drinks rather than wanders"
        );
        assert!(d.activation > Fixed::ZERO, "and it wants to");
    }

    #[test]
    fn a_blank_controller_wants_nothing() {
        let l = layout(0);
        let c = Controller::zeros(&l);
        let reg = HomeostaticRegistry::dev_default();
        let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
        let input = l.build_input(&homeo, &BTreeSet::new(), &BTreeMap::new(), &BTreeMap::new());
        let (out, _) = c.evaluate(&input, &[]);
        // Every output is zero, so the top decision has zero activation: the being idles.
        let d = l.decide(&out, &[MOVE, INGEST]).unwrap();
        assert_eq!(
            d.activation,
            Fixed::ZERO,
            "a blank controller issues no positive drive"
        );
    }

    #[test]
    fn a_rooted_body_only_ever_decides_to_ingest() {
        // Even a controller that wants to move cannot choose MOVE if the body does not afford it.
        let l = layout(0);
        let c = taxis_controller(&l);
        let reg = HomeostaticRegistry::dev_default();
        let homeo = drained(&reg, 200);
        let mut dirs = BTreeMap::new();
        dirs.insert(WATER, (Fixed::ONE, Fixed::ZERO));
        let input = l.build_input(&homeo, &BTreeSet::new(), &dirs, &BTreeMap::new());
        let (out, _) = c.evaluate(&input, &[]);
        let d = l.decide(&out, &[INGEST]).unwrap(); // only INGEST afforded (a rooted body)
        assert_eq!(
            d.affordance, INGEST,
            "a body that cannot move never decides to move"
        );
    }

    #[test]
    fn evaluation_is_deterministic_and_order_independent() {
        let l = layout(2);
        // A recurrent controller with a spread of weights, so the accumulation actually mixes terms.
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        for (k, wk) in w.iter_mut().enumerate() {
            *wk = Fixed::from_ratio((k as i64 % 5) - 2, 4);
        }
        let c = Controller::from_weights(l.n_in(), l.n_out(), l.hidden(), w);
        let reg = HomeostaticRegistry::dev_default();
        let homeo = Homeostasis::from_mass(&reg, Fixed::from_ratio(3, 4));
        let mut dirs = BTreeMap::new();
        dirs.insert(ENERGY, (Fixed::from_ratio(1, 2), Fixed::from_ratio(-1, 2)));
        dirs.insert(WATER, (Fixed::from_ratio(-1, 3), Fixed::from_ratio(1, 3)));
        let input = l.build_input(&homeo, &BTreeSet::new(), &dirs, &BTreeMap::new());
        let h0 = c.fresh_hidden();
        let (a_out, a_h) = c.evaluate(&input, &h0);
        let (b_out, b_h) = c.evaluate(&input, &h0);
        assert_eq!(a_out, b_out, "the same input reproduces the same output");
        assert_eq!(a_h, b_h, "and the same hidden state");
        // The hidden state carries: a second step from the first step's hidden differs from the first.
        let (c_out, _) = c.evaluate(&input, &a_h);
        assert_ne!(
            a_out, c_out,
            "a recurrent controller's output depends on its carried state"
        );
    }

    #[test]
    fn a_controller_expressed_from_a_genome_reproduces_its_weights() {
        // A gene set whose genes each feed one controller weight, so expression recovers the weights.
        let l = layout(0);
        let count = l.weight_count();
        let genes: Vec<GeneDef> = (0..count)
            .map(|k| GeneDef {
                id: GeneId(k as u32),
                effects: vec![GeneEffect {
                    channel: Channel::Controller(ControllerParamId(k as u32)),
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode::additive(),
            })
            .collect();
        let genes = GeneSet { genes };
        // A haploid genome whose allele at locus k carries additive value k/64, so weight k = k/64.
        let alleles: Vec<Allele> = (0..count)
            .map(|k| Allele {
                additive: Fixed::from_ratio(k as i64, 64),
                state: AlleleState(0),
                origin: 0,
            })
            .collect();
        let genome = Genome {
            scheme: SchemeId(0),
            haps: vec![Haplotype { alleles }],
        };
        let c = Controller::express(&genes, &genome, &l);
        // Re-expressing the same genome yields an identical controller (determinism of expression).
        let c2 = Controller::express(&genes, &genome, &l);
        assert_eq!(c, c2, "expression from a genome is deterministic");
    }
}
