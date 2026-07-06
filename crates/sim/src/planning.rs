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

//! The belief-graph planner (ideation / experiential-discovery arc, piece 4, slice 4a): given a GOAL, a
//! being searches what it believes for the actions that reach it, ranked by how sure it is. This is the
//! DELIBERATIVE complement to piece 1's reactive appetitive salience: where the salience lights a
//! believed-rewarding affordance the being currently PERCEIVES underfoot, this lets a being RECALL and rank
//! what it believes pays off toward a goal independent of what is in reach, the seed of goal-directed rather
//! than stimulus-driven action.
//!
//! The search is strictly GENERIC and reads NO domain knowledge (Principle 9): it takes a goal predicate (an
//! `attr` and the `value` the being wants to hold about it) and returns the committed beliefs that match,
//! ranked by the frame's own confidence (the commit margin, how decisively the being holds the belief) and
//! its support (how much evidence backs it), bounded by a reserved planning depth cap. It reads only
//! [`crate::agent::Mind::frames`] and each frame's [`InferenceFrame::clamped_total`], `commit`, and
//! `support`, never an affordance's authored valence, a race id, or a goal-to-action table.
//!
//! SINGLE-HOP, an honest limit (grounded against the shipped store, surfaced for the gate and confirmed): the
//! belief store holds PROPERTY beliefs (a value on a `(subject, attr)`: "action A pays off", "feature F
//! harms me"), and a belief's value is a `ValueId` category that can never BE another belief's subject, so
//! there is no subject-to-subject edge and no multi-hop graph to traverse yet. This planner therefore ranks
//! the being's direct goal-matching beliefs (a one-hop plan). It is written as a goal-predicate-in,
//! confidence-ranked-path-out search so a RELATIONAL belief kind (an edge whose head can be another belief's
//! subject: "A causes B", "A yields resource X") slots in later, turning the one-hop lookup into the arc's
//! full vision (insight as an untraversed path, innovation as combining beliefs from disparate domains),
//! without a rewrite. That relational substrate is flagged as its own next-substrate, not built here.
//!
//! This slice is READ only: the planner is a pure, RNG-free function off the run path, folding nothing into
//! `state_hash`, so every existing scenario replays bit-for-bit. Slice 4b (WIRE) reads a plan through the
//! grounding gate on consumption, so a recalled or taught template becomes executable only where the being's
//! own senses bind its wildcard slots, and only through a heritable action-bias weight lifted off
//! founder-zero by selection.

use civsim_core::{Fixed, StableId};

use crate::agent::Mind;
use crate::evidence::{AttrKindId, InferenceParams, ValueId};

/// One belief on a plan path: a committed belief `(subject, attr) -> value` the being holds toward a goal,
/// with the confidence and support that ranked it. For the single-hop store a plan is one such step (the
/// direct goal-matching belief); a multi-hop plan (once a relational belief kind exists) is an ordered run
/// of these, each step's subject reached from the last.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlanStep {
    /// The subject the belief is about (for a reward template, the action's sequence subject).
    pub subject: StableId,
    /// The attribute questioned (for the arc's reward goal, `REWARD_ATTR`).
    pub attr: AttrKindId,
    /// The value the being committed to (for the reward goal, `REWARDS`).
    pub value: ValueId,
    /// How decisively the being holds the belief: the commit margin (the committed value's clamped log-odds
    /// total less the strongest rival's), the same quantity `commit` thresholds on. Higher is surer.
    pub confidence: Fixed,
    /// How much evidence backs the belief (the frame's support count). A tie-break under confidence, so a
    /// belief a being has seen borne out many times outranks an equally-confident one seen once.
    pub support: usize,
}

/// The commit margin of a frame's committed value: its clamped log-odds total less the strongest rival's
/// (or the total itself for a single-hypothesis frame). The same decisiveness `InferenceFrame::commit`
/// gates on, exposed as a graded confidence for ranking. Pure and RNG-free.
fn commit_margin(
    frame: &crate::evidence::InferenceFrame,
    committed: ValueId,
    params: &InferenceParams,
) -> Fixed {
    let lead = frame
        .clamped_total(committed, params)
        .unwrap_or(Fixed::ZERO);
    let runner = frame
        .hyps()
        .iter()
        .filter(|&&v| v != committed)
        .filter_map(|&v| frame.clamped_total(v, params))
        .fold(None, |acc: Option<Fixed>, t| {
            Some(match acc {
                Some(a) if a >= t => a,
                _ => t,
            })
        });
    match runner {
        Some(r) => lead - r,
        None => lead,
    }
}

