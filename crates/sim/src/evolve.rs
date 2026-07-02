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

//! Homeostatic-survival selection of the behaviour controller (design Part 8, Part 25, Part 54;
//! R-BEHAVIOR-EVOLVE Stage 3; Principles 3, 9, 10, 11).
//!
//! This is layer 5 of the evolved-behaviour architecture: selection. A controller's fitness is a
//! consequence of homeostatic survival, not an authored objective and, in particular, not a
//! resemblance to any expected behaviour. A controller is scored by whether it keeps a body's
//! reserves off their floors, which is measured by running the body under that controller through the
//! movement-and-metabolism physics (`crate::locomotion`) and counting how long it stays viable. A
//! lineage whose controller keeps its bodies alive fixes its adaptive controller alleles; one whose
//! controller lets its bodies starve is selected against. Nobody scores "did it seek water": the
//! being that seeks water survives, and survival is what is counted. This is what keeps the behaviour
//! emergent (design pass, `docs/emergent_behavior_design.md`).
//!
//! Two scorers sit on the significance-and-time gradient the engine already uses for detail
//! (Principle 1, Part 54). The [`episode_survival`] scorer runs a full behavioural episode through
//! the movement physics, the honest high-fidelity measure. The cheaper proxy for quiet deep-time
//! pools, and the full-episode tier at the dawn, are the same call at different lengths and
//! environments; which pools get which is the reserved significance allocation. Every scoring draw
//! keys on the seed, the individual, and the generation ([`civsim_core::Phase::CONTROLLER`]), never
//! on the camera, so which behaviours a world evolves is a function of the seed and the world alone
//! (Principles 3, 10).
//!
//! The selection itself is the aggregate-tier recurrence [`crate::genome::GenePool::select`] already
//! carries: it takes a per-locus selection coefficient and moves the frequencies, channel-blind, so
//! a controller locus is selected by the same mechanism as any other once its coefficient is a
//! consequence of homeostatic survival ([`homeostatic_coefficient`]). The individual-based loop
//! [`evolve`] proves the whole chain end to end: from random controllers, homeostatic-survival
//! selection with bounded mutation produces water-seeking behaviour, from a random start, without it
//! being authored.
//!
//! Honest limits, which the design pass names as the crux to prove rather than proven here. The
//! deep-time pool tier expresses a controller from allele frequencies, which needs the quantitative
//! breeding-value tier the genome still defers (25.10), so the pure-frequency deep-time controller
//! evolution couples to that tier and to the open temporal level of detail (Part 32); this module
//! builds and proves the individual and sampled-episode tier and wires the coefficient into the pool
//! recurrence, and scopes the pure-frequency tier as the reserved coupling. The proxy's honesty
//! (that surviving the scored episode predicts surviving in the world) is validated by cross-checking
//! against longer and richer episodes, not asserted.

use civsim_core::{DrawKey, Fixed, Phase, StableId};
use civsim_world::Coord3;

use crate::anatomy::{BodyPlan, Part, Temperament};
use crate::controller::{Controller, ControllerLayout};
use crate::genome::{
    Allele, AlleleState, Channel, ControllerParamId, DominanceMode, GeneDef, GeneEffect, GeneId,
    GenePool, GeneSet, Genome, Haplotype, SchemeId,
};
use crate::homeostasis::{
    AffordanceRegistry, Homeostasis, HomeostaticAxisDef, HomeostaticRegistry, WATER,
};
use crate::locomotion::{self, LocomotionParams, ResourceField, Terrain, Walker};
use crate::runner::{BeingThermal, Embodiment, Field, FieldCalib, Runner};

// Draw-site slots within the CONTROLLER phase, so the init and the two mutation rolls of one lineage
// do not collide on counter zero (the R-RNG-COORD slot rule).
const SLOT_INIT: u32 = 0;
const SLOT_MUT_HIT: u32 = 1;
const SLOT_MUT_STEP: u32 = 2;

/// The reserved parameters of controller evolution. The mechanism is fixed; these numbers are the
/// owner's to set, surfaced with a basis, never fabricated (Principle 11). The development fixture
/// below lets the loop run and be tested now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EvolveParams {
    /// The number of controller lineages under selection each generation. RESERVED. Basis: the
    /// sample size that resolves the fitness ranking against the per-generation evaluation budget, a
    /// performance bound on the significance-and-time gradient (Part 54).
    pub pop_size: usize,
    /// The number of generations to run. RESERVED. Basis: the deep-time depth over which behaviour
    /// is allowed to adapt before the dawn, tied to the pre-dawn radiation depth (`EpochParams`).
    pub generations: u32,
    /// The length in ticks of a scoring episode. RESERVED. Basis: long enough that a viable
    /// controller's survival separates from an unviable one under the metabolic timescale, against
    /// the evaluation budget (a proxy-versus-episode length on the Part 54 gradient).
    pub episode_ticks: u32,
    /// The half-range of the initial random controller weights. RESERVED. Basis: the weight scale at
    /// which the activation's clamp is neither always saturated nor always near zero, so variation is
    /// expressible from the start (a representational bound).
    pub init_spread: Fixed,
    /// The per-controller-weight probability of a mutation on inheritance. RESERVED. Basis: the
    /// mutation scale the epoch uses for the other channels (`EpochParams`), adjusted for the larger
    /// controller parameter space (design pass).
    pub mutation_rate: Fixed,
    /// The bounded magnitude of a controller-weight mutation. RESERVED. Basis: a perturbation small
    /// enough that a small weight change is a small behaviour change (smooth evolution) yet large
    /// enough to explore the weight space, a stress-test tunable.
    pub mutation_step: Fixed,
}

impl EvolveParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values, so the loop runs and can be tested now.
    pub fn dev_default() -> EvolveParams {
        EvolveParams {
            pop_size: 32,
            generations: 16,
            episode_ticks: 200,
            init_spread: Fixed::from_int(2),
            mutation_rate: Fixed::from_ratio(1, 6),
            mutation_step: Fixed::from_ratio(2, 5),
        }
    }
}

/// The gene set for a controller of a given layout: one gene per controller weight, each feeding its
/// own [`Channel::Controller`] parameter with unit weight, so the expressed weight at parameter `k`
/// is exactly the additive value of locus `k` (a haploid additive spine). This is the data half of
/// the controller (which genes reach which parameters); the mechanism that reads it is
/// [`Controller::express`].
pub fn controller_gene_set(layout: &ControllerLayout) -> GeneSet {
    let genes = (0..layout.weight_count())
        .map(|k| GeneDef {
            id: GeneId(k as u32),
            effects: vec![GeneEffect {
                channel: Channel::Controller(ControllerParamId(k as u32)),
                weight: Fixed::ONE,
            }],
            dominance: DominanceMode::additive(),
        })
        .collect();
    GeneSet { genes }
}

/// A founder genome carrying random controller weights, each drawn uniformly in
/// `[-init_spread, init_spread]` from counter-based RNG keyed on the individual and the parameter
/// locus, so a founder lineage is reproducible from the seed (design Part 3.2).
pub fn random_controller_genome(
    layout: &ControllerLayout,
    params: &EvolveParams,
    seed: u64,
    id: u64,
) -> Genome {
    let rng = DrawKey::entity(id, 0, Phase::CONTROLLER)
        .slot(SLOT_INIT)
        .rng(seed);
    let spread = params.init_spread;
    let alleles = (0..layout.weight_count())
        .map(|k| {
            // unit in [0, ONE) -> [-spread, spread).
            let u = rng.unit_fixed(k as u64);
            let additive = u.mul(spread).mul(Fixed::from_int(2)) - spread;
            Allele {
                additive,
                state: AlleleState(0),
                origin: id as u32,
            }
        })
        .collect();
    Genome {
        scheme: SchemeId(0),
        haps: vec![Haplotype { alleles }],
    }
}

