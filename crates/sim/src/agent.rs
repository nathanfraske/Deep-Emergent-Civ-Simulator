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

//! A minimal mind: the epistemic core of an agent (design Parts 9 and 37).
//!
//! This composes the two resolved cognition mechanisms into one per-entity holder: the
//! evidence engine ([`crate::evidence`]) for what the mind believes about the world, and
//! the recursive theory-of-mind update ([`crate::tom`]) for what it believes other
//! minds believe. A [`Mind`] can perceive and be told things, form and revise beliefs,
//! model another mind from access evidence, be deceived, and see a lie through, all
//! deterministically.
//!
//! Its cognition is a phenotype rather than an authored number. [`Mind::from_genome`] expresses
//! the reasoning acuity, memory capacity, and belief plasticity from a being's genes through
//! the [`crate::genome`] machinery (design Part 25.6), so a mind's sharpness is a consequence
//! of its inheritance and differs by race and by individual. Acuity is the live channel today
//! (it scales every observation); memory and plasticity are carried, their consumers (belief
//! deterioration and update-rate modulation) the named follow-on.
//!
//! What a [`Mind`] is not: it does not decide or act. There is no utility-AI choice
//! (design Part 8), no drives or goals, and no body in a world to act on. It is the
//! knowing-and-believing half of an agent, the half whose mechanisms are resolved and
//! data-driven; the deciding-and-acting half waits on the systems and the reserved
//! numbers named in the crate's gating notes. The mind is also scheduler-agnostic: it
//! is a pure function of the evidence it is given, applied in any order with the same
//! result, so a later deterministic scheduler (design Part 57) can drive many minds
//! without changing what each concludes.

use std::collections::{BTreeMap, BTreeSet};

use crate::decision::Curve;
use crate::evidence::{AttrKindId, InferenceFrame, InferenceParams, ValueId};
use crate::genome::{Channel, CognitionChannel, GeneSet, Genome};
use crate::tom::{
    detects_deception, AccessChannelId, AccessWeights, NestedFrame, ProjectionRejected,
};
use civsim_core::{Fixed, StableId, StateHasher};

/// A question a mind holds a belief about: a subject and one of its attributes. Ordered
/// so the mind's state walks in a canonical, deterministic order.
type Question = (StableId, AttrKindId);

/// A belief a mind can share in conversation: the question and its committed value, plus
/// the hypothesis frame so a hearer can entertain the same question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedBelief {
    /// The subject the belief is about.
    pub subject: StableId,
    /// The attribute the belief is about.
    pub attr: AttrKindId,
    /// The candidate values of the question.
    pub hyps: Vec<ValueId>,
    /// The committed value being shared.
    pub value: ValueId,
}

/// One access observation about a target mind: which data channel it came through, the
/// target-belief value it points at, and who it came from (provenance).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AccessObs {
    /// The access-relation channel (witnessed, told, said, ...) in the data registry.
    pub channel: AccessChannelId,
    /// The target-belief value this observation supports.
    pub toward: ValueId,
    /// Who the observation came from.
    pub from: StableId,
}