/// Plan toward a goal: the being's committed beliefs that reach the goal predicate `(goal_attr,
/// goal_value)`, ranked most-confident first (then best-supported, then canonical subject and attr order,
/// a deterministic total order drawing no randomness), truncated to the reserved `depth_cap`. For the
/// arc's reward goal `plan_toward(mind, REWARD_ATTR, REWARDS, params, cap)` returns what the being believes
/// pays off, its best guesses ranked, so a being can pursue a goal from memory rather than only reacting to
/// what it perceives. Reads only the being's own committed beliefs and their confidence and support, never a
/// goal-to-action table or a race id (Principles 8, 9). Pure and RNG-free; a `depth_cap` of zero, or a being
/// that believes nothing toward the goal, returns an empty plan.
pub fn plan_toward(
    mind: &Mind,
    goal_attr: AttrKindId,
    goal_value: ValueId,
    params: &InferenceParams,
    depth_cap: usize,
) -> Vec<PlanStep> {
    if depth_cap == 0 {
        return Vec::new();
    }
    let mut steps: Vec<PlanStep> = mind
        .frames()
        .filter_map(|((subject, attr), frame)| {
            if attr != goal_attr {
                return None;
            }
            let committed = frame.commit(params)?;
            if committed != goal_value {
                return None;
            }
            Some(PlanStep {
                subject,
                attr,
                value: committed,
                confidence: commit_margin(frame, committed, params),
                support: frame.support().len(),
            })
        })
        .collect();
    // Rank most-confident first, then best-supported, then canonical (subject, attr) order so the plan is a
    // deterministic total order over the belief set with no RNG and no domain preference.
    steps.sort_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then_with(|| b.support.cmp(&a.support))
            .then_with(|| a.subject.0.cmp(&b.subject.0))
            .then_with(|| a.attr.0.cmp(&b.attr.0))
    });
    steps.truncate(depth_cap);
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learn::{sequence_subject, SequenceStep, NEUTRAL, REWARDS, REWARD_ATTR};

    fn params() -> InferenceParams {
        InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    fn subject_for(primitive: u16) -> StableId {
        sequence_subject(&[SequenceStep {
            primitive,
            target_bucket: 0,
            param_bucket: 0,
        }])
    }

    // Commit a REWARDS (or NEUTRAL) belief about a subject with `reps` reinforcing observations, the
    // associative engine's own commit path (more reps means a larger margin, so confidence ranks by it).
    fn commit(mind: &mut Mind, subject: StableId, toward: ValueId, reps: usize) {
        for _ in 0..reps {
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
    fn a_being_plans_toward_a_goal_by_ranking_what_it_believes_pays_off() {
        let p = params();
        // A being that believes two actions pay off, one more strongly (more reinforcement), and holds a
        // NEUTRAL belief about a third (it does not believe that one pays off).
        let strong = subject_for(3); // grasp
        let weak = subject_for(4); // extract
        let disbelieved = subject_for(1); // ingest, believed NEUTRAL
        let mut mind = Mind::new(StableId(1), Fixed::ONE);
        commit(&mut mind, strong, REWARDS, 16);
        commit(&mut mind, weak, REWARDS, 6);
        commit(&mut mind, disbelieved, NEUTRAL, 16);

        let plan = plan_toward(&mind, REWARD_ATTR, REWARDS, &p, 8);
        // The plan holds exactly the two the being believes REWARDS, not the NEUTRAL one: the search reaches
        // only beliefs that match the goal predicate.
        assert_eq!(
            plan.len(),
            2,
            "only the believed-rewarding actions are planned toward"
        );
        assert!(
            plan.iter().all(|s| s.value == REWARDS),
            "every plan step is a belief the being holds toward the goal value"
        );
        let subjects: Vec<StableId> = plan.iter().map(|s| s.subject).collect();
        assert!(subjects.contains(&strong) && subjects.contains(&weak));
        assert!(
            !subjects.contains(&disbelieved),
            "an action the being believes NEUTRAL is not a plan toward reward"
        );
        // The more strongly believed action ranks first: the plan is ordered by the being's own confidence,
        // so it pursues its surest bet before its weaker one.
        assert_eq!(
            plan[0].subject, strong,
            "the plan ranks the more strongly believed action first (by confidence)"
        );
        assert!(
            plan[0].confidence > plan[1].confidence,
            "the ranking is by the frame's commit margin"
        );
    }

    #[test]
    fn the_depth_cap_bounds_the_plan_and_a_naive_being_plans_nothing() {
        let p = params();
        let mut mind = Mind::new(StableId(2), Fixed::ONE);
        for primitive in 0..5u16 {
            commit(
                &mut mind,
                subject_for(primitive),
                REWARDS,
                8 + primitive as usize,
            );
        }
        // The reserved depth cap bounds how many beliefs the plan considers (the per-tick cognition budget),
        // keeping the most-confident within the cap.
        let capped = plan_toward(&mind, REWARD_ATTR, REWARDS, &p, 2);
        assert_eq!(capped.len(), 2, "the depth cap bounds the plan length");
        // A zero cap plans nothing (the degenerate budget).
        assert!(plan_toward(&mind, REWARD_ATTR, REWARDS, &p, 0).is_empty());

        // A being that believes nothing toward the goal plans nothing, however deep the cap: the planner
        // invents no action it has no belief for (no domain knowledge, Principle 9).
        let naive = Mind::new(StableId(3), Fixed::ONE);
        assert!(
            plan_toward(&naive, REWARD_ATTR, REWARDS, &p, 8).is_empty(),
            "a being with no reward beliefs plans nothing toward reward"
        );
    }

    #[test]
    fn the_plan_is_a_deterministic_total_order_and_replays_identically() {
        let p = params();
        let mut mind = Mind::new(StableId(4), Fixed::ONE);
        // Two equally-confident beliefs (same reps), so the tie-break falls to support then canonical
        // subject order, a deterministic total order with no RNG.
        commit(&mut mind, subject_for(3), REWARDS, 10);
        commit(&mut mind, subject_for(7), REWARDS, 10);
        let a = plan_toward(&mind, REWARD_ATTR, REWARDS, &p, 8);
        let b = plan_toward(&mind, REWARD_ATTR, REWARDS, &p, 8);
        assert_eq!(
            a, b,
            "the plan is a pure deterministic function of the belief store"
        );
        assert_eq!(a.len(), 2);
        // The equal-confidence tie breaks to canonical subject order, never a random or insertion order.
        assert!(
            a[0].subject.0 < a[1].subject.0,
            "equal-confidence steps order by canonical subject id"
        );
    }
}
