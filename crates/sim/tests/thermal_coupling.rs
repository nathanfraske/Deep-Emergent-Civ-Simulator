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

//! The field-to-behaviour coupling on the evolved-controller substrate (design Part 5.4, Part 8.4,
//! Part 20; R-BEHAVIOR-EVOLVE, R-EVOLVE-STEER; Principles 3, 8, 9, 10, 11).
//!
//! The canonical runner drives a temperature [`Field`] whose physics is authored (Principle 9). This
//! harness proves the increment that turns that physics into behaviour without authoring the
//! behaviour: the pure comfort-band map ([`comfort_fraction`]) turns a being's field-driven core
//! temperature into a temperature homeostatic reserve, the being's evolved controller reads the
//! reserve and moves or rests, and the beings' new coordinates re-sync the located index so the next
//! tick's thermal exchange reads where they moved. Physics in, behaviour out.
//!
//! What is proven. The comfort map peaks at the set point and is even in the deviation, so it authors
//! no direction. A comfortable being rests and an uncomfortable one moves, so temperature drives
//! behaviour through physiology, not a rule. The thermotaxis fixture controller authors no heading
//! (this first increment gives it no directional thermal percept), so when it moves it explores, an
//! undirected heading, and directed thermotaxis is left to emerge from moving-while-uncomfortable
//! under survival. The whole coupled tick replays bit for bit and is seed-sensitive, and a being
//! carried a full half-band past its set point dies of cold. Finally, the emergent benefit is real and
//! favours no compass direction: a population carrying the thermotaxis controller ends warmer than an
//! identical population of idle beings, and the advantage is the same whether the warmth lies to the
//! west or the east (the anti-steering discipline of R-EVOLVE-STEER, mirrored from the scorer fix).
//!
//! The controllers here are labelled development fixtures standing in for what homeostatic-survival
//! selection produces (R-BEHAVIOR-EVOLVE); the comfort band's set point and half-range are labelled
//! stand-ins for the reserved per-race physiology (Part 20). Neither is owner canon.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::{Fixed, StableId};
use civsim_sim::anatomy::{BodyPlan, Part, Temperament};
use civsim_sim::controller::{Controller, ControllerLayout};
use civsim_sim::homeostasis::{
    AffordanceRegistry, Homeostasis, HomeostaticAxisId, HomeostaticRegistry, MOVE, TEMPERATURE,
};
use civsim_sim::locomotion::{LocomotionParams, Walker};
use civsim_sim::runner::{comfort_fraction, BeingThermal, Embodiment, Field, FieldCalib, Runner};
use civsim_world::Coord3;

const SETPOINT: i32 = 37;
const HALF_BAND: i32 = 8;

/// A reserved thermal band fixture: a viable core-temperature band around the set point, with the
/// being's spawn core temperature. Labelled fixture values, not owner canon.
fn band(initial_temp: Fixed) -> BeingThermal {
    BeingThermal {
        setpoint: Fixed::from_int(SETPOINT),
        half_band: Fixed::from_int(HALF_BAND),
        initial_temp,
    }
}

/// Labelled field calibrations, within the documented bounds (diffusion below 0.25, relaxation and
/// exchange in [0, 1]). A moderate exchange so a being's core temperature tracks its cell over a few
/// ticks. A fixture, never owner canon.
fn calib() -> FieldCalib {
    FieldCalib {
        diffusion: Fixed::from_ratio(1, 16),
        relaxation: Fixed::from_ratio(1, 4),
        exchange: Fixed::from_ratio(1, 2),
    }
}

/// A uniform field at one temperature: a fixed point of the diffusion-and-relaxation step (every cell
/// equal, so the laplacian and the relaxation are both zero), so a being's cell temperature holds.
fn uniform_field(w: i32, h: i32, temp: Fixed) -> Field {
    Field::new(w, h, vec![temp; (w * h) as usize])
}

/// A field whose left `split` columns bear `warm` and the rest `cold` (or the mirror, warm on the
/// right). A labelled fixture geometry: the warmth is a physical region, not a route.
fn split_field(w: i32, h: i32, split: i32, warm: Fixed, cold: Fixed, warm_left: bool) -> Field {
    let baseline: Vec<Fixed> = (0..(w * h))
        .map(|i| {
            let x = i % w;
            let is_left = x < split;
            if is_left == warm_left {
                warm
            } else {
                cold
            }
        })
        .collect();
    Field::new(w, h, baseline)
}

/// A plain mobile body (it has legs, so it can walk); only its controller varies between beings.
fn mobile_body() -> BodyPlan {
    BodyPlan {
        body_mass: Fixed::from_ratio(1, 2),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons: vec![],
        covering: Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        },
        senses: vec![],
        locomotion: vec![1],
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(3, 4),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    }
}

