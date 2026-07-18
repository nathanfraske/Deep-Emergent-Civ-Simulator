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
//! [`civsim_bio::agent::Mind::frames`] and each frame's [`civsim_bio::evidence::InferenceFrame::clamped_total`], `commit`, and
//! `support`, never an affordance's authored valence, a race id, or a goal-to-action table.
//!
//! MULTI-HOP through the RELATIONAL substrate (relational-belief substrate, arc 2). The one-hop primitive,
//! [`plan_toward`], ranks the being's direct goal-matching PROPERTY beliefs (a value on a `(subject, attr)`:
//! "action A pays off"). On its own that is a one-hop plan, because a property belief's value is a `ValueId`
//! category that can never BE another belief's subject, so there is no subject-to-subject edge in the property
//! store. The RELATIONAL store ([`civsim_bio::agent::Mind::relations_into`]) supplies that edge: a belief `(head,
//! relation, tail)` the being holds RELATES ("A yields X", "A causes B"). [`plan_chain`] walks it: it seeds
//! from the one-hop frontier, then traverses relation edges BACKWARD from each goal subject, prepending the
//! antecedent action that brings the goal about, up to a reserved hop cap. So a being that believes "cutting
//! with a sharp thing pays off" and "striking the stone yields a sharp thing" chains them to strike first, a
//! goal a single hop cannot reach: insight as a traversed path, the seed of tool-reasoning. The traversal
//! reads only the being's OWN relational beliefs and their commit margin, never a goal-to-action table or a
//! race id (Principles 8, 9), and a being that holds no relation degenerates to the one-hop lookup exactly.
//!
//! This is a pure, RNG-free read off the run path, folding nothing into `state_hash`: a being with no
//! relation plans byte-identically to the one-hop planner, so every existing scenario replays bit-for-bit.
//! The run path (slice 4b, WIRE) reads a plan through the grounding gate on consumption: a recalled chain
//! becomes executable only where the being's own senses currently afford and perceive its FIRST action (the
//! chain's deepest antecedent), and only through a heritable action-bias weight lifted off founder-zero by
//! selection, so a plan with no matching percept is inert.

use civsim_core::{Fixed, StableId};

use std::collections::BTreeSet;

use crate::learn::RELATES;
use civsim_bio::agent::Mind;
use civsim_bio::evidence::{AttrKindId, InferenceParams, ValueId};

/// One belief on a plan path: a committed belief `(subject, attr) -> value` the being holds, with the
/// confidence and support that ranked it. A GOAL step is a property belief matching the goal predicate
/// (`(subject, goal_attr) -> goal_value`); an ANTECEDENT step is a relational belief the being holds RELATES
/// (`(subject, relation, tail) -> RELATES`, its `attr` the relation kind, its `value` [`crate::learn::RELATES`]),
/// the action that brings the next step's subject about. A one-hop plan is one goal step; a multi-hop plan is
/// an ordered run of these, each step's subject reached from the last through a relational edge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlanStep {
    /// The subject the belief is about (for a reward or antecedent step, the action's sequence subject).
    pub subject: StableId,
    /// The attribute questioned (for the arc's reward goal, `REWARD_ATTR`; for an antecedent, the relation kind).
    pub attr: AttrKindId,
    /// The value the being committed to (for the reward goal, `REWARDS`; for an antecedent, `RELATES`).
    pub value: ValueId,
    /// How decisively the being holds the belief: the commit margin (the committed value's clamped log-odds
    /// total less the strongest rival's), the same quantity `commit` thresholds on. Higher is surer.
    pub confidence: Fixed,
    /// How much evidence backs the belief (the frame's support count). A tie-break under confidence, so a
    /// belief a being has seen borne out many times outranks an equally-confident one seen once.
    pub support: usize,
}

