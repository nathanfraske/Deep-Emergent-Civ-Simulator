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

//! Emergent locomotion on a generated world: a few beings walk toward what they need, and the
//! walking is not scripted. Run with `cargo run -p civsim-sim --example walkers`.
//!
//! The physics is authored (Principle 9): each being's ground speed comes from its body (mass,
//! activity, having legs at all) and the terrain it crosses (deep water blocks it, higher ground
//! is costlier), read through the `Terrain` trait this example implements over the map. Where each
//! being goes and why is not authored: its thirst and hunger rise, the decision layer picks the
//! pressing one, and it walks toward the nearest shoreline (to drink) or grassland or forest (to
//! forage) it can perceive. A thirsty being heads for water, a hungry one for forage, because that
//! is where the drive is relieved, not because any rule says so. The whole run is deterministic and
//! replays bit for bit from the seed.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId};
use civsim_sim::anatomy::{BodyPlan, Part, Temperament};
use civsim_sim::decision::{
    ActionDef, ActionId, Behaviour, Consideration, Curve, DriveDef, DriveId,
};
use civsim_sim::locomotion::{self, LocomotionParams, ResourceField, Terrain, Walker};
use civsim_world::{BiomeSet, Coord3, FlatBounded, TileMap, WorldgenParams};

const THIRST: DriveId = DriveId(0);
const HUNGER: DriveId = DriveId(1);
/// The locomotion-mode id a swimming body would carry; a body without it cannot cross deep water.
const SWIM: u16 = 7;

/// The map as terrain the movement physics reads: deep water blocks a non-swimmer, and higher
/// ground costs more to cross. Pure physical facts about the tiles, no route and no behaviour.
struct MapTerrain<'a> {
    map: &'a TileMap,
    biomes: &'a BiomeSet,
}

impl Terrain for MapTerrain<'_> {
    fn passable(&self, c: Coord3, body: &BodyPlan) -> bool {
        match self.map.tile(c) {
            Some(t) => {
                let name = self.biomes.name(t.biome);
                if name == "ocean" {
                    body.locomotion.contains(&SWIM) // only a swimmer enters deep water
                } else {
                    true
                }
            }
            None => false, // off the map
        }
    }

    fn cost(&self, c: Coord3) -> Fixed {
        match self.map.tile(c) {
            Some(t) => Fixed::ONE + t.elevation, // higher ground is costlier to cross
            None => Fixed::from_int(100),
        }
    }
}

/// Build the resource field from the world content: shorelines relieve thirst, grassland and
/// forest relieve hunger. Data drawn from the map, not a rule wired into locomotion.
fn resources(map: &TileMap, biomes: &BiomeSet) -> ResourceField {
    let mut field = ResourceField::new();
    let topo = map.topo();
    for y in 0..topo.height {
        for x in 0..topo.width {
            let c = Coord3::ground(x, y);
            if let Some(t) = map.tile(c) {
                match biomes.name(t.biome) {
                    "coast" => field.add(THIRST, c),
                    "grassland" | "forest" => field.add(HUNGER, c),
                    _ => {}
                }
            }
        }
    }
    field
}

/// The behaviour: two drives, each with an action that seeks its satisfier. Thirst rises faster
/// than hunger, as it does in life.
fn behaviour() -> Behaviour {
    let ramp = Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)]);
    Behaviour {
        drives: vec![
            DriveDef {
                id: THIRST,
                rise_per_tick: Fixed::from_ratio(1, 40),
                satisfy_amount: Fixed::from_ratio(9, 10),
            },
            DriveDef {
                id: HUNGER,
                rise_per_tick: Fixed::from_ratio(1, 80),
                satisfy_amount: Fixed::from_ratio(9, 10),
            },
        ],
        curves: vec![ramp],
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

/// A walking body, varied by mass and activity so the beings move at different speeds.
fn body(mass: (i64, i64), activity: (i64, i64)) -> BodyPlan {
    BodyPlan {
        body_mass: Fixed::from_ratio(mass.0, mass.1),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons: vec![],
        covering: Part { kind: 0, development: Fixed::from_ratio(1, 2) },
        senses: vec![],
        locomotion: vec![1], // has legs, so it can walk (but not swim: no SWIM mode)
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(activity.0, activity.1),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    }
}

/// The nearest land tile to the map centre that is grassland or forest, a place a band would start.
fn start_tile(map: &TileMap, biomes: &BiomeSet) -> Coord3 {
    let topo = map.topo();
    let centre = Coord3::ground(topo.width / 2, topo.height / 2);
    let mut best: Option<(i64, Coord3)> = None;
    for y in 0..topo.height {
        for x in 0..topo.width {
            let c = Coord3::ground(x, y);
            if let Some(t) = map.tile(c) {
                let n = biomes.name(t.biome);
                if n == "grassland" || n == "forest" {
                    let dx = (c.x - centre.x) as i64;
                    let dy = (c.y - centre.y) as i64;
                    let d = dx * dx + dy * dy;
                    if best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, c));
                    }
                }
            }
        }
    }
    best.map(|(_, c)| c).unwrap_or(centre)
}