/// A thermotaxis fixture controller: move while uncomfortable, rest once comfortable enough. Its move
/// activation is `3 - 4 * comfort_level` (clamped), so it rests when the temperature comfort level is
/// at or above three-quarters and moves below it. Crucially its directional MOVE outputs are zero: it
/// authors NO heading, so when it moves it explores (an undirected, seed-keyed heading), and directed
/// thermotaxis is left to emerge from moving-while-uncomfortable under survival, never wired in. A
/// labelled dev fixture standing in for what homeostatic-survival selection produces (R-BEHAVIOR-EVOLVE).
fn thermotaxis(l: &ControllerLayout) -> Controller {
    let n_in = l.n_in();
    let bias = n_in - 1;
    let mut w = vec![Fixed::ZERO; l.weight_count()];
    w[bias] = Fixed::from_int(3); // move_act: a standing urge to move,
    w[0] = Fixed::from_int(-4); // suppressed as the temperature comfort level (index 0) rises.
                                // move_dx (row 1), move_dy (row 2), ingest_act (row 3): all zero.
    Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
}

/// A fresh empty embodiment over the temperature-only development physiology and the standard
/// affordances, a reaction-norm controller layout (hidden 0), and a locomotion seed.
fn embodiment(seed: u64) -> Embodiment {
    Embodiment::new(
        HomeostaticRegistry::dev_thermal(),
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        0,
        seed,
    )
}

/// A walker with the given controller at a tile, its physiology sized from the standard body.
fn walker(id: u64, tile: Coord3, controller: Controller) -> Walker {
    Walker::new(
        StableId(id),
        tile,
        mobile_body(),
        Homeostasis::new(&HomeostaticRegistry::dev_thermal(), Fixed::from_ratio(1, 2)),
        controller,
    )
}

#[test]
fn the_comfort_band_map_peaks_at_the_set_point_and_is_even() {
    // The map authors no direction: full comfort at the set point, and a temperature the same distance
    // above or below reads the same comfort (it is even in the deviation). This is the anti-steering
    // property of the physics-to-physiology map, bit-exact.
    let b = band(Fixed::from_int(SETPOINT));
    assert_eq!(
        comfort_fraction(b.setpoint, &b),
        Fixed::ONE,
        "full comfort at the set point"
    );
    for d in [1, 2, 3, 5, 7] {
        let hi = comfort_fraction(b.setpoint + Fixed::from_int(d), &b);
        let lo = comfort_fraction(b.setpoint - Fixed::from_int(d), &b);
        assert_eq!(
            hi, lo,
            "a deviation of {d} above or below the set point reads the same comfort (even, no direction)"
        );
    }
    assert_eq!(
        comfort_fraction(b.setpoint + Fixed::from_int(HALF_BAND), &b),
        Fixed::ZERO,
        "zero comfort a half-band above"
    );
    assert_eq!(
        comfort_fraction(b.setpoint - Fixed::from_int(HALF_BAND), &b),
        Fixed::ZERO,
        "and a half-band below"
    );
    assert_eq!(
        comfort_fraction(b.setpoint + Fixed::from_int(50), &b),
        Fixed::ZERO,
        "clamped to zero beyond the band"
    );
}

#[test]
fn the_thermotaxis_controller_authors_no_heading() {
    // Even offered a temperature direction and made uncomfortable, the fixture controller emits no
    // heading: the increment gives it no directional thermal percept, so it explores rather than
    // beelining. The bit-exact half of the anti-steering guarantee (with the even comfort map above).
    let emb = embodiment(0);
    let l = emb.layout().clone();
    let c = thermotaxis(&l);
    let mut homeo = Homeostasis::new(&HomeostaticRegistry::dev_thermal(), Fixed::from_ratio(1, 2));
    homeo.set_level(TEMPERATURE, Fixed::from_ratio(1, 4)); // uncomfortable, so it wants to move
    let here: BTreeSet<HomeostaticAxisId> = BTreeSet::new();
    let mut dirs = BTreeMap::new();
    dirs.insert(TEMPERATURE, (Fixed::ONE, Fixed::ZERO)); // a direction is on offer...
    let input = l.build_input(&homeo, &here, &dirs);
    let (out, _) = c.evaluate(&input, &[]);
    let afforded = AffordanceRegistry::dev_default().afforded(&mobile_body());
    let d = l
        .decide(&out, &afforded)
        .expect("the body affords movement");
    assert_eq!(d.affordance, MOVE, "uncomfortable, it chooses to move");
    assert_eq!(
        d.heading.expect("move is directional"),
        (Fixed::ZERO, Fixed::ZERO),
        "but it authors no heading: it explores, it does not beeline toward the offered direction"
    );
}