/// The epistemic state of one entity: its first-order beliefs about the world and its
/// models of other minds. Both are keyed by question and kept in sorted maps, so any
/// canonical walk (a hash, a save) is deterministic.
#[derive(Clone, Debug)]
pub struct Mind {
    /// Who this mind belongs to.
    pub id: StableId,
    /// The mind's reasoning acuity (design Part 25.6): it scales the weight extracted from
    /// every observation, so a sharper mind concludes from less. The live cognition channel,
    /// read by [`Mind::consider`], [`Mind::model`], and the world's perception roll.
    pub acuity: Fixed,
    /// The mind's memory capacity (design Part 25.6): it governs belief deterioration (Part
    /// 9), how well a belief resists decay over time. Carried from the genome; its consumer is
    /// [`RetentionLaw`], which scales a retention window by this memory (a sharper mind holds a
    /// belief or a usage-recency window longer), so the phenotype now has a derived law that reads
    /// it. Wiring that law into the canonical deterioration step of the tick is the named follow-on.
    /// Neutral (no modulation) at [`Fixed::ONE`].
    pub memory: Fixed,
    /// The mind's belief plasticity (design Part 25.6, Part 20): it governs how readily
    /// beliefs update under new evidence. Carried from the genome; its consumer, the
    /// update-rate modulation, is the named follow-on. Neutral at [`Fixed::ONE`].
    pub plasticity: Fixed,
    beliefs: BTreeMap<Question, InferenceFrame>,
    models: BTreeMap<Question, NestedFrame>,
    /// The questions this mind is motivated to resolve (the inquiry goals of design 9.13).
    /// A question it wonders about but has not committed a belief on is an open question it
    /// will ask about; being asked seeds a question here, so curiosity spreads.
    wondering: BTreeSet<Question>,
}

impl Mind {
    /// A fresh mind with the given reasoning acuity and a neutral memory and plasticity (no
    /// modulation), no beliefs, models, or open questions. Use this where a mind's cognition
    /// is supplied directly (tests, fixtures); [`Mind::from_genome`] is the path that derives
    /// the whole cognition phenotype from a being's genes.
    pub fn new(id: StableId, acuity: Fixed) -> Self {
        Mind {
            id,
            acuity,
            memory: Fixed::ONE,
            plasticity: Fixed::ONE,
            beliefs: BTreeMap::new(),
            models: BTreeMap::new(),
            wondering: BTreeSet::new(),
        }
    }

    /// A fresh mind whose cognition phenotype is expressed from a genome (design Part 25.6).
    /// Each cognition channel (reasoning acuity, memory capacity, belief plasticity) is read
    /// from the gene set through the same deterministic, float-free [`GeneSet::express`] every
    /// phenotype uses, with the supplied non-genetic `environment` offset (nurture) added per
    /// channel. The mechanism is fixed; which genes feed each channel and with what weight is
    /// data (Principle 11), so two races with different gene sets, or two beings with
    /// different alleles, get different minds from one rule. A being's mind is thus a
    /// consequence of its inheritance rather than an authored number.
    pub fn from_genome(id: StableId, genes: &GeneSet, genome: &Genome, environment: Fixed) -> Self {
        let acuity = genes.express(
            genome,
            Channel::Cognition(CognitionChannel::ReasoningAcuity),
            environment,
        );
        let memory = genes.express(
            genome,
            Channel::Cognition(CognitionChannel::MemoryCapacity),
            environment,
        );
        let plasticity = genes.express(
            genome,
            Channel::Cognition(CognitionChannel::BeliefPlasticity),
            environment,
        );
        Mind {
            id,
            acuity,
            memory,
            plasticity,
            beliefs: BTreeMap::new(),
            models: BTreeMap::new(),
            wondering: BTreeSet::new(),
        }
    }

    /// Take in first-order evidence about the world: a signed weight toward one value of
    /// a subject's attribute, scaled by this mind's acuity. The candidate hypotheses are
    /// supplied so the question can be entertained on first sight; a later assertion that
    /// raises new candidates unions them into the frame (its hypothesis space is the union of
    /// every candidate set asserted about the question), so the committed belief is a pure
    /// function of the evidence set and not of which informant spoke first. This is the
    /// witness-and-told path of the gossip loop (design 9.5) reduced to its core.
    pub fn consider(
        &mut self,
        subject: StableId,
        attr: AttrKindId,
        hyps: impl IntoIterator<Item = ValueId>,
        toward: ValueId,
        weight: Fixed,
        from: StableId,
    ) {
        let acuity = self.acuity;
        let hyps: Vec<ValueId> = hyps.into_iter().collect();
        let frame = self
            .beliefs
            .entry((subject, attr))
            .or_insert_with(|| InferenceFrame::new(subject, attr, hyps.iter().copied()));
        frame.merge_hyps(hyps.iter().copied());
        frame.add_evidence(toward, weight, acuity, from);
    }

