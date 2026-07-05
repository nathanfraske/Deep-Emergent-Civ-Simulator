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

//! The milestone for world-wiring increment 10 Part B: the anatomy-derived physiology producers LIVE in
//! the Runner embodiment step (R-METABOLIZE, R-MEDIUM; Principles 3, 9, 11). It proves the wiring, not
//! the producers (their kernels are unit-tested in `physiology.rs`, `homeostasis.rs`, and `medium.rs`):
//! two embodied beings with different body plans diverge in energy drain per tick and in time-to-death
//! from ANATOMY alone, through one kernel with no race or label branch, and a larger, denser body drains
//! a SMALLER fraction per tick (the Kleiber signature). It also proves the respiration sub-phase (a body
//! lives in a rich medium and suffocates in a poor one, from the medium's content, not a label) and that
//! the coupled runner is bit-identical across the scheduler variant and reproduces across runs.

use civsim_core::{Fixed, StableId};
use civsim_sim::anatomy::{
    BodyPlan, BodyPlanRegistry, OrganKindDef, Part, Temperament, TissueComposition,
};
use civsim_sim::controller::Controller;
use civsim_sim::edibility::Physiology;
use civsim_sim::homeostasis::{
    AffordanceRegistry, Homeostasis, HomeostaticAxisDef, HomeostaticRegistry, ENERGY, TEMPERATURE,
};
use civsim_sim::locomotion::{LocomotionParams, Walker};
use civsim_sim::medium::{MediumField, RESPIRATORY_SURFACE};
use civsim_sim::percept::PerceptRegistry;
use civsim_sim::physiology::ENERGY_DENSITY;
use civsim_sim::runner::{BeingThermal, EmbodiedPhysiology, Embodiment, Field, FieldCalib, Runner};
use civsim_world::Coord3;

/// A viable core-temperature band around a set point, with the being's spawn core temperature.
/// Labelled fixture, not owner canon.
fn band(setpoint: i32) -> BeingThermal {
    BeingThermal {
        setpoint: Fixed::from_int(setpoint),
        half_band: Fixed::from_int(8),
        initial_temp: Fixed::from_int(setpoint),
    }
}

/// Labelled field calibrations (within the documented bounds). A fixture, never owner canon.
fn calib() -> FieldCalib {
    FieldCalib {
        diffusion: Fixed::from_ratio(1, 16),
        relaxation: Fixed::from_ratio(1, 4),
        exchange: Fixed::from_ratio(1, 2),
    }
}

/// A uniform field at one temperature (a fixed point of the step: every cell equal), so a being's cell
/// temperature holds.
fn uniform_field(w: i32, h: i32, temp: Fixed) -> Field {
    Field::new(w, h, vec![temp; (w * h) as usize])
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

fn organ(kind: u16, dev: (i64, i64)) -> Part {
    Part {
        kind,
        development: Fixed::from_ratio(dev.0, dev.1),
    }
}

/// A body of a given mass bearing the given organs (mobile, so it carries a locomotion organ, but its
/// controller here is the blank resting controller so it never moves and stays at rest exertion).
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

/// An organ registry with an energy-store tissue (energy density) at a known id, alongside the
/// defaults. Labelled fixture.
fn energy_registry() -> (BodyPlanRegistry, u16) {
    let mut reg = BodyPlanRegistry::dev_default();
    let fat = reg.organs.len() as u16;
    reg.organs.push(OrganKindDef {
        id: fat,
        name: "energy-store".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[(ENERGY_DENSITY, Fixed::ONE)]),
    });
    (reg, fat)
}

/// A homeostatic registry with only a draining energy axis and the non-draining temperature axis the
/// embodiment requires, so time-to-death is governed purely by the derived energy drain (no water or
/// oxygen confound). Labelled fixture.
fn energy_thermal_registry() -> HomeostaticRegistry {
    HomeostaticRegistry {
        axes: vec![
            HomeostaticAxisDef {
                id: ENERGY,
                name: "energy".to_string(),
                backing_component: Some("bio.energy_density".to_string()),
                capacity_per_mass: Fixed::ONE,
                // These authored drains are OVERRIDDEN by the derived drain on the embodied+physiology
                // path; they stand only for a caller that runs the scalar path.
                base_drain: Fixed::from_ratio(1, 400),
                exertion_drain: Fixed::from_ratio(1, 100),
                death_floor: Fixed::ZERO,
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
            },
        ],
    }
}

/// A being at a tile with a full anatomy-derived homeostasis and the blank resting controller.
fn resting_walker(
    id: u64,
    tile: Coord3,
    plan: BodyPlan,
    reg: &HomeostaticRegistry,
    organs: &BodyPlanRegistry,
    controller: Controller,
) -> Walker {
    Walker::new(
        StableId(id),
        tile,
        plan.clone(),
        Homeostasis::new(reg, &plan, organs),
        Physiology::dev_for_registry(reg),
        controller,
    )
}

#[test]
fn two_body_plans_diverge_in_energy_drain_and_time_to_death_from_anatomy_alone() {
    // The milestone: two embodied beings that differ ONLY in body plan (a large, dense, energy-rich body
    // and a small one) diverge in their per-tick energy drain and their time-to-death through the LIVE
    // embodiment step, with no race or label branch. The field is thermoneutral (uniform at the set
    // point) so the divergence isolates the Kleiber basal term: the larger, denser body drains a SMALLER
    // fraction of its reserve per tick and so outlives the small one.
    let setpoint = 310;
    let (organs, fat) = energy_registry();
    let reg = energy_thermal_registry();

    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        0,
        0xB0D1,
    );
    let blank = Controller::zeros(emb.layout());
    // Large, dense body: full mass, a full energy store. Small body: a quarter mass, a quarter store.
    let large = body((1, 1), vec![organ(fat, (1, 1))]);
    let small = body((1, 4), vec![organ(fat, (1, 4))]);
    emb.add(
        resting_walker(1, Coord3::ground(2, 2), large, &reg, &organs, blank.clone()),
        band(setpoint),
    );
    emb.add(
        resting_walker(2, Coord3::ground(5, 5), small, &reg, &organs, blank),
        band(setpoint),
    );
    // Install the anatomy-derived physiology: this is the wiring under test. Medium content is
    // irrelevant here (no respiration axis), so a uniform field over the runner's extent.
    emb.set_physiology(EmbodiedPhysiology::dev_fixture(
        organs,
        MediumField::uniform(10, 10, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
    ));

    let mut runner = Runner::with_embodiment(
        uniform_field(10, 10, Fixed::from_int(setpoint)),
        calib(),
        emb,
    );

    // One tick, then read each being's per-tick energy-reserve drop (its drain fraction).
    let level = |r: &Runner, id: u64| -> Fixed {
        r.embodiment()
            .unwrap()
            .walkers()
            .iter()
            .find(|w| w.id == StableId(id))
            .unwrap()
            .homeostasis
            .level(ENERGY)
    };
    let full_large = level(&runner, 1);
    let full_small = level(&runner, 2);
    assert_eq!(
        full_large,
        Fixed::ONE,
        "the large body starts at full energy"
    );
    assert_eq!(
        full_small,
        Fixed::ONE,
        "the small body starts at full energy"
    );

    runner.step();
    let drain_large = full_large - level(&runner, 1);
    let drain_small = full_small - level(&runner, 2);
    assert!(
        drain_large > Fixed::ZERO && drain_small > Fixed::ZERO,
        "both bodies pay a nonzero resting drain through the live embodiment step"
    );
    assert!(
        drain_small > drain_large,
        "the larger, denser body drains a SMALLER fraction per tick (Kleiber): large {drain_large:?} < small {drain_small:?}"
    );

    // Run until each dies; the small body dies first (it spends its reserve faster).
    let mut death_large = None;
    let mut death_small = None;
    for t in 1..100_000u64 {
        runner.step();
        let alive = |r: &Runner, id: u64| {
            r.embodiment()
                .unwrap()
                .walkers()
                .iter()
                .find(|w| w.id == StableId(id))
                .unwrap()
                .alive
        };
        if death_large.is_none() && !alive(&runner, 1) {
            death_large = Some(t);
        }
        if death_small.is_none() && !alive(&runner, 2) {
            death_small = Some(t);
        }
        if death_large.is_some() && death_small.is_some() {
            break;
        }
    }
    let death_large = death_large.expect("the large body eventually starves");
    let death_small = death_small.expect("the small body eventually starves");
    assert!(
        death_small < death_large,
        "the small body dies first, the large body outlives it from anatomy alone: small {death_small} < large {death_large}"
    );
}

