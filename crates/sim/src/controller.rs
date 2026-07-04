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

use crate::genome::{Channel, ControllerParamId, GeneSet, Genome};
use crate::homeostasis::{
    AffordanceId, AffordanceParam, AffordanceRegistry, Homeostasis, HomeostaticAxisId,
    HomeostaticRegistry,
};

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
    /// The number of inputs (`INPUTS_PER_AXIS * axes + 1` for the bias).
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
        let axes: Vec<HomeostaticAxisId> = homeo.axes.iter().map(|a| a.id).collect();
        let n_in = INPUTS_PER_AXIS * axes.len() + 1;
        let mut defs: Vec<&crate::homeostasis::AffordanceDef> = afford.affordances.iter().collect();
        defs.sort_by_key(|d| d.id);
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
        v[INPUTS_PER_AXIS * self.axes.len()] = Fixed::ONE; // the bias input
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
