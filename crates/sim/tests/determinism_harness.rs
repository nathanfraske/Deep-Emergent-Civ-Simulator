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
//!
//! For that third property to bite, the command set must be non-empty. The sweep drives
//! the promoted beings with an observation schedule (`seed_observations`) so they form
//! beliefs and assert, accept, and refuse them in the converse phase, and a move-count
//! guard (`MIN_EXPECTED_MOVES`) asserts the run stays well clear of zero. An undriven tick
//! forms no belief, emits no move, and would let the barrier prove an empty set identical:
//! a vacuous pass the guard now refuses.

use civsim_bio::anatomy::{
    BodyPlan, BodyPlanRegistry, OrganKindDef, Part, Temperament, TissueComposition,
};
use civsim_bio::decision::Behaviour;
use civsim_bio::evidence::{AttrKindId, InferenceParams, ValueId};
use civsim_bio::tom::{AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights};
use civsim_core::{Fixed, StableId};
use civsim_sim::controller::Controller;
use civsim_sim::dialogue::{
    EffectSign, ForceEffectDef, ForceEffectId, ForceFloor, ForceKind, MoveKindDef, MoveKindId,
    MoveRegistry,
};
use civsim_sim::edibility::Physiology;
use civsim_sim::homeostasis::{AffordanceRegistry, Homeostasis, HomeostaticRegistry};
use civsim_sim::language::{ArticulationSubstrate, LanguageParams};
use civsim_sim::locomotion::{LocomotionParams, Walker};
use civsim_sim::lod::TwoTierWorld;
use civsim_sim::primes::nsm_concept_ids;
use civsim_sim::runner::{BeingThermal, Embodiment, Field, FieldCalib, Runner};
use civsim_sim::world::{GossipParams, Stimulus, TickInput, World};
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

// The observation schedule that drives the promoted beings to form beliefs, so the
// converse phase turns those beliefs into dialogue moves and the CommandKey barrier
// re-orders a non-empty, thread-scrambled command set (R-HARNESS-COVER Phase-0). Without
// this the sweep ticks on empty input, no belief forms, no move is emitted, and the
// barrier proves an empty command set identical: a vacuous pass. A fixed function of the
// being and the tick, so it stays deterministic and camera-free.
const OBS_SUBJECT: StableId = StableId(900_000);
const OBS_ATTR: AttrKindId = AttrKindId(0);
const OBS_HYPS: [ValueId; 3] = [10, 20, 30];
/// The observation weight, above the fixture commit threshold (3) and runner-up margin (2),
/// so one observation commits a belief the speaker can then assert.
const OBS_WEIGHT: i32 = 5;
/// Re-seed beliefs on this cadence; between re-seeds the converse phase propagates and
/// converges, so the run is a train of non-empty move bursts rather than one early burst.
const OBS_REFRESH: u64 = 10;

/// The observations for tick `t`: on each refresh boundary every being observes one value
/// of a shared question. Neighbouring indices (which land in different bands, and which
/// within one band step by `bands`) draw different values, so a band holds a disagreement
/// its promoted speakers assert to and take up from each other; the value each being sees
/// rotates every refresh, so later ticks keep producing moves. A pure function of the id
/// and the tick.
fn seed_observations(ids: &[StableId], t: u64) -> Vec<TickInput> {
    if !t.is_multiple_of(OBS_REFRESH) {
        return Vec::new();
    }
    let epoch = t / OBS_REFRESH;
    ids.iter()
        .enumerate()
        .map(|(i, &mind)| {
            let toward = OBS_HYPS[(i as u64 + epoch) as usize % OBS_HYPS.len()];
            TickInput {
                mind,
                ordinal: 0,
                stim: Stimulus::Observe {
                    subject: OBS_SUBJECT,
                    attr: OBS_ATTR,
                    hyps: OBS_HYPS.to_vec(),
                    toward,
                    weight: Fixed::from_int(OBS_WEIGHT),
                    from: mind,
                },
            }
        })
        .collect()
}

/// The floor the driven converse phase must clear so the worker sweep can never silently
/// go vacuous again. The observed count over the 40-being, 80-tick run is far above this;
/// the guard only has to be well clear of zero and of a trivial handful (R-HARNESS-COVER).
const MIN_EXPECTED_MOVES: usize = 200;

