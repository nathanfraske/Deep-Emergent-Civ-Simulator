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

//! The dawn of sentience seeding: the convergence point where the map, the genome, the value
//! substrate, and the axiom kernel first run together (design Part 28).

use std::collections::BTreeMap;

use civsim_core::Fixed;
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, Channel, CognitionChannel, DominanceMode,
    EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme,
    InferenceParams, IntrinsicBeliefs, Race, RaceId, ReproductionMode, SchemeId, SourceModeId,
    ValueAxisId, ValueProfile, World,
};

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

/// A race carrying two cognition genes (acuity, memory), a two-locus biallelic pool, and an
/// innate disposition (one value axis, one axiom, an evidence-weighted epistemic stance). The
/// environment baseline is 2; pool-promoted genomes carry zero additive, and the genes are
/// additive, so a member's expressed acuity equals that baseline.
fn a_race(id: u32) -> Race {
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
        ],
    };
    let pool = GenePool::new(
        SchemeId(0),
        20,
        vec![Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2)],
    );
    let intrinsic = IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 3)]),
        axioms: vec![Axiom {
            axis: AxiomAxisId(0),
            stance: Fixed::from_ratio(1, 2),
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 5,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 4),
            innate_seed: Fixed::from_ratio(1, 2),
            evidence: EvidenceRing::new(4),
        }],
        epistemic: EpistemicStance::new(
            [(SourceModeId(1), Fixed::ONE)],
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        ),
    };
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: Fixed::ZERO,
    };
    Race::new(
        RaceId(id),
        genes,
        pool,
        scheme,
        intrinsic,
        Fixed::from_int(2),
    )
}

#[test]
fn the_dawn_seeds_bands_with_genomes_minds_beliefs_and_places() {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_race(0));
    races.insert(RaceId(1), a_race(1));
    let bands = [
        BandSpec {
            race: RaceId(0),
            place: 10,
            members: 2,
        },
        BandSpec {
            race: RaceId(1),
            place: 20,
            members: 3,
        },
    ];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0xDA0D);
    let seeded = w.seed_dawn_populations(&races, &bands);

    assert_eq!(seeded.len(), 5, "two plus three members were seeded");
    assert_eq!(w.population(), 5);

    for (i, &id) in seeded.iter().enumerate() {
        let genome = w.genome_of(id).expect("a genome was seeded");
        assert_eq!(genome.haps.len(), 2, "diploid promotion");
        let intr = w.intrinsic_of(id).expect("intrinsic beliefs were seeded");
        assert_eq!(intr.axioms.len(), 1, "the race's axiom was seeded");
        let mind = w.mind(id).expect("a mind was expressed");
        assert_eq!(
            mind.acuity,
            Fixed::from_int(2),
            "acuity rides the environment baseline"
        );
        let expected_place = if i < 2 { 10 } else { 20 };
        assert_eq!(w.place_of(id), Some(expected_place), "placed with its band");
    }
}

#[test]
fn the_dawn_is_deterministic_in_the_genomes_it_draws() {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_race(0));
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members: 4,
    }];
    let draw = || {
        let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0xBEEF);
        let ids = w.seed_dawn_populations(&races, &bands);
        ids.iter()
            .map(|&id| w.genome_of(id).unwrap().clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        draw(),
        draw(),
        "the same seed and bands draw the same genomes"
    );
}

#[test]
fn an_unknown_race_band_is_skipped() {
    let races: BTreeMap<RaceId, Race> = BTreeMap::new();
    let bands = [BandSpec {
        race: RaceId(99),
        place: 1,
        members: 3,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(1);
    let seeded = w.seed_dawn_populations(&races, &bands);
    assert!(
        seeded.is_empty(),
        "a band whose race is not registered is skipped"
    );
    assert_eq!(w.population(), 0);
}
