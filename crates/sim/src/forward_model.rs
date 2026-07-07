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

//! The forward model and its prediction-error read (ideation / experiential-discovery arc, piece 3, slice
//! 3a): a being PREDICTS the interoceptive outcome of a candidate action, and the prediction ERROR is the
//! signed surprise between what it expected and what it felt. When a being enacts an action it predicted
//! well, there is no surprise; when the outcome defies the prediction, the surprise is what piece 3's slice
//! 3b turns into inflated motor variance along the acted-on parameter, so exploration rises where the world
//! is least understood and falls where it is well predicted.
//!
//! The model is a light associative READ over the SAME belief substrate the reward learner (piece 1) forms
//! and the discovery sampler (piece 2) reads: a template's committed reward IS its prediction. A being that
//! holds a committed REWARDS belief about a sequence PREDICTS that enacting it pays off (a supra-recovery
//! reserve rise); one with no such belief predicts nothing beyond recovery. There is no new learned
//! magnitude and no new engine: the prediction reads [`crate::agent::Mind::belief`] on the exact
//! [`sequence_subject`] the piece-1 credit pass commits and the piece-2 sampler weights, and the felt side
//! reduces the being's own reserve delta through the shipped [`is_reward_tick`] predicate, so the two sit on
//! one normalised reward scale and their difference is the signed surprise.
//!
//! This slice is READ only: both functions are pure and RNG-free, off the run path, folding nothing into
//! `state_hash`, so every existing scenario replays bit-for-bit. Slice 3b (WIRE) reads the prediction error
//! to inflate motor variance under a new registered phase, with the surprise threshold and the variance gain
//! reserved-with-basis.
//!
//! HONEST LIMIT (surfaced for the gate, faithful to the categorical substrate): the prediction is CATEGORICAL
//! (a committed reward belief predicts reward, an uncommitted or neutral one predicts none), because the
//! belief substrate carries a committed CATEGORY and a confidence, not a learned reserve-delta magnitude, and
//! this slice fabricates no magnitude. So the surprise is `{-ONE, ZERO, +ONE}`: a well-predicted outcome
//! reads zero, an expected reward that fails reads minus one, an unexpected reward reads plus one. A graded
//! refinement (normalising the felt delta continuously against the reward scale, once a per-sequence expected
//! magnitude is learned) is a natural later step; the categorical form is the honest read of a categorical
//! belief and is enough to drive slice 3b's variance along the surprising sequence's parameter.

use civsim_core::Fixed;

use crate::agent::Mind;
use crate::evidence::InferenceParams;
use crate::homeostasis::is_reward_tick;
use crate::learn::{step_belief_subject, SequenceStep, REWARDS, REWARD_ATTR};

/// The forward model's predicted interoceptive outcome for a candidate action step, in `[0, 1]`: `Fixed::ONE`
/// when the being holds a committed REWARDS belief about the step's [`sequence_subject`] (it PREDICTS the
/// action pays off, a supra-recovery reserve rise), `Fixed::ZERO` otherwise (it predicts nothing beyond
/// recovery). This is [`crate::learn::appetitive_salience`]'s committed-belief test generalised from a
/// single-primitive subject to a full [`SequenceStep`] subject (primitive, target channel, and param
/// bucket), so the prediction reads the EXACT template belief the piece-1 credit pass commits and the piece-2
/// discovery sampler weights: the model has no state of its own, the belief IS the prediction. Reads only the
/// being's own reward beliefs, never an authored valence or a race id (Principle 8). Pure and RNG-free.
pub fn predicted_reward(
    mind: &Mind,
    step: &SequenceStep,
    granular: bool,
    params: &InferenceParams,
) -> Fixed {
    // Key at the SAME grain the credit committed the belief on (social-learning arc, piece 3): primitive-only
    // when not granular (so a proposal's non-zero target channel does not silently mint a different subject
    // than the credit's), the full step when granular. Routing through the one shared helper keeps the
    // prediction and the credit on the identical belief.
    let subject = step_belief_subject(step, granular);
    if mind.belief(subject, REWARD_ATTR, params) == Some(REWARDS) {
        Fixed::ONE
    } else {
        Fixed::ZERO
    }
}

