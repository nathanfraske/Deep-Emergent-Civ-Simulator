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

//! Determinism harness for the cognition layers: the evidence engine (Part 9.10) and
//! the recursive theory-of-mind update (Part 37). Both accumulate signed evidence in
//! 128-bit log-odds with the clamp applied at read, so a committed belief is a function
//! of the evidence multiset and not of the order it arrived in (design Part 9.15, Part
//! 57 R-REDUCE-ORDER, Part 3.5 R-HARNESS-COVER). These tests pin that: the committed
//! belief and a canonical state hash over the frame are identical across reorderings
//! and across thread counts, and the theory-of-mind anti-projection guarantee holds as
//! a harness assertion (no world evidence ever moves a nested belief).

use civsim_bio::evidence::{AttrKindId, InferenceFrame, InferenceParams};
use civsim_bio::tom::{AccessChannelId, AccessWeights, EvidenceOrder, NestedFrame};
use civsim_core::{Fixed, Rng, StableId, StateHasher};

const ONE: Fixed = Fixed::ONE;
const HYPS: [u32; 3] = [10, 20, 30];

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

// A canonical hash over a first-order frame: the clamped total for each hypothesis in
// sorted order, then the committed value. A pure function of the frame's state.
fn evidence_hash(frame: &InferenceFrame, params: &InferenceParams) -> u128 {
    let mut sorted = HYPS;
    sorted.sort_unstable();
    let mut h = StateHasher::new();
    for v in sorted {
        h.write_u32(v);
        h.write_fixed(frame.clamped_total(v, params).unwrap());
    }
    h.write_u32(frame.commit(params).unwrap_or(u32::MAX));
    h.finish()
}

// The fixed evidence multiset for the order-independence tests: (toward, weight).
fn contributions() -> Vec<(u32, i32)> {
    vec![
        (10, 2),
        (20, 5),
        (30, 1),
        (20, 4),
        (10, -3),
        (30, 2),
        (20, -2),
        (10, 6),
    ]
}

#[test]
fn evidence_commit_is_order_independent() {
    let p = params();
    let from = StableId(1);

    let mut ascending = InferenceFrame::new(StableId(100), AttrKindId(0), HYPS);
    for (toward, w) in contributions() {
        ascending.add_evidence(toward, Fixed::from_int(w), ONE, from);
    }

    // Feed the same multiset reversed.
    let mut reversed = InferenceFrame::new(StableId(100), AttrKindId(0), HYPS);
    for (toward, w) in contributions().into_iter().rev() {
        reversed.add_evidence(toward, Fixed::from_int(w), ONE, from);
    }

    assert_eq!(ascending.commit(&p), reversed.commit(&p));
    assert_eq!(
        evidence_hash(&ascending, &p),
        evidence_hash(&reversed, &p),
        "the frame state hashes identically regardless of evidence order"
    );
}

#[test]
fn evidence_gather_is_thread_count_invariant() {
    // Evidence is produced in parallel keyed on canonical coordinates, then reduced.
    // The hashed frame state must not depend on the thread count.
    let p = params();
    let seed = 0x5EED_0099u64;
    let n = 90usize;

    let run = |threads: usize| -> u128 {
        let threads = threads.max(1);
        let mut draws = vec![0i32; n];
        let chunk = n.div_ceil(threads).max(1);
        std::thread::scope(|s| {
            for (c, slot) in draws.chunks_mut(chunk).enumerate() {
                let base = c * chunk;
                s.spawn(move || {
                    for (k, d) in slot.iter_mut().enumerate() {
                        let r = Rng::for_coords(seed, &[(base + k) as u64]);
                        // A signed weight in a modest range.
                        *d = (r.range_u32(0, 13) as i32) - 6;
                    }
                });
            }
        });
        let mut frame = InferenceFrame::new(StableId(100), AttrKindId(0), HYPS);
        for (k, &d) in draws.iter().enumerate() {
            frame.add_evidence(
                HYPS[k % HYPS.len()],
                Fixed::from_int(d),
                ONE,
                StableId(k as u64),
            );
        }
        evidence_hash(&frame, &p)
    };

    let one = run(1);
    assert_eq!(one, run(2), "1 vs 2 threads diverged");
    assert_eq!(one, run(4), "1 vs 4 threads diverged");
    assert_eq!(one, run(3), "1 vs 3 threads (uneven) diverged");
}

// Theory-of-mind fixtures.
const WITNESSED: AccessChannelId = AccessChannelId(1);
const TOLD: AccessChannelId = AccessChannelId(2);

fn weights() -> AccessWeights {
    AccessWeights::from_pairs([(WITNESSED, Fixed::from_int(4)), (TOLD, Fixed::from_int(3))])
}

fn nested_hash(frame: &NestedFrame, params: &InferenceParams) -> u128 {
    let mut sorted = HYPS;
    sorted.sort_unstable();
    let mut h = StateHasher::new();
    h.write_stable(frame.of());
    h.write_u32(frame.depth() as u32);
    for v in sorted {
        h.write_u32(v);
        h.write_fixed(frame.clamped_total(v, params).unwrap());
    }
    h.write_u32(frame.commit(params).unwrap_or(u32::MAX));
    h.finish()
}

#[test]
fn nested_belief_is_order_independent() {
    let p = params();
    let w = weights();
    let target = StableId(2);
    let m = StableId(1);

    // (channel, toward) access observations about the target.
    let obs = [
        (WITNESSED, 10u32),
        (TOLD, 20u32),
        (WITNESSED, 10u32),
        (TOLD, 30u32),
        (WITNESSED, 20u32),
    ];

    let mut a = NestedFrame::new(target, 1, AttrKindId(0), HYPS);
    for (ch, toward) in obs {
        a.observe_access(&w, ch, toward, ONE, m).unwrap();
    }
    let mut b = NestedFrame::new(target, 1, AttrKindId(0), HYPS);
    for (ch, toward) in obs.into_iter().rev() {
        b.observe_access(&w, ch, toward, ONE, m).unwrap();
    }

    assert_eq!(a.commit(&p), b.commit(&p));
    assert_eq!(
        nested_hash(&a, &p),
        nested_hash(&b, &p),
        "the nested belief hashes identically regardless of order"
    );
}

#[test]
fn projection_can_never_leak_into_a_nested_belief() {
    // The anti-projection guarantee as a harness assertion: offering world evidence, or
    // access evidence about a different mind, is refused AND leaves the nested belief
    // unchanged, so the modeller's own corpus can never move what it thinks the target
    // believes.
    let p = params();
    let w = weights();
    let target = StableId(2);
    let other = StableId(3);
    let m = StableId(1);

    let mut frame = NestedFrame::new(target, 1, AttrKindId(0), HYPS);
    frame.observe_access(&w, WITNESSED, 10, ONE, m).unwrap();
    let before = nested_hash(&frame, &p);

    // A flood of world evidence toward a rival hypothesis, all refused.
    for _ in 0..100 {
        assert!(frame
            .admit(EvidenceOrder::World, 20, Fixed::from_int(9), ONE, m)
            .is_err());
        assert!(frame
            .admit(
                EvidenceOrder::Access { of: other },
                20,
                Fixed::from_int(9),
                ONE,
                m
            )
            .is_err());
    }

    assert_eq!(
        nested_hash(&frame, &p),
        before,
        "refused evidence left the nested belief bit-identical"
    );
    assert_eq!(
        frame.commit(&p),
        Some(10),
        "the witnessed belief still stands"
    );
}