/// A dawn world with language, dialogue, and gossip installed, `beings` minds spread
/// across `bands` co-located groups, everyone promoted to move-by-move dialogue, so a
/// tick exercises the full phase sequence. Mirrors the `tick_bench` fixture. Returns the
/// world and the minted being ids, so the caller can drive them with observations.
fn dawn_world(beings: usize, bands: usize, seed: u64) -> (World, Vec<StableId>) {
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
                margin_steps: Some(1),
            },
            AccessChannelDef {
                id: SAID,
                name: "said".to_string(),
                margin_steps: Some(-1),
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
    (w, ids)
}

/// Run a dawn world for `ticks` at the given ActionStage worker width, driving the
/// promoted beings with the observation schedule so the converse phase produces dialogue
/// moves. Returns the (state hash, event-log hash) after each tick, the canonical
/// fingerprint of the run, and the total dialogue-move count (every dialogue move is the
/// only thing that appends to the event log, so this is exactly `events().len()`), which
/// the sweep guards against silently falling to zero.
fn run_trace_workers(
    beings: usize,
    bands: usize,
    seed: u64,
    ticks: u64,
    workers: usize,
) -> (Vec<(u128, u128)>, usize) {
    let (mut w, ids) = dawn_world(beings, bands, seed);
    w.set_workers(workers);
    let mut trace = Vec::with_capacity(ticks as usize);
    for t in 0..ticks {
        w.tick(&seed_observations(&ids, t));
        trace.push((w.state_hash(), w.event_log_hash()));
    }
    (trace, w.events().len())
}

