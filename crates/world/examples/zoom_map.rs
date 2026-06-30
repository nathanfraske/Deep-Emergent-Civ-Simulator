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

//! A headless walk down the zoom ladder, so the generated world is visible from the
//! whole-map overview to the per-tile detail. Run it with:
//!
//! ```text
//! cargo run -p civsim-world --example zoom_map
//! cargo run -p civsim-world --example zoom_map -- 0xBEEF 96 64
//! ```
//!
//! It prints the whole world at a few zoom levels (one glyph at the top, then quartering
//! down), then a camera viewport panned across the world at a mid zoom. The same seed
//! always prints the same picture (the quadtree and the worldgen are deterministic).

use civsim_world::view::{whole_map_frame, Camera};
use civsim_world::{BiomeSet, Coord3, FlatBounded, QuadTree, TileMap, WorldgenParams};

fn parse<T: std::str::FromStr>(arg: Option<&String>, default: T) -> T {
    arg.and_then(|s| {
        if let Some(hex) = s.strip_prefix("0x") {
            u64::from_str_radix(hex, 16).ok().and_then(|v| v.to_string().parse().ok())
        } else {
            s.parse().ok()
        }
    })
    .unwrap_or(default)
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let seed: u64 = parse(argv.get(1), 0xEA27);
    let width: i32 = parse(argv.get(2), 96);
    let height: i32 = parse(argv.get(3), 64);

    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(
        seed,
        FlatBounded::new(width, height, 1),
        &biomes,
        &WorldgenParams::dev_default(),
    );
    let tree = QuadTree::build(&map);

    println!(
        "seed 0x{seed:X}  world {width}x{height}  root {} (depth {})  map hash {:032x}",
        tree.root_side(),
        tree.depth(),
        map.state_hash()
    );
    println!("biomes: {}\n", biome_legend(&biomes));

    // The overview ladder: a handful of zoom levels from the whole-world glyph down.
    let depth = tree.depth();
    let rungs = [0u32, depth / 3, (2 * depth) / 3, depth];
    let mut shown = std::collections::BTreeSet::new();
    for &z in &rungs {
        if !shown.insert(z) {
            continue;
        }
        let side = tree.node_side(z);
        println!("== overview at zoom {z} (each glyph covers {side}x{side} tiles) ==");
        print!("{}", whole_map_frame(&tree, &biomes, z));
        println!();
    }

    // A camera viewport: a 48x18 window at a mid zoom, centred on the world.
    let cam = Camera::new(Coord3::ground(width / 2, height / 2), depth.saturating_sub(1));
    println!(
        "== camera viewport 48x18 at zoom {} centred on ({}, {}) ==",
        cam.zoom,
        width / 2,
        height / 2
    );
    print!("{}", cam.frame(&tree, &biomes, 48, 18));
}

/// A one-line legend of the biome glyphs and names in this set.
fn biome_legend(biomes: &BiomeSet) -> String {
    (0..biomes.len())
        .map(|i| {
            let id = civsim_world::BiomeId(i as u16);
            format!("{} {}", biomes.glyph(id), biomes.name(id))
        })
        .collect::<Vec<_>>()
        .join("   ")
}
