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

//! Regular form change (design 33.4, the R-LANG-DET core): a band converges a word and then
//! it drifts over generations, and two sister lineages forked from one ancestor diverge into
//! cognates, all deterministically. Every number here is a clearly-labelled fixture, never an
//! owner value.

use civsim_core::{Fixed, StableId};
use civsim_sim::evidence::InferenceParams;
use civsim_sim::language::{ArticulationSubstrate, ConceptId, DriftParams, LangId, LanguageParams};
use civsim_sim::tom::AccessWeights;
use civsim_sim::world::World;

const CONCEPT: ConceptId = ConceptId(1);

// Fixture cognition params: the language tests do not exercise belief, so these are inert.
fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

fn drift_params() -> DriftParams {
    // FIXTURE: a change every generation, a short generation, so drift is visible in a test run.
    DriftParams {
        sound_change_rate: Fixed::ONE,
        generation_ticks: 4,
    }
}

// A band of four at one place, speaking one lineage, with innovation off so the naming game
// converges cleanly. Drift is left unset for the caller to switch on after convergence.
fn setup(seed: u64) -> (World, Vec<StableId>, ArticulationSubstrate) {
    let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(seed);
    w.set_concepts([CONCEPT]);
    let (substrate, forms) = ArticulationSubstrate::syllabic(
        ["ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo"].map(String::from),
        2,
        3,
    );
    w.set_form_system(forms);
    w.set_language(LanguageParams {
        innovation_rate: Fixed::ZERO,
    });
    let band: Vec<StableId> = (0..4).map(|_| w.spawn(Fixed::ONE)).collect();
    for &m in &band {
        w.set_place(m, 1);
    }
    (w, band, substrate)
}

#[test]
fn a_word_drifts_over_generations_and_the_lineage_drifts_together() {
    let (mut w, band, sub) = setup(0xD1F7);
    // Converge a shared word with drift off.
    for _ in 0..20 {
        w.tick(&[]);
    }
    let early = w.word_for(band[0], CONCEPT).map(|x| sub.render(&x));
    assert!(early.is_some(), "the band coined a word");
    for &m in &band {
        assert_eq!(
            w.word_for(m, CONCEPT).map(|x| sub.render(&x)),
            early,
            "the band converged before drift"
        );
    }
    // Switch drift on and run several generations.
    w.set_drift(drift_params());
    for _ in 0..40 {
        w.tick(&[]);
    }
    let late = w.word_for(band[0], CONCEPT).map(|x| sub.render(&x));
    assert_ne!(early, late, "the word drifted over generations");
    // The whole lineage drifts as a unit: all speakers still share one word.
    let w0 = w.word_for(band[0], CONCEPT);
    for &m in &band {
        assert_eq!(w.word_for(m, CONCEPT), w0, "the lineage drifts together");
    }
}

#[test]
fn two_sisters_forked_from_one_ancestor_diverge_into_cognates() {
    let (mut w, band, sub) = setup(0x515);
    // Converge the ancestor word with drift off.
    for _ in 0..20 {
        w.tick(&[]);
    }
    let ancestor = w.word_for(band[0], CONCEPT).map(|x| sub.render(&x));
    assert!(ancestor.is_some(), "the ancestor lineage coined a word");

    // Fork the ancestor lineage into two daughters and split the band between them.
    let (l1, l2) = {
        let l0 = w.lineage(LangId(0)).expect("ancestor lineage");
        (l0.fork(LangId(1)), l0.fork(LangId(2)))
    };
    w.add_language(l1);
    w.add_language(l2);
    for &m in &band[..2] {
        w.set_language_of(m, LangId(1));
        w.set_place(m, 10);
    }
    for &m in &band[2..] {
        w.set_language_of(m, LangId(2));
        w.set_place(m, 20);
    }

    // Drift the two lineages independently.
    w.set_drift(drift_params());
    for _ in 0..40 {
        w.tick(&[]);
    }
    let w1 = w.word_for(band[0], CONCEPT).map(|x| sub.render(&x));
    let w2 = w.word_for(band[2], CONCEPT).map(|x| sub.render(&x));
    assert!(w1.is_some() && w2.is_some(), "each sister kept a word");
    assert_ne!(w1, w2, "the sisters drifted apart");
    // The family tree is reconstructable: both daughters point at the ancestor.
    assert_eq!(w.lineage(LangId(1)).unwrap().parent(), Some(LangId(0)));
    assert_eq!(w.lineage(LangId(2)).unwrap().parent(), Some(LangId(0)));
}

#[test]
fn drift_replays_bit_for_bit() {
    let run = |seed: u64| {
        let (mut w, _band, _sub) = setup(seed);
        w.set_drift(drift_params());
        for _ in 0..40 {
            w.tick(&[]);
        }
        w.state_hash()
    };
    assert_eq!(run(0xABC), run(0xABC), "drift replays bit for bit");
    assert_ne!(
        run(0xABC),
        run(0xDEF),
        "a different seed drifts differently"
    );
}
