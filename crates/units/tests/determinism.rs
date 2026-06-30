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

//! Determinism harness for the absolute-unit crate (design Part 55, Part 57
//! R-REDUCE-ORDER, R-UNITS-PIN). The crux is reducing a set of magnitudes: a fold of
//! the pairwise `add` is order-dependent under `Saturate` because saturating addition
//! is not associative, so the canonical reduction must accumulate exactly and clamp
//! once at read. These tests pin both halves: they demonstrate the order-dependence of
//! the naive fold (so the hazard is on record), and they prove the order-independence
//! and thread-count invariance of `AbsoluteQuantity::sum`.

use civsim_core::{Rng, StateHasher};
use civsim_units::{
    AbsoluteQuantity, BaseDimensionRegistry, Dimension, OverflowPolicy, QuantityDef,
    QuantityRegistry,
};

// A small fixture catalogue, not the authored physics set: one saturating quantity and
// one wrapping quantity, exercising both overflow disciplines.
fn fixture() -> (QuantityRegistry, u32, u32) {
    let mut base = BaseDimensionRegistry::new();
    let length = base.register("length");

    let mut q = QuantityRegistry::new();
    let saturating = q.register(QuantityDef {
        name: "distance".to_string(),
        dimension: Dimension::base(length),
        scale_bits: 16,
        overflow: OverflowPolicy::Saturate,
    });
    let wrapping = q.register(QuantityDef {
        name: "phase_angle".to_string(),
        dimension: Dimension::base(length),
        scale_bits: 16,
        overflow: OverflowPolicy::Wrap,
    });
    (q, saturating, wrapping)
}

#[test]
fn folding_add_is_order_dependent_under_saturate() {
    // The hazard the canonical reduction exists to avoid: a fold of pairwise `add`
    // gives different results in different orders once an intermediate sum saturates.
    let (q, dist, _w) = fixture();
    let max = AbsoluteQuantity::new(dist, i64::MAX);
    let plus = AbsoluteQuantity::new(dist, 1);
    let minus = AbsoluteQuantity::new(dist, -1);

    let forward = [max, plus, minus]
        .into_iter()
        .reduce(|a, b| a.add(b, &q))
        .unwrap();
    let reordered = [minus, plus, max]
        .into_iter()
        .reduce(|a, b| a.add(b, &q))
        .unwrap();

    assert_eq!(forward.bits, i64::MAX - 1, "max then +1 saturates, then -1");
    assert_eq!(reordered.bits, i64::MAX, "-1 then +1 cancels, then +max");
    assert_ne!(
        forward.bits, reordered.bits,
        "a fold of add is order-dependent under saturate; use sum instead"
    );
}

#[test]
fn sum_is_order_independent_under_saturate() {
    // The canonical reduction: the same multiset in any order yields the same result,
    // and that result is the exact total clamped once, not a saturated prefix.
    let (q, dist, _w) = fixture();
    let mk = |bits: i64| AbsoluteQuantity::new(dist, bits);
    let items = [mk(i64::MAX), mk(1), mk(-1)];

    let a = AbsoluteQuantity::sum(dist, items, &q);
    let b = AbsoluteQuantity::sum(dist, [mk(-1), mk(1), mk(i64::MAX)], &q);
    let c = AbsoluteQuantity::sum(dist, [mk(1), mk(i64::MAX), mk(-1)], &q);

    assert_eq!(a.bits, b.bits);
    assert_eq!(a.bits, c.bits);
    // The exact total MAX + 1 - 1 is MAX, which is in range, so no clamping occurs.
    assert_eq!(a.bits, i64::MAX);

    // And a total that genuinely exceeds the range clamps to the bound, in any order.
    let over = [mk(i64::MAX), mk(i64::MAX), mk(10)];
    let s1 = AbsoluteQuantity::sum(dist, over, &q);
    let s2 = AbsoluteQuantity::sum(dist, [mk(10), mk(i64::MAX), mk(i64::MAX)], &q);
    assert_eq!(s1.bits, i64::MAX);
    assert_eq!(s1.bits, s2.bits);
}

#[test]
fn sum_is_order_independent_under_wrap() {
    // Wrap is associative and commutative, so its reduction is order-independent too;
    // the wrapped total is the low 64 bits of the exact sum.
    let (q, _d, ang) = fixture();
    let mk = |bits: i64| AbsoluteQuantity::new(ang, bits);
    let items = [mk(i64::MAX), mk(i64::MAX), mk(3)];
    let a = AbsoluteQuantity::sum(ang, items, &q);
    let b = AbsoluteQuantity::sum(ang, [mk(3), mk(i64::MAX), mk(i64::MAX)], &q);
    assert_eq!(a.bits, b.bits);
    // i64::MAX + i64::MAX + 3 wraps to 1 in two's complement.
    assert_eq!(a.bits, 1);
}

#[test]
fn empty_sum_is_zero() {
    let (q, dist, _w) = fixture();
    assert_eq!(AbsoluteQuantity::sum(dist, [], &q).bits, 0);
}

#[test]
fn parallel_gather_then_canonical_sum_is_thread_count_invariant() {
    // The realistic pattern: magnitudes are produced in parallel keyed on canonical
    // coordinates, then reduced. The reduction is order-independent, so the hashed
    // result does not depend on how many threads produced the inputs.
    let (q, dist, _w) = fixture();
    let seed = 0x5EED_0055u64;
    let n = 64usize;

    let run = |threads: usize| -> u128 {
        let threads = threads.max(1);
        let mut bits = vec![0i64; n];
        let chunk = n.div_ceil(threads).max(1);
        std::thread::scope(|s| {
            for (c, slot) in bits.chunks_mut(chunk).enumerate() {
                let base = c * chunk;
                s.spawn(move || {
                    for (k, b) in slot.iter_mut().enumerate() {
                        // A coordinate-keyed draw, the observer-safe way, in a wide
                        // range so some partial orders would saturate a naive fold.
                        let r = Rng::for_coords(seed, &[(base + k) as u64]);
                        *b = (r.at(0) as i64) / 2;
                    }
                });
            }
        });
        let items = bits.iter().map(|&b| AbsoluteQuantity::new(dist, b));
        let total = AbsoluteQuantity::sum(dist, items, &q);
        let mut h = StateHasher::new();
        h.write_u32(total.quantity);
        h.write_i64(total.bits);
        h.finish()
    };

    let one = run(1);
    assert_eq!(one, run(2), "1 vs 2 threads diverged");
    assert_eq!(one, run(4), "1 vs 4 threads diverged");
    assert_eq!(one, run(3), "1 vs 3 threads (uneven) diverged");
}
