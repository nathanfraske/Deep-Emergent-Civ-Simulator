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

//! A deep end-to-end lifecycle: dawn seeding, then generations of bounded-confidence
//! enculturation (schism), calcification, and births, exercised together and proven to replay
//! bit for bit. This is the integration the genome (Part 25), value (Part 21), and axiom (Part
//! 28) work was building toward.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId};
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, Channel, CognitionChannel, Curve, DominanceMode,
    EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme,
    InferenceParams, IntrinsicBeliefs, Race, RaceId, ReproductionMode, RingCapacityLaw, SchemeId,
    SourceModeId, ValueAxisId, ValueProfile, World,
};

const AXIS: AxiomAxisId = AxiomAxisId(0);

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

/// A labelled test ring-capacity law (not owner data): a linear memory-to-slots curve and a
/// ceiling, used to size the beings' evidence rings from their expressed memory.
fn dev_ring_law() -> RingCapacityLaw {
    RingCapacityLaw {
        curve: Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::from_int(8), Fixed::from_int(16)),
        ]),
        hard_cap: 32,
    }
}

fn beliefs(stance: Fixed) -> IntrinsicBeliefs {
    IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 1)]),
        axioms: vec![Axiom {
            axis: AXIS,
            stance,
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 1,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 8),
            innate_seed: stance,
            evidence: EvidenceRing::new(3),
        }],
        epistemic: EpistemicStance::new(
            [(SourceModeId(1), Fixed::ONE)],
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        ),
    }
}

fn a_race() -> Race {
    let genes = GeneSet {
        genes: vec![GeneDef {
            id: GeneId(0),
            effects: vec![GeneEffect {
                channel: Channel::Cognition(CognitionChannel::ReasoningAcuity),
                weight: Fixed::ONE,
            }],
            dominance: DominanceMode::additive(),
        }],
    };
    let pool = GenePool::new(SchemeId(0), 20, vec![Fixed::from_ratio(1, 2)]);
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: Fixed::ZERO,
        additive_mutation_step: Fixed::ZERO,
        gauss: civsim_core::GaussApprox::default(),
    };
    Race::new(
        RaceId(0),
        genes,
        pool,
        scheme,
        beliefs(Fixed::ZERO),
        Fixed::from_int(2),
        // Homogeneous developmental environment (V_E zero): reproduces the pre-V_E dawn.
        Fixed::ZERO,
        // Fixture lifespan and maturity in life-cadence steps (labelled test values, not owner data).
        80,
        18,
    )
}

/// Run the whole lifecycle and return a digest of its final state: every founder's stance and
/// entrenchment, the sect count, the total population, and the first child's inherited seed.
fn lifecycle(seed: u64) -> (Vec<(Fixed, i32)>, usize, usize, Fixed) {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_race());
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members: 5,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(seed);
    let founders = w.seed_dawn_populations(&races, &bands, &dev_ring_law());

    // Spread the founders' stances across the axis so there is something to enculturate and
    // fracture (the dawn seeds an identical disposition; here we set a starting spread).
    let spread = [
        Fixed::ZERO,
        Fixed::from_ratio(1, 10),
        Fixed::from_ratio(1, 2),
        Fixed::from_ratio(9, 10),
        Fixed::ONE,
    ];
    for (&id, &s) in founders.iter().zip(spread.iter()) {
        w.set_intrinsic(id, beliefs(s));
    }

    let race = a_race();
    let epsilon = Fixed::from_ratio(1, 5);
    let mut first_child_seed = Fixed::ZERO;
    for generation in 0..6u64 {
        // Bounded-confidence enculturation: clusters tighten, far ones never merge.
        w.enculturate_band_bounded(&founders, AXIS, epsilon);
        // Unchallenged axioms calcify toward a cap.
        w.calcify_band(&founders, AXIS, 1, 5);
        // A birth each generation from the first two founders and the band.
        if let Some(child) = w.birth(
            &race,
            founders[0],
            founders[1],
            &founders,
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 50),
            generation,
            &dev_ring_law(),
        ) {
            if generation == 0 {
                first_child_seed = w.intrinsic_of(child).unwrap().axioms[0].innate_seed;
            }
        }
    }

    let final_stances: Vec<(Fixed, i32)> = founders
        .iter()
        .map(|&id| {
            let ax = &w.intrinsic_of(id).unwrap().axioms[0];
            (ax.stance, ax.entrenchment)
        })
        .collect();
    let sects = w.stance_clusters(&founders, AXIS, epsilon).len();
    (final_stances, sects, w.population(), first_child_seed)
}