#[test]
fn the_coupled_physiology_runner_replays_and_matches_the_scheduler() {
    // Determinism guard: with the derived physiology live, the coupled runner is bit-identical under the
    // pinned order and the scheduler variant, and reproduces exactly across runs. The derived drains are
    // recomputed each tick and the exchange rate is static config, so no new canonical state escapes the
    // existing fold (reserves and body temperatures are already folded into state_hash).
    let setpoint = 305;
    let build = || -> Runner {
        let (organs, fat) = energy_registry();
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x5EED,
        );
        let blank = Controller::zeros(emb.layout());
        emb.add(
            resting_walker(
                1,
                Coord3::ground(1, 1),
                body((3, 4), vec![organ(fat, (3, 4))]),
                &reg,
                &organs,
                blank.clone(),
            ),
            band(setpoint),
        );
        emb.add(
            resting_walker(
                2,
                Coord3::ground(6, 4),
                body((1, 3), vec![organ(fat, (1, 2))]),
                &reg,
                &organs,
                blank,
            ),
            band(setpoint),
        );
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(8, 8, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
        ));
        // A slightly colder field than the set point, so the thermoregulatory term is live (the base
        // drain reads the live body temperature each tick) rather than a thermoneutral constant.
        Runner::with_embodiment(uniform_field(8, 8, Fixed::from_int(300)), calib(), emb)
    };

    let trace = |mut r: Runner, scheduled: bool| -> Vec<u128> {
        (0..60)
            .map(|_| {
                if scheduled {
                    r.step_scheduled(&[]);
                } else {
                    r.step();
                }
                r.state_hash()
            })
            .collect()
    };

    let pinned_a = trace(build(), false);
    let pinned_b = trace(build(), false);
    assert_eq!(
        pinned_a, pinned_b,
        "the coupled physiology runner did not replay bit for bit"
    );
    let scheduled = trace(build(), true);
    assert_eq!(
        pinned_a, scheduled,
        "the scheduler variant diverged from the pinned order with the physiology live"
    );
}

#[test]
fn the_interoceptive_delta_fold_is_deterministic_and_scheduler_invariant() {
    // Harm-learning arc slice a determinism guard: with the feature percept declared, each being's
    // interoceptive reserve-delta memory is new per-being dynamic state that folds into state_hash. It
    // is snapshotted in the serial embodiment phase in canonical id order and draws no randomness, so
    // the coupled runner must stay bit-identical across runs and between the pinned order and the
    // scheduler variant. Two resting beings whose reserves drain each tick exercise the fold with real
    // changing levels; the run without percepts stays hash-neutral (carried by every existing suite).
    let setpoint = 305;
    let build = || -> Runner {
        let (organs, fat) = energy_registry();
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x5A17,
        );
        // Opt into the feature percept: this rebuilds the layout to carry the feature block and turns
        // on the per-being reserve-delta snapshot. The blank controller is expressed against the
        // rebuilt layout so its weight vector is the right length.
        emb.set_percepts(PerceptRegistry::dev_salinity());
        let blank = Controller::zeros(emb.layout());
        emb.add(
            resting_walker(
                1,
                Coord3::ground(1, 1),
                body((3, 4), vec![organ(fat, (3, 4))]),
                &reg,
                &organs,
                blank.clone(),
            ),
            band(setpoint),
        );
        emb.add(
            resting_walker(
                2,
                Coord3::ground(6, 4),
                body((1, 3), vec![organ(fat, (1, 2))]),
                &reg,
                &organs,
                blank,
            ),
            band(setpoint),
        );
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(8, 8, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
        ));
        Runner::with_embodiment(uniform_field(8, 8, Fixed::from_int(300)), calib(), emb)
    };

    let trace = |mut r: Runner, scheduled: bool| -> Vec<u128> {
        (0..50)
            .map(|_| {
                if scheduled {
                    r.step_scheduled(&[]);
                } else {
                    r.step();
                }
                r.state_hash()
            })
            .collect()
    };

    let pinned_a = trace(build(), false);
    let pinned_b = trace(build(), false);
    assert_eq!(
        pinned_a, pinned_b,
        "the feature-percept runner did not replay bit for bit"
    );
    let scheduled = trace(build(), true);
    assert_eq!(
        pinned_a, scheduled,
        "the interoceptive-delta fold diverged between the pinned order and the scheduler"
    );
}

#[test]
fn the_material_substrate_folds_into_state_hash_and_stays_deterministic() {
    // Material-substrate arc slice 1 WIRE determinism guard: a populated material layer is canonical
    // dynamic state that folds into state_hash beside the resource field, in canonical (Coord3,
    // substance-id, volume) order and drawing no randomness, so a runner carrying matter replays bit
    // for bit and folds identically between the pinned order and the scheduler variant. An empty
    // material layer (every existing scenario) folds no bytes, so the fold is byte-identical there
    // (carried by every existing suite); this test proves the fold is live and order-invariant.
    use civsim_sim::material::{MaterialField, SubstanceMix};

    let setpoint = 305;
    let material = || -> MaterialField {
        let mut field = MaterialField::new();
        let mut ground = SubstanceMix::new();
        ground.set("granite", Fixed::from_int(4));
        ground.set("soil", Fixed::from_int(1));
        field.set_cell(Coord3::ground(2, 3), ground);
        // A subsurface ore deposit, exercising a non-ground z and the deposit path.
        field.deposit(Coord3 { x: 5, y: 1, z: -1 }, "ore", Fixed::from_int(2));
        field
    };
    let build = |with_material: bool| -> Runner {
        let (organs, fat) = energy_registry();
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x5A17,
        );
        if with_material {
            emb.set_material(material());
        }
        let blank = Controller::zeros(emb.layout());
        emb.add(
            resting_walker(
                1,
                Coord3::ground(1, 1),
                body((3, 4), vec![organ(fat, (3, 4))]),
                &reg,
                &organs,
                blank,
            ),
            band(setpoint),
        );
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(8, 8, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
        ));
        Runner::with_embodiment(uniform_field(8, 8, Fixed::from_int(300)), calib(), emb)
    };

    let trace = |mut r: Runner, scheduled: bool| -> Vec<u128> {
        (0..30)
            .map(|_| {
                if scheduled {
                    r.step_scheduled(&[]);
                } else {
                    r.step();
                }
                r.state_hash()
            })
            .collect()
    };

    // With matter in the ground, the coupled runner replays bit for bit and folds identically between
    // the pinned order and the scheduler variant.
    let pinned_a = trace(build(true), false);
    let pinned_b = trace(build(true), false);
    assert_eq!(
        pinned_a, pinned_b,
        "the material-bearing runner did not replay bit for bit"
    );
    let scheduled = trace(build(true), true);
    assert_eq!(
        pinned_a, scheduled,
        "the material fold diverged between the pinned order and the scheduler"
    );

    // The fold is live: a populated material layer changes the hash versus an empty one (the opt-out
    // state every existing scenario stays in), so the substrate is part of the canonical state.
    let empty = trace(build(false), false);
    assert_ne!(
        pinned_a, empty,
        "folding the material layer left the hash unchanged, so it is not canonical"
    );
}

