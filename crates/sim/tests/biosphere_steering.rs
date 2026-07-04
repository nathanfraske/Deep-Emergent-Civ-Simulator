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

//! Steering audit for the biosphere viability cull (design Part 25.11, Part 15; R-BIOSPHERE,
//! R-BEHAVIOR-EVOLVE; Principle 9). The owner's decision on the birth-nonviable anatomy case (a
//! creature whose organ roll gives it no metabolic reserve) is to LEAN ON THE EXISTING CLOSURE CULL
//! rather than add a seed-time viability filter. This file proves that leaning on that cull steers no
//! emergent outcome: the cull is a physics-only viability filter, and the birth-viability it removes
//! is a pure function of the organ set, independent of every other body trait, so it removes only the
//! physically-impossible and biases no morphological distribution.
//!
//! The cull is the hybrid of design fork F1: at seed time the least-fixed-point `grounded` walk
//! (a species survives only if its `draws_on` reach an abiotic source through the food web), and at
//! runtime the Part 15 stock dynamics (a pool whose sustained demand exceeds its supply collapses and
//! does not revive). Neither reads morphology. The birth-nonviable anatomy leans on the runtime half:
//! a creature that stores no energy dies at once, so its aggregate pool draws no sustaining return and
//! collapses under that same over-harvest cull, no new filter.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::{DrawKey, Fixed, Phase};
use civsim_sim::anatomy::{
    sample_body_plan, BodyPlan, BodyPlanRegistry, Part, Temperament, WorldProfile,
};
use civsim_sim::biosphere::{grounded, Niche, SourceRef, Species};
use civsim_sim::body::{Body, BodyParams, BLOOD};
use civsim_sim::genome::{GenePool, SchemeId};
use civsim_sim::homeostasis::{birth_viable, Homeostasis, HomeostaticRegistry, ENERGY};
use civsim_sim::lineage::SpeciesId;
use civsim_sim::stocks::Stock;

// Dev-registry organ kind ids (crate::anatomy BodyPlanRegistry::dev_default): 0 fat-body (energy
// dense), 1 glycogen-store, 2 water-store (zero energy density, the reserveless case).
const FAT_BODY: u16 = 0;
const WATER_STORE: u16 = 2;

fn temperament() -> Temperament {
    Temperament {
        boldness: Fixed::from_ratio(1, 2),
        exploration: Fixed::from_ratio(1, 2),
        activity: Fixed::from_ratio(1, 2),
        sociability: Fixed::from_ratio(1, 2),
        aggression: Fixed::from_ratio(1, 4),
    }
}

fn part(kind: u16, dev: (i64, i64)) -> Part {
    Part {
        kind,
        development: Fixed::from_ratio(dev.0, dev.1),
    }
}

/// A body plan with explicit mass, covering thickness (armor), weapons, and organ set, so a test can
/// vary morphology while holding the organ set (the thing viability derives from) fixed.
fn body(mass: (i64, i64), armor: (i64, i64), weapons: Vec<Part>, organs: Vec<Part>) -> BodyPlan {
    BodyPlan {
        body_mass: Fixed::from_ratio(mass.0, mass.1),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons,
        covering: part(5, armor), // a covering kind; development is the armor thickness
        senses: vec![part(0, (1, 2))],
        locomotion: vec![1],
        organs,
        temperament: temperament(),
    }
}

/// A species carrying a given body plan and food-web draws, with a filler niche and pool (neither read
/// by the topological cull).
fn species(layer: u16, body_plan: BodyPlan, draws_on: Vec<SourceRef>) -> Species {
    Species {
        layer,
        niche: Niche {
            optimum: vec![Fixed::from_ratio(1, 2); 4],
            breadth: vec![Fixed::from_ratio(3, 10); 4],
        },
        body_plan,
        draws_on,
        pool: GenePool::new(SchemeId(0), 100, vec![Fixed::from_ratio(1, 2); 8]),
        extinct: false,
    }
}

