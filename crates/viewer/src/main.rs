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

use minifb::{Key, KeyRepeat, MouseMode, Scale, ScaleMode, Window, WindowOptions};

use civsim_sim::anatomy::WorldProfile;
use civsim_sim::clock::PlaybackDriver;
use civsim_sim::genesis::{genesis, GenesisParams, LivingWorld, WorldGenesis};
use civsim_sim::located::OccupantId;
use civsim_world::view::Camera;
use civsim_world::{BiomeSet, Coord3, QuadTree, Rgb};

/// Pixels per quadtree node in the overview. Zooming changes which level a node covers, not
/// the on-screen cell size.
const CELL: usize = 8;
/// The empty-space colour painted where the view falls off the world.
const BG: Rgb = Rgb::new(8, 9, 14);
/// The tile-selector cursor colour.
const CURSOR: Rgb = Rgb::new(255, 240, 90);
/// How many superfine levels sit past the whole-tile overview (each magnifies the tile more).
const SUPERFINE_LEVELS: u32 = 4;
/// The default live playback speed, in radiation generations per real second. A view-side
/// default the observer speeds up or slows down from; it never touches canonical state.
const DEFAULT_GEN_RATE: f64 = 4.0;

/// The selector readout for a tile's occupants: the selected creature inspected in full (its
/// derived trophic label, temperament, natural weapons, covering, and senses, named from the
/// registry), with extra occupants counted.
fn describe_occupants(living: &LivingWorld, occ: &[OccupantId]) -> String {
    if occ.is_empty() {
        return "no organisms".to_string();
    }
    let more = if occ.len() > 1 {
        format!("  +{}", occ.len() - 1)
    } else {
        String::new()
    };
    format!("{}{}", living.describe(occ[0]), more)
}

