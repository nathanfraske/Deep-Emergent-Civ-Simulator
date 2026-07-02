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

//! Emergent locomotion driven by an evolved controller (design Part 8, Part 9, Part 13, Part 20,
//! Part 25; R-BEHAVIOR-EVOLVE; Principles 8, 9, 10).
//!
//! What is authored here is physics, and only physics. A body's capacity to move is its morphology
//! ([`crate::anatomy::BodyPlan`]): a body with no locomotion organ is rooted and never moves,
//! whatever its kingdom, so a rooted tree stays put while a body that bears the organ moves, even an
//! autotroph, so a walking tree walks. Whether a body has that organ is itself an emergent
//! morphological outcome, not a rule keyed on being a plant. Its ground speed comes from its size,
//! its activity, and the terrain (passability and cost, read through [`Terrain`]). Its needs are the
//! homeostatic reserves that drain by metabolism ([`crate::homeostasis`]); its options are the
//! affordances its morphology permits. All of that is physics.
//!
//! What is not authored is the behaviour: which affordance the being issues, and where it aims it.
//! That is the evolved controller ([`crate::controller`]), a heritable mapping from the being's
//! reserves and percept to an affordance, expressed from its genome and (under the pre-dawn epoch)
//! selected by whether it keeps the body alive. Nobody writes "seek water when dry": each tick the
//! being perceives the sources within its sensory range and remembers them ([`Walker::known`]),
//! reads its own reserves, and its controller decides. A being that has evolved the adaptive coupling
//! walks up the gradient to a known source and ingests it; one that has not starves. This is the
//! retirement of the authored decision menu that the prior slice flagged: the drives-and-actions
//! policy is gone from this path, replaced by the expressed controller (the [`crate::decision`]
//! utility layer remains the shape of the sentient, deliberative tier above, which the controller
//! underlies rather than replaces).
//!
//! Non-omniscience stands: a being knows only what it has perceived (a small true sensory range) or
//! remembered, so it cannot head for a source it has never seen; when its controller wants to move
//! but has no known gradient to follow, it explores, a heading drawn from counter-based RNG keyed on
//! the being and the tick ([`civsim_core::Phase::EXPLORE`]), discovering the world by moving through
//! it. The mechanism is fixed Rust and fully deterministic: beings are walked in stable-id order,
//! position is exact fixed-point (a subtile fractional coordinate), the controller evaluation and the
//! metabolism draw no randomness, and every choice keys on the seed, the being, and the tick, never
//! on the camera (Principle 10). What the movement physics needs is reserved with its basis in
//! [`LocomotionParams`] and defaulted only by a labelled development fixture.
//!
//! Honest limits. Perception is a range gate, not yet line of sight or the full belief store of Part
//! 9, and knowledge is never forgotten or shared. Movement is straight-line with a passability gate
//! rather than routing around an obstacle (the pathfinding of Part 13). The reaction-norm controller
//! cannot gate a response on internal state through a product (it moves toward a known source
//! whenever away from it, whatever the reserve, and ingests underfoot when the reserve is low); the
//! recurrent controller lifts that ceiling (both are [`crate::controller`]). The intake yield here is
//! a reserved fraction standing in for the resolved edibility floor's measure (R-PHYS-BIO); wiring
//! the floor's net-nutrition and water-content per bite is the named follow-on.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::{DrawKey, Fixed, Phase, StableId};
use civsim_world::Coord3;

use crate::anatomy::BodyPlan;
use crate::controller::{Controller, ControllerLayout};
use crate::homeostasis::{
    AffordanceRegistry, Homeostasis, HomeostaticAxisId, HomeostaticRegistry, INGEST, MOVE,
};

/// The reserved parameters of the movement physics. The mechanism that reads them is fixed; these
/// numbers are the owner's to set, surfaced with a basis, never fabricated (Principle 11). The
/// development fixture below lets the module run and be tested now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LocomotionParams {
    /// Tiles per tick a maximal, fully active body crosses on flat, open ground. RESERVED. Basis:
    /// a real walking speed (about 1.4 m/s) divided by the tile edge in metres, at the one-second
    /// base tick, so a person crosses roughly one tile per second on open ground.
    pub base_speed: Fixed,
    /// How much terrain cost above the open-ground baseline slows movement (speed is divided by
    /// `1 + terrain_penalty * (cost - 1)`). RESERVED. Basis: the slowdown of real difficult ground
    /// (broken, steep, or wet terrain) relative to open ground.
    pub terrain_penalty: Fixed,
    /// The lowest activity factor, so even a sluggish body creeps rather than freezing (the
    /// temperament activity axis scales speed between this floor and one). RESERVED. Basis: the
    /// ratio of a slow gait to a brisk one.
    pub activity_floor: Fixed,
    /// How far, in whole tiles, a being perceives a source: the true sensory range within which it
    /// comes to know of a resource, so knowledge is earned by nearness, not read from the map.
    /// RESERVED. Basis: the perception range the being's sensory morphology and acuity imply
    /// (Part 9); small, a handful of tiles, not the whole world.
    pub sense_range: i64,
    /// How many ticks an exploration heading holds before it is redrawn, so a searching being keeps
    /// a direction rather than jittering in place. RESERVED. Basis: the persistence of a real
    /// search path before it turns.
    pub explore_persistence: u64,
    /// The fraction of a reserve's capacity restored by one tick of ingesting a source of it.
    /// RESERVED. Basis: the net-nutrition and water-content per bite the resolved edibility floor
    /// measures (R-PHYS-BIO, `crate::edibility`); this fraction is the interim stand-in until the
    /// floor's measure is wired to the located matter.
    pub intake_yield: Fixed,
}

