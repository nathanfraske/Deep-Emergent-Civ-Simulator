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

//! The sim-level determinism harness (design Part 3.5; R-HARNESS-COVER).
//!
//! The core harness (`crates/core/tests/determinism.rs`) proves the determinism
//! primitives, including the order-sensitive command barrier under a worker sweep. This
//! harness carries the contract up to the real simulation, the paths R-HARNESS-COVER
//! names that the core cannot reach: the full tick phase sequence (perceive, decide,
//! converse, gossip, the naming game, language drift), the gossip speaker partition, the
//! dialogue write pass and its event emission, structural mutation, the two-tier world
//! hash, and a save, load, and continued-replay cycle.
//!
//! Two properties are asserted. First, a dawn world run twice from one seed reproduces
//! the same canonical state hash and the same event-log hash at every tick, so any
//! iteration-order, gossip-partition, or dialogue-ordering regression in the live tick
//! surfaces immediately. Second, a two-tier world snapshotted mid-stream and reloaded
//! continues to the same hash as the world it was cloned from, so the save schema and
//! the id high-water marks reproduce the world exactly.
//!
//! The converse stage now runs its read pass on worker threads (the ActionStage of
//! design Part 4.1), with the produced moves re-ordered at the barrier by `CommandKey`
//! before application, so the third property asserted here is the R-CMD-ORDER contract
//! itself: the full tick is bit-identical at every worker count, because the applied
//! order is a pure function of the produced command set rather than of the thread that
//! produced it. The sweep runs the same replay at widths 1, 2, 3, and 8.

use civsim_core::{Fixed, StableId};
use civsim_sim::decision::Behaviour;
use civsim_sim::dialogue::{
    EffectSign, ForceEffectDef, ForceEffectId, ForceFloor, ForceKind, MoveKindDef, MoveKindId,
    MoveRegistry,
};
use civsim_sim::evidence::InferenceParams;
use civsim_sim::language::{ArticulationSubstrate, LanguageParams};
use civsim_sim::lod::TwoTierWorld;
use civsim_sim::primes::nsm_concept_ids;
use civsim_sim::runner::{Field, FieldCalib, Runner};
use civsim_sim::tom::{AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights};
use civsim_sim::world::{GossipParams, World};
use civsim_world::Coord3;

const WITNESSED: AccessChannelId = AccessChannelId(1);
const SAID: AccessChannelId = AccessChannelId(3);
const SYLLABLES: [&str; 12] = [
    "ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo", "ha", "du", "pe", "go",
];

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

