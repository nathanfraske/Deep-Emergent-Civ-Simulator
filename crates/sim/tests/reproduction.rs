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

//! The reproduce half of the life cadence (design Parts 25, 28, R-REPRO, world-wiring increment 5,
//! the keystone): within a band, mature compatible beings pair under their heritable mate preference
//! and each pair bears one child, so a run grows as well as shrinks, children inherit both halves of
//! their being, and short- and long-lived races cull on their own timescales. Every number here is a
//! labelled fixture, never an owner value.

use std::collections::BTreeMap;

use civsim_core::{Fixed, GaussApprox};
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, BreedingSystem, BreedingSystemId,
    BreedingSystemRegistry, Channel, CognitionChannel, Curve, DominanceKind, DominanceMode,
    EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme,
    InferenceParams, IntrinsicBeliefs, Race, RaceId, ReproductionMode, ReproductionParams,
    RingCapacityLaw, SchemeId, SourceModeId, ValueAxisId, ValueProfile, World,
};

const AXIS: AxiomAxisId = AxiomAxisId(0);

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

fn dev_ring_law() -> RingCapacityLaw {
    RingCapacityLaw {
        curve: Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::from_int(8), Fixed::from_int(16)),
        ]),
        hard_cap: 32,
    }
}

/// A sex-determined race: two cognition genes and a sex-determination gene whose heterozygote is the
/// heterogametic sex, a three-locus biallelic pool at frequency one half (so a cohort mixes both
/// classes), and a binary anisogamous breeding system. Lifespan and maturity are parameters so a test
/// can build short- and long-lived races. Labelled fixtures, not owner data.
fn sex_determined_race(id: u32, lifespan: u32, maturity: u32) -> Race {
    let genes = GeneSet {
        genes: vec![
            GeneDef {
                id: GeneId(0),
                effects: vec![GeneEffect {
                    channel: Channel::Cognition(CognitionChannel::ReasoningAcuity),
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode::additive(),
            },
            GeneDef {
                id: GeneId(1),
                effects: vec![GeneEffect {
                    channel: Channel::Cognition(CognitionChannel::MemoryCapacity),
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode::additive(),
            },
            GeneDef {
                id: GeneId(2),
                effects: vec![GeneEffect {
                    channel: Channel::SexDetermination,
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode {
                    a: Fixed::ZERO,
                    d: Fixed::ONE,
                    kind: DominanceKind::Complete,
                },
            },
        ],
    };
    let pool = GenePool::new(
        SchemeId(0),
        30,
        vec![
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 2),
        ],
    );
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: Fixed::ZERO,
        additive_mutation_step: Fixed::ZERO,
        gauss: GaussApprox::default(),
    };
    let intrinsic = IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 2)]),
        axioms: vec![Axiom {
            axis: AXIS,
            stance: Fixed::from_ratio(1, 2),
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 4,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 4),
            innate_seed: Fixed::from_ratio(1, 2),
            evidence: EvidenceRing::new(3),
        }],
        epistemic: EpistemicStance::new(
            [(SourceModeId(1), Fixed::ONE)],
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        ),
    };
    Race::new(
        RaceId(id),
        genes,
        pool,
        scheme,
        intrinsic,
        Fixed::from_int(2),
        Fixed::ZERO,
        lifespan,
        maturity,
    )
    .with_breeding(BreedingSystemId(0))
}

fn registry() -> BreedingSystemRegistry {
    let mut reg = BreedingSystemRegistry::new();
    reg.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    reg
}

fn reproduction() -> ReproductionParams {
    ReproductionParams {
        mutation_spread: Fixed::from_ratio(1, 50),
        ring_law: dev_ring_law(),
    }
}

