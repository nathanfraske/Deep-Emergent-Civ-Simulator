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
//! `Esc` or the window close button quits. The view runs continuously from the most-zoomed-out
//! star-lit planet globe (the derived planet as a lit sphere in space), through the whole-world
//! overview, down to the superfine, where organisms are drawn as marks coloured by kind (plants
//! green, herbivores amber, carnivores red). The window is an observer: it reads the living world
//! and never writes it (Principle 10), so the same seed always shows the same world and biosphere.

mod render;

use minifb::{Key, KeyRepeat, MouseMode, Scale, ScaleMode, Window, WindowOptions};

use civsim_core::Fixed;
use civsim_sim::anatomy::WorldProfile;
use civsim_sim::clock::PlaybackDriver;
use civsim_sim::genesis::{genesis, GenesisParams, LivingWorld, WorldGenesis};
use civsim_sim::geodynamics::{slice0_demo_field, DerivedTile};
use civsim_sim::located::OccupantId;
use civsim_world::terrain::TerrainRelief;
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
/// How many planet-globe levels sit ABOVE the whole-world overview (the most-zoomed-out view, the star-lit planet
/// growing closer as you zoom in toward the surface). A view-side count, present only when the derived-planet
/// fixture builds; it never touches canonical state.
const GLOBE_LEVELS: u32 = 3;
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
    let path = argv
        .get(2)
        .cloned()
        .unwrap_or_else(|| "living.ppm".to_string());
    let seed: u64 = parse(argv.get(3), 0xEA27);
    let mut params = GenesisParams::dev_default();
    params.width = parse(argv.get(4), 96);
    params.height = parse(argv.get(5), 64);
    params.profile = world_profile(argv.get(6));
    let living = genesis(
        seed,
        &params,
        &civsim_sim::environ::AbioticSourceRegistry::earth_dev(),
        None,
    );
    let biomes = BiomeSet::dev_default();
    let center = populated_center(&living, params.width, params.height);
    let (w, h, tile_px) = (720usize, 480usize, 18usize);
    let mut buf = render::superfine(&living, &biomes, center, tile_px, w, h, BG);
    // Draw the selector cursor on the centre tile, so the snapshot shows it too.
    let (cols, rows) = ((w / tile_px) as i32, (h / tile_px) as i32);
    let ccol = (cols / 2) as usize;
    let crow = (rows / 2) as usize;
    render::draw_outline(
        &mut buf,
        w,
        ccol * tile_px,
        crow * tile_px,
        tile_px,
        tile_px,
        CURSOR,
    );
    // The selector readout for the centre tile, drawn on the map like the live viewer.
    let biome = living
        .map
        .tile(center)
        .map(|t| biomes.name(t.biome).to_string())
        .unwrap_or_else(|| "off the world".to_string());
    let occ = living.occupants.occupants(center);
    let detail = format!(
        "tile ({},{})  {biome}  |  {}",
        center.x,
        center.y,
        describe_occupants(&living, &occ)
    );
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

/// Render the Slice-0 DERIVED-terrain demo to a binary PPM and exit (the capstone's visible spine, headless): the
/// labelled demo crust field's relief, derived from composition through the substrate, painted by
/// [`render::paint_derived_tiles`]. Usage: `--derived-terrain <path> [cols] [rows] [tile_px]`. The terrain in the
/// frame is what the material IS, not fractal noise, and the window is a pure observer (Principle 10).
fn derived_terrain_cmd(argv: &[String]) {
    let path = argv
        .get(2)
        .cloned()
        .unwrap_or_else(|| "derived-terrain.ppm".to_string());
    let cols: usize = parse(argv.get(3), 48);
    let rows: usize = parse(argv.get(4), 32);
    let tile_px: usize = parse(argv.get(5), 14);
    let tiles = match civsim_sim::geodynamics::slice0_demo_field(cols, rows) {
        Some(t) => t,
        None => {
            eprintln!("the derived demo field did not resolve (a data gap); nothing written");
            return;
        }
    };
    let (w, h) = (cols.max(1) * tile_px.max(3), rows.max(1) * tile_px.max(3));
    let buf = render::paint_derived_tiles(&tiles, cols, tile_px, w, h, BG);
    write_ppm(&path, w, h, &buf);
    let tally = |r: TerrainRelief| tiles.iter().filter(|t| t.relief == r).count();
    eprintln!(
        "wrote {path} ({w}x{h}) derived terrain: {} tiles ({} submarine, {} lowland, {} upland), all derived from composition",
        tiles.len(),
        tally(TerrainRelief::Submarine),
        tally(TerrainRelief::Lowland),
        tally(TerrainRelief::Upland),
    );
}

