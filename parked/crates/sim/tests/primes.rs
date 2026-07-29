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

//! The naming game over the full semantic-prime anchor set (design Part 33.9, roadmap M3):
//! a band coordinates a shared starter lexicon over all sixty-five NSM primes, and the run
//! replays bit for bit. Innovation is off so coordination converges cleanly; every number
//! here is a labelled fixture, never an owner value.

use civsim_bio::evidence::InferenceParams;
use civsim_bio::tom::AccessWeights;
use civsim_core::{Fixed, StableId};
use civsim_sim::language::{ArticulationSubstrate, ConceptId, LanguageParams};
use civsim_sim::primes::{nsm_concept_ids, nsm_gloss};
use civsim_sim::world::World;

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

// A band of five at one place, speaking one lineage, innovation off so the naming game
// converges cleanly over the whole prime set.
fn setup(seed: u64) -> (World, Vec<StableId>) {
    let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(seed);
    w.set_concepts(nsm_concept_ids());
    let (_substrate, forms) = ArticulationSubstrate::syllabic(
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
    (w, band)
}

fn all_converged(w: &World, band: &[StableId], concepts: &[ConceptId]) -> bool {
    concepts.iter().all(|&c| {
        let first = w.word_for(band[0], c);
        first.is_some() && band.iter().all(|&m| w.word_for(m, c) == first)
    })
}

#[test]
fn the_band_coordinates_a_word_for_every_prime() {
    let (mut w, band) = setup(0x9417);
    let concepts = nsm_concept_ids();
    let mut converged = false;
    for _ in 0..4000 {
        w.tick(&[]);
        if all_converged(&w, &band, &concepts) {
            converged = true;
            break;
        }
    }
    assert!(
        converged,
        "the band converged a shared word for all {} primes",
        concepts.len()
    );
    // The legibility guarantee: every prime renders a deterministic English gist (33.2).
    for &c in &concepts {
        assert!(nsm_gloss(c).is_some(), "prime {c:?} has no gloss");
    }
}

#[test]
fn the_prime_naming_game_replays_bit_for_bit() {
    let run = |seed: u64| {
        let (mut w, _) = setup(seed);
        for _ in 0..200 {
            w.tick(&[]);
        }
        w.state_hash()
    };
    assert_eq!(run(0xABC), run(0xABC), "same seed replays bit for bit");
    assert_ne!(run(0xABC), run(0xDEF), "a different seed diverges");
}