#[test]
fn the_population_grows_children_inherit_and_it_replays() {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), sex_determined_race(0, 80, 18));
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members: 12,
    }];
    let build = |seed: u64| {
        let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(seed);
        w.set_breeding_systems(registry());
        let founders = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
        // Age the founders past maturity so they can pair from the first cadence.
        for &id in &founders {
            w.set_age(id, 20);
        }
        w.set_life_cadence(4);
        w.set_reproduction(reproduction());
        (w, founders)
    };

    let (mut w, founders) = build(0x5EED_0005);
    let before = w.population();
    for _ in 0..24 {
        w.tick(&[]);
    }
    // The population grew: mature pairs bore children the birth beat placed into the band.
    assert!(
        w.population() > before,
        "the population grew from reproduction: {before} -> {}",
        w.population()
    );
    assert!(
        w.census().total_offspring() > 0,
        "offspring were credited to the census"
    );
    assert!(
        w.census().effective_size() > 0,
        "the effective population size tracks the real breeders"
    );

    // A child (a being that is not a founder) inherited both halves of its being: a genome, a mind,
    // and intrinsic beliefs, so lexicons and axioms carry across generations.
    let child = w
        .being_ids()
        .into_iter()
        .find(|id| !founders.contains(id))
        .expect("at least one child was born");
    assert!(w.genome_of(child).is_some(), "the child inherited a genome");
    assert!(w.mind(child).is_some(), "the child expresses a mind");
    assert!(
        w.intrinsic_of(child).is_some(),
        "the child inherited an intrinsic disposition"
    );

    // Bit-for-bit replay, and seed sensitivity.
    let (mut w2, _) = build(0x5EED_0005);
    for _ in 0..24 {
        w2.tick(&[]);
    }
    assert_eq!(
        w.state_hash(),
        w2.state_hash(),
        "the whole reproduction trajectory replays bit for bit"
    );
    assert_eq!(w.population(), w2.population());
    let (mut w3, _) = build(0x5EED_0006);
    for _ in 0..24 {
        w3.tick(&[]);
    }
    assert_ne!(
        w.state_hash(),
        w3.state_hash(),
        "a different seed reproduces a different lineage"
    );
}

#[test]
fn short_and_long_lived_races_cull_on_their_own_timescales() {
    // One life-fraction hazard curve, two races differing only in lifespan: at the same raw age the
    // short-lived race sits at a higher life fraction, so it culls harder. The differential falls out
    // of the per-race lifespan through one curve, no race branch (Principle 9).
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), sex_determined_race(0, 40, 10)); // short-lived
    races.insert(RaceId(1), sex_determined_race(1, 120, 10)); // long-lived
    let bands = [
        BandSpec {
            race: RaceId(0),
            place: 1,
            members: 40,
        },
        BandSpec {
            race: RaceId(1),
            place: 2,
            members: 40,
        },
    ];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0xC0FFEE);
    w.set_breeding_systems(registry());
    let founders = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    // Age every founder to the same raw age of thirty: life fraction 0.75 for the short-lived race,
    // 0.25 for the long-lived one.
    for &id in &founders {
        w.set_age(id, 30);
    }
    w.set_life_cadence(4);
    // A rising life-fraction hazard: negligible young, certain by the end of a lifespan.
    w.set_mortality_hazard_by_race(Curve::new([
        (Fixed::ZERO, Fixed::ZERO),
        (Fixed::ONE, Fixed::ONE),
    ]));

    let count_race = |w: &World, rid: RaceId| -> usize {
        w.being_ids()
            .into_iter()
            .filter(|&id| w.race_of(id) == Some(rid))
            .count()
    };
    let short_before = count_race(&w, RaceId(0));
    let long_before = count_race(&w, RaceId(1));
    // One cadence of aging and mortality (no reproduction armed, so this isolates the cull).
    w.tick(&[]);
    for _ in 0..3 {
        w.tick(&[]);
    }
    let short_after = count_race(&w, RaceId(0));
    let long_after = count_race(&w, RaceId(1));
    let short_dead = short_before - short_after;
    let long_dead = long_before - long_after;
    assert!(
        short_dead > long_dead,
        "the short-lived race culls harder at the same raw age: {short_dead} vs {long_dead}"
    );
}