impl LocomotionParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values, so locomotion runs and can be tested now.
    pub fn dev_default() -> LocomotionParams {
        LocomotionParams {
            base_speed: Fixed::from_ratio(1, 1),
            terrain_penalty: Fixed::from_ratio(1, 1),
            activity_floor: Fixed::from_ratio(1, 4),
            sense_range: 4,
            explore_persistence: 6,
            intake_yield: Fixed::from_ratio(1, 4),
        }
    }
}

/// The world's terrain, read by the movement physics. The world implements this over its map; the
/// module stays world-agnostic. Passability is body-aware, so a body that can swim crosses water a
/// walker cannot: physics gating a body against the ground, never a scripted route.
pub trait Terrain {
    /// Whether a body may enter this tile. A tile off the map is not passable.
    fn passable(&self, coord: Coord3, body: &BodyPlan) -> bool;

    /// The movement cost multiplier of a tile, at least one on open ground and higher on difficult
    /// ground (slope, mud, undergrowth). A pure physical property of the tile.
    fn cost(&self, coord: Coord3) -> Fixed;
}

/// Where the sources of each homeostatic axis really sit on the map, the world's ground truth: an
/// axis maps to the set of tiles that bear matter restoring it (water tiles bear water, forage tiles
/// bear energy). The world builds this from its content and the edibility floor; a being does not
/// get to read it, it can only perceive the tiles near it (see [`step`]). The module never hardcodes
/// which axis a tile restores, so a source-of-an-axis exists only where that pairing is placed
/// (Principle 11).
#[derive(Clone, Debug, Default)]
pub struct ResourceField {
    tiles: BTreeMap<HomeostaticAxisId, BTreeSet<Coord3>>,
}

impl ResourceField {
    /// An empty field.
    pub fn new() -> ResourceField {
        ResourceField::default()
    }

    /// Record that `coord` bears a source of `axis`.
    pub fn add(&mut self, axis: HomeostaticAxisId, coord: Coord3) {
        self.tiles.entry(axis).or_default().insert(coord);
    }

    /// Whether a coordinate bears a source of an axis.
    pub fn source(&self, axis: HomeostaticAxisId, coord: Coord3) -> bool {
        self.tiles.get(&axis).is_some_and(|s| s.contains(&coord))
    }

    /// The axes this field carries sources for, in canonical id order.
    pub fn axes(&self) -> impl Iterator<Item = HomeostaticAxisId> + '_ {
        self.tiles.keys().copied()
    }

    /// The axes whose source is on a given tile, in canonical id order (what a being can ingest
    /// where it stands).
    pub fn axes_here(&self, coord: Coord3) -> Vec<HomeostaticAxisId> {
        self.tiles
            .iter()
            .filter(|(_, s)| s.contains(&coord))
            .map(|(&a, _)| a)
            .collect()
    }
}

/// A being that occupies the map and can walk: its stable id, its exact position in fractional tile
/// coordinates, its body plan (the physics of how it moves), its homeostatic reserves (its needs,
/// draining by metabolism), its expressed behaviour controller (its evolved policy) and the hidden
/// state the controller carries, its own remembered knowledge of where sources of each axis are (a
/// belief earned by perceiving them, not a copy of the world), and whether it is still alive.
#[derive(Clone, Debug)]
pub struct Walker {
    /// The stable id, the canonical order beings are walked in.
    pub id: StableId,
    /// Position along the world x axis, in tiles, fractional.
    pub x: Fixed,
    /// Position along the world y axis, in tiles, fractional.
    pub y: Fixed,
    /// The body plan the movement physics reads (mass, activity, whether it has locomotion at all).
    pub body: BodyPlan,
    /// The homeostatic reserves: the being's needs as physical states of its body.
    pub homeostasis: Homeostasis,
    /// The expressed behaviour controller, the being's evolved policy.
    pub controller: Controller,
    /// The controller's carried hidden state (empty for a reaction norm).
    pub hidden: Vec<Fixed>,
    /// What this being knows: the tiles bearing a source of each axis that it has perceived. It
    /// navigates by this, not by the world, so it can only head for a source it has come to know.
    pub known: BTreeMap<HomeostaticAxisId, BTreeSet<Coord3>>,
    /// Whether the being is alive. A being whose reserve falls through its floor dies and stops.
    pub alive: bool,
}

