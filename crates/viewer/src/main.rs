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
use civsim_materials::grain_opacity::{GrainConstituent, GrainMixture, GrainOpticalEstimator};
use civsim_physics::band_gap::BandGapColumn;
use civsim_physics::crystal_field::CrystalFieldTables;
use civsim_physics::janaf::JanafTables;
use civsim_physics::metal_eos::MetalEosAnchors;
use civsim_physics::optical_constants::OpticalConstants;
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::petrology_data::PhaseRegistry;
use civsim_physics::solar_abundances::SolarAbundances;
use civsim_sim::anatomy::WorldProfile;
use civsim_sim::clock::PlaybackDriver;
use civsim_sim::genesis::{genesis, GenesisParams, LivingWorld, WorldGenesis};
use civsim_sim::geodynamics::{
    derive_mantle_density, generate_derived_tiles, slice0_demo_field, DerivedTile,
};
use civsim_sim::located::OccupantId;
use civsim_world::terrain::TerrainRelief;
use civsim_world::view::Camera;
use civsim_world::{BiomeSet, Coord3, QuadTree, Rgb};
use std::collections::BTreeMap;

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
        render::SurfaceStyle::default(),
        render::GlobeOrientation::IDENTITY,
    );
    write_ppm(&path, w, h, &buf);
    eprintln!(
        "wrote {path} ({w}x{h}) planet globe: derived radius ~{} km, star ~{} K, on-screen radius {} px",
        fx.radius_m.to_int() / 1000,
        fx.t_eff.to_int(),
        render::globe_radius_px(fx.radius_m, m_per_px),
    );
}

/// Headless render of a DERIVED planet's globe from a star mass and orbit: `--derived-globe <path> [mass] [orbit]
/// [w] [h] [zoom_frac]`. Builds the derived scene (the same star-and-orbit chain the interactive `--derived` mode runs)
/// and draws its globe (the derived radius, the blackbody star colour, the derived material surface, the derived
/// atmosphere sky) to a PPM, so the derived planet is viewable without a display. With no `zoom_frac` it renders the
/// distant planet; a `zoom_frac` in [0, 1] drills in along the same continuous zoom the interactive viewer uses, so at
/// `1` the surface fills the frame and the display tile grid shows, letting the drill-in be inspected headlessly.
/// Prints the derived readout too.
fn derived_globe_cmd(argv: &[String]) {
    let path = argv
        .get(2)
        .cloned()
        .unwrap_or_else(|| "derived-globe.ppm".to_string());
    let star_mass = parse_fixed(argv.get(3), Fixed::ONE);
    let orbit_au = parse_fixed(argv.get(4), Fixed::ONE);
    let w: usize = parse(argv.get(5), 720);
    let h: usize = parse(argv.get(6), 540);
    let scene = match build_derived_scene(star_mass, orbit_au) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("the derived planet did not resolve: {e}; nothing written");
            return;
        }
    };
    print_derived_readout(&scene);
    let fx = GlobeFixture {
        radius_m: scene.radius_m,
        t_eff: scene.t_eff,
        tiles: scene.tiles.clone(),
        cols: scene.cols,
        sky: scene.sky,
    };
    // With a zoom fraction, use the interactive drill-in scale (and its refining tile grid); without one, the distant
    // planet framing. So the same PPM path can show either the whole planet or the gridded surface up close.
    let (m_per_px, star_px, star_r, style) = match argv.get(7).and_then(|s| s.parse::<f32>().ok()) {
        Some(t) => {
            let (m_per_px, star_px, star_r) = derived_globe_view(&fx, w, h, t.clamp(0.0, 1.0));
            let radius_px = render::globe_radius_px(fx.radius_m, m_per_px);
            let min_dim = w.min(h);
            let grid = (radius_px as f32 >= min_dim as f32 * 0.5)
                .then(|| surface_grid_dims(radius_px, min_dim));
            (
                m_per_px,
                star_px,
                star_r,
                render::SurfaceStyle {
                    tint: Some(scene.material),
                    grid,
                },
            )
        }
        None => {
            let g = GLOBE_LEVELS.saturating_sub(1);
            let (m_per_px, star_px, star_r) = globe_view_params(&fx, w, h, g);
            (
                m_per_px,
                star_px,
                star_r,
                render::SurfaceStyle {
                    tint: Some(scene.material),
                    grid: None,
                },
            )
        }
    };
    // Paint the sphere the crust's DERIVED material colour (the same the interactive `--derived` viewer shows), so the
    // headless render matches what the owner sees on screen.
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
        style,
        render::GlobeOrientation::IDENTITY,
    );
    write_ppm(&path, w, h, &buf);
    eprintln!("wrote {path} ({w}x{h})");
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

/// Parse a decimal argument (a star mass in solar masses, an orbit in AU) into fixed-point, defaulting when absent
/// or malformed. Uses [`Fixed::from_decimal_str`] so `--derived 0.6 1.5` reads a 0.6-solar-mass star at 1.5 AU.
fn parse_fixed(arg: Option<&String>, default: Fixed) -> Fixed {
    arg.and_then(|s| Fixed::from_decimal_str(s.trim()).ok())
        .unwrap_or(default)
}

/// The DERIVED planet the `--derived` viewer shows: every field DERIVED from a star mass and an orbit through the
/// built pipeline (the star, the disk temperature, the condensed-and-differentiated crust, the isostatic tiles, the
/// crust's optical colour, the atmospheric speciation and its Rayleigh sky), never authored. The planet MASS and BULK
/// DENSITY are the Hadean-Earth fixtures the accretion and condensation arcs will supply (so the radius and gravity
/// are an Earth-mass body's until those arcs wire through), while the star temperature, disk temperature, crust
/// composition, tile relief, material colour, and atmosphere all respond to the star and orbit inputs.
struct DerivedScene {
    star_mass: Fixed,
    orbit_au: Fixed,
    radius_m: Fixed,
    t_eff: Fixed,
    /// The MATURE disk surface warmth (K), the finished-planet irradiation equilibrium (one epoch of the join-law).
    disk_t: Fixed,
    /// The DERIVED FORMATION-era midplane condensation temperature (K) the crust condensed against (the OTHER epoch).
    condensation_t: Fixed,
    gravity: Fixed,
    mass_earth: Fixed,
    density: Fixed,
    /// The derived crust element composition (the phase stoichiometry the tiles and colour read), sorted descending.
    crust: Vec<(String, Fixed)>,
    /// The derived crust phase names (the buoyant partial-melt assemblage), for the readout.
    crust_phases: Vec<String>,
    /// The derived atmospheric gas mix (species name, mole fraction), descending by fraction.
    atmosphere: Vec<(String, Fixed)>,
    tiles: Vec<DerivedTile>,
    cols: usize,
    /// The DERIVED Rayleigh atmosphere-limb sky colour, or [`render::PLACEHOLDER_SKY`] when the mix does not resolve.
    sky: Rgb,
    /// The DERIVED perceived colour of the crust material under the star ([`render::material_surface_rgb`]).
    material: Rgb,
}

/// The natural log of ten, for the `log_eps` (base-10) solar-abundance scale to natural-exponent conversion (the same
/// constant [`derive_surface_composition`] uses).
const LN_TEN: Fixed = Fixed::from_int(2_302_585).div(Fixed::from_int(1_000_000)); // 2.302585

// THE EPOCH JOIN-LAW, the SEAM-3 fix, made concrete as reserved disk residues. The crust CONDENSES against the
// FORMATION-era midplane temperature (the hot, optically-thick, accreting inner disk), and the FINISHED planet's
// surface reads the MATURE `disk_effective_temperature` (the cooled irradiation equilibrium). The two are separate
// epochs and must never be conflated under one variable. These constants are the FORMATION epoch's disk residues,
// each reserved-with-basis (never fabricated), surfaced here; the reserved list is carried in the report.

