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
    WATER,
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
                draw_set: Vec::new(),
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
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
        MediumField::uniform(
            10,
            10,
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::from_int(10),
            Fixed::ZERO,
        ),
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
            MediumField::uniform(
                8,
                8,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
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
            MediumField::uniform(
                8,
                8,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
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
            MediumField::uniform(
                8,
                8,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
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
            MediumField::uniform(
                8,
                8,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
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
            MediumField::uniform(
                8,
                8,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
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
        MediumField::uniform(
            10,
            10,
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::from_int(10),
            Fixed::ZERO,
        ),
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
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
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
        volume: Fixed::ONE,
        length: Fixed::ONE,
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
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
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
fn a_cut_frees_the_soft_constituent_of_a_composite_a_bare_extract_cannot_and_authors_nothing() {
    // The made-world arc, tool-use: a CUT gates PER CONSTITUENT of the cell's own composite, freeing every
    // substance whose OWN fracture strength the edge beats, where EXTRACT gates on the AGGREGATE (the hardest
    // constituent binds the whole cell). So a keen edge frees the soft flesh from a tough rind a bare press
    // cannot break, and WHICH substances are freed is derived from the cell's own composition, never a table.
    use civsim_sim::material::{ExtractionParams, MaterialField, WieldedTool};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    // A composite target: a rind tough in shear (shear strength 5000) binding a soft flesh (shear strength 1),
    // and a flint edge (shear strength 500, so its deliverable shear parts the flesh but not the rind). The
    // extraction contest still reads fracture strength (kept on rind/flesh), so a bare extract gates on the
    // aggregate rind. Kept in the test.
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

[[axis]]
id = "mat.shear_strength"
measures = "the shear stress a substance parts at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[substance]]
id = "rind"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1200" },
  { axis = "mat.fracture_strength", value = "5000" },
  { axis = "mat.shear_strength", value = "5000" },
]

[[substance]]
id = "flesh"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1050" },
  { axis = "mat.fracture_strength", value = "1" },
  { axis = "mat.shear_strength", value = "1" },
]

[[substance]]
id = "fibre"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1100" },
  { axis = "mat.fracture_strength", value = "1" },
  { axis = "mat.shear_strength", value = "5000" },
]

[[substance]]
id = "flint"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.indentation_hardness", value = "1000" },
  { axis = "mat.shear_strength", value = "500" },
]
"#;

    let cell = Coord3::ground(2, 2);
    let ration = Fixed::from_int(1000);
    let tool = || WieldedTool {
        contact_area: Fixed::from_ratio(1, 1_000_000),
        volume: Fixed::ONE,
        length: Fixed::ONE,
        substance: "flint".to_string(),
    };
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
            0x0C07,
        );
        let controller = Controller::zeros(emb.layout());
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
        field.deposit(cell, "rind", ration);
        field.deposit(cell, "flesh", ration);
        field.deposit(cell, "fibre", ration);
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
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let carried_of =
        |r: &Runner, s: &str| -> Fixed { r.embodiment().unwrap().walkers()[0].carried.volume(s) };

    // A sharp flint edge CUTS: it frees the soft flesh (its own shear strength is beaten by the edge's
    // deliverable shear) into the carried load, and frees NO rind (the rind's 5000 shear strength exceeds the
    // edge's deliverable shear, self-limited at the tool's own 500). Both outcomes derived.
    let mut cutter = build(Some(tool()));
    cutter.embodiment_mut().unwrap().cut_underfoot(StableId(1));
    assert!(
        carried_of(&cutter, "flesh") > Fixed::ZERO,
        "a keen edge frees the soft flesh by beating its own shear strength"
    );
    assert_eq!(
        carried_of(&cutter, "rind"),
        Fixed::ZERO,
        "the edge's deliverable shear is below the rind's shear strength, so it frees no rind: selective by physics"
    );
    // The sever gate reads SHEAR, not normal fracture: the fibre has the SAME low fracture strength as the
    // flesh (1) but a high SHEAR strength (5000), so the OLD normal-stress gate would have freed it and the
    // shear gate does NOT. It stays in the cell, proving R-CUT-SHEAR: the cut parts by shear.
    assert_eq!(
        carried_of(&cutter, "fibre"),
        Fixed::ZERO,
        "the fibre is weak in fracture but tough in shear, so the shear-parting cut leaves it: the gate is shear"
    );

    // The SAME being with the SAME tool EXTRACTS nothing: extraction gates on the aggregate (the hardest
    // constituent, the rind at 5000), which the edge cannot beat, so the whole cell holds. The cut's distinct
    // power is reaching the soft part the aggregate press cannot.
    let mut extractor = build(Some(tool()));
    extractor
        .embodiment_mut()
        .unwrap()
        .extract_underfoot(StableId(1));
    assert_eq!(
        carried_of(&extractor, "flesh") + carried_of(&extractor, "rind"),
        Fixed::ZERO,
        "extraction gates on the aggregate rind and frees nothing: only the per-constituent cut reaches the flesh"
    );

    // A BARE being cuts nothing: the cut needs an edge, so an opted-out being is inert (byte-neutral gate).
    let mut bare = build(None);
    bare.embodiment_mut().unwrap().cut_underfoot(StableId(1));
    assert_eq!(
        carried_of(&bare, "flesh") + carried_of(&bare, "rind"),
        Fixed::ZERO,
        "a bare being has no edge and cuts nothing"
    );
}

#[test]
fn a_crush_fails_compression_where_a_cut_parts_shear_the_same_tool_diverging_by_the_targets_axis() {
    // The made-world arc, tool-use, Section G: CRUSH fails matter in COMPRESSION where CUT parts it in SHEAR,
    // so the SAME tool with the SAME force frees opposite constituents depending on which resistance axis each
    // target is weak in. A chalk (weak in compression, tough in shear) is crushed but not cut; a fibre (tough
    // in compression, weak in shear) is cut but not crushed. The divergence is the target's own material axes,
    // by physics, never a per-action table or an `IsChalk`/`IsFibre` tag.
    use civsim_sim::material::{MaterialField, WieldedTool};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

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
id = "mat.shear_strength"
measures = "the shear stress a substance parts at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[axis]]
id = "mat.compressive_strength"
measures = "the compressive stress a substance fails at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[substance]]
id = "chalk"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1500" },
  { axis = "mat.compressive_strength", value = "2" },
  { axis = "mat.shear_strength", value = "5000" },
]

[[substance]]
id = "fibre"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1100" },
  { axis = "mat.compressive_strength", value = "5000" },
  { axis = "mat.shear_strength", value = "1" },
]

[[substance]]
id = "tool"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2500" },
  { axis = "mat.compressive_strength", value = "500" },
  { axis = "mat.shear_strength", value = "500" },
]
"#;

    let cell = Coord3::ground(2, 2);
    let ration = Fixed::from_int(1000);
    let tool = || WieldedTool {
        contact_area: Fixed::from_ratio(1, 1_000_000),
        volume: Fixed::ONE,
        length: Fixed::ONE,
        substance: "tool".to_string(),
    };

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
            AffordanceRegistry::dev_crusher(),
            LocomotionParams::dev_default(),
            0,
            0x0C0A,
        );
        let controller = Controller::zeros(emb.layout());
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
        field.deposit(cell, "chalk", ration);
        field.deposit(cell, "fibre", ration);
        emb.set_material(field);
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
        );
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let carried_of =
        |r: &Runner, s: &str| -> Fixed { r.embodiment().unwrap().walkers()[0].carried.volume(s) };

    // The SAME tool CRUSHES the chalk (weak in compression, effective stress beats its 2 MPa) and leaves the
    // fibre (tough in compression, 5000 MPa), by the target's compressive strength alone.
    let mut crusher = build(Some(tool()));
    crusher
        .embodiment_mut()
        .unwrap()
        .crush_underfoot(StableId(1));
    assert!(
        carried_of(&crusher, "chalk") > Fixed::ZERO,
        "the face crushes the compression-weak chalk"
    );
    assert_eq!(
        carried_of(&crusher, "fibre"),
        Fixed::ZERO,
        "the face leaves the compression-tough fibre: the crush gate is compression"
    );

    // The SAME tool CUTS the fibre (weak in shear) and leaves the chalk (tough in shear), the exact opposite
    // outcome, from the same geometry and force: the ONLY difference is which resistance axis the target is
    // weak in, so the two actions are distinct by physics, not a per-action table.
    let mut cutter = build(Some(tool()));
    cutter.embodiment_mut().unwrap().cut_underfoot(StableId(1));
    assert!(
        carried_of(&cutter, "fibre") > Fixed::ZERO,
        "the edge parts the shear-weak fibre"
    );
    assert_eq!(
        carried_of(&cutter, "chalk"),
        Fixed::ZERO,
        "the edge leaves the shear-tough chalk: the cut gate is shear"
    );

    // A bare being crushes nothing (a crush needs a wielded face), the byte-neutral gate.
    let mut bare = build(None);
    bare.embodiment_mut().unwrap().crush_underfoot(StableId(1));
    assert_eq!(
        carried_of(&bare, "chalk") + carried_of(&bare, "fibre"),
        Fixed::ZERO,
        "a bare being has no face and crushes nothing"
    );
}

#[test]
fn a_miners_actuator_work_fractures_rock_and_a_stronger_actuator_shatters_where_a_weaker_one_cannot(
) {
    // The made-world arc, tool-use, Section G, the STROKE-RATE substrate: a percussion STRIKE delivers the
    // MINER's own ACTUATOR WORK (its strength stress over its cross-section, over its grown stroke, F d), and
    // that energy fractures matter whose Griffith energy the blow exceeds. The tool CONCENTRATES the blow (its
    // contact area) but no longer carries the energy: a STRONG-actuator miner shatters a brittle rock a WEAK one
    // cannot, the payoff derived from the miner's own body, and two tools of DIFFERENT mass but the same shape
    // shatter identically, the free tool-mass term dropped (the gate-ruled feel-change-for-correctness; the
    // founded tool-geometry coupling is the arc's flagged follow-on (b)).
    use civsim_sim::contact_transfer::ContactTransferRegistry;
    use civsim_sim::material::{MaterialField, StrikeParams, WieldedTool};
    use civsim_sim::morphogen::{Segment, Structure};
    use civsim_sim::physiology::MUSCLE_STRENGTH;
    use std::collections::BTreeMap;

    // A brittle rock: a Griffith fracture energy (1e6 J/m^2) that the heavy blow's delivered energy (400 J over
    // the 1e-4 struck face, so 100 J of resistance) beats but the light blow's (40 J) does not. Two tool
    // substances of identical everything but density: iron (dense) and pumice (light).
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
id = "mat.fracture_strength"
measures = "the stress a substance fractures at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[axis]]
id = "mat.fracture_energy"
measures = "the critical strain-energy release rate"
unit = "J/m^2"
dimension = "0,1,-2,0"
scale = "J/m^2"
tier = 0
range_lo = "1"
range_hi = "1000000"
real = "test fixture"

[[substance]]
id = "rock"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2700" },
  { axis = "mat.fracture_strength", value = "50" },
  { axis = "mat.fracture_energy", value = "1000000" },
]

[[substance]]
id = "iron"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "8000" },
]

