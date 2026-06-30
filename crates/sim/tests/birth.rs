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

//! The full birth integration (design Parts 25 and 28): a child of two parents inherits both
//! halves of its being, a recombined genome and an expressed mind, plus inherited intrinsic
//! beliefs, all deterministically.

use std::collections::BTreeMap;

use civsim_core::Fixed;
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, Channel, CognitionChannel, DominanceMode,
    EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme,
    InferenceParams, IntrinsicBeliefs, Race, RaceId, ReproductionMode, SchemeId, SourceModeId,
    ValueAxisId, ValueProfile, World,
};

const AXIS: AxiomAxisId = AxiomAxisId(0);

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

fn a_race() -> Race {
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
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: Fixed::ZERO,
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
        RaceId(0),
        genes,
        pool,
        scheme,
        intrinsic,
        Fixed::from_int(2),
    )
}

/// Seed two parents of one race onto a place and return the world, the race, and the parents.
fn dawn_pair(seed: u64) -> (World, Race, civsim_core::StableId, civsim_core::StableId) {
    let race = a_race();
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_race());
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members: 2,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(seed);
    let parents = w.seed_dawn_populations(&races, &bands);
    (w, race, parents[0], parents[1])
}

#[test]
fn a_birth_gives_the_child_a_genome_a_mind_and_inherited_beliefs() {
    let (mut w, race, pa, pb) = dawn_pair(0x5EED);
    let before = w.population();
    let child = w
        .birth(
            &race,
            pa,
            pb,
            &[pa, pb],
            Fixed::from_ratio(1, 2),
            Fixed::ZERO,
            1,
        )
        .unwrap();
    assert_ne!(child, pa);
    assert_ne!(child, pb);
    assert_eq!(w.population(), before + 1, "the child is a new mind");
    // The genome is recombined from two parents: a diploid child.
    let genome = w.genome_of(child).expect("the child has a genome");
    assert_eq!(genome.haps.len(), 2, "diploid offspring");
    // The mind is expressed from the child's genes (acuity rides the race's baseline of 2).
    let mind = w.mind(child).expect("the child has a mind");
    assert_eq!(mind.acuity, Fixed::from_int(2));
    // The intrinsic beliefs are inherited (one axiom).
    let intr = w.intrinsic_of(child).expect("the child has beliefs");
    assert_eq!(intr.axioms.len(), 1);
}

#[test]
fn birth_replays_deterministically() {
    let run = || {
        let (mut w, race, pa, pb) = dawn_pair(0xC0FFEE);
        let child = w
            .birth(
                &race,
                pa,
                pb,
                &[pa, pb],
                Fixed::from_ratio(1, 2),
                Fixed::from_ratio(1, 20),
                1,
            )
            .unwrap();
        let genome = w.genome_of(child).unwrap().clone();
        let seed = w.intrinsic_of(child).unwrap().axioms[0].innate_seed;
        (genome, seed)
    };
    assert_eq!(
        run(),
        run(),
        "the same parents and seed bear the same child"
    );
}

#[test]
fn a_parent_without_a_genome_cannot_bear() {
    let race = a_race();
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(1);
    // Spawned minds have no genome (only dawn-seeded or born beings do).
    let a = w.spawn(Fixed::ONE);
    let b = w.spawn(Fixed::ONE);
    assert!(w
        .birth(
            &race,
            a,
            b,
            &[a, b],
            Fixed::from_ratio(1, 2),
            Fixed::ZERO,
            1
        )
        .is_none());
}