/// RESERVED, the FORMATION-era mass-accretion rate `Mdot` (solar masses per megayear), the primary calibration knob of
/// the condensation epoch. Basis: pinned so the DERIVED 1 AU formation midplane reconstructs the forsterite/enstatite
/// silicate condensation front (~1400 K, the cited landmark the retired 1400 K fixture carried); ~1.9e-8 M_sun/yr, the
/// declining-disk accretion rate at the epoch the 1 AU midplane sits at that front. NOTE the degeneracy the owner
/// re-partitions: only the PRODUCT (this rate times the dust column times the opacity) is fixed by the 1 AU landmark,
/// so a lower dust column trades for a higher rate. DISTINCT from the MATURE surface-warmth accretion rate below.
const FORMATION_ACCRETION_RATE_MSUN_MYR: Fixed = Fixed::from_int(19).div(Fixed::from_int(100)); // 0.19

/// RESERVED, the MATURE mass-accretion rate (solar masses per megayear) that drives the finished planet's SURFACE
/// warmth ([`disk_effective_temperature`]). Basis: the observed class-II disk value ~1e-8 M_sun/yr (0.01 M_sun/Myr),
/// the epoch the planet's surface equilibrates in; kept the Mirror fixture. The MATURE epoch, not the formation one.
const MATURE_ACCRETION_RATE_MSUN_MYR: Fixed = Fixed::from_int(1).div(Fixed::from_int(100)); // 0.01

/// RESERVED, the disk characteristic (cutoff) radius `r_c` in AU. Basis: observed class-II disk radii ~30 to 100 AU
/// (Andrews et al. 2010, the sub-mm dust-continuum sizes). Shared by the dust (opacity) and solid (accretion) columns.
const DISK_CHARACTERISTIC_RADIUS_AU: Fixed = Fixed::from_int(30);

/// RESERVED, the DUST surface-density normalization `Sigma_c` (g/cm^2) of the Lynden-Bell-Pringle profile, the optical
/// column the midplane depth integrates. Basis: the Hayashi 1981 MMSN gas column ~1700 g/cm^2 at 1 AU times a
/// dust-to-gas ratio ~0.01, giving Sigma_dust(1 AU) ~17 g/cm^2 on this profile (`r_c` = 30 AU, `gamma` = 1).
const DUST_SURFACE_DENSITY_NORM_G_CM2: Fixed = Fixed::from_int(586).div(Fixed::from_int(1000)); // 0.586

/// RESERVED, the dust surface-density slope `gamma` (viscous-spreading exponent). Basis: the Lynden-Bell-Pringle
/// self-similar disk with viscosity `nu ~ r` (gamma = 1), the fiducial viscous profile.
const DUST_SURFACE_DENSITY_GAMMA: Fixed = Fixed::ONE;

/// RESERVED, the reference temperature (K) the grain Rosseland opacity is evaluated at to derive `kappa_ref`. Basis:
/// the silicate condensation front (~1400 K); IMMATERIAL to within the fourth-root damping because the single-component
/// silicate dust's Rosseland opacity is nearly FLAT across the condensation window (~500 to 660 cm^2/g per g dust).
const CONDENSATION_OPACITY_REFERENCE_K: Fixed = Fixed::from_int(1400);

/// RESERVED, the bisection bracket floor and ceiling (K) for the midplane fixed point. Basis: engine bounds spanning
/// the condensation window; the ceiling caps the fourth-root read at a refractory upper edge (above which nothing
/// condenses in the current data).
const MIDPLANE_BRACKET_LO_K: Fixed = Fixed::from_int(100);
const MIDPLANE_BRACKET_HI_K: Fixed = Fixed::from_int(1950);

/// RESERVED, the SOLID surface-density normalization `Sigma_c` (kg/m^2) for the accretion feeding-zone integral. Basis:
/// the Hayashi 1981 MMSN ROCK column ~7.1 g/cm^2 (= 71 kg/m^2) at 1 AU INSIDE the ice line (NOT the gas 1700, which
/// would overmass the planet ~30x, and NOT the 30 g/cm^2 beyond-ice-line rock+ice figure), giving Sigma_solid(1 AU)
/// ~7.1 g/cm^2 on this profile (`r_c` = 30 AU, `gamma` = 1.5, the MMSN `r^-3/2` solid slope).
const SOLID_SURFACE_DENSITY_NORM_KG_M2: Fixed = Fixed::from_int(519).div(Fixed::from_int(1000)); // 0.519

/// RESERVED, the solid surface-density slope `gamma`. Basis: the Hayashi 1981 MMSN solid column `Sigma ~ r^-3/2`.
const SOLID_SURFACE_DENSITY_GAMMA: Fixed = Fixed::from_int(15).div(Fixed::from_int(10)); // 1.5

/// RESERVED, the feeding-zone full width as a FRACTION of the orbit. Basis: ~10 Hill radii of a Mars-class embryo at
/// 1 AU (`R_H` ~0.0046 AU each), the isolation-mass feeding-zone width; retires when the Hill-radius/isolation-mass
/// closure lands. Scaled by the orbit because `R_H ~ r`.
const FEEDING_ZONE_WIDTH_FRACTION: Fixed = Fixed::from_int(5).div(Fixed::from_int(100)); // 0.05

/// The feeding-zone integration resolution (a fixed midpoint-sum step count, an engine accuracy bound, not a physical
/// knob; determinism holds by construction).
const FEEDING_ZONE_STEPS: u32 = 64;

// THE SEAM-6 PARTIAL-MELT (McKenzie-Bickle 1988) reserved interior inputs the crustal-thickness closure reads. The
// SOLIDUS (surface value and slope) and the SOURCE DENSITY are DERIVED inside the melt wiring (from the endmember
// signatures and the floating source assemblage), so only these interior-thermostat and mantle-floor values are
// reserved-with-basis here (never fabricated; the reserved list is in docs/working/MORNING_REVIEW.md). For the
// current solar-condensed floating set (a ~1680 K refractory ideal solidus) the normal potential temperature is
// sub-solidus, so the viewer scene falls back to buoyancy and these values are dormant until a fertile or hotter
// mantle engages the melt; they are wired so the payoff is live the moment the physics reaches it.

/// RESERVED, the mantle POTENTIAL TEMPERATURE (K) the adiabat projects to the surface. Basis: McKenzie-Bickle's
/// normal-MORB value ~1553 K, the melt rung validated at 1588 K; a per-world and per-epoch value (a hotter
/// Hadean/Archean mantle melts a more refractory set). Cited: McKenzie & Bickle (1988) J. Petrology 29, 625.
const MANTLE_POTENTIAL_TEMPERATURE_K: Fixed = Fixed::from_int(1588);

/// RESERVED, the mantle ADIABAT slope (K/GPa) along the isentrope. Basis: dT/dP|_S = alpha T V / Cp ~ 0.5 K/km ~
/// 15.5 K/GPa; a flagged derive-down (from the mantle assemblage's thermal expansion, density, and heat capacity
/// once those land in the petrology substrate). Cited: McKenzie & Bickle (1988).
const MANTLE_ADIABAT_SLOPE_K_PER_GPA: Fixed = Fixed::from_int(155).div(Fixed::from_int(10)); // 15.5

/// RESERVED, the isentropic melting PRODUCTIVITY dF/dP (per GPa) near the solidus. Basis: ~0.12/GPa; a flagged
/// derive-down (from the entropy of fusion and the heat capacity once the full McKenzie 1984 productivity lands).
/// Cited: McKenzie & Bickle (1988).
const MELT_PRODUCTIVITY_PER_GPA: Fixed = Fixed::from_int(12).div(Fixed::from_int(100)); // 0.12

