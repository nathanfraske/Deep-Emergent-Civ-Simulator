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

//! The discovery loop's action-as-hypothesis machinery (ideation / experiential-discovery arc, piece 2,
//! slice 2b): from what a being can DO (its afforded primitives) and what it SENSES it could act on (its
//! slice-2a affordance percepts), propose candidate action sequences, so a being discovers actions it has
//! not yet been rewarded for. The proposals are scored and repeated by the piece-1 appetitive learner, so a
//! technique (chop a fracturable thing with a sharp edge) EMERGES as a high-confidence belief PATH through
//! primitives, never a coded verb (Principle 8).
//!
//! The BINDING GRAPH ([`candidate_bindings`]) is the GENERIC cartesian of the body's afforded primitives
//! and the affordance-typed targets the being perceives, with NO coded what-binds-to-what table: the being
//! proposes "issue primitive P against a thing presenting affordance CHANNEL C" for the combinations, and
//! the reward learner sorts which pay off. The target is keyed on the affordance CHANNEL (the TYPE of thing,
//! fracturable or sharp), value-blind, so "strike a fracturable thing" is ONE template that generalises
//! across instances (the percept VALUE drives which candidate the sampler prefers, in slice 2b's sampler,
//! not the template's identity). This is the affordance-bound sampling the design calls for, kept emergent:
//! there is no `if primitive == STRIKE { target = fracturable }` branch anywhere.
//!
//! This slice is READ only: the enumeration is a pure, RNG-free function off the run path (nothing samples
//! or enacts yet, and `state_hash` folds nothing), so every existing scenario replays bit-for-bit. Slice
//! 2b's sampler draws a proposal from these candidates biased by belief and need (its RNG counter-keyed
//! under a new registered phase), and slice 2c (WIRE) advances a chosen candidate across ticks and enacts
//! it.

use civsim_core::{DrawKey, Fixed, Phase, StableId};

use crate::agent::Mind;
use crate::evidence::InferenceParams;
use crate::homeostasis::AffordanceId;
use crate::learn::{sequence_subject, SequenceStep, REWARDS, REWARD_ATTR};

/// The candidate single-step action bindings a being can propose this tick: the GENERIC cartesian of its
/// afforded primitives and the affordance-typed targets it currently perceives, in a canonical order
/// (primitive id, then affordance channel), drawing no randomness. For each afforded primitive and each
/// affordance percept the being senses as PRESENT (its scalar strictly positive), one
/// [`SequenceStep`] keyed on the primitive and the affordance CHANNEL as its `target_bucket` (the target's
/// TYPE, value-blind), with a zero `param_bucket` until the sampler (slice 2b) and the stepper (slice 2c)
/// supply a graded how.
///
/// There is NO coded primitive-to-affordance pairing: every afforded primitive is proposed against every
/// present affordance channel, and selection (the piece-1 reward learner) keeps the combinations that pay
/// off, so a technique emerges as a learned belief path rather than a designer's recipe (Principle 8). A
/// channel the being does not perceive (its percept zero, no such matter or tool in reach) contributes no
/// candidate, so the proposal set is bounded by what is present in reach. `percepts` is the
/// [`crate::affordance_percept::AffordancePerceptRegistry::perceive`] read, in its canonical channel order;
/// `afforded` is the [`crate::homeostasis::AffordanceRegistry::afforded`] set, in canonical id order.
pub fn candidate_bindings(afforded: &[AffordanceId], percepts: &[Fixed]) -> Vec<SequenceStep> {
    let mut out = Vec::with_capacity(afforded.len() * percepts.len());
    for &primitive in afforded {
        for (channel, &value) in percepts.iter().enumerate() {
            if value > Fixed::ZERO {
                out.push(SequenceStep {
                    primitive: primitive.0,
                    target_bucket: channel as i64,
                    param_bucket: 0,
                });
            }
        }
    }
    out
}

