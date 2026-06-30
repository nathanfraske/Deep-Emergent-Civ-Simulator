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

//! A headless colour snapshot of a generated world: it paints the whole map through the same
//! [`Camera::paint`] read the window uses and writes it as a binary PPM (`P6`), so the
//! coloured map can be inspected without a display. Run it with:
//!
//! ```text
//! cargo run -p civsim-world --example snapshot -- out.ppm 0xEA27 96 64 6
//! ```
//!
//! Arguments (all optional): output path, seed, width, height, pixels-per-tile.

use std::io::Write;

use civsim_world::view::Camera;
use civsim_world::{BiomeSet, Coord3, FlatBounded, QuadTree, Rgb, TileMap, WorldgenParams};

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
    let path = argv.get(1).cloned().unwrap_or_else(|| "world.ppm".to_string());
    let seed: u64 = parse(argv.get(2), 0xEA27);
    let width: i32 = parse(argv.get(3), 96);
    let height: i32 = parse(argv.get(4), 64);
    let cell: usize = parse(argv.get(5), 6);

    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(
        seed,
        FlatBounded::new(width, height, 1),
        &biomes,
        &WorldgenParams::dev_default(),
    );
    let tree = QuadTree::build(&map);

    // Per-tile zoom, centred, with the buffer sized so the whole map fills it exactly.
    let cam = Camera::new(Coord3::ground(width / 2, height / 2), tree.depth());
    let px_w = width as usize * cell;
    let px_h = height as usize * cell;
    let buf = cam.paint(&tree, &biomes, px_w, px_h, cell, Rgb::new(8, 9, 14));

    let mut out = Vec::with_capacity(px_w * px_h * 3 + 32);
    out.extend_from_slice(format!("P6\n{px_w} {px_h}\n255\n").as_bytes());
    for word in &buf {
        out.push((word >> 16) as u8);
        out.push((word >> 8) as u8);
        out.push(*word as u8);
    }
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(&out))
        .expect("write the PPM snapshot");
    println!("wrote {path} ({px_w}x{px_h}, map hash {:032x})", map.state_hash());
}