[[substance]]
id = "pumice"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "800" },
]
"#;

    let cell = Coord3::ground(2, 2);
    // Both tools: identical shape (a 1e-4 struck face), differing ONLY in substance density (iron dense, pumice
    // light). Under the actuator-work law the tool's mass no longer enters the delivered energy; only its
    // contact area concentrates the blow (here identical), so the two tools fracture identically.
    let tool = |substance: &str| WieldedTool {
        contact_area: Fixed::from_ratio(1, 10_000),
        volume: Fixed::from_ratio(1, 1_000),
        length: Fixed::ONE,
        substance: substance.to_string(),
    };

    // The miner's ACTUATOR: a grown Segment whose strength over its cross-section (the force) over its stroke is
    // the delivered energy (`F d`). A strong actuator (200) delivers 200 over the tool's 1e-4 face, beating the
    // rock's 100 Griffith resistance (fracture_energy 1e6 over the 1e-4 face); a weak one (50) delivers 50 and
    // does not. Grown per-body geometry, not a world-global swing speed.
    let actuator = |strength: Fixed| {
        let mut geometry = BTreeMap::new();
        // A 1e-6 m^2 cross-section: a 200 MPa strong actuator is a 200 N force (the stress_force megapascal-to-
        // newton bridge) over the 1 m stroke = 200 J, above the rock's 100 J resistance; a 50 MPa weak one is
        // 50 J, below it.
        geometry.insert(
            "mech.cross_section_area".to_string(),
            Fixed::from_ratio(1, 1_000_000),
        );
        geometry.insert("mech.stroke_length".to_string(), Fixed::ONE);
        let mut material = BTreeMap::new();
        material.insert("mat.fracture_strength".to_string(), strength);
        Structure {
            segments: vec![Segment {
                parent: None,
                depth: 0,
                geometry,
                material,
                damage: Fixed::ZERO,
            }],
        }
    };

    let build = |substance: &str, strength: Fixed, arm_strike: bool| -> Runner {
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
            0x0C0C,
        );
        let controller = Controller::zeros(emb.layout());
        let mut walker = resting_walker(
            1,
            cell,
            body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
            &reg,
            &organs,
            controller,
        )
        .with_structure(actuator(strength));
        walker.wielded = Some(tool(substance));
        emb.add(walker, band(305));
        let mut field = MaterialField::new();
        field.deposit(cell, "rock", Fixed::from_int(1000));
        emb.set_material(field);
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
        );
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        if arm_strike {
            emb.set_strike(StrikeParams::dev_fixture());
            emb.set_contact_transfer(ContactTransferRegistry::dev_terran());
        }
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let carried_rock =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().walkers()[0].carried.volume("rock") };

    // A STRONG-actuator miner shatters the rock: its actuator work (200 over the 1e-4 face) beats the rock's
    // 100 Griffith resistance. The energy is the MINER's own, derived from its grown body.
    let mut strong = build("iron", Fixed::from_int(200), true);
    strong
        .embodiment_mut()
        .unwrap()
        .strike_underfoot(StableId(1));
    assert!(
        carried_rock(&strong) > Fixed::ZERO,
        "a strong actuator's blow shatters the rock and frees it"
    );

    // A WEAK-actuator miner of the SAME tool does NOT: its actuator work (50) falls below the rock's resistance.
    // The difference is the MINER's own strength, derived from its body, never the tool's mass.
    let mut weak = build("iron", Fixed::from_int(50), true);
    weak.embodiment_mut().unwrap().strike_underfoot(StableId(1));
    assert_eq!(
        carried_rock(&weak),
        Fixed::ZERO,
        "a weak actuator cannot shatter the rock: the difference is the miner's strength, not the tool's mass"
    );

    // MASS-INDEPENDENCE: the same strong miner with a LIGHT (pumice) tool of the same shape shatters IDENTICALLY
    // to the heavy (iron) one. The tool's mass no longer enters the delivered energy (the free mass term
    // dropped, the gate-ruled feel-change-for-correctness); only its contact area (here identical) concentrates
    // the blow.
    let mut light_tool = build("pumice", Fixed::from_int(200), true);
    light_tool
        .embodiment_mut()
        .unwrap()
        .strike_underfoot(StableId(1));
    assert!(
        carried_rock(&light_tool) > Fixed::ZERO,
        "a light tool shatters the rock just as the heavy one does: the tool's mass no longer carries the energy"
    );

    // OPT-OUT: the same strong miner on an unarmed world (no strike params, no transfer registry) strikes nothing.
    let mut unarmed = build("iron", Fixed::from_int(200), false);
    unarmed
        .embodiment_mut()
        .unwrap()
        .strike_underfoot(StableId(1));
    assert_eq!(
        carried_rock(&unarmed),
        Fixed::ZERO,
        "an unarmed world never strikes (byte-neutral opt-in)"
    );
}

#[test]
fn a_worked_tool_wears_down_and_spends_out_over_repeated_use_and_an_unarmed_tool_is_immortal() {
    // The made-world arc, tool-use, Section D: a wielded tool that works matter loses volume by the Archard
    // wear law ([`laws::wear`]), its coefficient the tool material's own `mat.wear_coefficient`, so it wears
    // gradually and, once worn below the minimum viable tool volume (the craft threshold), is a spent nub and
    // is unwielded, so it must be remade. The wear is DERIVED (force over stroke distance over the tool's own
    // hardness, no authored durability count) and OPT-IN (a world that never arms the wear params keeps every
    // tool immortal, byte-identical to before). This proves both: an armed tool wears down over several uses
    // and spends out, and the same tool on an unarmed world never wears.
    use civsim_sim::material::{CraftParams, MaterialField, WearParams, WieldedTool};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    // A flesh cell (fracture 1, so a keen edge frees it) and a flint edge that carries a wear coefficient. The
    // coefficient is a large test value so each use saturates the worn volume to the ceiling, making the
    // spend-out countable; a real world sets a real Archard coefficient reserved-with-basis.
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

[[axis]]
id = "mat.wear_coefficient"
measures = "the dimensionless Archard wear coefficient"
unit = "1"
dimension = "0,0,0,0"
scale = "1"
tier = 0
range_lo = "0"
range_hi = "2000000000"
real = "test fixture"

[[axis]]
id = "mat.shear_strength"
measures = "the shear stress a substance parts at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[substance]]
id = "flesh"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1050" },
  { axis = "mat.fracture_strength", value = "1" },
  { axis = "mat.shear_strength", value = "1" },
]

[[substance]]
id = "flint"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2500" },
  { axis = "mat.indentation_hardness", value = "1000" },
  { axis = "mat.wear_coefficient", value = "66666666" },
  { axis = "mat.shear_strength", value = "500" },
]
"#;

    let cell = Coord3::ground(2, 2);
    // A tool with five units of volume: enough that the craft threshold (one unit) is reached only after
    // several worn units, so the spend-out is gradual, not a single saturating use.
    let tool_volume0 = Fixed::from_int(5);
    let tool = || WieldedTool {
        contact_area: Fixed::from_ratio(1, 1_000_000),
        volume: tool_volume0,
        length: Fixed::ONE,
        substance: "flint".to_string(),
    };

    let build = |arm_wear: bool| -> Runner {
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
            0x0C08,
        );
        let controller = Controller::zeros(emb.layout());
        let mut walker = resting_walker(
            1,
            cell,
            body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
            &reg,
            &organs,
            controller,
        );
        walker.wielded = Some(tool());
        emb.add(walker, band(305));
        let mut field = MaterialField::new();
        field.deposit(cell, "flesh", Fixed::from_int(1000));
        emb.set_material(field);
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
        );
        // The craft threshold (the minimum viable tool volume) below which a worn tool is a spent nub.
        emb.set_craft_params(CraftParams::dev_fixture());
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        if arm_wear {
            // The stroke distance is the dev fixture; with the flint coefficient each use abrades roughly half
            // a unit, so the five-unit tool crosses the one-unit craft floor after several uses (a gradual
            // spend-out, not an instant break). The ceiling is a non-binding representability cap.
            emb.set_wear(WearParams {
                stroke_distance: Fixed::from_ratio(1, 10),
                wear_max: Fixed::from_int(1000),
            });
        }
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let tool_volume_of =
        |e: &Embodiment| -> Option<Fixed> { e.walkers()[0].wielded.as_ref().map(|t| t.volume) };

    // ARMED: the tool wears gradually. After the first wear it is still wielded and has LESS volume (not spent
    // in one use), and after repeated wear it spends out (its wielded slot empties). Count the uses to prove
    // it is a gradual accumulation, not an instant break.
    let mut armed = build(true);
    let emb = armed.embodiment_mut().unwrap();
    let start = tool_volume_of(emb);
    assert_eq!(
        start,
        Some(tool_volume0),
        "the tool starts at its full volume"
    );
    emb.wear_tool(StableId(1));
    let after_one = tool_volume_of(emb);
    assert!(
        matches!(after_one, Some(v) if v < tool_volume0 && v > Fixed::ZERO),
        "one use wears the tool DOWN but does not spend it out: gradual, not instant (was {after_one:?})"
    );
    let mut uses = 1u32;
    while tool_volume_of(emb).is_some() {
        emb.wear_tool(StableId(1));
        uses += 1;
        assert!(
            uses < 10_000,
            "the tool must spend out in a bounded number of uses"
        );
    }
    assert!(
        uses > 1,
        "the tool spends out only after SEVERAL uses (it took {uses}), proving gradual wear"
    );

    // OPT-OUT: an unarmed world never wears the tool. The same number of `wear_tool` calls leaves the tool
    // wielded at its full volume: the wear step is a no-op without the params, so every existing scenario is
    // byte-identical.
    let mut unarmed = build(false);
    let emb2 = unarmed.embodiment_mut().unwrap();
    for _ in 0..uses {
        emb2.wear_tool(StableId(1));
    }
    assert_eq!(
        tool_volume_of(emb2),
        Some(tool_volume0),
        "an unarmed world never wears the tool: it stays wielded at full volume (byte-neutral opt-in)"
    );

    // WIRED TO WORK: the dispatch wears only on a positive-work use. A real cut that frees flesh, followed by
    // the wear the dispatch applies, reduces the tool's volume: wear rides on work, it is not a free tax.
    let mut worker = build(true);
    let emb3 = worker.embodiment_mut().unwrap();
    let before = tool_volume_of(emb3);
    if emb3.cut_underfoot(StableId(1)) > Fixed::ZERO {
        emb3.wear_tool(StableId(1));
    }
    let after = tool_volume_of(emb3);
    assert!(
        matches!((before, after), (Some(b), Some(a)) if a < b),
        "a cut that frees matter wears the wielded tool (before {before:?}, after {after:?})"
    );
}

#[test]
fn a_brittle_tool_snaps_under_its_own_working_stress_where_a_tough_one_survives_and_an_unarmed_tool_never_breaks(
) {
    // The made-world arc, tool-use, Section E: a wielded tool carries the reaction stress of its own working
    // force over its edge, and if that stress exceeds the tool material's own fracture strength the tool
    // fractures ([`laws::fracture_onset`]) and is unwielded. So the hardness-versus-fracture-strength material
    // tradeoff bites: two tools of IDENTICAL geometry, differing ONLY in fracture strength, meet the same
    // working stress, and the brittle one snaps where the tough one bears it, by physics not a durability tag.
    // Opt-in: an unarmed world never breaks a tool, byte-identical to before.
    use civsim_sim::material::{MaterialField, WieldedTool};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    // A cuttable flesh (so the tool does real work) and two edges of equal geometry: a brittle one (fracture
    // strength 10, below the ~75 MPa reaction stress its own force imposes over the 1e-6 edge) and a tough one
    // (fracture strength 1000, well above it). Both share the same indentation hardness, so they cut alike;
    // only their fracture strength differs, isolating the failure to the material tradeoff.
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

[[axis]]
id = "mat.shear_strength"
measures = "the shear stress a substance parts at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[substance]]
id = "flesh"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1050" },
  { axis = "mat.fracture_strength", value = "1" },
  { axis = "mat.shear_strength", value = "1" },
]

[[substance]]
id = "brittle_edge"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2500" },
  { axis = "mat.indentation_hardness", value = "1000" },
  { axis = "mat.fracture_strength", value = "10" },
  { axis = "mat.shear_strength", value = "500" },
]

