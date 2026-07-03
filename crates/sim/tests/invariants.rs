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

//! The conservation and referential-integrity harness (design Part 58, Part 60
//! Stage 1). It runs promotion, demotion, merge, and split against a two-tier world
//! and asserts, after every step, that every declared conserved projection balances
//! and that no relationship edge dangles. This is the second standing harness the
//! runbook requires in CI from the first tick, alongside the determinism harness.

use civsim_core::{Fixed, StableId};
use civsim_sim::conservation::ConservationRegistry;
use civsim_sim::lod::TwoTierWorld;
use civsim_sim::{AgeHistogram, AttrKindId, BeliefKey, BeliefParams, Curve, FacetStrength};

/// The total age-tracked population across both tiers: the members held in pool age
/// distributions plus the promoted individuals carrying an age. The conserved projection the
/// aggregate-tier demography must preserve across every tier crossing.
fn age_population(w: &TwoTierWorld) -> i128 {
    let pooled: i128 = w.pools.iter().map(|p| p.ages.total()).sum();
    let promoted: i128 = w.individuals.iter().filter(|i| i.age.is_some()).count() as i128;
    pooled + promoted
}

/// A harsh flat age hazard as data (a test fixture, never a fabricated calibration).
fn harsh_hazard() -> Curve {
    Curve::new([(Fixed::from_int(0), Fixed::from_ratio(3, 10))])
}

/// Build the registry of conserved projections for the two-tier world. Population
/// and wealth are the present entries; a future projection (an institution feature
/// vector, aggregate belief mass) is added by another `register` call and is
/// covered with nothing special-cased.
fn registry() -> ConservationRegistry<TwoTierWorld> {
    let mut reg = ConservationRegistry::new();
    reg.register("population", |w: &TwoTierWorld| w.population());
    reg.register("wealth", |w: &TwoTierWorld| w.total_wealth());
    reg
}

#[test]
fn promote_demote_merge_split_conserve_and_keep_references() {
    let reg = registry();
    let mut w = TwoTierWorld::new();

    // Two pools and a couple of already-promoted individuals.
    let pa = w.add_pool(20, Fixed::from_int(200));
    let pb = w.add_pool(12, Fixed::from_int(120));
    let seed_a = w.promote(pa, Fixed::from_int(5));
    let seed_b = w.promote(pb, Fixed::from_int(3));

    // Relationships that must never dangle, including ones we will demote.
    w.add_edge(seed_a, seed_b);

    let baseline = reg.snapshot(&w);
    assert!(reg.check_against(&baseline, &w).is_ok());
    assert!(w.referential_integrity_ok());

    // A sequence of structural changes; check the invariants after each.
    let p1 = w.promote(pa, Fixed::from_int(7));
    let p2 = w.promote(pa, Fixed::from_int(2));
    w.add_edge(p1, p2);
    w.add_edge(p1, seed_a);
    reg.check_against(&baseline, &w)
        .expect("promotion conserves");
    assert!(w.referential_integrity_ok(), "edges valid after promotion");

    // Demote one of the edge endpoints; its id must stay resolvable.
    w.demote(p1, pb);
    reg.check_against(&baseline, &w)
        .expect("demotion conserves");
    assert!(
        w.referential_integrity_ok(),
        "edge to a demoted individual still resolves"
    );

    // Merge the two pools.
    let merged = w.merge_pools(pa, pb);
    reg.check_against(&baseline, &w).expect("merge conserves");
    assert!(w.referential_integrity_ok(), "edges valid after merge");

    // Split the merged pool.
    w.split_pool(merged, 6, Fixed::from_int(45));
    reg.check_against(&baseline, &w).expect("split conserves");
    assert!(w.referential_integrity_ok(), "edges valid after split");

    // Demote the remaining promoted seeds too.
    w.demote(seed_a, merged);
    w.demote(seed_b, merged);
    w.demote(p2, merged);
    reg.check_against(&baseline, &w)
        .expect("final demotions conserve");
    assert!(
        w.referential_integrity_ok(),
        "every edge endpoint still resolves at the end"
    );
}