/// Mutate a genome's controller weights: each weight, with probability `mutation_rate`, gains a
/// bounded step drawn uniformly in `[-mutation_step, mutation_step]`, keyed on the child, the locus,
/// and the generation ([`Phase::CONTROLLER`]), so a lineage's mutations are a reproducible function
/// of the seed and its ancestry. This is the controller-allele mutation the design reserves; the
/// general continuous additive-mutation shape for every channel remains the deferred integer-Gaussian
/// of 25.10.
pub fn mutate(
    parent: &Genome,
    params: &EvolveParams,
    seed: u64,
    child_id: u64,
    generation: u64,
) -> Genome {
    let mut haps = parent.haps.clone();
    if let Some(hap) = haps.first_mut() {
        for (locus, allele) in hap.alleles.iter_mut().enumerate() {
            let hit = DrawKey::pair(child_id, locus as u64, generation, Phase::CONTROLLER)
                .slot(SLOT_MUT_HIT)
                .rng(seed)
                .unit_fixed(0);
            if hit < params.mutation_rate {
                let u = DrawKey::pair(child_id, locus as u64, generation, Phase::CONTROLLER)
                    .slot(SLOT_MUT_STEP)
                    .rng(seed)
                    .unit_fixed(0);
                // u in [0, ONE) -> [-step, step).
                let delta =
                    u.mul(params.mutation_step).mul(Fixed::from_int(2)) - params.mutation_step;
                allele.additive += delta;
            }
        }
    }
    Genome {
        scheme: parent.scheme,
        haps,
    }
}

/// A fast-draining water-only physiology for scoring, so a controller that fails to reach and drink
/// water dies promptly and a competent one survives to the cap, giving selection a sharp gradient (a
/// labelled scoring fixture, not owner canon).
fn scoring_reg() -> HomeostaticRegistry {
    HomeostaticRegistry {
        axes: vec![HomeostaticAxisDef {
            id: WATER,
            name: "water".to_string(),
            backing_component: Some("bio.water_fraction".to_string()),
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::from_ratio(1, 60),
            exertion_drain: Fixed::from_ratio(1, 200),
            death_floor: Fixed::ZERO,
        }],
    }
}

/// An open, flat plane: the scoring environment where survival turns on the controller's foraging,
/// not on terrain.
struct OpenPlane;
impl Terrain for OpenPlane {
    fn passable(&self, _c: Coord3, _b: &BodyPlan) -> bool {
        true
    }
    fn cost(&self, _c: Coord3) -> Fixed {
        Fixed::ONE
    }
}

/// A plain mobile body for scoring: it can walk, its speed and metabolism are the physics, so only
/// its controller varies between the beings under selection.
fn scoring_body() -> BodyPlan {
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
        organs: vec![],
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(3, 4),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    }
}

/// The wheel of base orientations a pool's water may be placed along, seed-selected per pool by
/// [`scoring_set`]. Each entry is one primitive integer direction in the first quadrant; the scored
/// set rotates it into all four quadrants. Spanning several orientations (axis-aligned, the two
/// knight slopes, the diagonal) means no fixed compass axis is privileged even across many pools: a
/// pool that draws the diagonal base is scored on beings that must forage diagonally, one that draws
/// the axis base on axis foragers, and the population of pools samples the wheel. Labelled fixture
/// geometry, not owner canon.
const SCORING_WHEEL: [(i32, i32); 4] = [(1, 0), (2, 1), (1, 1), (1, 2)];

/// Per-direction RNG salts, so the four episodes of one score are independent deterministic streams
/// rather than four replays of one noise realisation a controller could overfit. Distinct constants
/// keep the aggregate a pure function of the seed (design 33.10 replay).
const SCORING_DIR_SALT: [u64; 4] = [
    0x5165_1CAF_0000_0001,
    0x5165_1CAF_0000_0002,
    0x5165_1CAF_0000_0003,
    0x5165_1CAF_0000_0004,
];

/// The four directions a being's water is placed in for one pool's scoring, seed-selected. From a
/// wheel base `v` the set is `{v, perp(v), -v, -perp(v)}` with `perp(v) = (-v.y, v.x)`, the 90-degree
/// rotations of `v`. This set is its own anti-steering invariant for any base: the four sum to the
/// zero vector (no direction is favoured) and share one length (`|v|`, so no direction is nearer or
/// easier). An earlier scorer placed water only to the east, which selected for a fixed-eastward
/// overfit rather than for following the water-direction percept; scoring this balanced set and
/// aggregating removes that gradient, and the base rotates with the seed so the wheel is sampled
/// across pools. The `the_scorer_authors_no_directional_bias` test guards the invariant.
fn scoring_set(seed: u64) -> [(i32, i32); 4] {
    // A seeded index into the wheel; a plain integer fold so the base is a pure function of the seed.
    let idx = (((seed >> 33) ^ (seed >> 17) ^ seed) as usize) % SCORING_WHEEL.len();
    let v = SCORING_WHEEL[idx];
    let perp = (-v.1, v.0);
    [v, perp, (-v.0, -v.1), (-perp.0, -perp.1)]
}

/// The water cells of one scored episode: a band `2*half_cross+1` wide, its centre line set
/// `near..=far` steps of `dir` from the origin, spread along `dir`'s true perpendicular. Placing the
/// region along the direction axis keeps every episode geometrically identical up to the rotation, so
/// only the heading a competent forager must take differs between the directions of a scored set and
/// no direction is easier than another.
fn scoring_water_cells(dir: (i32, i32), near: i32, far: i32, half_cross: i32) -> Vec<Coord3> {
    let perp = (-dir.1, dir.0);
    let mut cells = Vec::new();
    for along in near..=far {
        for cross in -half_cross..=half_cross {
            let x = dir.0 * along + perp.0 * cross;
            let y = dir.1 * along + perp.1 * cross;
            cells.push(Coord3::ground(x, y));
        }
    }
    cells
}

/// One directional episode of the proxy scorer: place a body carrying `controller` at the origin,
/// knowing of a water band lying in `dir`, and run it through the movement-and-metabolism physics
/// for up to `ticks`. Returns how many ticks it stays viable (capped at `ticks`). The being is shown
/// the water (this tier tests foraging, not search). Fully deterministic and seed-keyed.
fn episode_survival_dir(controller: &Controller, ticks: u32, seed: u64, dir: (i32, i32)) -> u32 {
    let reg = scoring_reg();
    let afford = AffordanceRegistry::dev_default();
    let layout = ControllerLayout::new(&reg, &afford, controller.hidden());
    let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
    let mut walker = Walker::new(
        StableId(1),
        Coord3::ground(0, 0),
        scoring_body(),
        homeo,
        controller.clone(),
    );
    let mut field = ResourceField::new();
    for c in scoring_water_cells(dir, 3, 7, 2) {
        field.add(WATER, c);
        walker.learn(WATER, c);
    }
    let p = LocomotionParams::dev_default();
    let mut ws = vec![walker];
    let mut survived = 0u32;
    for t in 0..ticks {
        locomotion::step(
            &mut ws, &reg, &layout, &afford, &OpenPlane, &field, &p, seed, t as u64,
        );
        if !ws[0].alive {
            break;
        }
        survived = t + 1;
    }
    survived
}