/// Run a dawn world serially (worker width 1), returning the hash trace only.
fn run_trace(beings: usize, bands: usize, seed: u64, ticks: u64) -> Vec<(u128, u128)> {
    run_trace_workers(beings, bands, seed, ticks, 1).0
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
    // total-order sort can be holding the canon still. The beings are driven by the
    // observation schedule, so this command set is a non-empty, thread-scrambled train of
    // real dialogue moves, not the empty set an undriven tick would produce.
    let (serial, serial_moves) = run_trace_workers(40, 5, 0xC0FFEE, 80, 1);
    // The anti-vacuity guard (R-HARNESS-COVER Phase-0): the driven converse phase must
    // emit a substantial dialogue-move set, or the sweep would prove an empty command set
    // identical and catch no ordering regression. This fails loud on any regression that
    // silences the converse phase (a lost observation path, a broken promotion, a dropped
    // move) before the bit-identity check can go vacuously green.
    assert!(
        serial_moves >= MIN_EXPECTED_MOVES,
        "the converse phase produced only {serial_moves} dialogue moves (expected at least \
         {MIN_EXPECTED_MOVES}): the worker sweep would prove an empty command set identical \
         (R-HARNESS-COVER guard)"
    );
    for workers in [2usize, 3, 8] {
        let (parallel, parallel_moves) = run_trace_workers(40, 5, 0xC0FFEE, 80, workers);
        assert_eq!(
            serial, parallel,
            "the tick diverged at {workers} workers: the applied order leaked the thread schedule"
        );
        assert_eq!(
            serial_moves, parallel_moves,
            "the dialogue-move count is not width-invariant at {workers} workers: the barrier \
             dropped or duplicated a move under parallelism"
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
) -> (Vec<(u128, u128)>, usize) {
    let (mut world, ids) = dawn_world(beings, bands, seed);
    world.set_workers(workers);
    let mut runner = Runner::with_world(field_fixture(), field_calib(), world);
    for k in 0..4u64 {
        let id = StableId(10_000 + k);
        let coord = Coord3::ground((k as i32) % 8, (k as i32) % 6);
        runner.place_being(id, coord, Fixed::from_int(37));
    }
    let mut trace = Vec::with_capacity(ticks as usize);
    for t in 0..ticks {
        // The observation schedule feeds the cognition sub-phase of the composite step, so
        // the composed sweep exercises the same non-empty CommandKey barrier the direct
        // sweep does, rather than a cognition side that never speaks.
        runner.step_with_world_inputs(&seed_observations(&ids, t));
        let world = runner.world().expect("the composed runner owns a world");
        // Clock lockstep: the field spine and the cognition world advance together, one per step.
        assert_eq!(
            runner.clock(),
            world.clock(),
            "runner and world clocks drifted"
        );
        trace.push((runner.state_hash(), world.event_log_hash()));
    }
    let moves = runner
        .world()
        .expect("the composed runner owns a world")
        .events()
        .len();
    (trace, moves)
}

#[test]
fn composed_runner_tick_replay_is_bit_identical() {
    // The canonical runner now drives the cognition world as a fixed sub-phase after its field
    // phases. Two composed runs from one seed must produce the same composite state hash and world
    // event-log hash at every tick: the field spine and the six-phase cognition tick, folded into one
    // canonical fingerprint, reproduce bit for bit.
    let (a, _) = composed_trace(40, 5, 0xC0DE_F00D, 100, 1);
    let (b, _) = composed_trace(40, 5, 0xC0DE_F00D, 100, 1);
    assert_eq!(a.len(), 100);
    assert_eq!(a, b, "the composed tick did not replay bit for bit");
}

#[test]
fn composed_runner_tick_diverges_on_a_different_seed() {
    // The composite trace is seed-sensitive, so the bit-identity above is a real reproduction of a
    // non-trivial composed run rather than a constant.
    let (a, _) = composed_trace(40, 5, 1, 60, 1);
    let (b, _) = composed_trace(40, 5, 2, 60, 1);
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
    let (serial, serial_moves) = composed_trace(40, 5, 0xBEEF, 80, 1);
    // The same anti-vacuity guard the direct sweep carries: the composed cognition side
    // must produce a non-empty dialogue-move set, or its barrier proves nothing.
    assert!(
        serial_moves >= MIN_EXPECTED_MOVES,
        "the composed converse phase produced only {serial_moves} dialogue moves (expected at \
         least {MIN_EXPECTED_MOVES}): the composed sweep would prove an empty command set \
         identical (R-HARNESS-COVER guard)"
    );
    for workers in [2usize, 3, 8] {
        let (parallel, parallel_moves) = composed_trace(40, 5, 0xBEEF, 80, workers);
        assert_eq!(
            serial, parallel,
            "the composed tick diverged at {workers} workers"
        );
        assert_eq!(
            serial_moves, parallel_moves,
            "the composed dialogue-move count is not width-invariant at {workers} workers"
        );
    }
}

// --- Real-world unification, step 2: the shared-id runner (world minds AND embodiment bodies) ---

/// A mobile development body plan (the thermal-coupling fixture), so a founder's walker has a body to
/// thermoregulate. Labelled fixture, not owner data.
fn mobile_body() -> BodyPlan {
    BodyPlan {
        body_mass: Fixed::from_ratio(1, 2),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons: vec![],
        covering: Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        },
        senses: vec![],
        locomotion: vec![1],
        organs: vec![],
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(3, 4),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    }
}

/// A viable thermal band fixture centred on the `field_fixture` temperature range (its cells hold `i %
/// 5`, so 0 to 4), the spawn temperature the same, so a being stays inside its comfort band on that
/// field rather than freezing out of it. This matters since the lifecycle pairing (step 3c) retires the
/// mind of a body that dies of thermal exposure: a band matched to the field keeps the crux sweep's
/// population alive so it exercises a rich command set, and the dedicated 3c test exercises death.
fn thermal_band() -> BeingThermal {
    BeingThermal {
        setpoint: Fixed::from_int(2),
        half_band: Fixed::from_int(8),
        initial_temp: Fixed::from_int(2),
    }
}

/// An embodiment whose walkers REUSE the world's founder ids (a shared id space), the crux of the
/// unification: one `StableId` owns both a `World` mind and an `Embodiment` walker. Blank controllers
/// (the being still thermoregulates through the field-thermal coupling), the temperature-only
/// development physiology.
fn shared_embodiment(ids: &[StableId], seed: u64) -> Embodiment {
    let mut emb = Embodiment::new(
        HomeostaticRegistry::dev_thermal(),
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        0,
        seed,
    );
    let blank = Controller::zeros(emb.layout());
    for (k, &id) in ids.iter().enumerate() {
        let coord = Coord3::ground((k as i32) % 8, (k as i32) % 6);
        let walker = Walker::new(
            id,
            coord,
            mobile_body(),
            Homeostasis::from_mass(&HomeostaticRegistry::dev_thermal(), Fixed::from_ratio(1, 2)),
            Physiology::dev_for_registry(&HomeostaticRegistry::dev_thermal()),
            blank.clone(),
        );
        emb.add(walker, thermal_band());
    }
    emb
}

/// Run the UNIFIED tick: a runner carrying both a dawn world and an embodiment whose walkers share the
/// world's ids. Returns the composite (runner state hash, world event-log hash) trace and the move
/// count, so the shared-being sweep exercises the same non-empty dialogue barrier the disjoint sweep
/// does.
fn unified_trace(
    beings: usize,
    bands: usize,
    seed: u64,
    ticks: u64,
    workers: usize,
) -> (Vec<(u128, u128)>, usize) {
    let (mut world, ids) = dawn_world(beings, bands, seed);
    world.set_workers(workers);
    let emb = shared_embodiment(&ids, seed ^ 0x00B0_D1E5);
    let mut runner = Runner::with_world_and_embodiment(field_fixture(), field_calib(), world, emb);
    let mut trace = Vec::with_capacity(ticks as usize);
    for t in 0..ticks {
        runner.step_with_world_inputs(&seed_observations(&ids, t));
        let w = runner.world().expect("the unified runner owns a world");
        // Post-tick the two clocks agree (each advances once per tick), even though the embodiment and
        // cognition draws key on clocks that differ by one within a tick.
        assert_eq!(runner.clock(), w.clock(), "runner and world clocks drifted");
        trace.push((runner.state_hash(), w.event_log_hash()));
    }
    let moves = runner
        .world()
        .expect("the unified runner owns a world")
        .events()
        .len();
    (trace, moves)
}

#[test]
fn the_unified_runner_carries_minds_and_bodies_under_one_id_and_replays() {
    // The crux: one StableId owns both a World mind and an Embodiment walker in one runner. Every dawn
    // being is at once a cognition mind (a world being) and a located body (a body_temp entry and a
    // walker at the same id), the composite replays bit for bit, and it is seed-sensitive.
    let (world, ids) = dawn_world(20, 3, 0x0DD1DEA5);
    let emb = shared_embodiment(&ids, 0x00B0_D1E5);
    let runner = Runner::with_world_and_embodiment(field_fixture(), field_calib(), world, emb);
    assert!(
        runner.world().is_some() && runner.embodiment().is_some(),
        "the unified runner carries both halves"
    );
    let walker_ids: Vec<StableId> = runner
        .embodiment()
        .unwrap()
        .walkers()
        .iter()
        .map(|w| w.id)
        .collect();
    let mind_ids = runner.world().unwrap().being_ids();
    for &id in &ids {
        assert!(mind_ids.contains(&id), "founder {id:?} is a cognition mind");
        assert!(
            walker_ids.contains(&id),
            "founder {id:?} is a located walker"
        );
        assert!(
            runner.body_temp(id).is_some(),
            "founder {id:?} carries a body temperature (a body)"
        );
    }

    let (a, _) = unified_trace(20, 3, 0x5EED1, 60, 1);
    let (b, _) = unified_trace(20, 3, 0x5EED1, 60, 1);
    assert_eq!(a.len(), 60);
    assert_eq!(a, b, "the unified shared-id tick replays bit for bit");
    let (c, _) = unified_trace(20, 3, 0x5EED2, 60, 1);
    assert_ne!(a, c, "a different seed drives a different unified run");
}

#[test]
fn the_unified_runner_is_bit_identical_across_worker_counts() {
    // The first nontrivial exercise of the shared-id path under the parallel scheduler: with a being in
    // both the world and the embodiment, the composite must stay bit-identical at every World worker
    // width. The RES_BEING edge serializes the embodiment and cognition systems in the pinned order,
    // and the world's own command barrier keeps its parallel phases width-invariant.
    let (serial, serial_moves) = unified_trace(40, 5, 0xBEA57, 80, 1);
    assert!(
        serial_moves >= MIN_EXPECTED_MOVES,
        "the unified converse phase produced only {serial_moves} dialogue moves (expected at least \
         {MIN_EXPECTED_MOVES}): the sweep would prove an empty command set identical"
    );
    for workers in [2usize, 3, 8] {
        let (parallel, parallel_moves) = unified_trace(40, 5, 0xBEA57, 80, workers);
        assert_eq!(
            serial, parallel,
            "the unified tick diverged at {workers} workers: a beat leaked the thread schedule"
        );
        assert_eq!(
            serial_moves, parallel_moves,
            "the unified dialogue-move count is not width-invariant at {workers} workers"
        );
    }
}

#[test]
fn the_unified_runner_step_matches_step_scheduled() {
    // With a shared being present, the deterministic scheduler must reproduce the pinned step order bit
    // for bit: the RES_BEING write both the embodiment and the world declare serializes the two systems
    // in canonical order (SYS_EMBODIMENT before SYS_WORLD), the pinned step_inner order, so the composite
    // is identical whether stepped by hand or through the scheduler.
    let build = || {
        let (world, ids) = dawn_world(24, 4, 0x5C4ED);
        let emb = shared_embodiment(&ids, 0x00B0_D1E5);
        let runner = Runner::with_world_and_embodiment(field_fixture(), field_calib(), world, emb);
        (runner, ids)
    };
    let (mut pinned, ids) = build();
    let (mut scheduled, _) = build();
    for t in 0..40u64 {
        pinned.step_with_world_inputs(&seed_observations(&ids, t));
        scheduled.step_scheduled(&seed_observations(&ids, t));
        assert_eq!(
            pinned.state_hash(),
            scheduled.state_hash(),
            "the scheduled order diverged from the pinned order at tick {t} with a shared being"
        );
    }
}

#[test]
#[should_panic(expected = "authored decision repertoire")]
fn the_unified_runner_refuses_an_authored_behaviour_repertoire() {
    // The Principle 9 steering boundary survives into the unified constructor: a world carrying an
    // authored decision repertoire is refused on the with_world_and_embodiment path too.
    let (mut world, ids) = dawn_world(4, 2, 0x5EED);
    world.set_behaviour(Behaviour {
        drives: vec![],
        curves: vec![],
        actions: vec![],
    });
    let emb = shared_embodiment(&ids, 0x1);
    let _ = Runner::with_world_and_embodiment(field_fixture(), field_calib(), world, emb);
}

#[test]
#[should_panic(expected = "authored decision repertoire")]
fn the_canonical_runner_refuses_an_authored_behaviour_repertoire() {
    // The Principle 9 steering boundary the canonical runner holds: an authored drive-and-action
    // repertoire (the sentient deliberative tier, Part 8.1) is steering at the level of behaviour
    // (Part 8.4) and must not ride the canonical-emergent spine, whose behaviour source is the evolved
    // controller. Installing one (even an empty one, which still sets has_behaviour) and composing it
    // onto the canonical runner is a fail-loud steering leak.
    let (mut world, _ids) = dawn_world(4, 2, 0x5EED);
    world.set_behaviour(Behaviour {
        drives: vec![],
        curves: vec![],
        actions: vec![],
    });
    let _ = Runner::with_world(field_fixture(), field_calib(), world);
}

#[test]
fn the_runner_tick_runs_through_the_scheduler_bit_identically() {
    // The deterministic scheduler's first real tick (design Part 57): the runner's phases declared as
    // systems over their resources, scheduled into conflict-free batches, and run through the serial
    // executor must reproduce the hand-pinned tick bit for bit. Two identical composed runners, one
    // stepped in the pinned order and one through the scheduler, must track exactly.
    let build = || {
        let mut runner =
            Runner::with_world(field_fixture(), field_calib(), dawn_world(24, 4, 0x5C4E).0);
        for k in 0..3u64 {
            let coord = Coord3::ground((k as i32) % 8, (k as i32) % 6);
            runner.place_being(StableId(10_000 + k), coord, Fixed::from_int(37));
        }
        runner
    };
    let mut pinned = build();
    let mut scheduled = build();
    for _ in 0..40 {
        pinned.step();
        scheduled.step_scheduled(&[]);
        assert_eq!(
            scheduled.state_hash(),
            pinned.state_hash(),
            "the scheduled tick diverged from the pinned order"
        );
    }
    // The payoff the schedule demonstrates: the cognition world shares no resource with the field
    // phases, so it lands in the first batch alongside the field step (a parallelisable pair), while
    // the field-reading body exchange serialises after into a second batch.
    let sch = civsim_core::schedule::schedule(&pinned.tick_systems());
    assert_eq!(
        sch.len(),
        2,
        "field and world parallelise, the body exchange follows"
    );
    assert_eq!(
        sch[0].len(),
        2,
        "the field step and the independent world tick share the first batch"
    );
    assert_eq!(
        sch[1].len(),
        1,
        "the field-reading body exchange serialises after"
    );
}

/// A body plan bearing a single flesh organ that carries `mat.fracture_strength` (so a corpse of it is
/// worked matter) and `bio.energy_density`, plus the matching organ registry. Labelled test fixture, not
/// owner data: the corpse's physics is DERIVED from these axes, and the numbers are stand-ins.
fn flesh_body_and_registry() -> (BodyPlan, BodyPlanRegistry) {
    let mut registry = BodyPlanRegistry::dev_default();
    let flesh = registry.organs.len() as u16;
    registry.organs.push(OrganKindDef {
        id: flesh,
        name: "flesh".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[
            ("mat.fracture_strength", Fixed::from_int(3)),
            ("bio.energy_density", Fixed::from_int(5)),
        ]),
    });
    let body = BodyPlan {
        body_mass: Fixed::from_ratio(1, 2),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons: vec![],
        covering: Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        },
        senses: vec![],
        locomotion: vec![1],
        organs: vec![Part {
            kind: flesh,
            development: Fixed::ONE,
        }],
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(1, 2),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    };
    (body, registry)
}