/// A chain of belief steps from an actionable antecedent to a goal belief: the ordered inferential path a
/// being would follow to reach a goal, each step reached from the last through a committed relational belief
/// (relational-belief substrate, arc 2). A one-hop plan (the being directly believes an action reaches the
/// goal) is a chain of length one. A multi-hop plan chains a produced belief backward through the being's own
/// `(head, relation, tail)` RELATES edges: to reach a goal it cannot act on directly, do the antecedent that
/// yields it first, the seed of tool-reasoning.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Plan {
    /// The ordered steps, GOAL-LAST: `steps[len - 1]` is the goal belief `(subject, goal_attr) -> goal_value`,
    /// and each earlier step is an antecedent the being believes yields the next (`(this.subject, relation,
    /// next.subject)` commits RELATES). `steps[0]` is the ACTION to enact now, the chain's deepest antecedent.
    /// Never empty by construction.
    pub steps: Vec<PlanStep>,
    /// The chain's confidence: the WEAKEST link along it (the minimum step and edge confidence), so a plan is
    /// only as trustworthy as its least-certain inference. A one-hop chain's confidence is its single step's.
    /// A longer chain is not further discounted beyond this and the shorter-first tie-break; a graded per-hop
    /// decay is a reserved follow-on if calibration shows over-eager deep chaining.
    pub confidence: Fixed,
}

impl Plan {
    /// The step to enact NOW: the chain's deepest antecedent (`steps[0]`), the action whose enactment starts
    /// the chain toward the goal. For a one-hop plan this is the goal belief itself. The run path matches this
    /// step's subject against the being's present candidates (the grounding gate), so a chain becomes actable
    /// only where its first action is afforded and perceived now.
    pub fn action_step(&self) -> PlanStep {
        self.steps[0]
    }
}

