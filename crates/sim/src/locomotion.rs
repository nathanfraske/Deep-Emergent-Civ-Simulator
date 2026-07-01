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

//! Emergent locomotion: how a being walks, and how it comes to know where to walk (design Part 8,
//! Part 9, Part 13, Part 20, Part 25; Principles 8, 9, 10).
//!
//! Two things are authored, and both are physics. First, a body's capacity to move, derived from
//! its morphology ([`crate::anatomy::BodyPlan`]): a body with no locomotion organ does not walk,
//! whatever its kingdom, so a rooted tree stays put while a body that bears a locomotion organ
//! moves, even an autotroph, so a walking tree walks. Whether a body has that organ is itself an
//! emergent morphological outcome, not a rule keyed on being a plant (see
//! [`crate::anatomy::sample_body_plan`]). Second, the terrain a body crosses (passability and cost,
//! read through the [`Terrain`] trait). Everything else emerges.
//!
//! What emerges, and the correction this module is careful about: a being is not a god and does not
//! read the world map. It knows only what it has perceived. Each tick it perceives satisfiers
//! within a small true sensory range and remembers them ([`Walker::known`]), so its picture of
//! where water and forage are is a belief it has earned by being near them (a lean stand-in for the
//! full perception-and-belief of Part 9, which will also let a being be told of a place it has
//! never seen). Then its drives rise, the decision layer ([`crate::decision::Behaviour`]) picks the
//! pressing one, and it walks toward the nearest satisfier it *knows of*. When it knows of none it
//! explores, moving on a heading drawn from counter-based RNG keyed on the being and the tick
//! ([`civsim_core::Phase::EXPLORE`]), so it discovers the world by moving through it rather than by
//! being handed its coordinates. A thirsty being that has never seen water wanders until it comes
//! within sight of some, remembers it, and can then return to it, which is how knowing where to go
//! is supposed to arise.
//!
//! The mechanism is fixed Rust and fully deterministic: beings are walked in stable-id order,
//! position is exact fixed-point (a subtile fractional coordinate, so movement is smooth within a
//! tile, design.md line 640), every choice keys on the seed, the being, and the tick and never on
//! the camera (Principle 10), and the nearest-target and exploration ties break canonically. What
//! the physics needs is reserved with its basis in [`LocomotionParams`] and defaulted only by a
//! labelled development fixture; the drive rates are the decision layer's data.
//!
//! The load-bearing limit, and the honest one. What is authored physics here is the movement and
//! the perception: how fast a body goes, what ground it can cross, how far it senses, what it
//! remembers. What is *not* yet emergent, and is authored steering, is the policy: the drives and
//! the action menu ([`crate::decision::Behaviour`]) that say a being has a thirst and that a way to
//! relieve it is to seek water. A fixed list of actions a creature may take is a repertoire chosen
//! from outside the simulation, which is exactly the steering Principle 9 forbids at the level of
//! behaviour. The end goal is that the policy is not authored at all: a being's homeostatic state
//! (its energy, its water, its integrity) is a consequence of its body's physics, its motor
//! options are the affordances of its morphology, and the mapping from state to motion is a
//! heritable policy expressed from its genome that evolves under selection, so that seeking water
//! when dry is a behaviour the lineage came to have because the ones that did survived, not a rule
//! anyone wrote. This module supplies the physics substrate that such a policy would drive; the
//! [`crate::decision::Behaviour`] it currently consults is a placeholder standing in for the
//! evolved policy, flagged in the backlog as the emergent-behaviour work (R-BEHAVIOR-EVOLVE).
//!
//! Other honest limits of this slice. Perception is a range gate, not yet line of sight or the full
//! belief store of Part 9, and knowledge is never forgotten or shared (being told of a place is the
//! next layer). Movement is straight-line with a passability gate rather than routing around an
//! obstacle (the pathfinding of Part 13). Exploration is an undirected search modulated only by
//! the being's mobility, not yet by an exploration drive or a memory of where it has already been.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::{DrawKey, Fixed, Phase, StableId};
use civsim_world::Coord3;