#[test]
fn the_topological_closure_cull_reads_the_food_web_not_the_body() {
    // One abiotic source is present; a producer grounding on it survives, an orphan whose prey is
    // absent is culled. The proof: swap the two species' bodies, leaving the food web unchanged, and
    // the grounded set is identical. So the cull cannot key on a body plan.
    let abiotic: BTreeSet<u16> = [0u16].into_iter().collect();
    let giant = body(
        (1, 1),
        (1, 1),
        vec![part(0, (1, 1)), part(1, (1, 1))],
        vec![part(FAT_BODY, (1, 1))],
    );
    let tiny = body((1, 8), (1, 8), vec![], vec![part(FAT_BODY, (1, 8))]);

    // Arrangement A: the grounded producer is tiny; the orphan (prey id 9 is absent) is a giant.
    let mut a: BTreeMap<SpeciesId, Species> = BTreeMap::new();
    a.insert(
        SpeciesId(0),
        species(0, tiny.clone(), vec![SourceRef::Abiotic(0)]),
    );
    a.insert(
        SpeciesId(1),
        species(1, giant.clone(), vec![SourceRef::Species(SpeciesId(9))]),
    );
    let ga = grounded(&abiotic, &a);
    assert!(
        ga.contains(&SpeciesId(0)) && !ga.contains(&SpeciesId(1)),
        "the orphan is culled and the grounded producer survives, whatever their bodies"
    );

    // Arrangement B: swap the bodies (the grounded producer is now the giant). Topology unchanged.
    let mut b: BTreeMap<SpeciesId, Species> = BTreeMap::new();
    b.insert(SpeciesId(0), species(0, giant, vec![SourceRef::Abiotic(0)]));
    b.insert(
        SpeciesId(1),
        species(1, tiny, vec![SourceRef::Species(SpeciesId(9))]),
    );
    let gb = grounded(&abiotic, &b);

    assert_eq!(
        ga, gb,
        "the grounded set is identical under a body swap: the seed-time cull ignores morphology"
    );
}

#[test]
fn an_undersupplied_pool_collapses_under_the_stock_cull_and_does_not_revive() {
    // The runtime half of the cull: a population pool whose sustained demand exceeds its regeneration
    // collapses to zero and does not spontaneously revive (regeneration is zero at empty). A
    // birth-nonviable species leans on exactly this: its members die at once, so its pool draws no
    // sustaining return and collapses the same way. The Stock carries no morphology, so the collapse
    // cannot depend on a body plan.
    let mut pool = Stock::new(Fixed::ONE, Fixed::ONE, Fixed::from_ratio(1, 10));
    for _ in 0..1000 {
        pool.step(Fixed::ONE); // demand the whole capacity each tick, far above what it can regrow
    }
    assert!(
        pool.is_collapsed(),
        "sustained over-demand collapses the pool"
    );
    for _ in 0..1000 {
        pool.step(Fixed::ZERO); // no more demand
    }
    assert!(
        pool.is_collapsed(),
        "a collapsed pool does not spontaneously revive"
    );
}

#[test]
fn birth_viability_is_independent_of_body_mass_armor_and_weapons() {
    // The viability the cull leans on is a pure function of the organ set. Two bodies with the SAME
    // organs but wildly different mass, armor, and weaponry have the same verdict, so the cull cannot
    // correlate with any of those traits.
    let organs = BodyPlanRegistry::dev_default();
    let hreg = HomeostaticRegistry::dev_default();
    let light_bare = body((1, 50), (0, 1), vec![], vec![part(FAT_BODY, (1, 2))]);
    let giant_armored = body(
        (1, 1),
        (1, 1),
        vec![part(0, (1, 1)), part(1, (1, 1)), part(2, (1, 1))],
        vec![part(FAT_BODY, (1, 2))],
    );
    assert_eq!(
        birth_viable(&hreg, &light_bare, &organs),
        birth_viable(&hreg, &giant_armored, &organs),
        "viability tracks the organ set, not mass, armor, or weapons"
    );
    assert!(
        birth_viable(&hreg, &giant_armored, &organs),
        "both are viable: they bear an energy-storing organ"
    );
}

#[test]
fn the_cull_removes_only_the_reserveless_not_the_large_or_armored() {
    // The armored-giant case the owner raised, and its true nonviable neighbour. A huge, thickly
    // armored body with even one small energy organ is viable (it just holds small reserves). Only a
    // body that stores NO energy is culled, and that is physics: a creature with no energy reserve
    // cannot metabolize.
    let organs = BodyPlanRegistry::dev_default();
    let hreg = HomeostaticRegistry::dev_default();

    let armored_giant = body(
        (1, 1),
        (1, 1),
        vec![part(2, (1, 1))],
        vec![part(FAT_BODY, (1, 16))],
    );
    assert!(
        birth_viable(&hreg, &armored_giant, &organs),
        "an armored giant with even a tiny energy organ is viable; only its reserves are small"
    );

    let water_only = body((1, 1), (1, 1), vec![], vec![part(WATER_STORE, (1, 1))]);
    assert!(
        !birth_viable(&hreg, &water_only, &organs),
        "a creature that stores only water and no energy is physically nonviable"
    );
    assert_eq!(
        Homeostasis::new(&hreg, &water_only, &organs).dead_axis(&hreg),
        Some(ENERGY),
        "and the cause is the empty energy reserve, a physical fact, not an authored preference"
    );

    let organless = body((1, 1), (1, 1), vec![], vec![]);
    assert!(
        !birth_viable(&hreg, &organless, &organs),
        "a body with no organs holds no metabolic reserve and is nonviable"
    );
}