[[substance]]
id = "tough_edge"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2500" },
  { axis = "mat.indentation_hardness", value = "1000" },
  { axis = "mat.fracture_strength", value = "1000" },
  { axis = "mat.shear_strength", value = "500" },
]
"#;

    let cell = Coord3::ground(2, 2);
    let tool = |substance: &str| WieldedTool {
        contact_area: Fixed::from_ratio(1, 1_000_000),
        volume: Fixed::from_int(5),
        length: Fixed::ONE,
        substance: substance.to_string(),
    };

    let build = |substance: &str, arm_breakage: bool| -> Runner {
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
            0x0C09,
        );
        let controller = Controller::zeros(emb.layout());
        let mut walker = resting_walker(
            1,
            cell,
            body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
            &reg,
            &organs,
            controller,
        );
        walker.wielded = Some(tool(substance));
        emb.add(walker, band(305));
        let mut field = MaterialField::new();
        field.deposit(cell, "flesh", Fixed::from_int(1000));
        emb.set_material(field);
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
        );
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        emb.set_breakage(arm_breakage);
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let is_wielded =
        |r: &Runner| -> bool { r.embodiment().unwrap().walkers()[0].wielded.is_some() };

    // ARMED brittle: the reaction stress of its own force exceeds its fracture strength, so it snaps. The
    // break_check reports the fracture and the tool is unwielded.
    let mut brittle = build("brittle_edge", true);
    let broke = brittle.embodiment_mut().unwrap().break_check(StableId(1));
    assert!(
        broke,
        "a brittle edge fractures under the stress its own force imposes"
    );
    assert!(
        !is_wielded(&brittle),
        "the fractured tool is unwielded: it must be remade"
    );

    // ARMED tough: the same geometry and force, but a fracture strength well above the reaction stress, so it
    // bears the load and stays wielded. The ONLY difference from the brittle case is the material.
    let mut tough = build("tough_edge", true);
    let survived = tough.embodiment_mut().unwrap().break_check(StableId(1));
    assert!(
        !survived,
        "a tough edge bears the stress its force imposes and does not fracture"
    );
    assert!(is_wielded(&tough), "the surviving tool stays wielded");

    // OPT-OUT: the SAME brittle edge on an unarmed world never breaks. The break_check is a no-op without the
    // arm, so every existing scenario is byte-identical.
    let mut unarmed = build("brittle_edge", false);
    let broke_unarmed = unarmed.embodiment_mut().unwrap().break_check(StableId(1));
    assert!(
        !broke_unarmed,
        "an unarmed world never breaks a tool (byte-neutral opt-in)"
    );
    assert!(
        is_wielded(&unarmed),
        "the unarmed brittle tool stays wielded"
    );

    // WIRED TO WORK: a real cut frees flesh, then the dispatch's breakage check snaps the brittle edge. The
    // cut still happened (the freed matter is taken) and the tool is spent by fracture, the make-use-break
    // lifecycle in one stroke.
    let mut worker = build("brittle_edge", true);
    let emb = worker.embodiment_mut().unwrap();
    let freed = emb.cut_underfoot(StableId(1));
    let broke_on_use = if freed > Fixed::ZERO {
        emb.break_check(StableId(1))
    } else {
        false
    };
    assert!(
        freed > Fixed::ZERO,
        "the brittle edge cut the flesh before it broke"
    );
    assert!(
        broke_on_use,
        "the reaction stress of the cutting stroke snapped the brittle edge"
    );
    assert!(
        !is_wielded(&worker),
        "after the breaking cut the being holds no tool"
    );
}

#[test]
fn a_slender_tool_buckles_under_its_working_load_where_a_stout_one_of_the_same_stock_bears_it() {
    // The made-world arc, tool-use, the tool-geometry expansion (root R2): a tool now carries a characteristic
    // LENGTH, from which its body CROSS-SECTION derives (`volume / length`). A wielded tool loaded axially by
    // its own working force BUCKLES if that force exceeds its critical Euler load, so a SLENDER tool (a long
    // thin body, a small cross-section) buckles where a STOUT one of the SAME stock (same volume, same
    // material) bears the load. The two differ ONLY in length, so the failure is the geometry tradeoff a
    // tool's material choice trades against, derived from `laws::euler_buckle` over the tool's own geometry
    // and elastic modulus, no per-shape table. Opt-in: an unarmed world never buckles a tool.
    use civsim_sim::material::{MaterialField, WieldedTool};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    // One wood: a modest elastic modulus and a high fracture strength (so the edge STRESS never breaks either
    // tool, isolating the failure to BUCKLING). Both tools are this wood, differing only in length.
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
id = "mat.fracture_strength"
measures = "the stress a substance fractures at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[axis]]
id = "mat.elastic_modulus"
measures = "Young's modulus"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "1500000"
real = "test fixture"

[[substance]]
id = "wood"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "800" },
  { axis = "mat.fracture_strength", value = "100" },
  { axis = "mat.elastic_modulus", value = "1000" },
]
"#;

    let cell = Coord3::ground(2, 2);
    // Both tools share the SAME stock (volume 1e-3) and a modest edge (contact area 1e-4, so the reaction
    // stress ~0.75 MPa is far below the 100 MPa fracture strength and never breaks either by stress). They
    // differ ONLY in length: the stout is short (0.1 m, a fat cross-section), the slender is long (10 m, a
    // thin one), so the slender's critical buckling load falls far below the working force.
    let tool = |length: Fixed| WieldedTool {
        contact_area: Fixed::from_ratio(1, 10_000),
        volume: Fixed::from_ratio(1, 1_000),
        length,
        substance: "wood".to_string(),
    };

    let build = |length: Fixed, arm_breakage: bool| -> Runner {
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
            AffordanceRegistry::dev_cutter(),
            LocomotionParams::dev_default(),
            0,
            0x0C0B,
        );
        let controller = Controller::zeros(emb.layout());
        let mut walker = resting_walker(
            1,
            cell,
            body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
            &reg,
            &organs,
            controller,
        );
        walker.wielded = Some(tool(length));
        emb.add(walker, band(305));
        emb.set_material(MaterialField::new());
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
        );
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        emb.set_breakage(arm_breakage);
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let is_wielded =
        |r: &Runner| -> bool { r.embodiment().unwrap().walkers()[0].wielded.is_some() };

    // The STOUT tool (short, fat cross-section) has a high critical buckling load and bears the working force.
    let mut stout = build(Fixed::from_ratio(1, 10), true);
    let stout_broke = stout.embodiment_mut().unwrap().break_check(StableId(1));
    assert!(
        !stout_broke,
        "a stout tool bears the axial working load without buckling"
    );
    assert!(is_wielded(&stout), "the stout tool stays wielded");

    // The SLENDER tool (long, thin cross-section) of the SAME stock and material buckles under the same force:
    // the ONLY difference is its length, so the failure is the geometry, not the material.
    let mut slender = build(Fixed::from_int(10), true);
    let slender_broke = slender.embodiment_mut().unwrap().break_check(StableId(1));
    assert!(
        slender_broke,
        "a slender tool of the same stock buckles under the same working load"
    );
    assert!(!is_wielded(&slender), "the buckled tool is unwielded");

    // OPT-OUT: the SAME slender tool on an unarmed world never buckles (byte-neutral opt-in).
    let mut unarmed = build(Fixed::from_int(10), false);
    let unarmed_broke = unarmed.embodiment_mut().unwrap().break_check(StableId(1));
    assert!(!unarmed_broke, "an unarmed world never buckles a tool");
    assert!(
        is_wielded(&unarmed),
        "the unarmed slender tool stays wielded"
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
                draw_set: Vec::new(),
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
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
fn a_whole_body_bite_does_not_double_credit_two_reserves_backed_by_the_same_substance() {
    // Chemistry arc, Arc 2 hardening (an audit catch): when TWO homeostatic reserves are backed by the SAME
    // body substance, one whole-body bite must SPLIT that substance's removed mass between them, not credit
    // each reserve the full amount (which would create biomass in the eater). Proof: a body carrying one
    // substance, an eater whose two drained reserves both draw on it; after one bite the eater's TOTAL gain
    // does not exceed the assimilable value of the mass the body actually lost.
    use civsim_sim::homeostasis::HomeostaticAxisDef;
    use civsim_sim::material::TissueField;

    // Two DRAINING reserves (energy and water), BOTH backed by the same substance "flesh", plus the required
    // non-draining TEMPERATURE axis (set each tick from the body core, never self-drains).
    let reg = HomeostaticRegistry {
        axes: vec![
            HomeostaticAxisDef {
                id: ENERGY,
                name: "reserve-a".to_string(),
                backing_component: Some("flesh".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::from_ratio(1, 20),
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            },
            HomeostaticAxisDef {
                id: WATER,
                name: "reserve-b".to_string(),
                backing_component: Some("flesh".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::from_ratio(1, 20),
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            },
        ],
    };
    // Two tissues so both reserves have capacity to refill into.
    let mut organs = BodyPlanRegistry::dev_default();
    let store = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: store,
        name: "flesh-store".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[("flesh", Fixed::ONE)]),
    });
    let cell = Coord3::ground(2, 2);

    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_geophage(),
        LocomotionParams::dev_default(),
        0,
        0x111D,
    );
    let n_in = emb.layout().n_in();
    let mut w = vec![Fixed::ZERO; emb.layout().weight_count()];
    w[4 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> geophage activation
    let ctrl = Controller::from_weights(n_in, emb.layout().n_out(), 0, w);
    emb.add(
        resting_walker(
            1,
            cell,
            body((3, 4), vec![organ(store, (1, 1))]),
            &reg,
            &organs,
            ctrl,
        ),
        band(305),
    );
    let mut runner =
        Runner::with_embodiment(uniform_field(6, 6, Fixed::from_int(305)), calib(), emb);

    // Drain the reserves (no food present), so both have room to refill from a bite.
    for _ in 0..8 {
        runner.step();
    }
    // Deposit a body carrying ONLY "flesh" at the eater's cell.
    let mut tissue = TissueField::new();
    let flesh: std::collections::BTreeMap<String, Fixed> =
        [("flesh".to_string(), Fixed::ONE)].into_iter().collect();
    tissue.deposit(cell, flesh, Fixed::from_int(50));
    runner.embodiment_mut().unwrap().set_tissue(tissue);

    let reserves = |r: &Runner| -> (Fixed, Fixed) {
        let w = r
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .find(|w| w.id == StableId(1))
            .unwrap();
        (w.homeostasis.amount(ENERGY), w.homeostasis.amount(WATER))
    };
    let body_vol = |r: &Runner| -> Fixed { r.embodiment().unwrap().tissue().volume_at(cell) };
    let (a0, b0) = reserves(&runner);
    let vol0 = body_vol(&runner);

    // One tick: the geophage fires exactly one whole-body bite.
    runner.step();

    let (a1, b1) = reserves(&runner);
    let vol1 = body_vol(&runner);
    // The eater's TOTAL gain (both reserves), adding back the same-tick drain so we measure intake alone.
    let drain = Fixed::from_ratio(1, 20);
    let gain_a = (a1 - a0).saturating_add(drain);
    let gain_b = (b1 - b0).saturating_add(drain);
    let total_gain = gain_a.saturating_add(gain_b);
    // The body's lost mass this tick (flesh density is one, so lost mass == lost volume), times the ingest
    // efficiency: the assimilable value of what the body actually gave up.
    let removed = vol0 - vol1;
    let eta = LocomotionParams::dev_default().ingest_efficiency;
    let assimilable = removed.checked_mul(eta).unwrap_or(removed);
    assert!(removed > Fixed::ZERO, "the eater bit the body");
    assert!(
        total_gain <= assimilable.saturating_add(Fixed::from_ratio(1, 1000)),
        "the two reserves SHARED the bite's mass (total gain {total_gain:?} <= one bite's assimilable value \
         {assimilable:?}), not double-credited"
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
                draw_set: Vec::new(),
            },
            HomeostaticAxisDef {
                id: CONDITION,
                name: "condition".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
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
fn geophage_eats_the_carried_oilseed_and_feeds_the_reserve() {
    // Ideation viability arc, slice B: the join that closes the discovery loop. A being that EXTRACTED a
    // bonded food into its inventory can EAT it: geophage now draws its bite from what the being CARRIES,
    // not only the cell underfoot, so extract-then-eat feeds a reserve rise (the felt reward the appetitive
    // learner credits). Here the cell holds NO oilseed; the being carries it, and eating the carried oilseed
    // lifts its energy reserve. Nothing authors the payoff: the energy is the seed's own physics, read
    // through the same runtime edibility laws the cell path uses.
    use civsim_sim::material::{MaterialField, SubstanceMix};

    let cell = Coord3::ground(1, 1);
    // A seed-storing tissue backs the reserve so the being has room to eat into (the same shape the mineral
    // geophage fixture uses: the reserve's capacity is sized from a tissue carrying the backing axis).
    let mut organs = BodyPlanRegistry::dev_default();
    let store = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: store,
        name: "seed-store".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[("oilseed", Fixed::ONE)]),
    });
    // A homeostatic registry whose ENERGY reserve is BACKED BY the oilseed substance, so eating oilseed
    // refills it; dev_for_registry then requires and assimilates oilseed by that backing component.
    let reg = HomeostaticRegistry {
        axes: vec![
            HomeostaticAxisDef {
                id: ENERGY,
                name: "energy".to_string(),
                backing_component: Some("oilseed".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            },
        ],
    };
    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        0,
        0x0115EED,
    );
    let blank = Controller::zeros(emb.layout());
    let mut walker = resting_walker(
        1,
        cell,
        body((1, 1), vec![organ(store, (1, 1))]),
        &reg,
        &organs,
        blank,
    );
    // Open room in the energy reserve, and carry the oilseed the being extracted (the cell holds none).
    walker
        .homeostasis
        .set_level(ENERGY, Fixed::from_ratio(1, 2));
    let mut carried = SubstanceMix::new();
    carried.add("oilseed", Fixed::from_int(1000));
    walker.carried = carried;
    emb.add(walker, band(1));

    // The material field holds NO oilseed (the being already extracted it into its inventory), and the real
    // ground floor so the oilseed substance is known.
    emb.set_material(MaterialField::new());
    emb.set_material_registry(
        civsim_physics::PhysicsRegistry::ground().expect("the embedded ground floor loads"),
    );

    let energy = |e: &Embodiment| -> Fixed { e.walkers()[0].homeostasis.level(ENERGY) };
    let carried_oilseed = |e: &Embodiment| -> Fixed { e.walkers()[0].carried.volume("oilseed") };
    let before = energy(&emb);
    let held = carried_oilseed(&emb);
    assert!(
        held > Fixed::ZERO,
        "the being carries the extracted oilseed"
    );

    let gained = emb.geophage(StableId(1));
    assert!(
        gained > Fixed::ZERO,
        "the being eats the oilseed it carries, though the cell holds none: the extract-then-eat join"
    );
    assert!(
        energy(&emb) > before,
        "eating the carried oilseed lifts the energy reserve, the felt reward the learner credits"
    );
    assert!(
        carried_oilseed(&emb) < held,
        "the eaten oilseed leaves the being's inventory (conservation-honest)"
    );
}

