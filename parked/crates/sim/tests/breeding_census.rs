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

//! The world-level wiring of the sex / breeding-system substrate and the reproductive census
//! (design Part 25, R-REPRO). Sex is a gene-fed phenotype read off a sex-determination locus through
//! the ordinary expression map: a founding cohort's sex ratio emerges from the pool's
//! sex-determination allele frequencies (a diploid heterozygous locus is one sex, homozygous the
//! other, the XY pattern), never a drawn ratio; a birth credits the reproductive census with the
//! gene-fed sexes; and an effective population size Ne derives from the census through one race-blind
//! kernel. The Fisherian 1:1 emergence and the non-steering kernel are proven in the `breeding` and
//! `census` module unit tests; this exercises the `World` integration.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId};
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, BreedingSystem, BreedingSystemId,
    BreedingSystemRegistry, Channel, CognitionChannel, Curve, DominanceKind, DominanceMode,
    EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme,
    InferenceParams, IntrinsicBeliefs, Race, RaceId, ReproductionMode, ReproductiveMoments,
    RingCapacityLaw, SchemeId, SexClass, SourceModeId, TwoTierWorld, ValueAxisId, ValueProfile,
    World,
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

/// A diploid race carrying a designated sex-determination locus (gene id 2) alongside two cognition
/// genes. The sex-determination gene is additive-neutral (both allele states contribute zero) but
/// carries a dominance deviation of one, so a heterozygote at the locus expresses one and a
/// homozygote zero: the XY pattern, where the heterogametic genotype is one sex. Which gene feeds
/// [`Channel::SexDetermination`] is data (a `GeneEffect`); the engine knows only the channel. The
/// pool's sex-determination allele frequency is one half, so a promoted cohort is a mix of
/// heterozygotes (class 1) and homozygotes (class 0): the sex ratio emerges from the locus.
fn sex_determined_race() -> Race {
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
                // Additive-neutral, but a heterozygote deviates by one: the heterogametic sex.
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
        gauss: civsim_core::GaussApprox::default(),
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
        Fixed::ZERO,
        80,
        18,
    )
    // Binary anisogamous breeding system at id 0 (two classes, threshold at the midpoint).
    .with_breeding(BreedingSystemId(0))
}

fn registry() -> BreedingSystemRegistry {
    let mut reg = BreedingSystemRegistry::new();
    reg.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    reg
}

/// Seed a band of the sex-determined race and return the world and the seeded ids.
fn seed_band(seed: u64, members: usize) -> (World, Vec<StableId>) {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), sex_determined_race());
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(seed);
    w.set_breeding_systems(registry());
    let seeded = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    (w, seeded)
}

#[test]
fn founder_sex_ratio_emerges_from_the_locus_and_is_deterministic() {
    // The founding cohort's sexes are read off the sex-determination locus, so both classes appear
    // from the pool's allele frequency without any drawn ratio. And it replays: two identical worlds
    // record an identical census.
    let (w, seeded) = seed_band(0x5E1, 40);
    let mut class0 = 0;
    let mut class1 = 0;
    for &id in &seeded {
        match w.census().sex_of(id) {
            Some(SexClass(0)) => class0 += 1,
            Some(SexClass(1)) => class1 += 1,
            other => panic!("a founder carries no gene-fed sex: {other:?}"),
        }
    }
    assert!(
        class0 > 0 && class1 > 0,
        "both sex classes emerge from the locus (got {class0} class-0, {class1} class-1)"
    );

    let (w2, _) = seed_band(0x5E1, 40);
    assert_eq!(
        w.census().state_hash(),
        w2.census().state_hash(),
        "the gene-fed sex census replays bit for bit"
    );
    // Sex is RNG-free: the same being reads the same sex however many times it is queried, and a
    // different-seed world differs in its census (distinct genomes).
    let (w3, _) = seed_band(0x5E2, 40);
    assert_ne!(
        w.census().state_hash(),
        w3.census().state_hash(),
        "a different seed draws different genomes and so a different sex census"
    );
}

#[test]
fn birth_credits_the_reproductive_census_with_gene_fed_sex() {
    // A two-parent birth credits both parents once, records the child's gene-fed sex, and the census
    // yields an effective population size through the one kernel.
    let (mut w, seeded) = seed_band(0x81C, 6);
    let parent_a = seeded[0];
    let parent_b = seeded[1];
    let offspring_before = w.census().total_offspring();
    let births_before = w.census().births();

    let child = w
        .birth(
            &sex_determined_race(),
            parent_a,
            parent_b,
            &seeded,
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 10),
            1,
            &dev_ring_law(),
        )
        .expect("a birth succeeds between two seeded parents");

    assert_eq!(
        w.census().births(),
        births_before + 1,
        "the birth is counted"
    );
    assert_eq!(
        w.census().total_offspring(),
        offspring_before + 2,
        "a two-parent birth credits both parents once"
    );
    assert_eq!(w.census().offspring_of(parent_a), 1);
    assert_eq!(w.census().offspring_of(parent_b), 1);
    assert!(
        w.census().sex_of(child).is_some(),
        "the child's gene-fed sex is recorded in the census"
    );
    assert!(
        w.census().effective_size() > 0,
        "Ne derives from the census through the one kernel"
    );
}

#[test]
fn pool_tier_add_births_derives_ne_without_individuals() {
    // The pool tier carries the same reproductive-moment accumulator: a coarse pool takes birth
    // inflows and derives Ne with no individuals, agreeing exactly with an individual census fed the
    // same events (record 62.9). Here a balanced, low-variance census sits near its head count.
    let mut world = TwoTierWorld::new();
    let pool = world.add_pool(0, Fixed::ZERO);
    let pi = world.pools.iter().position(|p| p.id == pool).unwrap();

    // Ten breeders of each class, each rearing two young.
    let mut mirror = ReproductiveMoments::new();
    for _ in 0..10 {
        world.pools[pi].add_births(SexClass(0), 2);
        world.pools[pi].add_births(SexClass(1), 2);
        mirror.record_parent(SexClass(0), 2);
        mirror.record_parent(SexClass(1), 2);
    }
    assert_eq!(
        world.pools[pi].ages.count_at(0),
        40,
        "the newborns entered the pool's age-zero cohort (20 breeders, two young each)"
    );
    let ne_pool = world.pools[pi].effective_size();
    assert_eq!(
        ne_pool,
        mirror.effective_size(),
        "the pool tier and a matching individual census reduce to the same Ne"
    );
    // Balanced sexes and equalized family sizes (every breeder rears exactly two) push Ne toward the
    // 2N ceiling, the known equalized-family effect, well above the 20 breeders.
    assert!(
        (30..=40).contains(&ne_pool),
        "an equalized-family census lifts Ne toward 2N (got {ne_pool})"
    );
}
