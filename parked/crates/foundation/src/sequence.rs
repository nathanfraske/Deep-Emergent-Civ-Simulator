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

//! The executed-primitive-SEQUENCE substrate: one step of a sequence a being enacted, and the
//! recency-decayed eligibility trace over the sequences it recently executed (the ideation and
//! experiential-discovery arc, piece 1, slice 1b).
//!
//! These two types were the whole of the `learn` and `locomotion` cycle in `civsim-sim`. `learn`
//! minted and read them, `locomotion` carried them on a walker, and each module imported the other,
//! so 3,419 lines of production code sat in one strongly connected component held by two import
//! lines. Both types are plain state over `civsim_core` primitives and reach nothing else, so
//! lifting them here cuts the cycle at its narrowest point: `learn` still reads
//! `civsim_sim::locomotion::ResourceField`, `locomotion` no longer reads `learn`, and the edge runs
//! one way.
//!
//! The functions that MINT a sequence subject (`civsim_sim::learn::sequence_subject` and
//! `civsim_sim::learn::step_belief_subject`) stayed in `learn`: they read the belief-subject band
//! and the attribute-kind ids, which belong to the learner. Only the DATA moved. Names in
//! `civsim-sim` are written here as backticked by-name prose rather than rustdoc links, because a
//! rustdoc link is a dependency in miniature and this crate sits below `civsim-sim`.

use civsim_core::{Fixed, StableId, StateHasher};
use std::collections::BTreeMap;

/// One step of an executed primitive sequence (ideation arc, piece 1, slice 1b): the PRIMITIVE the being
/// enacted (an affordance id), the quantized TARGET-AFFORDANCE bucket of the matter it acted on (the raw
/// derived affordance scalar bucketed like a feature, which slice 2a supplies), and the quantized action
/// PARAM bucket (a force or aim level bucketed the same way). All three are small quantized ids, so a step
/// is a wildcard predicate `primitive(target-kind, param-kind)` a template can match, never an object id or
/// a coded primitive pair (Principles 8, 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceStep {
    /// The affordance id of the primitive enacted (grasp, extract, ...).
    pub primitive: u16,
    /// The quantized target-affordance bucket (the kind of thing acted on).
    pub target_bucket: i64,
    /// The quantized action-parameter bucket (the kind of how: a force or aim level).
    pub param_bucket: i64,
}

/// The per-being ELIGIBILITY TRACE (ideation / experiential-discovery arc, piece 1, slice 1b): a short
/// memory of the primitive SEQUENCES a being recently executed, each with a recency-decayed eligibility in
/// `(0, 1]`, so a reserve rise felt some ticks after an action can still credit the sequence that produced
/// it (temporal-difference credit assignment). The head sequence (just executed) carries full eligibility;
/// each tick every trace decays by the reserved `civsim_sim::learn::RewardLearningCalib::eligibility_decay` (the TD lambda)
/// and a trace that underflows to zero is pruned, so the memory reaches back only as far as the lag allows.
///
/// This is new per-being DYNAMIC state, the sibling of `civsim_sim::homeostasis::ReserveMemory`: it folds into
/// `state_hash` in canonical (sequence-subject, eligibility) order, draws no randomness (a run stays
/// bit-identical across worker widths), and is EMPTY-BY-DEFAULT, so a being that has executed no sequence
/// folds nothing and a scenario that does not opt in replays bit-for-bit. Slice 1c populates it on the run
/// path and reads it to route delayed credit through the shipped `consider` path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EligibilityTrace {
    /// The recently-executed sequences keyed by their `civsim_sim::learn::sequence_subject`, each with its current
    /// eligibility factor. Canonical `BTreeMap` order, so the fold and the credit walk are reproducible.
    traces: BTreeMap<StableId, Fixed>,
}

impl EligibilityTrace {
    /// An empty trace: no sequence remembered, so nothing folds into the hash until the first record.
    pub fn new() -> EligibilityTrace {
        EligibilityTrace::default()
    }

    /// Whether no sequence is remembered (an empty trace folds nothing into the hash, the opt-out state).
    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }

    /// Record a just-executed sequence at FULL eligibility (one), the head of the trace: it earns full
    /// credit for a reserve rise felt this tick and decays from there. Re-executing a sequence refreshes it
    /// to full.
    pub fn record(&mut self, subject: StableId) {
        self.traces.insert(subject, Fixed::ONE);
    }

    /// Decay every trace by the eligibility lambda and prune those that underflow to zero, so a sequence's
    /// eligibility for delayed credit falls with the ticks since it was executed. With a lambda in `(0, 1)`
    /// each trace shrinks and eventually leaves the memory, keeping it bounded and empty-neutral. A pure
    /// deterministic fold in canonical key order.
    pub fn decay(&mut self, lambda: Fixed) {
        self.traces.retain(|_, e| {
            *e = e.checked_mul(lambda).unwrap_or(Fixed::ZERO);
            *e > Fixed::ZERO
        });
    }

    /// The current eligibility of a sequence (how much delayed credit it still earns), zero if it was not
    /// recently executed. Slice 1c scales the reward observation's weight by this.
    pub fn eligibility(&self, subject: StableId) -> Fixed {
        self.traces.get(&subject).copied().unwrap_or(Fixed::ZERO)
    }

    /// The remembered sequences with their eligibilities, in canonical order (the credit walk slice 1c runs).
    pub fn entries(&self) -> impl Iterator<Item = (&StableId, &Fixed)> {
        self.traces.iter()
    }

    /// Fold the trace into a hash in canonical (sequence-subject, eligibility) order, beside the reserve
    /// memory. An empty trace folds nothing, so an opted-out run is byte-identical. The `BTreeMap` walks in
    /// canonical key order, so the fold is reproducible and thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (subject, eligibility) in &self.traces {
            h.write_u64(subject.0);
            h.write_fixed(*eligibility);
        }
    }
}