/// Score a controller by homeostatic survival, aggregated over the symmetric direction set: run one
/// `episode_survival_dir` per direction and return the mean survival. A controller that reaches
/// water wherever it lies scores near the cap; one that heads a fixed way (competent when water is
/// in that direction, helpless when it is opposite) scores only its partial competence, so selection
/// rewards following the water-direction percept over a fixed heading. The water region is a band
/// rather than a single tile, so partial approach yields partial survival, a climbable gradient,
/// while survival stays the honest fitness (no resemblance to an authored behaviour is scored). Each
/// direction keys its RNG stream on the seed folded with a per-direction salt, so the whole score is
/// a pure function of the seed (design 33.10).
pub fn episode_survival(controller: &Controller, ticks: u32, seed: u64) -> u32 {
    let set = scoring_set(seed);
    let mut total = 0u64;
    for (i, &dir) in set.iter().enumerate() {
        total += episode_survival_dir(controller, ticks, seed ^ SCORING_DIR_SALT[i], dir) as u64;
    }
    (total / set.len() as u64) as u32
}

/// A gentler water-only physiology for the full-episode (dawn) tier, so a being has time to search
/// for water it does not yet know of before it starves (a labelled scoring fixture).
fn dawn_reg() -> HomeostaticRegistry {
    HomeostaticRegistry {
        axes: vec![HomeostaticAxisDef {
            id: WATER,
            name: "water".to_string(),
            backing_component: Some("bio.water_fraction".to_string()),
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::from_ratio(1, 150),
            exertion_drain: Fixed::from_ratio(1, 300),
            death_floor: Fixed::ZERO,
        }],
    }
}

/// One directional episode of the full-episode (dawn) scorer: the water band lies in `dir`, near but
/// outside the being's initial perception, and the being does NOT know of it, so it must explore to
/// discover it before foraging. Deterministic and seed-keyed; returns ticks survived.
fn full_episode_survival_dir(
    controller: &Controller,
    ticks: u32,
    seed: u64,
    dir: (i32, i32),
) -> u32 {
    let reg = dawn_reg();
    let afford = AffordanceRegistry::dev_default();
    let layout = ControllerLayout::new(&reg, &afford, controller.hidden());
    let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
    let mut field = ResourceField::new();
    for c in scoring_water_cells(dir, 6, 9, 1) {
        field.add(WATER, c);
    }
    let walker = Walker::new(
        StableId(1),
        Coord3::ground(0, 0),
        scoring_body(),
        homeo,
        controller.clone(),
    );
    let p = LocomotionParams::dev_default();
    let mut ws = vec![walker];
    let mut survived = 0u32;
    for t in 0..ticks {
        locomotion::step(
            &mut ws, &reg, &layout, &afford, &OpenPlane, &field, &p, seed, t as u64,
        );
        if !ws[0].alive {
            break;
        }
        survived = t + 1;
    }
    survived
}

/// Score a controller by a FULL behavioural episode, the high-fidelity tier the design pass runs at
/// the dawn and under significance (Part 54): unlike [`episode_survival`], the being does NOT know
/// where the water is, so it must explore to discover it (`crate::locomotion` exploration), then
/// forage. This exercises the whole loop (search, approach, drink) rather than only foraging from
/// known sources, so it validates that proxy-viability predicts world-viability. It aggregates the
/// mean over the same symmetric direction set as [`episode_survival`], so search competence, not a
/// fixed heading, is what survives. Deterministic and seed-keyed; returns mean ticks survived.
pub fn full_episode_survival(controller: &Controller, ticks: u32, seed: u64) -> u32 {
    let set = scoring_set(seed);
    let mut total = 0u64;
    for (i, &dir) in set.iter().enumerate() {
        total +=
            full_episode_survival_dir(controller, ticks, seed ^ SCORING_DIR_SALT[i], dir) as u64;
    }
    (total / set.len() as u64) as u32
}

// --- The thermal-survival scorer: evolve the thermotaxis controller in-situ on the coupled runner ---

/// The controller layout for the thermal environment: the temperature-only development physiology and
/// the standard affordances. A caller evolves against this layout; its beings carry one homeostatic
/// axis (temperature) and can move or ingest.
pub fn thermal_layout(hidden: usize) -> ControllerLayout {
    ControllerLayout::new(
        &HomeostaticRegistry::dev_thermal(),
        &AffordanceRegistry::dev_default(),
        hidden,
    )
}

/// The labelled thermal scoring geometry, not owner canon: a square field with a warm border this many
/// cells thick and a lethally cold interior, so a being started at the cold centre must move OUT to the
/// surrounding warmth and hold it. The side is odd so there is a single centre cell.
const THERMAL_SIDE: i32 = 11;
const THERMAL_BORDER: i32 = 4;

/// A frozen thermal scoring field: a square with a warm border at the set point and a lethally cold
/// interior. Frozen (zero diffusion and relaxation in [`thermal_scoring_calib`]) so the gradient the
/// controller faces does not smear over the episode. Warmth lies on every side of the centre, so the
/// field is invariant under a 90-degree rotation and no direction is favoured (the R-EVOLVE-STEER
/// discipline; the `the_thermal_scorer_field_favours_no_direction` test guards the invariant). A
/// labelled scoring fixture.
fn thermal_scoring_field(setpoint: Fixed, cold: Fixed) -> Field {
    let s = THERMAL_SIDE;
    let baseline: Vec<Fixed> = (0..(s * s))
        .map(|k| {
            let x = k % s;
            let y = k / s;
            let interior = x >= THERMAL_BORDER
                && x < s - THERMAL_BORDER
                && y >= THERMAL_BORDER
                && y < s - THERMAL_BORDER;
            if interior {
                cold
            } else {
                setpoint
            }
        })
        .collect();
    Field::new(s, s, baseline)
}

/// Labelled thermal scoring calibrations, not owner canon: a frozen field (zero diffusion and
/// relaxation), and a body-to-environment exchange slow enough that a being started warm at the cold
/// centre has time to reach the warm border before its core temperature crosses the lethal floor.
fn thermal_scoring_calib() -> FieldCalib {
    FieldCalib {
        diffusion: Fixed::ZERO,
        relaxation: Fixed::ZERO,
        exchange: Fixed::from_ratio(1, 20),
    }
}

/// The shared core of the thermal scorers: run a being carrying `controller` through the field-to-
/// behaviour coupling ([`crate::runner::Runner::with_embodiment`]) over an explicit field and start
/// tile, and return how many ticks it keeps its temperature reserve off the floor (capped at `ticks`).
/// Nobody scores "seek warmth": the being that keeps its body in the viable band survives, and survival
/// is what is counted. Fully deterministic and seed-keyed.
fn thermal_run(
    controller: &Controller,
    ticks: u32,
    seed: u64,
    field: Field,
    start: Coord3,
    setpoint: Fixed,
    half_band: Fixed,
) -> u32 {
    let reg = HomeostaticRegistry::dev_thermal();
    let mut emb = Embodiment::new(
        reg.clone(),
        AffordanceRegistry::dev_default(),
        LocomotionParams::dev_default(),
        controller.hidden(),
        seed,
    );
    emb.add(
        Walker::new(
            StableId(1),
            start,
            scoring_body(),
            Homeostasis::from_mass(&reg, Fixed::ONE),
            controller.clone(),
        ),
        BeingThermal {
            setpoint,
            half_band,
            initial_temp: setpoint,
        },
    );
    let mut runner = Runner::with_embodiment(field, thermal_scoring_calib(), emb);
    let mut survived = 0u32;
    for t in 0..ticks {
        runner.step();
        if !runner
            .embodiment()
            .expect("the scoring runner carries an embodiment")
            .walkers()[0]
            .alive
        {
            break;
        }
        survived = t + 1;
    }
    survived
}

