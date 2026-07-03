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

use civsim_core::{Fixed, StableId};
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, Channel, CognitionChannel, Curve, DominanceKind,
    DominanceMode, EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet,
    GeneticScheme, InferenceParams, IntrinsicBeliefs, Race, RaceId, ReproductionMode,
    RingCapacityLaw, SchemeId, SourceModeId, ValueAxisId, ValueProfile, World,
};

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

/// A labelled test ring-capacity law (not owner data): a linear memory-to-slots curve and a
/// ceiling, enough to size the founders' evidence rings from their expressed memory.
fn dev_ring_law() -> RingCapacityLaw {
    RingCapacityLaw {
        curve: Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::from_int(8), Fixed::from_int(16)),
        ]),
        hard_cap: 32,
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
        // Environment variance zero: a homogeneous developmental environment, so this fixture
        // reproduces the pre-V_E dawn (labelled test value, not owner data).
        Fixed::ZERO,
        // Fixture lifespan and maturity in life-cadence steps (labelled test values, not owner data).
        80,
        18,
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
    let seeded = w.seed_dawn_populations(&races, &bands, &dev_ring_law());

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
        let ids = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
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
    let seeded = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    assert!(
        seeded.is_empty(),
        "a band whose race is not registered is skipped"
    );
    assert_eq!(w.population(), 0);
}

/// Two races identical except the weight of their MemoryCapacity gene (labelled fixtures, not
/// owner data): the only difference feeds the memory phenotype. The memory gene carries a
/// heterozygote deviation, so a member heterozygous at that locus expresses the gene while a
/// homozygous one expresses only the environment baseline; the pool keeps the locus polymorphic
/// so a band spreads across both. Everything else (acuity gene, pool, scheme, disposition,
/// environment) is held identical, so any ring-capacity divergence is forced through the memory
/// data alone (Principle 9).
fn race_by_memory_weight(id: u32, memory_weight: Fixed) -> Race {
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
                    weight: memory_weight,
                }],
                dominance: DominanceMode {
                    a: Fixed::ZERO,
                    d: Fixed::from_int(4),
                    kind: DominanceKind::Over,
                },
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
        Fixed::ZERO,
        80,
        18,
    )
}

#[test]
fn ring_capacity_diverges_by_race_data_through_one_law_with_no_race_branch() {
    // A memory-to-slots curve y = 2x (labelled fixture): memory 2 -> 4 slots, 4 -> 8, 6 -> 12.
    let law = RingCapacityLaw {
        curve: Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::from_int(8), Fixed::from_int(16)),
        ]),
        hard_cap: 32,
    };
    // Race A weights its memory gene twice as hard as race B; nothing else differs.
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), race_by_memory_weight(0, Fixed::ONE));
    races.insert(RaceId(1), race_by_memory_weight(1, Fixed::from_ratio(1, 2)));
    let members = 12;
    let bands = [
        BandSpec {
            race: RaceId(0),
            place: 1,
            members,
        },
        BandSpec {
            race: RaceId(1),
            place: 2,
            members,
        },
    ];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0x21A1);
    let seeded = w.seed_dawn_populations(&races, &bands, &law);
    let (band_a, band_b) = seeded.split_at(members);

    let cap = |id: &StableId| w.intrinsic_of(*id).unwrap().axioms[0].evidence.cap();
    let mem = |id: &StableId| w.mind(*id).unwrap().memory;
    let caps_a: Vec<usize> = band_a.iter().map(cap).collect();
    let caps_b: Vec<usize> = band_b.iter().map(cap).collect();

    // Guard against a vacuous band: race A must spread in memory (a heterozygous and a homozygous
    // member both drawn at the memory locus), or nothing is being compared.
    let min_a = *caps_a.iter().min().unwrap();
    let max_a = *caps_a.iter().max().unwrap();
    assert!(max_a > min_a, "race A spread in memory alleles: {caps_a:?}");

    // Within race A, two members with different memory alleles get individually-differing caps,
    // each from its own expressed memory rather than one race-wide number.
    let a_hi = band_a.iter().max_by_key(|id| cap(id)).unwrap();
    let a_lo = band_a.iter().min_by_key(|id| cap(id)).unwrap();
    assert_ne!(
        mem(a_hi),
        mem(a_lo),
        "the two members differ in expressed memory"
    );
    assert_ne!(cap(a_hi), cap(a_lo), "and so in ring capacity");

    // Race A's larger memory-gene weight buys a larger evidence ring than race B's, the direction
    // the gene data implies.
    let max_b = *caps_b.iter().max().unwrap();
    assert!(
        max_a > max_b,
        "race A's memory gene weight buys a larger ring: A {caps_a:?} vs B {caps_b:?}"
    );

    // No RaceId branch: the law reads only the memory value, so two members of different races
    // with the same expressed memory get the same ring. A homozygous member of each race
    // expresses the bare environment baseline, so their caps match across the race line.
    let base = Fixed::from_int(2);
    let a_base = band_a
        .iter()
        .find(|id| mem(id) == base)
        .expect("race A has a baseline-memory member");
    let b_base = band_b
        .iter()
        .find(|id| mem(id) == base)
        .expect("race B has a baseline-memory member");
    assert_eq!(
        cap(a_base),
        cap(b_base),
        "equal memory yields an equal ring across races: capacity_for has no race branch"
    );
    assert_eq!(cap(a_base), law.capacity_for(base));
}
