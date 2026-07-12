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

//! Q1 Stone 2: the observation-schedule invariance guarantee (Principle 10). The world's canonical
//! state is independent of WHEN and HOW it is observed, so interleaving an observer's reads at any
//! cadence, or none, leaves the trajectory bit-identical. This is the load-bearing prerequisite the
//! deep-time level-of-detail tower is built against: a coarsen-then-reconstruct that is not bit-exact
//! would change the trajectory under a different observation schedule and fail this test at exactly the
//! change under review, so it is stood up now, while it is green, before the tower.
//!
//! The observer surface does not compose on one world (verified at source: `TwoTierWorld::to_snapshot`,
//! a generic `ConservationRegistry::snapshot`, and `WorldGenesis::snapshot` are distinct subsystems), so
//! the guard is per-surface. This file carries the RICH cadence test on the `WorldGenesis` ecological
//! driver, the one bed with a canonical tick (`step_once`), an observation (`snapshot` yielding the
//! `LivingWorld` the viewer renders), and a hash (`LivingWorld::state_hash`). The recorded object is the
//! PER-STEP trajectory (the radiation runs to a fixed endpoint, so a final-state hash is invariant even
//! to a perturbation and would be a vacuous target; the per-step sequence is the trajectory a
//! side-effecting observation would shift). A render-equivalent read of the `LivingWorld` is folded
//! into the observation so the guard covers the render path with no `viewer` dependency.

use civsim_sim::conservation::ConservationRegistry;
use civsim_sim::environ::AbioticSourceRegistry;
use civsim_sim::genesis::{GenesisParams, WorldGenesis};
use civsim_sim::lod::TwoTierWorld;
use std::hint::black_box;

/// A small deterministic ecological driver: dev-default worldgen and founders, ready to radiate.
fn driver(seed: u64) -> WorldGenesis {
    WorldGenesis::new(
        seed,
        &GenesisParams::dev_default(),
        &AbioticSourceRegistry::earth_dev(),
        None,
    )
}

/// A read-only observation: take the `LivingWorld` snapshot the viewer renders, read its render-facing
/// projection (its hash, its region and occupant structure), and discard it. Touches nothing canonical.
fn read_only_observation(d: &WorldGenesis) {
    let living = d.snapshot();
    black_box(living.state_hash());
    black_box(living.regions.len());
    black_box(living.species());
    black_box(living.alive());
}

/// The per-step trajectory: the sequence of `LivingWorld` state hashes recorded after each generation
/// step, with a read-only observation additionally applied at `extra_cadence` (0 observes never).
fn trajectory(seed: u64, extra_cadence: u64) -> Vec<u128> {
    let mut d = driver(seed);
    let mut traj = Vec::new();
    let mut t: u64 = 0;
    while d.step_once() {
        if extra_cadence != 0 && t.is_multiple_of(extra_cadence) {
            read_only_observation(&d);
        }
        traj.push(d.snapshot().state_hash());
        t += 1;
    }
    traj
}

#[test]
fn the_world_trajectory_is_invariant_to_the_observation_schedule() {
    let seed = 0x0B5E_12AA;
    let baseline = trajectory(seed, 0);
    assert!(
        baseline.len() >= 2,
        "the radiation must run enough generations for the trajectory to be non-trivial (got {})",
        baseline.len()
    );
    // The same run observed at three cadences: every step, every third, and a coarser eighth.
    assert_eq!(
        baseline,
        trajectory(seed, 1),
        "observing every step changed the trajectory"
    );
    assert_eq!(
        baseline,
        trajectory(seed, 3),
        "observing every third step changed the trajectory"
    );
    assert_eq!(
        baseline,
        trajectory(seed, 8),
        "observing every eighth step changed the trajectory"
    );
}

#[test]
fn a_non_read_only_observation_breaks_the_invariance_so_the_guard_is_live() {
    // The red control (non-vacuous guard): an "observation" that perturbs canonical state, modelled here
    // by advancing the driver an extra generation (a stand-in for any non-read-only observation), MUST
    // shift the per-step trajectory, proving the invariance assertion can fail and is not trivially true.
    let seed = 0x0B5E_12AA;
    let baseline = trajectory(seed, 0);
    let mut d = driver(seed);
    let mut perturbed = Vec::new();
    let mut t: u64 = 0;
    while d.step_once() {
        if t == 0 {
            // The perturbing "observation": it writes through (advances state) where a real observer reads.
            d.step_once();
        }
        perturbed.push(d.snapshot().state_hash());
        t += 1;
    }
    assert_ne!(
        baseline, perturbed,
        "a state-perturbing observation must shift the trajectory; if it does not, the guard is vacuous"
    );
}

#[test]
fn the_trajectory_hash_discriminates_distinct_worlds() {
    // A discrimination control so the invariance is not passing on a constant hash: two distinct seeds
    // radiate to distinct living worlds and must produce distinct trajectories.
    assert_ne!(
        trajectory(0x0B5E_12AA, 0),
        trajectory(0x0B5E_12AB, 0),
        "distinct seeds must produce distinct trajectory hashes"
    );
}

// The two static observation surfaces (`TwoTierWorld::to_snapshot`, `ConservationRegistry::snapshot`) have
// no canonical tick yet, so cadence-invariance is not testable on them (that coverage rides the LoD
// tower, the deferred follow-on). The meaningful guard now is read-only-ness: an observation that mutates
// its source through an interior-mutability side-effect would change the source state, which these catch.

#[test]
fn to_snapshot_is_read_only_the_lod_source_is_unchanged() {
    let w = TwoTierWorld::new();
    let before = w.state_hash();
    let a = w.to_snapshot();
    let b = w.to_snapshot();
    let after = w.state_hash();
    assert_eq!(
        a, b,
        "to_snapshot is not idempotent; the read mutates its own output"
    );
    assert_eq!(
        before, after,
        "to_snapshot changed the LoD source state; the observation is not read-only"
    );
}

#[test]
fn conservation_snapshot_is_read_only_the_measured_world_is_unchanged() {
    let mut ledger: ConservationRegistry<TwoTierWorld> = ConservationRegistry::new();
    // `register` is `&mut self`; the read path under test is `snapshot(&self, &world)`, which reads the
    // world through the registered projection. The projection reads the world, so a side-effecting read
    // would show up in the world's state hash below.
    ledger.register("lod_hash_popcount", |w: &TwoTierWorld| {
        i128::from(w.state_hash().count_ones())
    });
    let world = TwoTierWorld::new();
    let before = world.state_hash();
    let a = ledger.snapshot(&world);
    let b = ledger.snapshot(&world);
    let after = world.state_hash();
    assert_eq!(
        a, b,
        "conservation snapshot is not idempotent; the measure mutates through the read"
    );
    assert_eq!(
        before, after,
        "conservation snapshot changed the measured world; the observation is not read-only"
    );
}
