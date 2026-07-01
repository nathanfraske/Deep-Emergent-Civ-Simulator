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
//! The full R-BIOSPHERE simulation, made visible: it runs the world-genesis sequence
//! (worldgen, then the pre-dawn biosphere epoch, then the dawn-ready living world), prints
//! the coloured map, then zooms to the superfine to list a region's surviving species and the
//! organisms placed on its tiles, and shows the same organism reading as food to one consumer
//! and poison to another. Run it with:
//!
//! ```text
//! cargo run -p civsim-sim --example living_world
//! cargo run -p civsim-sim --example living_world -- 0xBEEF
//! ```

use civsim_core::Fixed;
use civsim_sim::biosphere::SourceRef;
use civsim_sim::edibility::{assess, verdict, Composition, FloorCaps, Physiology};
use civsim_sim::genesis::{genesis, GenesisParams};
use civsim_world::view::whole_map_frame_color;
use civsim_world::{BiomeSet, QuadTree};

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let seed: u64 = argv
        .get(1)
        .and_then(|s| {
            s.strip_prefix("0x")
                .and_then(|h| u64::from_str_radix(h, 16).ok())
                .or_else(|| s.parse().ok())
        })
        .unwrap_or(0x11FE);

    let params = GenesisParams::dev_default();
    let world = genesis(seed, &params);

    println!(
        "seed 0x{seed:X}  world {}x{}  regions {}  species {} (alive {})  living-world hash {:032x}",
        params.width,
        params.height,
        world.regions.len(),
        world.species(),
        world.alive(),
        world.state_hash(),
    );
    let daughters: u32 = world.regions.values().map(|r| r.report.daughters).sum();
    let extinctions: u32 = world.regions.values().map(|r| r.report.extinctions).sum();
    println!(
        "the pre-dawn epoch ran {} generations: {daughters} daughter species radiated, {extinctions} went extinct\n",
        params.epoch.generations
    );

    // The large-scale coloured map.
    let biomes = BiomeSet::dev_default();
    let tree = QuadTree::build(&world.map);
    println!("== the world (colour overview) ==");
    print!("{}", whole_map_frame_color(&tree, &biomes, tree.depth().min(5)));
    println!();

    // Zoom to one region: its surviving species and the organisms on its tiles.
    if let Some(((rx, ry), rb)) = world.regions.iter().find(|(_, r)| r.report.alive > 0) {
        println!("== superfine: region ({rx}, {ry}) ==");
        let mut shown = 0;
        for id in rb.biosphere.species.ids() {
            let sp = rb.biosphere.species.get(id).unwrap();
            if sp.extinct {
                continue;
            }
            let fit = sp.niche.suitability(&rb.region.env);
            let draws = match sp.draws_on.first() {
                Some(SourceRef::Abiotic(a)) => format!("abiotic source {a}"),
                Some(SourceRef::Species(dep)) => format!("species {}", dep.0),
                None => "nothing".to_string(),
            };
            println!(
                "  species {:>3}  trophic layer {}  biome-fit {:.2}  draws on {}",
                id.0,
                sp.layer,
                fit.to_f64_lossy(),
                draws
            );
            shown += 1;
            if shown >= 8 {
                println!("  ...");
                break;
            }
        }
        let occupied: Vec<_> = world.occupants.occupied().take(6).collect();
        println!("\n  organisms placed on the map (first {} tiles):", occupied.len());
        for coord in occupied {
            let occ = world.occupants.occupants(coord);
            println!("    tile ({}, {}): {} organism(s)", coord.x, coord.y, occ.len());
        }
    }

    // The relational edibility: one organism, two consumers.
    println!("\n== edibility is a relation (the same berry) ==");
    let caps = FloorCaps::dev_default();
    let berry = Composition {
        nutrients: vec![Fixed::from_ratio(6, 10), Fixed::from_ratio(4, 10)],
        toxins: vec![Fixed::from_ratio(6, 10)],
    };
    let tolerant = Physiology {
        requirements: vec![Fixed::from_ratio(3, 10), Fixed::from_ratio(3, 10)],
        tolerances: vec![Some(Fixed::from_ratio(9, 10))],
        hill: vec![2],
    };
    let sensitive = Physiology {
        requirements: vec![Fixed::from_ratio(3, 10), Fixed::from_ratio(3, 10)],
        tolerances: vec![Some(Fixed::from_ratio(2, 100))],
        hill: vec![2],
    };
    for (who, phys) in [("a tolerant eater", &tolerant), ("a sensitive eater", &sensitive)] {
        let e = assess(&berry, phys, &caps);
        let v = verdict(&e, Fixed::from_ratio(1, 10), Fixed::from_ratio(1, 2));
        println!(
            "  to {who}: nutrition {:.2}, harm {:.2}  =>  {:?}",
            e.net_nutrition.to_f64_lossy(),
            e.net_harm.to_f64_lossy(),
            v
        );
    }
}
