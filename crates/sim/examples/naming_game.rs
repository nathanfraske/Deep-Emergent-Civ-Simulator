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
//! A band coordinating its first words over the semantic primes, printed so the emergence
//! is visible. Run with: `cargo run -p civsim-sim --example naming_game`.
//!
//! Five minds at one place play the naming game over all sixty-five NSM primes with
//! innovation off, until they share a word for every prime, and the emergent lexicon is
//! printed as gloss and coined word. Every number here is a labelled fixture, never an
//! owner value, and the run is keyed on a seed so it replays identically.

use civsim_bio::evidence::InferenceParams;
use civsim_bio::tom::AccessWeights;
use civsim_core::{Fixed, StableId};
use civsim_sim::language::{ArticulationSubstrate, LanguageParams};
use civsim_sim::primes::{nsm_concept_ids, nsm_gloss};
use civsim_sim::world::World;

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

fn main() {
    let seed = 0x5A1Du64;
    let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(seed);
    w.set_concepts(nsm_concept_ids());
    let (substrate, forms) = ArticulationSubstrate::syllabic(
        [
            "ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo", "ha", "du", "pe", "go",
        ]
        .map(String::from),
        2,
        3,
    );
    w.set_form_system(forms);
    w.set_language(LanguageParams {
        innovation_rate: Fixed::ZERO,
    });
    let band: Vec<StableId> = (0..5).map(|_| w.spawn(Fixed::ONE)).collect();
    for &m in &band {
        w.set_place(m, 1);
    }

    let concepts = nsm_concept_ids();
    let converged = |w: &World| {
        concepts.iter().all(|&c| {
            let first = w.word_for(band[0], c);
            first.is_some() && band.iter().all(|&m| w.word_for(m, c) == first)
        })
    };

    println!("Five minds at one place, seed {seed:#x}, coordinating their first words.");
    println!(
        "Playing the naming game over {} semantic primes, innovation off.\n",
        concepts.len()
    );

    let mut ticks = 0;
    while ticks < 4000 && !converged(&w) {
        w.tick(&[]);
        ticks += 1;
    }

    println!(
        "After {ticks} ticks the band shares a word for every prime: {}\n",
        converged(&w)
    );
    println!("The band's first lexicon (English gist, coined word):");
    for &c in &concepts {
        let gloss = nsm_gloss(c).unwrap_or("?");
        let word = w
            .word_for(band[0], c)
            .map(|word| substrate.render(&word))
            .unwrap_or_else(|| "-".to_string());
        println!("  {gloss:<14} {word}");
    }

    // Determinism, shown rather than asserted: the same seed reproduces the same world.
    let replay = {
        let mut w2 = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(seed);
        w2.set_concepts(nsm_concept_ids());
        let (_s, forms2) = ArticulationSubstrate::syllabic(
            [
                "ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo", "ha", "du", "pe", "go",
            ]
            .map(String::from),
            2,
            3,
        );
        w2.set_form_system(forms2);
        w2.set_language(LanguageParams {
            innovation_rate: Fixed::ZERO,
        });
        let band2: Vec<StableId> = (0..5).map(|_| w2.spawn(Fixed::ONE)).collect();
        for &m in &band2 {
            w2.set_place(m, 1);
        }
        for _ in 0..ticks {
            w2.tick(&[]);
        }
        w2.state_hash()
    };
    println!("\nDeterminism:");
    println!("  state hash this run : {:032x}", w.state_hash());
    println!("  same seed replayed  : {replay:032x}");
    println!("  matches: {}", w.state_hash() == replay);
}