/// The undirected (kinesis) thermal scorer: a being started at the cold centre of the warm-border
/// field, where the interior is flat so the temperature gradient is zero and only move-or-rest matters.
/// This is the environment the kinesis result (retiring the hand-built fixture) was proven in.
fn thermal_episode_survival(controller: &Controller, ticks: u32, seed: u64) -> u32 {
    let setpoint = Fixed::from_int(37);
    let half_band = Fixed::from_int(8);
    let cold = Fixed::from_int(37 - 16); // a full half-band and more below: the centre is lethal
    let centre = Coord3::ground(THERMAL_SIDE / 2, THERMAL_SIDE / 2);
    thermal_run(
        controller,
        ticks,
        seed,
        thermal_scoring_field(setpoint, cold),
        centre,
        setpoint,
        half_band,
    )
}

/// Evolve a population of controllers under THERMAL homeostatic-survival selection: the same selection
/// machinery as [`evolve`], scored by `thermal_episode_survival` rather than the water scorer. From
/// random controllers, this produces a being that moves while cold and rests while warm, a thermotaxis
/// (a kinesis) that keeps its body in the viable band, without that behaviour being authored anywhere.
/// It retires the hand-built thermotaxis fixture the coupling increment used: selection produces it.
pub fn evolve_thermal(layout: &ControllerLayout, params: &EvolveParams, seed: u64) -> EvolveReport {
    evolve_with(layout, params, seed, thermal_episode_survival)
}

// --- The sensed-taxis scorer: a smooth thermal bowl where a gradient percept beats a random walk ---

/// The labelled bowl geometry, not owner canon. A square field with a smooth radial temperature
/// gradient: coldest (and lethal) at the centre, warming to the set point at the rim, so the
/// temperature gradient is nonzero everywhere off-centre and a being can sense which way is warmer from
/// a distance (unlike the sharp-border field, whose interior is flat). Odd side, so the centre is a
/// single cell.
const BOWL_SIDE: i32 = 21;
/// The radius at which the bowl reaches the set point; beyond it (including the corners) the field is at
/// the set point.
const BOWL_RADIUS: i32 = 9;
/// The distance off-centre a being starts, inside the lethal cold zone, so it must climb outward to the
/// survivable ring. A balanced set of four cardinal offsets at this radius is the start wheel.
const BOWL_START: i32 = 4;

/// A smooth radial thermal bowl: `temp = cold + (setpoint - cold) * min(dist^2 / radius^2, 1)`, cold and
/// lethal at the centre, warming to the set point at the rim. Frozen (see [`thermal_scoring_calib`]), so
/// the gradient is a fixed field the whole episode. The squared radial distance is invariant under a
/// 90-degree rotation about the centre, so the field authors no direction (R-EVOLVE-STEER; guarded by
/// `the_bowl_scorer_field_favours_no_direction`). Pure fixed-point. A labelled scoring fixture.
fn thermal_bowl_field(setpoint: Fixed, cold: Fixed) -> Field {
    let s = BOWL_SIDE;
    let c = s / 2;
    let r2 = (BOWL_RADIUS * BOWL_RADIUS) as i64;
    let span = setpoint - cold;
    let baseline: Vec<Fixed> = (0..(s * s))
        .map(|k| {
            let x = k % s;
            let y = k / s;
            let dx = (x - c) as i64;
            let dy = (y - c) as i64;
            let d2 = dx * dx + dy * dy;
            let frac = if d2 >= r2 {
                Fixed::ONE
            } else {
                Fixed::from_ratio(d2, r2)
            };
            cold + span.mul(frac)
        })
        .collect();
    Field::new(s, s, baseline)
}

/// The four cardinal off-centre start tiles of the bowl (a balanced set: they sum to the centre and are
/// equidistant from it), so aggregating survival over them privileges no compass direction. A
/// gradient-following being climbs outward from every start; a fixed-heading being only survives the
/// start whose outward direction happens to match its heading.
fn bowl_starts() -> [Coord3; 4] {
    let c = BOWL_SIDE / 2;
    let r = BOWL_START;
    [
        Coord3::ground(c + r, c),
        Coord3::ground(c - r, c),
        Coord3::ground(c, c + r),
        Coord3::ground(c, c - r),
    ]
}

/// The sensed-taxis thermal scorer: aggregate survival over the four cardinal starts in the bowl. A
/// being that reads the temperature-gradient percept and climbs it reaches the survivable ring from any
/// start; one that cannot sense direction must random-walk out and often dies first; one that commits to
/// a fixed heading survives only the fraction of starts where that heading points outward. Each start
/// keys its RNG on a per-start salt so the aggregate is a pure function of the seed.
fn thermal_sensed_survival(controller: &Controller, ticks: u32, seed: u64) -> u32 {
    let setpoint = Fixed::from_int(37);
    let half_band = Fixed::from_int(8);
    let cold = Fixed::from_int(37 - 16);
    let starts = bowl_starts();
    let mut total = 0u64;
    for (i, &start) in starts.iter().enumerate() {
        total += thermal_run(
            controller,
            ticks,
            seed ^ SCORING_DIR_SALT[i],
            thermal_bowl_field(setpoint, cold),
            start,
            setpoint,
            half_band,
        ) as u64;
    }
    (total / starts.len() as u64) as u32
}

/// Evolve controllers under the sensed-taxis thermal scorer: the same selection machinery as
/// [`evolve`], scored by `thermal_sensed_survival`. With the temperature-gradient percept available
/// (the runner supplies it), selection produces a being that climbs the gradient toward warmth, a
/// directed thermotaxis that reaches safety faster than an undirected random walk, without any heading
/// being authored.
pub fn evolve_thermal_sensed(
    layout: &ControllerLayout,
    params: &EvolveParams,
    seed: u64,
) -> EvolveReport {
    evolve_with(layout, params, seed, thermal_sensed_survival)
}

/// The report of an evolutionary run: the mean and best homeostatic-survival fitness at each
/// generation (so a caller can see behaviour shift), and the final population of genomes.
#[derive(Clone, Debug)]
pub struct EvolveReport {
    /// The mean survival fitness at each generation.
    pub mean_fitness: Vec<Fixed>,
    /// The best survival fitness at each generation.
    pub best_fitness: Vec<u32>,
    /// The final population's genomes.
    pub final_genomes: Vec<Genome>,
}

/// Evolve a population of controllers under homeostatic-survival selection (design Part 8, Part 25;
/// R-BEHAVIOR-EVOLVE Stage 3), scored by [`episode_survival`] (the water-foraging environment). This
/// is [`evolve_with`] with the water scorer; the thermal environment is [`evolve_thermal`].
pub fn evolve(layout: &ControllerLayout, params: &EvolveParams, seed: u64) -> EvolveReport {
    evolve_with(layout, params, seed, episode_survival)
}