/// Render one zoomed-out planet-globe frame to a binary PPM and exit (a display-free way to see the star-lit derived
/// planet, the seeable-world payoff headless): the derived-planet fixture drawn by [`render::render_solar_system_view`].
/// Usage: `--globe <path> [w] [h] [level]`, where `level` is 1..=GLOBE_LEVELS and sets how close the globe sits. The
/// globe's on-screen size is its DERIVED radius; the star's colour is its derived `T_eff`; the surface is the derived
/// tiles. A pure observer, canon -> pixels only (Principle 10).
fn globe_cmd(argv: &[String]) {
    let path = argv
        .get(2)
        .cloned()
        .unwrap_or_else(|| "globe.ppm".to_string());
    let w: usize = parse(argv.get(3), 720);
    let h: usize = parse(argv.get(4), 540);
    let level: u32 = parse(argv.get(5), GLOBE_LEVELS);
    let fx = match build_globe_fixture() {
        Some(f) => f,
        None => {
            eprintln!("the derived-planet fixture did not resolve (a data gap); nothing written");
            return;
        }
    };
    let g = level.clamp(1, GLOBE_LEVELS.max(1)) - 1;
    let (m_per_px, star_px, star_r) = globe_view_params(&fx, w, h, g);
    let buf = render::render_solar_system_view(
        fx.radius_m,
        fx.t_eff,
        &fx.tiles,
        fx.cols,
        w,
        h,
        m_per_px,
        star_px,
        star_r,
        BG,
        fx.sky,
    );
    write_ppm(&path, w, h, &buf);
    eprintln!(
        "wrote {path} ({w}x{h}) planet globe: derived radius ~{} km, star ~{} K, on-screen radius {} px",
        fx.radius_m.to_int() / 1000,
        fx.t_eff.to_int(),
        render::globe_radius_px(fx.radius_m, m_per_px),
    );
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
    let path = argv
        .get(2)
        .cloned()
        .unwrap_or_else(|| "frame.ppm".to_string());
    let mode = argv.get(3).map(String::as_str).unwrap_or("overview");
    let seed: u64 = parse(argv.get(4), 0xEA27);
    let mut params = GenesisParams::dev_default();
    params.width = parse(argv.get(5), 256);
    params.height = parse(argv.get(6), 192);
    params.profile = world_profile(argv.get(7));
    let living = genesis(
        seed,
        &params,
        &civsim_sim::environ::AbioticSourceRegistry::earth_dev(),
        None,
    );
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
    let living = genesis(
        seed,
        &params,
        &civsim_sim::environ::AbioticSourceRegistry::earth_dev(),
        None,
    );
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
    let mut wg = WorldGenesis::new(
        seed,
        &params,
        &civsim_sim::environ::AbioticSourceRegistry::earth_dev(),
        None,
    );
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
    let batch = genesis(
        seed,
        &params,
        &civsim_sim::environ::AbioticSourceRegistry::earth_dev(),
        None,
    );
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
            u64::from_str_radix(hex, 16)
                .ok()
                .and_then(|v| v.to_string().parse().ok())
        } else {
            s.parse().ok()
        }
    })
    .unwrap_or(default)
}

/// The DERIVED-planet fixture the zoomed-out globe view draws: the planet's derived radius, its star's derived
/// effective temperature, the derived-terrain tiles textured onto the sphere, and the derived Rayleigh sky tint.
/// This is a labelled development fixture (a one-Earth-mass planet at Earth's mean density around a Sun-mass star,
/// the demo crust field for the surface, modern-Earth air for the sky). The manager wires the real
/// star-plus-orbit and atmospheric-composition integration into these fields; the render entry point
/// [`render::render_solar_system_view`] reads them directly, so the swap is mechanical.
struct GlobeFixture {
    radius_m: Fixed,
    t_eff: Fixed,
    tiles: Vec<DerivedTile>,
    cols: usize,
    /// The atmosphere-limb tint: the DERIVED Rayleigh sky colour ([`render::rayleigh_sky_rgb`]) computed from a
    /// FIXTURE gas mix (the pending Stage-8 input), or [`render::PLACEHOLDER_SKY`] when the mix does not resolve.
    sky: Rgb,
}