#[test]
fn a_carried_load_folds_into_state_hash_and_stays_deterministic() {
    // Material-substrate arc item 3 (the carry substrate): a being's carried load is per-being dynamic
    // state folded into state_hash after the reserve memory, in canonical (substance-id, volume) order
    // with no randomness, so a runner whose being bears matter replays bit for bit and folds identically
    // between the pinned order and the scheduler variant. A being carrying nothing (every existing
    // scenario) folds no bytes, so the fold is byte-identical there; this proves it is live and
    // order-invariant.
    use civsim_sim::material::SubstanceMix;

    let setpoint = 305;
    let build = |carrying: bool| -> Runner {
        let (organs, fat) = energy_registry();
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x5A17,
        );
        let blank = Controller::zeros(emb.layout());
        let mut w = resting_walker(
            1,
            Coord3::ground(1, 1),
            body((3, 4), vec![organ(fat, (3, 4))]),
            &reg,
            &organs,
            blank,
        );
        if carrying {
            let mut load = SubstanceMix::new();
            load.set("granite", Fixed::from_int(2));
            load.set("hematite", Fixed::from_int(1));
            w.carried = load;
        }
        emb.add(w, band(setpoint));
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(8, 8, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
        ));
        Runner::with_embodiment(uniform_field(8, 8, Fixed::from_int(300)), calib(), emb)
    };
    let trace = |mut r: Runner, scheduled: bool| -> Vec<u128> {
        (0..30)
            .map(|_| {
                if scheduled {
                    r.step_scheduled(&[]);
                } else {
                    r.step();
                }
                r.state_hash()
            })
            .collect()
    };
    let pinned_a = trace(build(true), false);
    let pinned_b = trace(build(true), false);
    assert_eq!(
        pinned_a, pinned_b,
        "the carrying runner did not replay bit for bit"
    );
    let scheduled = trace(build(true), true);
    assert_eq!(
        pinned_a, scheduled,
        "the carried fold diverged between the pinned order and the scheduler"
    );
    let empty = trace(build(false), false);
    assert_ne!(
        pinned_a, empty,
        "folding the carried load left the hash unchanged, so it is not canonical"
    );
}

#[test]
fn a_being_picks_up_and_puts_down_matter_bounded_by_its_grown_strength() {
    // Material-substrate arc item 3, the carry hinge: a being takes matter from the ground into its
    // carried load, as much as its grown whole-body muscle force bears against the load's derived weight
    // and no more, and puts it back conserved. The limit is grown strength versus physics-derived weight,
    // never a per-race carry table, so a being with no muscle carries nothing and a strong being is
    // bounded by its strength rather than by the size of the heap.
    use civsim_sim::material::MaterialField;
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    let cell = Coord3::ground(1, 1);
    let build = |with_muscle: bool, with_registry: bool| -> Embodiment {
        let (mut organs, fat) = energy_registry();
        let mut parts = vec![organ(fat, (1, 2))];
        if with_muscle {
            let muscle = organs.organs.len() as u16;
            organs.organs.push(OrganKindDef {
                id: muscle,
                name: "muscle".to_string(),
                fantasy: false,
                composition: TissueComposition::from_pairs(&[(MUSCLE_STRENGTH, Fixed::ONE)]),
            });
            parts.push(organ(muscle, (1, 1)));
        }
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x5A17,
        );
        let blank = Controller::zeros(emb.layout());
        emb.add(
            resting_walker(1, cell, body((3, 4), parts), &reg, &organs, blank),
            band(305),
        );
        let mut field = MaterialField::new();
        field.deposit(cell, "granite", Fixed::from_int(100000));
        emb.set_material(field);
        if with_registry {
            emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        }
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(8, 8, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
        ));
        emb
    };

    // Without a material registry the load's weight cannot be derived, so the carry actions no-op.
    let mut no_reg = build(true, false);
    assert_eq!(
        no_reg.pick_up(StableId(1), cell, "granite", Fixed::from_int(1)),
        Fixed::ZERO
    );

    // A being with no muscle has zero carry capacity and lifts nothing.
    let mut weak = build(false, true);
    assert_eq!(
        weak.pick_up(StableId(1), cell, "granite", Fixed::from_int(1)),
        Fixed::ZERO
    );

    // A strong being lifts some granite, bounded by its strength: asking for the whole heap it takes a
    // positive but limited amount, its strength is then spent (a second pick-up takes nothing more), and
    // the heap still holds granite (the bound was strength, not availability). Conservation holds: the
    // ground lost exactly what the being took.
    let mut strong = build(true, true);
    let taken = strong.pick_up(StableId(1), cell, "granite", Fixed::from_int(100000));
    assert!(taken > Fixed::ZERO, "a strong being lifts some granite");
    assert_eq!(
        strong.pick_up(StableId(1), cell, "granite", Fixed::from_int(100000)),
        Fixed::ZERO,
        "its strength is spent, so it cannot bear more"
    );
    assert_eq!(
        strong.material().volume(cell, "granite"),
        Fixed::from_int(100000) - taken,
        "the ground lost exactly what was taken"
    );

    // Put it back down: the load returns to the ground, conserved.
    let dropped = strong.put_down(StableId(1), cell, "granite", Fixed::from_int(100000));
    assert_eq!(dropped, taken, "the being sets down all it carried");
    assert_eq!(
        strong.material().volume(cell, "granite"),
        Fixed::from_int(100000),
        "the granite is fully restored to the ground"
    );
}

