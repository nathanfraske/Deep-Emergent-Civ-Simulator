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

//! The non-steering audit for the derived resting metabolism and body-to-medium exchange
//! (R-METABOLIZE, Principle 9), modelled on the physics substrate's
//! `a_symmetric_kernel_carries_no_hidden_bias`. It converts the substrate's load-bearing invariant from
//! a prose claim into a build-enforced check: the derived base drain and body-exchange rate are pure
//! functions of a body's COMPOSITION, MASS, MEDIUM, and TEMPERATURE, and read no identity. Two bodies
//! diverge because their physics differ, and a body cannot get a different drain by relabelling itself.

use civsim_core::Fixed;
use civsim_sim::anatomy::{
    BodyPlan, BodyPlanRegistry, OrganKindDef, Part, Temperament, TissueComposition,
};
use civsim_sim::homeostasis::{Homeostasis, HomeostaticRegistry, ENERGY};
use civsim_sim::physiology::{
    derive_base_drain, derive_body_exchange_rate, whole_body_energy_density, MetabolicAnchors,
    CONVECTIVE_SURFACE, ENERGY_DENSITY, TISSUE_SPECIFIC_HEAT,
};

/// A registry adding a high-surface water-rich tissue and a low-surface energy-dense tissue at known
/// ids, alongside the defaults. Labelled fixtures.
fn registry() -> (BodyPlanRegistry, u16, u16) {
    let mut reg = BodyPlanRegistry::dev_default();
    // A high-surface, water-rich tissue (a gilled/skinned water-rich body): much convective surface,
    // high specific heat (water), a modest energy density (energy density on the floor's kJ/g scale).
    let watery = reg.organs.len() as u16;
    reg.organs.push(OrganKindDef {
        id: watery,
        name: "watery-skin".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[
            (CONVECTIVE_SURFACE, Fixed::from_int(2)),
            (TISSUE_SPECIFIC_HEAT, Fixed::from_int(4186)),
            (ENERGY_DENSITY, Fixed::from_int(8)),
        ]),
    });
    // A low-surface, energy-dense, insulated tissue (a compact fat body): little convective surface, low
    // specific heat, a high energy density (rendered fat approaching the floor's gross-energy ceiling).
    let dense = reg.organs.len() as u16;
    reg.organs.push(OrganKindDef {
        id: dense,
        name: "dense-fat".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[
            (CONVECTIVE_SURFACE, Fixed::from_ratio(1, 4)),
            (TISSUE_SPECIFIC_HEAT, Fixed::from_int(2000)),
            (ENERGY_DENSITY, Fixed::from_int(35)),
        ]),
    });
    (reg, watery, dense)
}

fn temperament() -> Temperament {
    Temperament {
        boldness: Fixed::from_ratio(1, 2),
        exploration: Fixed::from_ratio(1, 2),
        activity: Fixed::from_ratio(1, 2),
        sociability: Fixed::from_ratio(1, 2),
        aggression: Fixed::from_ratio(1, 4),
    }
}

fn body(mass: (i64, i64), organs: Vec<Part>) -> BodyPlan {
    BodyPlan {
        body_mass: Fixed::from_ratio(mass.0, mass.1),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons: vec![],
        covering: Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        },
        senses: vec![],
        locomotion: vec![1],
        organs,
        temperament: temperament(),
    }
}

fn organ(kind: u16, dev: (i64, i64)) -> Part {
    Part {
        kind,
        development: Fixed::from_ratio(dev.0, dev.1),
    }
}

fn energy_cap(reg: &HomeostaticRegistry, plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    Homeostasis::new(reg, plan, organs).capacity(ENERGY)
}

#[test]
fn two_bodies_diverge_in_derived_drain_and_exchange_from_composition_and_mass_alone() {
    // A large, low-surface, energy-dense body versus a small, high-surface, water-rich body: nothing
    // labels either, yet their derived base drain and body-to-medium exchange rate differ, purely from
    // their composition and mass. This is the Principle-9 outcome: physics in, divergence out, no tag.
    let (organs, watery, dense) = registry();
    let reg = HomeostaticRegistry::dev_default();
    let anchors = MetabolicAnchors::dev_fixture();
    let setpoint = Fixed::from_int(310);
    let ambient = Fixed::from_int(280);
    let tick = Fixed::ONE;

    let big_dense = body((1, 1), vec![organ(dense, (1, 1))]);
    let small_watery = body((1, 4), vec![organ(watery, (1, 1))]);
    let cap_big = energy_cap(&reg, &big_dense, &organs);
    let cap_small = energy_cap(&reg, &small_watery, &organs);

    let drain_big = derive_base_drain(
        &big_dense,
        &organs,
        cap_big,
        whole_body_energy_density(&big_dense, &organs),
        ambient,
        setpoint,
        Fixed::from_int(10),
        tick,
        &anchors,
    );
    let drain_small = derive_base_drain(
        &small_watery,
        &organs,
        cap_small,
        whole_body_energy_density(&small_watery, &organs),
        ambient,
        setpoint,
        Fixed::from_int(10),
        tick,
        &anchors,
    );
    let rate_big =
        derive_body_exchange_rate(&big_dense, &organs, Fixed::from_int(10), tick, &anchors);
    let rate_small =
        derive_body_exchange_rate(&small_watery, &organs, Fixed::from_int(10), tick, &anchors);

    assert_ne!(
        drain_big.to_bits(),
        drain_small.to_bits(),
        "two bodies with different composition and mass derive different base drains"
    );
    assert_ne!(
        rate_big.to_bits(),
        rate_small.to_bits(),
        "and different body-to-medium exchange rates"
    );
    // The high-surface, water-rich body couples to the medium faster (more surface over its thermal
    // mass), the physically-expected direction.
    assert!(
        rate_small > rate_big,
        "the high-surface water-rich body couples faster than the compact dense one"
    );
}

