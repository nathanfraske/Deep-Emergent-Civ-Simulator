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
//! the reward learner sorts which pay off. The candidate carries the affordance CHANNEL (the TYPE of thing,
//! fracturable or sharp) as its target. The GRAIN the belief keys on is a data-driven choice
//! (social-learning arc, piece 3, material granularity), set by the world through
//! [`crate::runner::Embodiment::set_granular_beliefs`] and routed through the one shared
//! [`crate::learn::step_belief_subject`] every write and read call: by DEFAULT the belief keys on the
//! PRIMITIVE ALONE (`target_bucket` and `param_bucket` zeroed), so a primitive a being learned pays off is
//! preferred against every present target rather than re-learned per type; when a world arms GRANULAR
//! beliefs, the belief keys on the primitive against the target's affordance channel AND its quantized
//! perceived value, so "strike a hard thing" and "strike a soft thing" diverge as distinct learned actions.
//! The credit, [`candidate_weight`], [`crate::learn::appetitive_salience`], the planner match, and the
//! forward model all key through that one function, so write and read never disagree on the grain (no belief
//! is masked). This is the affordance-bound sampling the design calls for, kept emergent: there is no
//! `if primitive == STRIKE { target = fracturable }` branch anywhere.
//!
//! This slice is READ only: the enumeration is a pure, RNG-free function off the run path (nothing samples
//! or enacts yet, and `state_hash` folds nothing), so every existing scenario replays bit-for-bit. Slice
//! 2b's sampler draws a proposal from these candidates biased by belief and need (its RNG counter-keyed
//! under a new registered phase), and slice 2c (WIRE) advances a chosen candidate across ticks and enacts
//! it.

use civsim_core::{DrawKey, Fixed, Phase, StableId};