#[test]
fn the_geophage_enact_deposits_a_spent_hull_trace_and_is_inert_unarmed() {
    // The physical-trace cultural-persistence substrate (the lifetime/demography keystone, pillar 2, trace
    // slice B, the WIRE slice): an enacted extract-and-eat bite leaves a durable located residue of what it
    // ate. Armed with a byproduct map (oilseed -> spent_hull), the geophage deposits a fraction of the eaten
    // oilseed volume as spent_hull into the cell underfoot: the world's own physical record that the technique
    // happened here, the mark a later being re-earns a belief from (trace slice C), never a handed conclusion.
    // It is opt-in: with NO byproduct map the identical bite deposits nothing, so the material field stays empty
    // and the run is byte-identical. The deposit reads only the eaten substance id and the data map, never a
    // belief, race, or kind (Principle 9): the mark is a physical fact whether or not the eater understands it.
    use civsim_sim::material::{MaterialField, SubstanceMix};

    // The same extract-then-eat fixture the reserve-feeding test uses: an ENERGY reserve backed by oilseed, the
    // being carrying the extracted oilseed, the cell holding none.
    let cell = Coord3::ground(1, 1);
    let mut organs = BodyPlanRegistry::dev_default();
    let store = organs.organs.len() as u16;
    organs.organs.push(OrganKindDef {
        id: store,
        name: "seed-store".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[("oilseed", Fixed::ONE)]),
    });
    let reg = HomeostaticRegistry {
        axes: vec![
            HomeostaticAxisDef {
                id: ENERGY,
                name: "energy".to_string(),
                backing_component: Some("oilseed".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            },
            HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
                draw_set: Vec::new(),
            },
        ],
    };

    // A factory so the armed and unarmed runs start from bit-identical state (only the byproduct map differs).
    let build = || {
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x0115EED,
        );
        let blank = Controller::zeros(emb.layout());
        let mut walker = resting_walker(
            1,
            cell,
            body((1, 1), vec![organ(store, (1, 1))]),
            &reg,
            &organs,
            blank,
        );
        walker
            .homeostasis
            .set_level(ENERGY, Fixed::from_ratio(1, 2));
        let mut carried = SubstanceMix::new();
        carried.add("oilseed", Fixed::from_int(1000));
        walker.carried = carried;
        emb.add(walker, band(1));
        emb.set_material(MaterialField::new());
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::ground().expect("the embedded ground floor loads"),
        );
        emb
    };

    // UNARMED: no byproduct map. The bite feeds the reserve exactly as before and deposits NOTHING, so the
    // material field stays empty (the opt-in default that keeps every existing scenario byte-identical).
    let mut bare = build();
    let bare_gain = bare.geophage(StableId(1));
    assert!(bare_gain > Fixed::ZERO, "the unarmed bite still eats");
    assert!(
        bare.material().is_empty(),
        "an unarmed embodiment deposits no trace: the material field stays empty and folds no bytes"
    );

    // ARMED: map oilseed -> spent_hull. The deposit fraction is a dev value here; the reserved basis is the
    // shell-to-kernel mass ratio of an oil nut converted through the two substances' densities (surfaced
    // reserved-with-basis, not fabricated), and it lives as world data, never a code constant.
    let fraction = Fixed::from_ratio(2, 5);
    let mut armed = build();
    let mut byproducts = std::collections::BTreeMap::new();
    byproducts.insert("oilseed".to_string(), ("spent_hull".to_string(), fraction));
    armed.set_byproducts(byproducts);

    let coord = armed.walkers()[0].coord();
    let held = armed.walkers()[0].carried.volume("oilseed");
    let armed_gain = armed.geophage(StableId(1));

    // The trace is a byproduct, not a tax on the bite: the being gains exactly what the unarmed run gained.
    assert_eq!(
        armed_gain, bare_gain,
        "arming the byproduct deposits a trace without changing what the being gains from the bite"
    );

    // What left the inventory is what was eaten (the cell held no oilseed), and the deposit is that fraction of
    // it as spent_hull underfoot: the technique marked the ground it was practised on.
    let eaten = held - armed.walkers()[0].carried.volume("oilseed");
    assert!(
        eaten > Fixed::ZERO,
        "the bite ate oilseed to leave a trace of"
    );
    let deposited = armed.material().volume(coord, "spent_hull");
    assert_eq!(
        deposited,
        eaten
            .checked_mul(fraction)
            .expect("the fraction is bounded"),
        "the enact deposits the byproduct fraction of the eaten volume as spent_hull underfoot"
    );
    assert!(
        deposited > Fixed::ZERO && deposited < eaten,
        "the trace is present and is a residue (a fraction of the eaten mass, not the whole of it), so a being \
         that comes later can perceive it (trace slice C)"
    );
}

#[test]
fn a_being_crafts_a_tool_from_its_carried_stone_only_through_an_evolved_weight() {
    // Material-substrate arc item 4, crafting, THE KNAPPING: a being shapes its carried stone into a wielded
    // tool only because its evolved controller decided to, never because the engine scripts toolmaking. A
    // being carrying stone with a lifted craft weight wields a tool of that stone (its carried stock spent
    // by the tool volume); a blank founder carrying the same stone never crafts. Founder-zero, evolved
    // decision, the tool made of the stone worked (Principles 8, 9).
    use civsim_sim::material::{CraftParams, MaterialField, SubstanceMix};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    // A floor so the crafted edge derives from the worked granite's own fracture strength under the being's
    // forming force (the craft is physics-derived now, so it needs the stone's physics and a muscle to knap).
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
id = "mat.fracture_strength"
measures = "the stress a substance fractures at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[axis]]
id = "mat.edge_length_scale"
measures = "the finest working edge a material's microstructure holds"
unit = "m"
dimension = "1,0,0,0"
scale = "m"
tier = 0
range_lo = "0"
range_hi = "1"
real = "test fixture"

[[substance]]
id = "granite"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2700" },
  { axis = "mat.fracture_strength", value = "15" },
  { axis = "mat.edge_length_scale", value = "0.001" },
]
"#;

    let cell = Coord3::ground(2, 2);
    let stock = Fixed::from_int(5);

    let build = |craft_weight: bool| -> Runner {
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
            body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
            &reg,
            &organs,
            controller,
        );
        let mut carried = SubstanceMix::new();
        carried.add("granite", stock);
        walker.carried = carried;
        emb.add(walker, band(310));
        emb.set_craft_params(CraftParams::dev_fixture()); // tool volume 1; the edge is derived
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
        );
        emb.set_material(MaterialField::new());
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                8,
                8,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
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
fn a_crafted_edge_is_derived_from_the_worked_stone_so_a_hard_stone_makes_a_sharper_tool() {
    // The made-world arc, tool-use: the crafted edge is the INTRINSIC finest working edge the stone's
    // microstructure holds (mat.edge_length_scale), DERIVED from the material not an authored constant and not
    // the crafter's force (so cutting stays a real function of the wielder's force, R-EDGE-INTRINSIC). A
    // fine-grained hard obsidian holds a finer edge (a smaller contact area) than a coarse soft sandstone, and
    // a being carrying both shapes the fitter stone into its tool, the material chosen by physics not by id.
    use civsim_sim::material::{CraftParams, MaterialField, SubstanceMix, WieldedTool};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    // Two workable stones: a fine hard obsidian (edge 1e-4 m, hardness 6000) and a coarse soft sandstone (edge
    // 1e-3 m, hardness 80). The ids sort obsidian first; the fitness pick is proven distinct from id order
    // because obsidian also wins the derived cutting power.
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

[[axis]]
id = "mat.edge_length_scale"
measures = "the finest working edge a material's microstructure holds"
unit = "m"
dimension = "1,0,0,0"
scale = "m"
tier = 0
range_lo = "0"
range_hi = "1"
real = "test fixture"

[[substance]]
id = "obsidian"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2400" },
  { axis = "mat.indentation_hardness", value = "6000" },
  { axis = "mat.fracture_strength", value = "5000" },
  { axis = "mat.edge_length_scale", value = "0.0001" },
]

[[substance]]
id = "sandstone"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2300" },
  { axis = "mat.indentation_hardness", value = "80" },
  { axis = "mat.fracture_strength", value = "50" },
  { axis = "mat.edge_length_scale", value = "0.001" },
]
"#;

    let cell = Coord3::ground(2, 2);

    let build = |carried_stones: &[(&str, i32)]| -> Runner {
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
            0xC0DE,
        );
        let controller = Controller::zeros(emb.layout());
        let mut walker = resting_walker(
            1,
            cell,
            body((3, 4), vec![organ(fat, (1, 2)), organ(muscle, (1, 1))]),
            &reg,
            &organs,
            controller,
        );
        let mut carried = SubstanceMix::new();
        for (s, v) in carried_stones {
            carried.add(s, Fixed::from_int(*v));
        }
        walker.carried = carried;
        emb.add(walker, band(310));
        emb.set_craft_params(CraftParams::dev_fixture());
        emb.set_material_registry(
            civsim_physics::PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses"),
        );
        emb.set_material(MaterialField::new());
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                8,
                8,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        Runner::with_embodiment(uniform_field(8, 8, Fixed::from_int(310)), calib(), emb)
    };

    let wielded = |r: &Runner| -> Option<WieldedTool> {
        r.embodiment().unwrap().walkers()[0].wielded.clone()
    };

    // Craft from obsidian alone, then from sandstone alone: the hard obsidian holds the finer edge.
    let mut obs = build(&[("obsidian", 2)]);
    obs.embodiment_mut()
        .unwrap()
        .craft_from_carried(StableId(1));
    let mut sand = build(&[("sandstone", 2)]);
    sand.embodiment_mut()
        .unwrap()
        .craft_from_carried(StableId(1));
    let obs_tool = wielded(&obs).expect("obsidian holds an edge");
    let sand_tool = wielded(&sand).expect("sandstone holds an edge");
    assert_eq!(obs_tool.substance, "obsidian");
    assert_eq!(sand_tool.substance, "sandstone");
    assert!(
        obs_tool.contact_area < sand_tool.contact_area,
        "the harder stone holds a finer (smaller-area) edge under the same force: {} < {}",
        obs_tool.contact_area,
        sand_tool.contact_area
    );

    // Carrying BOTH, the being shapes the fitter stone (obsidian) into its tool, by capability not by id.
    let mut both = build(&[("obsidian", 2), ("sandstone", 2)]);
    both.embodiment_mut()
        .unwrap()
        .craft_from_carried(StableId(1));
    assert_eq!(
        wielded(&both).expect("a tool is made").substance,
        "obsidian",
        "carrying two stones, the being crafts the fitter one derived from physics, not the first by id"
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

[[axis]]
id = "mat.edge_length_scale"
measures = "the finest working edge a material's microstructure holds"
unit = "m"
dimension = "1,0,0,0"
scale = "m"
tier = 0
range_lo = "0"
range_hi = "1"
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
  { axis = "mat.fracture_strength", value = "100" },
  { axis = "mat.edge_length_scale", value = "0.001" },
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
        MediumField::uniform(
            8,
            8,
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::from_int(10),
            Fixed::ZERO,
        ),
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
fn a_being_digs_a_pit_lowering_the_terrain_only_through_an_evolved_weight() {
    // Material-substrate arc item 5, MODIFIABLE TERRAIN: a being excavates the ground underfoot, and the
    // removed matter both loads its carrier AND lowers the column (a pit forms), so digging reshapes the
    // terrain, not only mines it. It happens only through an evolved dig decision and the fracture contest:
    // a deciding being that can fracture the ground digs a pit and carries the spoil; a blank founder on the
    // same ground never digs and the terrain is untouched. Founder-zero, physics-gated (Principles 8, 9).
    use civsim_sim::material::{ExtractionParams, MaterialField};
    use civsim_sim::physiology::MUSCLE_STRENGTH;

    let cell = Coord3::ground(2, 2);
    let heap = Fixed::from_int(100000);
    let build = |dig_weight: bool| -> Runner {
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
            AffordanceRegistry::dev_digger(),
            LocomotionParams::dev_default(),
            0,
            0xD16,
        );
        // The dig output is the fifth output (move [act,dx,dy] 0..2, ingest [act] 3, dig [act] 4).
        assert_eq!(
            emb.layout().n_out(),
            5,
            "digger layout: move(3) + ingest(1) + dig(1)"
        );
        let n_in = emb.layout().n_in();
        let controller = if dig_weight {
            let mut w = vec![Fixed::ZERO; emb.layout().weight_count()];
            w[4 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> dig activation
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
        // A small working area, so the being's force concentrates to a pressure that fractures the granite.
        emb.set_extraction_params(ExtractionParams {
            working_area: Fixed::from_ratio(1, 1_000_000),
            pressure_max: Fixed::from_int(150_000),
        });
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                10,
                10,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        Runner::with_embodiment(uniform_field(10, 10, Fixed::from_int(305)), calib(), emb)
    };

    let carried =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().walkers()[0].carried.total_volume() };
    let ground =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(cell, "granite") };
    let elevation = |r: &Runner| -> Fixed { r.embodiment().unwrap().earthwork().delta(cell) };

    // The deciding being digs: it fractures the ground, carries the spoil, and the column drops by exactly
    // the volume it removed (a pit), the ground conserved between the carrier and the lowered terrain.
    let mut digger = build(true);
    digger.step();
    let spoil = carried(&digger);
    assert!(
        spoil > Fixed::ZERO,
        "the deciding being excavates the ground"
    );
    assert_eq!(
        ground(&digger),
        heap - spoil,
        "the cell lost exactly what was excavated"
    );
    assert_eq!(
        elevation(&digger),
        Fixed::ZERO - spoil,
        "the column dropped by the excavated volume: a pit formed"
    );

    // The blank founder never digs: the terrain is untouched.
    let mut founder = build(false);
    founder.step();
    assert_eq!(
        carried(&founder),
        Fixed::ZERO,
        "the blank founder does not dig"
    );
    assert_eq!(elevation(&founder), Fixed::ZERO, "the terrain is unchanged");
}