/// Render a superfine frame of a living world to a binary PPM and exit (a display-free way to
/// see the individual organisms). Centres on the first occupied tile.
fn snapshot(argv: &[String]) {
    use std::io::Write as _;
    let path = argv.get(2).cloned().unwrap_or_else(|| "living.ppm".to_string());
    let seed: u64 = parse(argv.get(3), 0xEA27);
    let mut params = GenesisParams::dev_default();
    params.width = parse(argv.get(4), 96);
    params.height = parse(argv.get(5), 64);
    params.profile = world_profile(argv.get(6));
    let living = genesis(seed, &params);
    let biomes = BiomeSet::dev_default();
    let center = populated_center(&living, params.width, params.height);
    let (w, h, tile_px) = (720usize, 480usize, 18usize);
    let mut buf = render::superfine(&living, &biomes, center, tile_px, w, h, BG);
    // Draw the selector cursor on the centre tile, so the snapshot shows it too.
    let (cols, rows) = ((w / tile_px) as i32, (h / tile_px) as i32);
    let ccol = (cols / 2) as usize;
    let crow = (rows / 2) as usize;
    render::draw_outline(&mut buf, w, ccol * tile_px, crow * tile_px, tile_px, tile_px, CURSOR);
    // The selector readout for the centre tile, drawn on the map like the live viewer.
    let biome = living
        .map
        .tile(center)
        .map(|t| biomes.name(t.biome).to_string())
        .unwrap_or_else(|| "off the world".to_string());
    let occ = living.occupants.occupants(center);
    let detail = format!("tile ({},{})  {biome}  |  {}", center.x, center.y, describe_occupants(&living, &occ));
    render::draw_label(
        &mut buf,
        w,
        h,
        (ccol * tile_px) as i32,
        (crow * tile_px + tile_px + 3) as i32,
        &detail,
        2,
        Rgb::new(245, 245, 245),
        Rgb::new(12, 14, 22),
    );
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

/// Write an RGB pixel buffer as a binary PPM.
fn write_ppm(path: &str, w: usize, h: usize, buf: &[u32]) {
    use std::io::Write as _;
    let mut out = Vec::with_capacity(w * h * 3 + 32);
    out.extend_from_slice(format!("P6\n{w} {h}\n255\n").as_bytes());
    for word in buf {
        out.push((word >> 16) as u8);
        out.push((word >> 8) as u8);
        out.push(*word as u8);
    }
    std::fs::File::create(path)
        .and_then(|mut f| f.write_all(&out))
        .expect("write the PPM");
}

/// The occupied tile nearest the map centre, for centring a superfine frame.
fn populated_center(living: &LivingWorld, w: i32, h: i32) -> Coord3 {
    let home = Coord3::ground(w / 2, h / 2);
    living
        .occupants
        .occupied()
        .min_by_key(|c| {
            let dx = (c.x - home.x) as i64;
            let dy = (c.y - home.y) as i64;
            dx * dx + dy * dy
        })
        .unwrap_or(home)
}

/// The test-harness render: `--render <path> <mode> <seed> <w> <h>`, mode overview or
/// superfine, writes a PPM the harness converts and inspects.
fn render_cmd(argv: &[String]) {
    let path = argv.get(2).cloned().unwrap_or_else(|| "frame.ppm".to_string());
    let mode = argv.get(3).map(String::as_str).unwrap_or("overview");
    let seed: u64 = parse(argv.get(4), 0xEA27);
    let mut params = GenesisParams::dev_default();
    params.width = parse(argv.get(5), 256);
    params.height = parse(argv.get(6), 192);
    params.profile = world_profile(argv.get(7));
    let living = genesis(seed, &params);
    let biomes = BiomeSet::dev_default();
    if mode == "superfine" {
        let center = populated_center(&living, params.width, params.height);
        let (w, h, tile_px) = (720usize, 480usize, 18usize);
        let buf = render::superfine(&living, &biomes, center, tile_px, w, h, BG);
        write_ppm(&path, w, h, &buf);
    } else {
        let tree = QuadTree::build(&living.map);
        let cell = (720 / params.width.max(1)).max(1) as usize;
        let (w, h) = (params.width as usize * cell, params.height as usize * cell);
        let cam = Camera::new(
            Coord3::ground(params.width / 2, params.height / 2),
            tree.depth(),
        );
        let buf = cam.paint(&tree, &biomes, w, h, cell, BG);
        write_ppm(&path, w, h, &buf);
    }
    eprintln!("wrote {path} ({mode})");
}

/// The test-harness stats: `--stats <seed> <w> <h>` prints the living world's summary as JSON.
fn stats_cmd(argv: &[String]) {
    let seed: u64 = parse(argv.get(2), 0xEA27);
    let mut params = GenesisParams::dev_default();
    params.width = parse(argv.get(3), 256);
    params.height = parse(argv.get(4), 192);
    params.profile = world_profile(argv.get(5));
    let living = genesis(seed, &params);
    let daughters: u32 = living.regions.values().map(|r| r.report.daughters).sum();
    let extinctions: u32 = living.regions.values().map(|r| r.report.extinctions).sum();
    println!(
        "{{\"seed\":{seed},\"width\":{},\"height\":{},\"regions\":{},\"species\":{},\"alive\":{},\"daughters\":{daughters},\"extinctions\":{extinctions},\"hash\":\"{:032x}\"}}",
        params.width,
        params.height,
        living.regions.len(),
        living.species(),
        living.alive(),
        living.state_hash()
    );
}

/// The headless live-radiation trace: `--radiate [seed] [w] [h] [profile]` steps the staged world
/// genesis one generation at a time and prints the species and survivor counts as the pre-dawn
/// ecology radiates, so the deep-time evolution can be watched unfolding without a display (the
/// coarse, deep-time end of the playback spectrum). It exercises the same `step_once`/`snapshot`
/// path the windowed viewer drives, and it ends bit-identical to a one-shot genesis.
fn radiate_cmd(argv: &[String]) {
    let seed: u64 = parse(argv.get(2), 0xEA27);
    let mut params = GenesisParams::dev_default();
    params.width = parse(argv.get(3), 96);
    params.height = parse(argv.get(4), 64);
    params.profile = world_profile(argv.get(5));
    let mut wg = WorldGenesis::new(seed, &params);
    println!(
        "staged genesis: {} regions, {} founder species over {} generations",
        wg.snapshot().regions.len(),
        wg.species(),
        wg.generations_planned()
    );
    println!(
        "one generation ~= {} in-world years (owner-set)",
        civsim_sim::YEARS_PER_GENERATION
    );
    println!("gen   ~kyr  species  alive  daughters  extinctions");
    loop {
        let snap = wg.snapshot();
        let daughters: u32 = snap.regions.values().map(|r| r.report.daughters).sum();
        let extinctions: u32 = snap.regions.values().map(|r| r.report.extinctions).sum();
        println!(
            "{:>3}  {:>5}  {:>7}  {:>5}  {:>9}  {:>11}",
            wg.generation(),
            wg.generation() * civsim_sim::YEARS_PER_GENERATION / 1000,
            wg.species(),
            wg.alive(),
            daughters,
            extinctions
        );
        if !wg.step_once() {
            break;
        }
    }
    // Confirm the stepped world matches the one-shot batch genesis bit for bit.
    let stepped = wg.snapshot();
    let batch = genesis(seed, &params);
    let ok = stepped.state_hash() == batch.state_hash();
    println!(
        "final living-world hash {:032x}  ({} batch genesis)",
        stepped.state_hash(),
        if ok { "matches" } else { "DIFFERS FROM" }
    );
}

/// The world profile from a test-world name: Arcanum and Confluence carry magic, Mirror and
/// Tempest (and anything else) are grounded (Part 34, the test worlds).
fn world_profile(name: Option<&String>) -> WorldProfile {
    match name.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("arcanum") | Some("confluence") | Some("magic") | Some("magical") => {
            WorldProfile::magical()
        }
        _ => WorldProfile::grounded(),
    }
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
    // Headless render for the test harness: `--render <path> <mode> <seed> <w> <h>` where mode
    // is overview or superfine. Writes a PPM and exits.
    if argv.get(1).map(|s| s == "--render").unwrap_or(false) {
        render_cmd(&argv);
        return;
    }
    // Headless stats for the test harness: `--stats <seed> <w> <h>` prints JSON and exits.
    if argv.get(1).map(|s| s == "--stats").unwrap_or(false) {
        stats_cmd(&argv);
        return;
    }
    // Headless live-radiation trace: `--radiate [seed] [w] [h]` steps the staged genesis and
    // prints the ecology unfolding generation by generation, then exits.
    if argv.get(1).map(|s| s == "--radiate").unwrap_or(false) {
        radiate_cmd(&argv);
        return;
    }
    // Scripted demo: `--demo [seconds] [seed] [w] [h]` auto-zooms from the whole world into a
    // populated tile, holds, and self-closes, for when interactive control is unavailable.
    let (demo_secs, base) = if argv.get(1).map(|s| s == "--demo").unwrap_or(false) {
        (Some(parse(argv.get(2), 12.0f32)), 3usize)
    } else {
        (None, 1usize)
    };
    let seed: u64 = parse(argv.get(base), 0xEA27);
    let width: i32 = parse(argv.get(base + 1), 256);
    let height: i32 = parse(argv.get(base + 2), 192);

    // Stage world genesis so the pre-dawn radiation can be watched unfolding rather than shown as
    // a finished snapshot: worldgen and the founders are seeded up front, then each frame advances
    // the radiation at the observer's chosen speed. The window is an observer that reads canon and
    // never writes it (Principle 10); the playback is a speed over the deterministic timeline, not
    // a change to it (Part 14.6). Stepped to completion the world is bit-identical to a one-shot
    // genesis.
    let mut params = GenesisParams::dev_default();
    params.width = width;
    params.height = height;
    params.profile = world_profile(argv.get(base + 3));
    eprintln!("staging world genesis (worldgen + founders; the radiation runs live)...");
    let mut wg = WorldGenesis::new(seed, &params);
    // Demo mode has no interactive control, so run the radiation to completion up front and
    // showcase the finished, matured world with the auto-zoom.
    if demo_secs.is_some() {
        while wg.step_once() {}
    }
    let mut living = wg.snapshot();
    eprintln!(
        "staged living world: {} regions, {} species ({} alive) at generation {}/{}",
        living.regions.len(),
        wg.species(),
        wg.alive(),
        wg.generation(),
        wg.generations_planned(),
    );
    let biomes = BiomeSet::dev_default();
    let tree = QuadTree::build(&living.map);
    let max_zoom = tree.depth() + SUPERFINE_LEVELS;

    // The observer's time control: a playback speed over the radiation, with pause and single
    // step, decoupled from the render frame rate. The window redraws at its own fps while the
    // simulation advances by whole generations banked from real elapsed time.
    let mut driver = PlaybackDriver::new(DEFAULT_GEN_RATE);
    let mut last_frame = std::time::Instant::now();

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

    // Demo mode zooms into the populated tile nearest the map centre and self-closes.
    let start = std::time::Instant::now();
    let target_center = living
        .occupants
        .occupied()
        .min_by_key(|c| {
            let dx = (c.x - home.x) as i64;
            let dy = (c.y - home.y) as i64;
            dx * dx + dy * dy
        })
        .unwrap_or(home);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Advance the live radiation by the whole generations the playback banks this frame. The
        // real frame delta comes from the render layer (float is fine here, Part 14.6); the driver
        // turns it into an integer number of whole generation steps, so determinism is untouched.
        // Interactive mode only: demo mode already ran the radiation to completion.
        let frame_dt = {
            let now = std::time::Instant::now();
            let dt = now.duration_since(last_frame).as_secs_f64();
            last_frame = now;
            dt
        };
        if demo_secs.is_none() && !wg.is_complete() {
            let steps = driver.advance(frame_dt);
            let mut advanced = false;
            for _ in 0..steps {
                if wg.step_once() {
                    advanced = true;
                }
            }
            if advanced {
                // Re-read the canonical state into a fresh snapshot for drawing. Only the occupants
                // change as the ecology radiates; the terrain and its quadtree are fixed.
                living = wg.snapshot();
            }
        }

        let depth = tree.depth();
        if let Some(total) = demo_secs {
            let t = start.elapsed().as_secs_f32();
            if t >= total {
                break; // the demo has run its course; close the window
            }
            // Zoom in over the first 70% of the time, then hold at the deepest level.
            let ramp = (total * 0.7).max(0.001);
            let frac = (t / ramp).min(1.0);
            zoom = ((frac * (max_zoom as f32 + 0.999)) as u32).min(max_zoom);
            cam.center = target_center;
        } else {
            // Pan by one node in the overview, one tile in the superfine, so panning is steady.
            let step = if zoom <= depth { tree.node_side(zoom) } else { 1 };
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
                    // Time control: space pauses, `.` and `,` speed up and slow down, `n` steps
                    // one generation. These change how fast the observer watches, never what
                    // happens (Principle 10).
                    Key::Space => {
                        driver.toggle_pause();
                    }
                    Key::Period => driver.scale_rate(2.0),
                    Key::Comma => driver.scale_rate(0.5),
                    Key::N => driver.request_steps(1),
                    _ => {}
                }
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

        let level = zoom.min(depth);
        let (mut buf, cell_px, side, mode) = if zoom <= depth {
            (
                cam.paint(&tree, &biomes, win_w, win_h, CELL, BG),
                CELL as i32,
                tree.node_side(level),
                format!("overview {zoom}/{depth}"),
            )
        } else {
            let sf = zoom - depth; // 1..=SUPERFINE_LEVELS
            let tile_px = (6 + 6 * sf) as i32;
            (
                render::superfine(&living, &biomes, cam.center, tile_px as usize, win_w, win_h, BG),
                tile_px,
                1,
                format!("superfine {sf} ({tile_px}px/tile)"),
            )
        };

        // The tile selector: outline the hovered cell and read out what is under it. In demo
        // mode there is no mouse, so point at the centre of the window (the target tile).
        let mut detail = "point at a tile".to_string();
        let mouse = if demo_secs.is_some() {
            Some((win_w as f32 / 2.0, win_h as f32 / 2.0))
        } else {
            window.get_mouse_pos(MouseMode::Discard)
        };
        if let Some((mx, my)) = mouse {
            let (mx, my) = (mx as i32, my as i32);
            if mx >= 0 && my >= 0 && (mx as usize) < win_w && (my as usize) < win_h {
                let cols = (win_w as i32 / cell_px).max(1);
                let rows = (win_h as i32 / cell_px).max(1);
                let ccol = (mx / cell_px).min(cols - 1);
                let crow = (my / cell_px).min(rows - 1);
                let (unit_x, unit_y) = if zoom <= depth {
                    (cam.center.x.div_euclid(side), cam.center.y.div_euclid(side))
                } else {
                    (cam.center.x, cam.center.y)
                };
                let cell_x = (unit_x - cols / 2) + ccol;
                let cell_y = (unit_y - rows / 2) + crow;
                render::draw_outline(
                    &mut buf,
                    win_w,
                    (ccol * cell_px) as usize,
                    (crow * cell_px) as usize,
                    cell_px as usize,
                    cell_px as usize,
                    CURSOR,
                );
                detail = if zoom <= depth {
                    match tree.node(level, cell_x, cell_y) {
                        Some(s) => format!(
                            "region ({cell_x},{cell_y}) from tile ({},{}), {}x{} tiles, mostly {}",
                            cell_x * side,
                            cell_y * side,
                            side,
                            side,
                            biomes.name(s.dominant)
                        ),
                        None => "off the world".to_string(),
                    }
                } else {
                    let coord = Coord3::ground(cell_x, cell_y);
                    let biome = living
                        .map
                        .tile(coord)
                        .map(|t| biomes.name(t.biome).to_string())
                        .unwrap_or_else(|| "off the world".to_string());
                    let occ = living.occupants.occupants(coord);
                    format!("tile ({cell_x},{cell_y})  {biome}  |  {}", describe_occupants(&living, &occ))
                };
                // Draw the readout on the map, just below the selected cell, so the names of
                // what the cursor sits on are visible without watching the title bar.
                render::draw_label(
                    &mut buf,
                    win_w,
                    win_h,
                    ccol * cell_px,
                    crow * cell_px + cell_px + 3,
                    &detail,
                    2,
                    Rgb::new(245, 245, 245),
                    Rgb::new(12, 14, 22),
                );
            }
        }

        // The time-control HUD, drawn top-left: how far the radiation has run, the playback speed,
        // whether it is paused or complete, and any temporal-LOD debt (the honest signal that the
        // chosen speed asked for more generations in a frame than the budget could run).
        let debt = if driver.lod_debt() > 0 {
            format!("  lod-debt {}", driver.lod_debt())
        } else {
            String::new()
        };
        let state = if wg.is_complete() {
            "[complete]"
        } else if driver.is_paused() {
            "[paused]"
        } else {
            "radiating"
        };
        // Deep-time readout: one radiation generation is the owner-set YEARS_PER_GENERATION.
        let years = wg.generation() * civsim_sim::YEARS_PER_GENERATION;
        let status = format!(
            "gen {}/{} (~{}k yr)  {state}  {:.2} gen/s  alive {}{debt}",
            wg.generation(),
            wg.generations_planned(),
            years / 1000,
            driver.rate(),
            wg.alive(),
        );
        render::draw_label(
            &mut buf,
            win_w,
            win_h,
            4,
            4,
            &status,
            2,
            Rgb::new(240, 240, 170),
            Rgb::new(10, 12, 20),
        );
        if demo_secs.is_none() {
            render::draw_label(
                &mut buf,
                win_w,
                win_h,
                4,
                20,
                "space pause  . faster  , slower  n step  +/- zoom  wasd pan",
                1,
                Rgb::new(170, 180, 200),
                Rgb::new(10, 12, 20),
            );
        }

        window.set_title(&format!(
            "civsim living world  0x{seed:X}  {mode}  |  {status}  |  {detail}"
        ));
        window
            .update_with_buffer(&buf, win_w, win_h)
            .expect("blit the frame");
    }
}
