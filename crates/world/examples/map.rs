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

//! A generated map, printed so the worldgen is visible rather than only asserted. Run
//! with: `cargo run -p civsim-world --example map`.
//!
//! It generates the ground layer of a `FlatBounded` world from a seed with the
//! development-fixture biome set and worldgen parameters, prints the glyph map, and shows
//! the determinism: the same seed reproduces the same world hash, a different seed differs.
//! Every number here is a labelled fixture, never an owner value.

use civsim_world::{BiomeSet, FlatBounded, TileMap, WorldgenParams};

fn main() {
    let seed = 0x1A2B3Cu64;
    let topo = FlatBounded::new(72, 32, 1);
    let biomes = BiomeSet::dev_default();
    let params = WorldgenParams::dev_default();

    let map = TileMap::generate(seed, topo, &biomes, &params);

    println!(
        "A generated world: {}x{}, seed {seed:#x}.",
        topo.width, topo.height
    );
    println!("Glyphs: ~ ocean  . coast  \" grassland  # forest  : desert  , tundra  ^ mountain  * snowcap\n");
    print!("{}", map.render_glyphs(&biomes));

    // Determinism, shown rather than asserted: the same seed reproduces the same world.
    let again = TileMap::generate(seed, topo, &biomes, &params);
    let other = TileMap::generate(0x999, topo, &biomes, &params);
    println!("\nDeterminism:");
    println!("  state hash this run : {:032x}", map.state_hash());
    println!("  same seed replayed  : {:032x}", again.state_hash());
    println!("  different seed (999) : {:032x}", other.state_hash());
    println!(
        "  same seed matches: {}, different seed differs: {}",
        map.state_hash() == again.state_hash(),
        map.state_hash() != other.state_hash()
    );
}