#[test]
fn a_being_grasps_matter_only_through_an_evolved_controller_weight() {
    // Material-substrate arc item 3, THE DRIVER: a being picks matter up only because its evolved
    // controller decided to, never because the engine scripts "beings carry". Two beings share one
    // embodiment, one body plan, one strength, and one granite heap each; they differ in ONE gene, the
    // controller weight feeding the grasp output. The one whose grasp weight selection has lifted off zero
    // grasps its heap (its cell loses matter, its carried load fills, bounded by its grown strength); the
    // blank-controller founder, expressing zero on that channel, never grasps though it stands on the same
    // matter and affords the same operation. This is the emergent pattern (Principles 8, 9): the affordance
    // and the physics are fixed, the DECISION is an evolved phenotype, and a founder does nothing until
    // selection gives it a reason to. The selection pressure that lifts the grasp weight (a need the carried
    // matter serves) arrives with item 4's extraction contest; here the mechanism is proven at the decision.
    use civsim_sim::material::MaterialField;
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    let grasper_cell = Coord3::ground(2, 2);
    let founder_cell = Coord3::ground(5, 5);

    // A shared organ registry with an energy store and a full muscle (so both bodies have carry capacity).
    let (mut organs, fat) = energy_registry();
    let muscle = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: muscle,
        name: "muscle".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[(MUSCLE_STRENGTH, Fixed::ONE)]),
    });
    let reg = energy_thermal_registry();

    // The carrier affordance registry adds GRASP (move, ingest, grasp), so the layout carries a grasp
    // output the controller can drive. Every existing scenario keeps the two-affordance dev_default and is
    // untouched.
    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_carrier(),
        LocomotionParams::dev_default(),
        0,
        0x64A5,
    );

    // The grasp output is the fifth output of the carrier layout (move [act,dx,dy] at 0..2, ingest [act]
    // at 3, grasp [act] at 4), so the grasp being's controller is a single nonzero weight: the bias input
    // (the always-on last input) driving the grasp activation to one, every other weight zero, so grasp
    // wins its decision over the resting move and ingest outputs. A reaction-norm weight feeding output o
    // from input i is index o * n_in + i.
    assert_eq!(
        emb.layout().n_out(),
        5,
        "the carrier layout has move(3) + ingest(1) + grasp(1) outputs"
    );
    let n_in = emb.layout().n_in();
    let grasp_out = 4usize;
    let bias = n_in - 1;
    let mut grasp_weights = vec![Fixed::ZERO; emb.layout().weight_count()];
    grasp_weights[grasp_out * n_in + bias] = Fixed::ONE;
    let grasp_controller = Controller::from_weights(n_in, emb.layout().n_out(), 0, grasp_weights);
    let blank = Controller::zeros(emb.layout());

    let plan = || body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]);
    emb.add(
        resting_walker(1, grasper_cell, plan(), &reg, &organs, grasp_controller),
        band(305),
    );
    emb.add(
        resting_walker(2, founder_cell, plan(), &reg, &organs, blank),
        band(305),
    );

    // A granite heap under each being, identical.
    let heap = Fixed::from_int(100000);
    let mut field = MaterialField::new();
    field.deposit(grasper_cell, "granite", heap);
    field.deposit(founder_cell, "granite", heap);
    emb.set_material(field);
    emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
    emb.set_physiology(EmbodiedPhysiology::dev_fixture(
        organs,
        MediumField::uniform(10, 10, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
    ));

    let mut runner =
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb);
    runner.step();

    let carried = |r: &Runner, id: u64| -> Fixed {
        r.embodiment()
            .unwrap()
            .walkers()
            .iter()
            .find(|w| w.id == StableId(id))
            .unwrap()
            .carried
            .total_volume()
    };
    let ground = |r: &Runner, coord: Coord3| -> Fixed {
        r.embodiment().unwrap().material().volume(coord, "granite")
    };

    // The evolved-weight being grasped: it now carries a positive load, its heap lost exactly that much,
    // and the amount is bounded by its strength (it did not scoop the whole heap).
    let grasped = carried(&runner, 1);
    assert!(
        grasped > Fixed::ZERO,
        "the being whose grasp weight is lifted picks matter up"
    );
    assert_eq!(
        ground(&runner, grasper_cell),
        heap - grasped,
        "its cell lost exactly what it carried (conservation)"
    );
    assert!(
        ground(&runner, grasper_cell) > Fixed::ZERO,
        "its strength, not the heap, bounded the lift: matter remains"
    );

    // The blank founder, expressing zero on the grasp channel, never grasped though it stood on the same
    // matter and afforded the same operation: it carries nothing and its heap is untouched.
    assert_eq!(
        carried(&runner, 2),
        Fixed::ZERO,
        "the blank founder does not grasp"
    );
    assert_eq!(
        ground(&runner, founder_cell),
        heap,
        "the founder's heap is untouched"
    );
}

#[test]
fn a_being_mines_bonded_rock_only_when_it_decides_to_and_can_fracture_it() {
    // Material-substrate arc item 4, THE EXTRACTION CONTEST wired into the run: a being breaks bonded
    // matter loose only because its evolved controller decided to (the EXTRACT affordance) AND its contact
    // pressure clears the rock's fracture-gating hardness (the physics contest). Two axes, both proven
    // here: the DECISION (an evolved extract weight, a blank founder never mines) and the PHYSICS (the same
    // deciding being mines when its force is concentrated over a small working area and fails when the same
    // force is spread over a large one, because the fracture gate is pressure, not raw force). All against
    // granite's cited fracture strength, no "miner" branch, no per-race yield table (Principles 8, 9).
    use civsim_sim::material::{ExtractionParams, MaterialField};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    let cell = Coord3::ground(2, 2);
    let heap = Fixed::from_int(100000);
    // The extract output is the fifth output of the miner layout (move [act,dx,dy] at 0..2, ingest [act]
    // at 3, extract [act] at 4), so the extractor's controller is a single nonzero weight: the bias input
    // driving the extract activation to one, so extract wins its decision over the resting move and ingest.
    let build = |extract_weight: bool, working_area: Fixed| -> Runner {
        let (mut organs, fat) = energy_registry();
        let muscle = organs.organs.len() as u16;
        organs.organs.push(OrganKindDef {
            id: muscle,
            name: "muscle".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(MUSCLE_STRENGTH, Fixed::ONE)]),
        });
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_miner(),
            LocomotionParams::dev_default(),
            0,
            0x4174,
        );
        assert_eq!(
            emb.layout().n_out(),
            5,
            "miner layout: move(3) + ingest(1) + extract(1)"
        );
        let n_in = emb.layout().n_in();
        let controller = if extract_weight {
            let mut w = vec![Fixed::ZERO; emb.layout().weight_count()];
            w[4 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> extract activation
            Controller::from_weights(n_in, emb.layout().n_out(), 0, w)
        } else {
            Controller::zeros(emb.layout())
        };
        emb.add(
            resting_walker(
                1,
                cell,
                body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
                &reg,
                &organs,
                controller,
            ),
            band(305),
        );
        let mut field = MaterialField::new();
        field.deposit(cell, "granite", heap);
        emb.set_material(field);
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        emb.set_extraction_params(ExtractionParams {
            working_area,
            pressure_max: Fixed::from_int(150_000),
        });
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(10, 10, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
        ));
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let carried =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().walkers()[0].carried.total_volume() };
    let ground =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(cell, "granite") };

    // A tiny working area concentrates the being's force to a pressure far above granite's fracture
    // strength: the deciding being fractures the rock and takes a strength-bounded load, its heap losing
    // exactly that much.
    let small_area = Fixed::from_ratio(1, 1_000_000);
    let mut mining = build(true, small_area);
    mining.step();
    let mined = carried(&mining);
    assert!(
        mined > Fixed::ZERO,
        "a being that decides to extract and can fracture the rock mines it"
    );
    assert_eq!(
        ground(&mining),
        heap - mined,
        "its cell lost exactly what it extracted (conservation)"
    );
    assert!(
        ground(&mining) > Fixed::ZERO,
        "its strength, not the seam, bounded the take: rock remains"
    );

    // The blank founder, expressing zero on the extract channel, never mines though it stands on the same
    // rock, affords the same operation, and could fracture it: no decision, no mining.
    let mut founder = build(false, small_area);
    founder.step();
    assert_eq!(
        carried(&founder),
        Fixed::ZERO,
        "the blank founder does not mine"
    );
    assert_eq!(ground(&founder), heap, "the founder's seam is untouched");

    // The SAME deciding being with the same strength, but its force spread over a large working area, cannot
    // raise its pressure over granite's fracture strength: it decides to extract and gets nothing, because
    // the contest is pressure, not raw force (a fist where a pick would bite). This is the fracture gate.
    let large_area = Fixed::from_int(1000);
    let mut spread = build(true, large_area);
    spread.step();
    assert_eq!(
        carried(&spread),
        Fixed::ZERO,
        "spread too thin to fracture granite, the deciding being mines nothing"
    );
    assert_eq!(
        ground(&spread),
        heap,
        "the rock holds against too low a pressure"
    );
}