/// The signed PREDICTION ERROR (the surprise) for an action the being just enacted: the felt reward outcome
/// minus the [`predicted_reward`], on one normalised reward scale. The felt outcome is `Fixed::ONE` when the
/// being's own reserve delta this tick counts as reward ([`is_reward_tick`] against the reward-noise floor,
/// the SAME reduction the piece-1 credit pass uses), `Fixed::ZERO` otherwise. So the error is:
///
/// - `ZERO` when the prediction matched the outcome (a well-predicted reward, or a well-predicted non-reward):
///   no surprise, and slice 3b draws near-zero extra variance.
/// - `-ONE` when the being predicted reward but felt none (an expected payoff that failed): negative surprise.
/// - `+ONE` when the being predicted none but felt reward (an unexpected payoff): positive surprise.
///
/// Slice 3b reads the MAGNITUDE of this error to widen motor variance along the acted-on sequence's parameter
/// and its SIGN is the direction the model was wrong in; a being converges its exploration down as its
/// predictions come true. Pure and RNG-free; `felt_delta` is the being's own interoceptive reserve change and
/// `reward_noise_floor` is the piece-1 reserved floor (this READ fabricates no value, it takes the floor the
/// reward learner already carries).
pub fn prediction_error(
    mind: &Mind,
    step: &SequenceStep,
    felt_delta: Fixed,
    reward_noise_floor: Fixed,
    granular: bool,
    params: &InferenceParams,
) -> Fixed {
    let felt = if is_reward_tick(felt_delta, reward_noise_floor) {
        Fixed::ONE
    } else {
        Fixed::ZERO
    };
    felt - predicted_reward(mind, step, granular, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{AttrKindId, ValueId};
    use crate::learn::{sequence_subject, NEUTRAL};
    use civsim_core::StableId;

    fn params() -> InferenceParams {
        // The evidence engine's dev thresholds: clamp 50, commit at 3 nats, margin 1 (the same fixture the
        // learn and discovery tests use).
        InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    fn step() -> SequenceStep {
        // One candidate: grasp (primitive 3) against a fracturable thing (channel 0), no graded how yet.
        SequenceStep {
            primitive: 3,
            target_bucket: 0,
            param_bucket: 0,
        }
    }

    // Drive a mind to a committed belief about a subject by feeding it repeated single-tick observations
    // toward `toward` on the reward frame, the associative engine's own commit path (the same
    // `Mind::consider` call the learn-module tests use, no injected belief).
    fn commit(mind: &mut Mind, subject: StableId, toward: ValueId) {
        for _ in 0..12 {
            mind.consider(
                subject,
                REWARD_ATTR,
                [REWARDS, NEUTRAL],
                toward,
                Fixed::ONE,
                mind.id,
            );
        }
    }

    #[test]
    fn a_being_predicts_reward_only_for_a_sequence_it_believes_pays_off() {
        let p = params();
        let s = step();
        let subject = sequence_subject(std::slice::from_ref(&s));

        // A fresh mind holds no belief, so it predicts nothing (the forward model has no state of its own).
        let mut believer = Mind::new(StableId(1), Fixed::ONE);
        assert_eq!(
            predicted_reward(&believer, &s, false, &p),
            Fixed::ZERO,
            "a being that believes nothing about the action predicts no reward"
        );

        // After the being commits a REWARDS belief about this sequence, it predicts reward for it.
        commit(&mut believer, subject, REWARDS);
        assert_eq!(
            believer.belief(subject, REWARD_ATTR, &p),
            Some(REWARDS),
            "the being committed the REWARDS belief (the associative engine's own commit)"
        );
        assert_eq!(
            predicted_reward(&believer, &s, false, &p),
            Fixed::ONE,
            "the committed reward belief IS the prediction that the action pays off"
        );

        // A committed NEUTRAL belief is not a reward prediction (the belief category is what is read).
        let mut neutral = Mind::new(StableId(2), Fixed::ONE);
        commit(&mut neutral, subject, NEUTRAL);
        assert_eq!(
            predicted_reward(&neutral, &s, false, &p),
            Fixed::ZERO,
            "a committed NEUTRAL belief predicts no reward"
        );
    }

    #[test]
    fn the_prediction_error_is_the_signed_surprise_between_expected_and_felt_reward() {
        let p = params();
        let s = step();
        let subject = sequence_subject(std::slice::from_ref(&s));
        let floor = Fixed::from_ratio(1, 100);
        let big_rise = Fixed::from_int(1); // a supra-floor reserve rise: a felt reward
        let no_rise = Fixed::ZERO; // no reserve change: no felt reward

        // A being that believes the action pays off.
        let mut believer = Mind::new(StableId(3), Fixed::ONE);
        commit(&mut believer, subject, REWARDS);
        // Predicted reward, and it came: no surprise.
        assert_eq!(
            prediction_error(&believer, &s, big_rise, floor, false, &p),
            Fixed::ZERO,
            "a well-predicted reward is no surprise"
        );
        // Predicted reward, and it FAILED to come: negative surprise (the expected payoff did not arrive).
        assert_eq!(
            prediction_error(&believer, &s, no_rise, floor, false, &p),
            -Fixed::ONE,
            "an expected reward that fails to come is a negative surprise"
        );

        // A naive being that predicts nothing.
        let naive = Mind::new(StableId(4), Fixed::ONE);
        // Predicted none, and none came: no surprise.
        assert_eq!(
            prediction_error(&naive, &s, no_rise, floor, false, &p),
            Fixed::ZERO,
            "a well-predicted non-reward is no surprise"
        );
        // Predicted none, and reward CAME: positive surprise (an unexpected payoff).
        assert_eq!(
            prediction_error(&naive, &s, big_rise, floor, false, &p),
            Fixed::ONE,
            "an unexpected reward is a positive surprise"
        );
    }

    #[test]
    fn a_sub_floor_rise_is_not_read_as_a_felt_reward_so_it_is_no_surprise() {
        // The felt side uses the SAME is_reward_tick reduction the credit pass uses, so a rise below the
        // reward-noise floor (ordinary recovery jitter) is not a felt reward and a being that predicted none
        // reads no surprise from it: the forward model does not chase noise.
        let p = params();
        let s = step();
        let floor = Fixed::from_ratio(1, 10);
        let tiny_rise = Fixed::from_ratio(1, 100); // below the floor
        let naive = Mind::new(StableId(5), Fixed::ONE);
        assert_eq!(
            prediction_error(&naive, &s, tiny_rise, floor, false, &p),
            Fixed::ZERO,
            "a sub-floor rise is not a felt reward, so predicting none of it is no surprise"
        );
        // The reward frame reads its own disjoint attr, never a harm or other kind.
        assert_eq!(REWARD_ATTR, AttrKindId(u32::MAX - 3));
    }
}