use crate::anatomy::BodyPlan;
use crate::decision::{Behaviour, DriveId};

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
    /// Metabolic energy spent per tile of distance moved, drawn from the being's reserve. RESERVED.
    /// Basis: the movement energy cost per body mass over distance from the metabolic model of
    /// Part 20, scaled so a being must rest or feed rather than move without limit.
    pub move_cost: Fixed,
    /// Energy restored on reaching and using a satisfier (feeding, drinking, resting). RESERVED.
    /// Basis: the intake-to-expenditure ratio of the same metabolic model.
    pub rest_energy: Fixed,
    /// Within this distance, in tiles, of a target tile's centre the being has arrived. RESERVED.
    /// Basis: a fraction of a tile, the reach at which a being is at the resource.
    pub arrive_radius: Fixed,
    /// The lowest activity factor, so even a sluggish body creeps rather than freezing (the
    /// temperament activity axis scales speed between this floor and one). RESERVED. Basis: the
    /// ratio of a slow gait to a brisk one.
    pub activity_floor: Fixed,
    /// How far, in whole tiles, a being perceives a satisfier: the true sensory range within which
    /// it comes to know of a resource, so knowledge is earned by nearness, not read from the map.
    /// RESERVED. Basis: the perception range the being's sensory morphology and acuity imply
    /// (Part 9); small, a handful of tiles, not the whole world.
    pub sense_range: i64,
    /// How many ticks an exploration heading holds before it is redrawn, so a searching being keeps
    /// a direction rather than jittering in place. RESERVED. Basis: the persistence of a real
    /// search path before it turns.
    pub explore_persistence: u64,
}

impl LocomotionParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values, so locomotion runs and can be tested now.
    pub fn dev_default() -> LocomotionParams {
        LocomotionParams {
            base_speed: Fixed::from_ratio(1, 1),
            terrain_penalty: Fixed::from_ratio(1, 1),
            move_cost: Fixed::from_ratio(1, 100),
            rest_energy: Fixed::from_ratio(1, 2),
            arrive_radius: Fixed::from_ratio(1, 2),
            activity_floor: Fixed::from_ratio(1, 4),
            sense_range: 4,
            explore_persistence: 6,
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

/// Where the satisfiers of each drive really sit on the map, the world's ground truth: a drive
/// maps to the set of tiles that relieve it (water tiles relieve thirst, forage tiles relieve
/// hunger). The world builds this from its content; a being does not get to read it, it can only
/// perceive the tiles near it (see [`step`]). The module never hardcodes which drive seeks what, so
/// a drive that seeks a resource exists only where that pairing is placed (Principle 11).
#[derive(Clone, Debug, Default)]
pub struct ResourceField {
    tiles: BTreeMap<DriveId, BTreeSet<Coord3>>,
}

impl ResourceField {
    /// An empty field.
    pub fn new() -> ResourceField {
        ResourceField::default()
    }

    /// Record that `coord` bears a satisfier of `drive`.
    pub fn add(&mut self, drive: DriveId, coord: Coord3) {
        self.tiles.entry(drive).or_default().insert(coord);
    }

    /// Whether a coordinate bears a satisfier of a drive.
    pub fn satisfies(&self, drive: DriveId, coord: Coord3) -> bool {
        self.tiles.get(&drive).is_some_and(|s| s.contains(&coord))
    }

    /// The drives this field carries satisfiers for.
    pub fn drives(&self) -> impl Iterator<Item = DriveId> + '_ {
        self.tiles.keys().copied()
    }
}

/// A being that occupies the map and can walk: its stable id, its exact position in fractional
/// tile coordinates, its body plan (the physics of how it moves), its current drive levels, its
/// metabolic reserve, and, crucially, its own remembered knowledge of where satisfiers are: a
/// belief earned by perceiving them, not a copy of the world.
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
    /// The current level of each drive, in the unit interval.
    pub drives: BTreeMap<DriveId, Fixed>,
    /// The metabolic reserve, in the unit interval. Movement draws it; feeding and resting restore
    /// it. At zero the being cannot move until it recovers.
    pub energy: Fixed,
    /// What this being knows: the tiles bearing a satisfier of each drive that it has perceived. It
    /// navigates by this, not by the world, so it can only head for a resource it has come to know.
    pub known: BTreeMap<DriveId, BTreeSet<Coord3>>,
}