#[test]
fn a_wielded_tool_multiplies_the_extraction_affordance_by_its_geometry_and_material() {
    // Material-substrate arc item 4, crafting, THE TOOL MULTIPLIES THE AFFORDANCE: the same being, too
    // weak-handed to fracture granite bare, breaks it when it wields a sharp hard tool, because the tool
    // concentrates its force over a small contact area into a pressure that clears the rock, AND the tool
    // must itself be hard enough (a sharp soft tool blunts and does nothing). This is the payoff loop's
    // hinge, mining harder matter with a made tool, all geometry and material against substance data.
    use civsim_sim::material::{ExtractionParams, MaterialField, WieldedTool};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    // A controlled floor: a granite target (a fracture strength to clear, a density to weigh the load), a
    // hard flint tool material, and a soft chalk tool material. Kept in the test so the tool-material cap is
    // exercised on known values without pinning shipped floor data.
    const FLOOR: &str = r#"
[[axis]]
id = "mat.density"
measures = "bulk density"
unit = "kg/m^3"
dimension = "-3,1,0,0"
scale = "kg/m^3"
tier = 0
range_lo = "0.08"
range_hi = "23000"
real = "test fixture"

[[axis]]
id = "mat.indentation_hardness"
measures = "the contact pressure a surface resists before plastic indentation"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "1"
range_hi = "150000"
real = "test fixture"

[[axis]]
id = "mat.fracture_strength"
measures = "the stress a substance fractures at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[substance]]
id = "granite"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2700" },
  { axis = "mat.fracture_strength", value = "15" },
]

[[substance]]
id = "flint"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.indentation_hardness", value = "1000" },
]

[[substance]]
id = "chalk"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.indentation_hardness", value = "5" },
]
"#;

    let cell = Coord3::ground(2, 2);
    let heap = Fixed::from_int(100000);
    // A tool with a tiny contact area (a sharp point), of the given substance.
    let tool = |substance: &str| WieldedTool {
        contact_area: Fixed::from_ratio(1, 1_000_000),
        substance: substance.to_string(),
    };
    // A large bare working area, so the bare being cannot raise its pressure over granite's fracture
    // strength: only a concentrating tool can.
    let bare_area = Fixed::from_int(1000);

    let build = |wielded: Option<WieldedTool>| -> Runner {
        let (mut organs, fat) = energy_registry();
        let muscle = organs.organs.len() as u16;
        organs.organs.push(OrganKindDef {
            id: muscle,
            name: "muscle".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(MUSCLE_STRENGTH, Fixed::ONE)]),
        });
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_miner(),
            LocomotionParams::dev_default(),
            0,
            0x700,
        );
        let n_in = emb.layout().n_in();
        let mut w = vec![Fixed::ZERO; emb.layout().weight_count()];
        w[4 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> extract activation
        let controller = Controller::from_weights(n_in, emb.layout().n_out(), 0, w);
        let mut walker = resting_walker(
            1,
            cell,
            body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
            &reg,
            &organs,
            controller,
        );
        walker.wielded = wielded;
        emb.add(walker, band(305));
        let mut field = MaterialField::new();
        field.deposit(cell, "granite", heap);
        emb.set_material(field);
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
        );
        emb.set_extraction_params(ExtractionParams {
            working_area: bare_area,
            pressure_max: Fixed::from_int(150_000),
        });
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(10, 10, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
        ));
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let carried =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().walkers()[0].carried.total_volume() };

    // Bare-handed, the deciding being spreads its force too thin to fracture granite: it mines nothing.
    let mut bare = build(None);
    bare.step();
    assert_eq!(
        carried(&bare),
        Fixed::ZERO,
        "bare-handed the being cannot fracture the granite"
    );

    // Wielding a sharp HARD flint tool, the same being concentrates its force into a pressure that clears
    // the granite and mines it: the tool multiplied the affordance.
    let mut with_flint = build(Some(tool("flint")));
    with_flint.step();
    assert!(
        carried(&with_flint) > Fixed::ZERO,
        "a sharp hard tool lets the same being mine the rock it could not touch bare-handed"
    );

    // Wielding an equally sharp but SOFT chalk tool, the being mines nothing: the tool blunts at its own low
    // hardness before it reaches the granite's fracture strength, so the tool's material matters, not only
    // its edge.
    let mut with_chalk = build(Some(tool("chalk")));
    with_chalk.step();
    assert_eq!(
        carried(&with_chalk),
        Fixed::ZERO,
        "a sharp but soft tool blunts and mines nothing: the tool material is part of the contest"
    );
}

#[test]
fn a_being_geophages_a_needed_mineral_and_outlives_one_that_does_not() {
    // Material-substrate arc item 4, INGEST-FOR-COMPOSITION, the mining payoff and emergence-closer: a
    // mineral in the ground is worth something because a being whose reserve needs it eats it and survives,
    // where one that does not starves of the mineral. Two beings share one embodiment, one body, one
    // draining mineral reserve, and one identical halite (rock salt) seam each; they differ in ONE gene, the
    // controller weight feeding the geophage output. The one whose geophage weight is lifted eats the salt
    // it stands on each tick and lives; the blank founder, expressing zero on that channel, never eats
    // though it stands on the same salt, drains its reserve, and dies. So a mineral's fitness value, the
    // reason to seek it and (later) mine it, is an evolved phenotype resolved by physiology against
    // substance data, no per-race diet table (Principles 8, 9).
    use civsim_sim::homeostasis::HomeostaticAxisDef;
    use civsim_sim::material::MaterialField;

    // A registry whose one draining reserve is backed by the halite substance (rock salt): a mineral need.
    // The required temperature axis does not drain. The reserve drains each tick and, refilled or not,
    // decides survival.
    let reg = HomeostaticRegistry {
        axes: vec![
            HomeostaticAxisDef {
                id: ENERGY,
                name: "mineral".to_string(),
                backing_component: Some("halite".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::from_ratio(1, 50),
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
            },
        ],
    };
    // A mineral-storing tissue backs the halite reserve, so the being has a mineral reserve to drain and
    // refill (an axis with no tissue backing it would have zero capacity, the starve-at-birth cull).
    let mut organs = BodyPlanRegistry::dev_default();
    let mineral_store = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: mineral_store,
        name: "mineral-store".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[("halite", Fixed::ONE)]),
    });
    let store_body = || body((3, 4), vec![organ(mineral_store, (1, 1))]);
    let seam = Fixed::from_int(100000);
    let eater_cell = Coord3::ground(2, 2);
    let founder_cell = Coord3::ground(5, 5);

    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_geophage(),
        LocomotionParams::dev_default(),
        0,
        0x6E0,
    );
    // The geophage output is the fifth output (move [act,dx,dy] 0..2, ingest [act] 3, geophage [act] 4).
    assert_eq!(
        emb.layout().n_out(),
        5,
        "geophage layout: move(3) + ingest(1) + geophage(1)"
    );
    let n_in = emb.layout().n_in();
    let mut w = vec![Fixed::ZERO; emb.layout().weight_count()];
    w[4 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> geophage activation
    let eater_ctrl = Controller::from_weights(n_in, emb.layout().n_out(), 0, w);
    let blank = Controller::zeros(emb.layout());

    emb.add(
        resting_walker(1, eater_cell, store_body(), &reg, &organs, eater_ctrl),
        band(305),
    );
    emb.add(
        resting_walker(2, founder_cell, store_body(), &reg, &organs, blank),
        band(305),
    );
    let mut field = MaterialField::new();
    field.deposit(eater_cell, "halite", seam);
    field.deposit(founder_cell, "halite", seam);
    emb.set_material(field);
    // No material registry and no anatomy physiology are needed: geophagy reads the being's own edibility
    // physiology (halite assimilation, from the mineral-backed axis) and the cell's volume, not the floor.

    let mut runner =
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb);
    for _ in 0..200 {
        runner.step();
    }

    let alive = |r: &Runner, id: u64| -> bool {
        r.embodiment()
            .unwrap()
            .walkers()
            .iter()
            .find(|w| w.id == StableId(id))
            .unwrap()
            .alive
    };
    let ground = |r: &Runner, coord: Coord3| -> Fixed {
        r.embodiment().unwrap().material().volume(coord, "halite")
    };

    // The evolved-weight being ate its way through the drain and lives; its seam lost some salt.
    assert!(
        alive(&runner, 1),
        "the being that geophages its needed mineral survives"
    );
    assert!(
        ground(&runner, eater_cell) < seam,
        "the eater drew salt from its seam (the mineral is consumed to satisfy the need)"
    );
    // The blank founder never ate, drained its mineral reserve, and died; its seam is untouched.
    assert!(
        !alive(&runner, 2),
        "the blank founder never geophages and starves of the mineral it stood on"
    );
    assert_eq!(
        ground(&runner, founder_cell),
        seam,
        "the founder's seam is untouched: it never ate"
    );
}

