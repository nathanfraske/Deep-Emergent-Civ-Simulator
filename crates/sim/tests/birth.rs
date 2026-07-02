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
use civsim_core::StableId;
use civsim_sim::{
    genetic_distance, AccessWeights, Axiom, AxiomAxisId, BandSpec, Channel, CognitionChannel,
    DominanceMode, EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet,
    GeneticScheme, InferenceParams, IntrinsicBeliefs, MatePreference, Race, RaceId,
    ReproductionMode, SchemeId, SourceModeId, ValueAxisId, ValueProfile, World,
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

// --- Follow-on B: the heritable mate preference at the choose site (R-REPRO) ---

/// A race with many independent genes, so dawn-promoted genomes spread across a range of
/// genetic distances (the two-gene `a_race` gives too few distinct distances to tell a
/// nearest-mate pick from a farthest one).
fn diverse_race() -> Race {
    const N: usize = 8;
    let genes = GeneSet {
        genes: (0..N)
            .map(|i| GeneDef {
                id: GeneId(i as u32),
                effects: vec![GeneEffect {
                    channel: Channel::Cognition(CognitionChannel::ReasoningAcuity),
                    weight: Fixed::ZERO,
                }],
                dominance: DominanceMode::additive(),
            })
            .collect(),
    };
    let pool = GenePool::new(SchemeId(0), 20, vec![Fixed::from_ratio(1, 2); N]);
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: Fixed::ZERO,
    };
    let intrinsic = IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 2)]),
        axioms: Vec::new(),
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

/// Seed one diverse-race band of `members` onto a place; return the world, the race, and the ids.
fn dawn_band(seed: u64, members: usize) -> (World, Race, Vec<StableId>) {
    let race = diverse_race();
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), diverse_race());
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(seed);
    let seeded = w.seed_dawn_populations(&races, &bands);
    (w, race, seeded)
}

#[test]
fn the_dawn_seeds_a_mate_preference_with_unbiased_variation() {
    let (w, _race, seeded) = dawn_band(0xA11CE, 12);
    // Every seeded being carries a preference.
    let weights: Vec<Fixed> = seeded
        .iter()
        .map(|id| {
            w.mate_pref_of(*id)
                .expect("a dawn being has a mate preference")
                .distance_weight
        })
        .collect();
    // The population carries variation (the preferences are not all one value), the raw
    // material selection needs. A vacuous all-equal seeding would author no variation to select
    // over, so this guards against it.
    let first = weights[0];
    assert!(
        weights.iter().any(|w| *w != first),
        "the dawn preferences vary across the band"
    );
    // The variation is unbiased: at least one homophile (negative) and one heterophile
    // (positive), so no single direction is seeded into the whole population.
    assert!(
        weights.iter().any(|w| *w < Fixed::ZERO) && weights.iter().any(|w| *w > Fixed::ZERO),
        "the dawn seeds both directions, not one authored sign"
    );
    // The seeding replays bit for bit.
    let (w2, _r2, seeded2) = dawn_band(0xA11CE, 12);
    let weights2: Vec<Fixed> = seeded2
        .iter()
        .map(|id| w2.mate_pref_of(*id).unwrap().distance_weight)
        .collect();
    assert_eq!(weights, weights2, "same seed, same dawn preferences");
}

#[test]
fn choose_mate_honours_the_preference_sign_and_excludes_self() {
    let (mut w, _race, seeded) = dawn_band(0xBEEF, 8);
    let chooser = seeded[0];
    let candidates: Vec<StableId> = seeded.clone();
    // The real genetic distances from the chooser to the other members.
    let chooser_genome = w.genome_of(chooser).unwrap().clone();
    let dists: Vec<(StableId, Fixed)> = seeded
        .iter()
        .filter(|id| **id != chooser)
        .map(|id| {
            (
                *id,
                genetic_distance(&chooser_genome, w.genome_of(*id).unwrap()),
            )
        })
        .collect();
    let min_d = dists.iter().map(|(_, d)| *d).min().unwrap();
    let max_d = dists.iter().map(|(_, d)| *d).max().unwrap();
    // Guard against a vacuous test: the band must spread in distance, or nearest and
    // farthest are the same pick and the preference sign proves nothing.
    assert!(min_d != max_d, "the candidates spread in genetic distance");

    // A homophile picks a nearest-distance mate; a heterophile picks a farthest-distance one.
    w.set_mate_pref(chooser, MatePreference::homophilic());
    let near = w
        .choose_mate(chooser, &candidates)
        .expect("a mate is chosen");
    assert_ne!(
        near, chooser,
        "the chooser is excluded from its own candidates"
    );
    let near_d = genetic_distance(&chooser_genome, w.genome_of(near).unwrap());
    assert_eq!(near_d, min_d, "the homophile picks a nearest mate");

    w.set_mate_pref(chooser, MatePreference::heterophilic());
    let far = w
        .choose_mate(chooser, &candidates)
        .expect("a mate is chosen");
    let far_d = genetic_distance(&chooser_genome, w.genome_of(far).unwrap());
    assert_eq!(far_d, max_d, "the heterophile picks a farthest mate");

    // The choice replays bit for bit.
    let (mut w2, _r2, _s2) = dawn_band(0xBEEF, 8);
    w2.set_mate_pref(chooser, MatePreference::heterophilic());
    assert_eq!(
        w2.choose_mate(chooser, &candidates),
        Some(far),
        "same seed and preference, same chosen mate"
    );
}

#[test]
fn birth_inherits_the_mate_preference_by_midparent_and_replays() {
    // With mutation off, the child's weight is the exact midparent of the two parents', so the
    // inheritance is the quantitative-genetics midparent rule and authors no drift of its own.
    let (mut w, race, seeded) = dawn_band(0xF00D, 2);
    let (pa, pb) = (seeded[0], seeded[1]);
    w.set_mate_pref(pa, MatePreference::new(Fixed::from_ratio(1, 2)));
    w.set_mate_pref(
        pb,
        MatePreference::new(Fixed::from_int(-1) + Fixed::from_ratio(1, 2)),
    );
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
    assert_eq!(
        w.mate_pref_of(child).unwrap().distance_weight,
        Fixed::ZERO,
        "midparent of +1/2 and -1/2 is zero with mutation off"
    );

    // With mutation on, the child's weight replays bit for bit under the same seed.
    let run = || {
        let (mut w, race, seeded) = dawn_band(0x1234, 2);
        let (pa, pb) = (seeded[0], seeded[1]);
        w.set_mate_pref(pa, MatePreference::homophilic());
        w.set_mate_pref(pb, MatePreference::heterophilic());
        let child = w
            .birth(
                &race,
                pa,
                pb,
                &[pa, pb],
                Fixed::from_ratio(1, 2),
                Fixed::from_ratio(1, 8),
                1,
            )
            .unwrap();
        w.mate_pref_of(child).unwrap().distance_weight
    };
    assert_eq!(run(), run(), "same seed, same inherited preference");
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