#[test]
fn the_whole_lifecycle_replays_bit_for_bit() {
    assert_eq!(
        lifecycle(0xDA2C),
        lifecycle(0xDA2C),
        "dawn, schism, calcification, and births all replay identically"
    );
}

#[test]
fn the_lifecycle_produces_its_emergent_signatures() {
    let (stances, sects, population, _seed) = lifecycle(0xDA2C);
    // Six births grew the population from five founders to eleven.
    assert_eq!(population, 11, "five founders plus six births");
    // Calcification ran six quiet phases at rate 1 to a cap of 5, so the founders hardened.
    for (_stance, entrenchment) in &stances {
        assert_eq!(*entrenchment, 5, "the founders calcified to the cap");
    }
    // The starting spread did not collapse to one stance: the band holds more than one sect.
    assert!(
        sects >= 2,
        "bounded confidence kept the band fractured into sects"
    );
}

/// Run a generational-turnover loop and return `(births, deaths, final_population)`. Each
/// generation the band ages one life-cadence step, the two eldest survivors may bear a child, and
/// an age-hazard mortality pass culls the old. This exercises the R-AGING life-process loop
/// (`age_step`, `apply_mortality`, `remove_being`) together with the full birth integration.
fn turnover(seed: u64) -> (usize, usize, usize) {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_race());
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members: 5,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(seed);
    let founders = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    // Age the founders across the lifespan so the eldest face real mortality from the start.
    for (k, &id) in founders.iter().enumerate() {
        w.set_age(id, 20 + 18 * k as u32); // 20, 38, 56, 74, 92
    }
    // A rising hazard: certain survival young, certain death by age 100. The data-driven
    // default, non-final; the owner sets the real curve.
    let hazard = Curve::new([
        (Fixed::ZERO, Fixed::ZERO),
        (Fixed::from_int(100), Fixed::ONE),
    ]);
    let race = a_race();
    let mut roster: Vec<StableId> = founders.clone();
    let mut births = 0usize;
    let mut deaths = 0usize;
    for generation in 0..10u64 {
        w.age_step();
        // The living roster (a being is alive while its age is still tracked): two of them bear
        // a child into the same band.
        let alive: Vec<StableId> = roster
            .iter()
            .copied()
            .filter(|&id| w.age_of(id).is_some())
            .collect();
        if alive.len() >= 2 {
            if let Some(child) = w.birth(
                &race,
                alive[0],
                alive[1],
                &alive,
                Fixed::from_ratio(1, 2),
                Fixed::from_ratio(1, 50),
                generation,
                &dev_ring_law(),
            ) {
                roster.push(child);
                births += 1;
            }
        }
        deaths += w.apply_mortality(&hazard).len();
    }
    (births, deaths, w.population())
}

#[test]
fn the_being_model_turns_over_generations_and_replays() {
    let (births, deaths, population) = turnover(0x70E2);
    assert!(births > 0, "the band bore children");
    assert!(deaths > 0, "the eldest died: the population turned over");
    // The population identity holds across every birth and death: no being is double-counted or
    // leaked, which is the referential-integrity guarantee remove_being upholds (Part 58).
    assert_eq!(
        population,
        5 + births - deaths,
        "population = founders + births - deaths"
    );
    // The whole turnover loop replays bit for bit (the aging and mortality draws are keyed on the
    // being and its age, the birth draws on the parents and the generation).
    assert_eq!(
        turnover(0x70E2),
        (births, deaths, population),
        "the generational turnover replays identically"
    );
}

#[test]
fn two_seeds_differ_only_where_rng_enters() {
    // The lifecycle is seeded only through the genome draws (births), so two seeds give the
    // same founder stances and sect structure (no RNG there) but can differ in child genomes.
    let a = lifecycle(0x1111);
    let b = lifecycle(0x2222);
    assert_eq!(
        a.0, b.0,
        "founder stances are RNG-free and seed-independent"
    );
    assert_eq!(a.1, b.1, "the sect structure is seed-independent");
    assert_eq!(a.2, b.2, "the population count matches");
}