#[test]
fn eating_a_food_that_sickens_harms_the_sensitive_eater_and_spares_the_tolerant_one() {
    // Material-substrate arc item 4, the HARM-HALF of INGEST-FOR-COMPOSITION (the owner's seam): eating is
    // not only benefit. A being that eats a substance carrying a toxin takes the harm against its OWN
    // inherited tolerance, so the same brackish food (a nutrient laced with salt) sickens a salt-sensitive
    // eater and spares a salt-tolerant one, per consumer, no per-substance poison label (Principle 9). The
    // felt harm lands on CONDITION, the same reserve the harm-learning loop reads, so a being can learn "this
    // food sickens me", the symmetric completion of the composition read harm-learning opened.
    use civsim_sim::homeostasis::{HomeostaticAxisDef, CONDITION};
    use civsim_sim::material::MaterialField;
    use civsim_sim::physiology::SALINITY;

    // A registry whose brackish substance both FEEDS a mineral reserve and carries a salinity toxin.
    const FLOOR: &str = r#"
[[axis]]
id = "mat.density"
measures = "bulk density"
unit = "kg/m^3"
dimension = "-3,1,0,0"
scale = "kg/m^3"
tier = 0
range_lo = "0.08"
range_hi = "23000"
real = "test fixture"

[[axis]]
id = "bio.salinity"
measures = "a salinity toxin concentration"
unit = "ratio"
dimension = "dimensionless"
scale = "1"
tier = 0
range_lo = "0"
range_hi = "1000"
real = "test fixture"

[[substance]]
id = "brackish"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1200" },
  { axis = "bio.salinity", value = "2" },
]
"#;

    let cell = Coord3::ground(2, 2);
    // A mineral-storing tissue backs the brackish reserve so the being has room to eat into.
    let mut organs = BodyPlanRegistry::dev_default();
    let store = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: store,
        name: "mineral-store".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[("brackish", Fixed::ONE)]),
    });
    // A registry with a brackish-backed mineral reserve, a CONDITION reserve for the harm to land on, and
    // the required temperature axis.
    let reg = HomeostaticRegistry {
        axes: vec![
            HomeostaticAxisDef {
                id: ENERGY,
                name: "mineral".to_string(),
                backing_component: Some("brackish".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
            },
            HomeostaticAxisDef {
                id: CONDITION,
                name: "condition".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
            },
        ],
    };

    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_geophage(),
        LocomotionParams::dev_default(),
        0,
        0x5A17,
    );
    let blank = Controller::zeros(emb.layout());
    // Two beings differing only in salinity tolerance: sensitive (low) and tolerant (high).
    let mut add_eater = |id: u64, tolerance: Fixed| {
        let mut walker = resting_walker(
            id,
            cell,
            body((1, 1), vec![organ(store, (1, 1))]),
            &reg,
            &organs,
            blank.clone(),
        );
        walker
            .physiology
            .tolerances
            .insert(SALINITY.to_string(), tolerance);
        // Open room in the mineral reserve so the being eats brackish (and takes its toxin).
        walker
            .homeostasis
            .set_level(ENERGY, Fixed::from_ratio(1, 2));
        emb.add(walker, band(305));
    };
    add_eater(1, Fixed::from_ratio(1, 10)); // sensitive: low salt tolerance
    add_eater(2, Fixed::from_int(100)); // tolerant: high salt tolerance

    let mut field = MaterialField::new();
    field.deposit(cell, "brackish", Fixed::from_int(100000));
    emb.set_material(field);
    emb.set_material_registry(
        civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
    );

    let condition = |e: &Embodiment, id: u64| -> Fixed {
        e.walkers()
            .iter()
            .find(|w| w.id == StableId(id))
            .unwrap()
            .homeostasis
            .level(CONDITION)
    };
    let full = condition(&emb, 1);
    assert_eq!(full, Fixed::ONE, "both beings start at full condition");
    assert_eq!(condition(&emb, 2), Fixed::ONE);

    // Both eat the brackish food (gaining the mineral); the toxin harm lands per tolerance.
    assert!(
        emb.geophage(StableId(1)) > Fixed::ZERO,
        "the sensitive being eats the brackish food"
    );
    assert!(
        emb.geophage(StableId(2)) > Fixed::ZERO,
        "the tolerant being eats the same food"
    );

    let sensitive = condition(&emb, 1);
    let tolerant = condition(&emb, 2);
    // The salt-sensitive eater is harmed by the food (its condition fell); the salt-tolerant one is not (or
    // far less), the SAME food read against each being's own inherited tolerance.
    assert!(
        sensitive < full,
        "the food sickens the sensitive eater: its condition fell to {sensitive:?}"
    );
    assert!(
        tolerant > sensitive,
        "the same food spares the tolerant eater: {tolerant:?} > {sensitive:?}"
    );
}

