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

//! Second-language acquisition (design Part 33.6, the R-LANG-MODALITY / R-AGING seam): the
//! age-of-acquisition breakpoint DERIVES from each race's own maturation, never the human late
//! teens. The load-bearing test is the non-steering proof: two races with different `maturity_years`
//! cross their critical-period breakpoints at different raw ages from the one kernel and one curve,
//! with no per-race branch (Principle 9). Every number here is a clearly-labelled fixture.

use civsim_core::{Fixed, GaussApprox};
use civsim_sim::{
    Curve, EpistemicStance, GenePool, GeneSet, GeneticScheme, IntrinsicBeliefs, L2AcquisitionLaw,
    LangKnowledge, Race, RaceId, ReproductionMode, SchemeId, ValueProfile,
};

// A minimal race carrying only the datum the L2 breakpoint reads, its `maturity_years`. The rest is
// an inert fixture, never an owner value.
fn race_with_maturity(id: u32, maturity_years: u32) -> Race {
    Race::new(
        RaceId(id),
        GeneSet { genes: vec![] },
        GenePool::new(SchemeId(0), 1, vec![]),
        GeneticScheme {
            id: SchemeId(0),
            reproduction: ReproductionMode::Haploid,
            linkage_groups: vec![],
            mutation_rate: Fixed::ZERO,
            additive_mutation_step: Fixed::ZERO,
            gauss: GaussApprox::default(),
        },
        IntrinsicBeliefs {
            values: ValueProfile::new(),
            axioms: vec![],
            epistemic: EpistemicStance::new([], Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, Fixed::ZERO),
        },
        Fixed::ZERO,
        Fixed::ZERO,
        maturity_years.max(1).saturating_mul(4), // a lifespan comfortably past maturity
        maturity_years,
    )
}

// A decreasing increment curve: fast while immature (maturation fraction below one), the adult
// residual once mature (fraction saturated at one). A fixture curve, never an owner value.
fn law() -> L2AcquisitionLaw {
    L2AcquisitionLaw {
        increment_by_maturation: Curve::new([
            (Fixed::ZERO, Fixed::from_ratio(1, 10)),
            (Fixed::ONE, Fixed::from_ratio(1, 100)),
        ]),
    }
}

#[test]
fn the_breakpoint_derives_from_each_race_maturity_no_hardcoded_age_no_raceid() {
    let l = law();
    let early = race_with_maturity(0, 10); // matures at age 10
    let late = race_with_maturity(1, 20); // matures at age 20

    // At one fixed raw age of 15: the early race is past its breakpoint (mature, adult residual),
    // the late race is still plastic (three-quarters matured), so the SAME kernel gives the late
    // race the larger increment. The breakpoint is each race's own maturity, not one shared age.
    let inc_early = l.increment_for(&early, 15);
    let inc_late = l.increment_for(&late, 15);
    assert!(
        inc_late > inc_early,
        "at one age the less-matured race still gains faster ({inc_late:?} > {inc_early:?})"
    );
    assert_eq!(
        inc_early,
        Fixed::from_ratio(1, 100),
        "the matured race gains at the adult residual"
    );

    // The increment slows past each race's own breakpoint: just below maturity gains more than at
    // it, and both races show the same slowdown on their own scale (no human late-teens constant).
    for race in [&early, &late] {
        let m = race.maturity_years;
        let before = l.increment_for(race, m - 1);
        let after = l.increment_for(race, m);
        assert!(
            before > after,
            "the increment slows past the breakpoint (maturity {m})"
        );
    }
}

#[test]
fn proficiency_climbs_under_the_derived_increment_and_a_flat_curve_is_age_independent() {
    // The kernel and the per-being state compose end to end: a plastic (pre-maturity) learner and a
    // mature learner acquire the same language over ten ticks, and the plastic one ends more
    // proficient, from the derived per-age increment alone.
    let l = law();
    let race = race_with_maturity(3, 10);
    let lang = civsim_sim::language::LangId(1);
    let mut child = LangKnowledge::new();
    let mut adult = LangKnowledge::new();
    for _ in 0..10 {
        child.acquire(lang, l.increment_for(&race, 2)); // age 2, still plastic
        adult.acquire(lang, l.increment_for(&race, 40)); // age 40, past the breakpoint
    }
    assert!(
        child.proficiency(lang) > adult.proficiency(lang),
        "the plastic learner outpaces the past-breakpoint learner"
    );

    // A flat increment curve is the no-critical-period special case: every age, every race, gains
    // the same, with no race branch anywhere.
    let flat = L2AcquisitionLaw {
        increment_by_maturation: Curve::new([
            (Fixed::ZERO, Fixed::from_ratio(1, 20)),
            (Fixed::ONE, Fixed::from_ratio(1, 20)),
        ]),
    };
    let a = race_with_maturity(0, 5);
    let b = race_with_maturity(1, 50);
    assert_eq!(flat.increment_for(&a, 0), Fixed::from_ratio(1, 20));
    assert_eq!(flat.increment_for(&a, 100), flat.increment_for(&b, 100));
    assert_eq!(flat.increment_for(&a, 3), flat.increment_for(&b, 3));
}