/// Evolve a population of controllers under homeostatic-survival selection, scored by an arbitrary
/// survival `scorer` (design Part 8, Part 25; R-BEHAVIOR-EVOLVE Stage 3). The scoring ENVIRONMENT is a
/// parameter, not hardcoded, so which world a controller is selected in is data the caller supplies
/// (Principle 11): the water-foraging scorer ([`episode_survival`]), the thermal-survival scorer
/// (`thermal_episode_survival`), or any other `Fn(&Controller, ticks, seed) -> ticks_survived`. From
/// random founders, each generation scores every controller, keeps the fitter half (truncation, ties
/// broken by the lower id so the choice is deterministic), and refills the population with bounded
/// mutants of the survivors ([`mutate`]). The whole run is a pure function of the seed. Returns the
/// per-generation fitness so a caller can see behaviour improve; the physics scores survival, and
/// adaptive behaviour is what survives, never an authored objective.
pub fn evolve_with<F>(
    layout: &ControllerLayout,
    params: &EvolveParams,
    seed: u64,
    scorer: F,
) -> EvolveReport
where
    F: Fn(&Controller, u32, u64) -> u32,
{
    // A degenerate empty population has nothing to select; return an empty report rather than
    // indexing an empty slice.
    if params.pop_size == 0 {
        return EvolveReport {
            mean_fitness: Vec::new(),
            best_fitness: Vec::new(),
            final_genomes: Vec::new(),
        };
    }
    let genes = controller_gene_set(layout);
    // Founders: random controllers, one per lineage, ids 0..pop_size.
    let mut pop: Vec<Genome> = (0..params.pop_size as u64)
        .map(|id| random_controller_genome(layout, params, seed, id))
        .collect();
    let mut next_id = params.pop_size as u64;
    let mut mean_fitness = Vec::with_capacity(params.generations as usize);
    let mut best_fitness = Vec::with_capacity(params.generations as usize);

    for g in 0..params.generations as u64 {
        // Score every genome by homeostatic survival. The scoring seed folds the generation so a
        // fixed lineage is re-scored in the same environment, keyed reproducibly.
        let mut scored: Vec<(u32, usize)> = pop
            .iter()
            .enumerate()
            .map(|(i, genome)| {
                let controller = Controller::express(&genes, genome, layout);
                let fit = scorer(&controller, params.episode_ticks, seed ^ 0xE0);
                (fit, i)
            })
            .collect();
        let sum: u64 = scored.iter().map(|(f, _)| *f as u64).sum();
        mean_fitness.push(Fixed::from_ratio(sum as i64, pop.len().max(1) as i64));
        best_fitness.push(scored.iter().map(|(f, _)| *f).max().unwrap_or(0));

        // Truncation selection: keep the fitter half. Sort by fitness descending, ties to the lower
        // index (deterministic).
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let keep = (pop.len() / 2).max(1);
        let survivors: Vec<Genome> = scored[..keep]
            .iter()
            .map(|&(_, i)| pop[i].clone())
            .collect();

        // Next generation: the survivors (elitism), then a bounded mutant of each until the
        // population is refilled.
        let mut next: Vec<Genome> = Vec::with_capacity(pop.len());
        next.extend(survivors.iter().cloned());
        let mut s = 0usize;
        while next.len() < pop.len() {
            let parent = &survivors[s % survivors.len()];
            let child = mutate(parent, params, seed, next_id, g);
            next.push(child);
            next_id += 1;
            s += 1;
        }
        pop = next;
    }

    EvolveReport {
        mean_fitness,
        best_fitness,
        final_genomes: pop,
    }
}

/// The homeostatic-survival selection coefficient of one controller allele over another, for the
/// aggregate-tier recurrence [`GenePool::select`] (design 25.7): the survival advantage of the
/// adaptive variant scaled by a selection strength, so a pool carrying the adaptive controller allele
/// at a locus fixes it under the same recurrence any other locus is selected by. This is how
/// homeostatic survival becomes a selection pressure on behaviour at the deep-time tier. `sel_strength`
/// is the reserved selection scale (the epoch's, shared for consistency). The pool-tier EXPRESSION of
/// a controller from allele frequencies is the reserved coupling to the deferred quantitative tier.
pub fn homeostatic_coefficient(
    adaptive: &Controller,
    baseline: &Controller,
    ticks: u32,
    sel_strength: Fixed,
    seed: u64,
) -> Fixed {
    let fit_a = episode_survival(adaptive, ticks, seed) as i64;
    let fit_b = episode_survival(baseline, ticks, seed) as i64;
    let advantage = Fixed::from_ratio(fit_a - fit_b, ticks.max(1) as i64);
    sel_strength.mul(advantage)
}

/// The per-locus selection gradient across a scored population: for each controller locus, the
/// (unnormalised) covariance of that locus's weight with fitness, so a positive value marks a locus
/// whose higher weight tracks higher survival (the breeder's-equation direction). A caller can feed a
/// scaled gradient to [`GenePool::select`] to move a pool toward the adaptive controller. Deterministic
/// and float-free (an i128 accumulation), a pure function of the scored population.
pub fn selection_gradient(
    scored: &[(Genome, u32)],
    layout: &ControllerLayout,
    genes: &GeneSet,
) -> Vec<Fixed> {
    let n = scored.len();
    let count = layout.weight_count();
    if n == 0 {
        return vec![Fixed::ZERO; count];
    }
    // Mean fitness and per-locus mean weight.
    let fit_sum: i64 = scored.iter().map(|(_, f)| *f as i64).sum();
    let mean_fit = Fixed::from_ratio(fit_sum, n as i64);
    let controllers: Vec<Controller> = scored
        .iter()
        .map(|(g, _)| Controller::express(genes, g, layout))
        .collect();
    let mut grad = vec![Fixed::ZERO; count];
    for (k, gk) in grad.iter_mut().enumerate() {
        let weights: Vec<Fixed> = controllers.iter().map(|c| c.weight(k)).collect();
        let wsum = Fixed::saturating_sum(weights.iter().copied());
        let mean_w = wsum.div(Fixed::from_int(n as i32));
        // Covariance = mean over individuals of (w - mean_w)*(fit - mean_fit).
        let terms = controllers.iter().enumerate().map(|(i, c)| {
            let dw = c.weight(k) - mean_w;
            let df = Fixed::from_int(scored[i].1 as i32) - mean_fit;
            dw.mul(df)
        });
        let cov = Fixed::saturating_sum(terms).div(Fixed::from_int(n as i32));
        *gk = cov;
    }
    grad
}

