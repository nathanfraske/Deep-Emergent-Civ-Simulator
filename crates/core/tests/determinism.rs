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

//! The determinism reproducibility harness (design Part 3.5, Part 60 Stage 1).
//!
//! This is the project's standing correctness contract: one seed yields one world,
//! bit for bit, on any machine and at any thread count. The harness runs a minimal
//! headless world at one, four, and the machine's worker count and asserts the
//! canonical state hash is identical. It is built on the determinism core alone, so
//! the contract is provable before any simulation system exists, and a regression
//! is caught the moment it is introduced rather than discovered as drift.
//!
//! The world here is deliberately small: each entity accumulates a fixed-point
//! quantity from its own counter-based RNG stream over a number of ticks. Because
//! every draw is a pure function of `(seed, entity, phase, counter)` and the
//! accumulation is exact `Fixed` addition, the result depends on neither the thread
//! count nor the chunking. The state is folded in `StableId` order, the fixed
//! canonical order of Part 3.5.

use civsim_core::{Fixed, Rng, StableId, StateHasher};

/// Compute one entity's final accumulator. Pure function of its coordinate.
fn entity_accumulator(seed: u64, id: StableId, ticks: u64) -> Fixed {
    let rng = Rng::for_entity(seed, id, /* phase */ 0);
    let mut acc = Fixed::ZERO;
    for t in 0..ticks {
        acc += rng.unit_fixed(t);
    }
    acc
}

/// Run the headless world at a given thread count and return the canonical state
/// hash. Entities are partitioned across `threads` scoped threads; each writes only
/// its own slots, so the result is independent of which thread computes which
/// entity and of where the chunk boundaries fall.
fn run_world(seed: u64, n: u32, ticks: u64, threads: usize) -> u128 {
    let mut acc = vec![Fixed::ZERO; n as usize];
    let threads = threads.max(1);
    let chunk = acc.len().div_ceil(threads).max(1);

    std::thread::scope(|s| {
        for (c, slice) in acc.chunks_mut(chunk).enumerate() {
            let base = c * chunk;
            s.spawn(move || {
                for (k, slot) in slice.iter_mut().enumerate() {
                    let id = StableId((base + k) as u64);
                    *slot = entity_accumulator(seed, id, ticks);
                }
            });
        }
    });

    // Fold in the fixed canonical order: ascending StableId.
    let mut hasher = StateHasher::new();
    for (i, a) in acc.iter().enumerate() {
        hasher.write_stable(StableId(i as u64));
        hasher.write_fixed(*a);
    }
    hasher.finish()
}

#[test]
fn same_seed_is_bit_identical_across_thread_counts() {
    let seed = 0x0BAD_F00D_DEAD_BEEF;
    let n = 5_000;
    let ticks = 64;

    let one = run_world(seed, n, ticks, 1);
    let four = run_world(seed, n, ticks, 4);
    let many = run_world(
        seed,
        n,
        ticks,
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4),
    );
    let odd = run_world(seed, n, ticks, 3); // a thread count that does not divide n evenly

    assert_eq!(one, four, "1 vs 4 threads diverged");
    assert_eq!(one, many, "1 vs machine-width threads diverged");
    assert_eq!(one, odd, "1 vs 3 threads (uneven chunking) diverged");
}

#[test]
fn different_seeds_give_different_worlds() {
    let n = 2_000;
    let ticks = 32;
    let a = run_world(1, n, ticks, 4);
    let b = run_world(2, n, ticks, 4);
    assert_ne!(a, b, "distinct seeds should not collide");
}

#[test]
fn reruns_reproduce_the_same_hash() {
    let seed = 42;
    let h1 = run_world(seed, 1_000, 50, 4);
    let h2 = run_world(seed, 1_000, 50, 4);
    assert_eq!(h1, h2, "a rerun reproduces the world exactly");
}

#[test]
fn parallel_fixed_reduction_matches_sequential() {
    // The Part 3.3 parallel-reduction hazard defense: a Fixed sum is associative,
    // so a parallel partition that sums slices and then the partials equals the
    // straight sequential fold, bit for bit.
    let seed = 7;
    let n = 10_000u32;
    let rng = Rng::for_entity(seed, StableId(0), 1);
    let values: Vec<Fixed> = (0..n as u64).map(|c| rng.unit_fixed(c)).collect();

    let sequential: Fixed = values.iter().copied().sum();

    let threads = 4;
    let chunk = values.len().div_ceil(threads);
    let partials: Vec<Fixed> = std::thread::scope(|s| {
        let handles: Vec<_> = values
            .chunks(chunk)
            .map(|sl| s.spawn(move || sl.iter().copied().sum::<Fixed>()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let parallel: Fixed = partials.into_iter().sum();

    assert_eq!(
        sequential, parallel,
        "parallel partition changed the canonical sum"
    );

    // The same reduction via the partition-safe primitive: partials in 128-bit bits,
    // combined in 128-bit bits, must equal the sequential bit total exactly.
    let seq_bits = Fixed::sum_bits(values.iter().copied());
    let par_bits: i128 = values
        .chunks(chunk)
        .map(|sl| Fixed::sum_bits(sl.iter().copied()))
        .sum();
    assert_eq!(seq_bits, par_bits, "sum_bits is partition-independent");
}

#[test]
fn order_independent_reduction_survives_intermediate_overflow() {
    // Regression for the determinism audit C-05: a multiset whose total is in range
    // but whose prefix is not. The naive Sum panics on the (ZERO + MAX) + 10 prefix
    // (overflow checks are on in both profiles for this repo), and whether that path
    // runs depends on the chunking, so it is a partition-dependent divergence.
    // Fixed::sum_bits accumulates in i128 and is identical for any order or grouping.
    let xs = [
        Fixed::from_bits(i64::MAX),
        Fixed::from_bits(10),
        Fixed::from_bits(-10),
    ];
    let mut rev = xs;
    rev.reverse();

    let a = Fixed::sum_bits(xs);
    let b = Fixed::sum_bits(rev);
    let partitioned = Fixed::sum_bits([xs[0]]) + Fixed::sum_bits([xs[1], xs[2]]);
    assert_eq!(a, b, "order does not change the bit total");
    assert_eq!(a, partitioned, "grouping does not change the bit total");
    assert_eq!(a, i64::MAX as i128, "the total is exactly i64::MAX bits");
    assert_eq!(Fixed::from_bits_i128(a), Some(Fixed::from_bits(i64::MAX)));

    // Demonstrate that the naive operator fold is the unsafe path on this input.
    let naive = std::panic::catch_unwind(|| xs.iter().copied().sum::<Fixed>());
    assert!(naive.is_err(), "naive Sum overflows on the bad prefix");
}