/// RESERVED, the surface GRAVITY (m/s^2) the pooled-melt thickness divides by. Basis: the planet's surface gravity
/// g = G M / R^2, which DERIVES from the planet mass and radius; reserved here only because the surface chain
/// precedes the planet-gravity derivation in the scene ordering (a flagged derive-down, the g = 9.80665 convention
/// is not a canon anchor). Dormant while the viewer set is sub-solidus. Cited: derived planet gravity / CODATA G.
const MELT_COLUMN_GRAVITY_M_S2: Fixed = Fixed::from_int(98).div(Fixed::from_int(10)); // 9.8

/// The single-component disk dust the opacity is derived from: astronomical silicate, the canonical protoplanetary
/// disk-opacity carrier, present in the standard optical library. HONEST LIMIT (a named #54 grain-wire follow-on): the
/// full condensate mixture (silicate + metallic iron + troilite) cannot yet be Rosseland-averaged in production, both
/// because the Bruggeman effective-medium solve returns `None` for those index combinations across the window and
/// because metallic iron's optical table under-covers the Rosseland window, and the missing production optical
/// estimator is the deeper gap; so the disk opacity is a first-grade single-component silicate dust.
const DUST_SPECIES: &str = "astronomical_silicate";
/// RESERVED, the astronomical-silicate bulk density (g/cm^3) for the grain mass-to-volume conversion (only the single
/// grain's density, which the size-average consumes). Basis: the standard astronomical-silicate density ~3.3 g/cm^3.
const DUST_BULK_DENSITY_G_CM3: Fixed = Fixed::from_int(33).div(Fixed::from_int(10)); // 3.3
/// RESERVED, the grain size-distribution slope (Dohnanyi collisional-cascade steady state 3.5) and radius bounds
/// (micron), the #54 wire's reserved size-distribution residues reused here.
const DUST_SIZE_SLOPE: Fixed = Fixed::from_int(35).div(Fixed::from_int(10)); // 3.5
const DUST_A_MIN_UM: Fixed = Fixed::from_int(1).div(Fixed::from_int(10)); // 0.1
const DUST_A_MAX_UM: Fixed = Fixed::from_int(10);

/// A loud-miss optical estimator: the single-component silicate dust is entirely in the measured optical library, so
/// this estimator is never reached; if it were, a `None` is the loud miss (never a silent zero). The production
/// phonon-backed estimator is the named #54 follow-on.
struct LoudMissEstimator;
impl GrainOpticalEstimator for LoudMissEstimator {
    fn index(&self, _species: &str, _lambda_um: Fixed) -> Option<(Fixed, Fixed)> {
        None
    }
}

/// Snap a derived temperature to the nearest condensation-grid node (nearest 100 K). THE CONDENSATION-GRID DATA-CEILING
/// (named, not hidden): the equilibrium-condensation element-potential minimizer converges only at the JANAF
/// tabulation grid (multiples of 100 K); at intermediate temperatures the ill-conditioned trace-abundance solve
/// diverges (returns no crust). So the DERIVED orbit-dependent midplane temperature is evaluated at the nearest
/// tabulated node, its ~100 K resolution the honest grain of the condensation staircase. Retires when the minimizer is
/// hardened to converge off-grid.
fn snap_to_condensation_grid(t: Fixed) -> Fixed {
    let rounded = t.checked_add(Fixed::from_int(50)).unwrap_or(t).to_int();
    Fixed::from_int((rounded / 100) * 100)
}

/// The solar-abundance linear amount for an element, `n_X/n_H = 10^(log_eps(X) - 12)`, or `None` if the element carries
/// no cited abundance (the same conversion [`derive_surface_composition`] uses).
fn abundance_amount(abundances: &SolarAbundances, element: &str) -> Option<Fixed> {
    let log_eps = abundances.preferred(element)?;
    let exponent = log_eps
        .checked_sub(Fixed::from_int(12))?
        .checked_mul(LN_TEN)?;
    Some(exponent.exp())
}

/// The DERIVED FORMATION-era condensation temperature at the orbit (K), snapped to the condensation grid, or `None` if a
/// link does not resolve. It derives the disk opacity `kappa_ref` from the #54 grain wire (the astronomical-silicate
/// dust's Rosseland mean, evaluated ONCE at the reference because it is nearly flat in T; calling it inside the
/// 60-step bisection would be both far too slow and, for a fixed composition, without the sublimation cliff that a
/// T-dependent condensate set would add, a named #54 follow-on), then calls the banked optically-thick midplane fixed
/// point ([`civsim_sim::astro::formation_midplane_temperature`]) with the FORMATION accretion rate and the reserved
/// disk residues. This is the condensation epoch of the join-law, kept apart from the mature surface warmth.
fn derive_formation_condensation_temperature(
    star_mass: Fixed,
    orbit_au: Fixed,
    optical: &OpticalConstants,
) -> Option<Fixed> {
    let estimator = LoudMissEstimator;
    let dust = GrainMixture::new(
        vec![GrainConstituent {
            species: DUST_SPECIES.to_string(),
            amount: Fixed::ONE,
            bulk_density_g_cm3: DUST_BULK_DENSITY_G_CM3,
            condensation_rank: 0,
        }],
        DUST_SIZE_SLOPE,
        DUST_A_MIN_UM,
        DUST_A_MAX_UM,
        optical,
        &estimator,
    )?;
    let kappa_ref = dust.rosseland_opacity(CONDENSATION_OPACITY_REFERENCE_K)?;
    let raw = civsim_sim::astro::formation_midplane_temperature(
        FORMATION_ACCRETION_RATE_MSUN_MYR,
        star_mass,
        Fixed::from_ratio(35, 10), // alpha (mass-luminosity), the grid-extracted stellar slope
        orbit_au,
        Fixed::from_ratio(1, 4), // reprocessing factor (spherical-grain equilibrium)
        Fixed::ONE,              // inner-boundary factor (~1 in the bulk disk)
        DISK_CHARACTERISTIC_RADIUS_AU,
        DUST_SURFACE_DENSITY_GAMMA,
        DUST_SURFACE_DENSITY_NORM_G_CM2,
        |_t| Some(kappa_ref),
        MIDPLANE_BRACKET_LO_K,
        MIDPLANE_BRACKET_HI_K,
    )?;
    Some(snap_to_condensation_grid(raw))
}

/// The DERIVED isolation (feeding-zone) mass at the orbit, in Earth masses, or `None` if the integral fails. It sweeps
/// the accretion feeding zone (a few Hill radii around the orbit) over the MMSN SOLID column (not the gas, which would
/// overmass the planet ~30x) and folds to Earth masses ([`feeding_zone_mass`] then [`feeding_zone_mass_earth`]). The
/// honest output is Mars-class (~0.08 to 0.11 M_earth): a pure accretion isolation mass, the Earth-size deferred to the
/// Layer-4 event tier (the giant-impact merger), never authored here.
fn derive_isolation_mass_earth(orbit_au: Fixed) -> Option<Fixed> {
    let half = orbit_au
        .checked_mul(FEEDING_ZONE_WIDTH_FRACTION)?
        .checked_div(Fixed::from_int(2))?;
    let inner_au = orbit_au.checked_sub(half)?;
    let outer_au = orbit_au.checked_add(half)?;
    let feeding = civsim_sim::astro::feeding_zone_mass(
        inner_au,
        outer_au,
        DISK_CHARACTERISTIC_RADIUS_AU,
        SOLID_SURFACE_DENSITY_GAMMA,
        SOLID_SURFACE_DENSITY_NORM_KG_M2,
        FEEDING_ZONE_STEPS,
    )?;
    civsim_sim::astro::feeding_zone_mass_earth(feeding)
}