#[test]
fn a_comfortable_being_rests() {
    let mut emb = embodiment(0xC0);
    let l = emb.layout().clone();
    let start = Coord3::ground(4, 3);
    emb.add(
        walker(1, start, thermotaxis(&l)),
        band(Fixed::from_int(SETPOINT)),
    );
    // A uniform field at the set point: the being stays comfortable, so it never moves.
    let mut runner = Runner::with_embodiment(
        uniform_field(10, 8, Fixed::from_int(SETPOINT)),
        calib(),
        emb,
    );
    for _ in 0..40 {
        runner.step();
    }
    let w = &runner.embodiment().unwrap().walkers()[0];
    assert!(w.alive, "the comfortable being lives");
    assert_eq!(
        w.coord(),
        start,
        "and rests: at the set point it never moves"
    );
}

#[test]
fn an_uncomfortable_being_moves() {
    let mut emb = embodiment(0xC1);
    let l = emb.layout().clone();
    let start = Coord3::ground(4, 3);
    emb.add(
        walker(1, start, thermotaxis(&l)),
        band(Fixed::from_int(SETPOINT)),
    );
    // A uniform cold field, inside the survivable band but far from comfort: the being cools, grows
    // uncomfortable, and moves (an undirected explore, since it has no directional percept).
    let mut runner =
        Runner::with_embodiment(uniform_field(10, 8, Fixed::from_int(30)), calib(), emb);
    let mut moved = false;
    for _ in 0..40 {
        runner.step();
        if runner.embodiment().unwrap().walkers()[0].coord() != start {
            moved = true;
            break;
        }
    }
    let w = &runner.embodiment().unwrap().walkers()[0];
    assert!(w.alive, "cold but within the band, it survives");
    assert!(moved, "uncomfortable, it moves");
}

/// A coupled run of four scattered beings in a split field, returning the composite state hash after
/// each tick (the canonical fingerprint of the coupled run).
fn coupled_trace(seed: u64, ticks: u64) -> Vec<u128> {
    let mut emb = embodiment(seed);
    let l = emb.layout().clone();
    // Start the beings in the cold half (x >= 8), so they grow uncomfortable and explore: the
    // exploration headings key on the seed, so the run is a non-trivial seed-sensitive trace.
    for k in 0..4u64 {
        let start = Coord3::ground(9 + 2 * (k as i32), 1 + (k as i32));
        emb.add(
            walker(1 + k, start, thermotaxis(&l)),
            band(Fixed::from_int(SETPOINT)),
        );
    }
    let field = split_field(
        16,
        8,
        8,
        Fixed::from_int(SETPOINT),
        Fixed::from_int(31),
        true,
    );
    let mut runner = Runner::with_embodiment(field, calib(), emb);
    let mut trace = Vec::with_capacity(ticks as usize);
    for _ in 0..ticks {
        runner.step();
        trace.push(runner.state_hash());
    }
    trace
}

#[test]
fn the_coupled_runner_replays_bit_for_bit() {
    // The coupled tick (field step, thermal exchange, comfort-band map, evolved-controller locomotion,
    // index re-sync) is fully deterministic: two runs from one seed produce the same composite state
    // hash at every tick.
    let a = coupled_trace(0xF00D, 80);
    let b = coupled_trace(0xF00D, 80);
    assert_eq!(a.len(), 80);
    assert_eq!(a, b, "the coupled runner did not replay bit for bit");
}

#[test]
fn the_coupled_runner_diverges_on_a_different_seed() {
    // The trace is seed-sensitive (the exploration headings key on the seed), so the bit-identity above
    // is a real reproduction of a non-trivial run rather than a constant.
    let a = coupled_trace(1, 60);
    let b = coupled_trace(2, 60);
    assert_ne!(
        a, b,
        "a different locomotion seed should not produce the same run"
    );
}

#[test]
fn the_embodiment_state_is_folded_into_the_hash() {
    // A coupled runner folds each being's position, reserves, and controller state after the field; a
    // field-only runner with the same field and the same being temperatures placed does not. So at
    // tick zero (identical field, identical body temperatures) the two hashes differ by exactly the
    // embodiment fold, and a field-only runner's hash is unchanged by this composition.
    let temp = Fixed::from_int(SETPOINT);
    let coord = Coord3::ground(3, 2);

    let mut emb = embodiment(0xB0);
    let l = emb.layout().clone();
    emb.add(walker(1, coord, thermotaxis(&l)), band(temp));
    let coupled = Runner::with_embodiment(uniform_field(10, 8, temp), calib(), emb);

    let mut field_only = Runner::new(uniform_field(10, 8, temp), calib());
    field_only.place_being(StableId(1), coord, temp);

    assert_ne!(
        coupled.state_hash(),
        field_only.state_hash(),
        "the coupled runner's hash carries the embodiment fold the field-only runner omits"
    );
}