/// Draw the map with the beings on it (each a letter), the rest the biome glyphs.
fn frame(map: &TileMap, biomes: &BiomeSet, walkers: &[Walker], names: &[char]) {
    let topo = map.topo();
    let mut at: BTreeMap<(i32, i32), char> = BTreeMap::new();
    for (i, w) in walkers.iter().enumerate() {
        let c = w.coord();
        at.insert((c.x, c.y), names.get(i).copied().unwrap_or('?'));
    }
    for y in 0..topo.height {
        let mut line = String::with_capacity(topo.width as usize);
        for x in 0..topo.width {
            if let Some(&ch) = at.get(&(x, y)) {
                line.push(ch);
            } else if let Some(t) = map.tile(Coord3::ground(x, y)) {
                line.push(biomes.glyph(t.biome));
            } else {
                line.push(' ');
            }
        }
        println!("{line}");
    }
}

/// A short bar for a unit-interval level.
fn bar(level: Fixed) -> String {
    let n = ((level.to_bits() * 10) >> 32).clamp(0, 10) as usize;
    format!("{}{}", "#".repeat(n), "-".repeat(10 - n))
}

fn main() {
    let seed = 0x5A11_u64;
    let (w, h) = (56, 22);
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(seed, FlatBounded::new(w, h, 1), &biomes, &WorldgenParams::dev_default());
    let field = resources(&map, &biomes);
    let terrain = MapTerrain { map: &map, biomes: &biomes };
    let b = behaviour();
    let p = LocomotionParams::dev_default();

    // A small band, bodies of different mass and activity, started on the same home tile, primed
    // with a little thirst and hunger so they set off at once.
    let home = start_tile(&map, &biomes);
    let names = ['A', 'B', 'C', 'D'];
    let mut walkers = vec![
        Walker::new(StableId(1), home, body((3, 4), (3, 4))),
        Walker::new(StableId(2), home, body((1, 4), (1, 2))),
        Walker::new(StableId(3), home, body((9, 10), (9, 10))),
        Walker::new(StableId(4), home, body((1, 2), (2, 5))),
    ];
    for wk in &mut walkers {
        wk.drives.insert(THIRST, Fixed::from_ratio(4, 10));
        wk.drives.insert(HUNGER, Fixed::from_ratio(3, 10));
    }

    println!(
        "A generated world ({w}x{h}, seed {seed:#x}). A band of {} starts on {} at ({}, {}),",
        walkers.len(),
        biomes.name(map.tile(home).unwrap().biome),
        home.x,
        home.y
    );
    println!("marked A-D. Shorelines (coast) relieve thirst, grassland and forest relieve hunger.");
    println!("They start knowing of nothing: they must perceive or explore to find water and food.");
    println!("Watch them search, discover, and return. Nothing hands them the map.\n");

    let total = 240;
    for tick in 0..=total {
        if tick % 60 == 0 {
            println!("--- tick {tick} (~{tick}s in-world) ---");
            frame(&map, &biomes, &walkers, &names);
            for (i, wk) in walkers.iter().enumerate() {
                println!(
                    "  {}  ({:>2},{:>2})  thirst [{}]  hunger [{}]  energy [{}]",
                    names[i],
                    wk.coord().x,
                    wk.coord().y,
                    bar(wk.drives.get(&THIRST).copied().unwrap_or(Fixed::ZERO)),
                    bar(wk.drives.get(&HUNGER).copied().unwrap_or(Fixed::ZERO)),
                    bar(wk.energy),
                );
            }
            println!();
        }
        if tick < total {
            locomotion::step(&mut walkers, &b, &terrain, &field, &p, seed, tick as u64);
        }
    }

    // Determinism: the run is a pure function of the seed. A rerun lands the band on the same tiles.
    let fingerprint: Vec<(i32, i32)> = walkers.iter().map(|w| (w.coord().x, w.coord().y)).collect();
    println!("Determinism: the band's final tiles are {fingerprint:?}, the same on every run.");
}