impl Walker {
    /// A walker placed at the centre of a tile with the given reserves and controller, no knowledge
    /// yet: it has seen nothing and must perceive or explore to learn the world.
    pub fn new(
        id: StableId,
        tile: Coord3,
        body: BodyPlan,
        homeostasis: Homeostasis,
        controller: Controller,
    ) -> Walker {
        let hidden = controller.fresh_hidden();
        Walker {
            id,
            x: Fixed::from_int(tile.x) + HALF,
            y: Fixed::from_int(tile.y) + HALF,
            body,
            homeostasis,
            controller,
            hidden,
            known: BTreeMap::new(),
            alive: true,
        }
    }

    /// The tile the being currently stands on.
    pub fn coord(&self) -> Coord3 {
        Coord3::ground(floor_i32(self.x), floor_i32(self.y))
    }

    /// Whether this body can walk at all: a body with no locomotion organ is rooted (a fixed tree)
    /// and never moves however its controller reads, whatever its kingdom; a body that bears one
    /// moves, even an autotroph, so a walking tree walks. Mobility is the body, never the kingdom.
    pub fn is_mobile(&self) -> bool {
        !self.body.locomotion.is_empty() && self.body.locomotion.iter().any(|&m| m != ROOTED_MODE)
    }

    /// Record that this being now knows of a source of `axis` at `coord`.
    pub fn learn(&mut self, axis: HomeostaticAxisId, coord: Coord3) {
        self.known.entry(axis).or_default().insert(coord);
    }

    /// The nearest source of `axis` this being knows of, to where it stands, by squared distance
    /// with a canonical tie-break. `None` if it knows of none.
    fn nearest_known(&self, axis: HomeostaticAxisId) -> Option<Coord3> {
        let from = self.coord();
        self.known.get(&axis)?.iter().copied().min_by_key(|c| {
            let dx = (c.x - from.x) as i64;
            let dy = (c.y - from.y) as i64;
            (dx * dx + dy * dy, c.x, c.y)
        })
    }
}

/// One-half, the tile centre offset.
const HALF: Fixed = Fixed::from_bits(1i64 << 31);
/// The registry id of the rooted (non-)locomotion mode: a body carrying only this does not walk.
const ROOTED_MODE: u16 = 0;
/// The smallest squared heading magnitude that counts as a directional signal; below it the being
/// has no gradient to follow and explores instead.
const HEADING_EPS: Fixed = Fixed::from_bits(1i64 << 22); // ~1e-3

/// Floor a fractional tile coordinate to its integer tile (arithmetic shift floors negatives too;
/// Q32.32 fixed point).
fn floor_i32(v: Fixed) -> i32 {
    (v.to_bits() >> 32) as i32
}

/// The physics of a body's ground speed on a tile, in tiles per tick. It rises with body size (an
/// allometric square-root of mass, larger bodies taking longer strides), scales with the temperament
/// activity axis between the reserved floor and one, and is divided down by terrain cost above open
/// ground. A body with no locomotion organ does not move. Whether the being has the reserves to move
/// is the metabolism's concern, not this pure physical speed.
pub fn locomotion_speed(body: &BodyPlan, terrain_cost: Fixed, p: &LocomotionParams) -> Fixed {
    let mobile = !body.locomotion.is_empty() && body.locomotion.iter().any(|&m| m != ROOTED_MODE);
    if !mobile {
        return Fixed::ZERO;
    }
    // Allometric size factor: sqrt(body_mass) in [0, 1], so a bigger body strides faster.
    let size = body.body_mass.clamp(Fixed::ZERO, Fixed::ONE).sqrt();
    // Activity factor between the reserved floor and one.
    let activity = p.activity_floor
        + (Fixed::ONE - p.activity_floor)
            .mul(body.temperament.activity.clamp(Fixed::ZERO, Fixed::ONE));
    // Terrain divisor: 1 + terrain_penalty * (cost - 1), never below one.
    let over = if terrain_cost > Fixed::ONE {
        terrain_cost - Fixed::ONE
    } else {
        Fixed::ZERO
    };
    let divisor = Fixed::ONE + p.terrain_penalty.mul(over);
    let raw = p.base_speed.mul(size).mul(activity);
    let speed = if divisor > Fixed::ZERO {
        raw.div(divisor)
    } else {
        raw
    };
    speed.clamp(Fixed::ZERO, p.base_speed)
}