#[test]
fn a_being_carried_past_the_band_dies() {
    let mut emb = embodiment(0xDE);
    let l = emb.layout().clone();
    emb.add(
        walker(1, Coord3::ground(4, 3), thermotaxis(&l)),
        band(Fixed::from_int(SETPOINT)),
    );
    // A uniformly lethal-cold field a full half-band and more below the set point: comfort falls to
    // zero and the being dies of cold, wherever its undirected search takes it.
    let lethal = Fixed::from_int(SETPOINT - HALF_BAND - 4);
    let mut runner = Runner::with_embodiment(uniform_field(10, 8, lethal), calib(), emb);
    let mut died = false;
    for _ in 0..80 {
        runner.step();
        if !runner.embodiment().unwrap().walkers()[0].alive {
            died = true;
            break;
        }
    }
    assert!(
        died,
        "carried a full half-band past its set point, the being dies of cold"
    );
}

/// A population of beings on a two-row grid across a split field, all with the same controller kind,
/// run for `ticks`, returning the population's mean temperature comfort at the end. With the idle
/// (blank) controller a being never moves; with the thermotaxis controller it moves while uncomfortable
/// and rests once warm. The grid, the ids, and the seed are identical across calls, so the ONLY thing
/// that varies between the warm-left and warm-right runs is the field orientation.
fn mean_final_comfort(warm_left: bool, blank: bool, seed: u64, ticks: u64) -> Fixed {
    let (w, h) = (24i32, 6i32);
    let field = split_field(
        w,
        h,
        12,
        Fixed::from_int(SETPOINT),
        Fixed::from_int(31),
        warm_left,
    );
    let mut emb = embodiment(seed);
    let l = emb.layout().clone();
    let ctrl = if blank {
        Controller::zeros(&l)
    } else {
        thermotaxis(&l)
    };
    let mut id = 1u64;
    // Columns closed under the field's mirror x -> 23 - x (a width-24 field split at 12), so the warm-
    // left and warm-right setups are exact geometric mirrors: the idle baselines match to the bit and
    // any advantage gap is the per-being exploration RNG (which does not mirror), not the map.
    for gx in [2, 5, 8, 11, 12, 15, 18, 21] {
        for gy in [1, 3] {
            emb.add(
                walker(id, Coord3::ground(gx, gy), ctrl.clone()),
                band(Fixed::from_int(SETPOINT)),
            );
            id += 1;
        }
    }
    let mut runner = Runner::with_embodiment(field, calib(), emb);
    for _ in 0..ticks {
        runner.step();
    }
    let ws = runner.embodiment().unwrap().walkers();
    let sum = Fixed::saturating_sum(ws.iter().map(|w| w.homeostasis.level(TEMPERATURE)));
    sum.div(Fixed::from_int(ws.len() as i32))
}

#[test]
fn thermotaxis_emerges_and_favours_no_direction() {
    // The emergent behaviour, and the R-EVOLVE-STEER anti-steering discipline carried onto the coupled
    // runner. From undirected movement gated on discomfort, a population carrying the thermotaxis
    // controller ends warmer than an identical idle population: net movement toward warmth emerges,
    // though no heading is ever authored. And the advantage is the same whether the warmth lies west or
    // east, so the coupling privileges no compass direction (the same balance the scorer fix restored).
    let ticks = 400;
    let seed = 0x7EA1;
    let left_active = mean_final_comfort(true, false, seed, ticks);
    let left_idle = mean_final_comfort(true, true, seed, ticks);
    let right_active = mean_final_comfort(false, false, seed, ticks);
    let right_idle = mean_final_comfort(false, true, seed, ticks);

    let adv_left = left_active - left_idle;
    let adv_right = right_active - right_idle;
    assert!(
        adv_left > Fixed::ZERO,
        "warmth to the west: the movers end warmer than the idle ({} vs {})",
        left_active.to_f64_lossy(),
        left_idle.to_f64_lossy()
    );
    assert!(
        adv_right > Fixed::ZERO,
        "warmth to the east: the movers end warmer than the idle ({} vs {})",
        right_active.to_f64_lossy(),
        right_idle.to_f64_lossy()
    );
    // Anti-steering: the two advantages match closely, so no direction is favoured. The idle baselines
    // are bit-identical (the setups are exact mirrors), so the only gap is the per-being exploration
    // RNG, which does not mirror under a field flip; a directional bias like the retired eastward
    // scorer would blow far past this bound.
    let diff = (adv_left - adv_right).abs();
    assert!(
        diff < Fixed::from_ratio(1, 10),
        "the warmth-west and warmth-east advantages match ({} vs {}); no direction is favoured",
        adv_left.to_f64_lossy(),
        adv_right.to_f64_lossy()
    );
}