#[test]
fn a_being_crafts_a_tool_from_its_carried_stone_only_through_an_evolved_weight() {
    // Material-substrate arc item 4, crafting, THE KNAPPING: a being shapes its carried stone into a wielded
    // tool only because its evolved controller decided to, never because the engine scripts toolmaking. A
    // being carrying stone with a lifted craft weight wields a tool of that stone (its carried stock spent
    // by the tool volume); a blank founder carrying the same stone never crafts. Founder-zero, evolved
    // decision, the tool made of the stone worked (Principles 8, 9).
    use civsim_sim::material::{CraftParams, SubstanceMix};

    let cell = Coord3::ground(2, 2);
    let (organs, fat) = energy_registry();
    let reg = energy_thermal_registry();
    let stock = Fixed::from_int(5);

    let build = |craft_weight: bool| -> Runner {
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_toolmaker(),
            LocomotionParams::dev_default(),
            0,
            0xC0A1,
        );
        // The craft output is the sixth output (move [act,dx,dy] 0..2, ingest [act] 3, extract [act] 4,
        // craft [act] 5).
        assert_eq!(
            emb.layout().n_out(),
            6,
            "toolmaker layout: move(3)+ingest(1)+extract(1)+craft(1)"
        );
        let n_in = emb.layout().n_in();
        let controller = if craft_weight {
            let mut w = vec![Fixed::ZERO; emb.layout().weight_count()];
            w[5 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> craft activation
            Controller::from_weights(n_in, emb.layout().n_out(), 0, w)
        } else {
            Controller::zeros(emb.layout())
        };
        let mut walker = resting_walker(
            1,
            cell,
            body((1, 1), vec![organ(fat, (1, 1))]),
            &reg,
            &organs,
            controller,
        );
        let mut carried = SubstanceMix::new();
        carried.add("granite", stock);
        walker.carried = carried;
        emb.add(walker, band(310));
        emb.set_craft_params(CraftParams::dev_fixture()); // edge 1e-6 m^2, tool volume 1
        Runner::with_embodiment(uniform_field(8, 8, Fixed::from_int(310)), calib(), emb)
    };

    let tool_substance = |r: &Runner| -> Option<String> {
        r.embodiment().unwrap().walkers()[0]
            .wielded
            .as_ref()
            .map(|t| t.substance.clone())
    };
    let carried_granite = |r: &Runner| -> Fixed {
        r.embodiment().unwrap().walkers()[0]
            .carried
            .volume("granite")
    };

    // The evolved-weight being shaped its carried granite into a granite tool, spending the tool volume.
    let mut maker = build(true);
    maker.step();
    assert_eq!(
        tool_substance(&maker).as_deref(),
        Some("granite"),
        "the being crafts a tool of the stone it carried"
    );
    assert_eq!(
        carried_granite(&maker),
        stock - Fixed::ONE,
        "the tool spent the reserved tool volume of its carried stone"
    );

    // The blank founder, expressing zero on the craft channel, never crafts though it carries the same stone.
    let mut founder = build(false);
    founder.step();
    assert_eq!(
        tool_substance(&founder),
        None,
        "the blank founder wields no tool"
    );
    assert_eq!(
        carried_granite(&founder),
        stock,
        "the founder's carried stone is untouched"
    );
}

#[test]
fn a_crafted_tool_closes_the_loop_and_mines_rock_the_bare_body_cannot() {
    // Material-substrate arc item 4, the RECURSIVE TOOL LOOP end to end: a being that carries a hard stone
    // shapes it into a tool and then, wielding it, breaks a rock its bare body could not. Driven at the
    // method level (craft then extract) so the two-step chain is proven directly: knapping makes the tool,
    // the tool multiplies the extraction. This is the payoff that gives mining and carrying a reason: a made
    // point breaks matter a body cannot.
    use civsim_sim::material::{CraftParams, ExtractionParams, MaterialField, SubstanceMix};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    // A floor: a granite target (fracture strength to clear), and a hard flint the being carries and shapes.
    const FLOOR: &str = r#"
[[axis]]
id = "mat.density"
measures = "bulk density"
unit = "kg/m^3"
dimension = "-3,1,0,0"
scale = "kg/m^3"
tier = 0
range_lo = "0.08"
range_hi = "23000"
real = "test fixture"

[[axis]]
id = "mat.indentation_hardness"
measures = "the contact pressure a surface resists before plastic indentation"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "1"
range_hi = "150000"
real = "test fixture"

[[axis]]
id = "mat.fracture_strength"
measures = "the stress a substance fractures at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[substance]]
id = "granite"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2700" },
  { axis = "mat.fracture_strength", value = "15" },
]

[[substance]]
id = "flint"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2600" },
  { axis = "mat.indentation_hardness", value = "1000" },
]
"#;

    let cell = Coord3::ground(2, 2);
    let heap = Fixed::from_int(100000);
    let (mut organs, fat) = energy_registry();
    let muscle = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: muscle,
        name: "muscle".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[(MUSCLE_STRENGTH, Fixed::ONE)]),
    });
    let reg = energy_thermal_registry();

    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_toolmaker(),
        LocomotionParams::dev_default(),
        0,
        0xC10E,
    );
    let blank = Controller::zeros(emb.layout());
    let mut walker = resting_walker(
        1,
        cell,
        body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
        &reg,
        &organs,
        blank,
    );
    // The being carries exactly the tool volume of flint to shape, so after crafting it carries no residual
    // load and its full strength is free to bear the granite it then mines.
    let mut carried = SubstanceMix::new();
    carried.add("flint", Fixed::ONE);
    walker.carried = carried;
    emb.add(walker, band(310));
    let mut field = MaterialField::new();
    field.deposit(cell, "granite", heap);
    emb.set_material(field);
    emb.set_material_registry(
        civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
    );
    // A large bare working area, so bare-handed the being cannot fracture the granite.
    emb.set_extraction_params(ExtractionParams {
        working_area: Fixed::from_int(1000),
        pressure_max: Fixed::from_int(150_000),
    });
    emb.set_craft_params(CraftParams::dev_fixture()); // small edge, tool volume 1
    emb.set_physiology(EmbodiedPhysiology::dev_fixture(
        organs,
        MediumField::uniform(8, 8, Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
    ));
    let mut runner =
        Runner::with_embodiment(uniform_field(8, 8, Fixed::from_int(310)), calib(), emb);

    // Bare-handed (its large working area), the being cannot fracture the granite.
    assert_eq!(
        runner
            .embodiment_mut()
            .unwrap()
            .extract_underfoot(StableId(1)),
        Fixed::ZERO,
        "bare-handed the being cannot break the granite"
    );
    // It shapes its carried flint into a tool.
    assert!(
        runner
            .embodiment_mut()
            .unwrap()
            .craft_from_carried(StableId(1)),
        "the being crafts a flint tool from its carried stone"
    );
    // Now wielding the flint point, the same being breaks the granite it could not touch bare-handed.
    let mined = runner
        .embodiment_mut()
        .unwrap()
        .extract_underfoot(StableId(1));
    assert!(
        mined > Fixed::ZERO,
        "wielding its made tool, the being mines the rock its bare body could not: {mined:?}"
    );
}

#[test]
fn medium_respiration_lives_in_a_rich_medium_and_suffocates_in_a_poor_one() {
    // The respiration sub-phase, through the runner: a body with a respiratory surface breathes its
    // ambient medium each tick. In a rich medium it replenishes what metabolism spends and survives; the
    // identical body in a poor medium off-gasses and suffocates. The outcome is the medium's content, not
    // a label on the being (Principle 9). Only the medium content differs between the two runs.
    let setpoint = 300;
    // A registry carrying the respiration axis (an oxygen buffer that drains and fails through a hypoxia
    // floor) plus the required non-draining temperature axis.
    let reg = {
        let mut r = civsim_sim::medium::dev_respiration();
        r.axes.push(HomeostaticAxisDef {
            id: TEMPERATURE,
            name: "temperature".to_string(),
            backing_component: None,
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::ZERO,
            exertion_drain: Fixed::ZERO,
            death_floor: Fixed::ZERO,
        });
        r
    };
    // A body with a full gill (respiratory surface).
    let mut organs = BodyPlanRegistry::dev_default();
    let gill = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: gill,
        name: "gill".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[(RESPIRATORY_SURFACE, Fixed::ONE)]),
    });

    let survives = |respirable: Fixed| -> bool {
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x611,
        );
        let blank = Controller::zeros(emb.layout());
        emb.add(
            resting_walker(
                1,
                Coord3::ground(2, 2),
                body((1, 2), vec![organ(gill, (1, 1))]),
                &reg,
                &organs,
                blank,
            ),
            band(setpoint),
        );
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs.clone(),
            MediumField::uniform(6, 6, respirable, Fixed::ZERO, Fixed::ZERO),
        ));
        let mut runner =
            Runner::with_embodiment(uniform_field(6, 6, Fixed::from_int(setpoint)), calib(), emb);
        for _ in 0..300 {
            runner.step();
        }
        runner.embodiment().unwrap().walkers()[0].alive
    };

    assert!(
        survives(Fixed::ONE),
        "a full respiratory surface in a rich medium keeps breathing and survives"
    );
    assert!(
        !survives(Fixed::from_ratio(1, 5)),
        "the same body in a medium too poor in the respirable species suffocates"
    );
}

