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

//! A windowed, colour viewer onto a generated world (design Part 14). It opens a desktop
//! window (on Windows, macOS, Linux, and WSLg), generates a world deterministically from a
//! seed, and lets you pan and zoom a coloured map of it. Run it with:
//!
//! ```text
//! cargo run -p civsim-viewer
//! cargo run -p civsim-viewer -- 0xBEEF 256 192
//! ```
//!
//! Controls: arrow keys or WASD pan, `+`/`-` (or `=`/`-`) zoom in and out, `Home` recentres,
//! `Esc` or the window close button quits. The window is an observer: it only reads the
//! quadtree and never writes canon (Principle 10), so the same seed always shows the same
//! world and looking at it cannot perturb the simulation.

use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};

use civsim_world::view::Camera;
use civsim_world::{BiomeSet, Coord3, FlatBounded, QuadTree, Rgb, TileMap, WorldgenParams};

/// Pixels per quadtree node. A node is drawn as a square of this many pixels, so zooming
/// changes which level (how coarse a region each node covers), not the on-screen cell size.
const CELL: usize = 8;
/// The empty-space colour painted where the view falls off the world.
const BG: Rgb = Rgb::new(8, 9, 14);

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
    let width: i32 = parse(argv.get(2), 256);
    let height: i32 = parse(argv.get(3), 192);

    // Generate the world and the level-of-detail tree once. Both are deterministic and
    // immutable for the life of the window; only the camera (an observer) changes.
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(
        seed,
        FlatBounded::new(width, height, 1),
        &biomes,
        &WorldgenParams::dev_default(),
    );
    let tree = QuadTree::build(&map);

    let mut win_w = 960usize;
    let mut win_h = 640usize;
    let mut window = Window::new(
        "civsim world viewer",
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
             On WSL this needs WSLg (Windows 11) or an X server. The headless colour map \
             still works: cargo run -p civsim-world --example zoom_map"
        );
        std::process::exit(1);
    });
    window.set_target_fps(30);

    // Start zoomed to the deepest level (per-tile detail), centred on the world.
    let home = Coord3::ground(width / 2, height / 2);
    let mut cam = Camera::new(home, tree.depth());

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Pan by one node per frame while a direction is held, so the pan speed is steady in
        // nodes-per-second at every zoom.
        let side = tree.node_side(cam.level(&tree));
        let mut dx = 0i32;
        let mut dy = 0i32;
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            dx -= side;
        }
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            dx += side;
        }
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            dy -= side;
        }
        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            dy += side;
        }
        cam.center.x += dx;
        cam.center.y += dy;

        // Discrete zoom and recenter on key edges.
        for k in window.get_keys_pressed(KeyRepeat::No) {
            match k {
                Key::Equal | Key::NumPadPlus => cam.zoom = (cam.zoom + 1).min(tree.depth()),
                Key::Minus | Key::NumPadMinus => cam.zoom = cam.zoom.saturating_sub(1),
                Key::Home => cam = Camera::new(home, tree.depth()),
                _ => {}
            }
        }

        // Repaint at the window's current size (it may have been resized).
        let (w, h) = window.get_size();
        if w == 0 || h == 0 {
            window.update();
            continue;
        }
        if (w, h) != (win_w, win_h) {
            win_w = w;
            win_h = h;
        }
        let buf = cam.paint(&tree, &biomes, win_w, win_h, CELL, BG);
        window.set_title(&format!(
            "civsim world viewer  seed 0x{seed:X}  {width}x{height}  zoom {}/{}  centre ({}, {})",
            cam.level(&tree),
            tree.depth(),
            cam.center.x,
            cam.center.y
        ));
        window
            .update_with_buffer(&buf, win_w, win_h)
            .expect("blit the frame");
    }
}
