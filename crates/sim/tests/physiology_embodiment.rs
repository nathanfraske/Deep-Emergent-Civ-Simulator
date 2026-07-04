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
use civsim_sim::medium::RESPIRATORY_SURFACE;
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
    // irrelevant here (no respiration axis).
    emb.set_physiology(EmbodiedPhysiology::dev_fixture(organs, Fixed::ONE));

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
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(organs, Fixed::ONE));
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
        emb.set_physiology(EmbodiedPhysiology::dev_fixture(organs.clone(), respirable));
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
fn embodied_physiology_reads_a_set_manifest_and_fails_loud_when_reserved() {
    // The canonical sourcing: EmbodiedPhysiology::from_manifest reads the metabolic anchors, the medium's
    // respirable content, the reserved transfer coefficient, and the base tick from a set manifest, and a
    // reserved input refuses to fabricate a number (Principle 11).
    use civsim_sim::calibration::{CalibrationError, CalibrationManifest};
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
id = "medium.air"
basis = "fixture"
status = "set"
value = "density=1.2,respirable_content=9,conductivity=0.0262,specific_heat=1005"
unit = "medium_profile"
source = "test"
"#;
    let m = CalibrationManifest::from_toml_str(set).unwrap();
    let organs = BodyPlanRegistry::dev_default();
    // The set manifest threads the anchors, the medium respirable content, the transfer coefficient, and
    // the base tick into a usable physiology.
    let _phys = EmbodiedPhysiology::from_manifest(&m, organs.clone(), "medium.air").unwrap();

    // A reserved transfer coefficient refuses to build.
    let reserved = set.replace(
        "id = \"metabolism.respiration_transfer_coefficient\"\nbasis = \"fixture\"\nstatus = \"set\"\nvalue = \"0.5\"",
        "id = \"metabolism.respiration_transfer_coefficient\"\nbasis = \"fixture\"\nstatus = \"reserved\"\nvalue = \"\"",
    );
    let mr = CalibrationManifest::from_toml_str(&reserved).unwrap();
    assert_eq!(
        EmbodiedPhysiology::from_manifest(&mr, organs, "medium.air").unwrap_err(),
        CalibrationError::Reserved("metabolism.respiration_transfer_coefficient".to_string()),
    );
}