/// The DERIVED UNCOMPRESSED bulk density (g/cm^3) from the differentiated composition: the volume-weighted mean of the
/// sinking METAL core (density from the Fe EOS anchor, atomic weight over molar volume) and the floating SILICATE
/// mantle (`rho_silicate`, the derived mantle assemblage density), split by the solar core:silicate MASS ratio. FIRST
/// GRADE (the coordinator-endorsed fallback, because the VCS modal amounts are degenerate at these vertices): the mass
/// split is derived from the solar bulk abundances with a normative-oxide rock stoichiometry (O per rock cation: Si and
/// Ti 2, Al 1.5, Mg and Ca 1, Na and K 0.5) and all Fe assigned to the metal-plus-troilite core; the COMPRESSED density
/// (the interior-structure EOS integration, hydrostatic against the Vinet EOS on the metal anchors, ~5% higher for a
/// Mars-class body) is a NAMED refinement, not fabricated. `None` if an abundance, atomic weight, or the Fe anchor is
/// missing. For solar composition this lands ~4.2 g/cm^3, the Earth-uncompressed grade.
fn derive_uncompressed_bulk_density(
    abundances: &SolarAbundances,
    table: &PeriodicTable,
    eos: &MetalEosAnchors,
    rho_silicate: Fixed,
) -> Option<Fixed> {
    let oxygen_mass = table.element("O")?.standard_atomic_weight;
    // The sinking metal-plus-sulfide core: all Fe (metal), Ni (metal), and S (troilite sulfur), at bulk abundance.
    let mut core_mass = Fixed::ZERO;
    for element in ["Fe", "Ni", "S"] {
        if let (Some(n), Some(e)) = (
            abundance_amount(abundances, element),
            table.element(element),
        ) {
            core_mass = core_mass.checked_add(n.checked_mul(e.standard_atomic_weight)?)?;
        }
    }
    // The floating silicate-plus-oxide mantle: the rock cations at bulk abundance, each with its normative oxide oxygen.
    let mut mantle_mass = Fixed::ZERO;
    for (element, o_num, o_den) in [
        ("Mg", 1, 1),
        ("Si", 2, 1),
        ("Al", 3, 2),
        ("Ca", 1, 1),
        ("Na", 1, 2),
        ("K", 1, 2),
        ("Ti", 2, 1),
    ] {
        if let (Some(n), Some(e)) = (
            abundance_amount(abundances, element),
            table.element(element),
        ) {
            let oxide_o = oxygen_mass.checked_mul(Fixed::from_ratio(o_num, o_den))?;
            let oxide_mass = e.standard_atomic_weight.checked_add(oxide_o)?;
            mantle_mass = mantle_mass.checked_add(n.checked_mul(oxide_mass)?)?;
        }
    }
    let total = core_mass.checked_add(mantle_mass)?;
    if total <= Fixed::ZERO {
        return None;
    }
    let core_fraction = core_mass.checked_div(total)?;
    let mantle_fraction = Fixed::ONE.checked_sub(core_fraction)?;
    // The core (metal) density from the Fe EOS anchor: atomic weight over measured molar volume.
    let iron_mass = table.element("Fe")?.standard_atomic_weight;
    let iron_molar_volume = eos.molar_volume("Fe")?;
    let rho_metal = iron_mass.checked_div(iron_molar_volume)?;
    // Volume-weighted mean: rho = total mass / total volume = 1 / (f_core/rho_metal + f_mantle/rho_silicate).
    let inverse = core_fraction
        .checked_div(rho_metal)?
        .checked_add(mantle_fraction.checked_div(rho_silicate)?)?;
    Fixed::ONE.checked_div(inverse)
}

