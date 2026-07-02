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

//! Order behaviour of the two live reduce sites (R-REDUCE-ORDER, design Part 57), including the
//! order-dependence seam the determinism audit found in one of them.
//!
//! The determinism cluster needs every combine a parallel fold could reorder to be safe. Two live
//! sites are named: the gossip conflict apply (`world.rs`, the `apply_assertion` write pass) and
//! the typology weighted pick (`typology.rs`, the cumulative walk in `sample_profile`). Their
//! determinism guarantees differ, and the difference matters for parallelism; this harness proves
//! each rather than rewriting the code, the standing preference for the hot `world.rs` path.
//!
//! The TYPOLOGY pick is order-independent by construction: the cumulative walk is over the
//! value-id-sorted weight vector `TypologyPrior::set` guarantees, so value id is the total key
//! that recovers the canonical vector from any permutation. It is safe to fold in any order.
//!
//! The GOSSIP apply is order-independent only when competing assertions share a candidate set
//! (`hyps`): first-order evidence toward a shared candidate accumulates as a raw sum with the
//! certainty clamp applied at read (`evidence.rs`), and the mind hash reads only clamped totals
//! and the committed value. But when two assertions about the same `(subject, attr)` carry
//! DIFFERENT candidate sets, the apply is order-DEPENDENT: the frame's `hyps` is fixed
//! first-writer-wins (`agent.rs`, `or_insert_with`), `add_evidence` then drops any evidence toward
//! a value absent from that frozen set (`evidence.rs`: "Evidence toward a value not in the frame
//! is ignored"), and `state_hash` walks `frame.hyps()` in first-writer order. So the gossip apply
//! is NOT an order-independent reduce site; its tick determinism rides the canonical `CommandKey`
//! barrier (R-CMD-ORDER), which fixes the apply order single-threaded. The mismatched-candidate
//! case is not a live divergence today, but it would surface the moment the APPLY pass (rather
//! than the pure-read generation pass) is parallelised or reordered, so it is surfaced for the
//! owner: canonicalize the per-question candidate set, or hold the apply to the canonical barrier.
//! The tests below prove the shared-set order-independence and lock the mismatched-set
//! order-dependence on record.
//!
//! The shuffles here are deterministic: a fixed reversal and a hand-written permutation, never
//! `rand`, so the harness itself replays bit for bit.

use civsim_core::{canonical_sorted, Fixed, StableId};
use civsim_sim::evidence::{AttrKindId, InferenceParams, ValueId};
use civsim_sim::tom::{AccessChannelId, AccessWeights};
use civsim_sim::typology::{
    sample_profile, tilted_weights, wals_seed, HarmonyModel, TypologyParamId, TypologyPrior,
    TypologyRegistry, TypologyValueId,
};
use civsim_sim::{AccessObs, Mind};

// --- Site (a): the gossip conflict apply ---

const WITNESSED: AccessChannelId = AccessChannelId(1);
const SAID: AccessChannelId = AccessChannelId(3);
const GOSSIP_SUBJECT: StableId = StableId(99);
const GOSSIP_ATTR: AttrKindId = AttrKindId(0);
const GOSSIP_HYPS: [ValueId; 3] = [10, 20, 30];