#[test]
fn releasing_a_carried_load_raises_the_column_the_mound_half_of_terraforming() {
    // Material-substrate arc item 5, the DEPOSIT-AND-MOUND half: a being sets its carried load down and the
    // column rises by the volume deposited, conservation-symmetric with digging lowering it. So a mound is
    // the consequence of the release primitive, not a coded verb: what a being digs from a pit and carries
    // elsewhere raises a mound there, and terracing emerges from the dig and release primitives. It happens
    // only through the evolved release decision; a blank founder holding the same load never sets it down.
    use civsim_sim::material::SubstanceMix;

    let cell = Coord3::ground(3, 3);
    let load = Fixed::from_int(5);
    let (organs, fat) = energy_registry();
    let reg = energy_thermal_registry();

    let build = |release_weight: bool| -> Runner {
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_earthmover(),
            LocomotionParams::dev_default(),
            0,
            0xEA47,
        );
        // Outputs: move [act,dx,dy] 0..2, ingest [act] 3, dig [act] 4, release [act] 5.
        assert_eq!(
            emb.layout().n_out(),
            6,
            "earthmover layout: move(3) + ingest(1) + dig(1) + release(1)"
        );
        let n_in = emb.layout().n_in();
        let controller = if release_weight {
            let mut w = vec![Fixed::ZERO; emb.layout().weight_count()];
            w[5 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> release activation
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
        // The being already carries a load (dug elsewhere and brought here).
        let mut carried = SubstanceMix::new();
        carried.add("granite", load);
        walker.carried = carried;
        emb.add(walker, band(310));
        Runner::with_embodiment(uniform_field(8, 8, Fixed::from_int(310)), calib(), emb)
    };

    let carried =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().walkers()[0].carried.total_volume() };
    let ground =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(cell, "granite") };
    let elevation = |r: &Runner| -> Fixed { r.embodiment().unwrap().earthwork().delta(cell) };

    // The deciding being sets its load down: the cell gains the matter, the column rises by exactly the
    // deposited volume (a mound), and the being carries nothing more.
    let mut mover = build(true);
    mover.step();
    assert_eq!(
        carried(&mover),
        Fixed::ZERO,
        "the being set its whole load down"
    );
    assert_eq!(ground(&mover), load, "the matter is now on the ground");
    assert_eq!(
        elevation(&mover),
        load,
        "the column rose by the deposited volume: a mound"
    );

    // The blank founder never releases: it holds its load and the terrain is unchanged.
    let mut founder = build(false);
    founder.step();
    assert_eq!(carried(&founder), load, "the blank founder holds its load");
    assert_eq!(ground(&founder), Fixed::ZERO, "nothing was set down");
    assert_eq!(elevation(&founder), Fixed::ZERO, "the terrain is unchanged");
}

#[test]
fn a_dug_pit_recouples_the_hydrology_so_the_cell_becomes_a_basin_through_the_runner() {
    // Material-substrate arc item 5, the HYDROLOGY COUPLING through the full runner: a being digs a pit and
    // the environmental stack recouples its downhill routing to the reshaped terrain, so the dug cell
    // becomes a basin that retains its water where before it drained. It happens only through the evolved
    // dig decision and only where the ground is reshaped: a deciding being turns its cell into a basin; a
    // blank being that digs nothing leaves the worldgen routing untouched. Proven identically in the pinned
    // and scheduled tick orders. Physics-gated, no race or label (Principles 3, 9).
    use civsim_sim::environ::{EnvironCalib, EnvironFields};
    use civsim_sim::material::{ExtractionParams, MaterialField};
    use civsim_sim::physiology::MUSCLE_STRENGTH;
    use civsim_world::{BiomeSet, FlatBounded, TileMap, WorldgenParams};

    let (w, h) = (16, 12);
    let map = TileMap::generate(
        0x5EED,
        FlatBounded::new(w, h, 1),
        &BiomeSet::dev_default(),
        &WorldgenParams::dev_default(),
    );
    // Choose the interior cell that is CLOSEST to becoming a basin: it drains now (its elevation is above
    // its lowest neighbour, so it is no worldgen basin), by the smallest margin on the map. A single dig
    // then tips it below its lowest neighbour and into a basin, so the dig is the sole cause of the flip (a
    // clean causal separation, robust to the generated elevation). The margin is read from the exposed tile
    // elevations, so the test picks a cell a realistic scoop can clear.
    let elev = |x: i32, y: i32| map.tile(Coord3::ground(x, y)).unwrap().elevation();
    let margin = |x: i32, y: i32| -> Option<Fixed> {
        let here = elev(x, y);
        let low = [
            elev(x, y - 1),
            elev(x, y + 1),
            elev(x - 1, y),
            elev(x + 1, y),
        ]
        .into_iter()
        .fold(here, |a, b| if b < a { b } else { a });
        if here > low {
            Some(here - low)
        } else {
            None // already a basin (no strictly-lower neighbour)
        }
    };
    let (fx, fy) = (1..w - 1)
        .flat_map(|x| (1..h - 1).map(move |y| (x, y)))
        .filter(|&(x, y)| margin(x, y).is_some())
        .min_by_key(|&(x, y)| margin(x, y).unwrap())
        .expect("some interior cell of the generated map drains rather than ponding");
    let cell = Coord3::ground(fx, fy);
    let heap = Fixed::from_int(100000);

    let build = |dig_weight: bool| -> Runner {
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
            AffordanceRegistry::dev_digger(),
            LocomotionParams::dev_default(),
            0,
            0xD16,
        );
        let n_in = emb.layout().n_in();
        let controller = if dig_weight {
            let mut wv = vec![Fixed::ZERO; emb.layout().weight_count()];
            wv[4 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> dig activation
            Controller::from_weights(n_in, emb.layout().n_out(), 0, wv)
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
            working_area: Fixed::from_ratio(1, 1_000_000),
            pressure_max: Fixed::from_int(150_000),
        });
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                w,
                h,
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        let mut r =
            Runner::with_embodiment(uniform_field(w, h, Fixed::from_int(305)), calib(), emb);
        r.set_environ(EnvironFields::from_map(&map), EnvironCalib::dev_fixture());
        r
    };

    let is_basin = |r: &Runner| -> bool { r.environ().unwrap().is_basin(fx, fy) };
    let dug = |r: &Runner| -> Fixed { r.embodiment().unwrap().earthwork().delta(cell) };

    // Pinned order: the being excavates a pit and the routing recouples the cell into a basin.
    let mut pinned = build(true);
    assert!(
        !is_basin(&pinned),
        "the chosen cell drains on the bare worldgen map, it is no basin yet"
    );
    pinned.step();
    assert!(dug(&pinned) < Fixed::ZERO, "the deciding being dug a pit");
    assert!(
        is_basin(&pinned),
        "the dug pit recoupled the hydrology: the cell is now a basin that pools its water (pinned order)"
    );

    // Scheduled order: the deterministic scheduler recouples identically.
    let mut scheduled = build(true);
    scheduled.step_scheduled(&[]);
    assert!(
        is_basin(&scheduled),
        "the scheduled order recouples the same basin (bit-identical to the pinned order)"
    );

    // A blank being digs nothing: the earthwork stays empty, so the routing keeps the worldgen baseline and
    // the cell still drains (the opt-in no-op that keeps a non-digging run byte-identical).
    let mut blank = build(false);
    blank.step();
    assert_eq!(dug(&blank), Fixed::ZERO, "the blank being digs nothing");
    assert!(
        !is_basin(&blank),
        "with nothing dug the routing is untouched and the cell still drains"
    );
}

#[test]
fn combustible_matter_burns_when_it_is_hot_enough_and_a_cold_cell_and_rock_do_not() {
    // Material-substrate arc item 6, LIVE FIRE: a cell of combustible matter that stands at or above its
    // ignition temperature burns through the resolved combustion law, consuming its fuel and lighting the
    // fire field; the identical cell in a cold field does not, and a non-combustible substance in the same
    // hot cell is untouched. The outcome is the substance's own combustion data against the cell temperature,
    // no race, kind, or role (Principle 9), and it is opt-in: with no combustion armed nothing burns.
    use civsim_sim::material::{CombustionCalib, MaterialField};

    let cell = Coord3::ground(2, 2);
    let fuel0 = Fixed::from_int(4);
    let rock0 = Fixed::from_int(4);
    let (w, h) = (8, 8);

    // A runner holding oak (a combustible carrying therm.fuel_value and therm.ignition_temperature 570) and
    // granite (no fuel value) at one cell, over a UNIFORM temperature field (a fixed point of the diffusion
    // step, so the cell holds its temperature), with the combustion beat armed.
    let build = |field_temp: i32| -> Runner {
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg,
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x00F1,
        );
        let mut field = MaterialField::new();
        field.deposit(cell, "oak", fuel0);
        field.deposit(cell, "granite", rock0);
        emb.set_material(field);
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        let mut r = Runner::with_embodiment(
            uniform_field(w, h, Fixed::from_int(field_temp)),
            calib(),
            emb,
        );
        r.set_combustion(CombustionCalib::dev_fixture());
        r
    };

    let oak = |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(cell, "oak") };
    let rock = |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(cell, "granite") };
    let burning = |r: &Runner| -> Fixed { r.embodiment().unwrap().fire().intensity(cell) };

    // Hot field (600 K, above oak's 570 K ignition): the oak ignites, burns down its fuel, and lights the
    // fire field; the granite in the same cell has no fuel value and is untouched.
    let mut hot = build(600);
    hot.step();
    assert!(
        oak(&hot) < fuel0,
        "the hot oak burned down some of its fuel"
    );
    assert!(
        burning(&hot) > Fixed::ZERO,
        "the burning cell lit the fire field"
    );
    assert_eq!(
        rock(&hot),
        rock0,
        "the non-combustible granite in the same cell did not burn"
    );

    // Burning continues while fuel and heat remain: a second tick consumes more oak.
    let after_one = oak(&hot);
    hot.step();
    assert!(
        oak(&hot) < after_one,
        "the fire keeps consuming fuel while it burns"
    );

    // Cold field (305 K, below the ignition temperature): the same oak never ignites, so its fuel is intact
    // and the fire field stays dark.
    let mut cold = build(305);
    cold.step();
    assert_eq!(
        oak(&cold),
        fuel0,
        "cold oak below its ignition point does not burn"
    );
    assert_eq!(burning(&cold), Fixed::ZERO, "a cold cell is not on fire");

    // The scheduled tick order sources the same fire as the pinned order (bit-identical).
    let mut scheduled = build(600);
    scheduled.step_scheduled(&[]);
    assert!(
        burning(&scheduled) > Fixed::ZERO && oak(&scheduled) < fuel0,
        "the scheduled order burns the fuel and lights the fire identically"
    );
}