    /// The mind's committed belief about a question, or `None` if it has not concluded.
    pub fn belief(
        &self,
        subject: StableId,
        attr: AttrKindId,
        params: &InferenceParams,
    ) -> Option<ValueId> {
        self.beliefs.get(&(subject, attr))?.commit(params)
    }

    /// Update this mind's model of a target mind from second-order access evidence: what
    /// this mind believes the target witnessed, was told, or said, weighted by the data
    /// channel and scaled by this mind's acuity. The nested frame admits only access
    /// evidence about the target (the anti-projection guarantee of [`crate::tom`]), so a
    /// mind models another from access rather than by projecting its own world belief.
    pub fn model(
        &mut self,
        weights: &AccessWeights,
        target: StableId,
        attr: AttrKindId,
        hyps: impl IntoIterator<Item = ValueId>,
        obs: AccessObs,
    ) -> Result<(), ProjectionRejected> {
        let acuity = self.acuity;
        let hyps: Vec<ValueId> = hyps.into_iter().collect();
        let nf = self
            .models
            .entry((target, attr))
            .or_insert_with(|| NestedFrame::new(target, 1, attr, hyps.iter().copied()));
        nf.merge_hyps(hyps.iter().copied());
        nf.observe_access(weights, obs.channel, obs.toward, acuity, obs.from)
    }

    /// Reflect a belief to share: the first committed belief in canonical question
    /// order, or `None` if the mind has concluded nothing. This is the minimal
    /// reflection step of the gossip loop (design 9.5); a richer version weights the
    /// choice by salience.
    pub fn first_committed(&self, params: &InferenceParams) -> Option<SharedBelief> {
        for ((subject, attr), frame) in &self.beliefs {
            if let Some(value) = frame.commit(params) {
                return Some(SharedBelief {
                    subject: *subject,
                    attr: *attr,
                    hyps: frame.hyps().to_vec(),
                    value,
                });
            }
        }
        None
    }

    /// Every belief this mind has committed, in canonical question order. The dialogue
    /// step walks these to decide what is worth telling and to whom (a richer reflection
    /// than [`Mind::first_committed`], which returns only the first).
    pub fn committed_beliefs(&self, params: &InferenceParams) -> Vec<SharedBelief> {
        self.beliefs
            .iter()
            .filter_map(|((subject, attr), frame)| {
                frame.commit(params).map(|value| SharedBelief {
                    subject: *subject,
                    attr: *attr,
                    hyps: frame.hyps().to_vec(),
                    value,
                })
            })
            .collect()
    }

    /// Iterate this mind's belief frames, each a `(subject, attr)` question and the inference frame over
    /// it, in canonical `(subject, attr)` order (the `BTreeMap` order). Lets the deliberative planner
    /// (ideation arc, piece 4) walk the belief store to rank what the being believes toward a goal, and any
    /// reader needing a frame's confidence or support, without owning a copy or exposing the map itself.
    pub fn frames(&self) -> impl Iterator<Item = ((StableId, AttrKindId), &InferenceFrame)> {
        self.beliefs.iter().map(|(&q, f)| (q, f))
    }

    /// This mind's committed belief on one question as a shareable belief, or `None` if it
    /// has not concluded. The answer a being gives when asked.
    pub fn shared_belief(
        &self,
        subject: StableId,
        attr: AttrKindId,
        params: &InferenceParams,
    ) -> Option<SharedBelief> {
        let frame = self.beliefs.get(&(subject, attr))?;
        frame.commit(params).map(|value| SharedBelief {
            subject,
            attr,
            hyps: frame.hyps().to_vec(),
            value,
        })
    }