/// Perceive the world within the being's sensory range: for each axis the field carries, any source
/// tile within `sense_range` tiles of where the being stands is learned. This is the being seeing
/// what is near it; it learns nothing about tiles beyond its senses.
fn perceive(w: &mut Walker, resources: &ResourceField, range: i64) {
    let here = w.coord();
    let axes: Vec<HomeostaticAxisId> = resources.axes().collect();
    for axis in axes {
        for dy in -range..=range {
            for dx in -range..=range {
                let c = Coord3::ground(here.x + dx as i32, here.y + dy as i32);
                if resources.source(axis, c) {
                    w.learn(axis, c);
                }
            }
        }
    }
}

/// The unit direction from a being to the nearest known source of each axis it knows of. A source
/// on the being's own tile reads as a zero direction (there is nowhere to go for it); the being
/// tells that case apart through the separate here-flag the percept carries.
fn source_dirs(w: &Walker) -> BTreeMap<HomeostaticAxisId, (Fixed, Fixed)> {
    let mut m = BTreeMap::new();
    let axes: Vec<HomeostaticAxisId> = w.known.keys().copied().collect();
    for axis in axes {
        if let Some(c) = w.nearest_known(axis) {
            let tx = Fixed::from_int(c.x) + HALF;
            let ty = Fixed::from_int(c.y) + HALF;
            let dx = tx - w.x;
            let dy = ty - w.y;
            let dist = (dx.mul(dx) + dy.mul(dy)).sqrt();
            if dist > Fixed::ZERO {
                let ux = dx.div(dist).clamp(Fixed::from_int(-1), Fixed::ONE);
                let uy = dy.div(dist).clamp(Fixed::from_int(-1), Fixed::ONE);
                m.insert(axis, (ux, uy));
            } else {
                m.insert(axis, (Fixed::ZERO, Fixed::ZERO));
            }
        }
    }
    m
}

/// Advance every being one tick of controller-driven locomotion. Each perceives nearby sources into
/// its memory, reads its reserves and its percept, and its controller decides which affordance to
/// issue: moving (toward a known source it is drawn to, or exploring when it has no gradient) or
/// ingesting the matter underfoot. Then its metabolism drains its reserves, more when it exerted
/// itself, and a being whose reserve falls through its floor dies. Deterministic: beings are walked
/// in stable-id order, the controller and metabolism draw no randomness, exploration keys on
/// `(seed, being, tick)`, and every step is exact fixed-point. Returns the number of beings that
/// moved this tick.
#[allow(clippy::too_many_arguments)]
pub fn step<T: Terrain>(
    walkers: &mut [Walker],
    homeo: &HomeostaticRegistry,
    layout: &ControllerLayout,
    afford: &AffordanceRegistry,
    terrain: &T,
    resources: &ResourceField,
    p: &LocomotionParams,
    seed: u64,
    tick: u64,
) -> usize {
    step_with_field_dirs(
        walkers,
        homeo,
        layout,
        afford,
        terrain,
        resources,
        p,
        seed,
        tick,
        &BTreeMap::new(),
    )
}