/// Build the DERIVED scene from a star mass and an orbit, or an error naming the link that did not resolve (fail-soft:
/// the viewer prints the message and shows no planet, never a fabricated one). The chain is the built pipeline, each
/// link a derivation: the star and disk ([`civsim_sim::planet::derive_planet`]), the condensed-and-differentiated
/// crust at a labelled formation-era condensation temperature ([`derive_surface_composition`], the two-temperature seam
/// documented at its site), the mantle density from the derived mantle
/// composition ([`derive_mantle_density`]), the isostatic tiles ([`generate_derived_tiles`]) off a uniform crust field
/// (uniform is the honest state for a fresh planet; lateral variation is a named geodynamics follow-on), the crust's
/// optical colour under the star ([`render::material_surface_rgb`]), and the atmospheric speciation
/// ([`atmosphere_gas_equilibrium`]) with its Rayleigh sky ([`render::rayleigh_sky_rgb`]).
fn build_derived_scene(star_mass: Fixed, orbit_au: Fixed) -> Result<DerivedScene, String> {
    let janaf = JanafTables::standard().map_err(|_| "the JANAF tables did not load")?;
    let abundances =
        SolarAbundances::standard().map_err(|_| "the solar abundances did not load")?;
    let optical =
        OpticalConstants::standard().map_err(|_| "the optical-constants library did not load")?;

    // THE EPOCH JOIN-LAW (the SEAM-3 fix), no longer a single conflated temperature. The crust CONDENSED during
    // planet formation against the FORMATION-era midplane: the hot, optically-thick, accreting inner disk, DERIVED at
    // the orbit from the banked viscous-plus-irradiation midplane fixed point
    // ([`civsim_sim::astro::formation_midplane_temperature`], consuming the #54 grain opacity), then snapped to the
    // condensation grid. This is orbit-DEPENDENT (a closer orbit condenses a more refractory crust, a farther one a
    // cooler assemblage), the free payoff the two-temperature seam had blocked. The FINISHED planet's surface warmth
    // (`planet.disk_temperature_k`, ~279 K at 1 AU below) is the MATURE irradiation equilibrium, a distinct epoch; the
    // two are never conflated. `derive_formation_condensation_temperature` and the reserved disk residues carry the
    // basis.
    let condensation_temperature_k = derive_formation_condensation_temperature(
        star_mass, orbit_au, &optical,
    )
    .ok_or("the formation-era midplane condensation temperature did not derive at this orbit")?;
    // The seam-6 partial-melt interior inputs (reserved-with-basis McKenzie-Bickle values; the solidus and source
    // density derive inside the chain). For the refractory solar set these are dormant (sub-solidus, buoyancy
    // fallback); they engage the moment a fertile or hotter mantle crosses the derived solidus.
    let reserved_melt = civsim_materials::surface_composition::ReservedMeltParams {
        potential_temperature_k: MANTLE_POTENTIAL_TEMPERATURE_K,
        adiabat_slope_k_per_gpa: MANTLE_ADIABAT_SLOPE_K_PER_GPA,
        productivity_per_gpa: MELT_PRODUCTIVITY_PER_GPA,
        gravity_m_per_s2: MELT_COLUMN_GRAVITY_M_S2,
    };
    let sc = civsim_materials::surface_composition::derive_surface_composition(
        &janaf,
        &abundances,
        condensation_temperature_k,
        &reserved_melt,
    )
    .ok_or("no crust condensed at the derived formation-era midplane temperature (the gas did not precipitate a surface)")?;

    let registry = PhaseRegistry::standard().map_err(|_| "the phase registry did not load")?;
    let table = PeriodicTable::standard().map_err(|_| "the periodic table did not load")?;
    let eos = MetalEosAnchors::standard().map_err(|_| "the metal EOS anchors did not load")?;

    // The crust and mantle ELEMENT compositions are now the ASSEMBLAGE the substrate derives (seam 5): the crust's
    // enstatite at its real Mg:Si:O = 1:1:3, the mantle's forsterite at 2:1:4, each phase weighted by its VCS modal
    // amount, which the petrology density and isostasy consume. `sc.surface` and `sc.mantle_composition` are the rock
    // (no longer the oxygen-heavy solar budget), so the viewer reads them directly rather than re-deriving the
    // stoichiometry here.
    let crust_composition = sc.surface.clone();
    let mantle_composition = sc.mantle_composition.clone();
    if crust_composition.is_empty() {
        return Err("the derived crust has no phases to read a composition from".to_string());
    }

    // The mantle density from the DERIVED mantle composition, read as the density of its stable assemblage at the
    // surface reference (300 K, 1 bar, the reference-pressure first pass the geodynamics substrate documents). The
    // crust floats on this derived mantle density, and it is the silicate density the bulk-density derivation reads.
    let surface_t = Fixed::from_int(300);
    let surface_p = Fixed::ONE;
    let mantle_density =
        derive_mantle_density(&mantle_composition, surface_t, surface_p, &registry, &table)
            .ok_or("the mantle density did not derive from the mantle composition")?;

    // THE ACCRETED MASS and the UNCOMPRESSED BULK DENSITY, both now DERIVED (the Fixed::ONE and 5.514 fixtures are
    // retired). The mass is the accretion feeding-zone ISOLATION mass over the MMSN SOLID column: honestly Mars-class
    // (~0.08 to 0.11 M_earth), the pure accretion output, with the Earth-size deferred to the Layer-4 event tier (the
    // giant-impact merger), never authored here. The bulk density is the volume-weighted metal-core-plus-silicate-
    // mantle mean from the differentiated composition (~4.2 g/cm^3, the Earth-uncompressed grade). Each reserved disk
    // residue carries its basis at its definition above.
    let planet_mass_earth = derive_isolation_mass_earth(orbit_au)
        .ok_or("the accretion isolation mass did not derive at this orbit")?;
    let planet_bulk_density =
        derive_uncompressed_bulk_density(&abundances, &table, &eos, mantle_density).ok_or(
            "the uncompressed bulk density did not derive from the differentiated composition",
        )?;

    // The DERIVED planet: the star's L/R/T_eff, the MATURE surface warmth at the orbit (the finished-planet epoch of
    // the join-law), and the radius/gravity from the DERIVED accreted mass and uncompressed bulk density. The stellar
    // slopes are the grid-extracted values; the MATURE accretion rate, reprocessing, and inner-boundary factor are the
    // reserved-with-basis residues (the surface-warmth epoch, distinct from the formation accretion rate above).
    let planet = civsim_sim::planet::derive_planet(
        star_mass,
        Fixed::ONE,                   // solar metallicity
        Fixed::from_ratio(35, 10),    // alpha (mass-luminosity)
        Fixed::from_ratio(8, 10),     // beta (mass-radius)
        Fixed::from_ratio(-44, 100),  // lambda (metallicity-luminosity)
        Fixed::from_ratio(-18, 1000), // mu (metallicity-radius)
        orbit_au,
        MATURE_ACCRETION_RATE_MSUN_MYR, // the MATURE surface-warmth accretion rate (not the formation one)
        Fixed::from_ratio(1, 4),        // reprocessing factor
        Fixed::ONE,                     // inner-boundary factor
        planet_mass_earth,              // DERIVED accretion isolation mass (Mars-class)
        planet_bulk_density,            // DERIVED uncompressed bulk density
        Fixed::from_int(100_000),
    )
    .ok_or("the star-and-orbit derivation did not resolve")?;

    // The isostatic tiles off a UNIFORM derived-crust field: every tile carries the derived crust, so its elevation is
    // what the material is by Airy isostasy against the derived mantle. A representable column thickness (30 km) and a
    // zero sea-level datum are the Slice-0 fixtures (retire with the interior and water-budget lanes).
    let cols = 48usize;
    let rows = 32usize;
    let field: Vec<Vec<(String, Fixed)>> = vec![crust_composition.clone(); cols * rows];
    // The crustal thickness: the seam-6 McKenzie-Bickle DERIVED partial-melt thickness when the melt mechanism
    // engaged, else the Slice-0 representable fixture (30 km, retires with the interior lane). For the refractory
    // solar set the melt is sub-solidus (buoyancy fallback, `crust_thickness_km` None), so the fixture stands.
    let crustal_thickness = sc.crust_thickness_km.unwrap_or_else(|| Fixed::from_int(30));
    let sea_level = Fixed::ZERO;
    let tiles = generate_derived_tiles(
        &field,
        mantle_density,
        crustal_thickness,
        surface_t,
        surface_p,
        sea_level,
        &registry,
        &table,
    )
    .ok_or("the derived tiles did not resolve from the crust composition")?;

    // The crust's perceived colour under the star, DERIVED from its absorption spectrum. The broadening temperature is
    // the derived disk (surface) warmth. Fail-soft to the relief swatch if the material read returns None.
    let gaps = BandGapColumn::standard().map_err(|_| "the band-gap column did not load")?;
    let crystal =
        CrystalFieldTables::standard().map_err(|_| "the crystal-field table did not load")?;
    let material = render::material_surface_rgb(
        &crust_composition,
        planet.star_effective_temperature_k,
        planet.disk_temperature_k,
        &gaps,
        &crystal,
        &table,
    )
    .unwrap_or_else(|| render::derived_tile_color(TerrainRelief::Lowland));

    // The atmospheric speciation of a labelled volatile OUTGASSING budget (an oxidized volcanic C-N-O-H-S budget, the
    // pending item-#40 outgassing input), DERIVED through the gas-phase Gibbs solve at the reserved volcanic quench
    // temperature (~1400 K, the magma-degassing basis the atmosphere substrate states), then its Rayleigh sky. So the
    // SPECIATION and the sky COLOUR are derived; the elemental budget is the labelled pending input, not fabricated.
    let atmo_budget: BTreeMap<String, Fixed> =
        [("H", 160), ("O", 100), ("C", 10), ("N", 4), ("S", 3)]
            .iter()
            .map(|(el, n)| (el.to_string(), Fixed::from_int(*n)))
            .collect();
    let quench_t = Fixed::from_int(1400);
    let atmosphere =
        civsim_materials::atmosphere::atmosphere_gas_equilibrium(&janaf, &atmo_budget, quench_t)
            .unwrap_or_default();
    let gas_mix: Vec<(&str, f64)> = atmosphere
        .iter()
        .map(|(name, frac)| (name.as_str(), frac.to_f64_lossy()))
        .collect();
    let sky = render::rayleigh_sky_rgb(&gas_mix, planet.star_effective_temperature_k, &table)
        .unwrap_or(render::PLACEHOLDER_SKY);

    // The crust elements (phase stoichiometry), sorted descending by amount (the readout's top few), and the crust
    // phase names for the readout.
    let mut crust = crust_composition;
    crust.sort_by(|a, b| b.1.to_bits().cmp(&a.1.to_bits()).then(a.0.cmp(&b.0)));
    let crust_phases: Vec<String> = sc.crust.iter().map(|(name, _)| name.clone()).collect();

    Ok(DerivedScene {
        star_mass,
        orbit_au,
        radius_m: planet.radius_m,
        t_eff: planet.star_effective_temperature_k,
        disk_t: planet.disk_temperature_k,
        condensation_t: condensation_temperature_k,
        gravity: planet.surface_gravity_m_s2,
        mass_earth: planet.mass_earth,
        density: planet.bulk_density_g_cm3,
        crust,
        crust_phases,
        atmosphere,
        tiles,
        cols,
        sky,
        material,
    })
}