    /// Register an open question this mind is motivated to resolve (an inquiry goal of
    /// design 9.13). A being wonders about a question it has reason to want answered;
    /// being asked seeds the same goal, so curiosity spreads through a conversation.
    pub fn wonder(&mut self, subject: StableId, attr: AttrKindId) {
        self.wondering.insert((subject, attr));
    }

    /// Whether this mind is motivated to resolve a question.
    pub fn is_wondering(&self, subject: StableId, attr: AttrKindId) -> bool {
        self.wondering.contains(&(subject, attr))
    }

    /// The open questions this mind would ask about: those it wonders about but has not yet
    /// committed a belief on, in canonical order. Once it commits, a question drops out, so
    /// a being stops asking what it has learned.
    pub fn open_questions(&self, params: &InferenceParams) -> Vec<Question> {
        self.wondering
            .iter()
            .copied()
            .filter(|(subject, attr)| self.belief(*subject, *attr, params).is_none())
            .collect()
    }

    /// What this mind believes a target mind believes about a question, or `None`.
    pub fn modeled_belief(
        &self,
        target: StableId,
        attr: AttrKindId,
        params: &InferenceParams,
    ) -> Option<ValueId> {
        self.models.get(&(target, attr))?.commit(params)
    }

    /// Whether this mind sees through an assertion: it holds an access-built model of the
    /// speaker's own belief that has committed to a value other than what was asserted.
    /// Returns `false` if it has no committed model of the speaker on this question.
    pub fn detects_lie(
        &self,
        speaker: StableId,
        attr: AttrKindId,
        asserted: ValueId,
        params: &InferenceParams,
    ) -> bool {
        match self.models.get(&(speaker, attr)) {
            Some(nf) => detects_deception(nf, asserted, params),
            None => false,
        }
    }

    /// A canonical 128-bit hash of the mind's whole epistemic state: its beliefs then its
    /// models, each walked in sorted question order, each question's clamped totals and
    /// committed value folded in. First-order beliefs are read under `belief_params` (the
    /// `evidence.*` calibrations) and models under `meta_params` (the `tom.*`
    /// calibrations), since the design reserves the two separately. A pure function of the
    /// state, so two minds that took the same evidence in any order hash identically.
    pub fn state_hash(
        &self,
        belief_params: &InferenceParams,
        meta_params: &InferenceParams,
    ) -> u128 {
        let mut h = StateHasher::new();
        h.write_stable(self.id);
        h.write_fixed(self.acuity);
        h.write_fixed(self.memory);
        h.write_fixed(self.plasticity);
        for ((subject, attr), frame) in &self.beliefs {
            h.write_stable(*subject);
            h.write_u32(attr.0);
            for &v in frame.hyps() {
                h.write_u32(v);
                h.write_fixed(frame.clamped_total(v, belief_params).unwrap());
            }
            h.write_u32(frame.commit(belief_params).unwrap_or(u32::MAX));
        }
        for ((target, attr), nf) in &self.models {
            h.write_stable(*target);
            h.write_u32(attr.0);
            for &v in nf.hyps() {
                h.write_u32(v);
                h.write_fixed(nf.clamped_total(v, meta_params).unwrap());
            }
            h.write_u32(nf.commit(meta_params).unwrap_or(u32::MAX));
        }
        // The open questions, in canonical order (the BTreeSet is already sorted).
        for (subject, attr) in &self.wondering {
            h.write_stable(*subject);
            h.write_u32(attr.0);
        }
        h.finish()
    }
}