/// Build the labelled derived-planet fixture, or `None` if any derivation fails (fail-soft: the viewer then simply
/// offers no globe zoom-out and behaves as before, never a fabricated planet). The radius is
/// [`civsim_sim::astro::planet_radius_m`] at one Earth mass and Earth's ~5.514 g/cm^3 mean density; the star's
/// `T_eff` is [`civsim_sim::astro::stellar_effective_temperature`] at a Sun-mass star (the two mass-relation
/// exponents and the fourth-root ceiling are the labelled Sun fixture); the surface tiles are the Slice-0 demo
/// crust field. Every value here is DERIVED or a clearly-labelled fixture, none fabricated.
fn build_globe_fixture() -> Option<GlobeFixture> {
    // The DERIVED planet through the integration spine: the authored Sun (mass 1, solar metallicity) at Earth's
    // orbit (1 AU) -> the stellar L/R/T_eff -> the disk temperature -> the accreted mass and condensed density ->
    // the radius. The stellar slopes are the grid-extracted values (alpha 3.5, beta 0.8, lambda -0.44, mu -0.018);
    // the accreted mass and whole-planet mean density are the Hadean-Earth fixtures until the accretion and
    // condensation arcs wire through. So the globe's on-screen SIZE and the star's blackbody COLOUR are read from the
    // real star-plus-orbit pipeline, not authored. Fail-soft: a None leaves the globe absent, never a fabricated one.
    let planet = civsim_sim::planet::derive_planet(
        Fixed::ONE,                    // Sun mass ratio
        Fixed::ONE,                    // solar metallicity
        Fixed::from_ratio(35, 10),     // alpha (mass-luminosity)
        Fixed::from_ratio(8, 10),      // beta (mass-radius)
        Fixed::from_ratio(-44, 100),   // lambda (metallicity-luminosity)
        Fixed::from_ratio(-18, 1000),  // mu (metallicity-radius)
        Fixed::ONE,                    // 1 AU
        Fixed::from_ratio(1, 100),     // accretion rate (Mirror fixture)
        Fixed::from_ratio(1, 4),       // reprocessing factor
        Fixed::ONE,                    // inner-boundary factor
        Fixed::ONE,                    // 1 Earth mass (accretion output, fixture)
        Fixed::from_ratio(5514, 1000), // 5.514 g/cm^3 mean density (condensation output, fixture)
        Fixed::from_int(100_000),
    )?;
    let cols = 48usize;
    let rows = 32usize;
    let tiles = slice0_demo_field(cols, rows)?;
    // The atmosphere-limb tint, DERIVED from the gas mix by Rayleigh scattering ([`render::rayleigh_sky_rgb`]).
    // The gas mix is a labelled FIXTURE (the pending Stage-8 atmospheric-composition input, the same
    // fixture-until-derived pattern as the accreted mass and mean density above): modern-Earth air, which the
    // physics scatters into a recognizably blue sky. When the composition arc wires through, this slice becomes a
    // read of the derived atmosphere. Fail-soft: an unresolvable mix (None) falls back to the labelled placeholder
    // sky, never a fabricated colour. The colour is observability-non-canon: it tints pixels only, never canon.
    const AIR_FIXTURE: [(&str, f64); 3] = [("N2", 0.78), ("O2", 0.21), ("Ar", 0.01)];
    let sky = civsim_physics::periodic::PeriodicTable::standard()
        .ok()
        .and_then(|table| {
            render::rayleigh_sky_rgb(&AIR_FIXTURE, planet.star_effective_temperature_k, &table)
        })
        .unwrap_or(render::PLACEHOLDER_SKY);
    Some(GlobeFixture {
        radius_m: planet.radius_m,
        t_eff: planet.star_effective_temperature_k,
        tiles,
        cols,
        sky,
    })
}