/// Print the DERIVED numbers to the terminal: the star mass and orbit inputs, then the derived radius, gravity, star
/// effective temperature, disk temperature, the top crust elements, and the atmospheric gas mix. A one-way read of the
/// derived scene (Principle 10).
fn print_derived_readout(scene: &DerivedScene) {
    eprintln!("derived planet from star mass and orbit alone (no worldgen, no life):");
    eprintln!(
        "  input:  star mass {:.3} M_sun,  orbit {:.3} AU",
        scene.star_mass.to_f64_lossy(),
        scene.orbit_au.to_f64_lossy()
    );
    eprintln!(
        "  star:   T_eff {:.0} K  (blackbody light colour)",
        scene.t_eff.to_f64_lossy()
    );
    eprintln!(
        "  planet: radius {:.0} km,  gravity {:.2} m/s^2,  mass {:.2} M_earth,  bulk density {:.2} g/cm^3",
        scene.radius_m.to_f64_lossy() / 1000.0,
        scene.gravity.to_f64_lossy(),
        scene.mass_earth.to_f64_lossy(),
        scene.density.to_f64_lossy()
    );
    eprintln!(
        "  disk:   surface warmth {:.0} K (mature irradiation equilibrium) | crust condensed at {:.0} K (derived formation midplane)",
        scene.disk_t.to_f64_lossy(),
        scene.condensation_t.to_f64_lossy()
    );
    let crust_top: Vec<String> = scene
        .crust
        .iter()
        .take(5)
        .map(|(el, amt)| format!("{el} {:.2}", amt.to_f64_lossy()))
        .collect();
    eprintln!(
        "  crust:  {}  ({})",
        crust_top.join(", "),
        scene.crust_phases.join(", ")
    );
    eprintln!(
        "  crust colour under the star: rgb({}, {}, {})",
        scene.material.r, scene.material.g, scene.material.b
    );
    let air_top: Vec<String> = scene
        .atmosphere
        .iter()
        .take(6)
        .map(|(name, frac)| format!("{name} {:.1}%", frac.to_f64_lossy() * 100.0))
        .collect();
    if air_top.is_empty() {
        eprintln!("  air:    (no atmosphere resolved from the labelled outgassing budget)");
    } else {
        eprintln!("  air:    {}", air_top.join(", "));
    }
    eprintln!("  controls: +/- zoom, wasd/arrows rotate the globe, p provenance, Esc quit");
}

/// The interactive `--derived [star_mass] [orbit_au]` viewer: derive the planet from the star mass and orbit and show
/// it, the DERIVED planet alone (no [`WorldGenesis`], no [`LivingWorld`], no radiation, no occupants). Zoomed out it is
/// the star-lit globe (derived radius, blackbody star colour, day/night terminator, the derived Rayleigh atmosphere
/// limb); zoomed in it is the derived crust tiles coloured by [`render::material_surface_rgb`]. The window is a pure
/// observer, canon -> pixels (Principle 10).
/// The PROVENANCE BREAKDOWN of the derived crust tile: every derived property with its derivation chain and the
/// grade of each input, so hovering a tile SHOWS whether each value is derived, cited, or a labelled fixture. This
/// is the sloppy-work catcher the provenance stack exists for: a fixture (a not-yet-derived interim) is surfaced in
/// amber, never hidden. The cited grades `[M]` are the data files' own citations (AGSS09, NIST-JANAF,
/// Robie-Hemingway); the derivation grades `[D]` are structural (the named derivation functions); the floor
/// authoring-surface count is read LIVE from the provenance register ([`civsim_physics::floor_provenance`]), the
/// global honesty number. Nothing here is fabricated: each line is a real derivation, a cited source, or a labelled
/// fixture, so a value that is authored where it should be derived shows up as a fixture rather than a clean tag.
fn provenance_lines(
    scene: &DerivedScene,
    floor_prov: Option<&civsim_physics::floor_provenance::FloorProvenance>,
    hovered: Option<(usize, usize)>,
) -> Vec<(String, Rgb)> {
    let derived = Rgb::new(150, 220, 150); // green: DERIVED
    let cited = Rgb::new(150, 190, 235); // blue: [M] cited
    let fixture = Rgb::new(240, 200, 120); // amber: a labelled FIXTURE (the sloppy-work spot)
    let head = Rgb::new(238, 238, 238);
    let crust_phase = scene
        .crust_phases
        .first()
        .cloned()
        .unwrap_or_else(|| "unresolved".to_string());
    // The header keys off the cursor's surface tile: the (col,row) under the cursor when it sits on the sphere, or a
    // prompt when it is off the globe. The crust is uniform for now (a named geodynamics follow-on), so every tile
    // reads the same derivation; naming the tile makes the hover explicit and is ready for lateral variation.
    let header = match hovered {
        Some((cu, cv)) => format!("PROVENANCE  tile ({cu},{cv})  the derived crust"),
        None => "PROVENANCE  (hover the sphere to pick a tile)".to_string(),
    };
    let mut lines = vec![
        (header, head),
        (format!("material  {crust_phase}"), head),
        (
            "  <- condensation of AGSS09 abundances  [M cited]".to_string(),
            cited,
        ),
        (
            "  <- NIST-JANAF thermochemistry         [M cited]".to_string(),
            cited,
        ),
        (
            "  <- VCS amounts + differentiation      [D derived]".to_string(),
            derived,
        ),
        (
            "  <- buoyant partial-melt crust         [D derived]".to_string(),
            derived,
        ),
        ("elevation  (Airy isostasy)".to_string(), head),
        (
            "  <- crustal_density, molar volumes     [M Robie-Hemingway]".to_string(),
            cited,
        ),
        (
            "  <- isostatic float on the mantle      [D derived]".to_string(),
            derived,
        ),
        (
            format!(
                "colour  rgb({},{},{})",
                scene.material.r, scene.material.g, scene.material.b
            ),
            head,
        ),
        (
            "  <- absorption spectrum x star light   [D derived]".to_string(),
            derived,
        ),
        (
            "  <- band gap + crystal-field d-d line  [M cited]".to_string(),
            cited,
        ),
        ("FIXTURES surfaced (not yet derived):".to_string(), head),
        (
            "  formation-era T ~1400 K   [fixture: disk history]".to_string(),
            fixture,
        ),
        (
            "  accretion mass, bulk density  [fixture]".to_string(),
            fixture,
        ),
        (
            "  atmosphere volatile budget    [fixture #40]".to_string(),
            fixture,
        ),
    ];
    match floor_prov {
        Some(fp) => lines.push((
            format!(
                "floor authoring surface: {} authored",
                fp.authoring_surface().len()
            ),
            head,
        )),
        None => lines.push((
            "floor provenance register: unavailable".to_string(),
            fixture,
        )),
    }
    lines
}

/// The derived viewer's continuous globe scale at zoom fraction `t` in [0, 1]: the metres-per-pixel scale (so the
/// DERIVED radius drives the on-screen size), the star's on-screen position, and its on-screen radius. The globe
/// grows from a distant star-lit sphere (t = 0, the star beside it) to the surface filling the frame up close
/// (t = 1): one continuous zoom, all of it the sphere. The two radius endpoints are non-canon display-framing choices
/// for the viewer, documented at their site (Principle 10, pixels only).
fn derived_globe_view(fx: &GlobeFixture, w: usize, h: usize, t: f32) -> (Fixed, (i32, i32), usize) {
    let min_dim = w.min(h);
    // The on-screen radius grows geometrically so the zoom reads smoothly: from ~0.12 of the frame (a distant globe
    // with the star beside it) up to ~8x the frame (a close drill-down onto a small surface patch, where the tile grid
    // subdivides). Both fractions are non-canon display-framing choices, surfaced here at their definition.
    const R_MIN_FRAC: f32 = 0.12;
    const R_MAX_FRAC: f32 = 8.0;
    let frac = R_MIN_FRAC * (R_MAX_FRAC / R_MIN_FRAC).powf(t.clamp(0.0, 1.0));
    let target_r = ((min_dim as f32) * frac).max(1.0) as i32;
    let m_per_px = fx
        .radius_m
        .checked_div(Fixed::from_int(target_r.max(1)))
        .unwrap_or(Fixed::ONE);
    // The star sits off to the upper-left; it is occluded once the globe grows to fill the frame (you are at the
    // surface), the honest look for the close view.
    let star_px = ((w / 5) as i32, (h / 4) as i32);
    let star_r = (min_dim / 22).max(3);
    (m_per_px, star_px, star_r)
}

