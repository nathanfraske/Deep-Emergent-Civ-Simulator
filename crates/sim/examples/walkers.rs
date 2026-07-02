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

//! QUARANTINED DEV-FIXTURE HARNESS (not canonical). This example uses authored, dev-fixture numbers
//! (calibrations, seeds, scenario values) to produce a result for demonstration and testing only, and
//! its behaviour is not authoritative (design Principle 11, the reserved-value discipline: an authored
//! constant in the path of world content is a defect until it earns its place). The canonical runner
//! is manifest-driven and fail-loud with zero unapproved authored features; see docs/QUARANTINE.md.
//!
//! Evolved locomotion on a generated world: beings forage for what they need, and the behaviour is
//! not scripted. Run with `cargo run -p civsim-sim --example walkers`.
//!
//! The physics is authored (Principle 9): each being's ground speed comes from its body (mass,
//! activity, having legs at all) and the terrain it crosses (deep water blocks it, higher ground is
//! costlier), read through the `Terrain` trait this example implements over the map; its need is a
//! homeostatic water reserve that drains by metabolism; its option is the affordance its morphology
//! permits. What is not authored is the behaviour: which affordance it issues, and where it aims it,
//! is its controller (`civsim_sim::controller`), a heritable policy expressed from its genome and,
//! under the pre-dawn epoch, selected by survival. Here two controllers stand side by side: a
//! forager whose weights make it walk to known water and drink when dry, and a blank one whose
//! weights are all zero. Nobody wrote "seek water": the forager's weights did, and the blank one,
//! wanting nothing, dies of thirst. The forager's weights are exactly what Stage 3's homeostatic
//! selection produces from random starts. The whole run is deterministic and replays bit for bit.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId};
use civsim_sim::anatomy::{BodyPlan, Part, Temperament};
use civsim_sim::controller::{Controller, ControllerLayout};
use civsim_sim::homeostasis::{
    AffordanceRegistry, Homeostasis, HomeostaticAxisDef, HomeostaticRegistry, WATER,
};
use civsim_sim::locomotion::{self, LocomotionParams, ResourceField, Terrain, Walker};
use civsim_world::{BiomeSet, Coord3, FlatBounded, TileMap, WorldgenParams};

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
                if self.biomes.name(t.biome) == "ocean" {
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

/// A water-only physiology, so the demo turns on the one need without energy-starvation confounds
/// (a labelled development fixture, not owner canon).
fn water_reg() -> HomeostaticRegistry {
    HomeostaticRegistry {
        axes: vec![HomeostaticAxisDef {
            id: WATER,
            name: "water".to_string(),
            backing_component: Some("bio.water_fraction".to_string()),
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::from_ratio(1, 120),
            exertion_drain: Fixed::from_ratio(1, 300),
            death_floor: Fixed::ZERO,
        }],
    }
}

/// Where water is on the map: the shorelines. Drawn from the world content, not a rule in locomotion.
fn resources(map: &TileMap, biomes: &BiomeSet) -> ResourceField {
    let mut field = ResourceField::new();
    let topo = map.topo();
    for y in 0..topo.height {
        for x in 0..topo.width {
            let c = Coord3::ground(x, y);
            if let Some(t) = map.tile(c) {
                if biomes.name(t.biome) == "coast" {
                    field.add(WATER, c);
                }
            }
        }
    }
    field
}

/// A forager controller: move toward known water while away from it, and drink the water underfoot
/// when the reserve is low. Water is axis 0 in the water-only registry, so its input block is
/// [level, here, dir_x, dir_y] at indices 0..3, with the bias at index 4; the outputs are
/// [move_act, move_dx, move_dy, ingest_act]. These weights are the kind selection converges on.
fn forager(l: &ControllerLayout) -> Controller {
    let n_in = l.n_in();
    let bias = n_in - 1;
    let mut w = vec![Fixed::ZERO; l.weight_count()];
    w[bias] = Fixed::ONE; // move_act: wants to move,
    w[1] = Fixed::from_int(-1); //         but not off the water underfoot (here flag)
    w[n_in + 2] = Fixed::ONE; // move_dx follows the water direction
    w[2 * n_in + 3] = Fixed::ONE; // move_dy follows the water direction
    w[3 * n_in + 1] = Fixed::ONE; // ingest_act: fire when water is underfoot
    w[3 * n_in] = Fixed::from_int(-1); //          and the reserve is low
    Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
}

/// A walking body, varied by mass and activity so the beings move at different speeds.
fn body(mass: (i64, i64), activity: (i64, i64)) -> BodyPlan {
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
        locomotion: vec![1], // has legs, so it can walk (but not swim: no SWIM mode)
        organs: vec![],
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

/// Draw the map with the beings on it (each a letter, a dead one lowercased), the rest biome glyphs.
fn frame(map: &TileMap, biomes: &BiomeSet, walkers: &[Walker], names: &[char]) {
    let topo = map.topo();
    let mut at: BTreeMap<(i32, i32), char> = BTreeMap::new();
    for (i, w) in walkers.iter().enumerate() {
        let c = w.coord();
        let ch = names.get(i).copied().unwrap_or('?');
        at.insert(
            (c.x, c.y),
            if w.alive { ch } else { ch.to_ascii_lowercase() },
        );
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
    let map = TileMap::generate(
        seed,
        FlatBounded::new(w, h, 1),
        &biomes,
        &WorldgenParams::dev_default(),
    );
    let field = resources(&map, &biomes);
    let terrain = MapTerrain {
        map: &map,
        biomes: &biomes,
    };
    let reg = water_reg();
    let afford = AffordanceRegistry::dev_default();
    let layout = ControllerLayout::new(&reg, &afford, 0);
    let p = LocomotionParams::dev_default();

    // A small band on one home tile, bodies of different mass and activity. Three carry the forager
    // controller; the fourth (D) carries a blank one, wanting nothing.
    let home = start_tile(&map, &biomes);
    let names = ['A', 'B', 'C', 'D'];
    let full = || Homeostasis::from_mass(&reg, Fixed::ONE);
    let mut walkers = vec![
        Walker::new(
            StableId(1),
            home,
            body((3, 4), (3, 4)),
            full(),
            forager(&layout),
        ),
        Walker::new(
            StableId(2),
            home,
            body((1, 4), (1, 2)),
            full(),
            forager(&layout),
        ),
        Walker::new(
            StableId(3),
            home,
            body((9, 10), (9, 10)),
            full(),
            forager(&layout),
        ),
        Walker::new(
            StableId(4),
            home,
            body((1, 2), (2, 5)),
            full(),
            Controller::zeros(&layout),
        ),
    ];

    println!(
        "A generated world ({w}x{h}, seed {seed:#x}). A band of {} starts on {} at ({}, {}),",
        walkers.len(),
        biomes.name(map.tile(home).unwrap().biome),
        home.x,
        home.y
    );
    println!("marked A-D. Shorelines (coast) bear water. A, B, C carry a forager controller; D a blank one.");
    println!("They start knowing of nothing: they must perceive or explore to find water.");
    println!("Nothing hands them the map, and no rule tells them to seek water: their controllers decide.\n");

    let total = 300;
    for tick in 0..=total {
        if tick % 75 == 0 {
            println!("--- tick {tick} (~{tick}s in-world) ---");
            frame(&map, &biomes, &walkers, &names);
            for (i, wk) in walkers.iter().enumerate() {
                println!(
                    "  {}  ({:>2},{:>2})  water [{}]  {}",
                    names[i],
                    wk.coord().x,
                    wk.coord().y,
                    bar(wk.homeostasis.level(WATER)),
                    if wk.alive { "alive" } else { "DIED of thirst" },
                );
            }
            println!();
        }
        if tick < total {
            locomotion::step(
                &mut walkers,
                &reg,
                &layout,
                &afford,
                &terrain,
                &field,
                &p,
                seed,
                tick as u64,
            );
        }
    }

    let survivors: Vec<char> = walkers
        .iter()
        .enumerate()
        .filter(|(_, w)| w.alive)
        .map(|(i, _)| names[i])
        .collect();
    println!("Survivors: {survivors:?}. The foragers found water and drank; the blank one, wanting nothing, did not.");
    let fingerprint: Vec<(i32, i32)> = walkers.iter().map(|w| (w.coord().x, w.coord().y)).collect();
    println!("Determinism: the band's final tiles are {fingerprint:?}, the same on every run.");
}
