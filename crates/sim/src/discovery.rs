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

use civsim_core::Fixed;

use crate::homeostasis::AffordanceId;
use crate::learn::SequenceStep;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::{EXTRACT, GRASP, STRIKE};
    use crate::learn::sequence_subject;

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
}