use crate::homeostasis::AffordanceId;
use crate::learn::{step_belief_subject, REWARDS, REWARD_ATTR};
use civsim_bio::agent::Mind;
use civsim_bio::evidence::InferenceParams;
use civsim_foundation::calibration::{CalibrationError, CalibrationManifest};
use civsim_foundation::sequence::SequenceStep;

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
/// `granular` and `value_granularity` (social-learning arc, piece 3, material granularity): when `granular`
/// is true, each candidate carries the target's quantized perceived VALUE in its `param_bucket`
/// (`feature_bucket(value, value_granularity)`, the just-noticeable how-hard), so a primitive against a HARD
/// target and against a SOFT target of the same channel are DISTINCT candidates the reward learner sorts
/// separately. When false (the default), `param_bucket` is zero, so the candidate is value-blind exactly as
/// before and the run is byte-identical.
pub fn candidate_bindings(
    afforded: &[AffordanceId],
    percepts: &[Fixed],
    granular: bool,
    value_granularity: Fixed,
) -> Vec<SequenceStep> {
    let mut out = Vec::with_capacity(afforded.len() * percepts.len());
    for &primitive in afforded {
        for (channel, &value) in percepts.iter().enumerate() {
            if value > Fixed::ZERO {
                out.push(SequenceStep {
                    primitive: primitive.0,
                    // When granular, a real candidate's channel is 1-based, so `target_bucket == 0` is RESERVED
                    // for the base controller's "no target" sentinel (a controller action names no target and
                    // keys `(primitive, 0, 0)`); a real granular candidate never keys `(primitive, 0, 0)`, so
                    // an untargeted controller belief and a channel-0 candidate belief cannot alias (the deep
                    // audit's zero-triple finding). Non-granular keeps the raw channel: `target_bucket` never
                    // reaches the primitive-only belief key there, so the run stays byte-identical.
                    target_bucket: if granular {
                        channel as i64 + 1
                    } else {
                        channel as i64
                    },
                    param_bucket: if granular {
                        crate::percept::feature_bucket(value, value_granularity)
                    } else {
                        0
                    },
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
    /// RESERVED. The deliberation HOP cap: the longest inferential CHAIN a being follows when it plans through
    /// its relational beliefs (relational-belief substrate, arc 2, `planning.hop_cap` wired here). A one-hop
    /// plan is a direct reward belief; a multi-hop plan chains "A yields X" backward from a goal the being
    /// cannot act on directly. This bounds the chain LENGTH (distinct from `plan_depth_cap`, which bounds how
    /// many alternative plans are returned), so a being does not chase an ever-deeper chain of ever-weaker
    /// inference. Read only where a being holds a relational belief ([`civsim_bio::agent::Mind::has_relations`]);
    /// a mind with no relation never reads it and plans one-hop byte-identically. Basis: the per-tick cognition
    /// budget and the chain depth beyond which the weakest-link confidence has decayed below the belief
    /// commit margin (a plan no surer than a guess), a performance-and-resolution bound. Surfaced with its
    /// basis, never fabricated.
    pub plan_hop_cap: usize,
    /// RESERVED. The TARGET-VALUE granularity, the quantization step that buckets a candidate's perceived
    /// affordance VALUE into a coarse kind (social-learning arc, piece 3, material granularity), the
    /// just-noticeable difference at which "a hard thing" and "a soft thing" become distinct learned targets.
    /// Read only when a world arms granular beliefs ([`crate::runner::Embodiment::set_granular_beliefs`]);
    /// unarmed, the belief keys on the primitive alone and this is never read. Basis: the sensorium per-class
    /// just-noticeable difference for the affordance scalar, the same basis `harm.feature_granularity` and
    /// `reward.feature_granularity` carry, so a discovered technique specialises over the same perceptual
    /// resolution a harm or reward belief generalises over. Surfaced with its basis, never fabricated.
    pub target_value_granularity: Fixed,
}

impl DiscoveryCalib {
    /// Read the discovery-sampler calibrations fail-loud from the manifest (Principle 11): a reserved value
    /// left unset refuses to build rather than running on a fabricated default, so a Calibrated world arms the
    /// discovery loop only once the owner has set the values it needs (that fail-loud is what maps the
    /// remaining calibration work). The surprise threshold DERIVES from the interoceptive noise floor already
    /// defined for reward (`reward.noise_floor`), the exact quantity `discovery.surprise_threshold`'s own basis
    /// names ("the interoceptive noise floor already defined for reward and harm"), so the two agree by
    /// construction rather than through a second reserved key. The plan depth and hop caps are the shared
    /// `planning.*` cognition budgets, read as counts.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<DiscoveryCalib, CalibrationError> {
        Ok(DiscoveryCalib {
            exploration_floor: m.require_fixed("discovery.exploration_floor")?,
            surprise_threshold: m.require_fixed("reward.noise_floor")?,
            surprise_gain: m.require_fixed("discovery.surprise_gain")?,
            plan_depth_cap: m.require_usize("planning.depth_cap")?,
            plan_hop_cap: m.require_usize("planning.hop_cap")?,
            target_value_granularity: m.require_fixed("discovery.target_value_granularity")?,
        })
    }

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
            plan_hop_cap: 4,
            target_value_granularity: Fixed::ONE,
        }
    }
}

/// The roulette weight of one candidate binding: the unit weight when the being holds a committed REWARDS
/// belief about the candidate's action (it believes this action pays off, so it exploits), the reserved
/// exploration floor otherwise (an untried or unrewarded action, still proposed at the floor rate so
/// discovery never stops). Reads only the being's own reward belief on the disjoint `REWARD_ATTR`, never a
/// coded preference (Principle 9).
///
/// The belief is looked up through the shared [`crate::learn::step_belief_subject`] at the caller's
/// `granular` grain, the exact key the reward learner's credit pass commits and
/// [`crate::learn::appetitive_salience`], the planner match, and the surprise read all use at the SAME grain,
/// so the belief this reads is the belief those form (no mask). By default the grain is the PRIMITIVE ALONE,
/// so a being that learned a primitive pays off proposes it against every present target rather than
/// re-learning it per type; when the world arms granular beliefs the grain includes the target's affordance
/// channel and quantized value, so "strike a hard thing" and "strike a soft thing" become distinct learned
/// actions (social-learning arc, piece 3, material granularity). The candidate always carries its affordance
/// CHANNEL in `target_bucket` and, when granular, its quantized value in `param_bucket`; the grain decides
/// which of those the belief keys on.
///
/// The `social_prior` (social-learning arc, piece 2, observe-and-imitate) LIFTS the exploration floor of an
/// UNBELIEVED candidate whose action the being just perceived a co-located neighbour enact, scaled by the
/// being's founder-zero social-learning weight, so a demonstrated technique is tried more than an unseen one
/// while a rewarded habit still exploits at full weight. It is zero for a founder (zero social weight), for
/// an unobserved action, and for every run with observe-and-imitate unarmed, so the draw is unchanged there
/// (opt-in, founder-zero). It only tips the PROPOSAL; the being's own felt reward stays the sole gate to a
/// committed belief, so a copied action never becomes a belief until eating it pays off (copy-and-verify).
fn candidate_weight(
    mind: &Mind,
    step: &SequenceStep,
    calib: &DiscoveryCalib,
    params: &InferenceParams,
    social_prior: Fixed,
    granular: bool,
) -> Fixed {
    let subject = step_belief_subject(step, granular);
    if mind.belief(subject, REWARD_ATTR, params) == Some(REWARDS) {
        Fixed::ONE
    } else {
        calib
            .exploration_floor
            .saturating_add(social_prior)
            .min(Fixed::ONE)
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
///
/// `observed` and `social_weight` carry the observe-and-imitate bias (social-learning arc, piece 2): a
/// candidate whose primitive is in `observed` (the actions co-located beings enacted last tick) has its
/// exploration floor lifted by `social_weight`, so the being tries a demonstrated technique more. An empty
/// `observed` set or a zero `social_weight` (a founder, or observe-and-imitate unarmed) leaves every weight
/// at its pre-social value, so the draw is byte-identical.
#[allow(clippy::too_many_arguments)]
pub fn sample_candidate(
    candidates: &[SequenceStep],
    mind: &Mind,
    calib: &DiscoveryCalib,
    params: &InferenceParams,
    being: StableId,
    tick: u64,
    seed: u64,
    observed: &std::collections::BTreeSet<u16>,
    social_weight: Fixed,
    granular: bool,
) -> Option<SequenceStep> {
    if candidates.is_empty() {
        return None;
    }
    let weights: Vec<Fixed> = candidates
        .iter()
        .map(|s| {
            let social_prior = if observed.contains(&s.primitive) {
                social_weight
            } else {
                Fixed::ZERO
            };
            candidate_weight(mind, s, calib, params, social_prior, granular)
        })
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
    use crate::learn::{sequence_subject, RewardLearningCalib, NEUTRAL};
    use civsim_foundation::calibration::CalibrationManifest;

    #[test]
    fn discovery_calib_reads_the_manifest_and_derives_surprise_from_reward_noise() {
        // The discovery sampler reads its calibration fail-loud from the manifest (Arc 1 gap b); the surprise
        // threshold DERIVES from reward.noise_floor (its own basis) rather than a second reserved key.
        let m = CalibrationManifest::from_toml_str(
            r#"
[[reserved]]
id = "discovery.exploration_floor"
basis = "b"
status = "set"
value = "0.25"
source = "s"
[[reserved]]
id = "reward.noise_floor"
basis = "b"
status = "set"
value = "0.01"
source = "s"
[[reserved]]
id = "discovery.surprise_gain"
basis = "b"
status = "set"
value = "1"
source = "s"
[[reserved]]
id = "discovery.target_value_granularity"
basis = "b"
status = "set"
value = "1"
source = "s"
[[reserved]]
id = "planning.depth_cap"
basis = "b"
status = "set"
value = "8"
source = "s"
[[reserved]]
id = "planning.hop_cap"
basis = "b"
status = "set"
value = "4"
source = "s"
"#,
        )
        .unwrap();
        let c = DiscoveryCalib::from_manifest(&m).unwrap();
        assert_eq!(c.exploration_floor, Fixed::from_ratio(1, 4));
        assert_eq!(
            c.surprise_threshold,
            m.require_fixed("reward.noise_floor").unwrap(),
            "surprise threshold derives from the reward noise floor"
        );
        assert_eq!(c.surprise_gain, Fixed::ONE);
        assert_eq!(c.plan_depth_cap, 8);
        assert_eq!(c.plan_hop_cap, 4);
        assert_eq!(c.target_value_granularity, Fixed::ONE);

        // Fail-loud: without the reward noise floor the surprise threshold cannot derive, so the build refuses
        // rather than fabricating a default.
        let missing = CalibrationManifest::from_toml_str(
            "[[reserved]]\nid = \"discovery.exploration_floor\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"0.25\"\nsource = \"s\"\n",
        )
        .unwrap();
        assert!(DiscoveryCalib::from_manifest(&missing).is_err());
    }

    fn params() -> InferenceParams {
        InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    const SEED: u64 = 0x00D1_5C05;

    // No observed actions and a zero social-learning weight: the pre-observe-and-imitate draw, unchanged, so
    // these tests read the sampler's belief-and-floor behaviour with the social bias inert.
    fn no_obs() -> std::collections::BTreeSet<u16> {
        std::collections::BTreeSet::new()
    }

    // Candidate bindings at the pre-granularity grain (primitive-only beliefs), so these tests read the
    // binding-graph and sampling behaviour with piece 3's target-value keying inert (byte-identical).
    fn bind(afforded: &[AffordanceId], percepts: &[Fixed]) -> Vec<SequenceStep> {
        candidate_bindings(afforded, percepts, false, Fixed::ONE)
    }

    #[test]
    fn granular_beliefs_split_a_hard_target_from_a_soft_one_and_the_default_keeps_them_one() {
        // Social-learning arc, piece 3 (material granularity): with granular beliefs the SAME primitive against
        // a HARD target and a SOFT target of the same affordance channel mints DISTINCT belief subjects, so
        // "strike a hard thing" and "strike a soft thing" are learned separately; the default (primitive-only)
        // mints the SAME subject for both, the byte-identical generalisation that keeps one flat "the primitive
        // pays off." A value-blind candidate (granular off) carries a zero value, so its subject equals the
        // primitive-only key regardless of the target's value.
        let gran = Fixed::ONE;
        // One affordance channel, two target values: a hard target (value 5) and a soft one (value 1), which
        // bucket to distinct kinds at a unit just-noticeable difference.
        let hard = candidate_bindings(&[STRIKE], &[Fixed::from_int(5)], true, gran);
        let soft = candidate_bindings(&[STRIKE], &[Fixed::ONE], true, gran);
        assert_ne!(
            step_belief_subject(&hard[0], true),
            step_belief_subject(&soft[0], true),
            "granular beliefs split a hard target from a soft one of the same channel"
        );
        assert_eq!(
            step_belief_subject(&hard[0], false),
            step_belief_subject(&soft[0], false),
            "the default keeps a hard and a soft target one belief (the generalising key)"
        );
        // A value-blind (granular-off) candidate carries a zero value, so its granular-off subject is the
        // primitive-only key, identical to the soft target's default subject.
        let flat = candidate_bindings(&[STRIKE], &[Fixed::from_int(5)], false, gran);
        assert_eq!(
            flat[0].param_bucket, 0,
            "a value-blind candidate carries no value"
        );
        assert_eq!(
            step_belief_subject(&flat[0], false),
            step_belief_subject(&soft[0], false),
            "a value-blind candidate keys the primitive-only subject regardless of the target value"
        );
    }

    #[test]
    fn the_binding_graph_is_the_generic_cartesian_of_afforded_primitives_and_present_targets() {
        // Slice 2b: every afforded primitive is proposed against every PRESENT affordance channel, with no
        // coded pairing. A being that affords GRASP, STRIKE, and EXTRACT and perceives two present
        // affordances (channel 0 fracturable, channel 1 sharp) proposes all six primitive-times-channel
        // bindings, each keyed on the primitive and the affordance CHANNEL as its target.
        let afforded = [GRASP, STRIKE, EXTRACT];
        let percepts = [Fixed::from_ratio(8, 10), Fixed::from_ratio(9, 10)]; // both present
        let candidates = bind(&afforded, &percepts);
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
        let strong = bind(&[STRIKE], &[Fixed::from_ratio(9, 10)]);
        let weak = bind(&[STRIKE], &[Fixed::from_ratio(2, 10)]);
        assert_eq!(
            sequence_subject(&[strong[0]]),
            sequence_subject(&[weak[0]]),
            "the same primitive on the same affordance channel is one template, value-blind"
        );
        // A different channel is a different template.
        let sharp = bind(&[STRIKE], &[Fixed::ZERO, Fixed::from_ratio(9, 10)]);
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
        let candidates = bind(&[STRIKE, GRASP], &[Fixed::ZERO, Fixed::from_ratio(9, 10)]);
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
        assert!(bind(&[], &[Fixed::ONE]).is_empty());
        assert!(bind(&[STRIKE], &[]).is_empty());
        assert!(bind(&[STRIKE], &[Fixed::ZERO, Fixed::ZERO]).is_empty());
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
        // Slice 2b: the sampler prefers the PRIMITIVE the being believes pays off, but the reserved
        // exploration floor keeps an unproven action in play. A being that has committed the "STRIKE pays
        // off" belief proposes a STRIKE every tick when the floor is zero (pure exploit), while a naive being
        // with a positive floor still proposes SOMETHING (it explores). The belief generalises over the
        // primitive (keyed on `target_bucket` zero, the same key the reward learner commits), so it lifts a
        // STRIKE against EITHER present target, rather than the channel the belief was first committed on.
        let candidates = bind(&[STRIKE, GRASP], &[Fixed::ONE, Fixed::ONE]); // 4 candidates
        let believed = candidates[0]; // STRIKE against channel 0
        let mut sage = Mind::new(StableId(1), Fixed::ONE);
        believe(&mut sage, &believed);

        // Zero floor: only the STRIKE candidates (against either present channel) carry weight, so a STRIKE
        // is proposed every tick, by belief alone, never a coded preference. The GRASP candidates, unbelieved,
        // carry no weight and are never proposed.
        let exploit = DiscoveryCalib {
            exploration_floor: Fixed::ZERO,
            ..DiscoveryCalib::dev_default()
        };
        for tick in 0..8 {
            let proposal = sample_candidate(
                &candidates,
                &sage,
                &exploit,
                &params(),
                StableId(1),
                tick,
                SEED,
                &no_obs(),
                Fixed::ZERO,
                false,
            );
            assert_eq!(
                proposal.map(|p| p.primitive),
                Some(STRIKE.0),
                "with no exploration floor, the being proposes only the primitive it believes pays off"
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
                SEED,
                &no_obs(),
                Fixed::ZERO,
                false,
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
                SEED,
                &no_obs(),
                Fixed::ZERO,
                false,
            )
            .is_some(),
            "with an exploration floor, an unproven action is still tried"
        );
    }

    #[test]
    fn an_observed_action_is_proposed_by_the_social_prior_and_a_founder_ignores_it() {
        // Social-learning arc, piece 2 (observe-and-imitate): the social prior LIFTS an UNBELIEVED candidate
        // whose action the being observed a co-located neighbour enact, scaled by its heritable
        // social-learning weight. With a zero exploration floor an observed action carries weight ONLY
        // through the social prior, so the effect is clean to read: a being with a positive social weight
        // proposes the observed STRIKE every tick (it copies what it saw), a founder (zero social weight)
        // ignores the observation and, with a zero floor, proposes nothing, so imitation emerges only with
        // the heritable weight (founder-zero); and an action the being did NOT observe gets no lift, so only
        // a demonstrated action is copied.
        use std::collections::BTreeSet;
        let candidates = bind(&[STRIKE, GRASP], &[Fixed::ONE, Fixed::ONE]); // 4
        let naive = Mind::new(StableId(3), Fixed::ONE);
        let exploit = DiscoveryCalib {
            exploration_floor: Fixed::ZERO,
            ..DiscoveryCalib::dev_default()
        };
        let observed_strike: BTreeSet<u16> = [STRIKE.0].into_iter().collect();
        // A positive social weight: the observed STRIKE carries weight (floor zero plus the social prior),
        // so the being proposes it every tick, by imitation alone, never a coded preference.
        for tick in 0..8 {
            let proposal = sample_candidate(
                &candidates,
                &naive,
                &exploit,
                &params(),
                StableId(3),
                tick,
                SEED,
                &observed_strike,
                Fixed::ONE,
                false,
            );
            assert_eq!(
                proposal.map(|p| p.primitive),
                Some(STRIKE.0),
                "a being with a social weight copies the action it observed a neighbour enact"
            );
        }
        // Founder-zero: a zero social weight ignores what it observed, and with a zero floor nothing carries
        // weight, so imitation appears only once selection lifts the social-learning weight off zero.
        assert_eq!(
            sample_candidate(
                &candidates,
                &naive,
                &exploit,
                &params(),
                StableId(3),
                0,
                SEED,
                &observed_strike,
                Fixed::ZERO,
                false,
            ),
            None,
            "a founder ignores what it observed: imitation emerges only with the heritable weight"
        );
        // Nothing observed: the social weight lifts nothing, so only a demonstrated action is ever copied.
        let observed_none: BTreeSet<u16> = BTreeSet::new();
        assert_eq!(
            sample_candidate(
                &candidates,
                &naive,
                &exploit,
                &params(),
                StableId(3),
                0,
                SEED,
                &observed_none,
                Fixed::ONE,
                false,
            ),
            None,
            "an action the being did not observe gets no social lift"
        );
    }

    #[test]
    fn the_proposal_is_deterministic_and_counter_keyed() {
        // The draw is a reproducible function of the being, the tick, and the seed (counter-keyed under the
        // hypothesis phase), so a replayed run proposes the identical hypothesis, and an empty candidate set
        // proposes nothing.
        let candidates = bind(&[STRIKE, GRASP, EXTRACT], &[Fixed::ONE, Fixed::ONE]); // 6
        let naive = Mind::new(StableId(7), Fixed::ONE);
        let calib = DiscoveryCalib::dev_default();
        let a = sample_candidate(
            &candidates,
            &naive,
            &calib,
            &params(),
            StableId(7),
            3,
            SEED,
            &no_obs(),
            Fixed::ZERO,
            false,
        );
        let b = sample_candidate(
            &candidates,
            &naive,
            &calib,
            &params(),
            StableId(7),
            3,
            SEED,
            &no_obs(),
            Fixed::ZERO,
            false,
        );
        assert_eq!(
            a, b,
            "the proposal is reproducible for one being, tick, and seed"
        );
        assert!(a.is_some());
        assert_eq!(
            sample_candidate(
                &[],
                &naive,
                &calib,
                &params(),
                StableId(7),
                0,
                SEED,
                &no_obs(),
                Fixed::ZERO,
                false,
            ),
            None,
            "no candidates, no proposal"
        );
    }
}