/// The dialogue substrate (assert / accept / refuse), so the converse phase has work.
fn substrate() -> (ForceFloor, MoveRegistry) {
    let floor = ForceFloor {
        effects: vec![
            ForceEffectDef {
                id: ForceEffectId(1),
                kind: ForceKind::TellEvidence,
                sign: EffectSign::Neutral,
                name: "assert".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(2),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Positive,
                name: "accept".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(3),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Negative,
                name: "refuse".to_string(),
            },
        ],
    };
    let registry = MoveRegistry {
        moves: vec![
            MoveKindDef {
                id: MoveKindId(1),
                name: "assertion".to_string(),
                force: vec![ForceEffectId(1)],
                expects: vec![MoveKindId(2), MoveKindId(3)],
                sincerity_judged: true,
                felicity: vec![],
                gloss: "tells".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(2),
                name: "acceptance".to_string(),
                force: vec![ForceEffectId(2)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "agrees".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(3),
                name: "refusal".to_string(),
                force: vec![ForceEffectId(3)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "doubts".to_string(),
            },
        ],
    };
    (floor, registry)
}

/// A dawn world with language, dialogue, and gossip installed, `beings` minds spread
/// across `bands` co-located groups, everyone promoted to move-by-move dialogue, so a
/// tick exercises the full phase sequence. Mirrors the `tick_bench` fixture.
fn dawn_world(beings: usize, bands: usize, seed: u64) -> World {
    let bands = bands.max(1);
    let mut w = World::new(
        params(),
        params(),
        AccessWeights::from_pairs([(WITNESSED, Fixed::from_int(4)), (SAID, Fixed::from_int(2))]),
    )
    .with_seed(seed);
    w.set_channels(AccessChannelRegistry {
        channels: vec![
            AccessChannelDef {
                id: WITNESSED,
                name: "witnessed".to_string(),
            },
            AccessChannelDef {
                id: SAID,
                name: "said".to_string(),
            },
        ],
    });
    w.set_gossip(GossipParams {
        told_weight: Fixed::from_int(3),
        trust_baseline: Fixed::ONE,
        trust_penalty: Fixed::from_ratio(1, 2),
    });
    w.set_concepts(nsm_concept_ids());
    let (_substr, forms) = ArticulationSubstrate::syllabic(SYLLABLES.map(String::from), 2, 3);
    w.set_form_system(forms);
    w.set_language(LanguageParams {
        innovation_rate: Fixed::ZERO,
    });
    let (floor, registry) = substrate();
    w.set_dialogue(registry, floor)
        .expect("the dialogue substrate passes the content gate");

    let ids: Vec<StableId> = (0..beings).map(|_| w.spawn(Fixed::ONE)).collect();
    for (i, &m) in ids.iter().enumerate() {
        w.set_place(m, (i % bands) as u32);
        w.promote(m);
    }
    w
}

/// Run a dawn world for `ticks` at the given ActionStage worker width and return the
/// (state hash, event-log hash) after each tick, the canonical fingerprint of the run.
fn run_trace_workers(
    beings: usize,
    bands: usize,
    seed: u64,
    ticks: u64,
    workers: usize,
) -> Vec<(u128, u128)> {
    let mut w = dawn_world(beings, bands, seed);
    w.set_workers(workers);
    let mut trace = Vec::with_capacity(ticks as usize);
    for _ in 0..ticks {
        w.tick(&[]);
        trace.push((w.state_hash(), w.event_log_hash()));
    }
    trace
}

/// Run a dawn world serially (worker width 1).
fn run_trace(beings: usize, bands: usize, seed: u64, ticks: u64) -> Vec<(u128, u128)> {
    run_trace_workers(beings, bands, seed, ticks, 1)
}

#[test]
fn dawn_tick_replay_is_bit_identical() {
    // The live tick exercises the gossip speaker partition, the dialogue write pass and
    // its event appends, the naming game, and structural mutation, none of which the
    // core accumulation harness reaches. Two runs from one seed must produce the same
    // canonical state hash and event-log hash at every tick; a divergence anywhere is an
    // iteration-order or ordering regression in a real phase (R-HARNESS-COVER).
    let a = run_trace(40, 5, 0xD00D_F00D, 120);
    let b = run_trace(40, 5, 0xD00D_F00D, 120);
    assert_eq!(a.len(), 120);
    assert_eq!(a, b, "the dawn tick did not replay bit for bit");
}

#[test]
fn dawn_tick_diverges_on_a_different_seed() {
    // The trace is seed-sensitive, so the bit-identity above is a real reproduction of a
    // non-trivial run rather than a constant.
    let a = run_trace(40, 5, 1, 60);
    let b = run_trace(40, 5, 2, 60);
    assert_ne!(a, b, "distinct seeds should not produce the same trace");
}

#[test]
fn full_tick_is_bit_identical_across_worker_counts() {
    // The R-CMD-ORDER adoption proof, and the thread-count sweep R-HARNESS-COVER names:
    // the converse read pass runs on worker threads producing keyed commands, the
    // barrier re-orders them by CommandKey, and the whole tick's canonical state and
    // event log must be bit-identical at every width. Round-robin turn assignment means
    // the pre-barrier production order scrambles differently at each width, so only the
    // total-order sort can be holding the canon still.
    let serial = run_trace_workers(40, 5, 0xC0FFEE, 80, 1);
    for workers in [2usize, 3, 8] {
        let parallel = run_trace_workers(40, 5, 0xC0FFEE, 80, workers);
        assert_eq!(
            serial, parallel,
            "the tick diverged at {workers} workers: the applied order leaked the thread schedule"
        );
    }
}

#[test]
fn two_tier_save_load_and_continued_replay_match() {
    // The save, load, and replay cycle R-HARNESS-COVER names, over the two-tier world
    // hashed in canonical order: build some state, snapshot mid-stream, reload into a
    // fresh world, then apply the identical further mutations to the original and the
    // reload. They must track bit for bit, so the snapshot restores the id high-water
    // marks and the canonical state exactly (never reusing an id after load).
    let mut live = TwoTierWorld::new();
    let pa = live.add_pool(12, Fixed::from_int(120));
    let pb = live.add_pool(6, Fixed::from_int(60));
    let x = live.promote(pa, Fixed::from_int(7));
    let y = live.promote(pb, Fixed::from_int(4));
    live.add_edge(x, y);

    // Snapshot mid-stream and reload.
    let snap = live.to_snapshot();
    let mut reload = TwoTierWorld::from_snapshot(&snap);
    assert_eq!(
        reload.state_hash(),
        live.state_hash(),
        "the reload hashes identically at the snapshot point"
    );

    // Apply the identical continuation to both. A newly promoted individual must be
    // minted the same id in each, which only holds if the high-water mark was restored.
    let continue_run = |w: &mut TwoTierWorld| {
        let z = w.promote(pa, Fixed::from_int(2));
        w.add_edge(y, z);
        w.demote(x, pb);
    };
    continue_run(&mut live);
    continue_run(&mut reload);

    assert_eq!(
        reload.state_hash(),
        live.state_hash(),
        "the reload diverged from the original after continued replay"
    );
    assert!(reload.referential_integrity_ok());
}

// --- The composed canonical tick: the runner drives the cognition world (roadmap Tier B item 2) ---

/// A labelled temperature-field fixture: a small bounded field with a fixed, clearly-authored
/// baseline gradient. A test fixture, never owner canon (the runner carries no default calibration).
fn field_fixture() -> Field {
    let (w, h) = (8i32, 6i32);
    let baseline: Vec<Fixed> = (0..(w * h)).map(|i| Fixed::from_int(i % 5)).collect();
    Field::new(w, h, baseline)
}

/// Labelled field calibrations, within the documented stability bounds (diffusion below 0.25,
/// relaxation and exchange in [0, 1]). A fixture, never owner canon.
fn field_calib() -> FieldCalib {
    FieldCalib {
        diffusion: Fixed::from_ratio(1, 8),
        relaxation: Fixed::from_ratio(1, 16),
        exchange: Fixed::from_ratio(1, 4),
    }
}

/// Run the COMPOSED canonical tick: a runner that owns a live dawn world steps the field, the
/// body-thermal exchange, and the world cognition tick as one composite step, returning the composite
/// (runner state hash, world event-log hash) after each tick. A few located beings sit in the field
/// (labelled fixture ids in a high range, distinct from the world's cognition beings) so the
/// field-thermal state is exercised; the two sides share no data across the seam yet, only the
/// composite step and hash. Unifying the two populations is the field-to-cognition coupling increment.
fn composed_trace(
    beings: usize,
    bands: usize,
    seed: u64,
    ticks: u64,
    workers: usize,
) -> Vec<(u128, u128)> {
    let mut world = dawn_world(beings, bands, seed);
    world.set_workers(workers);
    let mut runner = Runner::with_world(field_fixture(), field_calib(), world);
    for k in 0..4u64 {
        let id = StableId(10_000 + k);
        let coord = Coord3::ground((k as i32) % 8, (k as i32) % 6);
        runner.place_being(id, coord, Fixed::from_int(37));
    }
    let mut trace = Vec::with_capacity(ticks as usize);
    for _ in 0..ticks {
        runner.step();
        let world = runner.world().expect("the composed runner owns a world");
        // Clock lockstep: the field spine and the cognition world advance together, one per step.
        assert_eq!(
            runner.clock(),
            world.clock(),
            "runner and world clocks drifted"
        );
        trace.push((runner.state_hash(), world.event_log_hash()));
    }
    trace
}

#[test]
fn composed_runner_tick_replay_is_bit_identical() {
    // The canonical runner now drives the cognition world as a fixed sub-phase after its field
    // phases. Two composed runs from one seed must produce the same composite state hash and world
    // event-log hash at every tick: the field spine and the six-phase cognition tick, folded into one
    // canonical fingerprint, reproduce bit for bit.
    let a = composed_trace(40, 5, 0xC0DE_F00D, 100, 1);
    let b = composed_trace(40, 5, 0xC0DE_F00D, 100, 1);
    assert_eq!(a.len(), 100);
    assert_eq!(a, b, "the composed tick did not replay bit for bit");
}

#[test]
fn composed_runner_tick_diverges_on_a_different_seed() {
    // The composite trace is seed-sensitive, so the bit-identity above is a real reproduction of a
    // non-trivial composed run rather than a constant.
    let a = composed_trace(40, 5, 1, 60, 1);
    let b = composed_trace(40, 5, 2, 60, 1);
    assert_ne!(
        a, b,
        "distinct seeds should not produce the same composed trace"
    );
}

#[test]
fn composed_runner_tick_is_bit_identical_across_worker_counts() {
    // The field spine is worker-agnostic and the cognition tick re-orders its commands at the
    // CommandKey barrier, so the composite must be bit-identical at every World worker width: the
    // field-first-then-cognition order is fixed and the applied command order is a pure function of
    // the produced set, not of the thread that produced it (R-CMD-ORDER, R-HARNESS-COVER).
    let serial = composed_trace(40, 5, 0xBEEF, 80, 1);
    for workers in [2usize, 3, 8] {
        let parallel = composed_trace(40, 5, 0xBEEF, 80, workers);
        assert_eq!(
            serial, parallel,
            "the composed tick diverged at {workers} workers"
        );
    }
}

#[test]
#[should_panic(expected = "authored decision repertoire")]
fn the_canonical_runner_refuses_an_authored_behaviour_repertoire() {
    // The Principle 9 steering boundary the canonical runner holds: an authored drive-and-action
    // repertoire (the sentient deliberative tier, Part 8.1) is steering at the level of behaviour
    // (Part 8.4) and must not ride the canonical-emergent spine, whose behaviour source is the evolved
    // controller. Installing one (even an empty one, which still sets has_behaviour) and composing it
    // onto the canonical runner is a fail-loud steering leak.
    let mut world = dawn_world(4, 2, 0x5EED);
    world.set_behaviour(Behaviour {
        drives: vec![],
        curves: vec![],
        actions: vec![],
    });
    let _ = Runner::with_world(field_fixture(), field_calib(), world);
}