#[test]
fn an_oxygen_demanding_fire_burns_in_air_and_starves_in_an_anoxic_medium() {
    // Material-substrate arc item 6, the OXYGEN GATE: an oxygen-demanding fuel (oak, which declares a
    // therm.oxidiser_demand) burns only where the cell's medium supplies the oxidiser. In a rich (breathable)
    // medium the same hot oak burns as it does in open air; in a near-anoxic medium the combustion goes
    // oxidiser-limited to nothing and the fuel is spared, so fire needs air, from the medium's respirable
    // content against the fuel's own stoichiometry, no coded rule (Principles 8, 9).
    use civsim_sim::material::{CombustionCalib, MaterialField};

    let (w, h) = (8, 8);
    let cell = Coord3::ground(2, 2);
    let fuel0 = Fixed::from_int(4);

    let build = |respirable: Fixed| -> Runner {
        let (organs, _fat) = energy_registry();
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg,
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x0A17,
        );
        let mut field = MaterialField::new();
        field.deposit(cell, "oak", fuel0);
        emb.set_material(field);
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        // A uniform medium at the given respirable content: rich air breathes the fire, a near-anoxic medium
        // starves it. Density and temperature are irrelevant to the oxidiser read.
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(
            organs,
            MediumField::uniform(
                w,
                h,
                respirable,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
        ));
        let mut r =
            Runner::with_embodiment(uniform_field(w, h, Fixed::from_int(600)), calib(), emb);
        r.set_combustion(CombustionCalib::dev_fixture());
        r
    };

    let oak = |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(cell, "oak") };
    let burning = |r: &Runner| -> Fixed { r.embodiment().unwrap().fire().intensity(cell) };

    // Rich air: the hot oak ignites and burns down its fuel.
    let mut air = build(Fixed::ONE);
    air.step();
    assert!(oak(&air) < fuel0, "oak in rich air burns");
    assert!(burning(&air) > Fixed::ZERO, "the fire is alight in air");

    // Near-anoxic medium: the same hot oak cannot get the oxidiser it demands, so it does not burn.
    let mut anoxic = build(Fixed::ZERO);
    anoxic.step();
    assert_eq!(
        oak(&anoxic),
        fuel0,
        "oak in an anoxic medium is spared: no oxidiser, no burn"
    );
    assert_eq!(
        burning(&anoxic),
        Fixed::ZERO,
        "the fire starves without air"
    );
}

#[test]
fn fire_spreads_along_a_fuel_row_and_burns_out_behind_it() {
    // Material-substrate arc item 6, LIVE FIRE, the emergent payoff: a burning cell raises its temperature
    // by the heat its combustion releases, that heat spreads through the ordinary temperature diffusion, and
    // a neighbouring fuel cell whose temperature crosses its ignition gate catches. So fire SPREADS from cell
    // to cell and EXTINGUISHES when a cell's fuel runs out, both emergent from the physics with no coded
    // spread rule: it is the combustion gate over the diffused temperature, tick after tick.
    use civsim_sim::material::{CombustionCalib, MaterialField};

    let (w, h) = (8, 8);
    let src = Coord3::ground(2, 2);
    let mid = Coord3::ground(3, 2);
    let far = Coord3::ground(4, 2);

    let build = || -> Runner {
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg,
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x00F1,
        );
        let mut field = MaterialField::new();
        for c in [src, mid, far] {
            field.deposit(c, "oak", Fixed::from_int(50));
        }
        emb.set_material(field);
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        // A sustained ignition source at src (baseline 900, which holds above oak's 570 K ignition against
        // the diffusion to its cold neighbours), the rest of the field cold ambient (305 K).
        let mut baseline = vec![Fixed::from_int(305); (w * h) as usize];
        baseline[(2 * w + 2) as usize] = Fixed::from_int(900);
        // A fire-timescale field calibration: the ambient relaxation is slow next to a fire, so the
        // combustion heat persists and spreads rather than being pulled straight back to the cold baseline.
        // A labelled fixture, not owner canon; the reserved heat fraction and these field rates jointly set
        // how readily fire spreads, the owner's to calibrate to the real fire and ambient dynamics.
        let fire_calib = FieldCalib {
            diffusion: Fixed::from_ratio(1, 8),
            relaxation: Fixed::from_ratio(1, 64),
            exchange: Fixed::from_ratio(1, 2),
        };
        let mut r = Runner::with_embodiment(Field::new(w, h, baseline), fire_calib, emb);
        r.set_combustion(CombustionCalib::dev_fixture());
        r
    };

    let lit = |r: &Runner, c: Coord3| -> Fixed { r.embodiment().unwrap().fire().intensity(c) };

    let mut r = build();
    // A few ticks in: only the source is alight; the far cell has not caught yet.
    for _ in 0..5 {
        r.step();
    }
    let src_early = lit(&r, src);
    assert!(src_early > Fixed::ZERO, "the ignition source is alight");
    assert_eq!(lit(&r, far), Fixed::ZERO, "the far cell has not caught yet");

    // Run on: the heat has spread down the row and the far cell, cold at the start, is now burning.
    for _ in 0..35 {
        r.step();
    }
    assert!(
        lit(&r, far) > Fixed::ZERO,
        "fire spread down the fuel row and the far cell caught, through the temperature field alone"
    );
    assert!(
        lit(&r, src) < src_early,
        "the source is burning down: its fire dims as its fuel depletes"
    );

    // Run to exhaustion: the source, out of fuel, has dropped out of the fire field (extinction), no coded
    // burnout rule, the combustion gate simply stops firing when there is nothing left to burn.
    for _ in 0..80 {
        r.step();
    }
    assert_eq!(
        lit(&r, src),
        Fixed::ZERO,
        "the source burned out and dropped out of the fire field"
    );

    // The scheduled tick order spreads the fire bit-identically to the pinned order.
    let mut pinned = build();
    let mut scheduled = build();
    for _ in 0..40 {
        pinned.step();
        scheduled.step_scheduled(&[]);
    }
    assert_eq!(
        lit(&pinned, far),
        lit(&scheduled, far),
        "the scheduled and pinned orders spread the fire identically"
    );
}

#[test]
fn organic_matter_decomposes_when_warm_and_the_matter_cycle_conserves_mass() {
    // Material-substrate arc item 8, THE MATTER CYCLE: a cell's organic matter (carrion, which carries a
    // biological composition) decomposes over time when it is warm enough, its lost mass leaving the located
    // matter for the environment; a frozen cell preserves it. The decomposition is EXACTLY mass-conserving:
    // what a cell loses enters the decomposed-mass sink bit for bit, so the total is invariant, the hard
    // conservation the ConservationRegistry guards. Keyed off the substance's own composition physics and
    // the cell temperature, no race, kind, or role (Principles 8, 9).
    use civsim_sim::conservation::ConservationRegistry;
    use civsim_sim::material::{MaterialField, MatterCycleCalib};

    let (w, h) = (8, 8);
    let cell = Coord3::ground(2, 2);
    let flesh0 = Fixed::from_int(10);
    let reg = civsim_physics::PhysicsRegistry::ground().unwrap();

    let build = |field_temp: i32| -> Runner {
        let hreg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            hreg,
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0xDECA,
        );
        let mut field = MaterialField::new();
        field.deposit(cell, "carrion", flesh0);
        emb.set_material(field);
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        let mut r = Runner::with_embodiment(
            uniform_field(w, h, Fixed::from_int(field_temp)),
            calib(),
            emb,
        );
        r.set_matter_cycle(MatterCycleCalib::dev_fixture());
        r
    };

    let flesh =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(cell, "carrion") };
    // The total matter mass the conservation guard watches: the located matter plus the decomposed sink.
    let total_mass = move |r: &Runner| -> i128 {
        let m = r.embodiment().unwrap().material().mass(cell, &reg);
        let sink = r.embodiment().unwrap().decomposed_mass();
        (m + sink).to_bits() as i128
    };

    // A warm cell (300 K, above the 273 K decomposition barrier): the carrion rots, its mass moving into the
    // sink, and the ConservationRegistry sees the total unchanged (exact conservation).
    let mut warm = build(300);
    let mut conservation: ConservationRegistry<Runner> = ConservationRegistry::new();
    conservation.register("matter_mass", total_mass);
    let baseline = conservation.snapshot(&warm);
    warm.step();
    assert!(flesh(&warm) < flesh0, "warm carrion decomposes");
    assert!(
        warm.embodiment().unwrap().decomposed_mass() > Fixed::ZERO,
        "the decomposed mass moved into the sink"
    );
    conservation
        .check_against(&baseline, &warm)
        .expect("the matter cycle conserves mass exactly (cell matter plus sink is invariant)");

    // Many ticks keep conserving as the carrion rots down.
    for _ in 0..10 {
        warm.step();
    }
    conservation
        .check_against(&baseline, &warm)
        .expect("mass stays conserved as the carrion rots over many ticks");
    assert!(
        flesh(&warm) < flesh0.checked_div(Fixed::from_int(2)).unwrap(),
        "the carrion has rotted well down"
    );

    // Slice C: the decomposed mass re-materialised into the cell's SOIL nutrient store (not a placeless
    // scalar), LOCATED where the carrion rotted and SPLIT by the substance's own composition into a mineral
    // class (the ash fraction) and an organic class (the remainder). The split is exact and complete.
    let soil = warm.embodiment().unwrap().soil();
    let mineral = soil.mass(cell, "bio.mineral_ash_fraction");
    let organic = soil.mass(cell, "bio.organic_residue");
    assert!(mineral > Fixed::ZERO, "the mineral ash enriched the soil");
    assert!(
        organic > Fixed::ZERO,
        "the organic residue enriched the soil"
    );
    assert_eq!(
        mineral + organic,
        warm.embodiment().unwrap().decomposed_mass(),
        "the mineral and organic shares are the whole decomposed mass (the split is complete)"
    );
    assert_eq!(
        soil.cell_total(cell),
        warm.embodiment().unwrap().decomposed_mass(),
        "all the nutrient is located at the cell where the carrion rotted"
    );
    assert_eq!(
        soil.cell_total(Coord3::ground(0, 0)),
        Fixed::ZERO,
        "a cell where nothing rotted is not enriched"
    );
    // Carrion's mineral-ash fraction (0.05) is small, so the organic residue is the larger share.
    assert!(
        organic > mineral,
        "the organic residue outweighs the mineral ash (carrion is mostly soft tissue)"
    );

    // A frozen cell (250 K, below the barrier): the carrion is preserved and nothing decomposes.
    let mut frozen = build(250);
    frozen.step();
    assert_eq!(flesh(&frozen), flesh0, "frozen carrion is preserved");
    assert_eq!(
        frozen.embodiment().unwrap().decomposed_mass(),
        Fixed::ZERO,
        "nothing decomposed in the cold"
    );

    // The scheduled tick order decomposes bit-identically to the pinned order.
    let mut pinned = build(300);
    pinned.step();
    let mut scheduled = build(300);
    scheduled.step_scheduled(&[]);
    assert_eq!(
        scheduled.embodiment().unwrap().decomposed_mass(),
        pinned.embodiment().unwrap().decomposed_mass(),
        "the scheduled and pinned orders decompose identically"
    );
}

