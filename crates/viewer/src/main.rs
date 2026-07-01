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

//! A windowed, colour viewer onto a living world (design Parts 14, 1). It opens a desktop
//! window (on Windows, macOS, Linux, and WSLg), runs the world-genesis sequence
//! deterministically from a seed (worldgen, then the pre-dawn biosphere epoch), and lets you
//! zoom from the large-scale coloured map down to the superfine, where the individual plants
//! and animals stand on their tiles. Run it with:
//!
//! ```text
//! cargo run -p civsim-viewer
//! cargo run -p civsim-viewer -- 0xBEEF 96 64
//! ```
//!
//! Controls: arrow keys or WASD pan, `+`/`-` (or `=`/`-`) zoom in and out, `Home` recentres,
//! `Esc` or the window close button quits. Zooming in past the whole-tile view enters the
//! superfine, where organisms are drawn as marks coloured by kind (plants green, herbivores
//! amber, carnivores red). The window is an observer: it reads the living world and never
//! writes it (Principle 10), so the same seed always shows the same world and biosphere.

mod render;

use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};

use civsim_sim::genesis::{genesis, GenesisParams};
use civsim_world::view::Camera;
use civsim_world::{BiomeSet, Coord3, QuadTree, Rgb};

/// Pixels per quadtree node in the overview. Zooming changes which level a node covers, not
/// the on-screen cell size.
const CELL: usize = 8;
/// The empty-space colour painted where the view falls off the world.
const BG: Rgb = Rgb::new(8, 9, 14);
/// How many superfine levels sit past the whole-tile overview (each magnifies the tile more).
const SUPERFINE_LEVELS: u32 = 4;

/// Render a superfine frame of a living world to a binary PPM and exit (a display-free way to
/// see the individual organisms). Centres on the first occupied tile.
fn snapshot(argv: &[String]) {
    use std::io::Write as _;
    let path = argv.get(2).cloned().unwrap_or_else(|| "living.ppm".to_string());
    let seed: u64 = parse(argv.get(3), 0xEA27);
    let mut params = GenesisParams::dev_default();
    params.width = parse(argv.get(4), 96);
    params.height = parse(argv.get(5), 64);
    let living = genesis(seed, &params);
    let biomes = BiomeSet::dev_default();
    let center = Coord3::ground(params.width / 2, params.height / 2);
    let (w, h, tile_px) = (720usize, 480usize, 18usize);
    let buf = render::superfine(&living, &biomes, center, tile_px, w, h, BG);
    let mut out = Vec::with_capacity(w * h * 3 + 32);
    out.extend_from_slice(format!("P6\n{w} {h}\n255\n").as_bytes());
    for word in &buf {
        out.push((word >> 16) as u8);
        out.push((word >> 8) as u8);
        out.push(*word as u8);
    }
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(&out))
        .expect("write the PPM snapshot");
    eprintln!(
        "wrote {path} ({w}x{h}) superfine at ({}, {}); living-world hash {:032x}",
        center.x,
        center.y,
        living.state_hash()
    );
}

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
    // Headless snapshot: `--ppm <path> [seed] [w] [h]` writes a superfine frame and exits, so
    // the living world can be inspected without a display.
    if argv.get(1).map(|s| s == "--ppm").unwrap_or(false) {
        snapshot(&argv);
        return;
    }
    let seed: u64 = parse(argv.get(1), 0xEA27);
    let width: i32 = parse(argv.get(2), 96);
    let height: i32 = parse(argv.get(3), 64);

    // Run the whole world-genesis sequence once: worldgen, then the pre-dawn biosphere epoch.
    // Deterministic and immutable for the life of the window; only the camera changes.
    let mut params = GenesisParams::dev_default();
    params.width = width;
    params.height = height;
    eprintln!("running world genesis (worldgen + pre-dawn biosphere epoch)...");
    let living = genesis(seed, &params);
    eprintln!(
        "living world: {} regions, {} species ({} alive), hash {:032x}",
        living.regions.len(),
        living.species(),
        living.alive(),
        living.state_hash()
    );
    let biomes = BiomeSet::dev_default();
    let tree = QuadTree::build(&living.map);
    let max_zoom = tree.depth() + SUPERFINE_LEVELS;

    let mut win_w = 960usize;
    let mut win_h = 640usize;
    let mut window = Window::new(
        "civsim living-world viewer",
        win_w,
        win_h,
        WindowOptions {
            resize: true,
            scale: Scale::X1,
            scale_mode: ScaleMode::Stretch,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| {
        eprintln!(
            "could not open a window: {e}\n\
             On WSL this needs WSLg (Windows 11) or an X server. The headless living world \
             still works: cargo run -p civsim-sim --example living_world"
        );
        std::process::exit(1);
    });
    window.set_target_fps(30);

    // Start at the whole-world overview, centred.
    let home = Coord3::ground(width / 2, height / 2);
    let mut cam = Camera::new(home, 0);
    let mut zoom: u32 = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let depth = tree.depth();
        // Pan by one node in the overview, one tile in the superfine, so panning stays steady.
        let step = if zoom <= depth {
            tree.node_side(zoom)
        } else {
            1
        };
        let mut dx = 0i32;
        let mut dy = 0i32;
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            dx -= step;
        }
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            dx += step;
        }
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            dy -= step;
        }
        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            dy += step;
        }
        cam.center.x += dx;
        cam.center.y += dy;

        for k in window.get_keys_pressed(KeyRepeat::No) {
            match k {
                Key::Equal | Key::NumPadPlus => zoom = (zoom + 1).min(max_zoom),
                Key::Minus | Key::NumPadMinus => zoom = zoom.saturating_sub(1),
                Key::Home => {
                    zoom = 0;
                    cam.center = home;
                }
                _ => {}
            }
        }
        cam.zoom = zoom.min(depth);

        let (w, h) = window.get_size();
        if w == 0 || h == 0 {
            window.update();
            continue;
        }
        if (w, h) != (win_w, win_h) {
            win_w = w;
            win_h = h;
        }

        let (buf, mode) = if zoom <= depth {
            (cam.paint(&tree, &biomes, win_w, win_h, CELL, BG), format!("overview {zoom}/{depth}"))
        } else {
            let sf = zoom - depth; // 1..=SUPERFINE_LEVELS
            let tile_px = (6 + 6 * sf) as usize;
            (
                render::superfine(&living, &biomes, cam.center, tile_px, win_w, win_h, BG),
                format!("superfine {sf} ({tile_px}px/tile)"),
            )
        };

        let here = living.occupants.occupants(cam.center).len();
        window.set_title(&format!(
            "civsim living world  seed 0x{seed:X}  {width}x{height}  {mode}  centre ({}, {})  organisms here: {here}",
            cam.center.x, cam.center.y
        ));
        window
            .update_with_buffer(&buf, win_w, win_h)
            .expect("blit the frame");
    }
}