/// The globe view scale for level `g` (0-based, closer as it grows) in a `w` by `h` frame: the metres-per-pixel
/// scale (so the DERIVED radius drives the on-screen size, filling more of the frame as you zoom in toward the
/// overview), the star's on-screen position, and its on-screen radius. Shared by the interactive loop and the
/// headless `--globe` command so the two stay in step.
fn globe_view_params(fx: &GlobeFixture, w: usize, h: usize, g: u32) -> (Fixed, (i32, i32), usize) {
    let min_dim = w.min(h);
    let t = if GLOBE_LEVELS > 1 {
        g.min(GLOBE_LEVELS - 1) as f32 / (GLOBE_LEVELS - 1) as f32
    } else {
        1.0
    };
    let target_r = ((min_dim as f32) * (0.12 + 0.34 * t)) as i32;
    let m_per_px = fx
        .radius_m
        .checked_div(Fixed::from_int(target_r.max(1)))
        .unwrap_or(Fixed::ONE);
    // The star sits off to the upper-left; its screen position is the orbit's projection (a fixture phase until the
    // manager wires the real orbit), which sets which hemisphere is lit.
    let star_px = ((w / 5) as i32, (h / 4) as i32);
    let star_r = (min_dim / 22).max(3);
    (m_per_px, star_px, star_r)
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    // Headless snapshot: `--ppm <path> [seed] [w] [h]` writes a superfine frame and exits, so
    // the living world can be inspected without a display.
    if argv.get(1).map(|s| s == "--ppm").unwrap_or(false) {
        snapshot(&argv);
        return;
    }
    // Headless derived-terrain demo: `--derived-terrain <path> [cols] [rows] [tile_px]` writes a frame whose
    // terrain is derived from the demo crust field's composition (the capstone's visible spine) and exits.
    if argv
        .get(1)
        .map(|s| s == "--derived-terrain")
        .unwrap_or(false)
    {
        derived_terrain_cmd(&argv);
        return;
    }
    // Headless planet-globe frame: `--globe <path> [w] [h] [level]` writes one frame of the star-lit derived planet
    // (its size the derived radius, its star colour the derived T_eff) and exits.
    if argv.get(1).map(|s| s == "--globe").unwrap_or(false) {
        globe_cmd(&argv);
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
    let mut wg = WorldGenesis::new(
        seed,
        &params,
        &civsim_sim::environ::AbioticSourceRegistry::earth_dev(),
        None,
    );
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
    // The zoomed-out planet globe sits above the whole-world overview: build the derived-planet fixture, and if it
    // resolves, prepend GLOBE_LEVELS zoom steps so the view runs continuously from the star-lit globe down to the
    // tiles. If it fails to derive, the globe is simply absent and the viewer behaves exactly as before.
    let globe_fixture = build_globe_fixture();
    let globe_levels = if globe_fixture.is_some() {
        GLOBE_LEVELS
    } else {
        0
    };
    if let Some(fx) = globe_fixture.as_ref() {
        eprintln!(
            "derived-planet globe: radius ~{} km, star ~{} K (a labelled fixture; zoom out to see it)",
            fx.radius_m.to_int() / 1000,
            fx.t_eff.to_int(),
        );
    }
    let max_zoom = globe_levels + tree.depth() + SUPERFINE_LEVELS;

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

    // Start fully zoomed out: at the star-lit planet globe when the fixture built, else the whole-world overview.
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
            // Pan by one node in the overview, one tile in the superfine, so panning is steady. The globe view is
            // a fixed planet object, so panning is a no-op there.
            let step = if zoom < globe_levels {
                0
            } else {
                let mz = zoom - globe_levels;
                if mz <= depth {
                    tree.node_side(mz)
                } else {
                    1
                }
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
        // The globe levels sit above the overview; map_zoom indexes the flat map (overview then superfine).
        let in_globe = zoom < globe_levels;
        let map_zoom = zoom.saturating_sub(globe_levels);
        cam.zoom = map_zoom.min(depth);

        let (w, h) = window.get_size();
        if w == 0 || h == 0 {
            window.update();
            continue;
        }
        if (w, h) != (win_w, win_h) {
            win_w = w;
            win_h = h;
        }

        let level = map_zoom.min(depth);
        let (mut buf, cell_px, side, mode) = if in_globe {
            // The most-zoomed-out view: the star-lit planet globe, growing closer across the globe levels. The
            // on-screen size is the DERIVED radius at a view scale set so the globe fills more of the frame as you
            // zoom in, then hands off to the whole-world overview at map_zoom 0.
            let fx = globe_fixture
                .as_ref()
                .expect("in_globe implies the fixture built");
            let (m_per_px, star_px, star_r) = globe_view_params(fx, win_w, win_h, zoom);
            (
                render::render_solar_system_view(
                    fx.radius_m,
                    fx.t_eff,
                    &fx.tiles,
                    fx.cols,
                    win_w,
                    win_h,
                    m_per_px,
                    star_px,
                    star_r,
                    BG,
                    fx.sky,
                ),
                CELL as i32,
                1,
                format!(
                    "planet globe ~{}K star {}/{}",
                    fx.t_eff.to_int(),
                    zoom + 1,
                    globe_levels
                ),
            )
        } else if map_zoom <= depth {
            (
                cam.paint(&tree, &biomes, win_w, win_h, CELL, BG),
                CELL as i32,
                tree.node_side(level),
                format!("overview {map_zoom}/{depth}"),
            )
        } else {
            let sf = map_zoom - depth; // 1..=SUPERFINE_LEVELS
            let tile_px = (6 + 6 * sf) as i32;
            (
                render::superfine(
                    &living,
                    &biomes,
                    cam.center,
                    tile_px as usize,
                    win_w,
                    win_h,
                    BG,
                ),
                tile_px,
                1,
                format!("superfine {sf} ({tile_px}px/tile)"),
            )
        };

        // The tile selector: outline the hovered cell and read out what is under it. In demo
        // mode there is no mouse, so point at the centre of the window (the target tile). The globe view is a
        // planet object with no tiles to select, so the selector is skipped there.
        let mut detail = if in_globe {
            "the derived planet: zoom in (+) to reach the surface".to_string()
        } else {
            "point at a tile".to_string()
        };
        let mouse = if in_globe {
            None
        } else if demo_secs.is_some() {
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
                let (unit_x, unit_y) = if map_zoom <= depth {
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
                detail = if map_zoom <= depth {
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
                    format!(
                        "tile ({cell_x},{cell_y})  {biome}  |  {}",
                        describe_occupants(&living, &occ)
                    )
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