/// The display surface-tile grid `(cols, rows)` at an on-screen globe `radius_px` in a frame of smallest dimension
/// `min_dim`: a lat/lon grid whose density DOUBLES each time the globe doubles in on-screen size past the fill radius,
/// so as you zoom in each tile opens into a 2x2 finer array (a display level-of-detail). Below the fill radius it stays
/// at the base density (a coarse hover grid on the distant planet). The base density, the fill radius, and the
/// subdivision cap are non-canon display choices, documented here. The cell COLOUR stays the DERIVED material (uniform
/// until lateral composition variation lands, a geodynamics follow-on); the grid is a coordinate graticule showing the
/// tile structure the surface will carry, never fabricated per-cell content.
fn surface_grid_dims(radius_px: usize, min_dim: usize) -> (usize, usize) {
    const BASE_COLS: usize = 12;
    const BASE_ROWS: usize = 8;
    const MAX_LEVEL: u32 = 5; // cap the subdivision so the grid stays legible and cheap (12<<5 = 384 cols)
    let fill = (min_dim.max(1) as f32) * 0.5; // the radius at which the base grid density holds
    let ratio = (radius_px as f32 / fill).max(1.0);
    let level = (ratio.log2().floor().max(0.0) as u32).min(MAX_LEVEL);
    (BASE_COLS << level, BASE_ROWS << level)
}