/// As [`step`], but with an additional per-being map of field-derived percept directions, keyed by
/// stable id then by homeostatic axis. This is a directional percept a being senses from a physical
/// field rather than from a remembered point source: the temperature comfort gradient the runner
/// supplies for the TEMPERATURE axis (the unit direction of increasing comfort at the being's cell),
/// and later a moisture or wind field, merged into that axis's direction slot of the controller input
/// alongside the known-source percept. It is a percept, not a heading: the controller must evolve to
/// follow it (Principle 9), and it draws no randomness, so determinism and camera-freedom hold. A
/// field direction for an axis overrides the known-source direction for that axis, since the field
/// percept is the live signal for a diffuse quantity that has no discrete source tile.
#[allow(clippy::too_many_arguments)]
pub fn step_with_field_dirs<T: Terrain>(
    walkers: &mut [Walker],
    homeo: &HomeostaticRegistry,
    layout: &ControllerLayout,
    afford: &AffordanceRegistry,
    terrain: &T,
    resources: &ResourceField,
    p: &LocomotionParams,
    seed: u64,
    tick: u64,
    field_dirs: &BTreeMap<StableId, BTreeMap<HomeostaticAxisId, (Fixed, Fixed)>>,
) -> usize {
    walkers.sort_by_key(|w| w.id);
    let mut moved = 0usize;
    for w in walkers.iter_mut() {
        if !w.alive {
            continue;
        }
        // Perceive first, so knowledge gained this tick is available to this tick's decision.
        perceive(w, resources, p.sense_range);
        let here = w.coord();
        let here_axes: BTreeSet<HomeostaticAxisId> =
            resources.axes_here(here).into_iter().collect();
        let mut dirs = source_dirs(w);
        // Merge the field-derived percept for this being: a directional signal it senses from a
        // physical field (the temperature comfort gradient), overriding the known-source direction for
        // that axis since a diffuse field has no discrete source tile to remember.
        if let Some(fd) = field_dirs.get(&w.id) {
            for (&axis, &d) in fd {
                dirs.insert(axis, d);
            }
        }
        let input = layout.build_input(&w.homeostasis, &here_axes, &dirs);
        let (out, new_hidden) = w.controller.evaluate(&input, &w.hidden);
        w.hidden = new_hidden;
        let afforded = afford.afforded(&w.body);
        let decision = layout.decide(&out, &afforded);

        let mut exertion = Fixed::ZERO;
        if let Some(d) = decision {
            if d.activation > Fixed::ZERO {
                match d.affordance {
                    MOVE => {
                        let cost = terrain.cost(here);
                        let speed = locomotion_speed(&w.body, cost, p);
                        if speed > Fixed::ZERO {
                            let (hx, hy) = d.heading.unwrap_or((Fixed::ZERO, Fixed::ZERO));
                            let mag2 = hx.mul(hx) + hy.mul(hy);
                            let did = if mag2 > HEADING_EPS {
                                walk_dir(w, hx, hy, speed, terrain)
                            } else {
                                // It wants to move but has no known gradient: it explores.
                                explore(w, terrain, speed, p, seed, tick)
                            };
                            if did {
                                moved += 1;
                                exertion = Fixed::ONE;
                            }
                        }
                    }
                    INGEST => {
                        // Take in what the current tile offers on each axis it is a source of; the
                        // yield is the reserved fraction of the reserve's capacity.
                        for axis in resources.axes_here(here) {
                            let cap = w.homeostasis.capacity(axis);
                            let amount = p.intake_yield.mul(cap);
                            w.homeostasis.ingest(axis, amount);
                        }
                    }
                    _ => {} // an affordance the engine has no enactment for yet: idle
                }
            }
        }

        // Metabolism drains the reserves every tick (basal, plus the tick's exertion); a being whose
        // reserve falls through its floor dies.
        if !w.homeostasis.metabolize(homeo, exertion) {
            w.alive = false;
        }
    }
    moved
}

/// The eight headings a searching being can take, unit vectors so a diagonal step covers the same
/// ground as a cardinal one.
fn headings() -> [(Fixed, Fixed); 8] {
    let d = Fixed::from_ratio(7071, 10000); // ~1/sqrt(2)
    let z = Fixed::ZERO;
    let o = Fixed::ONE;
    let n = |v: Fixed| Fixed::ZERO - v;
    [
        (o, z),
        (d, d),
        (z, o),
        (n(d), d),
        (n(o), z),
        (n(d), n(d)),
        (z, n(o)),
        (d, n(d)),
    ]
}

/// Explore: move one step along a heading drawn from counter-based RNG keyed on the being and the
/// exploration period, so the search is a reproducible function of the seed, the being, and the
/// tick, never of the camera. If the drawn heading is blocked, the being rotates through the other
/// headings deterministically and takes the first passable one, so it is not trapped against a wall.
fn explore<T: Terrain>(
    w: &mut Walker,
    terrain: &T,
    speed: Fixed,
    p: &LocomotionParams,
    seed: u64,
    tick: u64,
) -> bool {
    let period = p.explore_persistence.max(1);
    let base = DrawKey::entity(w.id.0, tick / period, Phase::EXPLORE)
        .rng(seed)
        .range_u32(0, 8);
    let dirs = headings();
    for k in 0..8u32 {
        let (dx, dy) = dirs[((base + k) % 8) as usize];
        let nx = w.x + dx.mul(speed);
        let ny = w.y + dy.mul(speed);
        let ncoord = Coord3::ground(floor_i32(nx), floor_i32(ny));
        if terrain.passable(ncoord, &w.body) {
            w.x = nx;
            w.y = ny;
            return true;
        }
    }
    false // hemmed in on every side
}