/// The reserved calibration the discovery sampler reads (RESERVED, fail-loud from the manifest, none
/// fabricated). The mechanism is fixed Rust; this is the owner's number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiscoveryCalib {
    /// RESERVED. The relative weight of a candidate the being does NOT yet believe pays off, against the
    /// unit weight of one it does: the baseline chance of trying a non-top-belief binding, so a being still
    /// explores when it already has a rewarded habit, a bounded exploration-versus-exploitation floor. In
    /// `[0, 1]`: at zero the being only ever proposes actions it already believes pay off (pure exploit, no
    /// discovery); at one every candidate is equally likely (pure explore, the belief no longer biases). Set
    /// at the baseline exploration rate the ecology needs to keep discovering without abandoning a working
    /// technique. Surfaced with its basis, never fabricated.
    pub exploration_floor: Fixed,
    /// RESERVED. The prediction-error threshold above which a being's surprise MODULATES its exploration
    /// (ideation arc, piece 3, slice 3b): a predicted-versus-felt reward mismatch below this is not read as
    /// surprise, so ordinary interoceptive noise does not lift exploration. Basis: the interoceptive noise
    /// floor already defined for reward and harm, so a sub-resolution mismatch is not surprise (a
    /// performance-and-resolution bound, not a realism one). Surfaced with its basis, never fabricated.
    pub surprise_threshold: Fixed,
    /// RESERVED. The exploration variance gain, how much a being's surprise LIFTS its effective exploration
    /// propensity above its heritable base (ideation arc, piece 3, slice 3b): the effective enact rate is
    /// `base * (1 + surprise_gain * surprise)`, so a surprised being enacts its proposals more and a
    /// well-predicting one stays at its base. MULTIPLICATIVE, so a founder (zero base) never explores however
    /// surprised (founder-zero holds). Basis: the existing EXPLORE heading-noise scale and the sensorium
    /// just-noticeable difference, set so a unit surprise lifts exploration by a proportionate step rather
    /// than swamping the heritable base. Surfaced with its basis, never fabricated.
    pub surprise_gain: Fixed,
    /// RESERVED. The deliberation depth cap: how many of its believed-good actions a being ranks when it
    /// PLANS toward a goal (ideation arc, piece 4, the planner's cognition budget, `planning.depth_cap`
    /// wired here). A being deliberates over its top few beliefs, not its whole store, so a longer plan
    /// past the confidence noise is not worth the per-tick cost. Basis: the per-tick cognition budget and
    /// the depth beyond which the next-ranked belief is no better than noise. Surfaced with its basis.
    pub plan_depth_cap: usize,
}

impl DiscoveryCalib {
    /// A labelled dev fixture for the unit tests and the pre-wire scenarios: a modest exploration floor, so
    /// a believed-good action is preferred but an unproven one is still tried a fraction of the time, and a
    /// surprise threshold and gain that let a mispredicted action lift exploration. The manifest values are
    /// reserved; this is only the fixture, never the canonical number.
    pub fn dev_default() -> DiscoveryCalib {
        DiscoveryCalib {
            exploration_floor: Fixed::from_ratio(1, 4),
            surprise_threshold: Fixed::from_ratio(1, 2),
            surprise_gain: Fixed::ONE,
            plan_depth_cap: 8,
        }
    }
}

/// The roulette weight of one candidate binding: the unit weight when the being holds a committed REWARDS
/// belief about the candidate's sequence (it believes this action pays off, so it exploits), the reserved
/// exploration floor otherwise (an untried or unrewarded action, still proposed at the floor rate so
/// discovery never stops). Reads only the being's own reward belief on the disjoint `REWARD_ATTR`, never a
/// coded preference (Principle 9); the binary belief test mirrors [`crate::learn::appetitive_salience`].
fn candidate_weight(
    mind: &Mind,
    step: &SequenceStep,
    calib: &DiscoveryCalib,
    params: &InferenceParams,
) -> Fixed {
    let subject = sequence_subject(std::slice::from_ref(step));
    if mind.belief(subject, REWARD_ATTR, params) == Some(REWARDS) {
        Fixed::ONE
    } else {
        calib.exploration_floor
    }
}