fn run_derived(argv: &[String]) {
    let star_mass = parse_fixed(argv.get(2), Fixed::ONE);
    let orbit_au = parse_fixed(argv.get(3), Fixed::ONE);
    let scene = match build_derived_scene(star_mass, orbit_au) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("the derived planet did not resolve: {e}");
            eprintln!("(no planet is shown; nothing is fabricated)");
            return;
        }
    };
    print_derived_readout(&scene);

    // The globe fixture reuses the shared globe renderer over the derived scene's fields.
    let globe = GlobeFixture {
        radius_m: scene.radius_m,
        t_eff: scene.t_eff,
        tiles: scene.tiles.clone(),
        cols: scene.cols,
        sky: scene.sky,
    };
    // One continuous zoom, all of it the sphere: from the distant star-lit globe (zoom 0) through the surface filling
    // the frame and on into a close drill-down where the display tile grid subdivides. This replaces the old
    // flat-tileset surface levels, so the zoom-in stays ON the sphere's surface.
    let zoom_levels = 12u32;
    let max_zoom = zoom_levels.saturating_sub(1);

    let mut win_w = 960usize;
    let mut win_h = 640usize;
    let mut window = Window::new(
        "civsim derived-planet viewer",
        win_w,
        win_h,
        WindowOptions {
            // A titled, resizable window: a resize reflows the render each frame from window.get_size(). Under WSLg
            // the Wayland compositor supplies no server-side title-bar decoration (minifb 0.27 has no client-side
            // fallback), so drag via the WSLg / desktop window chrome (for example Super+drag); that is a compositor
            // limitation outside the viewer's control, not a viewer bug.
            title: true,
            resize: true,
            scale: Scale::X1,
            scale_mode: ScaleMode::Stretch,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| {
        eprintln!(
            "could not open a window: {e}\n\
             On WSL this needs WSLg (Windows 11) or an X server. The derived numbers printed above \
             still describe the planet."
        );
        std::process::exit(1);
    });
    window.set_target_fps(30);

    // The provenance register, loaded once for the hover panel's live authoring-surface count (fail-soft: the panel
    // shows "register unavailable" rather than fabricating a number).
    let floor_prov = civsim_physics::floor_provenance::FloorProvenance::embedded().ok();
    let mut show_provenance = false;
    let mut zoom: u32 = 0;
    // The globe orientation the pan keys steer: a longitude spin (wraps) and a latitude tilt (clamped off the poles).
    // Rotating brings the far side of the sphere into view, so the whole surface is reachable. Display-only state,
    // never canon (Principle 10).
    let mut rot_lon = 0.0f32;
    let mut rot_lat = 0.0f32;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        for k in window.get_keys_pressed(KeyRepeat::No) {
            match k {
                Key::Equal | Key::NumPadPlus => zoom = (zoom + 1).min(max_zoom),
                Key::Minus | Key::NumPadMinus => zoom = zoom.saturating_sub(1),
                // Toggle the tile provenance panel (the sloppy-work catcher).
                Key::P => show_provenance = !show_provenance,
                Key::Home => {
                    zoom = 0;
                    rot_lon = 0.0;
                    rot_lat = 0.0;
                }
                _ => {}
            }
        }
        // WASD / arrow keys ROTATE the globe: longitude about the polar axis (wraps, so a full spin reaches every
        // meridian) and latitude tilt (clamped near the poles to avoid the projection singularity). Held keys pan
        // smoothly. The step and the latitude limit are non-canon display choices, documented at their site.
        const ROT_STEP: f32 = 0.045; // radians per frame while a pan key is held (display pan-rate)
        const LAT_LIMIT: f32 = 1.4; // ~80 degrees: keep the sampled centre off the pole singularity
        use std::f32::consts::TAU;
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            rot_lon -= ROT_STEP;
        }
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            rot_lon += ROT_STEP;
        }
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            rot_lat = (rot_lat + ROT_STEP).min(LAT_LIMIT);
        }
        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            rot_lat = (rot_lat - ROT_STEP).max(-LAT_LIMIT);
        }
        rot_lon = rot_lon.rem_euclid(TAU); // longitude wraps

        let (w, h) = window.get_size();
        if w == 0 || h == 0 {
            window.update();
            continue;
        }
        (win_w, win_h) = (w, h);

        // One continuous zoom, all sphere: t runs 0 (distant globe) to 1 (surface fills the frame). The whole render
        // is the star-lit globe (derived radius, blackbody star colour, day/night terminator, atmosphere limb),
        // sampled at the current orientation so panning rotates the surface.
        let t = if max_zoom == 0 {
            1.0
        } else {
            zoom as f32 / max_zoom as f32
        };
        let orient = render::GlobeOrientation { rot_lon, rot_lat };
        let (m_per_px, star_px, star_r) = derived_globe_view(&globe, win_w, win_h, t);
        let radius_px = render::globe_radius_px(globe.radius_m, m_per_px);
        let min_dim = win_w.min(win_h);
        // The display tile grid refines with zoom: it holds a coarse base density on the distant planet, then doubles
        // each time the globe doubles in on-screen size, so once the sphere fills the frame each tile opens into a 2x2
        // finer array as you keep zooming in. It is overlaid once the sphere is large enough (the distant planet reads
        // smooth); the pick and highlight always use the current grid, so hovering marks a cell of the visible tiles.
        let (grid_cols, grid_rows) = surface_grid_dims(radius_px, min_dim);
        let show_grid = radius_px as f32 >= min_dim as f32 * 0.5;
        let style = render::SurfaceStyle {
            tint: Some(scene.material),
            grid: show_grid.then_some((grid_cols, grid_rows)),
        };
        let mut buf = render::render_solar_system_view(
            globe.radius_m,
            globe.t_eff,
            &globe.tiles,
            globe.cols,
            win_w,
            win_h,
            m_per_px,
            star_px,
            star_r,
            BG,
            globe.sky,
            style,
            orient,
        );
        let mode = if show_grid {
            format!(
                "derived surface  zoom {}/{}  tiles {grid_cols}x{grid_rows}",
                zoom + 1,
                zoom_levels
            )
        } else {
            format!("derived globe  zoom {}/{}", zoom + 1, zoom_levels)
        };

        // The cursor -> surface pick: invert the sphere map at the mouse to the tile under it (None off the sphere).
        // The globe is centred in the frame; its on-screen radius is the derived radius at this zoom.
        let gcx = (win_w / 2) as i32;
        let gcy = (win_h / 2) as i32;
        let picked = window
            .get_mouse_pos(MouseMode::Discard)
            .and_then(|(mx, my)| {
                render::pick_surface_tile(
                    mx as i32, my as i32, gcx, gcy, radius_px, orient, grid_cols, grid_rows,
                )
            });
        // The highlight box marks the tile under the cursor, projected onto the sphere so it curves with the globe
        // and stays put as it rotates. Fail-soft: no pick, no highlight.
        if let Some((cu, cv)) = picked {
            render::draw_surface_highlight(
                &mut buf, win_w, win_h, gcx, gcy, radius_px, orient, grid_cols, grid_rows, cu, cv,
                CURSOR,
            );
        }

        // The readout HUD: the derived numbers, drawn top-left, so the planet is legible without the terminal.
        let readout = format!(
            "star {:.2} Msun  orbit {:.2} AU  |  T_eff {:.0}K  radius {:.0}km  g {:.2}",
            scene.star_mass.to_f64_lossy(),
            scene.orbit_au.to_f64_lossy(),
            scene.t_eff.to_f64_lossy(),
            scene.radius_m.to_f64_lossy() / 1000.0,
            scene.gravity.to_f64_lossy(),
        );
        render::draw_label(
            &mut buf,
            win_w,
            win_h,
            4,
            4,
            &readout,
            2,
            Rgb::new(240, 240, 170),
            Rgb::new(10, 12, 20),
        );
        let crust_line: Vec<String> = scene
            .crust
            .iter()
            .take(4)
            .map(|(el, _)| el.clone())
            .collect();
        render::draw_label(
            &mut buf,
            win_w,
            win_h,
            4,
            20,
            &format!(
                "crust {}  air {}  |  +/- zoom  wasd/arrows rotate  p provenance  esc quit",
                crust_line.join("-"),
                scene
                    .atmosphere
                    .first()
                    .map(|(n, _)| n.as_str())
                    .unwrap_or("none")
            ),
            1,
            Rgb::new(170, 180, 200),
            Rgb::new(10, 12, 20),
        );
        // The provenance panel: on toggle, list the hovered crust tile's provenance breakdown on the right edge, so a
        // fixture stands out amber against the derived and cited grades. It keys off the cursor's surface tile.
        if show_provenance {
            let lines = provenance_lines(&scene, floor_prov.as_ref(), picked);
            let panel_w = 372usize;
            let px = win_w.saturating_sub(panel_w);
            for (i, (text, colour)) in lines.iter().enumerate() {
                let py = 40 + i * 15;
                if py + 12 < win_h {
                    render::draw_label(
                        &mut buf,
                        win_w,
                        win_h,
                        (px + 6) as i32,
                        py as i32,
                        text,
                        1,
                        *colour,
                        Rgb::new(10, 12, 20),
                    );
                }
            }
        }
        window.set_title(&format!(
            "civsim derived planet  star {:.2} Msun  orbit {:.2} AU  |  {mode}",
            scene.star_mass.to_f64_lossy(),
            scene.orbit_au.to_f64_lossy()
        ));
        window
            .update_with_buffer(&buf, win_w, win_h)
            .expect("blit the frame");
    }
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
    if argv.get(1).map(|s| s == "--derived-globe").unwrap_or(false) {
        derived_globe_cmd(&argv);
        return;
    }
    if argv.get(1).map(|s| s == "--globe").unwrap_or(false) {
        globe_cmd(&argv);
        return;
    }
    // Interactive derived-planet viewer: `--derived [star_mass] [orbit_au]` derives a planet from a star mass and an
    // orbit alone (no worldgen, no life) and shows it: the star-lit globe zoomed out, the derived crust tiles coloured
    // by their material's optics zoomed in. A pure observer, canon -> pixels (Principle 10).
    if argv.get(1).map(|s| s == "--derived").unwrap_or(false) {
        run_derived(&argv);
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
                    render::SurfaceStyle::default(),
                    render::GlobeOrientation::IDENTITY,
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

#[cfg(test)]
mod seam3_tests {
    use super::*;

    #[test]
    fn the_hadean_orbit_derives_a_silicate_crust_and_a_mars_class_planet() {
        // SEAM-3 Hadean gate: Sun + 1 AU. The crust condenses at the DERIVED formation midplane (~1400 K, close to the
        // retired fixture because 1 AU sits at the silicate front), a SILICATE (enstatite); the accreted mass is the
        // Mars-class isolation mass (NOT the retired 1 Earth); the uncompressed bulk density is Earth-uncompressed
        // grade (NOT the retired 5.514). The two epoch temperatures are distinct (condensation hot, surface warmth
        // cold), the join-law the seam enforces.
        let scene = build_derived_scene(Fixed::ONE, Fixed::ONE).expect("the Hadean scene derives");
        assert!(
            scene.crust_phases.iter().any(|p| p.contains("enstatite")),
            "the 1 AU crust is a silicate (enstatite), got {:?}",
            scene.crust_phases
        );
        let cond = scene.condensation_t.to_f64_lossy();
        let surf = scene.disk_t.to_f64_lossy();
        assert!(
            (1300.0..=1500.0).contains(&cond),
            "the derived formation condensation T lands ~1400 K, got {cond}"
        );
        assert!(
            surf < 400.0 && cond > surf + 500.0,
            "the condensation ({cond} K) and mature surface ({surf} K) are distinct epochs, never conflated"
        );
        let m = scene.mass_earth.to_f64_lossy();
        assert!(
            (0.03..=0.20).contains(&m),
            "the accretion isolation mass is Mars-class, got {m} M_earth"
        );
        assert!(
            (m - 1.0).abs() > 0.5,
            "the 1 M_earth mass fixture is retired (got {m})"
        );
        let rho = scene.density.to_f64_lossy();
        assert!(
            (3.5..=5.0).contains(&rho),
            "the uncompressed bulk density is in grade, got {rho} g/cm^3"
        );
        assert!(
            (rho - 5.514).abs() > 0.3,
            "the 5.514 g/cm^3 density fixture is retired (got {rho})"
        );
    }

    #[test]
    fn the_crust_is_orbit_dependent_a_close_orbit_condenses_the_more_refractory_assemblage() {
        // SEAM-3 free payoff: the crust is now ORBIT-DEPENDENT (the acceptance the two-temperature seam had blocked). A
        // CLOSE, hot formation midplane condenses the more refractory Al-Mg oxide (spinel); a FAR, cool one condenses
        // the Mg silicate (enstatite). The dominant crust phase differs, and the close orbit condensed hotter.
        let close = build_derived_scene(Fixed::ONE, Fixed::from_ratio(8, 10))
            .expect("the close scene derives");
        let far = build_derived_scene(Fixed::ONE, Fixed::from_ratio(15, 10))
            .expect("the far scene derives");
        assert_ne!(
            close.crust_phases, far.crust_phases,
            "the crust differs across orbit: close {:?} vs far {:?}",
            close.crust_phases, far.crust_phases
        );
        assert!(
            close.condensation_t.to_f64_lossy() > far.condensation_t.to_f64_lossy(),
            "the closer orbit condenses against a hotter midplane ({} vs {} K)",
            close.condensation_t.to_f64_lossy(),
            far.condensation_t.to_f64_lossy()
        );
        let has = |s: &DerivedScene, el: &str| s.crust.iter().any(|(e, _)| e == el);
        assert!(
            has(&close, "Al"),
            "the close refractory crust carries aluminium (spinel/corundum), got {:?}",
            close.crust_phases
        );
        assert!(
            has(&far, "Si"),
            "the far silicate crust carries silicon (enstatite), got {:?}",
            far.crust_phases
        );
    }
}