fn gossip_params() -> InferenceParams {
    // Fixture calibration, not owner canon: the clamp at 50 lets competing considers cross the
    // certainty bound so the clamp-at-read invariance is exercised.
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

fn gossip_weights() -> AccessWeights {
    AccessWeights::from_pairs([(WITNESSED, Fixed::from_int(4)), (SAID, Fixed::from_int(2))])
}

/// Apply the gossip conflict combine (the `world.rs` `apply_assertion` non-deception branch) for
/// a set of speaker assertions to one listener, in the given arrival order: each asserting
/// speaker makes the listener model it as having said the value (the SAID channel), then the
/// listener integrates the told value at the trust-scaled told-weight. This mirrors the world's
/// write pass; the world walks the actions in ascending speaker id, and this helper lets a test
/// walk the same set in any order to prove the fold does not depend on that order.
fn apply_gossip(listener: &mut Mind, told_weight: Fixed, actions: &[(u64, ValueId, Fixed)]) {
    let weights = gossip_weights();
    for &(spk, toward, trust) in actions {
        let speaker = StableId(spk);
        let _ = listener.model(
            &weights,
            speaker,
            GOSSIP_ATTR,
            GOSSIP_HYPS,
            AccessObs {
                channel: SAID,
                toward,
                from: speaker,
            },
        );
        let w = told_weight.mul(trust);
        listener.consider(GOSSIP_SUBJECT, GOSSIP_ATTR, GOSSIP_HYPS, toward, w, speaker);
    }
}

#[test]
fn gossip_apply_is_order_independent_for_a_shared_candidate_set() {
    // R-REDUCE-ORDER site (a), the order-INDEPENDENT sub-case: competing assertions that share a
    // candidate set (every action carries GOSSIP_HYPS). Several speakers assert competing values
    // about one question to a shared listener in a single tick; the same assertion set applied in a canonical (id-sorted) order, reversed,
    // and a fixed scramble must drive the listener to a bit-identical state hash. The certainty
    // clamp bites here: three considers toward one value total sixty, past the clamp of fifty, so
    // a per-step clamp (rather than clamp-at-read) would leak arrival order and fail this test.
    let told_weight = Fixed::from_int(20);
    // Distinct speaker ids (the total key), mixed values, varied trust.
    let canonical: Vec<(u64, ValueId, Fixed)> = vec![
        (10, 10, Fixed::ONE),
        (20, 20, Fixed::from_ratio(1, 2)),
        (30, 10, Fixed::ONE),
        (40, 30, Fixed::ONE),
        (50, 10, Fixed::from_ratio(3, 4)),
        (60, 20, Fixed::ONE),
    ];
    let mut reversed = canonical.clone();
    reversed.reverse();
    // A fixed scramble (a hand-written permutation, never rand).
    let scramble = vec![
        canonical[3],
        canonical[0],
        canonical[5],
        canonical[1],
        canonical[4],
        canonical[2],
    ];

    let p = gossip_params();
    let hash = |order: &[(u64, ValueId, Fixed)]| {
        let mut m = Mind::new(StableId(1), Fixed::ONE);
        apply_gossip(&mut m, told_weight, order);
        m.state_hash(&p, &p)
    };
    let h_canon = hash(&canonical);
    assert_eq!(
        h_canon,
        hash(&reversed),
        "the gossip fold leaked arrival order under reversal"
    );
    assert_eq!(
        h_canon,
        hash(&scramble),
        "the gossip fold leaked arrival order under a scramble"
    );

    // The observable belief is the same set-function of the assertions: toward 10 totals 55
    // (clamped to 50) against 30 for 20 and 20 for 30, so 10 commits regardless of order.
    let mut m = Mind::new(StableId(1), Fixed::ONE);
    apply_gossip(&mut m, told_weight, &scramble);
    assert_eq!(
        m.belief(GOSSIP_SUBJECT, GOSSIP_ATTR, &p),
        Some(10),
        "the committed belief is a function of the assertion set, not its order"
    );
}

#[test]
fn gossip_apply_is_order_dependent_when_candidate_sets_differ() {
    // The seam the determinism audit found, locked here so it cannot be silently reclaimed as
    // order-independent. When two assertions about the same (subject, attr) carry DIFFERENT
    // candidate sets, the frame's hyps is fixed first-writer-wins and add_evidence drops evidence
    // toward a value absent from it (agent.rs, evidence.rs), so the committed belief depends on
    // which assertion applies first. The gossip apply is therefore NOT an order-independent reduce
    // site; it is deterministic in the live tick only because the CommandKey barrier fixes the
    // apply order (R-CMD-ORDER). Surfaced for the owner as the thing to close (canonicalize the
    // per-question candidate set) before the apply pass itself is parallelised.
    let p = gossip_params();
    let w = Fixed::from_int(20);
    // S1 believes value 10 with candidate set {10, 20}; S2 believes 30 with {10, 30}.
    let belief = |first_s1: bool| {
        let mut m = Mind::new(StableId(1), Fixed::ONE);
        let s1 = (StableId(10), [10, 20], 10);
        let s2 = (StableId(20), [10, 30], 30);
        let order = if first_s1 { [s1, s2] } else { [s2, s1] };
        for (spk, hyps, toward) in order {
            m.consider(GOSSIP_SUBJECT, GOSSIP_ATTR, hyps, toward, w, spk);
        }
        m.belief(GOSSIP_SUBJECT, GOSSIP_ATTR, &p)
    };
    // S1 first fixes the candidate set to {10, 20}, dropping S2's evidence toward 30, so 10
    // commits; S2 first admits both, and the 20-vs-20 tie leaves the belief uncommitted.
    assert_eq!(belief(true), Some(10), "S1-first commits 10");
    assert_eq!(belief(false), None, "S2-first ties and stays uncommitted");
    assert_ne!(
        belief(true),
        belief(false),
        "documented seam: the gossip apply is order-dependent for mismatched candidate sets"
    );
}

// --- Site (b): the typology weighted pick ---

// Fixture tier weights and disharmony (labelled, never owner canon) so the sampler runs.
fn strong() -> Fixed {
    Fixed::from_int(64)
}
fn weak() -> Fixed {
    Fixed::from_int(4)
}
fn disharmony() -> Fixed {
    Fixed::from_ratio(1, 20)
}

/// Rebuild the registry, prior, and harmony model with every insertion order reversed: the
/// parameters added last-first, each parameter's values reversed, each prior's counts reversed,
/// and the harmony rows added in reverse. Every constructor sorts on insert (`add_param`,
/// `TypologyPrior::set`, `HarmonyModel::add`), so a correct substrate is structurally identical
/// to the canonical one and samples identically; this is the construction shuffle that proves
/// arrival order never reaches a draw.
fn reversed_construction(
    reg: &TypologyRegistry,
    prior: &TypologyPrior,
    harmony: &HarmonyModel,
) -> (TypologyRegistry, TypologyPrior, HarmonyModel) {
    let mut reg2 = TypologyRegistry::new();
    for p in reg.params().iter().rev() {
        let mut q = p.clone();
        q.values.reverse();
        reg2.add_param(q);
    }
    let mut prior2 = TypologyPrior::new();
    let pids: Vec<TypologyParamId> = prior.params().collect();
    for &pid in pids.iter().rev() {
        let mut counts = prior.counts(pid).unwrap().to_vec();
        counts.reverse();
        prior2.set(pid, counts, prior.source(pid).unwrap());
    }
    let mut harmony2 = HarmonyModel::new();
    for b in harmony.biases().iter().rev() {
        harmony2.add(b.clone());
    }
    (reg2, prior2, harmony2)
}

#[test]
fn typology_weighted_pick_is_input_order_independent() {
    // R-REDUCE-ORDER site (b): the typology weighted pick (typology.rs, the cumulative walk in
    // sample_profile). Proven two ways: the whole sampler is invariant to construction order, and
    // value id is the total key that recovers the canonical weight vector from any permutation.
    let (reg, prior, harmony) = wals_seed(strong(), weak());
    let (reg2, prior2, harmony2) = reversed_construction(&reg, &prior, &harmony);

    // End to end: the sampled profile is bit-identical across the construction shuffle, over a
    // sweep of cultures and ticks. A permuted value or count arrival cannot move a draw.
    for culture in 0..40u64 {
        for &tick in &[0u64, 3, 9] {
            let a =
                sample_profile(&reg, &prior, &harmony, disharmony(), 0x7ED, culture, tick).unwrap();
            let b = sample_profile(
                &reg2,
                &prior2,
                &harmony2,
                disharmony(),
                0x7ED,
                culture,
                tick,
            )
            .unwrap();
            assert_eq!(
                a, b,
                "the sampler leaked construction order at culture {culture}, tick {tick}"
            );
        }
    }

    // Targeted at the pick's reduction. `tilted_weights` preserves its input order, so a permuted
    // prior yields a permuted weight vector (the pick input is order-sensitive), yet sorting by
    // value id (the total key) recovers the identical ordered vector the pick walks.
    let adposition = TypologyParamId(1);
    let drawn = [(TypologyParamId(0), TypologyValueId(0))]; // OV, which tilts postpositions
    let counts = prior.counts(adposition).unwrap().to_vec(); // value-id sorted (canonical)
    let canonical_w = tilted_weights(&counts, adposition, &drawn, &harmony);

    let mut permuted = counts.clone();
    permuted.reverse();
    let permuted_w = tilted_weights(&permuted, adposition, &drawn, &harmony);

    assert_ne!(
        canonical_w, permuted_w,
        "tilted_weights preserves input order, so a permuted prior must give a permuted vector"
    );
    let repinned = canonical_sorted(permuted_w.clone(), |&(v, _)| v);
    assert_eq!(
        canonical_w, repinned,
        "value id is the total key: sorting the permuted weights by value id recovers the exact \
         vector the cumulative pick walks, so the pick is a pure function of the weight set"
    );

    // The pick result itself: bit-identical across the value-id-pinned permutation for every
    // draw r, while the raw (unpinned) permuted vector is order-sensitive, so the canonical walk
    // is load-bearing rather than incidental.
    let pick = |weights: &[(TypologyValueId, u128)], r: u128| -> TypologyValueId {
        // Mirrors the cumulative walk in sample_profile.
        let mut acc = 0u128;
        let mut chosen = weights.last().unwrap().0;
        for &(v, w) in weights {
            acc += w;
            if r < acc {
                chosen = v;
                break;
            }
        }
        chosen
    };
    let total: u128 = canonical_w.iter().map(|&(_, w)| w).sum();
    let mut raw_differs = false;
    for step in 0..64u128 {
        let r = (step * total) / 64;
        assert_eq!(
            pick(&canonical_w, r),
            pick(&repinned, r),
            "the value-id-pinned pick moved under a permuted prior at r-step {step}"
        );
        if pick(&canonical_w, r) != pick(&permuted_w, r) {
            raw_differs = true;
        }
    }
    assert!(
        raw_differs,
        "the raw pick over the unpinned permuted vector is order-sensitive, so the value-id walk \
         is load-bearing (the control that shows the pinning matters)"
    );
}