/// Sample which candidate action a being PROPOSES this tick from the binding graph, a belief-weighted
/// roulette drawn from the counter-keyed RNG (ideation arc, piece 2, slice 2b). Each candidate carries the
/// unit weight when the being believes it pays off and the reserved exploration floor otherwise
/// ([`candidate_weight`]), so the being prefers a rewarded habit but still tries the unproven at the floor
/// rate. The draw is `DrawKey::entity(being, tick, Phase::HYPOTHESIZE).rng(seed)`, keyed on the being and
/// the tick under a phase disjoint from every other draw, so a proposal is a reproducible function of the
/// seed, the being, and the tick, never the camera: replaying a run reproduces the identical hypothesis.
/// Returns `None` where the being can propose nothing (no candidates) or nothing carries any weight (a zero
/// floor and no belief). Pure over `(candidates, mind, calib, params, being, tick, seed)`; the binding
/// graph and the perception it reads stay RNG-free, this is the only draw.
pub fn sample_candidate(
    candidates: &[SequenceStep],
    mind: &Mind,
    calib: &DiscoveryCalib,
    params: &InferenceParams,
    being: StableId,
    tick: u64,
    seed: u64,
) -> Option<SequenceStep> {
    if candidates.is_empty() {
        return None;
    }
    let weights: Vec<Fixed> = candidates
        .iter()
        .map(|s| candidate_weight(mind, s, calib, params))
        .collect();
    let total = Fixed::saturating_sum(weights.iter().copied());
    if total <= Fixed::ZERO {
        return None; // no candidate carries any weight: a being with a zero floor and no belief proposes nothing
    }
    // A uniform draw in [0, total), the roulette pointer, from the counter-keyed RNG under the hypothesis
    // phase (counter zero, one proposal per being per tick).
    let target = DrawKey::entity(being.0, tick, Phase::HYPOTHESIZE)
        .rng(seed)
        .unit_fixed(0)
        .checked_mul(total)
        .unwrap_or(Fixed::ZERO);
    let mut acc = Fixed::ZERO;
    for (cand, w) in candidates.iter().zip(&weights) {
        acc = acc.saturating_add(*w);
        if target < acc {
            return Some(*cand);
        }
    }
    // A fixed-point rounding overshoot leaves the pointer at the very top of the wheel: award the last
    // weighted candidate rather than none.
    candidates.last().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::{EXTRACT, GRASP, STRIKE};
    use crate::learn::{RewardLearningCalib, NEUTRAL};

    fn params() -> InferenceParams {
        InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    const SEED: u64 = 0x00D1_5C05;

    #[test]
    fn the_binding_graph_is_the_generic_cartesian_of_afforded_primitives_and_present_targets() {
        // Slice 2b: every afforded primitive is proposed against every PRESENT affordance channel, with no
        // coded pairing. A being that affords GRASP, STRIKE, and EXTRACT and perceives two present
        // affordances (channel 0 fracturable, channel 1 sharp) proposes all six primitive-times-channel
        // bindings, each keyed on the primitive and the affordance CHANNEL as its target.
        let afforded = [GRASP, STRIKE, EXTRACT];
        let percepts = [Fixed::from_ratio(8, 10), Fixed::from_ratio(9, 10)]; // both present
        let candidates = candidate_bindings(&afforded, &percepts);
        assert_eq!(
            candidates.len(),
            6,
            "three primitives times two present channels"
        );
        // Canonical order: primitive-major, then channel. The first two are GRASP against channels 0 and 1.
        assert_eq!(candidates[0].primitive, GRASP.0);
        assert_eq!(candidates[0].target_bucket, 0);
        assert_eq!(candidates[1].primitive, GRASP.0);
        assert_eq!(candidates[1].target_bucket, 1);
        // The target is the CHANNEL, value-blind: two beings sensing different fracturability levels on the
        // same channel propose the SAME binding and mint the SAME sequence subject, so one template
        // generalises across instances (the value drives the sampler, not the identity).
        let strong = candidate_bindings(&[STRIKE], &[Fixed::from_ratio(9, 10)]);
        let weak = candidate_bindings(&[STRIKE], &[Fixed::from_ratio(2, 10)]);
        assert_eq!(
            sequence_subject(&[strong[0]]),
            sequence_subject(&[weak[0]]),
            "the same primitive on the same affordance channel is one template, value-blind"
        );
        // A different channel is a different template.
        let sharp = candidate_bindings(&[STRIKE], &[Fixed::ZERO, Fixed::from_ratio(9, 10)]);
        assert_ne!(
            sequence_subject(&[strong[0]]),
            sequence_subject(&[sharp[0]]),
            "the same primitive on a different affordance channel is a distinct template"
        );
    }

    #[test]
    fn an_absent_affordance_or_no_primitive_proposes_nothing() {
        // A channel the being does not perceive (its percept zero) contributes no candidate, so the proposal
        // set is bounded by what is present in reach: a being sensing only channel 1 (sharp) proposes only
        // bindings against channel 1, never against the absent channel 0.
        let candidates =
            candidate_bindings(&[STRIKE, GRASP], &[Fixed::ZERO, Fixed::from_ratio(9, 10)]);
        assert_eq!(
            candidates.len(),
            2,
            "two primitives times one present channel"
        );
        assert!(
            candidates.iter().all(|c| c.target_bucket == 1),
            "only the present channel is bound"
        );
        // No afforded primitive, or no perceived affordance, proposes nothing (a being that can do nothing,
        // or senses nothing to act on, has no hypothesis to test).
        assert!(candidate_bindings(&[], &[Fixed::ONE]).is_empty());
        assert!(candidate_bindings(&[STRIKE], &[]).is_empty());
        assert!(candidate_bindings(&[STRIKE], &[Fixed::ZERO, Fixed::ZERO]).is_empty());
    }

    // Commit a REWARDS belief on a mind about one candidate's sequence, so the sampler weights it full.
    fn believe(mind: &mut Mind, step: &SequenceStep) {
        let subject = sequence_subject(std::slice::from_ref(step));
        let w = RewardLearningCalib::dev_default().observation_weight();
        for _ in 0..3 {
            mind.consider(
                subject,
                REWARD_ATTR,
                [REWARDS, NEUTRAL],
                REWARDS,
                w,
                mind.id,
            );
        }
        assert_eq!(mind.belief(subject, REWARD_ATTR, &params()), Some(REWARDS));
    }

    #[test]
    fn a_believed_candidate_dominates_the_draw_and_the_floor_keeps_exploring() {
        // Slice 2b: the sampler prefers what the being believes pays off, but the reserved exploration floor
        // keeps an unproven action in play. A being that has committed the "STRIKE a fracturable thing"
        // belief proposes that action every tick when the floor is zero (pure exploit), while a naive being
        // with a positive floor still proposes SOMETHING (it explores).
        let candidates = candidate_bindings(&[STRIKE, GRASP], &[Fixed::ONE, Fixed::ONE]); // 4 candidates
        let believed = candidates[0]; // STRIKE against channel 0
        let mut sage = Mind::new(StableId(1), Fixed::ONE);
        believe(&mut sage, &believed);

        // Zero floor: only the believed candidate carries weight, so it is proposed every tick, by belief
        // alone, never a coded preference.
        let exploit = DiscoveryCalib {
            exploration_floor: Fixed::ZERO,
            ..DiscoveryCalib::dev_default()
        };
        for tick in 0..8 {
            assert_eq!(
                sample_candidate(&candidates, &sage, &exploit, &params(), StableId(1), tick, SEED),
                Some(believed),
                "with no exploration floor, the being proposes only the action it believes pays off"
            );
        }
        // A naive being with a zero floor believes nothing and carries no weight: it proposes nothing.
        let naive = Mind::new(StableId(2), Fixed::ONE);
        assert_eq!(
            sample_candidate(
                &candidates,
                &naive,
                &exploit,
                &params(),
                StableId(2),
                0,
                SEED
            ),
            None,
            "a being that believes nothing and never explores proposes nothing"
        );
        // With the reserved floor, the naive being still proposes an (unproven) action: discovery never stops.
        let explore = DiscoveryCalib::dev_default();
        assert!(
            sample_candidate(
                &candidates,
                &naive,
                &explore,
                &params(),
                StableId(2),
                0,
                SEED
            )
            .is_some(),
            "with an exploration floor, an unproven action is still tried"
        );
    }

    #[test]
    fn the_proposal_is_deterministic_and_counter_keyed() {
        // The draw is a reproducible function of the being, the tick, and the seed (counter-keyed under the
        // hypothesis phase), so a replayed run proposes the identical hypothesis, and an empty candidate set
        // proposes nothing.
        let candidates = candidate_bindings(&[STRIKE, GRASP, EXTRACT], &[Fixed::ONE, Fixed::ONE]); // 6
        let naive = Mind::new(StableId(7), Fixed::ONE);
        let calib = DiscoveryCalib::dev_default();
        let a = sample_candidate(&candidates, &naive, &calib, &params(), StableId(7), 3, SEED);
        let b = sample_candidate(&candidates, &naive, &calib, &params(), StableId(7), 3, SEED);
        assert_eq!(
            a, b,
            "the proposal is reproducible for one being, tick, and seed"
        );
        assert!(a.is_some());
        assert_eq!(
            sample_candidate(&[], &naive, &calib, &params(), StableId(7), 0, SEED),
            None,
            "no candidates, no proposal"
        );
    }
}
