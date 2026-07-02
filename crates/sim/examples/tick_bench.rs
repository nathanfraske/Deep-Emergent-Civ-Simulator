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

//! QUARANTINED DEV-FIXTURE HARNESS (not canonical). This example uses authored, dev-fixture numbers
//! (calibrations, seeds, scenario values) to produce a result for demonstration and testing only, and
//! its behaviour is not authoritative (design Principle 11, the reserved-value discipline: an authored
//! constant in the path of world content is a defect until it earns its place). The canonical runner
//! is manifest-driven and fail-loud with zero unapproved authored features; see docs/QUARANTINE.md.
//!
//! A tick-cost profiler: seed a population into a running `World` and time the serial tick over
//! many iterations, so we have a real baseline before and after a scheduler change. Run with
//! `cargo run --release -p civsim-sim --example tick_bench -- [beings] [ticks] [bands]`.
//!
//! The timing uses `std::time::Instant`, which is view-side and non-canonical (it never enters
//! simulation state), so it does not perturb determinism: the reported `state_hash` is a pure
//! function of the seed and the tick count, and the wall-clock is measured around it.

use std::time::Instant;

use civsim_core::{Fixed, StableId};
use civsim_sim::dialogue::{
    EffectSign, ForceEffectDef, ForceEffectId, ForceFloor, ForceKind, MoveKindDef, MoveKindId,
    MoveRegistry,
};
use civsim_sim::evidence::InferenceParams;
use civsim_sim::language::{ArticulationSubstrate, LanguageParams};
use civsim_sim::primes::nsm_concept_ids;
use civsim_sim::tom::{AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights};
use civsim_sim::world::{GossipParams, World};

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

/// A small dialogue substrate (assert / accept / refuse), so the converse phase has work to do.
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

fn parse<T: std::str::FromStr>(arg: Option<&String>, default: T) -> T {
    arg.and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let beings: usize = parse(argv.get(1), 100);
    let ticks: u64 = parse(argv.get(2), 10_000);
    let bands: usize = parse(argv.get(3), 10).max(1);
    let seed = 0xB1A5_u64;

    // Build the dawn world with language, dialogue, and gossip installed, so a tick exercises the
    // full phase sequence (perceive, decide, converse, gossip, converse_language, drift_languages).
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

    // Seed `beings` minds spread across `bands` co-located groups, so gossip, the naming game, and
    // dialogue all have partners in each place. Promote everyone to move-by-move dialogue so the
    // heaviest phase (converse) runs.
    let ids: Vec<StableId> = (0..beings).map(|_| w.spawn(Fixed::ONE)).collect();
    for (i, &m) in ids.iter().enumerate() {
        w.set_place(m, (i % bands) as u32);
        w.promote(m);
    }

    let warmup = (ticks / 20).max(1);
    println!(
        "tick_bench: {beings} beings in {bands} bands, {ticks} ticks (warmup {warmup}), seed {seed:#x}"
    );

    // Warm up (page in, converge the naming game) without counting it.
    for _ in 0..warmup {
        w.tick(&[]);
    }

    let phase_names = [
        "perceive", "decide", "converse", "gossip", "language", "drift",
    ];
    let mut phase_ns = [0u128; 6];
    let start = Instant::now();
    for _ in 0..ticks {
        let ns = w.tick_timed();
        for (acc, n) in phase_ns.iter_mut().zip(ns) {
            *acc += n;
        }
    }
    let elapsed = start.elapsed();

    let secs = elapsed.as_secs_f64();
    let per_tick_us = secs * 1.0e6 / ticks as f64;
    let ticks_per_sec = ticks as f64 / secs;
    // At the owner-set base tick (one in-world second per tick), one in-world year is
    // civsim_sim::LIFE_CADENCE_TICKS ticks; report how long 100 in-world years would take at this
    // full per-tick rate, the number that motivates temporal LOD and significance-driven fidelity.
    let year_ticks = civsim_sim::LIFE_CADENCE_TICKS as f64;
    let hundred_years_secs = 100.0 * year_ticks / ticks_per_sec;

    println!("  clock reached      : {}", w.clock());
    println!("  wall time          : {secs:.3} s for {ticks} ticks");
    println!("  per tick           : {per_tick_us:.2} us");
    println!("  ticks per second   : {ticks_per_sec:.0}");
    let total_ns: u128 = phase_ns.iter().sum();
    println!("  per-phase breakdown (share of tick time):");
    for (name, ns) in phase_names.iter().zip(phase_ns) {
        let share = if total_ns > 0 {
            100.0 * ns as f64 / total_ns as f64
        } else {
            0.0
        };
        let us_per_tick = ns as f64 / 1000.0 / ticks as f64;
        println!("    {name:<9} {share:5.1}%   {us_per_tick:7.2} us/tick");
    }
    println!(
        "  100 in-world years : {:.0} s ({:.1} h) at full per-tick fidelity (why temporal LOD exists)",
        hundred_years_secs,
        hundred_years_secs / 3600.0
    );
    println!("  world state hash   : {:032x}", w.state_hash());
}
