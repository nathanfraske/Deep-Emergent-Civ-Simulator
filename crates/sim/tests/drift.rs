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
//! cognates, all deterministically. The drift cadence is no longer a global scalar: it derives
//! per lineage from the speaking race's maturity against the world's life cadence, so two lineages
//! of races with different `maturity_years` drift on different cadences from one mechanism. Every
//! number here is a clearly-labelled fixture, never an owner value.

use std::collections::BTreeMap;

use civsim_core::{Fixed, GaussApprox, StableId};
use civsim_sim::evidence::InferenceParams;
use civsim_sim::language::{
    ArticulationSubstrate, ConceptId, DriftParams, LangId, Language, LanguageParams,
};
use civsim_sim::tom::AccessWeights;
use civsim_sim::world::World;
use civsim_sim::{
    EpistemicStance, GenePool, GeneSet, GeneticScheme, IntrinsicBeliefs, Race, RaceId,
    ReproductionMode, SchemeId, ValueProfile,
};

const CONCEPT: ConceptId = ConceptId(1);

// The world's life cadence for these tests: a short orbital year in ticks, so a small maturity
// gives a short generation and drift is visible in a test run. A fixture, never an owner value.
const LIFE_CADENCE: u64 = 4;

// Fixture cognition params: the language tests do not exercise belief, so these are inert.
fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

fn drift_params() -> DriftParams {
    // FIXTURE: a change every generation. The generation length is no longer carried here; it
    // derives per lineage from the speaking race's maturity (see `race_with_maturity`).
    DriftParams {
        sound_change_rate: Fixed::ONE,
    }
}

// A minimal race carrying only the datum drift reads, its `maturity_years`. The generation length
// is that maturity times the world's life cadence, so a maturity of one on a life cadence of four
// gives a four-tick generation. Everything else is an inert fixture, never an owner value.
fn race_with_maturity(id: u32, maturity_years: u32) -> Race {
    Race::new(
        RaceId(id),
        GeneSet { genes: vec![] },
        GenePool::new(SchemeId(0), 1, vec![]),
        GeneticScheme {
            id: SchemeId(0),
            reproduction: ReproductionMode::Haploid,
            linkage_groups: vec![],
            mutation_rate: Fixed::ZERO,
            additive_mutation_step: Fixed::ZERO,
            gauss: GaussApprox::default(),
        },
        IntrinsicBeliefs {
            values: ValueProfile::new(),
            axioms: vec![],
            epistemic: EpistemicStance::new([], Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, Fixed::ZERO),
        },
        Fixed::ZERO,
        Fixed::ZERO,
        maturity_years.max(1).saturating_mul(4), // a lifespan comfortably past maturity
        maturity_years,
    )
}

// A band of four at one place, speaking one lineage, with innovation off so the naming game
// converges cleanly. The default lineage belongs to race 0 (maturity 1), so its drift cadence is
// one times the life cadence (four ticks). Drift is left unset for the caller to switch on after
// convergence.
fn setup(seed: u64) -> (World, Vec<StableId>, ArticulationSubstrate) {
    let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(seed);
    w.set_life_cadence(LIFE_CADENCE);
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), race_with_maturity(0, 1));
    w.set_races(races);
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

#[test]
fn two_lineages_of_races_with_different_maturity_drift_on_different_cadences() {
    // The non-steering derivation (Principle 9): the drift cadence is no longer one global scalar
    // (DriftParams carries none, compile-enforced). Two lineages of races identical but for their
    // maturity drift on different cadences from the ONE mechanism, purely from their data. With the
    // world's life cadence at four ticks, a maturity of one gives a four-tick generation and a
    // maturity of two an eight-tick generation.
    let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(0xCADE);
    w.set_life_cadence(LIFE_CADENCE);
    let mut races = BTreeMap::new();
    races.insert(RaceId(1), race_with_maturity(1, 1));
    races.insert(RaceId(2), race_with_maturity(2, 2));
    w.set_races(races);
    let (_sub, forms) = ArticulationSubstrate::syllabic(
        ["ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo"].map(String::from),
        2,
        3,
    );
    // Two lineages, one per race, drifting at rate one (one change per generation).
    w.add_language(Language::new(LangId(1), RaceId(1), forms.clone()));
    w.add_language(Language::new(LangId(2), RaceId(2), forms));
    w.set_drift(drift_params());
    for _ in 0..40 {
        w.tick(&[]);
    }
    let fast = w.lineage(LangId(1)).unwrap().change_log().len();
    let slow = w.lineage(LangId(2)).unwrap().change_log().len();
    // The faster-maturing race's lineage beats its shorter generation more often, accruing more
    // regular form changes over the same span: cadence four fires at ticks 4, 8, ... 40 (ten
    // generations), cadence eight at ticks 8, 16, ... 40 (five).
    assert_eq!(
        fast, 10,
        "the four-tick cadence fires ten times over forty ticks"
    );
    assert_eq!(
        slow, 5,
        "the eight-tick cadence fires five times over forty ticks"
    );
    assert!(
        fast > slow,
        "different maturity, different cadence, from one mechanism ({fast} vs {slow})"
    );

    // A lineage whose race is absent from the registry has no maturity to derive a cadence from and
    // does not drift: a fabricated cadence is never invented (Principle 11).
    let mut w2 = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(0xCADE);
    w2.set_life_cadence(LIFE_CADENCE);
    let (_s, forms2) =
        ArticulationSubstrate::syllabic(["ka", "lo", "mi", "tu"].map(String::from), 2, 3);
    w2.add_language(Language::new(LangId(1), RaceId(9), forms2)); // race 9 is not registered
    w2.set_drift(drift_params());
    for _ in 0..40 {
        w2.tick(&[]);
    }
    assert_eq!(
        w2.lineage(LangId(1)).unwrap().change_log().len(),
        0,
        "a lineage whose race is unregistered does not drift on a fabricated cadence"
    );
}

#[test]
fn the_state_hash_folds_each_lineage_race_maturity_and_lifespan() {
    // Defect 7: the per-lineage maturity_years drives the drift cadence and lifespan_years the
    // mortality age normalization, so two worlds whose lineages age or drift on different schedules
    // are different worlds. Both fold into state_hash alongside the global life cadence, so a change
    // to a race's maturity or lifespan surfaces in the fingerprint at once, before any drift or
    // mortality has run to diverge the observable state. No ticks run here, so the ONLY difference
    // between two builds is the folded race data.
    let build = |maturity: u32, lifespan: u32| -> u128 {
        let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(0xF01D);
        w.set_life_cadence(LIFE_CADENCE);
        let mut races = BTreeMap::new();
        let mut race = race_with_maturity(1, maturity);
        race.lifespan_years = lifespan;
        races.insert(RaceId(1), race);
        w.set_races(races);
        let (_s, forms) =
            ArticulationSubstrate::syllabic(["ka", "lo", "mi", "tu"].map(String::from), 2, 3);
        w.add_language(Language::new(LangId(1), RaceId(1), forms));
        w.state_hash()
    };
    let base = build(2, 40);
    assert_eq!(base, build(2, 40), "identical race data hashes the same");
    assert_ne!(
        base,
        build(3, 40),
        "a divergent maturity surfaces in the fingerprint at once"
    );
    assert_ne!(
        base,
        build(2, 50),
        "a divergent lifespan surfaces in the fingerprint at once"
    );
}