/// The commit margin of a frame's committed value: its clamped log-odds total less the strongest rival's
/// (or the total itself for a single-hypothesis frame). The same decisiveness `InferenceFrame::commit`
/// gates on, exposed as a graded confidence for ranking. Pure and RNG-free.
fn commit_margin(
    frame: &civsim_bio::evidence::InferenceFrame,
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

/// Plan toward a goal through the being's RELATIONAL beliefs: the ordered chains of belief that reach the goal
/// predicate `(goal_attr, goal_value)`, ranked most-confident first, truncated to `depth_cap` alternatives and
/// each bounded to `hop_cap` STEPS (relational-belief substrate, arc 2). Seeds from the one-hop frontier
/// ([`plan_toward`]), then for each goal step traverses the being's `(head, relation, tail)` RELATES edges
/// BACKWARD from the step's subject, prepending the antecedent action that brings it about, so a goal the being
/// cannot act on directly is reached by doing an antecedent first (insight as a traversed path). A being that
/// holds NO relational belief returns exactly the one-hop frontier as length-one chains, in the same order, so
/// the run path that consumes each chain's [`Plan::action_step`] behaves byte-identically to the one-hop
/// planner (the substrate is opt-in by being USED, never by a flag). Reads only the being's own beliefs and
/// their commit margin, never a goal-to-action table or a race id (Principles 8, 9). Pure and RNG-free.
///
/// `hop_cap` bounds the number of STEPS in a plan (the goal step plus its antecedents), so a plan of `k` steps
/// chains `k - 1` edges; the longest plan produced has exactly `hop_cap` steps.
///
/// `reachable` is the DATA-defined set of relation kinds the planner may traverse as means-ends edges
/// ([`crate::learn::builtin_reachable_relations`]): a relation whose kind is NOT in the set is inert to the
/// planner (default-off), so the planner authors no universal "every relation is causal" reading. A being may
/// still hold and hash such a relation; the planner just never plans through it until the kind is declared
/// causal in the data.
///
/// FLAGGED BOUND (deep audit, arc 2): the backward search enumerates every distinct simple path to a goal
/// node, with no cross-chain dedup, so a DENSE relation graph costs on the order of `b^(hop_cap-1)` chains per
/// goal node (branching factor `b`) held before the final truncate. It TERMINATES (the per-chain cycle guard
/// plus `hop_cap` bound each path's length) and stays deterministic, and it is inert on the current run path
/// (a being forms no relation yet, so the search never runs), but a dense graph under arc 3's relation
/// formation would make it expensive. The named remediation, to take before arc 3 forms dense relation graphs:
/// a best-first (widest-path) traversal with a per-node best-confidence memo, bounded by edges rather than
/// paths, which the consumer's needs permit since it reads only each plan's [`Plan::action_step`].
pub fn plan_chain(
    mind: &Mind,
    goal_attr: AttrKindId,
    goal_value: ValueId,
    params: &InferenceParams,
    depth_cap: usize,
    hop_cap: usize,
    reachable: &BTreeSet<AttrKindId>,
) -> Vec<Plan> {
    // The one-hop frontier: the being's direct goal-matching beliefs, already ranked and truncated to the
    // depth cap. Each is the GOAL step (the last step) of a chain.
    let frontier = plan_toward(mind, goal_attr, goal_value, params, depth_cap);
    // Fast path and byte-neutral guarantee: a being with no relational belief plans one-hop. Every plan is a
    // length-one chain in the frontier's order, so a consumer sees the identical action steps in the identical
    // order it saw before the relational substrate existed.
    if !mind.has_relations() {
        return frontier
            .into_iter()
            .map(|s| Plan {
                steps: vec![s],
                confidence: s.confidence,
            })
            .collect();
    }
    // Backward expansion: from each goal step, walk relation edges into its subject, prepending antecedents,
    // to the hop cap. Every chain generated (the seed and each extension) is a candidate plan; the final sort
    // imposes the deterministic total order, so the expansion order does not matter.
    let mut plans: Vec<Plan> = Vec::new();
    for goal_step in &frontier {
        let seed = Plan {
            steps: vec![*goal_step],
            confidence: goal_step.confidence,
        };
        // A working set of chains to extend. `pop` order is irrelevant: all reachable chains are collected and
        // ranked at the end, so the result is a pure function of the belief store.
        let mut open = vec![seed.clone()];
        plans.push(seed);
        while let Some(chain) = open.pop() {
            if chain.steps.len() >= hop_cap {
                continue;
            }
            // Extend backward from the chain's current deepest antecedent: an edge (A, relation, head) the
            // being believes RELATES is an action A that brings `head` about.
            let head = chain.steps[0].subject;
            for (a, relation, frame) in mind.relations_into(head) {
                // The relation kind must be a declared REACHABILITY edge (data, default-off): a non-causal
                // relation the being holds (a similarity, say) is inert to the planner, never read as a means
                // to an end. The planner authors no universal means-ends reading over all relations.
                if !reachable.contains(&relation) {
                    continue;
                }
                if frame.commit(params) != Some(RELATES) {
                    continue;
                }
                // No cycles: an action already in the chain is not prepended again, so the traversal terminates
                // and a chain never counts one action twice.
                if chain.steps.iter().any(|s| s.subject == a) {
                    continue;
                }
                let edge_conf = commit_margin(frame, RELATES, params);
                let antecedent = PlanStep {
                    subject: a,
                    attr: relation,
                    value: RELATES,
                    confidence: edge_conf,
                    support: frame.support().len(),
                };
                let mut steps = Vec::with_capacity(chain.steps.len() + 1);
                steps.push(antecedent);
                steps.extend_from_slice(&chain.steps);
                // Weakest-link: the chain is only as sure as its least-certain inference.
                let confidence = chain.confidence.min(edge_conf);
                let extended = Plan { steps, confidence };
                open.push(extended.clone());
                plans.push(extended);
            }
        }
    }
    // Rank most-confident first (weakest-link), then SHORTER chain first (a direct plan beats a chain of equal
    // confidence), then best-supported action step, then a canonical walk of the whole step vector, a strict
    // deterministic total order with no RNG and no domain preference.
    plans.sort_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then_with(|| a.steps.len().cmp(&b.steps.len()))
            .then_with(|| b.action_step().support.cmp(&a.action_step().support))
            .then_with(|| chain_order(&a.steps, &b.steps))
    });
    plans.truncate(depth_cap);
    plans
}