/// An embodiment whose walkers share the world ids and each carry the flesh body, with the matching organ
/// registry armed so a death can derive the corpse composition from the body plan.
fn fleshy_embodiment(ids: &[StableId], seed: u64) -> Embodiment {
    let reg = HomeostaticRegistry::dev_thermal();
    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        0,
        seed,
    );
    let (body, organs) = flesh_body_and_registry();
    emb.set_organs(organs);
    let blank = Controller::zeros(emb.layout());
    for (k, &id) in ids.iter().enumerate() {
        let coord = Coord3::ground((k as i32) % 8, (k as i32) % 6);
        let walker = Walker::new(
            id,
            coord,
            body.clone(),
            Homeostasis::from_mass(&reg, Fixed::from_ratio(1, 2)),
            Physiology::dev_for_registry(&reg),
            blank.clone(),
        );
        emb.add(walker, thermal_band());
    }
    emb
}

#[test]
fn a_culled_being_leaves_its_own_body_as_located_matter_when_corpse_matter_is_armed() {
    // Biosphere directive 2, organisms as usable material stuff (the wired demo). With corpse matter armed,
    // culling a mind retires its body AND leaves the body as located tissue where it fell, a composition
    // vector DERIVED from the being's own body plan (its flesh's fracture strength), never a minted
    // substance or an authored species-to-substance map (Principle 8). The corpse is then worked by the SAME
    // extraction contest and the SAME axis (mat.fracture_strength) as any other matter, so a forager could
    // break it down. With corpse matter OFF (the default) nothing is deposited, so the run is byte-identical,
    // and the deposit is deterministic (two armed runs agree bit for bit).
    let build = |armed: bool| -> Runner {
        let (world, ids) = dawn_world(1, 1, 0x0C0F_5EED);
        let emb = fleshy_embodiment(&ids, 0x00B0_D1E5);
        let mut runner =
            Runner::with_world_and_embodiment(field_fixture(), field_calib(), world, emb);
        if armed {
            runner.set_corpse_matter(true);
        }
        // Cull the one mind; the lifecycle reconciliation retires its body next tick.
        let victim = ids[0];
        runner.world_mut().unwrap().remove_being(victim);
        runner
    };

    // ARMED: the culled being leaves located matter derived from its own flesh.
    let mut armed = build(true);
    assert!(
        armed.embodiment().unwrap().tissue().is_empty(),
        "no corpse matter before the tick"
    );
    armed.step();
    let tissue = armed.embodiment().unwrap().tissue();
    assert!(
        !tissue.is_empty(),
        "the culled being left its body as located matter"
    );
    let cell = tissue.cells().next().expect("a corpse cell exists");
    // The corpse carries its OWN body's fracture strength (the flesh's 3), derived by the composition fold,
    // so the death cell is now worked matter a forager must overcome, read through the same axis as rock.
    assert_eq!(
        tissue.fracture_hardness(cell),
        Fixed::from_int(3),
        "the corpse's fracture hardness is its own flesh's, derived not authored"
    );
    assert!(
        tissue.volume_at(cell) > Fixed::ZERO,
        "the corpse deposited a positive volume of its own matter"
    );
    // The body is retired in lockstep (no orphaned body), the referential-integrity invariant, so corpse
    // matter rides alongside the retirement rather than blocking it.
    assert!(
        armed.embodiment().unwrap().walkers().is_empty(),
        "the culled being's body was retired"
    );

    // OFF (default): the same cull deposits nothing, so the tissue field stays empty (byte-neutral opt-in).
    let mut off = build(false);
    off.step();
    assert!(
        off.embodiment().unwrap().tissue().is_empty(),
        "with corpse matter off, a death leaves no located matter (the opt-in flip)"
    );

    // DETERMINISTIC: a second armed run reproduces the deposit and the whole state hash bit for bit.
    let mut armed2 = build(true);
    armed2.step();
    assert_eq!(
        armed.state_hash(),
        armed2.state_hash(),
        "the corpse deposit is deterministic (two armed runs agree bit for bit)"
    );
}
