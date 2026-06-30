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
//! What a [`Mind`] is not: it does not decide or act. There is no utility-AI choice
//! (design Part 8), no drives or goals, and no body in a world to act on. It is the
//! knowing-and-believing half of an agent, the half whose mechanisms are resolved and
//! data-driven; the deciding-and-acting half waits on the systems and the reserved
//! numbers named in the crate's gating notes. The mind is also scheduler-agnostic: it
//! is a pure function of the evidence it is given, applied in any order with the same
//! result, so a later deterministic scheduler (design Part 57) can drive many minds
//! without changing what each concludes.

use std::collections::{BTreeMap, BTreeSet};

use crate::evidence::{AttrKindId, InferenceFrame, InferenceParams, ValueId};
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
    /// The mind's reasoning acuity (design Part 25): it scales the weight extracted from
    /// every observation, so a sharper mind concludes from less.
    pub acuity: Fixed,
    beliefs: BTreeMap<Question, InferenceFrame>,
    models: BTreeMap<Question, NestedFrame>,
    /// The questions this mind is motivated to resolve (the inquiry goals of design 9.13).
    /// A question it wonders about but has not committed a belief on is an open question it
    /// will ask about; being asked seeds a question here, so curiosity spreads.
    wondering: BTreeSet<Question>,
}

impl Mind {
    /// A fresh mind with no beliefs, models, or open questions.
    pub fn new(id: StableId, acuity: Fixed) -> Self {
        Mind {
            id,
            acuity,
            beliefs: BTreeMap::new(),
            models: BTreeMap::new(),
            wondering: BTreeSet::new(),
        }
    }

    /// Take in first-order evidence about the world: a signed weight toward one value of
    /// a subject's attribute, scaled by this mind's acuity. The candidate hypotheses are
    /// supplied so the question can be entertained on first sight; once a question exists
    /// its hypothesis frame stands and the `hyps` argument is ignored. This is the
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
        let frame = self
            .beliefs
            .entry((subject, attr))
            .or_insert_with(|| InferenceFrame::new(subject, attr, hyps));
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
        let nf = self
            .models
            .entry((target, attr))
            .or_insert_with(|| NestedFrame::new(target, 1, attr, hyps));
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