#[test]
fn a_hot_body_in_the_cold_and_its_mirror_diverge_from_temperature_alone() {
    // A hot-set-point body in a cold medium versus its mirror (a cold-set-point body in a hot medium):
    // the SAME body and anchors, only the set point and ambient swap. The derived resting drain differs,
    // because the thermoregulatory heat loss reads the actual temperatures (the hot body in the cold
    // radiates and convects heat away; the cold body in the hot medium radiates none). The divergence is
    // temperature, never a race label.
    let (organs, watery, _dense) = registry();
    let reg = HomeostaticRegistry::dev_default();
    let anchors = MetabolicAnchors::dev_fixture();
    let plan = body((1, 2), vec![organ(watery, (1, 1))]);
    let cap = energy_cap(&reg, &plan, &organs);
    let tick = Fixed::ONE;

    let hot_in_cold = derive_base_drain(
        &plan,
        &organs,
        cap,
        whole_body_energy_density(&plan, &organs),
        Fixed::from_int(270),
        Fixed::from_int(330),
        Fixed::from_int(10),
        tick,
        &anchors,
    );
    let cold_in_hot = derive_base_drain(
        &plan,
        &organs,
        cap,
        whole_body_energy_density(&plan, &organs),
        Fixed::from_int(330),
        Fixed::from_int(270),
        Fixed::from_int(10),
        tick,
        &anchors,
    );
    assert_ne!(
        hot_in_cold.to_bits(),
        cold_in_hot.to_bits(),
        "the hot-set-point-in-cold body and its mirror derive different drains from temperature alone"
    );
    // A body exactly at its medium temperature pays only the basal drain (no thermoregulatory term).
    let thermoneutral = derive_base_drain(
        &plan,
        &organs,
        cap,
        whole_body_energy_density(&plan, &organs),
        Fixed::from_int(310),
        Fixed::from_int(310),
        Fixed::from_int(10),
        tick,
        &anchors,
    );
    assert!(
        hot_in_cold > thermoneutral,
        "shedding heat to a cold medium costs more than resting at the medium temperature"
    );
}

#[test]
fn identical_physics_derive_identical_results_and_replay_bit_for_bit() {
    // The anti-steering guarantee's other half: two bodies with IDENTICAL composition, mass, medium, and
    // temperature derive EXACTLY the same drain and rate (no hidden per-call or identity-keyed variation),
    // and a re-run reproduces the bits.
    let (organs, watery, _dense) = registry();
    let reg = HomeostaticRegistry::dev_default();
    let anchors = MetabolicAnchors::dev_fixture();
    let a = body((3, 5), vec![organ(watery, (2, 3))]);
    let b = body((3, 5), vec![organ(watery, (2, 3))]);
    let cap_a = energy_cap(&reg, &a, &organs);
    let cap_b = energy_cap(&reg, &b, &organs);
    let go = |p: &BodyPlan, cap: Fixed| {
        (
            derive_base_drain(
                p,
                &organs,
                cap,
                whole_body_energy_density(p, &organs),
                Fixed::from_int(270),
                Fixed::from_int(310),
                Fixed::from_int(10),
                Fixed::ONE,
                &anchors,
            )
            .to_bits(),
            derive_body_exchange_rate(p, &organs, Fixed::from_int(10), Fixed::ONE, &anchors)
                .to_bits(),
        )
    };
    assert_eq!(
        go(&a, cap_a),
        go(&b, cap_b),
        "identical physics, identical result"
    );
    assert_eq!(
        go(&a, cap_a),
        go(&a, cap_a),
        "the derivation replays bit for bit"
    );
}

#[test]
fn the_metabolism_kernels_read_no_race_identity() {
    // A structural guarantee, the sibling of the physics substrate's identity-blindness check: the
    // derivation module takes bodies, organs, and physics values, never a RaceId, so no body can steer
    // its metabolism by its label. A future edit that reaches for a race id here fails this build.
    let src = include_str!("../src/physiology.rs");
    assert!(
        !src.contains("RaceId"),
        "the metabolism derivation must read no RaceId: the drain is physics, not identity (Principle 9)"
    );
}