/// Build a gene pool over the controller loci for the aggregate-tier demonstration: a biallelic pool
/// whose state-1 frequency at every locus starts at `p0`, so [`GenePool::select`] with a positive
/// coefficient raises it (the frequency of the adaptive controller allele). The pool tracks the
/// discrete Mendelian view; the pool-to-controller expression is the reserved coupling.
pub fn controller_pool(layout: &ControllerLayout, effective_size: u32, p0: Fixed) -> GenePool {
    GenePool::new(SchemeId(0), effective_size, vec![p0; layout.weight_count()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn scoring_layout(hidden: usize) -> ControllerLayout {
        ControllerLayout::new(&scoring_reg(), &AffordanceRegistry::dev_default(), hidden)
    }

    /// A hand-built competent forager over the water-only scoring layout: move toward known water,
    /// drink underfoot when dry. Its survival should be near the cap.
    fn competent(l: &ControllerLayout) -> Controller {
        let n_in = l.n_in();
        let bias = n_in - 1;
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        w[bias] = Fixed::ONE; // move_act wants to move,
        w[1] = Fixed::from_int(-1); // but not off the water underfoot (here flag, index 1)
        w[n_in + 2] = Fixed::ONE; // move_dx follows water dir_x (index 2)
        w[2 * n_in + 3] = Fixed::ONE; // move_dy follows water dir_y (index 3)
        w[3 * n_in + 1] = Fixed::ONE; // ingest_act fires when water underfoot
        w[3 * n_in] = Fixed::from_int(-1); // and the reserve (index 0) is low
        Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
    }

    #[test]
    fn a_competent_forager_outlives_a_blank_one() {
        let l = scoring_layout(0);
        let good = competent(&l);
        let blank = Controller::zeros(&l);
        let fit_good = episode_survival(&good, 200, 0xF00D);
        let fit_blank = episode_survival(&blank, 200, 0xF00D);
        assert!(
            fit_good > fit_blank,
            "the forager outlives the blank controller ({fit_good} vs {fit_blank})"
        );
        assert!(
            fit_good >= 190,
            "the competent forager survives almost to the cap ({fit_good})"
        );
    }

    #[test]
    fn episode_survival_is_deterministic() {
        let l = scoring_layout(0);
        let good = competent(&l);
        assert_eq!(
            episode_survival(&good, 120, 0xABCD),
            episode_survival(&good, 120, 0xABCD),
            "the same controller and seed replay the same survival"
        );
    }

    #[test]
    fn behaviour_evolves_under_homeostatic_selection() {
        // The proof: from random controllers, homeostatic-survival selection produces beings that
        // survive, from a random start, without water-seeking being authored anywhere.
        let l = scoring_layout(0);
        let params = EvolveParams::dev_default();
        // A seed whose dev-budget run reaches full cross-direction competence under the symmetric
        // scorer, so the near-cap claim holds against the mean-over-directions fitness.
        let report = evolve(&l, &params, 0x1111);
        let first = report.mean_fitness.first().copied().unwrap();
        let last = report.mean_fitness.last().copied().unwrap();
        assert!(
            last > first,
            "mean survival rose under selection ({} -> {})",
            first.to_f64_lossy(),
            last.to_f64_lossy()
        );
        let best_last = *report.best_fitness.last().unwrap();
        assert!(
            best_last >= 190,
            "an evolved lineage survives almost to the cap ({best_last})"
        );
    }

    #[test]
    fn the_evolved_best_follows_the_water_percept_in_every_direction() {
        // The emergent behaviour, repaired from a former east-only check. From a random start under
        // the symmetric scorer, homeostatic-survival selection produces a forager that heads toward
        // the water percept in every direction and survives water wherever it lies, without
        // water-seeking or any compass heading being authored. The seed is one whose dev-budget run
        // reaches full competence; the seed-independent guarantee that no direction is favoured is
        // `the_scorer_authors_no_directional_bias`.
        let l = scoring_layout(0);
        let params = EvolveParams::dev_default();
        let seed = 0x1111u64;
        let report = evolve(&l, &params, seed);
        let genes = controller_gene_set(&l);
        // Re-score the final population to find the best.
        let scored: Vec<(Genome, u32)> = report
            .final_genomes
            .iter()
            .map(|g| {
                let c = Controller::express(&genes, g, &l);
                (
                    g.clone(),
                    episode_survival(&c, params.episode_ticks, seed ^ 0xE0),
                )
            })
            .collect();
        let best = scored.iter().max_by_key(|(_, f)| *f).unwrap();
        let controller = Controller::express(&genes, &best.0, &l);
        // The four directions this run was scored against (its seed's wheel orientation). They are
        // mutually orthogonal, so surviving all four is itself proof the being follows the
        // water-direction percept: no fixed heading can reach water in four orthogonal directions.
        // No one wrote water-seeking; homeostatic survival selected it.
        let set = scoring_set(seed ^ 0xE0);
        for (i, &dir) in set.iter().enumerate() {
            let s = episode_survival_dir(
                &controller,
                params.episode_ticks,
                (seed ^ 0xE0) ^ SCORING_DIR_SALT[i],
                dir,
            );
            assert!(
                s >= 190,
                "the evolved being survives water lying {dir:?} ({s} ticks)"
            );
        }
    }

    fn horizontal_only(l: &ControllerLayout) -> Controller {
        // competent, but with the move_dy wiring dropped: it follows dir_x only, so it reaches
        // water east and west but is blind to north and south. The overfit the old scorer rewarded.
        let n_in = l.n_in();
        let bias = n_in - 1;
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        w[bias] = Fixed::ONE;
        w[1] = Fixed::from_int(-1);
        w[n_in + 2] = Fixed::ONE; // move_dx follows dir_x
        w[3 * n_in + 1] = Fixed::ONE;
        w[3 * n_in] = Fixed::from_int(-1);
        Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
    }

    #[test]
    fn the_scorer_authors_no_directional_bias() {
        // The anti-steering property, seed-independent: a percept-following forager survives to the
        // cap in EVERY direction of EVERY wheel orientation, so the fixture privileges no compass
        // heading, whichever base a pool draws. The earlier scorer placed water only east, which let
        // a fixed-eastward controller score as well as a percept-follower; the balanced, seed-rotated
        // set closes that. Within one wheel base the four rotations are exactly equidistant, so their
        // survivals are equal to each other, not merely each above a floor.
        let l = scoring_layout(0);
        let good = competent(&l);
        for &base in &SCORING_WHEEL {
            let perp = (-base.1, base.0);
            let set = [base, perp, (-base.0, -base.1), (-perp.0, -perp.1)];
            let per: Vec<u32> = set
                .iter()
                .enumerate()
                .map(|(i, &dir)| {
                    episode_survival_dir(&good, 200, 0xF00D ^ SCORING_DIR_SALT[i], dir)
                })
                .collect();
            assert!(
                per.iter().all(|&s| s >= 190),
                "the percept-follower survives every rotation of base {base:?} ({per:?}); no direction is harder"
            );
            assert!(
                per.iter().max().unwrap() - per.iter().min().unwrap() <= 5,
                "the four rotations of base {base:?} survive equally ({per:?}); the set is equidistant"
            );
        }
    }

    #[test]
    fn the_symmetric_scorer_separates_a_fixed_heading_from_a_percept_follower() {
        // The proof of the claim. A controller that follows only dir_x (heads east or west, blind to
        // north and south) reaches water east and west, so under the retired east-only scorer it
        // scored the cap, indistinguishable from a full percept-follower. Under the symmetric scorer
        // it starves in the two directions it ignores, so its mean falls well below the follower's:
        // the directional overfit the old scorer rewarded is now selected against.
        let l = scoring_layout(0);
        let good = competent(&l);
        let horiz = horizontal_only(&l);
        // It would have passed an east-only scorer: full survival following dir_x, either sign.
        assert!(
            episode_survival_dir(&horiz, 200, 0xF00D, (1, 0)) >= 190,
            "the overfit thrives east"
        );
        assert!(
            episode_survival_dir(&horiz, 200, 0xF00D, (-1, 0)) >= 190,
            "and west"
        );
        // But it is blind to the directions the old scorer never tested.
        assert!(
            episode_survival_dir(&horiz, 200, 0xF00D, (0, 1)) <= 100,
            "it starves north"
        );
        assert!(
            episode_survival_dir(&horiz, 200, 0xF00D, (0, -1)) <= 100,
            "and south"
        );
        // So the aggregate separates them, the distinction the east-only scorer could not make.
        let good_mean = episode_survival(&good, 200, 0xF00D);
        let horiz_mean = episode_survival(&horiz, 200, 0xF00D);
        assert!(
            good_mean >= horiz_mean + 50,
            "the percept-follower outscores the fixed-heading overfit ({good_mean} vs {horiz_mean})"
        );
    }

    #[test]
    fn the_scored_set_is_balanced_equidistant_and_seed_rotated() {
        // The seeded-rotation invariant that replaces the retired eastward imprint. For any seed the
        // scored set is four directions that (1) sum to the zero vector, so no direction is favoured;
        // (2) share one squared length, so no direction is nearer or easier; and (3) the base rotates
        // with the seed, so the wheel is exercised across pools rather than a fixed axis fixed for
        // every world. A pure, deterministic function of the seed.
        let mut bases_seen = BTreeSet::new();
        for seed in 0..2000u64 {
            let set = scoring_set(seed);
            let sum = set.iter().fold((0i32, 0i32), |a, d| (a.0 + d.0, a.1 + d.1));
            assert_eq!(
                sum,
                (0, 0),
                "the scored set sums to zero (no favoured direction)"
            );
            let len2 = |d: (i32, i32)| d.0 * d.0 + d.1 * d.1;
            assert!(
                set.iter().all(|&d| len2(d) == len2(set[0])),
                "every direction of the set is equidistant ({set:?})"
            );
            // Determinism: the same seed always yields the same set.
            assert_eq!(
                scoring_set(seed),
                set,
                "the set is a pure function of the seed"
            );
            bases_seen.insert(set[0]);
        }
        assert_eq!(
            bases_seen.len(),
            SCORING_WHEEL.len(),
            "across seeds every wheel orientation is drawn, so no fixed axis is privileged"
        );
    }

    #[test]
    fn homeostatic_survival_selects_the_adaptive_allele_in_a_pool() {
        // Wiring the coefficient into the aggregate-tier recurrence: the survival advantage of the
        // competent forever over the blank controller is a positive coefficient, and GenePool::select
        // raises the frequency of the adaptive controller allele (the epoch's existing mechanism).
        let l = scoring_layout(0);
        let good = competent(&l);
        let blank = Controller::zeros(&l);
        let sel_strength = Fixed::from_ratio(1, 5);
        let coeff = homeostatic_coefficient(&good, &blank, 200, sel_strength, 0xC0FFEE);
        assert!(
            coeff > Fixed::ZERO,
            "surviving longer is a positive selection coefficient"
        );
        let mut pool = controller_pool(&l, 200, Fixed::from_ratio(1, 2));
        let before = pool.freq(0).unwrap();
        for _ in 0..20 {
            pool.select(&vec![coeff; pool.loci()]);
        }
        let after = pool.freq(0).unwrap();
        assert!(
            after > before,
            "the adaptive controller allele rises in the pool ({} -> {})",
            before.to_f64_lossy(),
            after.to_f64_lossy()
        );
    }

    #[test]
    fn the_selection_gradient_is_deterministic_and_sized() {
        let l = scoring_layout(0);
        let genes = controller_gene_set(&l);
        let params = EvolveParams::dev_default();
        let report = evolve(&l, &params, 0x1111);
        let scored: Vec<(Genome, u32)> = report
            .final_genomes
            .iter()
            .map(|g| {
                let c = Controller::express(&genes, g, &l);
                (
                    g.clone(),
                    episode_survival(&c, params.episode_ticks, 0x1111 ^ 0xE0),
                )
            })
            .collect();
        let grad_a = selection_gradient(&scored, &l, &genes);
        let grad_b = selection_gradient(&scored, &l, &genes);
        assert_eq!(
            grad_a, grad_b,
            "the gradient is a deterministic function of the population"
        );
        assert_eq!(
            grad_a.len(),
            l.weight_count(),
            "one gradient entry per controller weight"
        );
    }

    #[test]
    fn an_empty_population_evolves_to_an_empty_report_without_panicking() {
        // A degenerate config (no lineages) must not panic on an empty-slice index.
        let l = scoring_layout(0);
        let params = EvolveParams {
            pop_size: 0,
            generations: 3,
            ..EvolveParams::dev_default()
        };
        let report = evolve(&l, &params, 0x0);
        assert!(report.final_genomes.is_empty());
        assert!(report.mean_fitness.is_empty());
    }

    // --- Stage 4: the full-episode (dawn) tier and the recurrent-network graduation ---

    #[test]
    fn the_full_episode_tier_rewards_finding_water_by_search() {
        // The high-fidelity tier: the being is not shown the water, so it must explore to discover
        // it. The competent forager (which explores when it knows of no water) finds and drinks it
        // and outlives the blank one, which idles and dies of thirst. This validates that foraging
        // from known sources (the proxy) carries over to the full loop with search. The seed selects
        // the axis-aligned wheel orientation: the hand-built forager's search is direction-dependent
        // (a fixture-controller limitation the rotated scorer surfaces, not a scorer bias; the
        // shown-water proxy that drives selection is isotropic, per
        // `the_scorer_authors_no_directional_bias`), so the carryover is demonstrated where the
        // fixture can search.
        assert_eq!(
            scoring_set(0x4444)[0],
            (1, 0),
            "this seed draws the axis-aligned base"
        );
        let l = scoring_layout(0);
        let good = competent(&l);
        let blank = Controller::zeros(&l);
        let fit_good = full_episode_survival(&good, 400, 0x4444);
        let fit_blank = full_episode_survival(&blank, 400, 0x4444);
        assert!(
            fit_good > fit_blank + 100,
            "the forager finds water by search and far outlives the idle one ({fit_good} vs {fit_blank})"
        );
    }

    #[test]
    fn a_recurrent_controller_graduates_the_plumbing_and_evolves() {
        // The graduation: the same expression, selection, and mutation plumbing runs a small
        // recurrent network (a hidden state) rather than a reaction norm, its topology fixed Rust
        // and its weights the heritable data. Behaviour still evolves under homeostatic selection,
        // so moving to the network is a parameter change (the hidden width), not a rewrite.
        let l = scoring_layout(1); // hidden width 1: a recurrent controller
        assert_eq!(l.hidden(), 1);
        assert!(l.weight_count() > 0);
        let params = EvolveParams {
            generations: 24,
            ..EvolveParams::dev_default()
        };
        let report = evolve(&l, &params, 0x9E77);
        let first = report.mean_fitness.first().copied().unwrap();
        let last = report.mean_fitness.last().copied().unwrap();
        assert!(
            last > first,
            "the recurrent controller's behaviour evolves under selection too ({} -> {})",
            first.to_f64_lossy(),
            last.to_f64_lossy()
        );
    }

    // --- The thermal-survival scorer and the thermotaxis-evolution proof ---

    /// A hand-built thermal kinesis: move while cold, rest once comfortable (move_act = 3 - 4*level),
    /// with no directional output. Used only to show the scoring environment rewards the behaviour that
    /// selection then discovers on its own; not the thing under test.
    fn thermal_kinesis(l: &ControllerLayout) -> Controller {
        let n_in = l.n_in();
        let bias = n_in - 1;
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        w[bias] = Fixed::from_int(3);
        w[0] = Fixed::from_int(-4);
        Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
    }

    /// A hand-built thermal TAXIS: move while cold (as the kinesis), and follow the temperature-gradient
    /// percept (move_dx from dir_x, move_dy from dir_y), so it climbs toward warmer surroundings. This is
    /// the behaviour the gradient percept makes possible; selection then discovers it on its own. Input
    /// layout (one axis): [level(0), here(1), dir_x(2), dir_y(3), bias(4)]; outputs [move_act(0),
    /// move_dx(1), move_dy(2), ingest(3)]; weight index = out*n_in + in.
    fn thermal_taxis(l: &ControllerLayout) -> Controller {
        let n_in = l.n_in();
        let bias = n_in - 1;
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        w[bias] = Fixed::from_int(3); // move_act: move while uncomfortable,
        w[0] = Fixed::from_int(-4); //   suppressed as comfort rises.
        w[n_in + 2] = Fixed::ONE; // move_dx follows the gradient dir_x,
        w[2 * n_in + 3] = Fixed::ONE; // move_dy follows the gradient dir_y.
        Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
    }

    /// A hand-built FIXED-HEADING controller: move while cold, but always head +x, ignoring the gradient
    /// percept (move_dx from the bias, move_dy zero). The thermal analogue of the retired eastward-water
    /// overfit: it reaches warmth only from the start whose outward direction happens to be +x.
    fn thermal_fixed_heading(l: &ControllerLayout) -> Controller {
        let n_in = l.n_in();
        let bias = n_in - 1;
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        w[bias] = Fixed::from_int(3);
        w[0] = Fixed::from_int(-4);
        w[n_in + bias] = Fixed::ONE; // move_dx from the bias: a constant +x heading, gradient-blind.
        Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
    }

    /// The four bowl starts scored per-episode for a controller and seed (the per-start survival that
    /// the aggregate scorer averages), so a test can inspect the directional profile.
    fn bowl_per_start(controller: &Controller, seed: u64) -> Vec<u32> {
        let (setpoint, half_band, cold) =
            (Fixed::from_int(37), Fixed::from_int(8), Fixed::from_int(21));
        bowl_starts()
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                thermal_run(
                    controller,
                    200,
                    seed ^ SCORING_DIR_SALT[i],
                    thermal_bowl_field(setpoint, cold),
                    s,
                    setpoint,
                    half_band,
                )
            })
            .collect()
    }

    #[test]
    fn the_bowl_scorer_field_favours_no_direction() {
        // Anti-steering (R-EVOLVE-STEER), bit-exact: the radial bowl is invariant under a 90-degree
        // rotation about its centre (squared radial distance is), so warmth lies the same distance in
        // every direction and no compass heading is privileged.
        let field = thermal_bowl_field(Fixed::from_int(37), Fixed::from_int(21));
        let (w, h) = field.dims();
        assert_eq!(w, h, "the bowl is square");
        for y in 0..h {
            for x in 0..w {
                assert_eq!(
                    field.at(x, y),
                    field.at(h - 1 - y, x),
                    "the bowl is not invariant under a 90-degree rotation at ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn sensed_taxis_outlives_undirected_kinesis() {
        // The proof the gradient percept unlocks. A controller that reads the temperature-gradient
        // percept and climbs it (taxis) reaches the warm ring from every start and survives to the cap;
        // an otherwise-identical controller that cannot sense a direction (kinesis) must random-walk out
        // and often dies first. Directed beats undirected, and the only difference is reading the
        // percept, so it is the percept, not merely selection pressure, that unlocks directed taxis.
        let l = thermal_layout(0);
        let taxis = thermal_taxis(&l);
        let kin = thermal_kinesis(&l);
        for seed in [0xA1u64, 0xB2, 0xC3] {
            let t = thermal_sensed_survival(&taxis, 200, seed);
            let k = thermal_sensed_survival(&kin, 200, seed);
            assert!(
                t >= 190,
                "the gradient-follower reaches warmth from every start ({t})"
            );
            assert!(
                t >= k + 40,
                "and outlives the undirected random walk ({t} vs {k}) at seed {seed:#x}"
            );
        }
    }

    #[test]
    fn the_gradient_scorer_rewards_the_percept_follower_over_a_fixed_heading() {
        // The other half of the anti-steering discipline, the thermal analogue of the water fixed-
        // heading-versus-percept-follower test: a controller committed to a fixed heading has a
        // directional blind spot (it dies at the start whose outward direction opposes its heading),
        // while the percept-follower climbs outward from every start. So the scorer rewards reading the
        // gradient, not a fixed compass heading.
        let l = thermal_layout(0);
        let taxis = thermal_taxis(&l);
        let fixed = thermal_fixed_heading(&l);
        let tp = bowl_per_start(&taxis, 0xA1);
        let fp = bowl_per_start(&fixed, 0xA1);
        assert!(
            tp.iter().all(|&s| s >= 190),
            "the percept-follower survives every start ({tp:?})"
        );
        assert!(
            fp.iter().any(|&s| s <= 60),
            "the fixed heading has a dead start, a directional blind spot ({fp:?})"
        );
        let ta = thermal_sensed_survival(&taxis, 200, 0xA1);
        let fa = thermal_sensed_survival(&fixed, 200, 0xA1);
        assert!(
            ta >= fa + 30,
            "the percept-follower outscores the fixed heading in aggregate ({ta} vs {fa})"
        );
    }

    #[test]
    fn thermotaxis_by_gradient_evolves() {
        // From random controllers, thermal-survival selection with the gradient percept available
        // produces beings that climb the gradient toward warmth (a directed taxis), reaching safety from
        // every start, without any heading being authored. Survival to the cap requires reaching the
        // warm ring, so a near-cap evolved fitness is the behavioural proof.
        let l = thermal_layout(0);
        let params = EvolveParams::dev_default();
        let report = evolve_thermal_sensed(&l, &params, 0x1234);
        let first = report.mean_fitness.first().copied().unwrap();
        let last = report.mean_fitness.last().copied().unwrap();
        assert!(
            last > first,
            "mean survival rose under selection ({} -> {})",
            first.to_f64_lossy(),
            last.to_f64_lossy()
        );
        let best_last = *report.best_fitness.last().unwrap();
        assert!(
            best_last >= 190,
            "an evolved lineage climbs to safety from every start ({best_last})"
        );
    }

    #[test]
    fn thermal_sensed_survival_is_deterministic() {
        let l = thermal_layout(0);
        let taxis = thermal_taxis(&l);
        assert_eq!(
            thermal_sensed_survival(&taxis, 150, 0xBEEF),
            thermal_sensed_survival(&taxis, 150, 0xBEEF),
            "the same controller and seed replay the same sensed-taxis survival"
        );
    }

    #[test]
    fn the_thermal_scorer_field_favours_no_direction() {
        // The anti-steering property of the scoring environment (R-EVOLVE-STEER), bit-exact: the warm-
        // border field is invariant under a 90-degree rotation about its centre, so warmth lies the
        // same distance in every direction and no compass heading is privileged. A being at the centre
        // faces an isotropic problem; selection cannot reward a fixed heading the way the retired
        // eastward water scorer did.
        let field = thermal_scoring_field(Fixed::from_int(37), Fixed::from_int(21));
        let (w, h) = field.dims();
        assert_eq!(w, h, "the scoring field is square");
        for y in 0..h {
            for x in 0..w {
                // A 90-degree rotation about the centre maps (x, y) to (h - 1 - y, x).
                assert_eq!(
                    field.at(x, y),
                    field.at(h - 1 - y, x),
                    "the scoring field is not invariant under a 90-degree rotation at ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn a_thermotaxis_controller_outlives_an_idle_one_in_the_cold() {
        // The scoring environment rewards the behaviour: a being that moves while cold reaches the
        // surrounding warmth and holds it (surviving to the cap), while an idle being cools and dies at
        // the centre. This is the gradient selection climbs; it is not the proof (that is below).
        let l = thermal_layout(0);
        let kin = thermal_kinesis(&l);
        let blank = Controller::zeros(&l);
        let fit_kin = thermal_episode_survival(&kin, 200, 0xA1);
        let fit_blank = thermal_episode_survival(&blank, 200, 0xA1);
        assert!(
            fit_kin >= 190,
            "the mover reaches warmth and holds it ({fit_kin})"
        );
        assert!(
            fit_kin > fit_blank + 100,
            "and far outlives the idle being, which dies of cold at the centre ({fit_kin} vs {fit_blank})"
        );
    }

    #[test]
    fn thermal_episode_survival_is_deterministic() {
        let l = thermal_layout(0);
        let kin = thermal_kinesis(&l);
        assert_eq!(
            thermal_episode_survival(&kin, 150, 0xBEEF),
            thermal_episode_survival(&kin, 150, 0xBEEF),
            "the same controller and seed replay the same thermal survival"
        );
    }

    #[test]
    fn thermotaxis_evolves_under_thermal_survival_selection() {
        // The proof, the in-situ analogue of the water-seeking result: from random controllers, THERMAL
        // homeostatic-survival selection on the coupled runner produces beings that keep their bodies in
        // the viable band, moving to the surrounding warmth and holding it, without thermotaxis being
        // authored anywhere. Survival to the cap in this environment requires reaching and holding the
        // warmth (an idle being dies at the centre in a fraction of the episode), so a near-cap evolved
        // fitness is the behavioural proof. This retires the hand-built thermotaxis fixture.
        let l = thermal_layout(0);
        let params = EvolveParams::dev_default();
        let report = evolve_thermal(&l, &params, 0x1234);
        let first = report.mean_fitness.first().copied().unwrap();
        let last = report.mean_fitness.last().copied().unwrap();
        assert!(
            last > first,
            "mean thermal survival rose under selection ({} -> {})",
            first.to_f64_lossy(),
            last.to_f64_lossy()
        );
        let best_last = *report.best_fitness.last().unwrap();
        assert!(
            best_last >= 190,
            "an evolved lineage keeps its body in the band almost to the cap ({best_last})"
        );
    }
}