/// A canonical total order over two step chains: compare step by step on `(subject, attr, value)`, then by
/// length, so the plan ranking's final tie-break is a pure function of the chains with no RNG.
fn chain_order(a: &[PlanStep], b: &[PlanStep]) -> std::cmp::Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        let ord = x
            .subject
            .0
            .cmp(&y.subject.0)
            .then_with(|| x.attr.0.cmp(&y.attr.0))
            .then_with(|| x.value.cmp(&y.value));
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    a.len().cmp(&b.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learn::{
        builtin_reachable_relations, sequence_subject, SequenceStep, NEUTRAL, RELATES, REWARDS,
        REWARD_ATTR, UNRELATED, YIELDS,
    };

    // The data-defined causal relation set the run path uses (YIELDS is the one built-in causal kind).
    fn reachable() -> BTreeSet<AttrKindId> {
        builtin_reachable_relations()
    }

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

    // Commit a RELATES (or UNRELATED) edge "head yields tail" with `reps` reinforcing observations, the
    // same associative engine the reward belief uses, so a relational belief is inferred, never authored.
    fn commit_relation(
        mind: &mut Mind,
        head: StableId,
        tail: StableId,
        toward: ValueId,
        reps: usize,
    ) {
        for _ in 0..reps {
            mind.consider_relation(
                head,
                YIELDS,
                tail,
                [RELATES, UNRELATED],
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

    // --- relational-belief substrate, arc 2: the multi-hop planner ---

    #[test]
    fn a_being_chains_two_beliefs_to_reach_a_goal_a_single_hop_cannot() {
        let p = params();
        // The tool-reasoning scenario. A being believes CUTTING pays off (the goal belief), but cutting is not
        // an action it can do underfoot right now. It also believes STRIKING yields (the conditions for)
        // cutting: a relational belief, learned the same way. It holds NO direct reward belief about striking.
        let cut = subject_for(9);
        let strike = subject_for(3);
        let mut mind = Mind::new(StableId(10), Fixed::ONE);
        commit(&mut mind, cut, REWARDS, 12); // "cutting pays off"
        commit_relation(&mut mind, strike, cut, RELATES, 12); // "striking yields cutting"

        // The being's present binding graph (the grounding gate): only STRIKING is afforded here, not cutting.
        let present = [strike];

        // ONE-HOP: the being believes cutting pays off, but cutting is not present, and it has no reward belief
        // about striking, so a single-hop planner grounds NOTHING it can do now. A goal out of direct reach.
        let one_hop = plan_toward(&mind, REWARD_ATTR, REWARDS, &p, 8);
        assert!(
            one_hop.iter().all(|s| s.subject == cut),
            "the one-hop planner knows only the direct reward belief (cutting)"
        );
        assert!(
            !one_hop.iter().any(|s| present.contains(&s.subject)),
            "a single hop grounds no present action: the goal is out of direct reach"
        );

        // MULTI-HOP: the planner chains the relation backward from the goal, surfacing STRIKING as the
        // antecedent to do now. The being reaches a goal one hop could not, the seed of tool-reasoning.
        let plans = plan_chain(&mind, REWARD_ATTR, REWARDS, &p, 8, 4, &reachable());
        let grounded = plans
            .iter()
            .find(|plan| present.contains(&plan.action_step().subject))
            .expect(
                "the multi-hop planner grounds an actionable antecedent the one-hop planner cannot",
            );
        assert_eq!(
            grounded.action_step().subject,
            strike,
            "the grounded action is STRIKING, the antecedent the relation surfaced"
        );
        assert_eq!(
            grounded.steps.len(),
            2,
            "the grounded plan is a two-hop chain"
        );
        assert_eq!(
            grounded.steps.last().unwrap().subject,
            cut,
            "the chain's goal step is the reward belief it was chained toward"
        );
        assert_eq!(
            grounded.steps.last().unwrap().value,
            REWARDS,
            "the chain terminates at a REWARDS belief (the goal predicate)"
        );
        assert_eq!(
            grounded.steps[0].value, RELATES,
            "the antecedent step is a RELATES relational belief"
        );
    }

    #[test]
    fn removing_the_relation_removes_the_multi_hop_capability() {
        let p = params();
        // The falsifier control: the SAME reward belief, but WITHOUT the relation. The chain must vanish, so we
        // know the multi-hop plan came from the being's relational belief and nowhere else.
        let cut = subject_for(9);
        let strike = subject_for(3);
        let present = [strike];

        let mut without = Mind::new(StableId(11), Fixed::ONE);
        commit(&mut without, cut, REWARDS, 12); // believes cutting pays off, holds NO relation
        assert!(!without.has_relations());
        let plans = plan_chain(&without, REWARD_ATTR, REWARDS, &p, 8, 4, &reachable());
        assert!(
            plans.iter().all(|plan| plan.steps.len() == 1),
            "with no relation every plan is a length-one chain (no chaining)"
        );
        assert!(
            !plans
                .iter()
                .any(|plan| present.contains(&plan.action_step().subject)),
            "with the relation removed, striking is never surfaced: the capability is the relation's"
        );
    }

    #[test]
    fn with_no_relation_plan_chain_is_byte_identical_to_the_one_hop_planner() {
        let p = params();
        // Byte-neutrality at the unit level: a being with no relational belief must plan through plan_chain
        // exactly as through plan_toward, so the run path (which consumes each plan's action step) behaves
        // identically to before the relational substrate existed.
        let mut mind = Mind::new(StableId(12), Fixed::ONE);
        commit(&mut mind, subject_for(3), REWARDS, 16);
        commit(&mut mind, subject_for(4), REWARDS, 6);
        commit(&mut mind, subject_for(1), NEUTRAL, 16);
        assert!(!mind.has_relations());

        let one_hop = plan_toward(&mind, REWARD_ATTR, REWARDS, &p, 8);
        let chained = plan_chain(&mind, REWARD_ATTR, REWARDS, &p, 8, 4, &reachable());
        assert_eq!(
            one_hop.len(),
            chained.len(),
            "the two planners return the same number of plans with no relation"
        );
        for (step, plan) in one_hop.iter().zip(chained.iter()) {
            assert_eq!(plan.steps.len(), 1, "each plan is a length-one chain");
            assert_eq!(
                plan.action_step(),
                *step,
                "each chain's action step is the one-hop plan step, in the same order"
            );
        }
    }

    #[test]
    fn the_hop_cap_bounds_the_chain_length() {
        let p = params();
        // A three-link chain: A yields B, B yields C, and C pays off. With a generous hop cap the being can
        // chain all the way back to A; with a tight cap it can only reach one hop back.
        let a = subject_for(3);
        let b = subject_for(5);
        let c = subject_for(9);
        let mut mind = Mind::new(StableId(13), Fixed::ONE);
        commit(&mut mind, c, REWARDS, 12); // "C pays off"
        commit_relation(&mut mind, b, c, RELATES, 12); // "B yields C"
        commit_relation(&mut mind, a, b, RELATES, 12); // "A yields B"

        // Hop cap 3: the deepest chain reaches A (three steps: A -> B -> C).
        let deep = plan_chain(&mind, REWARD_ATTR, REWARDS, &p, 8, 3, &reachable());
        let longest = deep.iter().map(|plan| plan.steps.len()).max().unwrap();
        assert_eq!(
            longest, 3,
            "a hop cap of 3 admits the full A -> B -> C chain"
        );
        assert!(
            deep.iter().any(|plan| plan.action_step().subject == a),
            "the deepest chain grounds on A"
        );

        // Hop cap 2: the chain is bounded at two steps, so B is the deepest antecedent reachable, never A.
        let shallow = plan_chain(&mind, REWARD_ATTR, REWARDS, &p, 8, 2, &reachable());
        let longest = shallow.iter().map(|plan| plan.steps.len()).max().unwrap();
        assert_eq!(longest, 2, "a hop cap of 2 bounds the chain to two steps");
        assert!(
            shallow.iter().all(|plan| plan.action_step().subject != a),
            "A is one hop too deep for a cap of 2 and is never grounded"
        );
    }

    #[test]
    fn a_direct_belief_outranks_a_longer_chain_of_equal_confidence() {
        let p = params();
        // Two ways to reach reward: a DIRECT belief that action D pays off, and a two-hop chain to a
        // differently-rewarding action through a relation. At equal weakest-link confidence, the shorter
        // (direct) plan ranks first, so a being prefers the sure direct action over the inferred chain.
        let direct = subject_for(2);
        let goal = subject_for(9);
        let antecedent = subject_for(3);
        let mut mind = Mind::new(StableId(14), Fixed::ONE);
        commit(&mut mind, direct, REWARDS, 12);
        commit(&mut mind, goal, REWARDS, 12);
        commit_relation(&mut mind, antecedent, goal, RELATES, 12);

        let plans = plan_chain(&mind, REWARD_ATTR, REWARDS, &p, 8, 4, &reachable());
        // The direct one-hop plans (length one) come before the two-hop chain at equal confidence.
        let first_chain = plans.iter().position(|plan| plan.steps.len() == 2).unwrap();
        assert!(
            plans[..first_chain]
                .iter()
                .all(|plan| plan.steps.len() == 1),
            "every plan ranked above the first chain is a shorter, more direct plan"
        );
    }

    #[test]
    fn the_multi_hop_plan_is_a_deterministic_total_order_and_replays_identically() {
        let p = params();
        let mut mind = Mind::new(StableId(15), Fixed::ONE);
        let goal = subject_for(9);
        commit(&mut mind, goal, REWARDS, 10);
        // Two antecedents into the same goal, equally confident, so the tie-break falls to the canonical
        // chain order, a deterministic total order with no RNG.
        commit_relation(&mut mind, subject_for(3), goal, RELATES, 10);
        commit_relation(&mut mind, subject_for(7), goal, RELATES, 10);
        let a = plan_chain(&mind, REWARD_ATTR, REWARDS, &p, 8, 4, &reachable());
        let b = plan_chain(&mind, REWARD_ATTR, REWARDS, &p, 8, 4, &reachable());
        assert_eq!(
            a, b,
            "the multi-hop plan is a pure deterministic function of the belief store"
        );
    }

    #[test]
    fn a_non_causal_relation_kind_is_inert_to_the_planner_by_default() {
        // The steering falsifier (audit hardening): the planner may traverse ONLY relation kinds declared
        // causal in the data (the reachability set). A being holds a relation of a NON-causal kind between an
        // action and the goal; the planner must NOT read it as a means to the end, so the antecedent is never
        // surfaced. This proves the planner authors no universal "every relation is causal" reading.
        let p = params();
        // A relation kind absent from the reachability set (a made-up non-causal kind, e.g. "resembles").
        let non_causal = AttrKindId(u32::MAX - 5);
        assert!(!reachable().contains(&non_causal));

        let goal = subject_for(9);
        let antecedent = subject_for(3);
        let present = [antecedent];

        let mut mind = Mind::new(StableId(16), Fixed::ONE);
        commit(&mut mind, goal, REWARDS, 12); // believes the goal action pays off, but it is not present
        for _ in 0..12 {
            // A committed edge (antecedent, non_causal, goal), value RELATES, but of a non-causal KIND.
            mind.consider_relation(
                antecedent,
                non_causal,
                goal,
                [RELATES, UNRELATED],
                RELATES,
                Fixed::ONE,
                mind.id,
            );
        }
        assert_eq!(
            mind.relation(antecedent, non_causal, goal, &p),
            Some(RELATES)
        );

        // With the non-causal kind NOT in the reachability set, the planner never chains through it: no plan
        // grounds the antecedent, exactly as if the relation did not exist for planning.
        let plans = plan_chain(&mind, REWARD_ATTR, REWARDS, &p, 8, 4, &reachable());
        assert!(
            plans.iter().all(|plan| plan.steps.len() == 1),
            "a non-causal relation is not traversed: every plan stays one-hop"
        );
        assert!(
            !plans
                .iter()
                .any(|plan| present.contains(&plan.action_step().subject)),
            "the antecedent behind a non-causal relation is never surfaced as an action"
        );

        // The control: the SAME edge under the causal YIELDS kind IS traversed, so the difference is the
        // kind's declared causality, not anything else.
        let mut causal = Mind::new(StableId(17), Fixed::ONE);
        commit(&mut causal, goal, REWARDS, 12);
        commit_relation(&mut causal, antecedent, goal, RELATES, 12); // YIELDS
        let causal_plans = plan_chain(&causal, REWARD_ATTR, REWARDS, &p, 8, 4, &reachable());
        assert!(
            causal_plans
                .iter()
                .any(|plan| present.contains(&plan.action_step().subject)),
            "under the causal YIELDS kind, the same antecedent IS surfaced"
        );
    }
}