impl Walker {
    /// A walker placed at the centre of a tile with a full reserve, no drive pressure, and no
    /// knowledge yet: it has seen nothing and must perceive or explore to learn the world.
    pub fn new(id: StableId, tile: Coord3, body: BodyPlan) -> Walker {
        Walker {
            id,
            x: Fixed::from_int(tile.x) + HALF,
            y: Fixed::from_int(tile.y) + HALF,
            body,
            drives: BTreeMap::new(),
            energy: Fixed::ONE,
            known: BTreeMap::new(),
        }
    }

    /// The tile the being currently stands on.
    pub fn coord(&self) -> Coord3 {
        Coord3::ground(floor_i32(self.x), floor_i32(self.y))
    }

    /// Whether this body can walk at all: a body with no locomotion organ is rooted (a fixed tree)
    /// and never moves however its drives read, whatever its kingdom; a body that bears one moves,
    /// even an autotroph, so a walking tree walks. Mobility is the body, never the kingdom.
    pub fn is_mobile(&self) -> bool {
        !self.body.locomotion.is_empty() && self.body.locomotion.iter().any(|&m| m != ROOTED_MODE)
    }

    /// Record that this being now knows of a satisfier of `drive` at `coord`.
    pub fn learn(&mut self, drive: DriveId, coord: Coord3) {
        self.known.entry(drive).or_default().insert(coord);
    }