#[test]
fn culling_the_reserveless_selects_no_body_mass_class() {
    // The population-level steering audit. Over many generated species, viability is invariant to body
    // mass: recomputing it with the mass forced to each extreme never changes the verdict. So the cull
    // that removes the birth-nonviable cannot preferentially remove a light or a heavy body, and steers
    // no size distribution. The cull is also non-trivial and non-total: both classes occur.
    let organs = BodyPlanRegistry::dev_default();
    let hreg = HomeostaticRegistry::dev_default();
    let mut viable = 0u32;
    let mut nonviable = 0u32;

    for s in 0..300u64 {
        let rng = DrawKey::entity(s, 0, Phase::BIOSPHERE_SAMPLE).rng(0xB0D1);
        let plan = sample_body_plan(&rng, 2, Fixed::ZERO, &organs, WorldProfile::grounded(), 200);
        let verdict = birth_viable(&hreg, &plan, &organs);
        if verdict {
            viable += 1;
        } else {
            nonviable += 1;
            // The nonviable are exactly those with no energy-backing organ (a physical reason).
            assert_eq!(
                Homeostasis::new(&hreg, &plan, &organs).dead_axis(&hreg),
                Some(ENERGY),
                "a culled species is culled for an empty energy reserve, nothing else"
            );
        }

        // The load-bearing invariant: force the mass to each extreme, organs unchanged. The verdict
        // never moves, so no mass class is preferentially culled.
        let mut light = plan.clone();
        light.body_mass = Fixed::ZERO;
        let mut heavy = plan.clone();
        heavy.body_mass = Fixed::ONE;
        assert_eq!(
            birth_viable(&hreg, &light, &organs),
            verdict,
            "a lighter body has the same viability: mass does not enter the cull"
        );
        assert_eq!(
            birth_viable(&hreg, &heavy, &organs),
            verdict,
            "a heavier body has the same viability: mass does not enter the cull"
        );
    }

    assert!(
        viable > 0 && nonviable > 0,
        "the cull is non-trivial and non-total: {viable} viable, {nonviable} nonviable"
    );
}

#[test]
fn a_parts_weapon_function_is_a_pure_physics_read_with_no_layer_or_race_key() {
    // Emergent-anatomy step one, the derive-not-tag steering guarantee extended to CAPABILITY: a part's
    // weapon function is DERIVED from its own geometry and material (Body::can_strike over the compose
    // function-law dispatch), never from an authored F_STRIKE tag, and the read keys on no layer, kingdom,
    // niche, or race. The proof: the SAME weapon (a claws part, kind 0) reads the identical can_strike
    // whatever the body around it, because the derive reads only the weapon part's physics.
    let params = BodyParams::dev_default();
    let reg = BodyPlanRegistry::dev_default();

    // Two bodies sharing one weapon (claws, kind 0) but differing wildly in every other trait: a light
    // bare producer-shaped body and a heavy armored one with extra organs. can_strike on the weapon must
    // be identical, because the surrounding body, its mass, its armor, and its notional layer do not enter
    // the read.
    let light = body(
        (1, 50),
        (0, 1),
        vec![part(0, (1, 2))],
        vec![part(FAT_BODY, (1, 8))],
    );
    let heavy = body(
        (1, 1),
        (1, 1),
        vec![part(0, (1, 2))],
        vec![part(FAT_BODY, (1, 1)), part(WATER_STORE, (1, 1))],
    );
    let bl = Body::from_body_plan(&light, BLOOD, &params, &reg);
    let bh = Body::from_body_plan(&heavy, BLOOD, &params, &reg);
    let wl = bl.parts.len() - 1;
    let wh = bh.parts.len() - 1;
    assert!(
        bl.can_strike(wl, &params),
        "the claws part is a weapon by its physics"
    );
    assert_eq!(
        bl.can_strike(wl, &params),
        bh.can_strike(wh, &params),
        "the same weapon reads the same can_strike whatever the body, mass, armor, or layer around it"
    );

    // A body carrying no weapon reads no strike, again by physics (no weapon geometry), not a missing tag.
    let unarmed = body((1, 1), (1, 1), vec![], vec![part(FAT_BODY, (1, 2))]);
    let bu = Body::from_body_plan(&unarmed, BLOOD, &params, &reg);
    for i in 0..bu.parts.len() {
        assert!(
            !bu.can_strike(i, &params),
            "no part of a weaponless body strikes: weapon-ness is a physics read, not a tag"
        );
    }
}