#[test]
fn a_deliberate_leak_is_caught() {
    // Prove the harness has teeth: a hand-made imbalance must fail the check.
    let reg = registry();
    let mut w = TwoTierWorld::new();
    let p = w.add_pool(10, Fixed::from_int(100));
    let baseline = reg.snapshot(&w);

    // Promote without the pool ever having existed at the baseline count: simulate a
    // leak by minting an individual out of nowhere (bypassing the conserving path).
    let id = w.reg.mint();
    w.individuals.push(civsim_sim::lod::Individual {
        id,
        wealth: Fixed::from_int(1),
        age: None,
        beliefs: Vec::new(),
    });
    w.reg.set_location(
        id,
        civsim_core::EntityLocation::Promoted(civsim_core::EntityHandle(0)),
    );
    let _ = p;

    let err = reg
        .check_against(&baseline, &w)
        .expect_err("an invented individual must be caught as a population (and wealth) leak");
    assert!(err.projection == "population" || err.projection == "wealth");
}

#[test]
fn a_dangling_reference_is_caught() {
    let mut w = TwoTierWorld::new();
    // An edge to an id the registry has never seen must read as dangling.
    let p = w.add_pool(1, Fixed::ZERO);
    let real = w.promote(p, Fixed::ZERO);
    w.add_edge(real, StableId(9_999));
    assert!(
        !w.referential_integrity_ok(),
        "an edge to an unminted id is a dangling reference"
    );
}

#[test]
fn age_population_is_conserved_across_the_tier_boundary() {
    // R-AGING pool tier + R-PROJ-REGISTER: the age-tracked population is a conserved
    // projection across promotion, demotion, and merge (the tier crossings), and an exact
    // sink across mortality. The aggregate-tier demography runs inside the two-tier world
    // with the conservation invariant enforced, the same way population and wealth are.
    let mut reg = ConservationRegistry::new();
    reg.register("age_population", age_population);

    let mut w = TwoTierWorld::new();
    let pa = w.add_pool_aged(
        AgeHistogram::from_pairs([(10, 8), (40, 5), (70, 3)]),
        Fixed::from_int(160),
    );
    let pb = w.add_pool_aged(
        AgeHistogram::from_pairs([(20, 6), (40, 4)]),
        Fixed::from_int(100),
    );
    let baseline = reg.snapshot(&w);

    // Promote a 40-year-old out of pa: the member leaves the pool's distribution and the
    // age travels with the individual, so the total is unchanged.
    let x = w.promote_at_age(pa, Fixed::from_int(5), 40);
    reg.check_against(&baseline, &w)
        .expect("promotion conserves the age population");
    assert_eq!(w.pools[0].count as i128, w.pools[0].ages.total());
    w.add_edge(x, x);
    assert!(w.referential_integrity_ok());

    // Demote it into pb: the age returns to a pool distribution, still conserved.
    w.demote(x, pb);
    reg.check_against(&baseline, &w)
        .expect("demotion returns the age and conserves the population");

    // Merge pb into pa: the histograms combine age by age, the total is unchanged.
    w.merge_pools(pa, pb);
    reg.check_against(&baseline, &w)
        .expect("merge conserves the age population");
    assert!(w.referential_integrity_ok());

    // Mortality is a sink: the age population drops by exactly the deaths, no leak, and each
    // pool's head count still equals its distribution total.
    let before = age_population(&w);
    let deaths = w.age_pools(&harsh_hazard(), 0xD3, 0);
    assert!(deaths > 0, "a harsh hazard over a real cohort takes some");
    assert_eq!(
        age_population(&w) + deaths,
        before,
        "mortality removes exactly the deaths, no leak"
    );
    for p in &w.pools {
        assert_eq!(
            p.count as i128,
            p.ages.total(),
            "count tracks the distribution after mortality"
        );
    }
}

