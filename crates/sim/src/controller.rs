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
//! the reserve level, a flag for whether a source of that axis is on the current tile (matter within
//! reach), and the unit direction to the nearest known source, plus a constant bias. The outputs are,
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

/// The number of controller inputs each homeostatic axis contributes: its reserve level, a flag for
/// whether a source of it is on the current tile (matter within reach, so the being can tell food
/// underfoot from food in the distance), and the two components of the unit direction to its nearest
/// known source.
const INPUTS_PER_AXIS: usize = 4;

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

    /// The number of heritable weights this layout's controller carries, which is the number of
    /// [`ControllerParamId`]s a genome must reach to express a full controller. For a reaction norm
    /// this is `n_out * n_in`; for a recurrent network it adds the input-to-hidden, hidden-to-hidden,
    /// and hidden-to-output blocks.
    pub fn weight_count(&self) -> usize {
        weight_count(self.n_in, self.n_out, self.hidden)
    }

    /// Build the input vector for a being: per axis (canonical order) its reserve level, a flag for
    /// whether a source of that axis is on the current tile (`here`), and the unit direction to its
    /// nearest known source (zero if none is known), then the bias. A pure read of the being's
    /// physiology and its earned knowledge (Principle 10).
    pub fn build_input(
        &self,
        homeo: &Homeostasis,
        here: &BTreeSet<HomeostaticAxisId>,
        source_dirs: &BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>,
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
    fn the_layout_dims_follow_the_registries() {
        let l = layout(0);
        // Two axes (energy, water) -> 4*2 + 1 bias = 9 inputs.
        assert_eq!(l.n_in(), 9);
        // MOVE (directional, 3 outputs) + INGEST (scalar, 1) = 4 outputs.
        assert_eq!(l.n_out(), 4);
        // Reaction norm: n_out * n_in weights.
        assert_eq!(l.weight_count(), 36);
    }

    #[test]
    fn a_recurrent_layout_sizes_its_weight_blocks() {
        let l = layout(3);
        // hidden*n_in + hidden*hidden + n_out*hidden = 3*9 + 3*3 + 4*3 = 27 + 9 + 12 = 48.
        assert_eq!(l.weight_count(), 48);
        assert_eq!(l.hidden(), 3);
    }

    /// A reaction-norm controller whose non-zero weights make it move toward known water while it is
    /// away from it and ingest the water underfoot when its reserve is low. The input layout, in the
    /// dev registry order (energy, water), is per axis [level, here, dir_x, dir_y] then a bias:
    /// indices 0..3 energy, 4..7 water, 8 bias. The output layout is [move_act, move_dx, move_dy,
    /// ingest_act].
    fn taxis_controller(l: &ControllerLayout) -> Controller {
        let n_in = l.n_in();
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        // move_act (output 0): a desire to move (bias, index 8), suppressed when water is underfoot
        // (water-here, index 5), so a being on the water does not wander off it.
        w[8] = Fixed::ONE;
        w[5] = Fixed::from_int(-1);
        // move_dx / move_dy (outputs 1, 2): follow the water source direction (indices 6, 7).
        w[n_in + 6] = Fixed::ONE;
        w[2 * n_in + 7] = Fixed::ONE;
        // ingest_act (output 3): fire when water is underfoot (index 5) and the water reserve
        // (index 4) is low.
        w[3 * n_in + 5] = Fixed::ONE;
        w[3 * n_in + 4] = Fixed::from_int(-1);
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
        let input = l.build_input(&homeo, &BTreeSet::new(), &dirs);
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
        let input = l.build_input(&homeo, &here, &BTreeMap::new());
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
        let input = l.build_input(&homeo, &BTreeSet::new(), &BTreeMap::new());
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
        let input = l.build_input(&homeo, &BTreeSet::new(), &dirs);
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
        let input = l.build_input(&homeo, &BTreeSet::new(), &dirs);
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