#[test]
fn beings_respire_the_medium_of_their_own_cell_through_the_runner() {
    // The per-cell medium field through the full runner tick (real-world unification step 4): two identical
    // gilled bodies stand in the same MediumField, one in a rich-respirable cell and one in a poor cell, and
    // diverge in survival from the cell alone. The blank resting controller keeps each body at its spawn
    // cell, so the outcome is the medium of the cell it stands in, not a label (Principle 9). This is the
    // wiring the earlier uniform-field respiration test proves the regression of.
    let setpoint = 300;
    let reg = {
        let mut r = civsim_sim::medium::dev_respiration();
        r.axes.push(HomeostaticAxisDef {
            id: TEMPERATURE,
            name: "temperature".to_string(),
            backing_component: None,
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::ZERO,
            exertion_drain: Fixed::ZERO,
            death_floor: Fixed::ZERO,
        });
        r
    };
    let mut organs = BodyPlanRegistry::dev_default();
    let gill = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: gill,
        name: "gill".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[(RESPIRATORY_SURFACE, Fixed::ONE)]),
    });

    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        0,
        0x0CEA,
    );
    let blank = Controller::zeros(emb.layout());
    // Being 1 stands in the rich left half (x < 3), being 2 in the poor right half (x >= 3).
    emb.add(
        resting_walker(
            1,
            Coord3::ground(1, 1),
            body((1, 2), vec![organ(gill, (1, 1))]),
            &reg,
            &organs,
            blank.clone(),
        ),
        band(setpoint),
    );
    emb.add(
        resting_walker(
            2,
            Coord3::ground(4, 1),
            body((1, 2), vec![organ(gill, (1, 1))]),
            &reg,
            &organs,
            blank,
        ),
        band(setpoint),
    );
    // A per-cell medium: the left half (x < 3) rich in the respirable species, the right half poor. Density
    // and temperature are unread by respiration this increment; only the respirable content differs.
    let (w, h) = (6i32, 6i32);
    let respirable: Vec<Fixed> = (0..w * h)
        .map(|i| {
            if i % w < 3 {
                Fixed::ONE
            } else {
                Fixed::from_ratio(1, 5)
            }
        })
        .collect();
    let medium = MediumField::new(
        w,
        h,
        respirable,
        vec![Fixed::ZERO; (w * h) as usize],
        vec![Fixed::from_int(setpoint); (w * h) as usize],
    );
    emb.set_physiology(EmbodiedPhysiology::dev_fixture(organs, medium));
    let mut runner =
        Runner::with_embodiment(uniform_field(w, h, Fixed::from_int(setpoint)), calib(), emb);
    for _ in 0..300 {
        runner.step();
    }
    let alive = |id: u64| {
        runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .find(|w| w.id == StableId(id))
            .unwrap()
            .alive
    };
    assert!(alive(1), "the body in the rich cell breathes and survives");
    assert!(
        !alive(2),
        "the identical body in the poor cell suffocates: the divergence is the cell's medium"
    );
}

#[test]
fn embodied_physiology_reads_a_set_manifest_and_fails_loud_when_reserved() {
    // The canonical sourcing: EmbodiedPhysiology::from_manifest reads the metabolic anchors, the per-cell
    // medium field (the submersion elevation and the submerged and emergent medium profiles), the reserved
    // transfer coefficient, and the base tick from a set manifest, and a reserved input refuses to
    // fabricate a number (Principle 11).
    use civsim_sim::calibration::{CalibrationError, CalibrationManifest};
    use civsim_world::{BiomeSet, FlatBounded, TileMap, WorldgenParams};
    let map = TileMap::generate(
        7,
        FlatBounded::new(8, 6, 1),
        &BiomeSet::dev_default(),
        &WorldgenParams::dev_default(),
    );
    let set = r#"
[[reserved]]
id = "metabolism.kleiber_coefficient"
basis = "fixture"
status = "set"
value = "3.4"
unit = "w"
source = "test"
[[reserved]]
id = "metabolism.body_mass_kg_scale"
basis = "fixture"
status = "set"
value = "100"
unit = "kg"
source = "test"
[[reserved]]
id = "metabolism.medium_convective_coefficient"
basis = "fixture"
status = "set"
value = "10"
unit = "h"
source = "test"
[[reserved]]
id = "metabolism.surface_emissivity"
basis = "fixture"
status = "set"
value = "0.95"
unit = "e"
source = "test"
[[reserved]]
id = "metabolism.stefan_boltzmann"
basis = "fixture"
status = "set"
value = "0.0000000567"
unit = "sigma"
source = "test"
[[reserved]]
id = "metabolism.respiration_transfer_coefficient"
basis = "fixture"
status = "set"
value = "0.5"
unit = "k"
source = "test"
[[reserved]]
id = "time.base_tick_seconds"
basis = "fixture"
status = "set"
value = "1"
unit = "s"
source = "test"
[[reserved]]
id = "medium.submersion_elevation"
basis = "fixture"
status = "set"
value = "0.40"
unit = "normalised_elevation"
source = "test"
[[reserved]]
id = "medium.water"
basis = "fixture"
status = "set"
value = "density=1000,respirable_content=0.3,conductivity=0.606,specific_heat=4186"
unit = "medium_profile"
source = "test"
[[reserved]]
id = "medium.air"
basis = "fixture"
status = "set"
value = "density=1.2,respirable_content=9,conductivity=0.0262,specific_heat=1005"
unit = "medium_profile"
source = "test"
"#;
    let m = CalibrationManifest::from_toml_str(set).unwrap();
    let organs = BodyPlanRegistry::dev_default();
    // The set manifest threads the anchors, the per-cell medium field (water below the submersion
    // elevation, air above), the transfer coefficient, and the base tick into a usable physiology.
    let _phys =
        EmbodiedPhysiology::from_manifest(&m, organs.clone(), &map, "medium.water", "medium.air")
            .unwrap();

    // A reserved transfer coefficient refuses to build.
    let reserved = set.replace(
        "id = \"metabolism.respiration_transfer_coefficient\"\nbasis = \"fixture\"\nstatus = \"set\"\nvalue = \"0.5\"",
        "id = \"metabolism.respiration_transfer_coefficient\"\nbasis = \"fixture\"\nstatus = \"reserved\"\nvalue = \"\"",
    );
    let mr = CalibrationManifest::from_toml_str(&reserved).unwrap();
    assert_eq!(
        EmbodiedPhysiology::from_manifest(&mr, organs, &map, "medium.water", "medium.air")
            .unwrap_err(),
        CalibrationError::Reserved("metabolism.respiration_transfer_coefficient".to_string()),
    );
}