/// A fixture belief calibration (an identity level-to-strength curve, a small dispersion, a
/// diffusion rate), never an owner value: it exercises the mechanism while the manifest path
/// stays fail-loud until the owner sets the real numbers.
fn belief_fixture() -> BeliefParams {
    BeliefParams {
        level_to_strength: Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)]),
        dispersion: Fixed::from_ratio(1, 10),
        diffusion_rate: Fixed::from_ratio(1, 4),
    }
}

#[test]
fn belief_mass_is_conserved_across_the_tier_boundary() {
    // R-PROJ-REGISTER, the belief half of the Part 54 keystone: aggregate belief mass is a
    // conserved projection across the lift (promotion) and the restriction (demotion), balancing
    // bit for bit after every crossing, exactly as population and wealth do. Registering it is all
    // it takes to cover it, with nothing special-cased.
    let mut reg = ConservationRegistry::new();
    reg.register("aggregate_belief_mass", |w: &TwoTierWorld| w.belief_mass());

    let params = belief_fixture();
    let mut w = TwoTierWorld::new();
    let p = w.add_pool(8, Fixed::from_int(80));
    // Seed two prevailing beliefs at distinct levels, each held by all eight members.
    let subject = StableId(100);
    let k1 = BeliefKey {
        subject,
        attr: AttrKindId(0),
        value: 1,
    };
    let k2 = BeliefKey {
        subject,
        attr: AttrKindId(0),
        value: 2,
    };
    w.pools[0].beliefs.seed(k1, Fixed::from_ratio(3, 5), 8);
    w.pools[0].beliefs.seed(k2, Fixed::from_ratio(1, 4), 8);

    let baseline = reg.snapshot(&w);
    assert!(reg.check_against(&baseline, &w).is_ok());

    // Lift a cohort of four, one crossing at a time; belief mass balances after each.
    let mut promoted = Vec::new();
    for i in 0..4u64 {
        let id = w.promote_lifting(p, Fixed::from_int(1), &params, 0, 0xB111 + i);
        promoted.push(id);
        reg.check_against(&baseline, &w)
            .expect("a lift conserves total belief mass");
    }

    // Restrict them back into the same pool; belief mass balances after each demotion.
    for id in promoted {
        w.demote(id, p);
        reg.check_against(&baseline, &w)
            .expect("a restriction conserves total belief mass");
    }
    // After the full round trip the counts return and the mass is exact.
    assert_eq!(w.pools[0].beliefs.get(&k1).unwrap().count, 8);
    assert_eq!(w.pools[0].beliefs.get(&k2).unwrap().count, 8);
    assert!(reg.check_against(&baseline, &w).is_ok());
}

#[test]
fn a_deliberate_belief_mass_leak_is_caught() {
    // The belief projection has teeth: a hand-made imbalance (belief mass invented in a pool with
    // no promoted counterpart) must fail the check.
    let mut reg = ConservationRegistry::new();
    reg.register("aggregate_belief_mass", |w: &TwoTierWorld| w.belief_mass());

    let mut w = TwoTierWorld::new();
    let _p = w.add_pool(4, Fixed::ZERO);
    let key = BeliefKey {
        subject: StableId(1),
        attr: AttrKindId(0),
        value: 3,
    };
    w.pools[0].beliefs.seed(key, Fixed::from_ratio(1, 2), 4);
    let baseline = reg.snapshot(&w);

    // Fold a facet strength into the pool's belief out of nowhere (bypassing the conserving lift),
    // inflating the mass with no counterpart on the promoted tier.
    w.pools[0]
        .beliefs
        .get_mut(&key)
        .unwrap()
        .fold_one(FacetStrength::new(Fixed::from_ratio(9, 10)));
    let err = reg
        .check_against(&baseline, &w)
        .expect_err("invented belief mass must be caught");
    assert_eq!(err.projection, "aggregate_belief_mass");
}