    /// The nearest satisfier of `drive` this being knows of, to where it stands, by squared
    /// distance with a canonical tie-break. `None` if it knows of none.
    fn nearest_known(&self, drive: DriveId) -> Option<Coord3> {
        let from = self.coord();
        self.known.get(&drive)?.iter().copied().min_by_key(|c| {
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

/// Floor a fractional tile coordinate to its integer tile (arithmetic shift floors negatives too;
/// Q32.32 fixed point).
fn floor_i32(v: Fixed) -> i32 {
    (v.to_bits() >> 32) as i32
}

/// The physics of a body's ground speed on a tile, in tiles per tick. It rises with body size
/// (an allometric square-root of mass, larger bodies taking longer strides), scales with the
/// temperament activity axis between the reserved floor and one, and is divided down by terrain
/// cost above open ground. A body with no locomotion organ, or with no reserve, does not move.
pub fn locomotion_speed(body: &BodyPlan, energy: Fixed, terrain_cost: Fixed, p: &LocomotionParams) -> Fixed {
    let mobile = !body.locomotion.is_empty() && body.locomotion.iter().any(|&m| m != ROOTED_MODE);
    if !mobile || energy <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    // Allometric size factor: sqrt(body_mass) in [0, 1], so a bigger body strides faster.
    let size = body.body_mass.clamp(Fixed::ZERO, Fixed::ONE).sqrt();
    // Activity factor between the reserved floor and one.
    let activity = p.activity_floor + (Fixed::ONE - p.activity_floor).mul(body.temperament.activity.clamp(Fixed::ZERO, Fixed::ONE));
    // Terrain divisor: 1 + terrain_penalty * (cost - 1), never below one.
    let over = if terrain_cost > Fixed::ONE {
        terrain_cost - Fixed::ONE
    } else {
        Fixed::ZERO
    };
    let divisor = Fixed::ONE + p.terrain_penalty.mul(over);
    let raw = p.base_speed.mul(size).mul(activity);
    let speed = if divisor > Fixed::ZERO { raw.div(divisor) } else { raw };
    speed.clamp(Fixed::ZERO, p.base_speed)
}

/// Perceive the world within the being's sensory range: for each drive the field carries, any
/// satisfier tile within `sense_range` tiles of where the being stands is learned. This is the
/// being seeing what is near it; it learns nothing about tiles beyond its senses.
fn perceive(w: &mut Walker, resources: &ResourceField, range: i64) {
    let here = w.coord();
    let drives: Vec<DriveId> = resources.drives().collect();
    for drive in drives {
        for dy in -range..=range {
            for dx in -range..=range {
                let c = Coord3::ground(here.x + dx as i32, here.y + dy as i32);
                if resources.satisfies(drive, c) {
                    w.learn(drive, c);
                }
            }
        }
    }
}

/// Advance every being one tick of locomotion. Each perceives nearby satisfiers into its memory,
/// its drives rise, the decision layer picks the pressing one, and it walks toward the nearest
/// satisfier it knows of; knowing of none, it explores to discover. Deterministic: beings are
/// walked in stable-id order, exploration keys on `(seed, being, tick)`, and every step is exact
/// fixed-point. Returns the number of beings that moved this tick.
pub fn step<T: Terrain>(
    walkers: &mut [Walker],
    behaviour: &Behaviour,
    terrain: &T,
    resources: &ResourceField,
    p: &LocomotionParams,
    seed: u64,
    tick: u64,
) -> usize {
    walkers.sort_by_key(|w| w.id);
    let mut moved = 0usize;
    for w in walkers.iter_mut() {
        // Perceive first, so knowledge gained this tick is available to this tick's decision.
        perceive(w, resources, p.sense_range);
        // Drives rise on their reserved rates (the decision layer's data).
        for d in &behaviour.drives {
            let lvl = w.drives.entry(d.id).or_insert(Fixed::ZERO);
            *lvl = (*lvl + d.rise_per_tick).clamp(Fixed::ZERO, Fixed::ONE);
        }
        if !w.is_mobile() {
            continue; // a rooted body never moves, however its drives read
        }
        // The pressing action, chosen exactly as the decision layer chooses (emergent from drives).
        let action_id = match behaviour.choose(&w.drives) {
            Some(a) => a,
            None => continue,
        };
        let satisfies: Vec<DriveId> = match behaviour.action(action_id) {
            Some(def) => def.satisfies.clone(),
            None => continue,
        };
        if satisfies.is_empty() {
            continue;
        }
        // The nearest KNOWN satisfier of the pressing action, across the drives it satisfies.
        let target = satisfies
            .iter()
            .filter_map(|&d| w.nearest_known(d).map(|c| (d, c)))
            .min_by_key(|&(_, c)| {
                let here = w.coord();
                let dx = (c.x - here.x) as i64;
                let dy = (c.y - here.y) as i64;
                (dx * dx + dy * dy, c.x, c.y)
            });
        let cost = terrain.cost(w.coord());
        let speed = locomotion_speed(&w.body, w.energy, cost, p);
        if speed <= Fixed::ZERO {
            continue;
        }
        match target {
            Some((drive, tile)) => {
                if walk_toward(w, tile, speed, terrain, p) {
                    moved += 1;
                }
                if arrived(w, tile, p.arrive_radius) && resources.satisfies(drive, w.coord()) {
                    let satisfy_amount = behaviour
                        .drives
                        .iter()
                        .find(|d| d.id == drive)
                        .map(|d| d.satisfy_amount)
                        .unwrap_or(Fixed::ZERO);
                    if let Some(lvl) = w.drives.get_mut(&drive) {
                        *lvl = sub_floor(*lvl, satisfy_amount);
                    }
                    w.energy = (w.energy + p.rest_energy).clamp(Fixed::ZERO, Fixed::ONE);
                }
            }
            None => {
                // It knows of no satisfier: it explores to discover one, rather than heading for a
                // place it has no way of knowing about.
                if explore(w, terrain, speed, p, seed, tick) {
                    moved += 1;
                }
            }
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
            w.energy = sub_floor(w.energy, p.move_cost.mul(speed));
            return true;
        }
    }
    false // hemmed in on every side
}

/// Whether a walker is within the arrival radius of a target tile's centre.
fn arrived(w: &Walker, target: Coord3, radius: Fixed) -> bool {
    let tx = Fixed::from_int(target.x) + HALF;
    let ty = Fixed::from_int(target.y) + HALF;
    let dx = tx - w.x;
    let dy = ty - w.y;
    let d2 = dx.mul(dx) + dy.mul(dy);
    d2 <= radius.mul(radius)
}

/// Step a walker toward a target tile by up to `speed` tiles, straight line, only entering passable
/// tiles. Returns whether it moved. Snaps to the target centre when within one step of it.
fn walk_toward<T: Terrain>(
    w: &mut Walker,
    target: Coord3,
    speed: Fixed,
    terrain: &T,
    p: &LocomotionParams,
) -> bool {
    let tx = Fixed::from_int(target.x) + HALF;
    let ty = Fixed::from_int(target.y) + HALF;
    let dx = tx - w.x;
    let dy = ty - w.y;
    let dist = (dx.mul(dx) + dy.mul(dy)).sqrt();
    if dist <= p.arrive_radius {
        return false; // already there
    }
    let (nx, ny) = if dist <= speed {
        (tx, ty) // one step reaches the centre
    } else {
        let ux = dx.div(dist);
        let uy = dy.div(dist);
        (w.x + ux.mul(speed), w.y + uy.mul(speed))
    };
    let ncoord = Coord3::ground(floor_i32(nx), floor_i32(ny));
    if !terrain.passable(ncoord, &w.body) {
        return false; // blocked; a straight-line mover holds (routing is Part 13, future)
    }
    w.x = nx;
    w.y = ny;
    let step_dist = if dist <= speed { dist } else { speed };
    w.energy = sub_floor(w.energy, p.move_cost.mul(step_dist));
    true
}

/// Subtract, flooring at zero.
fn sub_floor(a: Fixed, b: Fixed) -> Fixed {
    let r = a - b;
    if r < Fixed::ZERO {
        Fixed::ZERO
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{BodyPlan, Part, Temperament};
    use crate::decision::{ActionDef, ActionId, Behaviour, Consideration, Curve, DriveDef};

    const THIRST: DriveId = DriveId(0);
    const HUNGER: DriveId = DriveId(1);
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

    fn ramp() -> Curve {
        Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)])
    }

    fn behaviour() -> Behaviour {
        Behaviour {
            drives: vec![
                DriveDef {
                    id: THIRST,
                    rise_per_tick: Fixed::from_ratio(1, 100),
                    satisfy_amount: Fixed::from_ratio(9, 10),
                },
                DriveDef {
                    id: HUNGER,
                    rise_per_tick: Fixed::from_ratio(1, 200),
                    satisfy_amount: Fixed::from_ratio(9, 10),
                },
            ],
            curves: vec![ramp()],
            actions: vec![
                ActionDef {
                    id: ActionId(0),
                    weight: Fixed::ONE,
                    considerations: vec![Consideration { drive: THIRST, curve: 0 }],
                    satisfies: vec![THIRST],
                },
                ActionDef {
                    id: ActionId(1),
                    weight: Fixed::ONE,
                    considerations: vec![Consideration { drive: HUNGER, curve: 0 }],
                    satisfies: vec![HUNGER],
                },
            ],
        }
    }

    fn mobile_body() -> BodyPlan {
        BodyPlan {
            body_mass: Fixed::from_ratio(1, 2),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: vec![],
            covering: Part { kind: 0, development: Fixed::from_ratio(1, 2) },
            senses: vec![],
            locomotion: vec![1], // a mobile mode (not the rooted mark 0), so it can walk
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

    /// A walking tree: an autotroph (it could photosynthesise) that nonetheless bears a mobile
    /// locomotion organ, so it walks. Mobility is the body, not the kingdom.
    fn walking_tree_body() -> BodyPlan {
        let mut b = mobile_body();
        b.locomotion = vec![3]; // a mobile mode, on a body one would call a plant
        b
    }

    #[test]
    fn a_rooted_body_never_moves_however_thirsty() {
        let mut field = ResourceField::new();
        field.add(THIRST, Coord3::ground(2, 0));
        let mut w = vec![Walker::new(StableId(1), Coord3::ground(0, 0), rooted_body())];
        let b = behaviour();
        let p = LocomotionParams::dev_default();
        let start = w[0].coord();
        for t in 0..50 {
            step(&mut w, &b, &OpenGround, &field, &p, SEED, t);
        }
        assert_eq!(w[0].coord(), start, "a rooted body stays put whatever its kingdom");
    }

    #[test]
    fn a_walking_tree_walks_because_its_body_can() {
        // Same setup, but the body bears a mobile locomotion organ though it is otherwise a plant.
        // It knows where the water is (it has been near it), and it walks there.
        let mut field = ResourceField::new();
        field.add(THIRST, Coord3::ground(6, 0));
        let mut w = vec![Walker::new(StableId(1), Coord3::ground(0, 0), walking_tree_body())];
        w[0].learn(THIRST, Coord3::ground(6, 0));
        w[0].drives.insert(THIRST, Fixed::from_ratio(9, 10));
        let b = behaviour();
        let p = LocomotionParams::dev_default();
        let start = w[0].coord();
        for t in 0..100 {
            step(&mut w, &b, &OpenGround, &field, &p, SEED, t);
        }
        assert_ne!(w[0].coord(), start, "a walking tree moves: mobility is the body, not the kingdom");
    }

    #[test]
    fn a_being_walks_to_water_it_knows_of() {
        let mut field = ResourceField::new();
        field.add(THIRST, Coord3::ground(9, 0));
        let mut w = vec![Walker::new(StableId(1), Coord3::ground(0, 0), mobile_body())];
        w[0].learn(THIRST, Coord3::ground(9, 0)); // it has seen this water before
        w[0].drives.insert(THIRST, Fixed::from_ratio(9, 10));
        let b = behaviour();
        let p = LocomotionParams::dev_default();
        let mut reached = false;
        for t in 0..100 {
            step(&mut w, &b, &OpenGround, &field, &p, SEED, t);
            if w[0].coord() == Coord3::ground(9, 0) {
                reached = true;
                break;
            }
        }
        assert!(reached, "the being walked to the water it knew of");
        assert!(w[0].drives[&THIRST] < Fixed::from_ratio(9, 10), "and was relieved");
    }

    #[test]
    fn a_being_does_not_head_for_water_it_has_never_perceived() {
        // The non-omniscience test: water sits far away, out of sensory range, and the being has
        // never seen it. On its first step it must not teleport toward the water it cannot know of;
        // it explores instead, so it does not close the distance to the water on purpose.
        let mut field = ResourceField::new();
        let water = Coord3::ground(40, 0);
        field.add(THIRST, water);
        let mut w = vec![Walker::new(StableId(1), Coord3::ground(0, 0), mobile_body())];
        w[0].drives.insert(THIRST, Fixed::from_ratio(9, 10));
        let b = behaviour();
        let p = LocomotionParams::dev_default();
        assert!(!w[0].known.contains_key(&THIRST), "it starts knowing of no water");
        step(&mut w, &b, &OpenGround, &field, &p, SEED, 0);
        // It moved (explored) but did not learn the far water, and is not within a step of it.
        assert!(
            w[0].known.get(&THIRST).map(|s| s.is_empty()).unwrap_or(true),
            "it did not learn of water outside its senses"
        );
        assert!(w[0].coord().x < 5, "it did not make a beeline for water it cannot know about");
    }

    #[test]
    fn a_being_discovers_water_by_exploring_then_is_relieved() {
        // The being knows of no water, but water is reachable. Left to explore, it should come
        // within sensory range of the water, learn it, walk to it, and slake its thirst.
        let mut field = ResourceField::new();
        // A short line of water tiles, so an undirected search is likely to sense one within range.
        for x in 6..=10 {
            field.add(THIRST, Coord3::ground(x, 3));
            field.add(THIRST, Coord3::ground(x, 4));
        }
        let mut w = vec![Walker::new(StableId(1), Coord3::ground(4, 4), mobile_body())];
        w[0].drives.insert(THIRST, Fixed::from_ratio(6, 10));
        let b = behaviour();
        let p = LocomotionParams::dev_default();
        let mut learned = false;
        for t in 0..400 {
            step(&mut w, &b, &OpenGround, &field, &p, SEED, t);
            if w[0].known.get(&THIRST).is_some_and(|s| !s.is_empty()) {
                learned = true;
            }
            if learned && w[0].drives[&THIRST] < Fixed::from_ratio(2, 10) {
                break;
            }
        }
        assert!(learned, "the being discovered water by exploring, not by reading the map");
        assert!(w[0].drives[&THIRST] < Fixed::from_ratio(4, 10), "and having found it, drank");
    }

    #[test]
    fn the_pressing_drive_chooses_among_known_destinations() {
        // It knows of both water (east) and forage (west). Very thirsty and barely hungry, it heads
        // east toward the water: the destination is chosen by the pressing drive.
        let mut field = ResourceField::new();
        field.add(THIRST, Coord3::ground(9, 0));
        field.add(HUNGER, Coord3::ground(0, 0));
        let mut w = vec![Walker::new(StableId(1), Coord3::ground(5, 0), mobile_body())];
        w[0].learn(THIRST, Coord3::ground(9, 0));
        w[0].learn(HUNGER, Coord3::ground(0, 0));
        w[0].drives.insert(THIRST, Fixed::from_ratio(95, 100));
        w[0].drives.insert(HUNGER, Fixed::from_ratio(10, 100));
        let b = behaviour();
        let p = LocomotionParams::dev_default();
        step(&mut w, &b, &OpenGround, &field, &p, SEED, 0);
        assert!(w[0].x > Fixed::from_int(5), "the thirstier being moved east toward known water");
    }

    #[test]
    fn perception_is_local_not_global() {
        let mut field = ResourceField::new();
        field.add(THIRST, Coord3::ground(2, 0)); // within sense range of the origin
        field.add(THIRST, Coord3::ground(40, 0)); // far outside it
        let mut w = vec![Walker::new(StableId(1), Coord3::ground(0, 0), mobile_body())];
        let b = behaviour();
        let p = LocomotionParams::dev_default();
        step(&mut w, &b, &OpenGround, &field, &p, SEED, 0);
        let known = w[0].known.get(&THIRST).cloned().unwrap_or_default();
        assert!(known.contains(&Coord3::ground(2, 0)), "it perceived the near water");
        assert!(!known.contains(&Coord3::ground(40, 0)), "it did not perceive the far water");
    }

    #[test]
    fn a_wall_blocks_a_straight_line_mover() {
        let mut field = ResourceField::new();
        field.add(THIRST, Coord3::ground(9, 0));
        let mut w = vec![Walker::new(StableId(1), Coord3::ground(0, 0), mobile_body())];
        w[0].learn(THIRST, Coord3::ground(9, 0));
        w[0].drives.insert(THIRST, Fixed::from_ratio(9, 10));
        let b = behaviour();
        let p = LocomotionParams::dev_default();
        for t in 0..100 {
            step(&mut w, &b, &Walled, &field, &p, SEED, t);
        }
        assert!(w[0].coord().x < 5, "the wall stops the straight-line mover short of the water");
    }

    #[test]
    fn locomotion_replays_bit_identically() {
        let run = || {
            let mut field = ResourceField::new();
            for x in 6..=10 {
                field.add(THIRST, Coord3::ground(x, 3));
            }
            let mut w = vec![
                Walker::new(StableId(2), Coord3::ground(0, 0), mobile_body()),
                Walker::new(StableId(1), Coord3::ground(1, 6), mobile_body()),
            ];
            let b = behaviour();
            let p = LocomotionParams::dev_default();
            for t in 0..80 {
                step(&mut w, &b, &OpenGround, &field, &p, SEED, t);
            }
            (w[0].x.to_bits(), w[0].y.to_bits(), w[1].x.to_bits(), w[1].y.to_bits())
        };
        assert_eq!(run(), run(), "the same setup, including exploration, replays bit for bit");
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
        let vs = locomotion_speed(&small, Fixed::ONE, Fixed::ONE, &p);
        let vb = locomotion_speed(&big, Fixed::ONE, Fixed::ONE, &p);
        assert!(vb > vs, "the larger, more active body has the greater ground speed");
    }

    #[test]
    fn difficult_terrain_slows_a_body() {
        let p = LocomotionParams::dev_default();
        let body = mobile_body();
        let open = locomotion_speed(&body, Fixed::ONE, Fixed::ONE, &p);
        let rough = locomotion_speed(&body, Fixed::ONE, Fixed::from_int(3), &p);
        assert!(rough < open, "costlier ground slows the body");
    }
}
