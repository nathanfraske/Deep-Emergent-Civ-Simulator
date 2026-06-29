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