/// Step a walker one step of `speed` along a heading vector, normalising the heading and entering
/// only a passable tile. Returns whether it moved. A blocked step holds the being in place (routing
/// is Part 13, future).
fn walk_dir<T: Terrain>(w: &mut Walker, hx: Fixed, hy: Fixed, speed: Fixed, terrain: &T) -> bool {
    let mag = (hx.mul(hx) + hy.mul(hy)).sqrt();
    if mag <= Fixed::ZERO {
        return false;
    }
    let ux = hx.div(mag);
    let uy = hy.div(mag);
    let nx = w.x + ux.mul(speed);
    let ny = w.y + uy.mul(speed);
    let ncoord = Coord3::ground(floor_i32(nx), floor_i32(ny));
    if !terrain.passable(ncoord, &w.body) {
        return false;
    }
    w.x = nx;
    w.y = ny;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{BodyPlan, Part, Temperament};
    use crate::controller::ControllerLayout;
    use crate::homeostasis::{
        AffordanceRegistry, HomeostaticAxisDef, HomeostaticRegistry, ENERGY, WATER,
    };

    const SEED: u64 = 0x10C0;

    struct OpenGround;
    impl Terrain for OpenGround {
        fn passable(&self, _c: Coord3, _b: &BodyPlan) -> bool {
            true
        }
        fn cost(&self, _c: Coord3) -> Fixed {
            Fixed::ONE
        }
    }

    struct Walled;
    impl Terrain for Walled {
        fn passable(&self, c: Coord3, _b: &BodyPlan) -> bool {
            c.x != 5
        }
        fn cost(&self, _c: Coord3) -> Fixed {
            Fixed::ONE
        }
    }

    /// A registry with only a water axis, so movement tests are not confounded by energy starvation
    /// (a labelled test fixture, not owner canon).
    fn water_reg() -> HomeostaticRegistry {
        HomeostaticRegistry {
            axes: vec![HomeostaticAxisDef {
                id: WATER,
                name: "water".to_string(),
                backing_component: Some("bio.water_fraction".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::from_ratio(1, 300),
                exertion_drain: Fixed::from_ratio(1, 400),
                death_floor: Fixed::ZERO,
            }],
        }
    }

    fn layout_for(reg: &HomeostaticRegistry) -> ControllerLayout {
        ControllerLayout::new(reg, &AffordanceRegistry::dev_default(), 0)
    }

    /// A taxis controller for a single target axis whose input block starts at `base`: it moves
    /// toward the known source when away from it and ingests the matter underfoot when the reserve is
    /// low. Output layout: [move_act, move_dx, move_dy, ingest_act].
    fn taxis_controller(l: &ControllerLayout, base: usize) -> Controller {
        let n_in = l.n_in();
        let bias = n_in - 1;
        let (lvl, here, dx, dy) = (base, base + 1, base + 2, base + 3);
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        // move_act (output 0): wants to move (bias), suppressed when the source is underfoot.
        w[bias] = Fixed::ONE;
        w[here] = Fixed::from_int(-1);
        // move_dx / move_dy (outputs 1, 2): follow the source direction.
        w[n_in + dx] = Fixed::ONE;
        w[2 * n_in + dy] = Fixed::ONE;
        // ingest_act (output 3): fire when the source is underfoot and the reserve is low.
        w[3 * n_in + here] = Fixed::ONE;
        w[3 * n_in + lvl] = Fixed::from_int(-1);
        Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
    }

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
            locomotion: vec![1], // a mobile mode (not the rooted mark 0), so it can walk
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

    /// A rooted body carries only the rooted mark, so it cannot walk, whatever its kingdom.
    fn rooted_body() -> BodyPlan {
        let mut b = mobile_body();
        b.locomotion = vec![ROOTED_MODE];
        b
    }

    /// A walking tree: an autotroph body that nonetheless bears a mobile locomotion organ, so it
    /// walks. Mobility is the body, not the kingdom.
    fn walking_tree_body() -> BodyPlan {
        let mut b = mobile_body();
        b.locomotion = vec![3];
        b
    }

    /// A walker with a taxis-for-water controller over the water-only registry, pre-drained so it is
    /// thirsty enough to drink on arrival.
    fn water_walker(
        id: u64,
        tile: Coord3,
        body: BodyPlan,
    ) -> (
        Walker,
        HomeostaticRegistry,
        ControllerLayout,
        AffordanceRegistry,
    ) {
        let reg = water_reg();
        let afford = AffordanceRegistry::dev_default();
        let l = layout_for(&reg);
        let c = taxis_controller(&l, 0); // water is axis 0 in this registry
        let mut homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
        for _ in 0..120 {
            homeo.metabolize(&reg, Fixed::ZERO); // grow thirsty
        }
        (
            Walker::new(StableId(id), tile, body, homeo, c),
            reg,
            l,
            afford,
        )
    }

    #[test]
    fn a_rooted_body_never_moves_however_thirsty() {
        let (mut wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), rooted_body());
        wk.learn(WATER, Coord3::ground(2, 0));
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.add(WATER, Coord3::ground(2, 0));
        let p = LocomotionParams::dev_default();
        let start = ws[0].coord();
        for t in 0..40 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
        }
        assert_eq!(
            ws[0].coord(),
            start,
            "a rooted body stays put whatever its kingdom"
        );
    }

    #[test]
    fn a_walking_tree_walks_because_its_body_can() {
        let (mut wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), walking_tree_body());
        wk.learn(WATER, Coord3::ground(6, 0));
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.add(WATER, Coord3::ground(6, 0));
        let p = LocomotionParams::dev_default();
        let start = ws[0].coord();
        for t in 0..60 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
        }
        assert_ne!(
            ws[0].coord(),
            start,
            "a walking tree moves: mobility is the body, not the kingdom"
        );
    }

    #[test]
    fn a_being_walks_to_water_it_knows_of_and_drinks() {
        let (mut wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), mobile_body());
        wk.learn(WATER, Coord3::ground(9, 0)); // it has seen this water before
        let thirst_before = wk.homeostasis.level(WATER);
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.add(WATER, Coord3::ground(9, 0));
        let p = LocomotionParams::dev_default();
        let mut reached = false;
        for t in 0..80 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
            if ws[0].coord() == Coord3::ground(9, 0) {
                reached = true;
                // give it a few ticks to drink
                for t2 in t + 1..t + 6 {
                    step(
                        &mut ws,
                        &reg,
                        &l,
                        &afford,
                        &OpenGround,
                        &field,
                        &p,
                        SEED,
                        t2,
                    );
                }
                break;
            }
        }
        assert!(reached, "the being walked to the water it knew of");
        assert!(
            ws[0].homeostasis.level(WATER) > thirst_before,
            "and drank, restoring its water"
        );
    }

    #[test]
    fn a_being_does_not_head_for_water_it_has_never_perceived() {
        // Non-omniscience: water sits far, out of sensory range; the being has never seen it, so on
        // its first step it explores rather than making a beeline for water it cannot know of.
        let (wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), mobile_body());
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.add(WATER, Coord3::ground(40, 0));
        let p = LocomotionParams::dev_default();
        assert!(
            !ws[0].known.contains_key(&WATER),
            "it starts knowing of no water"
        );
        step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, 0);
        assert!(
            ws[0]
                .known
                .get(&WATER)
                .map(|s| s.is_empty())
                .unwrap_or(true),
            "it did not learn of water outside its senses"
        );
        assert!(
            ws[0].coord().x < 5,
            "it did not make a beeline for water it cannot know about"
        );
    }

    #[test]
    fn a_being_discovers_water_by_exploring_then_drinks() {
        // The being knows of no water, but a band of water is reachable. Left to explore, it should
        // come within sensory range of some, learn it, walk to it, and slake its thirst.
        let (wk, reg, l, afford) = water_walker(1, Coord3::ground(4, 4), mobile_body());
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        for x in 6..=10 {
            field.add(WATER, Coord3::ground(x, 3));
            field.add(WATER, Coord3::ground(x, 4));
        }
        let p = LocomotionParams::dev_default();
        let mut learned = false;
        let mut drank = false;
        let start_thirst = ws[0].homeostasis.level(WATER);
        for t in 0..600 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
            if ws[0].known.get(&WATER).is_some_and(|s| !s.is_empty()) {
                learned = true;
            }
            if learned && ws[0].homeostasis.level(WATER) > start_thirst {
                drank = true;
                break;
            }
        }
        assert!(
            learned,
            "the being discovered water by exploring, not by reading the map"
        );
        assert!(drank, "and having found it, drank");
    }

    #[test]
    fn perception_is_local_not_global() {
        let (wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), mobile_body());
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.add(WATER, Coord3::ground(2, 0)); // within sense range of the origin
        field.add(WATER, Coord3::ground(40, 0)); // far outside it
        let p = LocomotionParams::dev_default();
        step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, 0);
        let known = ws[0].known.get(&WATER).cloned().unwrap_or_default();
        assert!(
            known.contains(&Coord3::ground(2, 0)),
            "it perceived the near water"
        );
        assert!(
            !known.contains(&Coord3::ground(40, 0)),
            "it did not perceive the far water"
        );
    }

    #[test]
    fn a_wall_blocks_a_straight_line_mover() {
        let (mut wk, reg, l, afford) = water_walker(1, Coord3::ground(0, 0), mobile_body());
        wk.learn(WATER, Coord3::ground(9, 0));
        let mut ws = vec![wk];
        let mut field = ResourceField::new();
        field.add(WATER, Coord3::ground(9, 0));
        let p = LocomotionParams::dev_default();
        for t in 0..80 {
            step(&mut ws, &reg, &l, &afford, &Walled, &field, &p, SEED, t);
        }
        assert!(
            ws[0].coord().x < 5,
            "the wall stops the straight-line mover short of the water"
        );
    }

    #[test]
    fn locomotion_replays_bit_identically() {
        let run = || {
            let reg = water_reg();
            let afford = AffordanceRegistry::dev_default();
            let l = layout_for(&reg);
            let c = taxis_controller(&l, 0);
            let mut field = ResourceField::new();
            for x in 6..=10 {
                field.add(WATER, Coord3::ground(x, 3));
            }
            let mk = |id: u64, tile: Coord3| {
                let mut h = Homeostasis::from_mass(&reg, Fixed::ONE);
                for _ in 0..80 {
                    h.metabolize(&reg, Fixed::ZERO);
                }
                Walker::new(StableId(id), tile, mobile_body(), h, c.clone())
            };
            let mut ws = vec![mk(2, Coord3::ground(0, 0)), mk(1, Coord3::ground(1, 6))];
            let p = LocomotionParams::dev_default();
            for t in 0..80 {
                step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
            }
            (
                ws[0].x.to_bits(),
                ws[0].y.to_bits(),
                ws[1].x.to_bits(),
                ws[1].y.to_bits(),
            )
        };
        assert_eq!(
            run(),
            run(),
            "the same setup, including exploration, replays bit for bit"
        );
    }

    #[test]
    fn metabolism_kills_an_unfed_being() {
        // With the real dev registry (energy and water) and no sources anywhere, a being that never
        // eats or drinks eventually dies: survival is a physical fact, the fitness Stage 3 selects on.
        let reg = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let l = ControllerLayout::new(&reg, &afford, 0);
        let c = taxis_controller(&l, 4); // water block starts at input 4 in the two-axis layout
        let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
        let mut ws = vec![Walker::new(
            StableId(1),
            Coord3::ground(0, 0),
            mobile_body(),
            homeo,
            c,
        )];
        let field = ResourceField::new(); // barren
        let p = LocomotionParams::dev_default();
        let mut died_at = None;
        for t in 0..100_000 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
            if !ws[0].alive {
                died_at = Some(t);
                break;
            }
        }
        assert!(died_at.is_some(), "unfed and unwatered, the being dies");
    }

    #[test]
    fn a_bigger_more_active_body_moves_faster() {
        let p = LocomotionParams::dev_default();
        let mut small = mobile_body();
        small.body_mass = Fixed::from_ratio(1, 16);
        small.temperament.activity = Fixed::from_ratio(1, 4);
        let mut big = mobile_body();
        big.body_mass = Fixed::ONE;
        big.temperament.activity = Fixed::ONE;
        let vs = locomotion_speed(&small, Fixed::ONE, &p);
        let vb = locomotion_speed(&big, Fixed::ONE, &p);
        assert!(
            vb > vs,
            "the larger, more active body has the greater ground speed"
        );
    }

    #[test]
    fn difficult_terrain_slows_a_body() {
        let p = LocomotionParams::dev_default();
        let body = mobile_body();
        let open = locomotion_speed(&body, Fixed::ONE, &p);
        let rough = locomotion_speed(&body, Fixed::from_int(3), &p);
        assert!(rough < open, "costlier ground slows the body");
    }

    #[test]
    fn energy_and_water_both_being_sought_is_the_next_layer() {
        // A sanity check that a two-axis being can be constructed and stepped without panic; the
        // full two-need forage loop is what selection (Stage 3) and the recurrent controller
        // (Stage 4) bring.
        let reg = HomeostaticRegistry::dev_default();
        let afford = AffordanceRegistry::dev_default();
        let l = ControllerLayout::new(&reg, &afford, 0);
        let c = Controller::zeros(&l);
        let homeo = Homeostasis::from_mass(&reg, Fixed::ONE);
        let mut ws = vec![Walker::new(
            StableId(1),
            Coord3::ground(0, 0),
            mobile_body(),
            homeo,
            c,
        )];
        let field = ResourceField::new();
        let p = LocomotionParams::dev_default();
        for t in 0..10 {
            step(&mut ws, &reg, &l, &afford, &OpenGround, &field, &p, SEED, t);
        }
        assert!(
            ws[0].alive,
            "a two-axis being steps without dying over a short unfed horizon"
        );
        assert_eq!(
            reg.axes.len(),
            2,
            "the dev registry carries both energy and water axes"
        );
        let _ = (ENERGY, WATER);
    }
}
