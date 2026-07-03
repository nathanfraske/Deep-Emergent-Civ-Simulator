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

//! The race-normalized life stage: maturation and life fractions off per-race lifespan and
//! maturity data (design Part 20, R-AGING), per-being race identity seeded at the dawn and at
//! birth, and a mortality pass that reads one shared hazard curve at each being's own life scale.
//! The load-bearing test is the non-steering proof: a short-lived and a long-lived race diverge
//! purely from their data through the one code path (Principle 9), never a per-race branch.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId};
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, Channel, CognitionChannel, Curve, DominanceMode,
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

fn beliefs() -> IntrinsicBeliefs {
    IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 1)]),
        axioms: vec![Axiom {
            axis: AXIS,
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

/// A race that differs from another only in its identity and its owner-set lifespan and maturity.
/// Everything else (genes, pool, scheme, intrinsic disposition) is held identical, so any
/// divergence between two such races is forced through the per-race data alone (Principle 9). The
/// lifespan and maturity are labelled fixture values passed by the test, never fabricated here.
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
    };
    Race::new(
        id,
        genes,
        pool,
        scheme,
        beliefs(),
        Fixed::from_int(2),
        lifespan_years,
        maturity_years,
    )
}

#[test]
fn maturation_fraction_and_life_fraction_saturate_correctly() {
    // Fixture lifespan and maturity in life-cadence steps (labelled test values, not owner data).
    let r = race_with(RaceId(0), 80, 20);
    assert_eq!(r.maturation_fraction(0), Fixed::ZERO, "unmatured at birth");
    assert_eq!(
        r.maturation_fraction(20),
        Fixed::ONE,
        "fully matured at maturity_years"
    );
    assert_eq!(
        r.maturation_fraction(100),
        Fixed::ONE,
        "maturation saturates past maturity_years"
    );
    assert_eq!(
        r.life_fraction(40),
        Fixed::from_ratio(1, 2),
        "half a lifespan reads one half"
    );
    assert!(!r.is_mature(19), "below maturity_years is immature");
    assert!(r.is_mature(20), "at maturity_years is mature");
}

#[test]
fn zero_maturity_and_lifespan_do_not_panic() {
    // A race mature from birth and with a zero lifespan: the zero-denominator guard returns ONE
    // rather than dividing by zero (Fixed::from_ratio panics on a zero divisor).
    let r = race_with(RaceId(0), 0, 0);
    assert_eq!(
        r.maturation_fraction(0),
        Fixed::ONE,
        "zero maturity is fully mature at any age"
    );
    assert_eq!(
        r.life_fraction(50),
        Fixed::ONE,
        "zero lifespan reads a full life at any age"
    );
    assert!(r.is_mature(0), "zero maturity is mature at birth");
}

#[test]
fn race_of_is_seeded_at_dawn_and_at_birth_and_pruned_on_death() {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), race_with(RaceId(0), 80, 18));
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members: 2,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0x11FE);

    // An untracked (bare-spawned) being carries no race identity.
    let bare = w.spawn(Fixed::ONE);
    assert_eq!(w.race_of(bare), None, "a bare-spawned being has no race");

    // The dawn seeds each founder's race identity.
    let founders = w.seed_dawn_populations(&races, &bands);
    for &id in &founders {
        assert_eq!(
            w.race_of(id),
            Some(RaceId(0)),
            "the dawn recorded the founder's race"
        );
    }

    // A birth records the child's race.
    let race = race_with(RaceId(0), 80, 18);
    let child = w
        .birth(
            &race,
            founders[0],
            founders[1],
            &founders,
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 50),
            0,
        )
        .expect("a child was born");
    assert_eq!(
        w.race_of(child),
        Some(RaceId(0)),
        "the birth recorded the child's race"
    );

    // Death prunes the race identity along with the rest of the being's state.
    w.remove_being(child);
    assert_eq!(
        w.race_of(child),
        None,
        "death pruned the dead being's race identity"
    );
    assert_eq!(
        w.race_of(founders[0]),
        Some(RaceId(0)),
        "a surviving founder is untouched"
    );
}

/// A true step hazard: zero below a life fraction of one half, one at or above it. Built from a
/// doubled point at one half so the transition is a jump rather than a ramp, which keeps the
/// mortality outcome deterministic for a fraction on either side of the step.
fn step_hazard() -> Curve {
    Curve::new([
        (Fixed::ZERO, Fixed::ZERO),
        (Fixed::from_ratio(1, 2), Fixed::ZERO),
        (Fixed::from_ratio(1, 2), Fixed::ONE),
        (Fixed::ONE, Fixed::ONE),
    ])
}

#[test]
fn two_races_diverge_by_data_not_by_branch() {
    // Two races identical except their owner-set lifespan and maturity (labelled fixtures, not
    // owner data): a short-lived race and a long-lived one.
    let short = race_with(RaceId(0), 80, 18);
    let long = race_with(RaceId(1), 700, 200);

    // The same maturity gate, keyed only off each race's own maturity datum, reads opposite
    // answers for a being of raw age fifty: the short race is adult, the long race is not.
    assert!(
        short.is_mature(50),
        "the short-lived race is mature at fifty"
    );
    assert!(
        !long.is_mature(50),
        "the long-lived race is not yet mature at fifty"
    );

    // Seed one being of each race onto the same place, then age each to fifty.
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), race_with(RaceId(0), 80, 18));
    races.insert(RaceId(1), race_with(RaceId(1), 700, 200));
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
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0x5EED);
    let seeded = w.seed_dawn_populations(&races, &bands);
    let short_being = seeded[0];
    let long_being = seeded[1];
    w.set_age(short_being, 50);
    w.set_age(long_being, 50);

    // One shared curve, one shared call, one shared RNG. The short being's life fraction is
    // 50/80 = 0.625 (above the step, so it dies); the long being's is 50/700 (below the step, so
    // it survives). The divergence is forced by each race's lifespan datum, not a code branch.
    let dead = w.apply_mortality_by_race(&races, &step_hazard());
    assert!(
        dead.contains(&short_being),
        "the short-lived being crossed the step and died"
    );
    assert!(
        !dead.contains(&long_being),
        "the long-lived being was below the step and survived"
    );
}

#[test]
fn mortality_by_race_falls_back_to_raw_age_for_an_untracked_being() {
    // With no race identity recorded and an empty races map, the race-keyed pass must behave
    // exactly like the raw-age apply_mortality: same beings, same ages, same seed, same curve,
    // so the two cull the identical set.
    let hazard = Curve::new([
        (Fixed::ZERO, Fixed::ZERO),
        (Fixed::from_int(100), Fixed::ONE),
    ]);
    let build = || {
        let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0xFA11);
        let ids: Vec<StableId> = (0..8).map(|_| w.spawn(Fixed::ONE)).collect();
        for (k, &id) in ids.iter().enumerate() {
            w.set_age(id, 40 + k as u32); // a spread of ages around the half-hazard region
        }
        w
    };
    let mut raw = build();
    let mut by_race = build();
    let empty: BTreeMap<RaceId, Race> = BTreeMap::new();
    let raw_dead = raw.apply_mortality(&hazard);
    let by_race_dead = by_race.apply_mortality_by_race(&empty, &hazard);
    assert_eq!(
        raw_dead, by_race_dead,
        "an untracked being falls back to the raw-age hazard, identical to apply_mortality"
    );
    assert!(!raw_dead.is_empty(), "the hazard culled some beings");
}
