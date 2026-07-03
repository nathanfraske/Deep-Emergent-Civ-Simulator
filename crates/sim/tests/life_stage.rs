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
/// ceiling, used to size a being's evidence ring from its expressed memory.
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
        additive_mutation_step: Fixed::ZERO,
        gauss: civsim_core::GaussApprox::default(),
    };
    Race::new(
        id,
        genes,
        pool,
        scheme,
        beliefs(),
        Fixed::from_int(2),
        // Homogeneous developmental environment (V_E zero) for the life-stage fixture.
        Fixed::ZERO,
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
    let founders = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
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
            &dev_ring_law(),
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
    let seeded = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    let short_being = seeded[0];
    let long_being = seeded[1];
    w.set_age(short_being, 50);
    w.set_age(long_being, 50);

    // One shared curve, one shared call, one shared RNG. The short being's life fraction is
    // 50/80 = 0.625 (above the step, so it dies); the long being's is 50/700 (below the step, so
    // it survives). The divergence is forced by each race's lifespan datum, not a code branch.
    let dead = w
        .apply_mortality_by_race(&races, &step_hazard())
        .expect("both beings are raced");
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
fn mortality_by_race_refuses_an_untracked_being_rather_than_reading_the_wrong_domain() {
    // Regression (audit defect 5): the race-keyed pass evaluates a LIFE-FRACTION hazard curve. An
    // untracked being has no lifespan to normalize its age by, so it cannot be placed on that domain.
    // Rather than read the curve at the being's RAW age (a domain mismatch that made the unraced
    // class near-immortal or culled wholesale), the pass fails loud with UnraceableBeing.
    let hazard = Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)]);
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0xFA11);
    let ids: Vec<StableId> = (0..8).map(|_| w.spawn(Fixed::ONE)).collect();
    for (k, &id) in ids.iter().enumerate() {
        w.set_age(id, 40 + k as u32);
    }
    let empty: BTreeMap<RaceId, Race> = BTreeMap::new();
    let before = w.population();
    let result = w.apply_mortality_by_race(&empty, &hazard);
    assert!(
        result.is_err(),
        "an untracked being is refused, never read in the raw-age domain"
    );
    assert_eq!(
        w.population(),
        before,
        "the refused pass removes no one (no partial cull)"
    );
}

#[test]
fn a_mixed_raced_and_unraced_population_does_not_make_one_class_immortal() {
    // Regression (audit defect 5): a population mixing raced and unraced beings does not silently
    // make one class immortal (the old fallback made the unraced class near-immortal against a
    // fraction curve). The pass refuses the whole cull and removes NEITHER class, so no class is
    // silently spared while the other is culled.
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), race_with(RaceId(0), 80, 18));
    let bands = [BandSpec {
        race: RaceId(0),
        place: 1,
        members: 1,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0x5EED);
    let seeded = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    let raced = seeded[0];
    w.set_age(raced, 60); // life fraction 60/80, above the step: would die if the pass ran
                          // An unraced being (spawned directly, no race identity), aged into the lethal zone.
    let unraced = w.spawn(Fixed::ONE);
    w.set_age(unraced, 60);
    let before = w.population();
    let err = w
        .apply_mortality_by_race(&races, &step_hazard())
        .expect_err("a mixed population is refused");
    assert_eq!(err.0, unraced, "the refusal names the unraceable being");
    assert_eq!(
        w.population(),
        before,
        "neither class is culled: no class is silently made immortal by a domain mismatch"
    );
}