#[test]
fn the_decomposition_split_is_data_defined_and_gated_on_the_barrier_not_the_ash_axis() {
    // Chemistry arc, T5: decomposability is the substance declaring a decomposition BARRIER (its own physical
    // gate), NOT the presence of an Earth mineral-ash axis (the retired gather-gate), and how the lost mass
    // splits into soil classes is a DATA-defined ConstituentRegistry, not a hardcoded ash-plus-organic pair.
    // Proof: arm a world whose registry references a fraction axis carrion does NOT carry. Carrion still
    // decomposes (the barrier gate, not the ash gate), and its whole lost mass lands in the world's own
    // residual class, none in the Earth `bio.*` classes, so the split follows the armed data, never a bucket
    // baked into the engine.
    use civsim_sim::material::{ConstituentRegistry, MaterialField, MatterCycleCalib};

    let (w, h) = (8, 8);
    let cell = Coord3::ground(2, 2);
    let flesh0 = Fixed::from_int(10);

    let hreg = energy_thermal_registry();
    let mut emb = Embodiment::new(
        hreg,
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        0,
        0xDECA,
    );
    let mut field = MaterialField::new();
    field.deposit(cell, "carrion", flesh0); // carrion carries mineral_ash_fraction AND a 273 K barrier
    emb.set_material(field);
    emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
    let mut r = Runner::with_embodiment(uniform_field(w, h, Fixed::from_int(300)), calib(), emb);
    r.set_matter_cycle(MatterCycleCalib::dev_fixture());
    // The world's OWN chemistry: a residual class and a constituent keyed on an axis carrion does not carry,
    // so nothing is carved out and the whole loss falls to the residual. Neither class is a `bio.*` Earth name.
    let mut world_reg = ConstituentRegistry::new("world.humus");
    world_reg.push("world.labile_fraction", "world.labile");
    r.set_constituents(world_reg);

    r.step();

    let soil = r.embodiment().unwrap().soil();
    let decomposed = r.embodiment().unwrap().decomposed_mass();
    assert!(
        decomposed > Fixed::ZERO,
        "carrion still decomposes: decomposability is its BARRIER, not the ash axis (retired gather-gate)"
    );
    assert_eq!(
        soil.mass(cell, "world.humus"),
        decomposed,
        "the whole lost mass lands in the WORLD'S own residual class (the split follows the armed data)"
    );
    assert_eq!(
        soil.mass(cell, "bio.mineral_ash_fraction"),
        Fixed::ZERO,
        "no mass leaks to the Earth mineral class: the ash bucket is no longer hardcoded"
    );
    assert_eq!(
        soil.mass(cell, "bio.organic_residue"),
        Fixed::ZERO,
        "no mass leaks to the Earth organic class: the split is data-defined end to end"
    );
}

#[test]
fn the_matter_cycle_conserves_material_plus_soil_plus_tissue_across_both_legs() {
    // Chemistry arc, Arc 4: the CLOSED decay ledger, registered. Decomposition moves matter between three
    // pools (located material, the soil-nutrient store, and located body tissue) and holds their SUM invariant:
    // a substance's lost mass re-materialises into the soil bit for bit through the constituent split, and a
    // decayed body's lost volume the same through the tissue return leg. With BOTH a rotting carrion substance
    // AND a rotting body parcel present, and the producer biomass layer OPEN (no extract beat armed), the
    // registered ledger is conserved across the tick and both legs are active (material and tissue both fall,
    // the soil store rises). This closes the loop the earlier material-only conservation half-covered.
    use civsim_sim::conservation::ConservationRegistry;
    use civsim_sim::material::{MaterialField, MatterCycleCalib, TissueField};

    let (w, h) = (8, 8);
    let mcell = Coord3::ground(2, 2);
    let tcell = Coord3::ground(3, 3);
    let ground = || civsim_physics::PhysicsRegistry::ground().unwrap();

    let hreg = energy_thermal_registry();
    let mut emb = Embodiment::new(
        hreg,
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        0,
        0xDECA,
    );
    let mut field = MaterialField::new();
    field.deposit(mcell, "carrion", Fixed::from_int(10)); // decomposes via its barrier (material leg)
    emb.set_material(field);
    emb.set_material_registry(ground());
    // A located body parcel that decays unconditionally at the reserved rate (the tissue return leg): its own
    // composition (here a single nutrient) carves nothing from the default split, so its whole volume returns.
    let mut tissue = TissueField::new();
    let body: std::collections::BTreeMap<String, Fixed> =
        [("bio.energy_density".to_string(), Fixed::from_int(5))]
            .into_iter()
            .collect();
    tissue.deposit(tcell, body, Fixed::from_int(8));
    emb.set_tissue(tissue);
    let mut r = Runner::with_embodiment(uniform_field(w, h, Fixed::from_int(300)), calib(), emb);
    r.set_matter_cycle(MatterCycleCalib::dev_fixture());

    let mut conservation: ConservationRegistry<Runner> = ConservationRegistry::new();
    conservation.register("decay_ledger", |r: &Runner| {
        r.embodiment().unwrap().decay_ledger_mass().to_bits() as i128
    });
    let material0 = r.embodiment().unwrap().material().total_mass(&ground());
    let tissue0 = r.embodiment().unwrap().tissue().total_volume();
    let baseline = conservation.snapshot(&r);

    r.step();
    conservation
        .check_against(&baseline, &r)
        .expect("the matter cycle conserves material + soil + tissue across both legs");
    assert!(
        r.embodiment().unwrap().material().total_mass(&ground()) < material0,
        "the material leg rotted (the carrion mass fell)"
    );
    assert!(
        r.embodiment().unwrap().tissue().total_volume() < tissue0,
        "the tissue leg rotted (the body volume fell)"
    );
    assert!(
        r.embodiment().unwrap().decomposed_mass() > Fixed::ZERO,
        "the soil store gained the matter both legs lost"
    );

    for _ in 0..10 {
        r.step();
    }
    conservation
        .check_against(&baseline, &r)
        .expect("the ledger stays conserved as both pools rot down over many ticks");
}

#[test]
fn decomposition_is_driven_by_life_and_conditions_not_by_an_engine_law() {
    // DECOMPOSITION-AS-EMERGENCE (Principle 8), the whole point: the matter cycle no longer asserts that all
    // warm matter rots. The substance's own rate is now its MAXIMUM susceptibility, and the fraction a cell
    // realises this tick is a per-cell ACTIVITY the world derives from the cell's decomposer LIFE and its
    // CONDITIONS. The decisive case (the one an abiotic proxy cannot express) is the sterile-but-favorable
    // cell: warm, but with no decomposer life present, it does not rot. The physics barrier gate is untouched,
    // so a frozen remains is still preserved, and an unarmed runner decays exactly as before (the opt-in flip).
    use civsim_sim::decompose::{DecomposerDriver, DecomposerDriverRegistry, DecomposerStockField};
    use civsim_sim::material::{MaterialField, MatterCycleCalib};

    let (w, h) = (8, 8);
    let cell = Coord3::ground(2, 2);
    let flesh0 = Fixed::from_int(10);

    // A runner with the matter cycle armed and, optionally, a decomposer registry and a standing-biomass
    // stock. The carrion carries the mineral-ash and 273 K freezing barrier axes, so it is organic matter the
    // cycle can act on; nothing else is armed, so moisture and oxygen have no field and default to full (the
    // open-air convention), leaving the conditions kernel's WARMTH term the live gate here.
    let build = |field_temp: i32,
                 decomposer: Option<DecomposerDriverRegistry>,
                 stock: Option<DecomposerStockField>|
     -> Runner {
        let hreg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            hreg,
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0xDECA,
        );
        let mut field = MaterialField::new();
        field.deposit(cell, "carrion", flesh0);
        emb.set_material(field);
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        let mut r = Runner::with_embodiment(
            uniform_field(w, h, Fixed::from_int(field_temp)),
            calib(),
            emb,
        );
        r.set_matter_cycle(MatterCycleCalib::dev_fixture());
        if let Some(d) = decomposer {
            r.set_decomposer(d);
        }
        if let Some(s) = stock {
            r.set_decomposer_stock(s);
        }
        r
    };
    let flesh =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(cell, "carrion") };
    let life_only = || {
        let mut d = DecomposerDriverRegistry::new();
        d.push(DecomposerDriver::life(Fixed::ONE));
        d
    };
    let conditions_only = || {
        let mut d = DecomposerDriverRegistry::new();
        d.push(DecomposerDriver::conditions(
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            Fixed::from_int(100),
        ));
        d
    };
    let seeded = || {
        let mut s = DecomposerStockField::new();
        s.seed(cell, "carrion", Fixed::ONE);
        s
    };

    // (1) THE CONTROL (the deliberate flip): the matter cycle armed but NO decomposer registry decays the warm
    // carrion at its unconditional rate, today's behaviour. The defect cure is the owner's arming, not a
    // silent engine default.
    let mut control = build(300, None, None);
    control.step();
    assert!(
        flesh(&control) < flesh0,
        "unarmed: warm carrion rots at its unconditional rate (the pre-emergence control)"
    );

    // (2) THE STERILE-BUT-FAVORABLE CASE (the decisive one an abiotic proxy cannot express): a Life-only
    // registry over a WARM cell with ZERO decomposer biomass decays NOTHING, because no decomposer life is
    // present. Decay is driven by life, not by warmth.
    let mut sterile = build(300, Some(life_only()), None);
    sterile.step();
    assert_eq!(
        flesh(&sterile),
        flesh0,
        "sterile: warm carrion no decomposer life acts on is preserved"
    );
    assert_eq!(
        sterile.embodiment().unwrap().decomposed_mass(),
        Fixed::ZERO,
        "nothing decomposed with no decomposer life present, however warm"
    );

    // The SAME warm cell, once decomposer life stands in it (a hand-seeded stock, the biosphere's job later),
    // now rots: decay emerges from the presence of life.
    let mut colonised = build(300, Some(life_only()), Some(seeded()));
    colonised.step();
    assert!(
        flesh(&colonised) < flesh0,
        "colonised: the same warm cell rots once decomposer life stands in it"
    );
    assert!(
        colonised.embodiment().unwrap().decomposed_mass() > Fixed::ZERO,
        "the standing decomposer biomass drove the decay"
    );

    // (3) THE FROZEN PHYSICS GATE (untouched even when armed): a decomposer-armed, colonised cell BELOW the
    // substance's own freezing barrier is preserved by the physics gate upstream of the activity factor, so
    // falsifiability-by-physics survives: a frozen world does not rot however much decomposer life it carries.
    let mut frozen = build(250, Some(life_only()), Some(seeded()));
    frozen.step();
    assert_eq!(
        flesh(&frozen),
        flesh0,
        "frozen: preserved by the barrier gate even with decomposer life present"
    );

    // (4) THE CONDITIONS KERNEL, wired end-to-end: under a conditions-only registry a barely-thawed cell (one
    // degree above the barrier) rots far SLOWER than a hot one, because the warmth term reads the cell
    // temperature through to the realised rate. Both are warm, so both rot; the hotter rots more.
    let mut barely = build(274, Some(conditions_only()), None);
    let mut hot = build(373, Some(conditions_only()), None);
    barely.step();
    hot.step();
    assert!(
        flesh(&barely) < flesh0 && flesh(&hot) < flesh0,
        "both warm cells rot under the conditions kernel"
    );
    assert!(
        flesh(&hot) < flesh(&barely),
        "the hotter cell rots faster: the conditions kernel reads the cell temperature end-to-end"
    );

    // (5) EXACT MASS CONSERVATION with the factor in the path: the activity multiplies the volume UPSTREAM of
    // the unchanged mass-difference math, so the colonised cell's lost mass still lands in the soil store bit
    // for bit (the conservation the ConservationRegistry guards).
    assert_eq!(
        colonised.embodiment().unwrap().soil().cell_total(cell),
        colonised.embodiment().unwrap().decomposed_mass(),
        "the decomposition conserves mass exactly with the activity factor in the path"
    );

    // (6) THE COMBINE MODE (a blind audit caught this): when a world arms BOTH a Life row and a favorable
    // Conditions row, the default All combine GATES them, so a sterile warm-wet cell still preserves (the
    // Life row's zero wins the minimum); the opt-in Any combine makes the Conditions row an independent
    // driver, so the same sterile cell decays. The world chooses which regime through data, so the abiotic
    // Conditions proxy cannot silently swallow the emergent Life signal.
    use civsim_sim::decompose::CombineMode;
    let both_all = || {
        let mut d = DecomposerDriverRegistry::new();
        d.push(DecomposerDriver::conditions(
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            Fixed::from_int(10),
        ));
        d.push(DecomposerDriver::life(Fixed::ONE));
        d
    };
    let mut sterile_both_all = build(300, Some(both_all()), None);
    sterile_both_all.step();
    assert_eq!(
        flesh(&sterile_both_all),
        flesh0,
        "All combine: a sterile warm-wet cell preserves even beside a favorable Conditions row (Life gates it)"
    );
    let mut sterile_both_any = build(300, Some(both_all().with_combine(CombineMode::Any)), None);
    sterile_both_any.step();
    assert!(
        flesh(&sterile_both_any) < flesh0,
        "Any combine: the same sterile cell decays through the Conditions row, the world's explicit choice"
    );
}