/// The retention law: how long a memory-governed window lasts, as a function of a mind's memory
/// capacity (design Parts 9 and 25.6, the belief-deterioration consumer of [`Mind::memory`]). A
/// window over the mind's history, a belief-deterioration span or the R-LANG-DET usage-recency
/// window, is not one fixed length: a sharper-memoried mind holds it longer. The law scales a base
/// window (in ticks) by a data [`Curve`] read at the mind's memory, so the same base window becomes
/// a longer effective window in a high-memory mind and a shorter one in a forgetful mind. The
/// mechanism is fixed Rust; the base window and the curve are data (Principle 11), and the law keys
/// on the memory scalar, never a race id (Principle 9), so it differentiates per being and per race
/// from one rule. The usage-recency window reads it at the representative memory (the band's mean
/// memory) with the base window `langdet.usage_recency_window` and the scaling curve
/// `langdet.retention_memory_scale`.
#[derive(Clone, Debug)]
pub struct RetentionLaw {
    /// The window-scaling curve: a memory capacity in, a multiplier on the base window out. Owner
    /// data; an increasing curve lengthens the window for a sharper memory, and a flat curve yields
    /// the base window for every memory (the memory-independent special case).
    pub scale_by_memory: Curve,
}

impl RetentionLaw {
    /// The retention window in whole ticks for a mind of the given `memory`, scaling `base_window`
    /// by the curve read at that memory. The multiply and the floor are done in i128 so a large base
    /// window cannot overflow the fixed-point grid, and the result is floored at one tick (a window
    /// is never zero-length). A non-positive scale (a curve dipping below zero) floors to one tick
    /// rather than vanishing. A pure, deterministic function of its inputs, so it replays bit for bit
    /// and carries no race branch.
    pub fn window_ticks_for(&self, memory: Fixed, base_window: u64) -> u64 {
        let scale_bits = self.scale_by_memory.eval(memory).to_bits().max(0) as i128;
        // base_window * scale, with scale carried as its Q32.32 bits: the product is
        // base_window * scale * 2^32, and the arithmetic shift right by the fractional bits is the
        // exact floor to whole ticks.
        let product = (base_window as i128) * scale_bits;
        // Saturate at u64::MAX rather than truncating a product past it (the same clamp
        // absence::scale_u64_by_fixed uses), then floor at one tick.
        let ticks = (product >> Fixed::FRAC_BITS).min(u64::MAX as i128).max(0) as u64;
        ticks.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{
        Allele, AlleleState, DominanceMode, GeneDef, GeneEffect, GeneId, Haplotype, SchemeId,
    };
    use crate::tom::AccessWeights;

    const LOCATION: AttrKindId = AttrKindId(0);
    const BASKET: u32 = 10;
    const BOX: u32 = 20;
    const WITNESSED: AccessChannelId = AccessChannelId(1);
    const TOLD: AccessChannelId = AccessChannelId(2);

    fn params() -> InferenceParams {
        InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    fn weights() -> AccessWeights {
        AccessWeights::from_pairs([(WITNESSED, Fixed::from_int(4)), (TOLD, Fixed::from_int(3))])
    }

    #[test]
    fn a_mind_forms_and_revises_a_belief() {
        let marble = StableId(99);
        let mut m = Mind::new(StableId(1), Fixed::ONE);
        m.consider(
            marble,
            LOCATION,
            [BASKET, BOX],
            BASKET,
            Fixed::from_int(4),
            StableId(2),
        );
        assert_eq!(m.belief(marble, LOCATION, &params()), Some(BASKET));
        // Heavier later evidence revises the belief (defeasible).
        m.consider(
            marble,
            LOCATION,
            [BASKET, BOX],
            BOX,
            Fixed::from_int(9),
            StableId(3),
        );
        assert_eq!(m.belief(marble, LOCATION, &params()), Some(BOX));
    }

    #[test]
    fn acuity_scales_what_a_mind_extracts() {
        let marble = StableId(99);
        let w = Fixed::from_int(2); // below the threshold of 3 at unit acuity
        let mut dull = Mind::new(StableId(1), Fixed::ONE);
        dull.consider(marble, LOCATION, [BASKET, BOX], BASKET, w, StableId(2));
        assert_eq!(dull.belief(marble, LOCATION, &params()), None);

        let mut sharp = Mind::new(StableId(1), Fixed::from_int(2));
        sharp.consider(marble, LOCATION, [BASKET, BOX], BASKET, w, StableId(2));
        assert_eq!(sharp.belief(marble, LOCATION, &params()), Some(BASKET));
    }

    #[test]
    fn a_mind_models_another_and_sees_a_lie() {
        let speaker = StableId(2);
        let mut m = Mind::new(StableId(1), Fixed::ONE);
        // The mind witnessed that the speaker saw the marble in the basket.
        let obs = AccessObs {
            channel: WITNESSED,
            toward: BASKET,
            from: StableId(1),
        };
        m.model(&weights(), speaker, LOCATION, [BASKET, BOX], obs)
            .unwrap();
        assert_eq!(m.modeled_belief(speaker, LOCATION, &params()), Some(BASKET));
        // The speaker asserts the box; the mind sees through it.
        assert!(m.detects_lie(speaker, LOCATION, BOX, &params()));
        assert!(!m.detects_lie(speaker, LOCATION, BASKET, &params()));
    }

    fn cognition_gene(id: u32, channel: Channel) -> GeneDef {
        GeneDef {
            id: GeneId(id),
            effects: vec![GeneEffect {
                channel,
                weight: Fixed::ONE,
            }],
            dominance: DominanceMode::additive(),
        }
    }

    fn cognition_genes() -> GeneSet {
        GeneSet {
            genes: vec![
                cognition_gene(0, Channel::Cognition(CognitionChannel::ReasoningAcuity)),
                cognition_gene(1, Channel::Cognition(CognitionChannel::MemoryCapacity)),
                cognition_gene(2, Channel::Cognition(CognitionChannel::BeliefPlasticity)),
            ],
        }
    }

    fn haploid(acuity: i32, memory: i32, plasticity: i32) -> Genome {
        let allele = |v: i32| Allele {
            additive: Fixed::from_int(v),
            state: AlleleState(0),
            origin: 0,
        };
        Genome {
            scheme: SchemeId(0),
            haps: vec![Haplotype {
                alleles: vec![allele(acuity), allele(memory), allele(plasticity)],
            }],
        }
    }

    #[test]
    fn cognition_phenotype_comes_from_the_genome() {
        // Each cognition channel is expressed from the genes, so the mind's sharpness is its
        // inheritance, not an authored number.
        let genes = cognition_genes();
        let m = Mind::from_genome(StableId(1), &genes, &haploid(2, 3, 4), Fixed::ZERO);
        assert_eq!(m.acuity, Fixed::from_int(2));
        assert_eq!(m.memory, Fixed::from_int(3));
        assert_eq!(m.plasticity, Fixed::from_int(4));

        // The expressed acuity drives behaviour: a sharp genome concludes from a sub-threshold
        // weight the same dull genome cannot, the genome-to-cognition-to-belief chain end to end.
        let marble = StableId(99);
        let w = Fixed::from_int(2); // below the threshold of 3 at unit acuity
        let mut sharp = Mind::from_genome(StableId(1), &genes, &haploid(2, 3, 4), Fixed::ZERO);
        sharp.consider(marble, LOCATION, [BASKET, BOX], BASKET, w, StableId(2));
        assert_eq!(sharp.belief(marble, LOCATION, &params()), Some(BASKET));

        let mut dull = Mind::from_genome(StableId(1), &genes, &haploid(1, 3, 4), Fixed::ZERO);
        dull.consider(marble, LOCATION, [BASKET, BOX], BASKET, w, StableId(2));
        assert_eq!(dull.belief(marble, LOCATION, &params()), None);
    }

    #[test]
    fn state_hash_is_order_independent() {
        let marble = StableId(99);
        let speaker = StableId(2);
        let p = params();
        let w = weights();

        let mut a = Mind::new(StableId(1), Fixed::ONE);
        a.consider(
            marble,
            LOCATION,
            [BASKET, BOX],
            BASKET,
            Fixed::from_int(4),
            StableId(5),
        );
        a.consider(
            marble,
            LOCATION,
            [BASKET, BOX],
            BOX,
            Fixed::from_int(2),
            StableId(6),
        );
        let obs = AccessObs {
            channel: TOLD,
            toward: BASKET,
            from: StableId(1),
        };
        a.model(&w, speaker, LOCATION, [BASKET, BOX], obs).unwrap();

        let mut b = Mind::new(StableId(1), Fixed::ONE);
        b.model(&w, speaker, LOCATION, [BASKET, BOX], obs).unwrap();
        b.consider(
            marble,
            LOCATION,
            [BASKET, BOX],
            BOX,
            Fixed::from_int(2),
            StableId(6),
        );
        b.consider(
            marble,
            LOCATION,
            [BASKET, BOX],
            BASKET,
            Fixed::from_int(4),
            StableId(5),
        );

        assert_eq!(a.state_hash(&p, &p), b.state_hash(&p, &p));
    }

    #[test]
    fn a_retention_window_scales_with_memory_and_a_flat_curve_holds_one_window_no_raceid() {
        // An increasing scale curve: memory 0 keeps the base window, memory 1 doubles it. A sharper
        // mind holds the window longer, from the memory scalar alone, no race id anywhere.
        let law = RetentionLaw {
            scale_by_memory: Curve::new([
                (Fixed::ZERO, Fixed::ONE),
                (Fixed::ONE, Fixed::from_int(2)),
            ]),
        };
        let base = 1000u64;
        let forgetful = law.window_ticks_for(Fixed::ZERO, base);
        let sharp = law.window_ticks_for(Fixed::ONE, base);
        assert_eq!(forgetful, base, "the base window at the low-memory end");
        assert_eq!(sharp, 2 * base, "a sharper memory holds the window longer");
        assert!(
            sharp > forgetful,
            "different memory gives different windows ({sharp} > {forgetful})"
        );
        // A mid-memory mind reads a window between the two ends (the curve interpolates).
        let mid = law.window_ticks_for(Fixed::from_ratio(1, 2), base);
        assert!(
            mid > forgetful && mid < sharp,
            "a mid memory reads a mid window"
        );

        // A flat curve reproduces one window for every memory: the memory channel is switched off,
        // the degenerate single-window special case.
        let flat = RetentionLaw {
            scale_by_memory: Curve::new([
                (Fixed::ZERO, Fixed::from_int(3)),
                (Fixed::ONE, Fixed::from_int(3)),
            ]),
        };
        let w0 = flat.window_ticks_for(Fixed::ZERO, base);
        let w1 = flat.window_ticks_for(Fixed::from_ratio(1, 2), base);
        let w2 = flat.window_ticks_for(Fixed::from_int(4), base);
        assert_eq!(w0, 3 * base);
        assert_eq!(w0, w1);
        assert_eq!(w1, w2);

        // A large base window does not overflow the fixed-point grid (the i128 multiply), and the
        // window is never zero-length.
        let big = law.window_ticks_for(Fixed::ONE, 31_536_000);
        assert_eq!(big, 63_072_000);
        assert_eq!(
            law.window_ticks_for(Fixed::ZERO, 0),
            1,
            "a window is at least one tick"
        );

        // Defect 8 (saturation): a base window and scale whose product exceeds u64::MAX saturates at
        // u64::MAX rather than truncating `(product >> 32) as u64`, which wrapped 2^64 to zero and then
        // floored to a one-tick window (a forgetful mind, the opposite of the intended huge window).
        // At memory one `law` doubles, so a 2^63 base window drives the product to exactly 2^96, whose
        // shift-right-by-32 is 2^64, one past u64::MAX.
        assert_eq!(
            law.window_ticks_for(Fixed::ONE, 1u64 << 63),
            u64::MAX,
            "a product past u64::MAX saturates rather than wrapping to a one-tick window"
        );
    }
}
