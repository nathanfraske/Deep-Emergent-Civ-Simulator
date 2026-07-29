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

//! Two-tier-world determinism harness (R-HARNESS-COVER groundwork, design Part 3.5,
//! Part 60 Stage 1). This extends the pure-accumulation determinism harness in
//! crates/core to a world that undergoes structural mutation (promote, demote,
//! merge, split) and references by edge. It asserts replay determinism (the same
//! seed reproduces the same canonical state hash), save and load round-trip
//! identity, and thread-count invariance of a per-entity fold keyed the
//! observer-safe way (`for_coords`). The full coverage the audit asks for (real
//! tick phases, command buffers, and gossip partitions) lands as those systems are
//! built.

use civsim_core::{Fixed, Rng, StableId, StateHasher};
use civsim_sim::lod::TwoTierWorld;

/// A fixed, deterministic sequence of structural mutations driven by counter-based
/// RNG keyed on canonical coordinates, never on allocation order.
fn build_world(seed: u64) -> TwoTierWorld {
    let mut w = TwoTierWorld::new();
    let pa = w.add_pool(50, Fixed::from_int(500));
    let pb = w.add_pool(30, Fixed::from_int(300));

    let mut promoted = Vec::new();
    for k in 0..10u64 {
        // Share in [1, 10], drawn from a coordinate-keyed stream (phase 1, ordinal k).
        let r = Rng::for_coords(seed, &[1, k]);
        let amt = 1 + r.range_u32(0, 10) as i32;
        let pool = if k % 2 == 0 { pa } else { pb };
        promoted.push(w.promote(pool, Fixed::from_int(amt)));
    }
    for k in 0..promoted.len() - 1 {
        w.add_edge(promoted[k], promoted[k + 1]);
    }
    // Demote a deterministic subset back into pool a.
    for k in (0..promoted.len()).step_by(3) {
        w.demote(promoted[k], pa);
    }
    let merged = w.merge_pools(pa, pb);
    w.split_pool(merged, 5, Fixed::from_int(20));
    w
}

#[test]
fn replay_reproduces_the_same_canonical_state() {
    let h1 = build_world(0x5EED_0001).state_hash();
    let h2 = build_world(0x5EED_0001).state_hash();
    assert_eq!(h1, h2, "the same seed reproduces the same world");
    let h3 = build_world(0x5EED_0002).state_hash();
    assert_ne!(h1, h3, "a different seed gives a different world");
}

#[test]
fn world_survives_a_snapshot_round_trip() {
    let w = build_world(0x5EED_0003);
    let before = w.state_hash();
    let snap = w.to_snapshot();
    let restored = TwoTierWorld::from_snapshot(&snap);
    assert_eq!(
        restored.state_hash(),
        before,
        "snapshot round trip is identity"
    );
    assert!(restored.referential_integrity_ok());
    // The high-water mark is restored, so the next mint does not reuse an id.
    let mut r2 = restored;
    let next = r2.reg.mint();
    assert_eq!(next.0, w.reg.next_raw(), "reload never reuses an id");
}

#[test]
fn per_entity_fold_is_thread_count_invariant() {
    // A canonical fold over per-entity work keyed on canonical coordinates is
    // independent of the thread count, the property the determinism harness exists
    // to hold once parallel phases run over the world.
    let w = build_world(0x5EED_0004);
    let seed = 0x5EED_0004;

    let mut ids: Vec<StableId> = w.individuals.iter().map(|i| i.id).collect();
    ids.sort();

    let fold = |threads: usize| -> u128 {
        let draws: Vec<u64> = {
            let threads = threads.max(1);
            let mut out = vec![0u64; ids.len()];
            let chunk = ids.len().div_ceil(threads).max(1);
            std::thread::scope(|s| {
                for (c, slot) in out.chunks_mut(chunk).enumerate() {
                    let base = c * chunk;
                    let ids = &ids;
                    s.spawn(move || {
                        for (k, d) in slot.iter_mut().enumerate() {
                            let id = ids[base + k];
                            *d = Rng::for_coords(seed, &[id.0]).at(0);
                        }
                    });
                }
            });
            out
        };
        // Fold in canonical (sorted-id) order.
        let mut h = StateHasher::new();
        for (id, d) in ids.iter().zip(draws.iter()) {
            h.write_stable(*id);
            h.write_u64(*d);
        }
        h.finish()
    };

    let one = fold(1);
    assert_eq!(one, fold(2), "1 vs 2 threads diverged");
    assert_eq!(one, fold(4), "1 vs 4 threads diverged");
    assert_eq!(one, fold(3), "1 vs 3 threads (uneven) diverged");
}