#[test]
fn a_spent_hull_trace_weathers_slowly_when_warm_and_is_preserved_when_frozen() {
    // The lifetime/demography keystone, pillar 2, physical-trace persistence, trace slice D (the WEATHERING
    // half): a spent_hull left in the world weathers by the SAME matter cycle carrion does, because it carries
    // the same physics gates (a mineral-ash fraction and a freezing decomposition barrier), keyed off its own
    // composition, not a tag (Principles 8, 11). This is the trace's falsifiability by PHYSICS: an unsupported
    // trace fades, so an abandoned site loses its marks (and the reward-attraction pull that rides them) rather
    // than persisting forever. The hull is RECALCITRANT (lignified shell), so it weathers SLOWLY: it outlives a
    // being's lifespan, which is what lets a technique's mark persist past its maker, then fades if unvisited.
    // The weathering rate is RESERVED, surfaced with its basis (the recalcitrant-lignin decomposition timescale
    // of a nut shell, far slower than soft tissue), never fabricated; a dev value proves the mechanism here.
    use civsim_sim::material::{MaterialField, MatterCycleCalib};

    let (w, h) = (8, 8);
    let cell = Coord3::ground(2, 2);
    let hull0 = Fixed::from_int(10);
    // RESERVED (the weathering rate): the per-tick decomposition fraction of a lignified spent hull. Basis: the
    // recalcitrant-lignin decomposition timescale (nut-shell material persists for years to decades in soil,
    // far slower than the soft-tissue carrion the global fixture rate models), surfaced for the owner. A dev
    // value here (slow enough to outlive a being, fast enough to show weathering in the test window).
    let hull_weathering_rate = Fixed::from_ratio(1, 50);
    let matter_cycle = MatterCycleCalib {
        decomposition_rate: hull_weathering_rate,
        fertility_scale: Fixed::from_ratio(1, 1000),
    };

    let build = |field_temp: i32| -> Runner {
        let hreg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            hreg,
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0xDECA,
        );
        let mut field = MaterialField::new();
        field.deposit(cell, "spent_hull", hull0);
        emb.set_material(field);
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        let mut r = Runner::with_embodiment(
            uniform_field(w, h, Fixed::from_int(field_temp)),
            calib(),
            emb,
        );
        r.set_matter_cycle(matter_cycle);
        r
    };

    let hull = |r: &Runner| -> Fixed {
        r.embodiment()
            .unwrap()
            .material()
            .volume(cell, "spent_hull")
    };

    // A warm cell (300 K, above the hull's 273 K decomposition barrier): the hull weathers, its mass moving
    // into the decomposed sink.
    let mut warm = build(300);
    warm.step();
    assert!(hull(&warm) < hull0, "a warm spent hull weathers");
    assert!(
        warm.embodiment().unwrap().decomposed_mass() > Fixed::ZERO,
        "the weathered mass moved into the sink"
    );
    // But SLOWLY: after a single step the hull is barely touched (it is recalcitrant), so it durably persists,
    // which is what lets a technique's mark outlive its maker.
    assert!(
        hull(&warm) > hull0.checked_div(Fixed::from_int(2)).unwrap(),
        "the recalcitrant hull weathers slowly: most of it survives a step"
    );
    // Over many ticks it does fade, so an unvisited trace does not persist forever (falsifiability by physics),
    // yet is still present after a span that outlives a being.
    for _ in 0..30 {
        warm.step();
    }
    assert!(hull(&warm) < hull0, "over many ticks the hull fades");
    assert!(
        hull(&warm) > Fixed::ZERO,
        "but it is still present after a span that outlives a being (durable, not instant)"
    );

    // A frozen cell (250 K, below the barrier): the hull is preserved and nothing weathers, exactly as a
    // frozen carcass is preserved.
    let mut frozen = build(250);
    for _ in 0..10 {
        frozen.step();
    }
    assert_eq!(hull(&frozen), hull0, "a frozen spent hull is preserved");
    assert_eq!(
        frozen.embodiment().unwrap().decomposed_mass(),
        Fixed::ZERO,
        "nothing weathered in the cold"
    );
}

#[test]
fn a_roof_of_insulating_matter_shelters_a_being_from_a_harsh_field() {
    // Material-substrate arc item 7, SHELTER: a being whose cell is enclosed by insulating matter (a roof of
    // oak in the air cells above it) is buffered from a harsh field, its body temperature holding nearer its
    // warm start while an identical exposed being tracks the cold field. The buffering is the enclosing
    // matter's own thermal resistance (its volume over its conductivity) attenuating the body-to-field
    // coupling, no shelter tag: it keys off the substance's conductivity (Principles 8, 9, 11). Building the
    // roof is the deferred emergent technique; this proves the primitive that makes a built roof matter.
    use civsim_sim::material::{MaterialField, ShelterCalib};

    let (w, h) = (8, 8);
    let sheltered = Coord3::ground(2, 2);
    let exposed = Coord3::ground(5, 5);

    let build = || -> Runner {
        let reg = energy_thermal_registry();
        let mut emb = Embodiment::new(
            reg,
            AffordanceRegistry::dev_default(),
            LocomotionParams::dev_default(),
            0,
            0x50F7,
        );
        // A roof of oak (a low-conductivity insulator) in the air cells above the sheltered cell; nothing
        // above the exposed cell.
        let mut field = MaterialField::new();
        for z in 1..=3 {
            field.deposit(Coord3::new(2, 2, z), "oak", Fixed::from_int(10));
        }
        emb.set_material(field);
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        // A harsh cold field (280 K, below the beings' 310 K start), so an exposed body loses heat to it.
        let mut r =
            Runner::with_embodiment(uniform_field(w, h, Fixed::from_int(280)), calib(), emb);
        r.set_shelter(ShelterCalib::dev_fixture());
        // Two beings starting warm (310 K), one under the roof, one in the open.
        r.place_being(StableId(1), sheltered, Fixed::from_int(310));
        r.place_being(StableId(2), exposed, Fixed::from_int(310));
        r
    };

    let mut r = build();
    for _ in 0..15 {
        r.step();
    }
    let warm = r.body_temp(StableId(1)).unwrap();
    let cold = r.body_temp(StableId(2)).unwrap();
    assert!(
        warm > cold,
        "the sheltered being held more of its warmth than the exposed one: {warm:?} vs {cold:?}"
    );
    assert!(
        warm < Fixed::from_int(310) && cold < Fixed::from_int(310),
        "both beings lost some heat to the cold field (shelter slows the loss, it does not stop it)"
    );
    assert!(
        cold < Fixed::from_int(285),
        "the exposed being cooled close to the field temperature"
    );

    // The scheduled tick order attenuates the exchange identically to the pinned order.
    let mut scheduled = build();
    for _ in 0..15 {
        scheduled.step_scheduled(&[]);
    }
    assert_eq!(
        scheduled.body_temp(StableId(1)).unwrap(),
        warm,
        "the scheduled order shelters the being bit-identically to the pinned order"
    );
}

#[test]
fn a_being_builds_its_own_roof_overhead_and_that_self_built_roof_shelters_it() {
    // Material-substrate arc item 7, the OVERHEAD-DEPOSIT TECHNIQUE (the deferred emergent build the shelter
    // primitive was waiting for): a being sets the insulating matter it carries down into the cell directly
    // ABOVE it, so the roof it raises is one it CHOSE to build (the SHELTER affordance won its decision), and
    // that self-built roof then attenuates its own body-to-field thermal exchange (item 7 slice A reads the
    // overhead matter). So shelter EMERGES from need plus the deposit affordance plus carried matter, no
    // shelter verb: the being that builds holds its warmth, the identical being carrying the same oak but not
    // building stays exposed and cools. A blank founder holding the same load never builds.
    use civsim_sim::material::{ShelterCalib, SubstanceMix};

    let (w, h) = (8, 8);
    let cell = Coord3::ground(2, 2);
    let overhead = Coord3::new(2, 2, 1);
    let load = Fixed::from_int(10);
    let (organs, fat) = energy_registry();
    let reg = energy_thermal_registry();

    let build = |shelter_weight: bool| -> Runner {
        let mut emb = Embodiment::new(
            reg.clone(),
            AffordanceRegistry::dev_builder(),
            LocomotionParams::dev_default(),
            0,
            0x0F00,
        );
        // Outputs: move [act,dx,dy] 0..2, ingest [act] 3, shelter [act] 4.
        assert_eq!(
            emb.layout().n_out(),
            5,
            "builder layout: move(3) + ingest(1) + shelter(1)"
        );
        let n_in = emb.layout().n_in();
        let controller = if shelter_weight {
            let mut wts = vec![Fixed::ZERO; emb.layout().weight_count()];
            wts[4 * n_in + (n_in - 1)] = Fixed::ONE; // bias -> shelter activation
            Controller::from_weights(n_in, emb.layout().n_out(), 0, wts)
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
        // The being carries a load of oak (a low-conductivity insulator) it could raise as a roof.
        let mut carried = SubstanceMix::new();
        carried.add("oak", load);
        walker.carried = carried;
        emb.add(walker, band(310));
        emb.set_material(civsim_sim::material::MaterialField::new());
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        // A harsh cold field (280 K, below the being's 310 K start), so an exposed body loses heat to it.
        let mut r =
            Runner::with_embodiment(uniform_field(w, h, Fixed::from_int(280)), calib(), emb);
        r.set_shelter(ShelterCalib::dev_fixture());
        r
    };

    let carried_now =
        |r: &Runner| -> Fixed { r.embodiment().unwrap().walkers()[0].carried.total_volume() };
    let roof = |r: &Runner| -> Fixed { r.embodiment().unwrap().material().volume(overhead, "oak") };

    // The deciding being sets its load overhead: the cell above it gains the oak (a roof), and it carries
    // nothing more. The build happened only through its evolved SHELTER decision.
    let mut builder = build(true);
    builder.step();
    assert_eq!(
        roof(&builder),
        load,
        "the being raised a roof of oak overhead"
    );
    assert_eq!(
        carried_now(&builder),
        Fixed::ZERO,
        "the being set its whole load overhead"
    );

    // The blank founder never builds: it holds its load and no roof rises.
    let mut founder = build(false);
    founder.step();
    assert_eq!(roof(&founder), Fixed::ZERO, "no roof rose over the founder");
    assert_eq!(
        carried_now(&founder),
        load,
        "the blank founder holds its load"
    );

    // The self-built roof SHELTERS the builder: over the cold field it holds more of its warmth than the
    // identical founder that carried the same oak but never raised it, which stays exposed and cools. The
    // shelter is the physics consequence of the matter the being chose to place overhead.
    for _ in 0..15 {
        builder.step();
        founder.step();
    }
    let sheltered = builder.body_temp(StableId(1)).unwrap();
    let exposed = founder.body_temp(StableId(1)).unwrap();
    assert!(
        sheltered > exposed,
        "the being under the roof it built held more warmth than the one that did not build: {sheltered:?} vs {exposed:?}"
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
            draw_set: Vec::new(),
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
            MediumField::uniform(
                6,
                6,
                respirable,
                Fixed::ZERO,
                Fixed::from_int(10),
                Fixed::ZERO,
            ),
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
            draw_set: Vec::new(),
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
        vec![Fixed::from_int(10); (w * h) as usize],
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
value = "density=1000,respirable_content=0.3,conductivity=0.606,specific_heat=4186,convective_coefficient=10"
unit = "medium_profile"
source = "test"
[[reserved]]
id = "medium.air"
basis = "fixture"
status = "set"
value = "density=1.2,respirable_content=9,conductivity=0.0262,specific_heat=1005,convective_coefficient=10"
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
