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

//! The age-personality substrate driven through the world's life cadence (design Part 20,
//! R-BEING-REP). The load-bearing test is the non-steering proof: two races whose only difference is
//! their per-race `TraitDef` data (a plasticity curve and a maturity target) get opposite age
//! trajectories from the one `age_personality` kernel run at the life-cadence beat, one trait rising
//! and the other falling, with no per-race code branch (Principle 9) and bit-for-bit determinism
//! (Principle 3). It also checks that a world installing no personality registry is unchanged, that
//! a birth-neutral instance is seeded at the dawn, and that death prunes the instance.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId};
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, Channel, CognitionChannel, Curve, DominanceMode,
    EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme,
    InferenceParams, IntrinsicBeliefs, PersonalityProfile, PersonalityRegistry, Race, RaceId,
    ReproductionMode, RingCapacityLaw, SchemeId, SourceModeId, TraitAxisId, TraitDef, ValueAxisId,
    ValueProfile, World,
};

const AXIS: TraitAxisId = TraitAxisId(0);

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

fn beliefs() -> IntrinsicBeliefs {
    IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 1)]),
        axioms: vec![Axiom {
            axis: AxiomAxisId(0),
            stance: Fixed::ZERO,
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 1,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 8),
            innate_seed: Fixed::ZERO,
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

/// A race differing from another only in its identity and its owner-set lifespan and maturity, so a
/// personality divergence is forced through the per-race `TraitDef` data alone.
fn race_with(id: RaceId, lifespan_years: u32, maturity_years: u32) -> Race {
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
        id,
        genes,
        pool,
        scheme,
        beliefs(),
        Fixed::ZERO,
        Fixed::ZERO,
        lifespan_years,
        maturity_years,
    )
}

/// A plasticity curve high in youth (maturation fraction 0) and low at the plateau (fraction 1): the
/// maturity-principle shape, high-youth-to-low-plateau, expressed as per-race data.
fn youth_high_curve() -> Curve {
    Curve::new([
        (Fixed::ZERO, Fixed::from_ratio(2, 5)),
        (Fixed::ONE, Fixed::from_ratio(1, 20)),
    ])
}

/// Two races and their bands, one member each on the same place.
fn two_race_world(seed: u64) -> (World, BTreeMap<RaceId, Race>, Vec<StableId>) {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), race_with(RaceId(0), 80, 20));
    races.insert(RaceId(1), race_with(RaceId(1), 80, 20));
    let bands = [
        BandSpec {
            race: RaceId(0),
            place: 1,
            members: 1,
        },
        BandSpec {
            race: RaceId(1),
            place: 1,
            members: 1,
        },
    ];

    // Race 0's trait rises with age (target above the neutral birth), race 1's falls (target below).
    // Same curve, mirror-image targets: only the data differs.
    let mut registry = PersonalityRegistry::new();
    registry.set(
        RaceId(0),
        PersonalityProfile::new([TraitDef {
            axis: AXIS,
            plasticity_curve: youth_high_curve(),
            maturity_target: Fixed::from_ratio(4, 5),
        }]),
    );
    registry.set(
        RaceId(1),
        PersonalityProfile::new([TraitDef {
            axis: AXIS,
            plasticity_curve: youth_high_curve(),
            maturity_target: Fixed::from_ratio(-4, 5),
        }]),
    );

    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(seed);
    // The registry must be installed before the dawn so the founders are seeded birth-neutral.
    w.set_personality_registry(registry);
    w.set_life_cadence(1);
    let seeded = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    (w, races, seeded)
}

#[test]
fn two_races_diverge_by_data_not_by_branch_over_the_life_cadence() {
    let (mut w, _races, seeded) = two_race_world(0x9E1D);
    let rising = seeded[0];
    let falling = seeded[1];

    // The dawn seeded both founders birth-neutral through the one seeding path.
    assert_eq!(
        w.trait_value(rising, AXIS),
        Some(Fixed::ZERO),
        "the rising-race founder starts neutral"
    );
    assert_eq!(
        w.trait_value(falling, AXIS),
        Some(Fixed::ZERO),
        "the falling-race founder starts neutral"
    );

    // Age both beings across their maturation through the life-cadence beat (cadence one, so every
    // tick beats aging then the personality drift). No mortality hazard is installed, so nobody dies.
    for _ in 0..40 {
        w.tick(&[]);
    }

    let a = w
        .trait_value(rising, AXIS)
        .expect("rising founder has traits");
    let b = w
        .trait_value(falling, AXIS)
        .expect("falling founder has traits");
    assert!(
        a > Fixed::ZERO,
        "race 0's trait rose toward its positive maturity target ({a:?})"
    );
    assert!(
        b < Fixed::ZERO,
        "race 1's trait fell toward its negative maturity target ({b:?})"
    );
    // Mirror-image data through one kernel gives mirror-image trajectories, up to floor rounding, so
    // no directional bias lives in the mechanism (Principle 9).
    let residual = (a.to_bits() + b.to_bits()).abs();
    assert!(
        residual < 1024,
        "the one kernel is direction-neutral: the two trajectories mirror (residual {residual} bits)"
    );
}

#[test]
fn the_life_cadence_drift_is_deterministic_across_runs() {
    let (mut w1, _r1, s1) = two_race_world(0x5A3E);
    let (mut w2, _r2, s2) = two_race_world(0x5A3E);
    for _ in 0..30 {
        w1.tick(&[]);
        w2.tick(&[]);
    }
    for (a, b) in s1.iter().zip(s2.iter()) {
        assert_eq!(
            w1.trait_value(*a, AXIS),
            w2.trait_value(*b, AXIS),
            "two identical runs drift the same personality bit for bit"
        );
    }
    // The state hashes agree too (the trait beat introduces no replay divergence).
    assert_eq!(w1.state_hash(), w2.state_hash());
}

#[test]
fn a_world_with_no_personality_registry_carries_no_traits() {
    // With no registry installed, the dawn seeds no trait instances and the beat is inert, so the
    // substrate is a pure add-on: an unconfigured world is unchanged.
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), race_with(RaceId(0), 80, 20));
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members: 2,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0xBA3E);
    w.set_life_cadence(1);
    let seeded = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    for &id in &seeded {
        assert_eq!(
            w.trait_value(id, AXIS),
            None,
            "no registry means no trait instance"
        );
    }
    for _ in 0..5 {
        w.tick(&[]);
    }
    for &id in &seeded {
        assert_eq!(w.trait_value(id, AXIS), None, "the beat stays inert");
    }
}

#[test]
fn death_prunes_a_beings_personality() {
    let (mut w, _races, seeded) = two_race_world(0xDEAD);
    let victim = seeded[0];
    assert!(w.trait_value(victim, AXIS).is_some());
    w.remove_being(victim);
    assert_eq!(
        w.trait_value(victim, AXIS),
        None,
        "death pruned the dead being's personality instance"
    );
    assert!(
        w.trait_value(seeded[1], AXIS).is_some(),
        "a surviving being's personality is untouched"
    );
}
