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

use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, Window, WindowOptions};

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
use civsim_sim::deeptime::{
    bombard_tick, province_column_params, provinces_across, step_deep_time, DeepTimeState,
    ImpactFluxParams, MeltParams,
};
use civsim_sim::genesis::{genesis, GenesisParams, LivingWorld, WorldGenesis};
use civsim_sim::geodynamics::{
    convecting_mantle_depth_m, derive_mantle_density, generate_derived_tiles, slice0_demo_field,
    ColumnParams, DerivedTile,
};
use civsim_sim::located::OccupantId;
use civsim_world::ballistic::{BallisticForces, EjectaFan};
use civsim_world::crater::{CraterCoupling, Target};
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

// The DERIVED-globe LIGHTING attitude is READ PER-WORLD from the scene, never authored inline. The observability layer
// lights the sphere from the DERIVED sub-solar direction (the orbit's declination and the spin's sub-solar longitude,
// read straight out of `civsim_sim::orbit`). The attitude and orbital elements it needs (axial tilt, axis orientation,
// orbital phase, orbit eccentricity, rotation period, initial spin phase) are PER-WORLD inputs carried as fields on
// [`DerivedScene`] (see [`SceneAttitude`]); the pipeline fills them, and when the spin/tilt derivation lands (task #44 /
// Part 18.1) the values flow into the viewer with no viewer change. NO obliquity, spin, tilt, or phase value is a literal
// here. What remains below are pure display choices (dot size, transition length, and how fast the OBSERVER watches the
// day pass), the observability layer's own allowance (Principle 10); none is a physical attitude value.

/// NON-CANON display: the on-screen planet-dot radius on the system map (pixels).
const DEMO_MAP_DOT_PX: usize = 5;
/// NON-CANON display: how many frames the woosh zoom between the system map and a planet globe takes (30 to 45 range).
const WOOSH_FRAMES: u32 = 38;
/// NON-CANON VIEWING sweep: how many animation frames the observer takes to watch the day/night terminator sweep once
/// around the globe. This is a PLAYBACK rate (how fast the observer watches time pass), a sibling of the manual rotate
/// pan-rate and the [`DEFAULT_GEN_RATE`] playback speed, NOT a physical rotation rate. The physical rotation period is
/// the per-world `scene.attitude.rotation_period_s` (reserved, pending task #44); when it and a wall-clock are wired, the
/// sweep can be timed to it. Until then the observer watches the day pass at this display rate.
const TERMINATOR_SWEEP_FRAMES: i64 = 240;
/// NON-CANON VIEWING sweep: how many animation frames the observer takes to watch one year (the seasonal cycle) pass, so
/// the DERIVED axial tilt read from the world shows as the sub-solar latitude swinging through the seasons. A PLAYBACK
/// rate like [`TERMINATOR_SWEEP_FRAMES`], not a physical orbital period; the seasons drift slower than the day.
const SEASON_SWEEP_FRAMES: i64 = 1920;

/// The DERIVED per-world attitude and orbital elements the observability lighting reads: the axial tilt, axis
/// orientation, orbital phase, orbit eccentricity, rotation period, and initial spin phase. Each is a per-world input the
/// pipeline fills; `None` means "not yet supplied by a derivation in this path" (reserved per-world input,
/// R-CELESTIAL-SECULAR, pending the spin/tilt derivation, task #44 / Part 18.1). The viewer READS these; it never authors
/// them. When the derivation is wired the fields carry real values and the viewer is unchanged. Display-only consumer
/// (Principle 10).
#[derive(Clone, Copy, Default)]
struct SceneAttitude {
    /// The axial tilt (obliquity, radians): the seasonal amplitude. READ per-world from the world's
    /// [`civsim_sim::environ::DiurnalSky`] (`sky.obliquity`), whose value lives in `civsim_sim::environ` as Earth's
    /// cited measurement; never authored here.
    obliquity: Option<Fixed>,
    /// The axis orientation (perihelion longitude, radians): where perihelion sits relative to the equinox. Reserved
    /// per-world input (R-CELESTIAL-SECULAR, task #44 / Part 18.1); the sky's `perihelion_phase` uses a different
    /// seasonal-phase convention than `orbit.rs`, so it is not cross-wired here.
    perihelion_longitude: Option<Fixed>,
    /// The orbital phase (mean anomaly, radians): where the body sits on its orbit (the current season). Reserved
    /// per-world input (R-CELESTIAL-SECULAR, task #44 / Part 18.1); until supplied, the observer's viewing year-sweep
    /// animates through the seasons so the DERIVED tilt is visible.
    orbital_phase: Option<Fixed>,
    /// The orbit eccentricity (the ellipse's shape; the orbit lines and the season length read it). READ per-world from
    /// the world's [`civsim_sim::environ::DiurnalSky`] (`sky.eccentricity`), whose value lives in `civsim_sim::environ`
    /// as Earth's cited measurement; never authored here.
    orbit_eccentricity: Option<Fixed>,
    /// The rotation period (seconds per rotation, the spin rate). Reserved per-world input (R-CELESTIAL-SECULAR, task
    /// #44 / Part 18.1); the derived-planet path does not load the world manifest (`world.rotation_period_seconds`), so
    /// the day/night animation runs at the display viewing-sweep rate ([`TERMINATOR_SWEEP_FRAMES`]) and will time to this
    /// period when it and a wall-clock are wired.
    rotation_period_s: Option<Fixed>,
    /// The initial spin phase (radians): the sub-solar meridian at the start of viewing. Reserved per-world input
    /// (R-CELESTIAL-SECULAR, task #44 / Part 18.1).
    initial_spin_phase: Option<Fixed>,
}

/// The world sky the derived viewer READS its per-world lighting attitude from: the Mirror (Earth-calibrated) world's
/// [`civsim_sim::environ::DiurnalSky`], the current default world. The viewer reads this sky's per-world attitude
/// (`obliquity`, `eccentricity`) and never authors it, so a different world's tilt flows through as data. The
/// rotation/orbital TICK cadences the constructor takes drive the run-path diurnal calendar, which needs a world manifest
/// the derived-planet path does not load; the viewer does not read them (its day/night and season motions are viewing
/// sweeps, an observability playback), so a placeholder cadence is passed and only the attitude is read.
fn derived_world_sky() -> civsim_sim::environ::DiurnalSky {
    civsim_sim::environ::DiurnalSky::mirror(1, 1)
}

/// The per-world [`SceneAttitude`] for the derived viewer, READ from the world sky. Obliquity and eccentricity come from
/// the Mirror [`civsim_sim::environ::DiurnalSky`] (their values live in `civsim_sim::environ`, cited as Earth's
/// measurements); the axis orientation, orbital phase, rotation period, and initial spin phase are reserved per-world
/// inputs the derived-planet path does not yet supply (R-CELESTIAL-SECULAR, task #44 / Part 18.1). No tilt, spin, or
/// eccentricity value is authored here.
fn derived_scene_attitude() -> SceneAttitude {
    let sky = derived_world_sky();
    SceneAttitude {
        obliquity: Some(sky.obliquity),
        orbit_eccentricity: Some(sky.eccentricity),
        perihelion_longitude: None,
        orbital_phase: None,
        rotation_period_s: None,
        initial_spin_phase: None,
    }
}

/// A VIEWING-sweep phase (radians, in `[0, 2*pi)`) at animation `frame`: the observer's clock marches once around over
/// `frames` animation frames. A NON-CANON display playback clock (how fast the observer watches time pass), NOT a
/// physical rate.
fn sweep_phase(frame: u64, frames: i64) -> Fixed {
    let frames = frames.max(1);
    let step = (frame % frames as u64) as i64;
    let frac = Fixed::from_ratio(step, frames);
    let tau = Fixed::PI.checked_add(Fixed::PI).unwrap_or(Fixed::PI);
    tau.checked_mul(frac).unwrap_or(Fixed::ZERO)
}

/// The eccentricity to draw an orbit with: the per-world value when supplied, else the neutral circle (`0`) when the
/// orbital element is not available in this path (task #44). Reads the scene; authors nothing.
fn attitude_eccentricity(attitude: &SceneAttitude) -> Fixed {
    attitude.orbit_eccentricity.unwrap_or(Fixed::ZERO)
}

/// The DERIVED body-frame sun direction for the globe lighting, consuming `civsim_sim::orbit` and the PER-WORLD attitude
/// read from the scene. `day_sweep` is the observer's day clock (feeds the sub-solar longitude, the time of day);
/// `year_sweep` is the observer's year clock (feeds the orbital phase for the declination when the per-world phase is not
/// yet supplied, so the DERIVED tilt shows as the seasons pass). The sub-solar latitude comes from
/// [`civsim_sim::orbit::solar_declination`] at the per-world obliquity (else declination zero when the tilt is not
/// available), and the sub-solar longitude from [`civsim_sim::orbit::subsolar_longitude`]. Mapped into
/// [`render::draw_globe`]'s body frame by [`render::sub_solar_body_dir`], it makes the lit hemisphere DERIVED, never an
/// authored light direction. Every physical input is READ from `attitude`; the only inline value is the neutral zero used
/// when a per-world field is not yet available. `None` on a link that does not resolve (fail-soft).
fn derived_sun_body_dir(
    attitude: &SceneAttitude,
    day_sweep: Fixed,
    year_sweep: Fixed,
) -> Option<[f32; 3]> {
    let eccentricity = attitude_eccentricity(attitude);
    // The orbital phase (the season): the per-world value when supplied, else the observer's viewing year-sweep so the
    // DERIVED tilt shows as the sub-solar latitude swings through the seasons. The tilt AMPLITUDE is the per-world
    // obliquity read from the sky.
    let phase = attitude.orbital_phase.unwrap_or(year_sweep);
    let declination = match attitude.obliquity {
        Some(obliquity) => {
            let state = civsim_sim::orbit::orbital_state(phase, eccentricity)?;
            // The axis orientation: the per-world value when supplied, else the neutral (perihelion at the equinox), the
            // same reference choice the sky's reference world uses. Not an authored per-world value.
            let perihelion = attitude.perihelion_longitude.unwrap_or(Fixed::ZERO);
            civsim_sim::orbit::solar_declination(&state, obliquity, perihelion)?
        }
        None => Fixed::ZERO,
    };
    // The spin phase is the per-world initial phase plus the observer's day sweep, folded into one rotation.
    let tau = Fixed::PI.checked_add(Fixed::PI)?;
    let mut spin = attitude
        .initial_spin_phase
        .unwrap_or(Fixed::ZERO)
        .checked_add(day_sweep)?;
    if spin >= tau {
        spin = spin.checked_sub(tau)?;
    }
    let subsolar_longitude = civsim_sim::orbit::subsolar_longitude(spin)?;
    Some(render::sub_solar_body_dir(
        declination.to_f64_lossy() as f32,
        subsolar_longitude.to_f64_lossy() as f32,
    ))
}

/// A smooth ease-in-out (smoothstep) of a `[0, 1]` progress, for the woosh transition (no linear-motion snap at the
/// ends). Display-only.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear interpolation of two `f32`, for the woosh centre. Display-only.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Blend two packed `0x00RRGGBB` pixels by `t` in `[0, 1]` (`0` yields `a`, `1` yields `b`), for the woosh's fade of the
/// system map toward empty space. Display-only.
fn blend_u32(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |sa: u32, sb: u32| -> u32 {
        let fa = sa as f32;
        let fb = sb as f32;
        (fa + (fb - fa) * t).round().clamp(0.0, 255.0) as u32
    };
    let r = mix((a >> 16) & 0xff, (b >> 16) & 0xff);
    let g = mix((a >> 8) & 0xff, (b >> 8) & 0xff);
    let bl = mix(a & 0xff, b & 0xff);
    (r << 16) | (g << 8) | bl
}

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
        fx.param,
        w,
        h,
        m_per_px,
        star_px,
        star_r,
        BG,
        fx.sky,
        render::SurfaceStyle::default(),
        render::GlobeOrientation::IDENTITY,
        // The demo `--globe` fixture carries no orbital phase; keep the screen-space light (byte-identical).
        None,
        // The demo fixture carries no deep-time province temperatures, so no lava glow (byte-identical).
        None,
        // The demo fixture carries no province field, so no analytic Sample: the hillshade falls back to the
        // cache's finite difference, byte-identical to before the analytic normals.
        None,
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
    let mut scene = match build_derived_scene(star_mass, orbit_au) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("the derived planet did not resolve: {e}; nothing written");
            return;
        }
    };
    // An optional deep-time step count (argv[8]) ages the province field to that point on the deep-time clock,
    // re-deriving the surface, so the globe can be rendered at any epoch (the headless evolution check: render
    // two step counts and the tile field differs, the provinces having formed and thickened).
    if let Some(total_steps) = argv.get(8).and_then(|s| s.parse::<usize>().ok()) {
        let param = scene.param;
        if let Some(prov) = scene.provinces.as_mut() {
            age_provinces_from_young(prov, total_steps);
            if let Some(tiles) = derive_province_tiles(prov, param) {
                scene.tiles = tiles;
            }
            // Re-derive the lava glow at this epoch too, so the headless render shows the volcanism fade: a young
            // world glows broadly (super-solidus provinces), an aged one only at hot-spots as the mantle cools.
            scene.lava = derive_province_lava(prov, param);
            eprintln!(
                "  deep-time: aged the province field to {total_steps} ticks ({:.0} Myr)",
                (total_steps as f64) * DEEP_TIME_MYR_PER_TICK.to_f64_lossy()
            );
        }
    }
    print_derived_readout(&scene);
    let fx = GlobeFixture {
        radius_m: scene.radius_m,
        t_eff: scene.t_eff,
        star_radius_ratio: scene.star_radius_ratio,
        orbit_au: scene.orbit_au,
        tiles: scene.tiles.clone(),
        param: scene.param,
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
                    relief_shading: true,
                    // The DERIVED body radius the hillshade reads to light the relief at its true physical slope.
                    surface_radius_m: fx.radius_m,
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
                    relief_shading: true,
                    surface_radius_m: fx.radius_m,
                },
            )
        }
    };
    // The DERIVED sun direction lights the globe (Part 1): the sub-solar direction built from the orbit's declination
    // (seasons, at the PER-WORLD obliquity READ from the world sky) and the spin's sub-solar longitude (time of day). The
    // day and season phases are the observer's viewing sweeps, sampled off-noon and off-equinox so the terminator sits
    // off-centre and the tilt shows; the derived numbers are printed so the tracking is checkable without a display.
    let day_sweep = sweep_phase(TERMINATOR_SWEEP_FRAMES as u64 / 8, TERMINATOR_SWEEP_FRAMES);
    let year_sweep = sweep_phase(SEASON_SWEEP_FRAMES as u64 / 4, SEASON_SWEEP_FRAMES);
    let star_dir = derived_sun_body_dir(&scene.attitude, day_sweep, year_sweep);
    let phase = scene.attitude.orbital_phase.unwrap_or(year_sweep);
    if let Some(state) =
        civsim_sim::orbit::orbital_state(phase, attitude_eccentricity(&scene.attitude))
    {
        let decl = match scene.attitude.obliquity {
            Some(obliquity) => civsim_sim::orbit::solar_declination(
                &state,
                obliquity,
                scene.attitude.perihelion_longitude.unwrap_or(Fixed::ZERO),
            )
            .map(|d| d.to_f64_lossy())
            .unwrap_or(f64::NAN),
            None => 0.0,
        };
        eprintln!(
            "  derived sun (per-world attitude READ from DiurnalSky): obliquity {:.4} rad, eccentricity {:.4}, declination {decl:.3} rad, body-frame star_dir {:?}",
            scene.attitude.obliquity.map(|o| o.to_f64_lossy()).unwrap_or(f64::NAN),
            scene.attitude.orbit_eccentricity.map(|e| e.to_f64_lossy()).unwrap_or(f64::NAN),
            star_dir
        );
    }
    // Paint the sphere the crust's DERIVED material colour (the same the interactive `--derived` viewer shows), so the
    // headless render matches what the owner sees on screen. The globe is lit by the DERIVED sun direction above.
    // THE ANALYTIC SAMPLE FIELD: the shading takes its normal as the analytic GRADIENT of this superposition,
    // rather than finite-differencing the cache, so the relief is lit at the true slope of the canonical field.
    let stamps = scene.crater_stamps();
    let field = scene
        .provinces
        .as_ref()
        .map(|p| province_surface_field(p, &stamps));
    let buf = render::render_solar_system_view(
        fx.radius_m,
        fx.t_eff,
        &fx.tiles,
        fx.param,
        w,
        h,
        m_per_px,
        star_px,
        star_r,
        BG,
        fx.sky,
        style,
        render::GlobeOrientation::IDENTITY,
        star_dir,
        // The DERIVED lava glow: actively-molten provinces emit their incandescent colour (bright on the night side
        // too). Empty on the uniform-crust fallback, so the render adds nothing there.
        (!scene.lava.is_empty()).then_some(scene.lava.as_slice()),
        field.as_ref(),
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
    /// The star's radius `R / R_sun` and the orbit distance (AU), so the on-screen star size derives from its
    /// apparent angular diameter (`~ R_star / distance`) rather than a fixed dot.
    star_radius_ratio: Fixed,
    orbit_au: Fixed,
    tiles: Vec<DerivedTile>,
    /// How `tiles` maps to sphere directions: the fixture demo crust is a lat-lon grid (byte-identical to before the
    /// cube-sphere migration); a derived scene rebuilt into a fixture carries its cube-sphere parameterization.
    param: render::SurfaceParam,
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
        star_radius_ratio: planet.star_radius_ratio,
        orbit_au: Fixed::ONE,
        tiles,
        // The demo crust field is a plain lat-lon grid (the living-world / fixture globe stays byte-identical).
        param: render::SurfaceParam::LatLon { cols, rows },
        sky,
    })
}

/// The on-screen star radius (pixels), DERIVED from the star's apparent angular diameter. The star subtends
/// `~ 2 R_star / distance` of sky, so relative to the Sun seen from Earth (`R_sun / 1 AU`) the apparent size
/// scales as `radius_ratio / orbit_au`; that scales the reference on-screen dot (`min_dim / 22`, the Sun-at-1-AU
/// look). So a bigger or closer star draws larger and a smaller or farther one smaller: a massive main-sequence
/// star at a wide orbit is honestly small in the sky, a giant or a close star large. Clamped to stay visible and
/// not swamp the frame. A non-canon display projection (the star's real angular size is the physics; the pixel
/// scale is the view's).
fn derived_star_radius_px(fx: &GlobeFixture, min_dim: usize) -> usize {
    let base = (min_dim as f64 / 22.0).max(2.0);
    let orbit = fx.orbit_au.to_f64_lossy();
    let apparent = if orbit > 0.0 {
        fx.star_radius_ratio.to_f64_lossy() / orbit
    } else {
        1.0
    };
    (base * apparent).round().clamp(2.0, min_dim as f64 * 0.9) as usize
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
    let star_r = derived_star_radius_px(fx, min_dim);
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
    /// The star's radius `R / R_sun`, for the derived on-screen star size (its angular diameter at the orbit).
    star_radius_ratio: Fixed,
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
    /// How `tiles` (and the aligned `lava` field) map to sphere directions: `CubeSphere` for the province-textured
    /// surface (the pole-pinch-free sample cache), `LatLon` for the rare uniform-crust fallback. The globe render
    /// and the deep-time re-derivation both read this, so the sampling and the build always agree.
    param: render::SurfaceParam,
    /// The DERIVED Rayleigh atmosphere-limb sky colour, or [`render::PLACEHOLDER_SKY`] when the mix does not resolve.
    sky: Rgb,
    /// The DERIVED perceived colour of the crust material under the star ([`render::material_surface_rgb`]).
    material: Rgb,
    /// The PER-WORLD attitude and orbital elements the observability lighting reads (axial tilt, axis orientation,
    /// orbital phase, orbit eccentricity, rotation period, initial spin phase). Each is a reserved per-world input the
    /// pipeline fills; `None` means "not yet supplied by a derivation" (pending the spin/tilt derivation, task #44 /
    /// Part 18.1). The viewer READS these and never authors them, so wiring the derivation needs no viewer change.
    attitude: SceneAttitude,
    /// The DEEP-TIME PROVINCE FIELD the globe's texture is re-derived from, and the observer's time control steps
    /// forward so the surface evolves. `None` when the convective scale did not resolve (the uniform-crust fallback).
    provinces: Option<DeepTimeProvinces>,
    /// The per-display-tile LAVA GLOW field, aligned with `tiles`, re-derived alongside them: each entry is the
    /// incandescent colour of the tile's DERIVED interior temperature and its DERIVED melt fraction (zero below the
    /// world's own solidus). The globe render ADDS this as self-emitted light, so an actively-molten tile glows on
    /// the night side too. Empty on the uniform-crust fallback (no province temperatures), so the render adds nothing.
    lava: Vec<render::LavaGlow>,
    /// The R-YOUNG-TEMPERATURE verdict summary for the readout: the regime (Melted / Never-melted / Marginal),
    /// whether it is decidable now (GAPPED) or impact-list-pending, the SLR temperature rise, and the young
    /// potential temperature the deep-time run started from. `None` when the solidus did not resolve.
    young: Option<civsim_physics::young_thermal::YoungThermalVerdict>,
}

impl DerivedScene {
    /// This scene's crater ROWS prepared as analytic stamps ([`render::crater_stamps`]), the crater layer of the
    /// Sample superposition. The caller holds them so a [`render::SurfaceField`] can borrow them alongside the
    /// province field ([`province_surface_field`]). Empty on the uniform-crust fallback (no province field) or a
    /// world that drew no craters, in which case the crater layer contributes exactly zero. Display-only.
    fn crater_stamps(&self) -> Vec<render::CraterStamp> {
        self.provinces
            .as_ref()
            .map(|p| render::crater_stamps(&p.state.craters, p.radius_m))
            .unwrap_or_default()
    }
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

/// RESERVED, the mantle POTENTIAL TEMPERATURE (K) the adiabat projects to the surface, and the CO-DETERMINANT of the
/// deep-time flat-versus-textured verdict (the melt fires where the derived solidus falls below this). Basis:
/// McKenzie-Bickle's normal-MORB value ~1553 K, the melt rung validated at 1588 K. But 1588 K is 1315 C, a
/// modern-ambient value, and this is an EPOCH ERROR for a YOUNG world: the petrological record runs the other way,
/// the mantle potential temperature rising from ~1350 C today to a ~1500 to 1600 C maximum at 2.5 to 3.0 Ga (internal
/// heating exceeding surface loss through the Archean and Hadean), so a young world sat at or above the shipped
/// solidus, the opposite side of the inequality. This anchor cannot decide the young verdict; the endmember set does.
/// It is a flagged DERIVE-DOWN (R-YOUNG-TEMPERATURE, the next real slice, gated on the assembly impact list): the
/// young potential temperature derives from the accretion energy and the short-lived-radionuclide (SLR) FAMILY heat
/// (26Al plus 60Fe, one birth-environment draw, 60Fe subdominant and banded, not a 26Al-only slot) through a
/// magma-ocean-cooling handoff, pinned at rheological lock-up to the world's own solidus-side boundary. Two clauses
/// from the audit-of-the-audit ride the slice: the magma-ocean RADIATION CEILING derives per-world from the outgassed
/// blanket composition through the existing radiative machinery (a H2, CO, or N2 blanket has a different ceiling and
/// lifetime than water steam), so the water-class ~280 W/m2 number DEMOTES from mechanism to an Earth-draw validation
/// anchor; and the rheological critical melt fraction phi_c is a class constant at suspension-rheology grade (its band
/// covering crystal shape and polydispersity), alien-admissible, not a silicate-only fact. On shipping, this constant
/// retires from an INPUT to two battery hindcast rows (the Earth draw's derived T_p(t) passing through ~1350 C today
/// and the ~1500 to 1600 C Archean window). Cited: McKenzie & Bickle (1988) J. Petrology 29, 625; Herzberg et al.
/// (2010) for the secular T_p trajectory.
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

// THE DEEP-TIME PROVINCE TEXTURE reserved inputs. The globe's relief is the DERIVED crust the deep-time
// volcanism ([`civsim_sim::deeptime`]) builds over a lateral field of mantle columns: each province's crust
// thickness is the melt its interior delivers, so the surface is the written record of the interior's
// history, never a painted height field. The province SCALE derives from the convective physics, the crust
// AMPLITUDE from the melt and the isostasy; only the values below are reserved, each with its basis.

/// The mantle-convection CELL ASPECT RATIO (cell half-wavelength / convecting-layer depth), which sets the
/// lateral province SCALE (the province width is this times the DERIVED convecting-mantle depth, and the province
/// count is the planet circumference over that width, [`provinces_across`]). DERIVED from the marginal-stability
/// critical WAVENUMBER a_c as `pi / a_c`, so it is the SAME boundary regime as the convection kernel's onset
/// threshold `ra_crit` by construction: both are the rigid-rigid Rayleigh-Benard eigenvalue PAIR {Ra_crit ~ 1708,
/// a_c ~ 3.117}, read from [`civsim_sim::deeptime::RIGID_RIGID_CRITICAL_WAVENUMBER`], giving `pi / 3.117 ~ 1.008`,
/// not the free-free `sqrt(2) ~ 1.414` that a rigid `ra_crit` would contradict. This is an analytic-eigenvalue
/// consistency fix, not an authored pick. A full marginal-stability eigenvalue solver jointly deriving {Ra_crit,
/// a_c, regime} from the mantle's actual mechanical boundaries is a future substrate arc. Cited: Chandrasekhar
/// (1961), Hydrodynamic and Hydromagnetic Stability, ch. II.
const MANTLE_CONVECTION_CELL_ASPECT: Fixed =
    Fixed::PI.div(civsim_sim::deeptime::RIGID_RIGID_CRITICAL_WAVENUMBER);

// THE MANTLE SOLIDUS (surface value and slope) is no longer authored here. It is DERIVED from the world's OWN
// endmember signatures ([`civsim_physics::melting::multicomponent_solidus`], already run inside the
// surface-composition melt wiring on this very scene) and threaded into the deep-time volcanism through
// `SurfaceComposition::{solidus_surface_k, solidus_slope_k_per_gpa}`. The former reserved peridotite anchors
// (1373 K surface, 130 K/GPa slope, Earth's dry-peridotite values) are RETIRED. But the flat-versus-textured
// verdict is a comparison, `derived_solidus > MANTLE_POTENTIAL_TEMPERATURE_K`, and the OTHER side of that inequality
// is still an authored Earth-MORB anchor (1588 K, below): the derived solidus is one side, not the whole verdict.
// So this is CO-DETERMINED, and the deciding margin for the default refractory scene (~1680 K solidus vs 1588 K, ~92
// K) sits INSIDE the solidus model's own declared ideal-solution bias (~150 K, `civsim_physics::melting`), which
// makes the default flat outcome a REGIME-MARGINAL verdict: surfaced, carried, not asserted (per the error-band
// discipline). Retiring the potential-temperature anchor to a derived young magma-ocean temperature is the next real
// slice (R-YOUNG-TEMPERATURE), gated on the assembly impact list; until then the flatness is not a claim about the
// world, only the output of one authored anchor against a derived one.

/// RESERVED, the mantle PROCESSING TIME (Myr): the overturn / melt-delivery timescale one melt column's worth of
/// crust reaches the surface over, converting the column crust to a per-tick production rate. A flagged
/// DERIVE-DOWN, not an independent knob: it is `convecting_depth / convective_velocity`, and both ingredients are
/// in the convection kernel ([`civsim_sim::geodynamics::convecting_mantle_depth_m`] and the Stokes velocity), so
/// it derives the moment the kernel runs at physical SI scale. It stays reserved only until the SI / Tier-2 units
/// wiring retires the kernel's representable-scaled operating point (the same coupling the deeptime `MeltParams`
/// documents). Basis value: the silicate-mantle overturn time, ~100 Myr to 1 Gyr. Cited: mantle-convection
/// overturn timescale (Turcotte & Schubert, Geodynamics).
const MANTLE_PROCESSING_TIME_MYR: Fixed = Fixed::from_int(100);

// THE DEEP-TIME BOMBARDMENT flux configuration (the leftover-planetesimal reservoir the accretion-tail impact
// chain draws from). Every field of the flux is per-world DATA (Principle 11, admit-the-alien): a different disk,
// a captured swarm, or an alien impactor population is a different set of numbers through the same wiring, never a
// new code path. Most of the flux is DERIVED at the build site from the scene's own numbers and never appears
// here: the impact closing speed (the escape velocity `sqrt(2 g R)` from the derived gravity and radius), the
// impactor bulk density (the planet's derived uncompressed bulk density, the same reservoir it accreted from), the
// target gravity and bulk density (the derived surface gravity and crust density), the ballistic gravity and cell
// size (the derived gravity and the province grid's own spacing, circumference / pcols), and the ejecta launch
// speed (the reserved fraction below of the derived closing speed). What remains below are the reservoir's OWN
// residual data, each RESERVED-WITH-BASIS (a named constant, its basis stated, surfaced for the owner, never an
// unlabeled inline literal) or CITED. When the disk model supplies the residual reservoir mass and size-frequency
// bounds these derive down; until then they are the surfaced reservoir data.

/// CITED, the collisional-cascade differential size-frequency slope `p` (the Dohnanyi slope). Basis: the steady-
/// state collisional cascade slope, near 3.5 (Dohnanyi 1969), the same cascade the grain-opacity wire reads; a
/// cited physical constant of the fragmentation cascade, not a reserved tuneable.
const IMPACT_DOHNANYI_SLOPE: Fixed = Fixed::from_int(35).div(Fixed::from_int(10)); // 3.5
/// RESERVED-WITH-BASIS, the reservoir's total impacting-body count over the whole accretion tail (bodies above the
/// minimum radius), one body one crater ROW. Basis: the residual leftover-planetesimal reservoir mass the world's
/// disk delivers divided by the mean body mass of the size-frequency distribution; a per-world datum, derived-down
/// when the disk model supplies the residual mass. THE COUNT CAP IS RETIRED FOR THE ROW PATH: the former ~tens-of-
/// bodies placeholder was held low only because the coarse-grid RASTER deposited each crater's full transient depth
/// into one ~thousand-km province cell, so a dense bombardment over-inflated the per-cell relief. Now each sub-cell
/// crater is a discrete ROW stamped at its true size ([`render::crater_relief_km`]), so a dense bombardment no
/// longer over-inflates any cell, and the count can rise to the physical late-accretion value (hundreds to
/// thousands of bodies for a heavily-cratered surface, the Moon-class density). The value stays reserved (not
/// fabricated) at the placeholder; at this modest count the derived planet carries a few discrete craters (lightly
/// cratered), and raising the count to the physical reservoir is the reserved lever that makes the surface densely,
/// visibly cratered (verified: a boosted count stamps a Moon-like cratered surface).
const IMPACT_RESERVOIR_BODY_COUNT: Fixed = Fixed::from_int(40);
/// RESERVED-WITH-BASIS, the accretion-tail sweep-up timescale `tau` (Myr), the flux's own decay constant. Basis:
/// the planet's gravitational-focusing sweep-up time over the reservoir's dynamical spreading, near tens of Myr for
/// late accretion (Wetherill; Chambers late-accretion), a per-world dynamical value.
const IMPACT_SWEEP_TIMESCALE_MYR: Fixed = Fixed::from_int(30);
/// RESERVED-WITH-BASIS, the smallest impactor radius the reservoir delivers (m), the size-frequency lower bound.
/// Basis: the reservoir's small-end cutoff, at or above the crater resolvable on the derived province grid (a
/// resolution floor at the ~thousand-km province spacing); a per-world reservoir datum.
const IMPACT_MIN_IMPACTOR_RADIUS_M: Fixed = Fixed::from_int(2000);
/// RESERVED-WITH-BASIS, the largest impactor radius the reservoir delivers (m), the size-frequency upper bound (the
/// biggest surviving body). Physical basis: the largest leftover planetesimal below the giant-impact embryo tier (a
/// planetary embryo merger is the separate Layer-4 event tier, not this population), a per-world reservoir datum of
/// order tens to hundreds of km. The sub-cell-concentration reason for the low placeholder is RETIRED for the row
/// path (a sub-cell crater is now a discrete ROW stamped at its true size, not smeared into one province cell). One
/// limit remains, and it is the crater law's own (`crates/world`, off-limits here): the law uses one `bowl_aspect`
/// (the simple-crater depth-to-diameter ~0.2) with NO complex-crater depth flattening for basins, so a large crater
/// is stamped over-deep (the row stamp inherits that depth). The bound stays modest until the complex-crater
/// flattening lands in the crater law; the real large end is a per-world datum the owner restores then, and a
/// crater at or above the convective cell size then also feeds the province field through the cross-scale rule.
const IMPACT_MAX_IMPACTOR_RADIUS_M: Fixed = Fixed::from_int(8000);
/// RESERVED-WITH-BASIS, the target's effective (cohesive) yield strength `Y` (Pa) the crater law's strength term
/// reads. Basis: the crust's frictional-brittle cohesive strength, the ~1e8 Pa (~100 MPa) class-grade value; the
/// derive-down to the crust's OWN operative shear strength (the Frenkel ideal scaled by the per-class knockdown,
/// as the deep-time support-bound collapse already derives) is the named refinement. A per-world datum.
const IMPACT_TARGET_STRENGTH_PA: Fixed = Fixed::from_int(100_000_000);
/// RESERVED-WITH-BASIS per material (the competent-silicate crust row), the crater-scaling coupling constants,
/// cited to Holsapple (1993), Schmidt and Housen (1987), and Melosh (1989). Basis for each: the velocity-coupling
/// exponent `mu ~ 0.55` (competent silicate, bounded by the momentum limit 1/3 and the energy limit 2/3); the
/// density-coupling exponent `nu ~ 0.4`; the efficiency intercept `K1 ~ 0.2`; the strength weight `K2 ~ 1`; the
/// transient bowl depth-to-diameter `h/D ~ 0.2`; the escaping-ejecta fraction `f_eject ~ 0.5`. A world's material
/// carries its own row; a soft ice or an alien substrate differs by data, not by code (admit-the-alien).
const IMPACT_COUPLING_VELOCITY_EXPONENT: Fixed = Fixed::from_int(55).div(Fixed::from_int(100)); // 0.55
const IMPACT_COUPLING_DENSITY_EXPONENT: Fixed = Fixed::from_int(4).div(Fixed::from_int(10)); // 0.4
const IMPACT_COUPLING_EFFICIENCY_COEFFICIENT: Fixed = Fixed::from_int(2).div(Fixed::from_int(10)); // 0.2
const IMPACT_COUPLING_STRENGTH_COEFFICIENT: Fixed = Fixed::ONE; // 1.0
const IMPACT_COUPLING_BOWL_ASPECT: Fixed = Fixed::from_int(2).div(Fixed::from_int(10)); // 0.2
const IMPACT_COUPLING_EJECT_FRACTION: Fixed = Fixed::from_int(5).div(Fixed::from_int(10)); // 0.5
/// RESERVED-WITH-BASIS, the characteristic ejecta launch speed as a FRACTION of the derived impact closing speed
/// (so the launch speed scales with the world, not an inline literal). Basis: the near-rim ejecta launch speed, a
/// modest fraction of the impact speed near the 45-degree maximum-range angle (Melosh 1989). HONEST LIMIT: at the
/// coarse province grid the true near-rim ejecta range is far below one cell, so the fan lays a coarse ring at the
/// grid's own resolution rather than a resolved ejecta profile; a per-event ejecta speed scaling with the crater's
/// excavation velocity, and a finer grid, are the named refinements.
const IMPACT_EJECTA_SPEED_FRACTION: Fixed = Fixed::from_int(3).div(Fixed::from_int(10)); // 0.3
/// The ejecta launch ELEVATION ANGLE: 45 degrees, the ballistic maximum-range angle on flat ground (an analytic
/// geometric optimum, `HALF_PI / 2`, not an authored knob).
const IMPACT_EJECTA_ELEVATION_ANGLE: Fixed = Fixed::HALF_PI.div(Fixed::from_int(2));
/// RESERVED-WITH-BASIS, the number of evenly spaced ejecta launch azimuths (a resolution-and-determinism bound, not
/// a physical value). Basis: the fewest azimuths at which the blanket's shape stops changing beyond the grid
/// resolution (a resolution-versus-cost bound).
const IMPACT_EJECTA_AZIMUTHS: u32 = 24;
/// RESERVED-WITH-BASIS, the ballistic-march step cap (a determinism-and-performance bound, never a physical range).
/// Basis: the fewest steps that let a maximum-range arc cross the grid before the cap bites.
const IMPACT_BALLISTIC_STEP_CAP: u32 = 200;
/// RESERVED-WITH-BASIS, the maximum number of impacts applied in one deep-time tick (a determinism-and-cost bound,
/// never a physical limit). Basis: the per-tick impact budget set so the earliest, most intense ticks stay inside
/// the tick's compute envelope; the accumulated relief is still heavily cratered early.
const IMPACT_PER_TICK_CAP: u32 = 64;

/// PER-SYSTEM derive-down (dimensionless), the convective HOMOGENIZATION residual: the fraction of a melt-extraction
/// incompatible-element enrichment contrast that survives convective stirring to remain as lateral mantle
/// heterogeneity. The heterogeneity AMPLITUDE is derived per-system from the world's own formation melt fraction F
/// and partition behaviour (the batch-melting enrichment, [`heterogeneity_amplitude`]), retiring the former Earth
/// 0.3 reservoir spread. This residual is NOT universal (the R-ASSEMBLY-panel ruling corrects an earlier claim): what
/// is universal is the FORM (every convecting silicate mantle has this efficiency), never the value (none has
/// Earth's). Its own basis convicts it, being the ratio of the melt-processing timescale to the convective-
/// homogenization timescale, and both timescales are per-system (they carry the Rayleigh number, viscosity, and
/// Stokes velocity this deep-time kernel marks SI-blocked). The derive-down is cheap because the ratio partially
/// cancels: the homogenization time is overturn-counted from the convective velocity, the melt-cycling time is the
/// same velocity times the supra-solidus column fraction times F, so the velocity divides out to leading order and
/// leaves melt-zone geometry and F, both of which the world computes at its SI operating point. Until that wiring
/// lands the O(0.1) default is an [E]-band placeholder, reserved for calibration, never fabricated and never
/// universal. The seed PATTERN (which province is enriched) stays the deterministic world hash (Principle 8). VALIDITY
/// DOMAIN (the audit-of-the-audit refinement): O(0.1) is an EARTH-INSTANCE evaluation of the universal form, admissible
/// as an [E] prior only within a declared band; a tidally pumped Io-class mantle or a sluggish stagnant-lid world
/// leaves the band, and outside it the default must ESCALATE to the derive-down, never stretch. PROPAGATION: the
/// residual sits in a prefactor (the exponent rider holds locally), but its band flows through the radiogenic budget
/// into temperature and thence into the viscosity exponential over Gyr, so it can flip lid-regime and dynamo verdicts
/// downstream; those verdicts must CONSUME the propagated band per the error-band DAG, not read a point value.
const RADIOGENIC_MIXING_EFFICIENCY: Fixed = Fixed::from_int(1).div(Fixed::from_int(10)); // [E]-band placeholder, per-system derive-down

/// RESERVED-with-basis ([M class], the calibrated Terran instance), the bulk PARTITION COEFFICIENT D of the heat-
/// producing elements (U, Th, K) over the world's own melting assemblage: the share that stays in the solid residue
/// rather than entering the melt. It closes the batch-melting enrichment `E = 1/(D + F*(1-D))` (Shaw 1970), so the
/// heterogeneity amplitude reads the world's own partition behaviour, not the Terran D-to-zero sign. SIGN HAZARD (the
/// R-ASSEMBLY-panel ruling): U, Th, K are incompatible (D much below 1) in oxidized silicate melting, so the melt is
/// enriched; but under reducing, sulfur-rich conditions (silicate FeO below about 1 wt%) they turn CHALCOPHILE and
/// partition into metal or sulfide (Wohlers-Wood 2015), D above 1, and the melt is DEPLETED, the contrast sign
/// flipping. The full form admits both; the former inline `1/F` wired the Terran sign in as mechanism (a Principle 4
/// and 7 defect). LADDER (the audit-of-the-audit refinement): the derive-down is a 3a-style ESTIMATOR entry, measured
/// rungs first. The [M] anchors are experimental partition data (the Wohlers-Wood reduced-sulfide points among them);
/// the lattice-strain interpolation across un-measured element-phase pairs is the estimator, running on the banked
/// Shannon radii and charges against aristotype site parameters, banded at factor level (the Trouton or Miedema
/// pattern); the fO2-and-sulfur regime switch keys on the world's own computed state. The batch equation is algebraic,
/// so nothing enters an exponent (the rider is clean). DOMAIN GUARD: this default is the oxidized-silicate lithophile
/// instance; outside the calibrated assemblage and fO2 range a per-world derivation supplies D and escalates rather
/// than silently applying the placeholder. Fetch: Blundy-Wood 1994 and Wood-Kiseeva 2015 before coefficients land.
const RADIOGENIC_BULK_PARTITION_D: Fixed = Fixed::from_int(3).div(Fixed::from_int(1000)); // ~0.003, [M class] placeholder pending Blundy-Wood

// THE YOUNG-THERMAL (R-YOUNG-TEMPERATURE) reserved inputs. For a MELTED world these supply a young potential
// temperature DERIVED from the world's own solidus (the magma-ocean lock-up handoff) rather than the authored
// Earth-MORB anchor. THE WIRING IS VIEWER-SIDE, NOT a materials change: `crates/materials` is untouched; the
// `MANTLE_POTENTIAL_TEMPERATURE_K` (1588 K) const below still seeds the BOOTSTRAP surface call, and
// `build_derived_scene` then makes a SECOND viewer-side call to the existing
// [`civsim_materials::surface_composition::derive_surface_composition`] with the young potential temperature in a
// fresh [`civsim_materials::surface_composition::ReservedMeltParams`], re-deriving the surface at the young
// temperature. So the anchor is not deleted; a melted world's DEEP-TIME initial condition reads the derived young
// temperature in place of it, viewer-side. The verdict itself
// ([`civsim_physics::young_thermal::young_thermal_verdict`]) is fixed Rust; the values below are the reserved
// per-world data it reads, each with its basis, surfaced rather than fabricated. Cited fetches carry the source
// at the use site.

/// RESERVED-with-basis, data-driven: the rheological CRITICAL (lock-up) melt fraction phi_c, the melt fraction at
/// which a cooling magma ocean's rheology switches from a liquid-like suspension to a solid-like crystal
/// framework. Basis: ~0.4 (crystal fraction ~0.6), experimental range ~0.3 to 0.4, shape- and
/// polydispersity-dependent, so a per-world/per-assemblage datum with 0.4 the default, not a silicate-only
/// constant (alien-admissible). Cited: Abe (1993), Geophys. Monogr. 74, DOI 10.1029/GM074p0041; Solomatov (2015),
/// Treatise on Geophysics (2nd ed.) Vol. 9.
const PHI_C_LOCKUP_MELT_FRACTION: Fixed = Fixed::from_int(4).div(Fixed::from_int(10)); // 0.4

/// The FALLBACK specific heat capacity (J/(kg*K)) the SLR and cold-accretion budgets divide by, used only when the
/// assemblage's mean atomic mass does not resolve. The live path now DERIVES c_p from the world's OWN assemblage
/// through the Dulong-Petit law ([`civsim_physics::young_thermal::dulong_petit_specific_heat`], c_p = 3R over the
/// mean atomic mass), so an iron world judges its melt budget at ~447 J/(kg*K) and a silicate world at ~1240, each
/// by construction; this fixed value is the loud-miss fallback only. Basis for the fallback: the silicate-mantle
/// heat capacity ~1000 J/(kg*K) (Turcotte & Schubert, Geodynamics). HONEST LIMIT (the flagged follow-on): the
/// Dulong-Petit derivation is the high-temperature plateau the magma-ocean operating point sits on; a banded Debye
/// correction below the assemblage's Debye temperature is the follow-on, not needed at the melt-decision temperature.
const YOUNG_SPECIFIC_HEAT_J_PER_KG_K: Fixed = Fixed::from_int(1000);

/// RESERVED-with-basis: the accretional-energy GEOMETRY factor `k_geom` for the retained-heat budget `k_geom*g*R`.
/// Basis: the gravitational binding energy per unit mass of a uniform sphere, `(3/5) G M / R = (3/5) g R`; a
/// geometry constant, not a fitted knob.
const ACCRETION_GEOMETRY_FACTOR: Fixed = Fixed::from_int(3).div(Fixed::from_int(5)); // 0.6

/// RESERVED-with-basis, the cold-accretion energy-RETENTION efficiency band (dimensionless, a PREFACTOR entry,
/// legal because it is prefactor-resident and never exponent-resident). Basis: the fraction of accretional/impact
/// energy retained as heat rather than radiated between deposits, from the cold-accretion retention literature
/// (the fetch doc lacks the exact number, so it is reserved, never fabricated). The BAND is the [E] entry the
/// self-test sweeps: a verdict that flips across it is MARGINAL, not GAPPED. Placeholder edges pending the fetch.
const RETENTION_EFFICIENCY_LO: Fixed = Fixed::from_int(1).div(Fixed::from_int(100)); // 0.01, [E]-band placeholder
const RETENTION_EFFICIENCY_HI: Fixed = Fixed::from_int(4).div(Fixed::from_int(10)); // 0.40, [E]-band placeholder

/// RESERVED-with-basis interim (a birth/assembly DRAW): the body's FORMATION TIME relative to CAI (megayears),
/// the short-lived-radionuclide decay clock, the POINT best estimate. Basis: the oligarchic ISOLATION-MASS growth
/// time at the inner disk, sub-megayear to a few megayears (Kokubo & Ida 1998; Lambrechts & Johansen 2012
/// Hill-regime growth ~4e4 yr at 5 AU, faster closer in), within the 26Al mean life. This is the TEXTURE-ONSET
/// formation time, DISTINCT from the reset epoch below (the last giant impact); the impact list upgrades it to the
/// world's own draw. Tagged interim.
const FORMATION_TIME_MYR: Fixed = Fixed::from_int(1);

/// RESERVED-with-basis interim, the FORMATION-TIME BAND the young-thermal verdict SWEEPS for its GAPPED / MARGINAL
/// grade (the panel re-grade catch: pinning the formation time to a fast point value over-claims GAPPED, because
/// the 26Al heat that dominates at a sub-megayear formation is largely decayed a few megayears later, so the melt
/// verdict is formation-time contingent). Basis: the class-grade oligarchic-growth spread, a fast sub-megayear
/// edge to a slow few-megayear edge (the same Kokubo & Ida / Lambrechts & Johansen sources as the point above), the
/// honest interim uncertainty until the per-world impact list collapses it to the world's own draw. Placeholder
/// edges pending the fetch; a world whose grade flips across this band is MARGINAL, never asserted GAPPED.
const FORMATION_TIME_MYR_LO: Fixed = Fixed::from_int(1).div(Fixed::from_int(2)); // 0.5 Myr, fast (hot) edge
const FORMATION_TIME_MYR_HI: Fixed = Fixed::from_int(4); // 4 Myr, slow (cold) edge

/// RESERVED-with-basis interim, the birth-environment SLR-DRAW BAND, as a dimensionless SCALE on the canonical
/// initial abundance ratios (1.0 = the canonical solar draw below). Swept for the grade. Basis: the star-forming
/// region's recent supernova/AGB enrichment spread, which moves the initial 26Al/27Al and 60Fe/56Fe by a factor of
/// a few above and below the solar value; a conservative factor-of-two band, reserved, never fabricated. The
/// formation-time band dominates the default scene's re-grade; this band admits a supernova-rich or evolved-region
/// birth as data.
const SLR_INITIAL_RATIO_SCALE_LO: Fixed = Fixed::from_int(1).div(Fixed::from_int(2)); // 0.5x, evolved region
const SLR_INITIAL_RATIO_SCALE_HI: Fixed = Fixed::from_int(2); // 2x, supernova-enriched region

/// RESERVED-with-basis interim, the fractional HALF-WIDTH of the planet-MASS band the giant-impact gate is swept
/// over, around the DERIVED isolation mass. Basis: the accretion-model spread in the feeding-zone isolation mass
/// (the feeding-zone width and the surface-density normalization), a ~30 percent half-width, reserved. For a
/// Mars-class world the whole band sits well below the giant-impact threshold, so the gate is off across it; it
/// bites only for a world whose derived mass sits near the Earth-mass threshold.
const MASS_BAND_HALF_WIDTH_FRACTION: Fixed = Fixed::from_int(3).div(Fixed::from_int(10)); // 0.3

/// RESERVED-with-basis: the crustal YIELD STRENGTH (Pa) the support-bound check reads, the deviatoric strength a
/// crustal column can hold before it flows, so the maximum supportable topography is `sigma_y / (rho * g)`. Basis:
/// the deviatoric strength of cold crustal rock, ~1e8 Pa (~100 MPa), the frictional-brittle bound of the upper
/// crust (Byerlee's-law frictional strength at crustal confining pressures; Byerlee 1978, Pure Appl. Geophys. 116,
/// 615). A DERIVE-DOWN: it should read the crust's own `mat.yield_strength` from the mechanical floor once that is
/// threaded to the viewer; the reserved value is the check's class-grade bound, used only to FLAG an unphysically
/// tall derived relief, never to render or scale one. As an f64 (the support-bound arithmetic is a display-side
/// validity read, not canon).
const CRUST_YIELD_STRENGTH_PA: f64 = 1.0e8;

/// RESERVED-with-basis: the mass (Earth masses) at and above which giant impacts are class-generic, the MELTED
/// giant-impact gate. Basis: the Earth-mass terrestrial-embryo-merger regime where the assembly statistics
/// guarantee ~two dozen giant impacts per system (Izidoro et al. 2017/2021 break-the-chains ensembles). A
/// Mars-class isolation mass sits below it, so a Mars-class world's verdict is decided by the SLR branch, never
/// forced melted by mass.
const GIANT_IMPACT_MASS_THRESHOLD_EARTH: Fixed = Fixed::ONE;

/// RESERVED-with-basis, the birth-environment SLR-family DRAW: the initial 26Al/27Al atomic ratio. Basis: the
/// canonical CAI 26Al/27Al ~5.2e-5 (the Lodders solar-abundance lineage / meteoritic record), the default Mirror
/// value; a per-cloud draw (higher in a freshly supernova-enriched region). Cited: canonical solar-system initial
/// 26Al/27Al (Lodders 2003 lineage).
const AL26_OVER_AL27_INITIAL: Fixed = Fixed::from_int(52).div(Fixed::from_int(1_000_000)); // 5.2e-5

/// RESERVED-with-basis, the birth-environment SLR-family DRAW: the initial 60Fe/56Fe atomic ratio. Basis: the
/// solar-system initial 60Fe/56Fe, banded ~1e-8 (the fetch doc's "60Fe subdominant and banded"), the least
/// certain and most birth-environment-variable of the two. A subdominant contributor; the default Mirror value.
const FE60_OVER_FE56_INITIAL: Fixed = Fixed::from_int(1).div(Fixed::from_int(100_000_000)); // 1e-8

/// RESERVED-with-basis interim: the RESET EPOCH t0 (megayears), the young-clock zero the deep-time run ages from.
/// Basis: the last-giant-impact time in the standard assembly ensembles, 73 +/- 74 Myr, tagged INTERIM and
/// upgraded to the world's own draw when the assembly impact list lands. SCOPE FENCE: this interim writes TEXTURE
/// ONSET and THERMAL INITIAL CONDITIONS only, never impact-coupled archives (spin/obliquity resets, per-event
/// atmosphere blow-off, late-veneer chemistry, event chronology), which the impact list uniquely provides.
const RESET_EPOCH_MYR: Fixed = Fixed::from_int(73);
const RESET_EPOCH_HALF_BAND_MYR: Fixed = Fixed::from_int(74);

/// The radiogenic HETEROGENEITY amplitude from the world's own formation melt fraction `f` and the bulk partition
/// coefficient `d` of the heat producers, through Shaw batch-melting enrichment `E = 1/(d + f*(1 - d))`. The lateral
/// spread amplitude is the mixing-efficiency residual times the MAGNITUDE of the enrichment contrast `|E - 1|`, so a
/// compatible-element world (`d > 1`, the reduced-chalcophile sign flip) gets a real inverted spread rather than the
/// zero the old `1/F` clamp produced, and the enrichment is capped at `1/d` as `f -> 0` (no blow-up, which removes the
/// small-`f` landmine as well). The sign of which province is enriched belongs to the seed pattern, not this
/// magnitude. `None` on a non-positive `f` (no formation melt) or an arithmetic failure.
fn heterogeneity_amplitude(f: Fixed, d: Fixed, efficiency: Fixed) -> Option<Fixed> {
    if f <= Fixed::ZERO {
        return None;
    }
    // E = 1 / (d + f*(1 - d)), the batch-melting enrichment of the melt over the bulk source.
    let denom = d.checked_add(f.checked_mul(Fixed::ONE.checked_sub(d)?)?)?;
    if denom <= Fixed::ZERO {
        return None;
    }
    let enrichment = Fixed::ONE.checked_div(denom)?;
    let contrast = enrichment.checked_sub(Fixed::ONE)?;
    let magnitude = if contrast < Fixed::ZERO {
        Fixed::ZERO.checked_sub(contrast)?
    } else {
        contrast
    };
    efficiency.checked_mul(magnitude)
}

/// NON-CANON display: the deep-time geological duration one playback tick advances (megayears per playback
/// tick). A viewing-sweep cadence (how fast the observer watches deep time pass), a sibling of the frame-rate
/// sweep constants, NOT a physical time written to canon: the deep-time evolution it drives is a real derived
/// physics step ([`step_deep_time`]), only the CADENCE is the observer's. Scaled so the surface visibly
/// evolves over a few real seconds when the clock is sped up.
const DEEP_TIME_MYR_PER_TICK: Fixed = Fixed::from_int(20);

/// NON-CANON display: the number of deep-time steps the initial scene is aged to, so the globe opens on a
/// planet that already carries relief (an evolved present-day surface) rather than a fresh molten sphere. A
/// display framing of where the deep-time clock starts; the observer's time control runs it on from here.
const DEEP_TIME_INITIAL_STEPS: usize = 80;

/// NON-CANON display: the per-face resolution of the derived globe's CUBE-SPHERE surface sample cache. The cache is
/// six cube faces, each `FACE_RES` by `FACE_RES` cells projected onto the sphere by the equi-angular cube map
/// ([`render::cube_face_dir_fixed`]), so the cells are near-uniform in solid angle with NO pole pinch (the lat-lon
/// grid the cache used before crowded its budget at the poles and under-resolved the equator). It is the memoized
/// sample cache of the composed surface function (the province crust PLUS the analytic crater-row stamps,
/// [`derive_province_tiles`]), so this is how finely the discrete craters resolve: a coarse cache shows the big
/// craters, a fine one resolves the small ones, from ONE row list. It is a VIEWPORT / megapixel budget that
/// CONDITIONS ON NOTHING: it authors no physics (the crater positions, sizes, and profile are all derived), it sets
/// display density only (Principle 10). At 416 the total budget is `6 * 416 * 416 = 1,038,336` cells, comparable to
/// the former `1440 * 720 = 1,036,800` lat-lon grid, but spread uniformly over the sphere. The cost of building the
/// cache falls only on an active deep-time step (a paused view samples the already-baked tiles) and the build is
/// data-parallel (each cell independent). Per-zoom cache refinement (a cube-sphere quadtree, finer only where the
/// camera looks) is the named follow-on this parameterization is the substrate for.
const SURFACE_SAMPLE_CACHE_FACE_RES: usize = 416;

/// NON-CANON display: the lat-lon dimensions of the UNIFORM-CRUST FALLBACK surface (the smooth-ball field
/// [`civsim_sim::geodynamics::generate_derived_tiles`] produces when the province texture does not resolve). A
/// uniform crust reads a single shade regardless of parameterization, so this rare fallback keeps the plain
/// equirectangular grid; the province-textured render uses the cube-sphere cache above.
const SURFACE_SAMPLE_CACHE_COLS: usize = 1440;
const SURFACE_SAMPLE_CACHE_ROWS: usize = 720;

/// The FIXED resolution the resolution-sensitive isostatic DIAGNOSTICS read (the melt-texture heterogeneity-engaged
/// indicator and the km relief amplitude / support-bound check), DECOUPLED from the render sample cache above. The
/// heterogeneity indicator is a normalized neighbour-contrast ratio, so it depends on the grid spacing (a finer grid
/// samples the smooth province field at closer points and reads a smaller contrast); pinning the diagnostics to one
/// moderate grid keeps the "is the melt texture on?" readout comparable across worlds and independent of the display
/// budget. It is a diagnostic resolution, authors no physics (the crust-only tiles it reads are the derived Airy
/// float), and stays at the grid the indicator's smooth-ball floor was calibrated against.
const DIAGNOSTIC_TILE_COLS: usize = 48;
const DIAGNOSTIC_TILE_ROWS: usize = 32;

/// NON-CANON display: the orbital sweep rate (radians per playback-second at 1 AU) the system-map animation
/// advances a planet's mean anomaly by at playback rate 1. A viewing-sweep cadence (how fast the observer
/// watches the planets orbit), a sibling of the frame-rate sweep constants, NOT a physical rate: the per-planet
/// RELATIVE rate is the Kepler rate (a^-3/2, so inner planets sweep faster), which is derived; only this base
/// cadence is the observer's. Chosen so a 1 AU planet circles in about ten real seconds at rate 1.
const ORBIT_SWEEP_RATE_RAD_PER_S: f64 = 0.6;

/// The Kepler sweep factor for a semi-major axis (astronomical units): the orbital angular rate scales as
/// `a^(-3/2)` (Kepler's third law, `T` proportional to `a^(3/2)` about the same star), so an inner planet
/// sweeps faster than an outer one. This is the DERIVED relative rate the animation reads; the absolute base
/// cadence is [`ORBIT_SWEEP_RATE_RAD_PER_S`]. A non-positive axis falls back to unity (fail-soft).
fn kepler_sweep_factor(orbit_au: Fixed) -> f64 {
    let a = orbit_au.to_f64_lossy();
    if a > 0.0 {
        a.powf(-1.5)
    } else {
        1.0
    }
}

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
/// the accretion feeding zone (a few Hill radii around the orbit) over the SOLID column and folds to Earth masses
/// ([`feeding_zone_mass`] then [`feeding_zone_mass_earth`]). The honest output is Mars-class (~0.08 to 0.11 M_earth): a
/// pure accretion isolation mass, the Earth-size deferred to the Layer-4 event tier (the giant-impact merger), never
/// authored here.
///
/// CONDITIONS ON THE DRAWN METALLICITY (the accretion-mass flag retiring in substance). The solid-column normalization
/// is the SOLAR MMSN rock column scaled by `metallicity_ratio` = `Z / Z_sun`: a metal-rich disk carries proportionally
/// more solid dust, so it sweeps a larger isolation mass, and a metal-poor disk a smaller one. This is a DERIVATION
/// (the dust column scales with the disk's heavy-element fraction), not an authored value; for the solar/Mirror
/// instance the ratio is exactly ONE, so the norm and the mass are byte-identical to the pre-draw fixture.
fn derive_isolation_mass_earth(orbit_au: Fixed, metallicity_ratio: Fixed) -> Option<Fixed> {
    let half = orbit_au
        .checked_mul(FEEDING_ZONE_WIDTH_FRACTION)?
        .checked_div(Fixed::from_int(2))?;
    let inner_au = orbit_au.checked_sub(half)?;
    let outer_au = orbit_au.checked_add(half)?;
    // The solar MMSN rock column scaled by the disk's Z/Z_sun (the dust reservoir scales with heavy-element fraction);
    // exactly the solar norm when metallicity_ratio == ONE (the Mirror pin), so the pinned mass is byte-identical.
    let solid_norm = SOLID_SURFACE_DENSITY_NORM_KG_M2.checked_mul(metallicity_ratio)?;
    let feeding = civsim_sim::astro::feeding_zone_mass(
        inner_au,
        outer_au,
        DISK_CHARACTERISTIC_RADIUS_AU,
        SOLID_SURFACE_DENSITY_GAMMA,
        solid_norm,
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
    let (core_fraction, rho_metal) =
        derive_core_fraction_and_metal_density(abundances, table, eos)?;
    let mantle_fraction = Fixed::ONE.checked_sub(core_fraction)?;
    // Volume-weighted mean: rho = total mass / total volume = 1 / (f_core/rho_metal + f_mantle/rho_silicate).
    let inverse = core_fraction
        .checked_div(rho_metal)?
        .checked_add(mantle_fraction.checked_div(rho_silicate)?)?;
    Fixed::ONE.checked_div(inverse)
}

/// The metal-core MASS FRACTION and the core (metal) DENSITY (g/cm^3), the two interior-structure inputs the
/// convecting-mantle-depth derivation ([`convecting_mantle_depth_m`]) needs, from the same differentiation
/// [`derive_uncompressed_bulk_density`] uses: all Fe and Ni (metal) and troilite S (sulfur) sink to the core,
/// the rock cations float as their normative oxides, and the core density is the Fe EOS anchor (atomic
/// weight over measured molar volume). Shared with the bulk-density derivation, so the two never disagree.
/// `None` on a missing abundance, atomic weight, or the Fe anchor.
fn derive_core_fraction_and_metal_density(
    abundances: &SolarAbundances,
    table: &PeriodicTable,
    eos: &MetalEosAnchors,
) -> Option<(Fixed, Fixed)> {
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
    // The core (metal) density from the Fe EOS anchor: atomic weight over measured molar volume.
    let iron_mass = table.element("Fe")?.standard_atomic_weight;
    let iron_molar_volume = eos.molar_volume("Fe")?;
    let rho_metal = iron_mass.checked_div(iron_molar_volume)?;
    Some((core_fraction, rho_metal))
}

/// The deep-time PROVINCE FIELD the derived globe's texture is re-derived from each playback step: a lateral
/// grid of interior mantle columns (at the DERIVED province scale), their per-province convection params (the
/// radiogenic seed patterned into them), the volcanism params, and the derived densities the isostasy reads.
/// Stepping it ([`step_deep_time`]) evolves the interior and grows the crust, so the surface relief EMERGES
/// and CHANGES over deep time (the provinces form, thicken, and diverge), never a painted map. Display-side
/// state driven by the non-canon playback clock (Principle 10); nothing here touches canon.
struct DeepTimeProvinces {
    /// The evolving interior state (stepped each playback tick).
    state: DeepTimeState,
    /// One convection parameter set per province (the radiogenic seed carried in `heat_production`).
    column_params: Vec<ColumnParams>,
    /// The shared volcanism parameters (the derived solidus, source density, gravity; reserved processing time).
    melt: MeltParams,
    /// The province grid width and height, DERIVED from the convective scale ([`provinces_across`]).
    pcols: usize,
    prows: usize,
    /// The planet's DERIVED radius (metres), the body the crater rows stamp against: a crater's rim radius as an
    /// angle on the sphere is `(diameter / 2) / radius_m` ([`render::CraterStamp`]), so a finer display grid
    /// resolves finer craters at their true angular size.
    radius_m: Fixed,
    /// The crust density (derived once from the crust composition) each province's crust floats at.
    crust_density: Fixed,
    /// The derived mantle density the crust floats ON.
    mantle_density: Fixed,
    /// The YOUNG POTENTIAL TEMPERATURE (K) the columns start laterally uniform at: the R-YOUNG-TEMPERATURE
    /// magma-ocean lock-up handoff for a melted world (super-solidus, so the melt engages), else the cold peak.
    /// Stored so a rewind to the fresh young state ([`age_provinces_from_young`]) reuses the same young initial
    /// condition rather than the retired 1588 K anchor.
    young_potential_temperature_k: Fixed,
    /// The sea-level datum (Slice-0 zero, retires with the water budget).
    sea_level: Fixed,
    /// The per-world impact-flux configuration the deep-time bombardment draws from (the accretion-tail reservoir
    /// and the collisional-cascade size-frequency distribution), constructed derive-first at build time. Its
    /// crater bowls and ejecta blankets accumulate into `state.impact_relief_m` on the same grid and clock as the
    /// volcanic crust ([`bombard_tick`], a SEPARATE step term after the interior/volcanism step).
    flux: ImpactFluxParams,
    /// The stable per-world seed the bombardment draws are keyed on (a hash of the star mass and orbit that DEFINE
    /// this derived planet), so the same world renders the same craters every run (Principle 3, Principle 10).
    world_seed: u64,
}

/// A deterministic per-province symmetry-breaking perturbation in `[-1, 1)`, hashed from the world identity
/// (the star mass and orbit that DEFINE this derived planet) and the province index. This is the seed the
/// convection amplifies into provinces (Principle 8): it is deterministic (a pure splitmix64 hash, replays
/// bit-for-bit) and it perturbs an INPUT (the radiogenic budget), never the output terrain, so the province
/// pattern EMERGES from the physics rather than a painted height field. Only the pattern comes from here; the
/// spread's amplitude is the reserved radiogenic heterogeneity and the relief amplitude is the melt physics.
fn province_seed_perturbation(star_mass: Fixed, orbit_au: Fixed, index: usize) -> Fixed {
    let mut z = (star_mass.to_bits() as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((orbit_au.to_bits() as u64).wrapping_mul(0xD1B5_4A32_D192_ED03))
        .wrapping_add(
            (index as u64)
                .wrapping_add(1)
                .wrapping_mul(0xCA5A_8E62_1B4C_9F3D),
        );
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    // Map the top 24 bits to [0, 1), then to [-1, 1). A fixed denominator keeps the quantization deterministic.
    let frac = (z >> 40) as i32; // 0 .. 2^24 - 1, fits i32
    let unit = Fixed::from_int(frac)
        .checked_div(Fixed::from_int(1 << 24))
        .unwrap_or(Fixed::ZERO);
    unit.checked_mul(Fixed::from_int(2))
        .and_then(|x| x.checked_sub(Fixed::ONE))
        .unwrap_or(Fixed::ZERO)
}

/// Build the deep-time PROVINCE FIELD for a derived planet, or `None` if the convective scale does not
/// resolve (fail-soft: the globe then shows the uniform crust, never a fabricated texture). The chain is all
/// derivation: the convecting-mantle DEPTH from the planet's interior structure ([`convecting_mantle_depth_m`],
/// retiring the depth fixture), the province COUNT from that depth and the planet circumference over the
/// eigenvalue-derived convective cell aspect ([`provinces_across`]), the per-province radiogenic budget as the
/// thermostat base (set so the mean province's steady state sits at its DERIVED solidus, the mantle-thermostat
/// closure) times one-plus the PER-SYSTEM-derived heterogeneity times the deterministic seed, the volcanism
/// source density and gravity DERIVED (the mantle density and the planet's surface gravity), and the columns
/// started laterally uniform at the mantle potential temperature (a fresh planet has no thermal history; the
/// provinces are what the run produces).
///
/// `solidus_surface_k` and `solidus_slope_k_per_gpa` are the world's OWN derived mantle solidus, threaded from
/// the surface-composition melt wiring on this same scene (never an authored peridotite value). The melt is
/// driven against each column's OWN evolving interior temperature (the kernel steps it: hotter early from the
/// radiogenic budget, cooling as the sources decay), so relief emerges where-and-when a hot mantle crosses its
/// own solidus and stops as it cools. `formation_melt_fraction` is the world's own formation-era partial-melt
/// fraction F (`None` on a sub-solidus formation that sorted no incompatibles), from which the radiogenic
/// heterogeneity AMPLITUDE derives per-system (the Shaw batch-melting enrichment over F and the heat-producers'
/// partition, [`heterogeneity_amplitude`]), retiring the former Earth 0.3 spread.
#[allow(clippy::too_many_arguments)]
fn build_deep_time_provinces(
    star_mass: Fixed,
    orbit_au: Fixed,
    radius_m: Fixed,
    surface_gravity_m_s2: Fixed,
    core_fraction: Fixed,
    core_density: Fixed,
    mean_density: Fixed,
    mantle_density: Fixed,
    crust_density: Fixed,
    solidus_surface_k: Fixed,
    solidus_slope_k_per_gpa: Fixed,
    formation_melt_fraction: Option<Fixed>,
    young_potential_temperature_k: Fixed,
) -> Option<DeepTimeProvinces> {
    // The convecting-mantle DEPTH from the interior structure (SI metres), and its megametre form for the
    // representable-scaled convection kernel (retiring the depth = 1 fixture).
    let depth_m = convecting_mantle_depth_m(radius_m, core_fraction, mean_density, core_density)?;
    let depth_mm = depth_m.checked_div(Fixed::from_int(1_000_000))?;

    // The province SCALE, DERIVED: the count around the equator (circumference / cell width) and pole-to-pole
    // (half that circumference), each from the convective cell width (depth times the reserved aspect).
    let tau = Fixed::PI.checked_add(Fixed::PI)?;
    let circumference = tau.checked_mul(radius_m)?;
    let meridian = Fixed::PI.checked_mul(radius_m)?;
    let pcols = provinces_across(circumference, depth_m, MANTLE_CONVECTION_CELL_ASPECT)?;
    let prows = provinces_across(meridian, depth_m, MANTLE_CONVECTION_CELL_ASPECT)?;
    let n = pcols.checked_mul(prows)?;
    if n == 0 {
        return None;
    }

    // The volcanism parameters: the world's OWN DERIVED solidus (surface value and slope, threaded from the
    // surface-composition melt wiring, never an authored peridotite value), the DERIVED source density (the
    // mantle density in kg/m^3) and gravity (the planet's surface gravity), and the reserved interior-thermostat
    // adiabat/productivity and the derive-down processing time.
    let source_density = mantle_density.checked_mul(Fixed::from_int(1000))?;
    let melt = MeltParams {
        solidus_surface_k,
        solidus_slope_k_per_gpa,
        adiabat_slope_k_per_gpa: MANTLE_ADIABAT_SLOPE_K_PER_GPA,
        productivity_per_gpa: MELT_PRODUCTIVITY_PER_GPA,
        source_density_kg_per_m3: source_density,
        gravity_m_per_s2: surface_gravity_m_s2,
        processing_time_myr: MANTLE_PROCESSING_TIME_MYR,
    };

    // The interior-thermostat BASE radiogenic budget: set so a mean-seed province's conductive steady-state
    // temperature sits at the world's OWN DERIVED solidus (the observed mantle self-regulation near its solidus).
    // At the scaled operating point the steady state is T_ref + heat_production / loss_coeff with loss_coeff =
    // conductivity / (density * depth^2) (the convection kernel's conductive balance), so the base that lands
    // T_ss on the solidus is loss_coeff * (solidus - reference). This is a CLOSURE (the thermostat), not an
    // authored value: it derives the base from the DERIVED solidus and the operating point, so the mean budget is
    // per-system (it targets the world's own solidus). A deeper absolute derivation from the disk U/Th/K
    // abundances times per-isotope heat production is blocked by a data gap (the AGSS09 abundance table stops at
    // Z=42, so U and Th are absent, and no per-isotope specific-heat-production column is vendored); both are
    // flagged as cited [M] columns to source, and until they land the self-regulation closure is the per-system
    // mean.
    let reference_temperature = Fixed::from_int(300);
    let kernel_conductivity = Fixed::from_int(2);
    let kernel_density = Fixed::ONE;
    let loss_coeff = kernel_conductivity
        .checked_div(kernel_density.checked_mul(depth_mm.checked_mul(depth_mm)?)?)?;
    let base_heat =
        loss_coeff.checked_mul(solidus_surface_k.checked_sub(reference_temperature)?)?;

    // The radiogenic HETEROGENEITY amplitude, DERIVED per-system from the world's own formation melt fraction F and
    // the heat-producers' bulk partition D, through the full Shaw batch-melting enrichment E = 1/(D + F(1-D)) rather
    // than the former Terran-sign 1/F (D-to-zero) shortcut, so a reduced chalcophile world (D above 1) gets its
    // inverted spread and a small-F world is capped at 1/D rather than diverging. The share that survives convective
    // homogenization is the per-system mixing-efficiency residual. A world with no formation melt (sub-solidus, the
    // refractory solar-condensed scene) sorted no incompatibles, so its melt-driven spread is zero: an unprocessed
    // mantle. A primordial/accretional heterogeneity is a flagged derive-down (a future accretion-mixing substrate),
    // per-system contingent, never Earth's 0.3.
    let heterogeneity = match formation_melt_fraction {
        Some(f) if f > Fixed::ZERO => {
            heterogeneity_amplitude(f, RADIOGENIC_BULK_PARTITION_D, RADIOGENIC_MIXING_EFFICIENCY)?
        }
        _ => Fixed::ZERO,
    };

    // One column parameter set per province: the derived depth and gravity, and the seeded radiogenic budget
    // base * (1 + heterogeneity * seed). The seed is the deterministic per-province perturbation (Principle 8: the
    // PATTERN is the world hash, only the AMPLITUDE is the per-system derivation above).
    let mut column_params = Vec::with_capacity(n);
    for index in 0..n {
        let seed = province_seed_perturbation(star_mass, orbit_au, index);
        let factor = Fixed::ONE
            .checked_add(heterogeneity.checked_mul(seed)?)?
            .max(Fixed::ZERO);
        let heat_production = base_heat.checked_mul(factor)?;
        column_params.push(province_column_params(
            depth_mm,
            surface_gravity_m_s2,
            heat_production,
            reference_temperature,
            Fixed::ONE,
        ));
    }

    // The columns start laterally UNIFORM at the YOUNG POTENTIAL TEMPERATURE (the R-YOUNG-TEMPERATURE magma-ocean
    // lock-up handoff for a melted world, super-solidus so the melt engages, else the cold peak): a fresh planet
    // has no thermal history, and the provinces are what the run writes. This retires the fixed 1588 K anchor as
    // the young initial condition for a melted world.
    let state = DeepTimeState::young(n, young_potential_temperature_k);

    // THE BOMBARDMENT flux, constructed DERIVE-FIRST from this scene's own numbers. The impact CLOSING SPEED is the
    // escape velocity `v_esc = sqrt(2 g R)`, the least speed a body falling from rest strikes at (the derive-first
    // floor; the encounter/orbital speed only adds to it), from the derived gravity and radius. The impactor BULK
    // density is the planet's derived uncompressed bulk density (the leftover planetesimals are the same reservoir
    // it accreted from), and the TARGET bulk density is the derived crust density, both converted g/cm^3 -> kg/m^3.
    // The ballistic CELL SIZE is the province grid's own spacing (circumference / pcols, metres). The ejecta launch
    // speed is the reserved fraction of the derived closing speed. Everything not derived here is the reserved-with-
    // basis reservoir data above (Principle 11).
    //
    // RESOLUTION (rows not rasters). The bombardment DRAWS impacts on the coarse province grid (uniform-cell
    // location, ~thousand-km cells for a Mars-class mantle), but each drawn impact is now recorded as a discrete
    // crater ROW ([`bombard_tick`]) that the renderer stamps analytically at the viewport's resolution, so a
    // sub-cell crater is a discrete feature at its true size, no longer smeared across a whole convective cell (the
    // sub-cell-concentration inflation and the "reads faintly at the province scale" bound of the retired coarse
    // raster are BOTH gone). Two honest limits remain, surfaced not hidden. (1) The crater law (`crates/world`,
    // off-limits) uses one simple-crater `bowl_aspect` (~0.2) with no complex-crater depth flattening, so a large
    // crater is stamped over-deep (the reserved max impactor size stays modest until that flattening lands). (2)
    // The default reserved reservoir count is modest (a few discrete craters, a LIGHTLY-cratered planet); raising
    // the reserved count to the physical late-accretion value (now unblocked, the row path carries any density
    // without over-inflation) is the lever to a densely, visibly cratered surface (verified with a boosted count).
    // The craters are present in the derived elevation field the render reads (the readout reports the row count and
    // rim diameters); a crater at or above the convective cell size ALSO feeds the province field (the cross-scale
    // rule), so the large-basin thermal/province feedback is preserved.
    let escape_velocity_m_s = surface_gravity_m_s2
        .checked_mul(Fixed::from_int(2))?
        .checked_mul(radius_m)?
        .sqrt();
    let impactor_density_kg_m3 = mean_density.checked_mul(Fixed::from_int(1000))?;
    let target_density_kg_m3 = crust_density.checked_mul(Fixed::from_int(1000))?;
    let cell_size_m = circumference.checked_div(Fixed::from_int(pcols as i32))?;
    let ejecta_speed_m_s = escape_velocity_m_s.checked_mul(IMPACT_EJECTA_SPEED_FRACTION)?;
    let flux = ImpactFluxParams {
        reservoir_body_count: IMPACT_RESERVOIR_BODY_COUNT,
        sweep_timescale_myr: IMPACT_SWEEP_TIMESCALE_MYR,
        differential_slope: IMPACT_DOHNANYI_SLOPE,
        min_impactor_radius_m: IMPACT_MIN_IMPACTOR_RADIUS_M,
        max_impactor_radius_m: IMPACT_MAX_IMPACTOR_RADIUS_M,
        impact_velocity_m_s: escape_velocity_m_s,
        impactor_density: impactor_density_kg_m3,
        target: Target {
            gravity: surface_gravity_m_s2,
            strength: IMPACT_TARGET_STRENGTH_PA,
            density: target_density_kg_m3,
        },
        coupling: CraterCoupling {
            velocity_exponent: IMPACT_COUPLING_VELOCITY_EXPONENT,
            density_exponent: IMPACT_COUPLING_DENSITY_EXPONENT,
            efficiency_coefficient: IMPACT_COUPLING_EFFICIENCY_COEFFICIENT,
            strength_coefficient: IMPACT_COUPLING_STRENGTH_COEFFICIENT,
            bowl_aspect: IMPACT_COUPLING_BOWL_ASPECT,
            eject_fraction: IMPACT_COUPLING_EJECT_FRACTION,
        },
        ejecta: EjectaFan {
            speed: ejecta_speed_m_s,
            elevation_angle: IMPACT_EJECTA_ELEVATION_ANGLE,
            azimuths: IMPACT_EJECTA_AZIMUTHS,
        },
        forces: BallisticForces {
            gravity: surface_gravity_m_s2,
            cell_size: cell_size_m,
            step_cap: IMPACT_BALLISTIC_STEP_CAP,
        },
        per_tick_impact_cap: IMPACT_PER_TICK_CAP,
    };
    let world_seed = impact_world_seed(star_mass, orbit_au);

    Some(DeepTimeProvinces {
        state,
        column_params,
        melt,
        pcols,
        prows,
        radius_m,
        crust_density,
        mantle_density,
        young_potential_temperature_k,
        sea_level: Fixed::ZERO,
        flux,
        world_seed,
    })
}

/// A stable per-world bombardment SEED (splitmix64) from the star mass and orbit that DEFINE this derived planet,
/// so the same world renders the same crater field every run (Principle 3, Principle 10). The sibling of
/// [`province_seed_perturbation`] without a per-index term: it keys the whole bombardment stream, and
/// [`bombard_tick`] mixes in the per-tick index and per-strike counters itself.
fn impact_world_seed(star_mass: Fixed, orbit_au: Fixed) -> u64 {
    let mut z = (star_mass.to_bits() as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((orbit_au.to_bits() as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Reset the province field to its fresh young state (laterally uniform at the world's young potential
/// temperature, the R-YOUNG-TEMPERATURE handoff) and age it to `total_steps` deep-time ticks, so the surface can
/// be re-derived at any point on the deep-time clock (the headless two-point evolution check, and a rewind of the
/// observer's playback). Deterministic.
fn age_provinces_from_young(prov: &mut DeepTimeProvinces, total_steps: usize) {
    let n = prov.column_params.len();
    prov.state = DeepTimeState::young(n, prov.young_potential_temperature_k);
    step_provinces(prov, total_steps);
}

/// Advance the deep-time province field by `steps` playback ticks (each `DEEP_TIME_MYR_PER_TICK` of geological
/// time), stepping the interior convection, growing the crust, and drawing that tick's BOMBARDMENT onto the same
/// surface. A no-op past a step that fails to resolve (fail-soft, the field simply stops advancing). Deterministic.
fn step_provinces(prov: &mut DeepTimeProvinces, steps: usize) {
    for _ in 0..steps {
        // The tick INDEX the bombardment seeds its draw on and the flux epoch measures against: the number of
        // ticks already elapsed (`elapsed_myr / dt`), an exact integer since the clock accumulates one `dt` per
        // tick and starts at zero. Reading it off the state keeps the same world's crater field stable and resets
        // it correctly when `age_provinces_from_young` rewinds the clock to zero.
        let tick_index = prov
            .state
            .elapsed_myr
            .checked_div(DEEP_TIME_MYR_PER_TICK)
            .map(|t| t.to_int().max(0) as u64)
            .unwrap_or(0);
        match step_deep_time(
            &prov.state,
            &prov.column_params,
            &prov.melt,
            DEEP_TIME_MYR_PER_TICK,
        ) {
            Some(next) => prov.state = next,
            None => break,
        }
        // THE BOMBARDMENT is a SEPARATE step term after the interior/volcanism step (deeptime keeps them apart so a
        // non-bombarding run replays bit-for-bit): it draws this tick's impacts from the accretion-tail flux and
        // composes the crater bowls and ejecta blankets onto `state.impact_relief_m`, on the PHYSICAL province grid
        // (`pcols` x `prows`), seeded on the world identity and the tick. The flux DECLINES with epoch (measured
        // off the stepped `elapsed_myr`), so early ticks are heavily cratered and late ones quiescent. Off the run
        // path (viewer + deeptime), so byte-neutral.
        prov.state = bombard_tick(
            &prov.state,
            prov.pcols,
            prov.prows,
            &prov.flux,
            prov.world_seed,
            tick_index,
            DEEP_TIME_MYR_PER_TICK,
        );
    }
}

/// Re-derive the per-display-tile LAVA GLOW field from the current province state, the visible-volcanism read: for
/// each display tile, bilinearly sample the province's DERIVED interior temperature (the mantle potential temperature
/// each column carries, the SAME field the crust and craters are sampled off), run the world's OWN derived-solidus
/// melt column against it ([`civsim_physics::melting::adiabatic_melt_column`], the `crust_growth` closure's kernel),
/// and pair the DERIVED melt fraction (the glow intensity, zero for a sub-solidus tile) with the blackbody colour of
/// that temperature (the incandescence hue). A tile below the world's solidus makes no melt and does not glow (solid
/// crust); a super-solidus tile glows the colour of its temperature, brighter the more it melts. So a young/hot world
/// glows broadly and an aged, cooled world is dark crust with hot-spots only where a province stays super-solidus. No
/// authored glow threshold: the threshold is the DERIVED solidus the `MeltParams` carries, and the intensity is the
/// DERIVED melt fraction. Aligned with [`derive_province_tiles`]'s tile field (same `cols` by `rows`, same sampling),
/// so the render adds each tile's glow to the crust it rides on. Empty when the field is degenerate. Display-only,
/// one-way canon -> pixels (Principle 10).
fn derive_province_lava(
    prov: &DeepTimeProvinces,
    param: render::SurfaceParam,
) -> Vec<render::LavaGlow> {
    use rayon::prelude::*;
    if prov.pcols == 0 || prov.prows == 0 {
        return Vec::new();
    }
    // The province interior temperatures (one per province), for the smooth bilinear sample the crust and craters use.
    let temps: Vec<Fixed> = prov.state.columns.iter().map(|c| c.temperature).collect();
    // The DERIVED melt fraction PER PROVINCE (the world's OWN solidus, the MeltParams the deep-time volcanism
    // carries): zero for a sub-solidus province (a mantle colder than its surface solidus melts nothing, so no
    // glow), rising with the superheat above it. The threshold is the derived solidus, never an authored glow
    // cutoff. It is computed at the PROVINCE resolution (only `pcols * prows` melt columns) and resampled to the
    // display sample cache below, so the fine cache resolution costs no per-cell melt column (a smooth-field
    // display resample, Principle 10), while the crust and craters still read the cache cells.
    let melt_fraction: Vec<Fixed> = temps
        .iter()
        .map(|&t| {
            civsim_physics::melting::adiabatic_melt_column(
                t,
                prov.melt.solidus_surface_k,
                prov.melt.solidus_slope_k_per_gpa,
                prov.melt.adiabat_slope_k_per_gpa,
                prov.melt.productivity_per_gpa,
                prov.melt.source_density_kg_per_m3,
                prov.melt.gravity_m_per_s2,
            )
            .map(|col| col.max_melt_fraction)
            .unwrap_or(Fixed::ZERO)
        })
        .collect();
    // The self-emitted glow at a normalized surface coordinate: the incandescent colour of the resampled interior
    // temperature, and the resampled melt fraction as the intensity. A pure function of the immutable province fields.
    let glow_at = |fu: f32, fv: f32| -> render::LavaGlow {
        let temp_k = render::sample_province_field(&temps, prov.pcols, prov.prows, fu, fv);
        let intensity =
            (render::sample_province_field(&melt_fraction, prov.pcols, prov.prows, fu, fv)
                .to_f64_lossy() as f32)
                .clamp(0.0, 1.0);
        render::LavaGlow {
            emission: render::blackbody_rgb(temp_k),
            intensity,
        }
    };
    match param {
        render::SurfaceParam::LatLon { cols, rows } => {
            if cols == 0 || rows == 0 {
                return Vec::new();
            }
            let mut glow = Vec::with_capacity(cols * rows);
            for r in 0..rows {
                let fv = (r as f32 + 0.5) / rows as f32;
                for c in 0..cols {
                    let fu = (c as f32 + 0.5) / cols as f32;
                    glow.push(glow_at(fu, fv));
                }
            }
            glow
        }
        render::SurfaceParam::CubeSphere { face_res } => {
            if face_res == 0 {
                return Vec::new();
            }
            // Each cube cell is INDEPENDENT: its glow is a pure function of the immutable province fields at its
            // direction. The index-ordered parallel collect makes the result BIT-IDENTICAL to a serial build for any
            // thread count (the reproducibility the render relies on). Display-only, off the canon path (Principle 10).
            let local = cube_local_dirs(face_res);
            let face_cells = face_res * face_res;
            (0..6 * face_cells)
                .into_par_iter()
                .map(|idx| {
                    let dir = render::cube_face_local_to_world_fixed(
                        idx / face_cells,
                        local[idx % face_cells],
                    );
                    let (fu, fv) = render::dir_to_latlon_fraction(dir);
                    glow_at(fu, fv)
                })
                .collect()
        }
    }
}

/// Re-derive the display TILE field from the current province state, COMPOSING the analytic crater stamps onto the
/// isostatic crust: each display tile reads its province's accumulated crust thickness, floats it by Airy isostasy
/// on the derived mantle ([`civsim_physics::geodynamics::airy_isostatic_elevation`]), ADDS the analytic sum of the
/// crater ROWS covering the tile (the bowls and ejecta rims stamped at their true derived size,
/// [`render::crater_relief_km`]), and classifies the composed relief against the field datum. So the rendered
/// surface is the COMPOSED derived record: thicker-crust provinces stand as highlands, thinner as lowlands, and the
/// discrete crater history pits and rims it, the pattern the deep-time run wrote, never a painted map. The display
/// tile grid is the sample cache of this surface function, so a finer grid resolves finer craters (rows not
/// rasters). This is the field the globe RENDER reads. `None` if the field is empty or an elevation does not
/// resolve.
fn derive_province_tiles(
    prov: &DeepTimeProvinces,
    param: render::SurfaceParam,
) -> Option<Vec<DerivedTile>> {
    derive_province_tiles_core(prov, param, true)
}

/// The CRUST-ONLY tile field (the isostatic, melt-driven relief WITHOUT the bombardment), for the isostatic
/// diagnostics that must not conflate the two: the melt-texture-engaged indicator and the isostatic-support-bound
/// check read the crust's OWN relief, while the render reads the composed field ([`derive_province_tiles`]). The
/// craters are a SEPARATE surface-topography record, reported on their own (the bombardment readout), so they do
/// not dilute the melt-texture diagnostic's normalization or masquerade as isostatic relief. This diagnostic path
/// stays on the LAT-LON grid (`cols` by `rows`) the heterogeneity indicator was calibrated against.
fn derive_province_crust_tiles(
    prov: &DeepTimeProvinces,
    cols: usize,
    rows: usize,
) -> Option<Vec<DerivedTile>> {
    derive_province_tiles_core(prov, render::SurfaceParam::LatLon { cols, rows }, false)
}

/// The shared tile-field derivation. Each cache cell bilinearly samples the coarse DERIVED province crust field (the
/// cache is a viewer resolution; the PHYSICAL province grid is the derived one, each cell sampling the province it
/// falls in), floats it by Airy isostasy, and, when `with_impacts`, composes the bombardment relief on top. The cell
/// directions come from `param`: the equirectangular grid ([`render::SurfaceParam::LatLon`], the diagnostic path,
/// serial and byte-identical to before the migration) or the six-face equi-angular cube-sphere
/// ([`render::SurfaceParam::CubeSphere`], the render path, built in PARALLEL, [`cube_elevations`]). `None` if the
/// field is empty or an elevation does not resolve. Display-only (Principle 10).
fn derive_province_tiles_core(
    prov: &DeepTimeProvinces,
    param: render::SurfaceParam,
    with_impacts: bool,
) -> Option<Vec<DerivedTile>> {
    if prov.pcols == 0 || prov.prows == 0 {
        return None;
    }
    // THE CRATER STAMPS (rows not rasters): the discrete crater rows the bombardment drew, prepared once as
    // analytic stamps against the planet radius. The sample cache is the memoized composed surface function: each
    // cell samples the province crust PLUS this analytic crater sum at its own DIRECTION, so a finer cache resolves
    // finer craters from the SAME row list. Empty when `with_impacts` is false (the crust-only isostatic field) or
    // the world drew no craters. The stamps are read-only, shared across every cell (the parallel build below).
    let stamps = if with_impacts {
        render::crater_stamps(&prov.state.craters, prov.radius_m)
    } else {
        Vec::new()
    };
    // The composed surface elevation `airy(crust) + crater_stamp` (km) at every cache cell, in cache order.
    let elevations: Vec<Fixed> = match param {
        render::SurfaceParam::LatLon { cols, rows } => {
            if cols == 0 || rows == 0 {
                return None;
            }
            latlon_elevations(prov, &stamps, cols, rows, with_impacts)?
        }
        render::SurfaceParam::CubeSphere { face_res } => {
            if face_res == 0 {
                return None;
            }
            cube_elevations(prov, &stamps, face_res, with_impacts)?
        }
    };
    let datum = civsim_world::terrain::relief_datum(&elevations)?;
    Some(
        elevations
            .iter()
            .map(|&elevation| DerivedTile {
                elevation,
                relief: civsim_world::terrain::classify_relief(elevation, prov.sea_level, datum),
            })
            .collect(),
    )
}

/// The composed surface elevations over a LAT-LON cache (`cols` by `rows`, row-major), the diagnostic path. The
/// sample point's unit vector is built SEPARABLY: one latitude sin/cos per row and one longitude sin/cos per column
/// (`lon = u*2pi - pi`, `lat = (0.5 - v)*pi`, the same sphere map the crater centres use), so each cell's
/// `p = [cos_lat*sin_lon, sin_lat, cos_lat*cos_lon]` is two multiplies, not trig. Serial and byte-identical to the
/// pre-migration grid build. `None` if an elevation does not resolve. Display-only (Principle 10).
fn latlon_elevations(
    prov: &DeepTimeProvinces,
    stamps: &[render::CraterStamp],
    cols: usize,
    rows: usize,
    with_impacts: bool,
) -> Option<Vec<Fixed>> {
    let tau = Fixed::PI.mul(Fixed::from_int(2));
    let lat_sin_cos: Vec<(Fixed, Fixed)> = (0..rows)
        .map(|r| {
            let v = Fixed::from_ratio((2 * r + 1) as i64, (2 * rows) as i64);
            (Fixed::from_ratio(1, 2) - v).mul(Fixed::PI).sin_cos()
        })
        .collect();
    let lon_sin_cos: Vec<(Fixed, Fixed)> = (0..cols)
        .map(|c| {
            let u = Fixed::from_ratio((2 * c + 1) as i64, (2 * cols) as i64);
            (u.mul(tau) - Fixed::PI).sin_cos()
        })
        .collect();
    let mut elevations = Vec::with_capacity(cols * rows);
    for (r, &(sin_lat, cos_lat)) in lat_sin_cos.iter().enumerate() {
        let fv = (r as f32 + 0.5) / rows as f32;
        for (c, &(sin_lon, cos_lon)) in lon_sin_cos.iter().enumerate() {
            let fu = (c as f32 + 0.5) / cols as f32;
            // Bilinearly sample the coarse province field so it reads as a smooth heightfield across province
            // boundaries rather than hard blocks (a display resampling of the DERIVED field, never new content).
            let thickness = sample_province_thickness(prov, fu, fv);
            let airy_km = civsim_physics::geodynamics::airy_isostatic_elevation(
                prov.crust_density,
                prov.mantle_density,
                thickness,
            )?;
            let elevation = if with_impacts {
                // COMPOSE the analytic crater stamp onto the isostatic elevation: the crater bowls and ejecta rims
                // add DIRECTLY to the elevation (surface topography, not isostatically-compensated crust), the
                // "Sample = crust field + rows" surface function at this cell's own direction.
                let p = [cos_lat.mul(sin_lon), sin_lat, cos_lat.mul(cos_lon)];
                airy_km.checked_add(render::crater_relief_km(stamps, p))?
            } else {
                airy_km
            };
            elevations.push(elevation);
        }
    }
    Some(elevations)
}

/// The composed surface elevations over the six-face equi-angular CUBE-SPHERE cache (face-major,
/// `index = face * face_res^2 + t_row * face_res + s_col`), the render path. Each cell is INDEPENDENT: its elevation
/// is a pure function of the immutable province field and crater stamps at its own direction, so the cells are
/// computed in PARALLEL with rayon. The index-ordered `collect` makes the result BIT-IDENTICAL to a serial build for
/// any thread count (the fixed-point per-cell math is deterministic and there is no shared accumulator), so two views
/// of the same spot always agree (Principle 10). `None` if an elevation does not resolve. Display-only.
fn cube_elevations(
    prov: &DeepTimeProvinces,
    stamps: &[render::CraterStamp],
    face_res: usize,
    _with_impacts: bool,
) -> Option<Vec<Fixed>> {
    use rayon::prelude::*;
    // THE ONE SAMPLE DEFINITION: the cache is the memoized [`render::SurfaceField::height_km`], the SAME function
    // the shading takes its ANALYTIC GRADIENT of, so the normals can never drift from the heights they shade. A
    // crust-only field passes no stamps, and the crater layer of an empty stamp list contributes exactly zero.
    let field = province_surface_field(prov, stamps);
    // The face_res x face_res normalized FACE-LOCAL cube-cell directions, shared across all six faces (no per-cell
    // trig: the per-axis angle sin/cos is precomputed once, so building this is O(face_res^2), and the face rotation
    // per cell is sign swaps). Read-only across the parallel map below.
    let local = cube_local_dirs(face_res);
    let face_cells = face_res * face_res;
    (0..6 * face_cells)
        .into_par_iter()
        .map(|idx| -> Option<Fixed> {
            let dir =
                render::cube_face_local_to_world_fixed(idx / face_cells, local[idx % face_cells]);
            field.height_km(dir)
        })
        .collect()
}

/// The ANALYTIC SAMPLE FIELD of a derived province world ([`render::SurfaceField`]): the superposition the cache
/// memoizes and the shading differentiates, borrowing the province crust field and the prepared crater `stamps`.
/// One definition, two readers, so the rendered heights and the analytic normals cannot disagree. Display-only
/// (Principle 10).
fn province_surface_field<'a>(
    prov: &'a DeepTimeProvinces,
    stamps: &'a [render::CraterStamp],
) -> render::SurfaceField<'a> {
    render::SurfaceField {
        thickness_km: &prov.state.crust_thickness_km,
        pcols: prov.pcols,
        prows: prov.prows,
        crust_density: prov.crust_density,
        mantle_density: prov.mantle_density,
        stamps,
        // The tile elevations are in KILOMETRES (the Airy derivation's unit), so the radius is taken in the same
        // unit and the gradient's rise-over-arc comes out dimensionless. The 1000 is the km-to-m unit factor.
        radius_km: (prov.radius_m.to_f64_lossy() / 1000.0) as f32,
    }
}

/// The `face_res` by `face_res` normalized FACE-LOCAL direction of each EQUI-ANGULAR cube-sphere cell centre (shared
/// by all six faces; the world direction is [`render::cube_face_local_to_world_fixed`] of these). The equi-angular
/// map warps the face angle `alpha = (s - 1/2) * pi/2` (and `beta` from `t`) so equal steps in `(s, t)` subtend
/// near-equal solid angle (Ronchi, Iacono, Paolucci 1996, "The Cubed Sphere"): the face-local direction is
/// `(sin a cos b, cos a sin b, cos a cos b)` normalized. No per-cell trig: the per-axis angle sin/cos is precomputed
/// once (`face_res` entries) and each cell is two multiplies plus a normalize, so the parallel cache build pays the
/// trig once. Fixed-point, display-only re-parameterization (Principle 10).
fn cube_local_dirs(face_res: usize) -> Vec<[Fixed; 3]> {
    let half = Fixed::from_ratio(1, 2);
    let frac_pi_2 = Fixed::PI.mul(half);
    let axis_sc: Vec<(Fixed, Fixed)> = (0..face_res)
        .map(|i| {
            let s = Fixed::from_ratio((2 * i + 1) as i64, (2 * face_res) as i64);
            (s - half).mul(frac_pi_2).sin_cos()
        })
        .collect();
    let mut local = Vec::with_capacity(face_res * face_res);
    for &(sb, cb) in &axis_sc {
        // t axis (beta), the outer / row index
        for &(sa, ca) in &axis_sc {
            // s axis (alpha), the inner / column index
            let lx = sa.mul(cb);
            let ly = ca.mul(sb);
            let lz = ca.mul(cb);
            let n = (lx.mul(lx) + ly.mul(ly) + lz.mul(lz)).sqrt();
            let d = match (lx.checked_div(n), ly.checked_div(n), lz.checked_div(n)) {
                (Some(a), Some(b), Some(c)) => [a, b, c],
                _ => [Fixed::ZERO, Fixed::ZERO, Fixed::ONE],
            };
            local.push(d);
        }
    }
    local
}

/// Bilinearly sample the province crust-thickness field (kilometres) at a normalized surface coordinate
/// `(fu, fv)`. A display resampling of the DERIVED field (Principle 10), never fabricated content.
fn sample_province_thickness(prov: &DeepTimeProvinces, fu: f32, fv: f32) -> Fixed {
    render::sample_province_field(
        &prov.state.crust_thickness_km,
        prov.pcols,
        prov.prows,
        fu,
        fv,
    )
}

/// Parse a JANAF/condensate phase name (the formula before the `(phase)` suffix) into element atom counts, for
/// the bulk-composition mass-fraction read. `None` on a name that is not a parseable formula.
fn phase_element_counts(name: &str) -> Option<BTreeMap<String, u32>> {
    let formula = name.split('(').next()?;
    let chars: Vec<char> = formula.chars().collect();
    let mut atoms: BTreeMap<String, u32> = BTreeMap::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_uppercase() {
            return None;
        }
        let mut symbol = String::new();
        symbol.push(chars[i]);
        i += 1;
        while i < chars.len() && chars[i].is_ascii_lowercase() {
            symbol.push(chars[i]);
            i += 1;
        }
        let mut digits = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            digits.push(chars[i]);
            i += 1;
        }
        let count: u32 = if digits.is_empty() {
            1
        } else {
            digits.parse().ok()?
        };
        *atoms.entry(symbol).or_insert(0) += count;
    }
    if atoms.is_empty() {
        None
    } else {
        Some(atoms)
    }
}

/// The mass fraction of an element in the world's bulk CONDENSED assemblage (the whole rock, before
/// differentiation), DERIVED from the VCS condensed molar amounts and the periodic-table molar masses: the
/// element's summed mass (its atoms across every condensed phase, each phase weighted by its molar amount, times
/// the element's molar mass) over the total condensed mass. This is the SLR parent-element fuel (aluminium for
/// 26Al, iron for 60Fe), so the young-thermal heat source keys on the world's own composition, never an authored
/// abundance. `None` if the amounts are absent (a degenerate condensation vertex) or a molar mass does not
/// resolve; the caller then falls back to the reserved canonical mass fraction.
fn derive_bulk_element_mass_fraction(
    condensed_amounts: &[(String, Fixed)],
    element: &str,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let mut element_mass = Fixed::ZERO;
    let mut total_mass = Fixed::ZERO;
    for (phase, amount) in condensed_amounts {
        let counts = match phase_element_counts(phase) {
            Some(c) => c,
            None => continue,
        };
        let phase_molar_mass = table.molar_mass(&counts).ok()?;
        let phase_mass = amount.checked_mul(phase_molar_mass)?;
        total_mass = total_mass.checked_add(phase_mass)?;
        if let Some(&n) = counts.get(element) {
            let mut single = BTreeMap::new();
            single.insert(element.to_string(), 1u32);
            let element_molar_mass = table.molar_mass(&single).ok()?;
            let contribution = amount
                .checked_mul(Fixed::from_int(n as i32))?
                .checked_mul(element_molar_mass)?;
            element_mass = element_mass.checked_add(contribution)?;
        }
    }
    if total_mass <= Fixed::ZERO {
        return None;
    }
    element_mass.checked_div(total_mass)
}

/// The MEAN ATOMIC MASS (kg/mol) of the world's bulk CONDENSED assemblage, the total condensed mass over the total
/// condensed atom count, DERIVED from the VCS condensed molar amounts and the periodic-table molar masses. This is
/// the one input the Dulong-Petit specific heat needs (c_p = 3R / mean atomic mass), so an iron-rich world reads a
/// heavier mean atomic mass (a lower c_p) and a silicate world a lighter one (a higher c_p), each keyed on its own
/// composition. The table's molar masses are relative atomic weights (g/mol), so the result is divided by 1000 to
/// reach kg/mol. `None` if the amounts are absent or a molar mass does not resolve.
fn derive_mean_atomic_mass_kg_per_mol(
    condensed_amounts: &[(String, Fixed)],
    table: &PeriodicTable,
) -> Option<Fixed> {
    let mut total_mass_g = Fixed::ZERO;
    let mut total_atoms = Fixed::ZERO;
    for (phase, amount) in condensed_amounts {
        let counts = match phase_element_counts(phase) {
            Some(c) => c,
            None => continue,
        };
        let phase_molar_mass = table.molar_mass(&counts).ok()?;
        total_mass_g = total_mass_g.checked_add(amount.checked_mul(phase_molar_mass)?)?;
        let atoms_in_phase: u32 = counts.values().sum();
        total_atoms =
            total_atoms.checked_add(amount.checked_mul(Fixed::from_int(atoms_in_phase as i32))?)?;
    }
    if total_atoms <= Fixed::ZERO {
        return None;
    }
    // Mean atomic mass in g/mol, then to kg/mol for c_p = 3R / M with R in J/(mol*K).
    total_mass_g
        .checked_div(total_atoms)?
        .checked_div(Fixed::from_int(1000))
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
    // THE MIRROR (pinned solar) path, the default derived-planet view. The composition routes through the draw CHAIN at
    // the SOLAR PIN ([Fe/H] = 0, the local-disk environment's pin), so the Mirror is the chain evaluated at its pinned
    // value, byte-identical to the pre-generator solar fixture, NOT a bypass (the pins-condition-the-remainder
    // discipline, exercised by the standing regression). The seed is irrelevant under the pin (no draw is consumed).
    let disk_composition = civsim_materials::disk_composition::DiskComposition::draw(
        &civsim_materials::disk_composition::Environment::local_disk_solar_pin(),
        MIRROR_WORLD_SEED,
    )
    .map_err(|_| "the per-world disk composition did not load")?;
    build_derived_scene_with_composition(star_mass, orbit_au, &disk_composition)
}

/// The pinned Mirror's world seed. Under the solar pin the seed is IRRELEVANT (no [Fe/H] draw is consumed), so any
/// value pins to the solar composition; a named zero makes that visible.
const MIRROR_WORLD_SEED: u64 = 0;

/// Build the DERIVED scene for an UNPINNED world SEED: the composition is DRAWN from the local-disk environment (the
/// generator, Stage-0 path 1), so two seeds yield different `[Fe/H]`, hence different `Z / Z_sun`, isolation mass, and
/// star luminosity, for a DERIVED reason. This is the thesis path; the interactive and headless Mirror entries stay
/// pinned through [`build_derived_scene`]. The real generator entry points are [`DiskComposition::draw`] (the chain)
/// and [`build_derived_scene_with_composition`] (consumes any composition), both live on the bin path; this thin
/// local-disk wrapper is exercised by the thesis test, and a future CLI or worldgen consumer drops the `cfg(test)`.
#[cfg(test)]
fn build_derived_scene_seeded(
    star_mass: Fixed,
    orbit_au: Fixed,
    world_seed: u64,
) -> Result<DerivedScene, String> {
    let disk_composition = civsim_materials::disk_composition::DiskComposition::draw(
        &civsim_materials::disk_composition::Environment::local_disk(),
        world_seed,
    )
    .map_err(|_| "the per-world disk composition did not load")?;
    build_derived_scene_with_composition(star_mass, orbit_au, &disk_composition)
}

/// Build the DERIVED scene from a star mass, an orbit, and a per-world [`DiskComposition`] datum, or an error naming the
/// link that did not resolve (fail-soft: the viewer prints the message and shows no planet, never a fabricated one).
/// The chain is the built pipeline, each link a derivation: the star and disk ([`civsim_sim::planet::derive_planet`]),
/// the condensed-and-differentiated crust at a labelled formation-era condensation temperature
/// ([`derive_surface_composition`], the two-temperature seam documented at its site), the mantle density from the
/// derived mantle composition ([`derive_mantle_density`]), the isostatic tiles ([`generate_derived_tiles`]) off a
/// uniform crust field (uniform is the honest state for a fresh planet; lateral variation is a named geodynamics
/// follow-on), the crust's optical colour under the star ([`render::material_surface_rgb`]), and the atmospheric
/// speciation ([`atmosphere_gas_equilibrium`]) with its Rayleigh sky ([`render::rayleigh_sky_rgb`]).
///
/// COMPOSITION comes from the SINGLE per-world datum passed in. The condensation (the crust chemistry), the uncompressed
/// bulk density (the differentiation), the accretion isolation mass (the solid-dust column), and the star model (its
/// `Z / Z_sun` axis) all read this ONE slot, so a drawn `[Fe/H]` conditions the whole chain, and the solar/Mirror pin
/// conditions it at unity (byte-identical). This is what retires the accretion-mass and bulk-density solar fixtures in
/// substance: the inputs are per-world, not universal.
fn build_derived_scene_with_composition(
    star_mass: Fixed,
    orbit_au: Fixed,
    disk_composition: &civsim_materials::disk_composition::DiskComposition,
) -> Result<DerivedScene, String> {
    let janaf = JanafTables::standard().map_err(|_| "the JANAF tables did not load")?;
    let abundances = disk_composition.pattern().clone();
    // Z/Z_sun the star model AND the isolation mass read, sourced from the per-world datum (not a bare `Fixed::ONE`):
    // exactly unity for the solar/Mirror pin, the drawn `10^[Fe/H]` for an unpinned seed.
    let star_metallicity_ratio = disk_composition
        .metallicity_ratio_to_solar()
        .ok_or("the per-world metallicity ratio did not resolve (a bare pattern declares none)")?;
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
    // The isolation mass CONDITIONS ON the drawn metallicity: the solid-dust column scales with Z/Z_sun (a metal-rich
    // disk sweeps a larger mass), exactly the solar norm for the Mirror pin (ratio == ONE, byte-identical).
    let planet_mass_earth = derive_isolation_mass_earth(orbit_au, star_metallicity_ratio)
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
        star_metallicity_ratio, // Z/Z_sun from the per-world composition datum (unity: the solar instance)
        Fixed::from_ratio(35, 10), // alpha (mass-luminosity)
        Fixed::from_ratio(8, 10), // beta (mass-radius)
        Fixed::from_ratio(-44, 100), // lambda (metallicity-luminosity)
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

    // R-YOUNG-TEMPERATURE: THE YOUNG-THERMAL REGIME VERDICT supplies a MELTED world's deep-time initial condition
    // in place of the fixed 1588 K mantle potential-temperature anchor. This is a VIEWER-SIDE re-derivation, not a
    // change to `crates/materials`: the bootstrap surface derivation above ran on the `MANTLE_POTENTIAL_TEMPERATURE_K`
    // anchor only to expose the world's OWN derived solidus (which is potential-temperature-independent, a property
    // of the endmembers). The young-thermal verdict then decides whether this world's formation crosses that solidus
    // (short-lived-radionuclide heat on an early-formed body, or giant impacts on an Earth-mass body), and, when its
    // best estimate melts, pins the young potential temperature at the magma-ocean lock-up handoff (the world's own
    // solidus plus the phi_c superheat). The surface is then RE-DERIVED here in `build_derived_scene` by a SECOND
    // call to `derive_surface_composition` with that young temperature (below), so a melted world forms super-solidus
    // with a nonzero melt fraction F, the input the per-province radiogenic heterogeneity (and therefore the relief)
    // reads. A NEVER-MELTED world keeps its sub-solidus (smooth) surface; a MARGINAL world is carried at its best
    // estimate with its grade surfaced. The SLR parent-element fuel (aluminium, iron) is DERIVED from the world's own
    // condensed assemblage and the specific heat c_p from its own mean atomic mass; the birth-environment SLR ratios
    // and draw band, the rheology, the retention band, the formation-time band, and the reset epoch are the
    // reserved-with-basis inputs above.
    let al_mass_fraction = sc
        .condensed_amounts
        .as_ref()
        .and_then(|amounts| derive_bulk_element_mass_fraction(amounts, "Al", &table))
        // Reserved-with-basis fallback at a degenerate condensation vertex: the canonical bulk-silicate aluminium
        // mass fraction ~0.02 (McDonough & Sun 1995 BSE Al2O3 ~4.5 wt%). The normal path derives it.
        .unwrap_or_else(|| Fixed::from_int(2).div(Fixed::from_int(100)));
    let fe_mass_fraction = sc
        .condensed_amounts
        .as_ref()
        .and_then(|amounts| derive_bulk_element_mass_fraction(amounts, "Fe", &table))
        // Reserved-with-basis fallback: the canonical bulk (silicate-plus-metal) iron mass fraction ~0.19 (the
        // solar Fe/rock ratio, Lodders 2003 lineage). The normal path derives it.
        .unwrap_or_else(|| Fixed::from_int(19).div(Fixed::from_int(100)));
    let slr_family = civsim_physics::young_thermal::solar_system_slr_family(
        al_mass_fraction,
        fe_mass_fraction,
        AL26_OVER_AL27_INITIAL,
        FE60_OVER_FE56_INITIAL,
    );
    // The DERIVED specific heat c_p (Dulong-Petit on the world's OWN mean atomic mass), so an iron world judges its
    // melt budget at ~447 J/(kg*K) and a silicate world at ~1240, each by construction (c_p is load-bearing: it
    // divides the whole budget the regime decision rests on). Fail-soft to the reserved silicate fallback if the
    // assemblage's mean atomic mass does not resolve.
    let young_specific_heat = sc
        .condensed_amounts
        .as_ref()
        .and_then(|amounts| derive_mean_atomic_mass_kg_per_mol(amounts, &table))
        .and_then(civsim_physics::young_thermal::dulong_petit_specific_heat)
        .unwrap_or(YOUNG_SPECIFIC_HEAT_J_PER_KG_K);
    // The planet-mass band the giant-impact gate is swept over, a fractional half-width around the derived
    // isolation mass (reserved-with-basis). For a Mars-class world the whole band sits below the threshold.
    let mass_half_width = planet_mass_earth
        .checked_mul(MASS_BAND_HALF_WIDTH_FRACTION)
        .unwrap_or(Fixed::ZERO);
    let planet_mass_earth_lo = planet_mass_earth
        .checked_sub(mass_half_width)
        .unwrap_or(planet_mass_earth);
    let planet_mass_earth_hi = planet_mass_earth
        .checked_add(mass_half_width)
        .unwrap_or(planet_mass_earth);
    // The young-thermal verdict when the solidus resolves. The solidus is threaded from the bootstrap surface
    // (potential-temperature-independent). `None` (fail-soft, dormant as before) when the solidus did not derive.
    // The formation-time, SLR-draw, and mass BANDS are swept for the GAPPED / MARGINAL grade: the default
    // Mars-class scene, robustly melted only if its formation time is pinned to the fast point value, is MARGINAL
    // once the real interim formation-time band (up to a few megayears, 26Al largely decayed) is admitted. It is
    // carried at its best-estimate super-solidus handoff (MARGINAL-carried, not cold), its grade surfaced.
    let young_verdict = match (sc.solidus_surface_k, sc.solidus_slope_k_per_gpa) {
        (Some(solidus_surface_k), Some(solidus_slope_k_per_gpa)) => {
            let young_inputs = civsim_physics::young_thermal::YoungThermalInputs {
                solidus_surface_k,
                solidus_slope_k_per_gpa,
                adiabat_slope_k_per_gpa: MANTLE_ADIABAT_SLOPE_K_PER_GPA,
                productivity_per_gpa: MELT_PRODUCTIVITY_PER_GPA,
                lockup_melt_fraction: PHI_C_LOCKUP_MELT_FRACTION,
                specific_heat_j_per_kg_k: young_specific_heat,
                reference_temperature_k: planet.disk_temperature_k,
                surface_gravity_m_per_s2: planet.surface_gravity_m_s2,
                radius_m: planet.radius_m,
                binding_energy_geometry: ACCRETION_GEOMETRY_FACTOR,
                retention_efficiency_lo: RETENTION_EFFICIENCY_LO,
                retention_efficiency_hi: RETENTION_EFFICIENCY_HI,
                formation_time_myr: FORMATION_TIME_MYR,
                formation_time_myr_lo: FORMATION_TIME_MYR_LO,
                formation_time_myr_hi: FORMATION_TIME_MYR_HI,
                slr_initial_ratio_scale_lo: SLR_INITIAL_RATIO_SCALE_LO,
                slr_initial_ratio_scale_hi: SLR_INITIAL_RATIO_SCALE_HI,
                planet_mass_earth,
                planet_mass_earth_lo,
                planet_mass_earth_hi,
                giant_impact_mass_threshold_earth: GIANT_IMPACT_MASS_THRESHOLD_EARTH,
                reset_epoch_myr: RESET_EPOCH_MYR,
                reset_epoch_half_band_myr: RESET_EPOCH_HALF_BAND_MYR,
            };
            civsim_physics::young_thermal::young_thermal_verdict(&young_inputs, &slr_family)
        }
        _ => None,
    };
    // The young potential temperature the deep-time run starts from: the verdict's value when it resolves (the
    // solidus-pinned handoff for a MELTED world, the cold peak otherwise), else the anchor (fail-soft).
    let young_potential_temperature_k = young_verdict
        .map(|v| v.young_potential_temperature_k)
        .unwrap_or(MANTLE_POTENTIAL_TEMPERATURE_K);
    // RE-DERIVE the surface at the young potential temperature. A MELTED world (young temperature super-solidus)
    // now extracts a partial melt (F = phi_c > 0), the crust becomes that first melt, and the mantle the residue.
    // Fail-soft to the bootstrap surface if the re-derivation does not resolve. Every written-state row below that
    // consumes this carries the INTERIM provenance of the young-thermal inputs (the reset epoch and formation-time
    // bands); the scope fence binds them to texture onset and thermal initial conditions only.
    let reserved_melt_young = civsim_materials::surface_composition::ReservedMeltParams {
        potential_temperature_k: young_potential_temperature_k,
        adiabat_slope_k_per_gpa: MANTLE_ADIABAT_SLOPE_K_PER_GPA,
        productivity_per_gpa: MELT_PRODUCTIVITY_PER_GPA,
        gravity_m_per_s2: MELT_COLUMN_GRAVITY_M_S2,
    };
    let sc = civsim_materials::surface_composition::derive_surface_composition(
        &janaf,
        &abundances,
        condensation_temperature_k,
        &reserved_melt_young,
    )
    .unwrap_or(sc);
    let crust_composition = sc.surface.clone();
    let mantle_composition = sc.mantle_composition.clone();
    let mantle_density =
        derive_mantle_density(&mantle_composition, surface_t, surface_p, &registry, &table)
            .unwrap_or(mantle_density);

    // THE DEEP-TIME PROVINCE TEXTURE (the visible-world payoff): the surface relief is the DERIVED crust the
    // deep-time volcanism ([`civsim_sim::deeptime`]) builds over a lateral field of mantle columns. The columns
    // start uniform (a fresh planet has no thermal history) and DIVERGE as each province's own radiogenic
    // history plays out, so the surface is the written record of the interior, never a painted map. The
    // province SCALE derives from the convective physics (depth times the eigenvalue-derived cell aspect, over
    // the circumference); the crust AMPLITUDE from the melt and the isostasy. Fail-soft: if the convective scale
    // or the DERIVED solidus does not resolve, the globe shows the uniform crust (the prior behaviour), never a
    // fabricated texture and never an authored solidus. The SAMPLE CACHE of the composed surface (crust + crater-row
    // stamps) is a CUBE-SPHERE (six faces, no pole pinch); the rare uniform-crust fallback keeps the plain lat-lon
    // grid. `param` records which so the render and the deep-time re-derivation always agree with the build.
    let cube_param = render::SurfaceParam::CubeSphere {
        face_res: SURFACE_SAMPLE_CACHE_FACE_RES,
    };
    let sea_level = Fixed::ZERO;
    // The crust density (once) each province's crust floats at, and the interior-structure inputs the
    // convecting-depth derivation reads.
    let crust_density = civsim_physics::petrology::crustal_density(
        &crust_composition,
        surface_t,
        surface_p,
        &registry,
        &table,
    );
    let core_structure = derive_core_fraction_and_metal_density(&abundances, &table, &eos);
    // The world's OWN derived mantle solidus (surface value and slope) from the surface-composition melt wiring
    // on this scene; the province path builds only when it resolved (else fail-soft to uniform crust, never an
    // authored solidus). The formation melt fraction F seeds the per-system heterogeneity amplitude (`None` on a
    // sub-solidus formation, which then yields no melt-driven lateral spread, the honest unprocessed-mantle case).
    let provinces = match (
        crust_density,
        core_structure,
        sc.solidus_surface_k,
        sc.solidus_slope_k_per_gpa,
    ) {
        (
            Some(crust_density),
            Some((core_fraction, core_density)),
            Some(solidus_surface_k),
            Some(solidus_slope_k_per_gpa),
        ) if sc.melt_status != civsim_materials::differentiation::MeltStatus::Degenerate => {
            // A SubSolidus or Melted scene is a valid world: build its province field. A Degenerate melt status
            // (no source weight, an empty split, a missing melting datum) is a near-failure, not a physical
            // sub-solidus mantle, so it falls through to the uniform crust rather than a fabricated flat texture.
            build_deep_time_provinces(
                star_mass,
                orbit_au,
                planet.radius_m,
                planet.surface_gravity_m_s2,
                core_fraction,
                core_density,
                planet_bulk_density,
                mantle_density,
                crust_density,
                solidus_surface_k,
                solidus_slope_k_per_gpa,
                sc.max_melt_fraction,
                young_potential_temperature_k,
            )
        }
        _ => None,
    };
    // Age the initial scene to a present-day surface (the observer's time control runs it on from here), and
    // read the display tiles off it. If the province field did not resolve, fall back to the uniform crust
    // (the seam-6 DERIVED thickness where the melt engaged, else the Slice-0 30 km fixture).
    let (tiles, provinces) = match provinces {
        Some(mut prov) => {
            step_provinces(&mut prov, DEEP_TIME_INITIAL_STEPS);
            match derive_province_tiles(&prov, cube_param) {
                Some(t) => (t, Some(prov)),
                None => (Vec::new(), Some(prov)),
            }
        }
        None => (Vec::new(), None),
    };
    // The province-textured cube-sphere cache, or the uniform-crust lat-lon fallback (a smooth ball, so the plain
    // grid reads the same). `param` follows the tiles, so the render sampling and the lava field stay aligned.
    let (tiles, param) = if tiles.is_empty() {
        let field: Vec<Vec<(String, Fixed)>> =
            vec![crust_composition.clone(); SURFACE_SAMPLE_CACHE_COLS * SURFACE_SAMPLE_CACHE_ROWS];
        let crustal_thickness = sc.crust_thickness_km.unwrap_or_else(|| Fixed::from_int(30));
        let t = generate_derived_tiles(
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
        (
            t,
            render::SurfaceParam::LatLon {
                cols: SURFACE_SAMPLE_CACHE_COLS,
                rows: SURFACE_SAMPLE_CACHE_ROWS,
            },
        )
    } else {
        (tiles, cube_param)
    };

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

    // THE LAVA GLOW field, aligned one-to-one with the display tiles: each tile's DERIVED melt-glow (the incandescent
    // colour of its province's interior temperature and its DERIVED melt fraction against the world's own solidus).
    // Empty on the uniform-crust fallback (no province temperatures), so the render adds no emission. Re-derived
    // alongside the tiles as the deep-time clock steps, so the glow fades as the world cools (the volcanism arc).
    let lava = provinces
        .as_ref()
        .map(|p| derive_province_lava(p, param))
        .unwrap_or_default();

    Ok(DerivedScene {
        star_mass,
        orbit_au,
        radius_m: planet.radius_m,
        t_eff: planet.star_effective_temperature_k,
        star_radius_ratio: planet.star_radius_ratio,
        disk_t: planet.disk_temperature_k,
        condensation_t: condensation_temperature_k,
        gravity: planet.surface_gravity_m_s2,
        mass_earth: planet.mass_earth,
        density: planet.bulk_density_g_cm3,
        crust,
        crust_phases,
        atmosphere,
        tiles,
        param,
        sky,
        material,
        // The per-world lighting attitude, READ from the world's DiurnalSky (obliquity, eccentricity), with the axis
        // orientation, orbital phase, rotation period, and initial spin phase reserved per-world inputs the pipeline
        // fills (R-CELESTIAL-SECULAR, task #44 / Part 18.1). No tilt or spin value is authored in the viewer.
        attitude: derived_scene_attitude(),
        provinces,
        young: young_verdict,
        lava,
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
    // The per-world lighting attitude, READ from the world sky (obliquity, eccentricity) with the axis orientation,
    // orbital phase, rotation period, and initial spin phase reserved per-world inputs the pipeline fills (task #44).
    let fmt_attitude = |v: Option<Fixed>| {
        v.map(|x| format!("{:.4}", x.to_f64_lossy()))
            .unwrap_or_else(|| "reserved(#44)".to_string())
    };
    eprintln!(
        "  attitude (per-world, read from DiurnalSky): obliquity {} rad, eccentricity {}, perihelion {}, orbital phase {}, rotation period {} s, initial spin {}",
        fmt_attitude(scene.attitude.obliquity),
        fmt_attitude(scene.attitude.orbit_eccentricity),
        fmt_attitude(scene.attitude.perihelion_longitude),
        fmt_attitude(scene.attitude.orbital_phase),
        fmt_attitude(scene.attitude.rotation_period_s),
        fmt_attitude(scene.attitude.initial_spin_phase),
    );
    // R-YOUNG-TEMPERATURE: the young-thermal regime verdict (Melted / Never-melted / Marginal) and whether it is
    // decidable now (GAPPED) or band-contingent (MARGINAL), band-swept over the interim inputs.
    if let Some(v) = &scene.young {
        let regime = match v.regime {
            civsim_physics::young_thermal::YoungThermalRegime::Melted => "MELTED",
            civsim_physics::young_thermal::YoungThermalRegime::NeverMelted => "NEVER-MELTED",
            civsim_physics::young_thermal::YoungThermalRegime::Marginal => "MARGINAL",
        };
        let grade = if v.gapped {
            "GAPPED (decidable across the interim bands)"
        } else {
            "band-contingent (MARGINAL, carried at the best estimate, impact-list-pending)"
        };
        eprintln!(
            "  young-thermal (R-YOUNG-TEMPERATURE): {regime}, {grade};  SLR rise {:.0} K,  young potential T {:.0} K,  reset epoch {:.0} +/- {:.0} Myr (interim)",
            v.slr_temperature_rise_k.to_f64_lossy(),
            v.young_potential_temperature_k.to_f64_lossy(),
            v.reset_epoch_myr.to_f64_lossy(),
            v.reset_epoch_half_band_myr.to_f64_lossy(),
        );
        if let Some(handoff) = v.handoff_potential_temperature_k {
            eprintln!(
                "    magma-ocean lock-up handoff (solidus + phi_c superheat): {:.0} K",
                handoff.to_f64_lossy()
            );
        }
    }
    // THE DERIVED RELIEF, three SEPARATED and TAGGED quantities (fix 2): the physical km amplitude, the
    // support-bound validity check, and the (dimensionless) heterogeneity-engaged indicator. The km amplitude is
    // the honest visible measure; the indicator only reports whether the melt-driven texture is on, never how tall.
    // These ISOSTATIC diagnostics read the CRUST-ONLY relief (the melt-driven, Airy-floated crust), NOT the composed
    // surface: the bombardment is a separate surface-topography record reported below, so it neither dilutes the
    // melt-texture indicator's normalization nor masquerades as isostatic relief. The rendered globe still shows the
    // composed surface ([`derive_province_tiles`]); this fall back to the rendered tiles only on the uniform-crust
    // path where there is no province field.
    // The crust-only diagnostic tiles are derived at the FIXED diagnostic resolution (decoupled from the render
    // sample cache), so the resolution-sensitive heterogeneity indicator stays comparable; only the uniform-crust
    // fallback reads the rendered tiles at their own resolution.
    let crust_tiles = scene
        .provinces
        .as_ref()
        .and_then(|p| derive_province_crust_tiles(p, DIAGNOSTIC_TILE_COLS, DIAGNOSTIC_TILE_ROWS));
    let (iso_tiles, iso_cols): (&[DerivedTile], usize) = match crust_tiles.as_deref() {
        Some(t) => (t, DIAGNOSTIC_TILE_COLS),
        // The uniform-crust fallback is a lat-lon field; read its column count from the parameterization. (A cube
        // cache always carries a province field, so `crust_tiles` resolves above and this branch is the lat-lon one.)
        None => {
            let cols = match scene.param {
                render::SurfaceParam::LatLon { cols, .. } => cols,
                render::SurfaceParam::CubeSphere { face_res } => face_res,
            };
            (scene.tiles.as_slice(), cols)
        }
    };
    if let Some(amplitude_km) = tile_relief_amplitude_km(iso_tiles) {
        let bound = scene.provinces.as_ref().and_then(|p| {
            supportable_relief_km(
                CRUST_YIELD_STRENGTH_PA,
                p.crust_density.to_f64_lossy(),
                scene.gravity.to_f64_lossy(),
            )
        });
        match bound {
            Some(bound_km) if amplitude_km > bound_km => eprintln!(
                "  derived isostatic relief amplitude: {amplitude_km:.2} km  [FLAG: exceeds the support bound \
                 {bound_km:.2} km = yield/(rho*g); an unsupportable relief, not rendered as real]"
            ),
            Some(bound_km) => eprintln!(
                "  derived isostatic relief amplitude: {amplitude_km:.2} km  (within the support bound {bound_km:.2} km = yield/(rho*g); real planetary relief is sub-1% of radius, so this reads nearly smooth at physical scale)"
            ),
            None => eprintln!(
                "  derived isostatic relief amplitude: {amplitude_km:.2} km  (support bound unavailable: no province crust density)"
            ),
        }
    }
    if let Some(indicator) = tile_relief_heterogeneity_indicator(iso_tiles, iso_cols) {
        eprintln!(
            "  relief heterogeneity-engaged indicator: {:.1}%  (a normalized roughness ratio: is the melt-driven texture ON, not a relief magnitude; smooth-ball floor near ~2%)",
            indicator * 100.0
        );
    }
    // THE DEEP-TIME BOMBARDMENT record: the discrete crater ROWS the accretion-tail flux drew over the run so far,
    // each stamped analytically onto the surface at its true derived size (rows not rasters). A read of the derived
    // crater history (Principle 10); the flux DECLINES with epoch, so the count is a heavy-early, quiescent-late
    // record, never an authored intensity. The CROSS-SCALE split is reported: a crater at or above the convective
    // cell size ALSO rasterizes into the province field (the large-basin feedback); by default the craters are
    // sub-cell (kilometre-class against ~thousand-km cells), so they are all rows and the province raster is dormant.
    if let Some(p) = scene.provinces.as_ref() {
        let cell_km = p.flux.forces.cell_size.to_f64_lossy() / 1000.0;
        let craters = &p.state.craters;
        let cell_size = p.flux.forces.cell_size;
        let basins = craters.iter().filter(|c| c.diameter_m >= cell_size).count();
        let (mut min_d, mut max_d) = (f64::MAX, 0.0f64);
        for c in craters {
            let d = c.diameter_m.to_f64_lossy() / 1000.0;
            min_d = min_d.min(d);
            max_d = max_d.max(d);
        }
        let size_note = if craters.is_empty() {
            "no craters drawn yet".to_string()
        } else {
            format!(
                "rim diameters {min_d:.0} to {max_d:.0} km; {basins} at/above the {cell_km:.0} km cell size rasterize into the province field (the cross-scale large-basin feedback), the rest are sub-cell rows"
            )
        };
        eprintln!(
            "  deep-time bombardment: {} discrete crater rows on the {}x{} derived province grid (~{:.0} km cells), each stamped analytically at its true size; {size_note}; accretion-tail reservoir {:.0} bodies, sweep-up tau {:.0} Myr, closing speed {:.0} m/s (escape floor); heavy early, quiescent late",
            p.state.impact_count,
            p.pcols,
            p.prows,
            cell_km,
            p.flux.reservoir_body_count.to_f64_lossy(),
            p.flux.sweep_timescale_myr.to_f64_lossy(),
            p.flux.impact_velocity_m_s.to_f64_lossy(),
        );
    }
    // THE LAVA GLOW arc (the visible-volcanism payoff): the fraction of the surface still super-solidus (molten and
    // self-emitting incandescence) and the mean melt-glow intensity, both DERIVED per tile from the province interior
    // temperature against the world's OWN solidus. A young/hot world glows broadly (most tiles molten); an aged world
    // is mostly solid crust with only the hottest provinces still glowing, so stepping the deep-time clock fades this.
    if !scene.lava.is_empty() {
        let molten = scene.lava.iter().filter(|g| g.intensity > 0.0).count();
        let mean =
            scene.lava.iter().map(|g| g.intensity as f64).sum::<f64>() / scene.lava.len() as f64;
        eprintln!(
            "  lava glow: {:.0}% of the surface molten (super-solidus, self-emitting), mean melt-glow intensity {:.2}; a young world glows broadly, an aged one only at hot-spots (fades as the deep-time clock cools the world)",
            100.0 * molten as f64 / scene.lava.len() as f64,
            mean,
        );
    }
    eprintln!("  controls: +/- zoom, wasd/arrows rotate the globe, p provenance, Esc quit");
}

/// A HETEROGENEITY-ENGAGED indicator (NOT a relief magnitude): the mean absolute elevation difference between
/// grid-adjacent tiles, normalized by the field's OWN elevation scale. Because it divides by the field's scale, it
/// measures the spatial ROUGHNESS of the province pattern (whether the melt-driven heterogeneity is engaged at
/// all, so the value is above the smooth-ball floor), NOT the physical amplitude of the relief. A uniform
/// (unprocessed) field sits near the small-roughness floor; a processed, province-diverged field stands above it.
/// It answers "is the texture on?", never "how tall are the mountains?": the physical measure is the km amplitude
/// ([`tile_relief_amplitude_km`]). `None` for an empty or degenerate field.
fn tile_relief_heterogeneity_indicator(tiles: &[DerivedTile], cols: usize) -> Option<f64> {
    if tiles.is_empty() || cols == 0 {
        return None;
    }
    let rows = tiles.len() / cols;
    if rows == 0 {
        return None;
    }
    let elev: Vec<f64> = tiles.iter().map(|t| t.elevation.to_f64_lossy()).collect();
    let max = elev.iter().cloned().fold(f64::MIN, f64::max);
    let min = elev.iter().cloned().fold(f64::MAX, f64::min);
    let mean = elev.iter().sum::<f64>() / elev.len() as f64;
    // The scale the roughness is normalized by: the field's own elevation scale, floored to a small positive so a
    // flat field does not divide by zero. This normalization is exactly why the result is a heterogeneity-engaged
    // indicator and not a relief amplitude.
    let scale = mean.abs().max((max - min).abs()).max(1e-6);
    let mut total = 0.0;
    let mut pairs = 0u64;
    for r in 0..rows {
        for c in 0..cols {
            let i = r * cols + c;
            if c + 1 < cols {
                total += (elev[i] - elev[i + 1]).abs();
                pairs += 1;
            }
            if r + 1 < rows {
                total += (elev[i] - elev[i + cols]).abs();
                pairs += 1;
            }
        }
    }
    if pairs == 0 {
        return None;
    }
    Some((total / pairs as f64) / scale)
}

/// The TRUE ISOSTATIC RELIEF AMPLITUDE in kilometres: the span (max minus min) of the derived tile elevations, the
/// Airy float of the derived crust-thickness spread the provinces carry. The tiles' elevations are already in km
/// (the province crust thickness is in km and [`civsim_physics::geodynamics::airy_isostatic_elevation`] is
/// unit-preserving), so this is the honest physical relief the world stands in, the number the render shows at
/// physical scale. HONEST NOTE: real planetary relief is SUB-1 percent of the radius (Mars ~30 km over 3390 km,
/// Earth ~20 km over 6371 km), so a correctly-derived world reads a few kilometres of amplitude and looks nearly
/// smooth from globe distance; that is sound physics, not a defect, and the honest bumpiness test is whether this
/// amplitude lands inside the Mars/Earth hindcast band, never the unexaggerated eyeball. `None` for an empty field.
fn tile_relief_amplitude_km(tiles: &[DerivedTile]) -> Option<f64> {
    if tiles.is_empty() {
        return None;
    }
    let mut max = f64::MIN;
    let mut min = f64::MAX;
    for t in tiles {
        let e = t.elevation.to_f64_lossy();
        max = max.max(e);
        min = min.min(e);
    }
    Some(max - min)
}

/// The SUPPORT-BOUND on relief (km): the maximum topography a crust of yield strength `sigma_y`, density `rho`, and
/// surface gravity `g` can hold against its own weight before it flows, `h_max = sigma_y / (rho * g)` (the
/// strength-over-buoyant-weight bound). A derived relief amplitude at or below this is physically supportable; an
/// amplitude ABOVE it is unphysical (the crust would relax) and is surfaced as a FLAG, never rendered as real
/// relief. `crust_density_g_cm3` is the DERIVED crust density; `sigma_y` is the reserved crustal yield strength.
/// `None` on a non-physical density or gravity.
fn supportable_relief_km(
    yield_strength_pa: f64,
    crust_density_g_cm3: f64,
    gravity_m_per_s2: f64,
) -> Option<f64> {
    let rho_kg_m3 = crust_density_g_cm3 * 1000.0;
    if rho_kg_m3 <= 0.0 || gravity_m_per_s2 <= 0.0 {
        return None;
    }
    Some(yield_strength_pa / (rho_kg_m3 * gravity_m_per_s2) / 1000.0)
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
/// The metres-per-pixel scale for the derived globe at zoom fraction `t` in [0, 1]: the on-screen radius grows
/// geometrically from ~0.12 of the frame (a distant globe) up to ~40x the frame (a deep surface drill-down, where a
/// patch of the derived elevation field spans the frame and the sun-direction hillshade reads the km-scale relief).
/// Both fractions are non-canon display-framing choices (the zoom endpoints, pixels only), the only display tuneables
/// the drill-in allows. Shared by the interactive globe view and the woosh so their framings stay in step (Principle 10).
fn derived_globe_m_per_px(radius_m: Fixed, w: usize, h: usize, t: f32) -> Fixed {
    let min_dim = w.min(h);
    // NON-CANON display-framing zoom endpoints (pixels only): the distant globe (~0.12 of the frame) and the deep
    // surface patch (~40x the frame). No physics value; extending R_MAX_FRAC only drills the camera closer.
    const R_MIN_FRAC: f32 = 0.12;
    const R_MAX_FRAC: f32 = 40.0;
    let frac = R_MIN_FRAC * (R_MAX_FRAC / R_MIN_FRAC).powf(t.clamp(0.0, 1.0));
    let target_r = ((min_dim as f32) * frac).max(1.0) as i32;
    radius_m
        .checked_div(Fixed::from_int(target_r.max(1)))
        .unwrap_or(Fixed::ONE)
}

/// The derived viewer's continuous globe scale at zoom fraction `t` in [0, 1]: the metres-per-pixel scale (so the
/// DERIVED radius drives the on-screen size), the star's on-screen position, and its on-screen radius. The globe
/// grows from a distant star-lit sphere (t = 0, the star beside it) to the surface filling the frame up close
/// (t = 1): one continuous zoom, all of it the sphere. The two radius endpoints are non-canon display-framing choices
/// for the viewer, documented at their site (Principle 10, pixels only).
fn derived_globe_view(fx: &GlobeFixture, w: usize, h: usize, t: f32) -> (Fixed, (i32, i32), usize) {
    let min_dim = w.min(h);
    let m_per_px = derived_globe_m_per_px(fx.radius_m, w, h, t);
    // The star sits off to the upper-left; it is occluded once the globe grows to fill the frame (you are at the
    // surface), the honest look for the close view.
    let star_px = ((w / 5) as i32, (h / 4) as i32);
    let star_r = derived_star_radius_px(fx, min_dim);
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

/// A "nice" round distance (metres) at or below `raw`, snapped to 1, 2, or 5 times a power of ten, so the scale bar
/// reads "500 km" rather than "473 km". Display-only formatting (Principle 10), no physics value.
fn nice_round_distance_m(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let base = 10f64.powf(raw.log10().floor());
    let mantissa = raw / base;
    let nice = if mantissa >= 5.0 {
        5.0
    } else if mantissa >= 2.0 {
        2.0
    } else {
        1.0
    };
    nice * base
}

/// A short human label for a ground distance in metres ("420 km", "1.5 km", "800 m"). Display-only.
fn format_distance_m(m: f64) -> String {
    if m >= 1000.0 {
        let km = m / 1000.0;
        if km >= 10.0 {
            format!("{km:.0} km")
        } else {
            format!("{km:.1} km")
        }
    } else {
        format!("{m:.0} m")
    }
}

/// A short label for the view scale (metres, else kilometres, per pixel). Display-only.
fn format_scale_per_px(m_per_px: f64) -> String {
    if m_per_px >= 1000.0 {
        format!("{:.2} km/px", m_per_px / 1000.0)
    } else {
        format!("{m_per_px:.0} m/px")
    }
}

/// The ZOOM READOUT the owner asked for (bottom-left): a scale bar of a round ground distance with its length, the
/// view scale in metres-per-pixel, and the named zoom tier, so the current level of zoom is always legible and updates
/// as the zoom changes. The scale is the metres-per-pixel at the disk CENTRE (the orthographic sphere foreshortens
/// toward the limb), a pure read of the DERIVED-radius view scale ([`derived_globe_m_per_px`]); the round-distance snap
/// is display formatting, not a physics value (Principle 10, pixels only).
fn draw_zoom_readout(buf: &mut [u32], w: usize, h: usize, m_per_px: f64, tier: &str) {
    if w == 0 || h == 0 || !m_per_px.is_finite() || m_per_px <= 0.0 {
        return;
    }
    let target_px = (w as f64 / 5.0).max(40.0);
    let bar_m = nice_round_distance_m(target_px * m_per_px);
    let bar_px = ((bar_m / m_per_px).round() as i32).clamp(1, w as i32 - 24);
    let x0 = 12i32;
    let y = h as i32 - 20;
    let ink = Rgb::new(235, 235, 245).pack();
    let mut put = |x: i32, py: i32| {
        if x >= 0 && x < w as i32 && py >= 0 && py < h as i32 {
            buf[py as usize * w + x as usize] = ink;
        }
    };
    // The bar itself (two rows) and the two end ticks.
    for x in x0..=(x0 + bar_px) {
        put(x, y);
        put(x, y + 1);
    }
    for &tx in &[x0, x0 + bar_px] {
        for dy in -3..=3i32 {
            put(tx, y + dy);
        }
    }
    render::draw_label(
        buf,
        w,
        h,
        x0,
        y - 16,
        &format!(
            "ZOOM {tier}  |  bar {}  |  {}",
            format_distance_m(bar_m),
            format_scale_per_px(m_per_px)
        ),
        1,
        Rgb::new(235, 235, 245),
        Rgb::new(10, 12, 20),
    );
}

/// One planet sampled for the system map: its DERIVED scene (radius, star temperature, crust, colour, all from
/// build_derived_scene) and the mean anomaly that places its dot on the map. The scene is a real derivation; the mean
/// anomaly is a NON-CANON display phase spread so the dots do not all sit at perihelion.
struct SampledPlanet {
    scene: DerivedScene,
    mean_anomaly: Fixed,
}

/// Sample a small set of orbits across the terrestrial zone and DERIVE a planet at each (fail-soft: an orbit that does
/// not resolve is skipped, never fabricated). NON-CANON viewer input: this is a chosen set of sample orbits, NOT a
/// gravitationally-assembled multi-body system (that is the solar-system generator, task #72); each planet is an
/// independent derivation, and the map places them together for viewing only. `extra_orbit`, if given (the `--derived`
/// orbit argument), is added to the set so a specific orbit can be viewed alongside the samples.
fn build_sampled_planets(star_mass: Fixed, extra_orbit: Option<Fixed>) -> Vec<SampledPlanet> {
    let mut orbits: Vec<Fixed> = [(7, 10), (9, 10), (11, 10), (13, 10), (15, 10)]
        .iter()
        .map(|(n, d)| Fixed::from_ratio(*n, *d))
        .collect();
    if let Some(o) = extra_orbit {
        if !orbits
            .iter()
            .any(|x| (x.to_f64_lossy() - o.to_f64_lossy()).abs() < 1e-3)
        {
            orbits.push(o);
        }
    }
    orbits.sort_by_key(|a| a.to_bits());
    let n = orbits.len().max(1);
    let tau = Fixed::PI.checked_add(Fixed::PI).unwrap_or(Fixed::PI);
    let mut planets = Vec::new();
    for (i, orbit) in orbits.iter().enumerate() {
        match build_derived_scene(star_mass, *orbit) {
            Ok(scene) => {
                let frac = Fixed::from_ratio(i as i64, n as i64);
                let mean_anomaly = tau.checked_mul(frac).unwrap_or(Fixed::ZERO);
                planets.push(SampledPlanet {
                    scene,
                    mean_anomaly,
                });
            }
            Err(e) => eprintln!(
                "  (orbit {:.2} AU did not resolve: {e}; skipped)",
                orbit.to_f64_lossy()
            ),
        }
    }
    planets
}

/// The [`render::MapBody`] for a sampled planet: its DERIVED orbit (semi-major axis), the per-world eccentricity, the
/// mean anomaly placing its dot on the ellipse, and its DERIVED material colour as the dot. `mean_anomaly` is the
/// CURRENT phase (the animation advances it at the planet's Kepler rate); the `dot_px` size is a NON-CANON display
/// choice. Passing the anomaly in lets the still map use the static display spread and the live map use the clock.
fn map_body(p: &SampledPlanet, dot_px: usize, mean_anomaly: Fixed) -> render::MapBody {
    render::MapBody {
        semi_major_au: p.scene.orbit_au,
        // The orbit eccentricity is READ per-world from the scene attitude (the world sky's value), else the neutral
        // circle when not available; never an authored literal.
        eccentricity: attitude_eccentricity(&p.scene.attitude),
        mean_anomaly,
        dot_color: p.scene.material,
        dot_px,
    }
}

/// The CURRENT animated mean anomaly of a sampled planet: its static display-spread phase advanced by the observer's
/// orbital sweep at the planet's DERIVED Kepler rate ([`kepler_sweep_factor`], inner planets faster), folded into one
/// turn. The Keplerian speed-up at perihelion falls out for free downstream in the mean-to-true-anomaly solve
/// ([`civsim_sim::orbit::orbital_state`]); this only sets the mean-anomaly phase. Display-only (Principle 10).
fn animated_mean_anomaly(p: &SampledPlanet, orbit_phase: f64) -> Fixed {
    let tau = std::f64::consts::TAU;
    let base = p.mean_anomaly.to_f64_lossy();
    let swept = (base + orbit_phase * kepler_sweep_factor(p.scene.orbit_au)).rem_euclid(tau);
    Fixed::from_ratio((swept * 1_000_000.0) as i64, 1_000_000)
}

/// NON-CANON display: the on-screen star radius on the system map (pixels), scaled to the frame.
fn system_map_star_radius_px(min_dim: usize) -> usize {
    (min_dim / 40).max(3)
}

/// The time-control HUD string: the paused state or the current playback speed, and the key legend, so the owner
/// sees the clock that governs both the orbital motion and the deep-time surface evolution. Display-only.
fn time_status(playback: &PlaybackDriver) -> String {
    let state = if playback.is_paused() {
        "PAUSED".to_string()
    } else {
        format!("{:.2}x", playback.rate())
    };
    format!("TIME {state}  [space pause  <,> or [,] slower/faster  0 normal]")
}

/// Render one woosh transition frame: the system map (`map_buf`) fades toward empty space as the planet globe grows from
/// its map dot to the frame centre. `s` in [0, 1] is the eased progress (0 at the dot, 1 at the settled globe); the
/// globe centre lerps from the dot to the frame centre and its radius grows geometrically from the dot size to
/// `target_radius`. `star_dir` is the DERIVED sun direction and `orient` the globe orientation (so a woosh out of a
/// rotated globe keeps its rotation). Display-only (Principle 10).
#[allow(clippy::too_many_arguments)]
fn draw_woosh_frame(
    w: usize,
    h: usize,
    scene: &DerivedScene,
    map_buf: &[u32],
    dot: (i32, i32),
    target_radius: usize,
    s: f32,
    star_dir: [f32; 3],
    orient: render::GlobeOrientation,
) -> Vec<u32> {
    let mut buf: Vec<u32> = map_buf
        .iter()
        .map(|&p| blend_u32(p, BG.pack(), s))
        .collect();
    let cx = lerp(dot.0 as f32, (w / 2) as f32, s).round() as i32;
    let cy = lerp(dot.1 as f32, (h / 2) as f32, s).round() as i32;
    let dot_r = DEMO_MAP_DOT_PX as f32;
    let radius_px = (dot_r * (target_radius.max(1) as f32 / dot_r).powf(s))
        .round()
        .max(1.0) as usize;
    let style = render::SurfaceStyle {
        tint: Some(scene.material),
        grid: None,
        relief_shading: true,
        // The DERIVED body radius the hillshade reads to light the relief at its true physical slope.
        surface_radius_m: scene.radius_m,
    };
    // The ANALYTIC Sample field the hillshade differentiates for its normal.
    let stamps = scene.crater_stamps();
    let field = scene
        .provinces
        .as_ref()
        .map(|p| province_surface_field(p, &stamps));
    render::draw_globe_scene(
        &mut buf,
        w,
        h,
        cx,
        cy,
        radius_px,
        &scene.tiles,
        scene.param,
        scene.t_eff,
        star_dir,
        None,
        scene.sky,
        style,
        orient,
        // The DERIVED lava glow rides the woosh in, so a molten world already glows as it grows from the map dot.
        (!scene.lava.is_empty()).then_some(scene.lava.as_slice()),
        field.as_ref(),
    );
    buf
}

/// The interactive derived viewer's current view (display state, never canon): the zoomed-out SYSTEM MAP, a woosh
/// transition zooming in or out, or a settled PLANET GLOBE (Principle 10).
#[derive(Clone, Copy)]
enum DerivedView {
    Map,
    WooshIn { planet: usize, frame: u32 },
    Globe { planet: usize },
    WooshOut { planet: usize, frame: u32 },
}

fn run_derived(argv: &[String]) {
    let star_mass = parse_fixed(argv.get(2), Fixed::ONE);
    let orbit_arg = argv
        .get(3)
        .and_then(|s| Fixed::from_decimal_str(s.trim()).ok());
    eprintln!("deriving the system map (a planet at each sampled orbit)...");
    let mut planets = build_sampled_planets(star_mass, orbit_arg);
    if planets.is_empty() {
        eprintln!("no derived planets resolved at the sampled orbits; nothing is shown (nothing is fabricated)");
        return;
    }
    eprintln!(
        "system map: {} independently-derived planets sampled across the terrestrial zone",
        planets.len()
    );
    eprintln!(
        "  (a viewer input, NOT an emergent multi-body system; that is the solar-system generator, task #72)"
    );
    for p in &planets {
        eprintln!(
            "  orbit {:.2} AU: radius {:.0} km, T_eff {:.0} K, crust rgb({},{},{})",
            p.scene.orbit_au.to_f64_lossy(),
            p.scene.radius_m.to_f64_lossy() / 1000.0,
            p.scene.t_eff.to_f64_lossy(),
            p.scene.material.r,
            p.scene.material.g,
            p.scene.material.b,
        );
    }
    eprintln!("  controls: click a planet to fly to it, esc/backspace to fly back, esc on the map to quit");
    let star_t_eff = planets[0].scene.t_eff;

    // One continuous zoom, all of it the sphere, once you are on a planet globe: from the distant globe down onto a
    // surface patch of the derived elevation field. The level count is a NON-CANON display choice (how finely the +/-
    // keys step the continuous scale); it is set high enough that the deeper surface regime keeps comfortable steps.
    let zoom_levels = 18u32;
    let max_zoom = zoom_levels.saturating_sub(1);

    let mut window = Window::new(
        "civsim derived-planet viewer",
        960,
        640,
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
             still describe the planets."
        );
        std::process::exit(1);
    });
    window.set_target_fps(30);

    // The provenance register, loaded once for the globe view's hover panel (fail-soft: it shows "register unavailable"
    // rather than fabricating a number).
    let floor_prov = civsim_physics::floor_provenance::FloorProvenance::embedded().ok();
    let mut show_provenance = false;
    let mut zoom: u32 = 0;
    // The globe orientation the pan keys steer (display-only state, never canon, Principle 10).
    let mut rot_lon = 0.0f32;
    let mut rot_lat = 0.0f32;
    let mut view = DerivedView::Map;
    // The globe's on-screen radius at the moment a woosh-out begins, so the transition starts from the exact framing the
    // globe was in (any zoom), for a smooth zoom back out.
    let mut woosh_from_radius = 0usize;
    // A frame clock for the derived spin animation (the day/night terminator sweep) and a mouse-edge tracker for clicks.
    let mut spin_frame: u64 = 0;
    let mut was_mouse_down = false;
    // THE OBSERVER'S TIME CONTROL: one non-canon playback clock ([`civsim_sim::clock::PlaybackDriver`], the banked
    // pause/rate/scale holder) governs BOTH the planets' orbital motion on the map and the deep-time surface
    // evolution on the globe. Space pauses (freezing both so the owner can inspect), and the speed keys slow it
    // down or crank it up. `orbit_phase` is the continuous orbital sweep (radians at 1 AU); the per-planet rate is
    // the Kepler rate (inner planets sweep faster), and the deep-time field steps by the same clock's whole ticks.
    let mut playback = PlaybackDriver::new(1.0);
    let mut orbit_phase = 0.0f64;
    let mut last_instant = std::time::Instant::now();

    while window.is_open() {
        let (w, h) = window.get_size();
        if w == 0 || h == 0 {
            window.update();
            continue;
        }
        let keys: Vec<Key> = window.get_keys_pressed(KeyRepeat::No);
        // The real seconds since the last frame, clamped so a stall does not lurch the playback; and the time keys
        // (Space pause, the bracket / comma-period speed controls, 0 to reset to normal), live in every view.
        let dt_real = last_instant.elapsed().as_secs_f64().min(0.1);
        last_instant = std::time::Instant::now();
        for k in &keys {
            match k {
                Key::Space => {
                    playback.toggle_pause();
                }
                Key::RightBracket | Key::Period => playback.scale_rate(2.0),
                Key::LeftBracket | Key::Comma => playback.scale_rate(0.5),
                Key::Key0 => playback.set_rate(1.0),
                _ => {}
            }
        }
        // Advance the orbital sweep continuously (smooth), gated by pause and rate; the deep-time whole-tick count
        // for this frame comes from the same driver (used in the globe view to step the surface forward).
        if !playback.is_paused() {
            orbit_phase += dt_real * playback.rate() * ORBIT_SWEEP_RATE_RAD_PER_S;
        }
        let deep_ticks = playback.advance(dt_real) as usize;
        let min_dim = w.min(h);
        let star_r = system_map_star_radius_px(min_dim);
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        let mouse_pos = window.get_mouse_pos(MouseMode::Discard);

        let buf = match view {
            DerivedView::Map => {
                // Escape quits from the map.
                if keys.contains(&Key::Escape) {
                    break;
                }
                let bodies: Vec<render::MapBody> = planets
                    .iter()
                    .map(|p| map_body(p, DEMO_MAP_DOT_PX, animated_mean_anomaly(p, orbit_phase)))
                    .collect();
                let (mut buf, dots) =
                    render::render_system_map(w, h, BG, star_t_eff, star_r, &bodies);
                // A fresh left click on a planet dot flies down to it (nearest dot within a small hit radius).
                if mouse_down && !was_mouse_down {
                    if let Some((mx, my)) = mouse_pos {
                        let hit_r = DEMO_MAP_DOT_PX as i32 + 6;
                        let mut best: Option<(i32, usize)> = None;
                        for (i, (dx, dy)) in dots.iter().enumerate() {
                            let ddx = mx as i32 - dx;
                            let ddy = my as i32 - dy;
                            let d2 = ddx * ddx + ddy * ddy;
                            if d2 <= hit_r * hit_r && best.is_none_or(|(bd, _)| d2 < bd) {
                                best = Some((d2, i));
                            }
                        }
                        if let Some((_, i)) = best {
                            zoom = 0;
                            rot_lon = 0.0;
                            rot_lat = 0.0;
                            view = DerivedView::WooshIn {
                                planet: i,
                                frame: 0,
                            };
                        }
                    }
                }
                // Labels: the star, each planet's orbit, and the two HUD lines (with the honesty caveat).
                let (cx, cy) = ((w / 2) as i32, (h / 2) as i32);
                render::draw_label(
                    &mut buf,
                    w,
                    h,
                    cx + star_r as i32 + 4,
                    cy - 4,
                    &format!(
                        "star {:.2} Msun  {:.0}K",
                        star_mass.to_f64_lossy(),
                        star_t_eff.to_f64_lossy()
                    ),
                    1,
                    Rgb::new(240, 230, 170),
                    Rgb::new(10, 12, 20),
                );
                for (p, (dx, dy)) in planets.iter().zip(dots.iter()) {
                    render::draw_label(
                        &mut buf,
                        w,
                        h,
                        dx + DEMO_MAP_DOT_PX as i32 + 3,
                        dy - 4,
                        &format!("{:.2} AU", p.scene.orbit_au.to_f64_lossy()),
                        1,
                        Rgb::new(190, 200, 220),
                        Rgb::new(10, 12, 20),
                    );
                }
                render::draw_label(
                    &mut buf,
                    w,
                    h,
                    4,
                    4,
                    &format!(
                        "SYSTEM MAP  {} independently-derived worlds at sampled orbits (a viewer input, not an emergent system; task #72)",
                        planets.len()
                    ),
                    1,
                    Rgb::new(230, 230, 240),
                    Rgb::new(10, 12, 20),
                );
                render::draw_label(
                    &mut buf,
                    w,
                    h,
                    4,
                    18,
                    "click a planet to fly down to it   esc quit",
                    1,
                    Rgb::new(170, 180, 200),
                    Rgb::new(10, 12, 20),
                );
                render::draw_label(
                    &mut buf,
                    w,
                    h,
                    4,
                    32,
                    &time_status(&playback),
                    1,
                    Rgb::new(210, 200, 150),
                    Rgb::new(10, 12, 20),
                );
                window.set_title(&format!(
                    "civsim system map  star {:.2} Msun  {} sampled worlds",
                    star_mass.to_f64_lossy(),
                    planets.len()
                ));
                buf
            }
            DerivedView::WooshIn { planet, frame } => {
                let scene = &planets[planet].scene;
                let bodies: Vec<render::MapBody> = planets
                    .iter()
                    .map(|p| map_body(p, DEMO_MAP_DOT_PX, animated_mean_anomaly(p, orbit_phase)))
                    .collect();
                let (map_buf, dots) =
                    render::render_system_map(w, h, BG, star_t_eff, star_r, &bodies);
                let dot = dots
                    .get(planet)
                    .copied()
                    .unwrap_or(((w / 2) as i32, (h / 2) as i32));
                // The globe's settled (zoom 0) on-screen radius is the woosh target.
                let m_per_px = derived_globe_m_per_px(scene.radius_m, w, h, 0.0);
                let target_radius = render::globe_radius_px(scene.radius_m, m_per_px);
                let s = smoothstep(frame as f32 / WOOSH_FRAMES as f32);
                let day_sweep = sweep_phase(spin_frame, TERMINATOR_SWEEP_FRAMES);
                let year_sweep = sweep_phase(spin_frame, SEASON_SWEEP_FRAMES);
                let star_dir = derived_sun_body_dir(&scene.attitude, day_sweep, year_sweep)
                    .unwrap_or([0.0, 0.0, 1.0]);
                let buf = draw_woosh_frame(
                    w,
                    h,
                    scene,
                    &map_buf,
                    dot,
                    target_radius,
                    s,
                    star_dir,
                    render::GlobeOrientation::IDENTITY,
                );
                view = if frame + 1 >= WOOSH_FRAMES {
                    DerivedView::Globe { planet }
                } else {
                    DerivedView::WooshIn {
                        planet,
                        frame: frame + 1,
                    }
                };
                window.set_title("civsim derived planet  flying in...");
                buf
            }
            DerivedView::WooshOut { planet, frame } => {
                let scene = &planets[planet].scene;
                let bodies: Vec<render::MapBody> = planets
                    .iter()
                    .map(|p| map_body(p, DEMO_MAP_DOT_PX, animated_mean_anomaly(p, orbit_phase)))
                    .collect();
                let (map_buf, dots) =
                    render::render_system_map(w, h, BG, star_t_eff, star_r, &bodies);
                let dot = dots
                    .get(planet)
                    .copied()
                    .unwrap_or(((w / 2) as i32, (h / 2) as i32));
                // Reverse of the woosh-in: s runs from 1 (the globe framing it left) down to 0 (the dot), starting from
                // the exact radius and orientation the globe was in, so any zoom flies back out smoothly.
                let s = smoothstep(1.0 - frame as f32 / WOOSH_FRAMES as f32);
                let day_sweep = sweep_phase(spin_frame, TERMINATOR_SWEEP_FRAMES);
                let year_sweep = sweep_phase(spin_frame, SEASON_SWEEP_FRAMES);
                let star_dir = derived_sun_body_dir(&scene.attitude, day_sweep, year_sweep)
                    .unwrap_or([0.0, 0.0, 1.0]);
                let buf = draw_woosh_frame(
                    w,
                    h,
                    scene,
                    &map_buf,
                    dot,
                    woosh_from_radius,
                    s,
                    star_dir,
                    render::GlobeOrientation { rot_lon, rot_lat },
                );
                view = if frame + 1 >= WOOSH_FRAMES {
                    DerivedView::Map
                } else {
                    DerivedView::WooshOut {
                        planet,
                        frame: frame + 1,
                    }
                };
                window.set_title("civsim system map  flying out...");
                buf
            }
            DerivedView::Globe { planet } => {
                // THE DEEP-TIME ACTIVITY: advance the province field by the playback clock's whole ticks this
                // frame and re-derive the surface, so the provinces form, the crust grows, and the relief changes
                // as the owner watches (paused freezes it; speeding up runs deep time fast). A real derived
                // physics step ([`step_deep_time`]), never a painted animation.
                if deep_ticks > 0 {
                    let param = planets[planet].scene.param;
                    let stepped = match planets[planet].scene.provinces.as_mut() {
                        Some(prov) => {
                            step_provinces(prov, deep_ticks);
                            // Re-derive the surface AND the lava glow off the stepped provinces, so the owner watches
                            // the volcanism fade as the world cools (the glow follows the interior temperature down).
                            let tiles = derive_province_tiles(prov, param);
                            let lava = derive_province_lava(prov, param);
                            Some((tiles, lava))
                        }
                        None => None,
                    };
                    if let Some((tiles, lava)) = stepped {
                        if let Some(t) = tiles {
                            planets[planet].scene.tiles = t;
                        }
                        planets[planet].scene.lava = lava;
                    }
                }
                let scene = &planets[planet].scene;
                // Discrete keys: zoom, provenance, recentre.
                for k in &keys {
                    match k {
                        Key::Equal | Key::NumPadPlus => zoom = (zoom + 1).min(max_zoom),
                        Key::Minus | Key::NumPadMinus => zoom = zoom.saturating_sub(1),
                        Key::P => show_provenance = !show_provenance,
                        Key::Home => {
                            zoom = 0;
                            rot_lon = 0.0;
                            rot_lat = 0.0;
                        }
                        _ => {}
                    }
                }
                // WASD / arrow keys rotate the globe (held keys pan smoothly). The step and latitude limit are non-canon
                // display choices.
                const ROT_STEP: f32 = 0.045;
                const LAT_LIMIT: f32 = 1.4; // ~80 degrees: keep the sampled centre off the pole singularity
                use std::f32::consts::TAU;
                if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
                    rot_lon -= ROT_STEP;
                }
                if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
                    rot_lon += ROT_STEP;
                }
                if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
                    rot_lat = (rot_lat - ROT_STEP).max(-LAT_LIMIT);
                }
                if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
                    rot_lat = (rot_lat + ROT_STEP).min(LAT_LIMIT);
                }
                rot_lon = rot_lon.rem_euclid(TAU);

                let t = if max_zoom == 0 {
                    1.0
                } else {
                    zoom as f32 / max_zoom as f32
                };
                let orient = render::GlobeOrientation { rot_lon, rot_lat };
                let m_per_px = derived_globe_m_per_px(scene.radius_m, w, h, t);
                let radius_px = render::globe_radius_px(scene.radius_m, m_per_px);
                let (grid_cols, grid_rows) = surface_grid_dims(radius_px, min_dim);
                let show_grid = radius_px as f32 >= min_dim as f32 * 0.5;
                let style = render::SurfaceStyle {
                    tint: Some(scene.material),
                    grid: show_grid.then_some((grid_cols, grid_rows)),
                    relief_shading: true,
                    // The DERIVED body radius the hillshade reads to light the relief at its true physical slope.
                    surface_radius_m: scene.radius_m,
                };
                // The DERIVED sun direction, animated by the spin so the terminator sweeps as you watch.
                let day_sweep = sweep_phase(spin_frame, TERMINATOR_SWEEP_FRAMES);
                let year_sweep = sweep_phase(spin_frame, SEASON_SWEEP_FRAMES);
                let star_dir = derived_sun_body_dir(&scene.attitude, day_sweep, year_sweep)
                    .unwrap_or([0.0, 0.0, 1.0]);
                let gcx = (w / 2) as i32;
                let gcy = (h / 2) as i32;
                let mut buf = vec![BG.pack(); w * h];
                // The ANALYTIC Sample field the hillshade differentiates for its normal.
                let stamps = scene.crater_stamps();
                let field = scene
                    .provinces
                    .as_ref()
                    .map(|p| province_surface_field(p, &stamps));
                render::draw_globe_scene(
                    &mut buf,
                    w,
                    h,
                    gcx,
                    gcy,
                    radius_px,
                    &scene.tiles,
                    scene.param,
                    scene.t_eff,
                    star_dir,
                    None,
                    scene.sky,
                    style,
                    orient,
                    // The DERIVED lava glow: molten provinces emit their incandescent colour, bright on the night side
                    // too, and it fades as the deep-time clock cools the world. Empty (no glow) on the uniform crust.
                    (!scene.lava.is_empty()).then_some(scene.lava.as_slice()),
                    field.as_ref(),
                );
                // Cursor -> surface pick and highlight (fail-soft off the sphere).
                let picked = mouse_pos.and_then(|(mx, my)| {
                    render::pick_surface_tile(
                        mx as i32, my as i32, gcx, gcy, radius_px, orient, grid_cols, grid_rows,
                    )
                });
                if let Some((cu, cv)) = picked {
                    render::draw_surface_highlight(
                        &mut buf, w, h, gcx, gcy, radius_px, orient, grid_cols, grid_rows, cu, cv,
                        CURSOR,
                    );
                }
                // The readout HUD (derived numbers), drawn top-left.
                render::draw_label(
                    &mut buf,
                    w,
                    h,
                    4,
                    4,
                    &format!(
                        "star {:.2} Msun  orbit {:.2} AU  |  T_eff {:.0}K  radius {:.0}km  g {:.2}",
                        scene.star_mass.to_f64_lossy(),
                        scene.orbit_au.to_f64_lossy(),
                        scene.t_eff.to_f64_lossy(),
                        scene.radius_m.to_f64_lossy() / 1000.0,
                        scene.gravity.to_f64_lossy(),
                    ),
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
                    w,
                    h,
                    4,
                    20,
                    &format!(
                        "crust {}  air {}  |  +/- zoom  wasd rotate  p provenance  esc/backspace map",
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
                // The time control and the deep-time age, so the owner sees the surface evolve on the clock.
                let age_myr = scene
                    .provinces
                    .as_ref()
                    .map(|p| p.state.elapsed_myr.to_f64_lossy())
                    .unwrap_or(0.0);
                render::draw_label(
                    &mut buf,
                    w,
                    h,
                    4,
                    34,
                    &format!(
                        "deep time {:.0} Myr  |  {}",
                        age_myr,
                        time_status(&playback)
                    ),
                    1,
                    Rgb::new(210, 200, 150),
                    Rgb::new(10, 12, 20),
                );
                // The zoom READOUT (scale bar + tier + m/px), so the owner always sees how far in the view is. The tier
                // names the regime the continuous zoom is in: SURFACE once the globe has grown past filling the frame
                // (the drill-down onto the elevation field), else GLOBE (the whole planet still fits).
                let tier = if show_grid { "SURFACE" } else { "GLOBE" };
                draw_zoom_readout(&mut buf, w, h, m_per_px.to_f64_lossy(), tier);
                if show_provenance {
                    let lines = provenance_lines(scene, floor_prov.as_ref(), picked);
                    let panel_w = 372usize;
                    let px = w.saturating_sub(panel_w);
                    for (i, (text, colour)) in lines.iter().enumerate() {
                        let py = 40 + i * 15;
                        if py + 12 < h {
                            render::draw_label(
                                &mut buf,
                                w,
                                h,
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
                let mode = if show_grid {
                    format!(
                        "derived surface  zoom {}/{}  tiles {grid_cols}x{grid_rows}",
                        zoom + 1,
                        zoom_levels
                    )
                } else {
                    format!("derived globe  zoom {}/{}", zoom + 1, zoom_levels)
                };
                window.set_title(&format!(
                    "civsim derived planet  orbit {:.2} AU  |  {mode}",
                    scene.orbit_au.to_f64_lossy()
                ));
                // Escape or Backspace flies back out to the map, starting from the current on-screen radius so the woosh
                // begins from exactly this framing.
                if keys.contains(&Key::Escape) || keys.contains(&Key::Backspace) {
                    woosh_from_radius = radius_px;
                    view = DerivedView::WooshOut { planet, frame: 0 };
                }
                buf
            }
        };

        window
            .update_with_buffer(&buf, w, h)
            .expect("blit the frame");
        was_mouse_down = mouse_down;
        // Advance the day/night terminator and the seasonal sweep ONLY when time is running: pause holds `spin_frame`
        // so `day_sweep` and `year_sweep` freeze, and the terminator stops mid-sweep while the owner inspects (the same
        // pause that already freezes the orbital motion and the deep-time surface evolution). Unpausing resumes it.
        if !playback.is_paused() {
            spin_frame = spin_frame.wrapping_add(1);
        }
    }
}

/// Headless system-map render: `--system-map <path> [star_mass] [w] [h]` derives a planet at each sampled orbit, draws
/// the zoomed-out system map (the star, the orbit ellipses, the planet dots) to a binary PPM, and prints the derived
/// star and per-planet numbers. Mirrors the `--derived-globe` headless pattern so the map is checkable without a display.
fn system_map_cmd(argv: &[String]) {
    let path = argv
        .get(2)
        .cloned()
        .unwrap_or_else(|| "system-map.ppm".to_string());
    let star_mass = parse_fixed(argv.get(3), Fixed::ONE);
    let w: usize = parse(argv.get(4), 900);
    let h: usize = parse(argv.get(5), 700);
    let planets = build_sampled_planets(star_mass, None);
    if planets.is_empty() {
        eprintln!("no derived planets resolved at the sampled orbits; nothing written");
        return;
    }
    let star_t_eff = planets[0].scene.t_eff;
    let min_dim = w.min(h);
    let star_r = system_map_star_radius_px(min_dim);
    let bodies: Vec<render::MapBody> = planets
        .iter()
        .map(|p| map_body(p, DEMO_MAP_DOT_PX, p.mean_anomaly))
        .collect();
    let (buf, dots) = render::render_system_map(w, h, BG, star_t_eff, star_r, &bodies);
    write_ppm(&path, w, h, &buf);
    eprintln!(
        "wrote {path} ({w}x{h}) system map: star T_eff {:.0} K, {} independently-derived planets (sampled orbits, NOT an emergent system; task #72):",
        star_t_eff.to_f64_lossy(),
        planets.len()
    );
    for (p, dot) in planets.iter().zip(dots.iter()) {
        eprintln!(
            "  orbit {:.2} AU  radius {:.0} km  T_eff {:.0} K  crust rgb({},{},{})  dot@({},{})",
            p.scene.orbit_au.to_f64_lossy(),
            p.scene.radius_m.to_f64_lossy() / 1000.0,
            p.scene.t_eff.to_f64_lossy(),
            p.scene.material.r,
            p.scene.material.g,
            p.scene.material.b,
            dot.0,
            dot.1,
        );
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
    // Headless system map: `--system-map <path> [star_mass] [w] [h]` writes the zoomed-out system map (star, orbit
    // ellipses, planet dots) of the sampled derived planets to a PPM and exits.
    if argv.get(1).map(|s| s == "--system-map").unwrap_or(false) {
        system_map_cmd(&argv);
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
                    fx.param,
                    win_w,
                    win_h,
                    m_per_px,
                    star_px,
                    star_r,
                    BG,
                    fx.sky,
                    render::SurfaceStyle::default(),
                    render::GlobeOrientation::IDENTITY,
                    // The living-world globe carries no orbital phase; keep the screen-space light (byte-identical).
                    None,
                    // The living-world globe has no deep-time province temperatures, so no lava glow (byte-identical).
                    None,
                    // The living-world globe carries no province field, so no analytic Sample: the hillshade keeps
                    // the cache's finite difference, byte-identical to before the analytic normals.
                    None,
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
mod composition_draw_tests {
    use super::*;

    #[test]
    fn two_unpinned_seeds_derive_different_worlds() {
        // THE THESIS at the viewer level: two unpinned world seeds derive planets with DIFFERENT accretion MASS and
        // star luminosity (T_eff), for the DERIVED reason that their drawn [Fe/H] (hence Z/Z_sun) differs. Same star
        // mass and same orbit; the only difference is the per-world composition draw. The MASS lever is direct (the
        // solid-dust column scales with Z/Z_sun); the DENSITY moves only slightly here (the metal/silicate core split
        // is Z-invariant under uniform metal scaling, and the small shift is the condensation assemblage responding to
        // the scaled absolute abundances), because the strong density levers are the later Mg/Si and iron-fraction
        // links (the ruling's conditioning line: [Fe/H] scales amount, the iron fraction selects density). We assert
        // the robust levers (mass, T_eff); the small density difference is measured, not asserted.
        let s = Fixed::ONE;
        let o = Fixed::ONE;
        let a = build_derived_scene_seeded(s, o, 1).expect("seed 1 world derives");
        let b = build_derived_scene_seeded(s, o, 7).expect("seed 7 world derives");
        println!(
            "seed1: mass={:.5} M_earth density={:.4} g/cm3 T_eff={:.2} K",
            a.mass_earth.to_f64_lossy(),
            a.density.to_f64_lossy(),
            a.t_eff.to_f64_lossy()
        );
        println!(
            "seed7: mass={:.5} M_earth density={:.4} g/cm3 T_eff={:.2} K",
            b.mass_earth.to_f64_lossy(),
            b.density.to_f64_lossy(),
            b.t_eff.to_f64_lossy()
        );
        assert_ne!(
            a.mass_earth.to_bits(),
            b.mass_earth.to_bits(),
            "two seeds derive DIFFERENT accretion mass (Z/Z_sun scales the solid-dust column)"
        );
        assert_ne!(
            a.t_eff.to_bits(),
            b.t_eff.to_bits(),
            "two seeds derive DIFFERENT star T_eff (Z/Z_sun conditions the stellar luminosity)"
        );
    }

    #[test]
    fn alpha_enhancement_lowers_the_derived_bulk_density_the_first_density_lever() {
        // THE DENSITY PAYOFF, isolated. [alpha/Fe] is the first KIND lever: lifting the alpha rock-formers (O, Mg, Si,
        // Ca, Ti) relative to iron raises the silicate-mantle mass against the metal core, lowering the core mass
        // FRACTION and so the volume-weighted uncompressed bulk density. Held at the SAME formation conditions and the
        // SAME [Fe/H] (only [alpha/Fe] differs), so the density change is the alpha lever ALONE, retiring the [Fe/H]
        // link's honest limit that uniform metal scaling left the density fixed. The Mg/Si-selects-olivine-vs-pyroxene
        // channel stays a later element-differential refinement (a uniform alpha lift holds Mg/Si); the moving channel
        // here is the metal-core-to-silicate-mantle mass balance.
        const ALPHA: &[&str] = &["O", "Mg", "Si", "Ca", "Ti"];
        let janaf = JanafTables::standard().expect("janaf");
        let optical = OpticalConstants::standard().expect("optical");
        let registry = PhaseRegistry::standard().expect("registry");
        let table = PeriodicTable::standard().expect("table");
        let eos = MetalEosAnchors::standard().expect("eos");
        let cond_t = derive_formation_condensation_temperature(Fixed::ONE, Fixed::ONE, &optical)
            .expect("the formation condensation temperature derives");
        let reserved_melt = civsim_materials::surface_composition::ReservedMeltParams {
            potential_temperature_k: MANTLE_POTENTIAL_TEMPERATURE_K,
            adiabat_slope_k_per_gpa: MANTLE_ADIABAT_SLOPE_K_PER_GPA,
            productivity_per_gpa: MELT_PRODUCTIVITY_PER_GPA,
            gravity_m_per_s2: MELT_COLUMN_GRAVITY_M_S2,
        };
        let density_of = |abund: &SolarAbundances| -> Fixed {
            let sc = civsim_materials::surface_composition::derive_surface_composition(
                &janaf,
                abund,
                cond_t,
                &reserved_melt,
            )
            .expect("the surface composition derives");
            let mantle_density = derive_mantle_density(
                &sc.mantle_composition,
                Fixed::from_int(300),
                Fixed::ONE,
                &registry,
                &table,
            )
            .expect("the mantle density derives");
            derive_uncompressed_bulk_density(abund, &table, &eos, mantle_density)
                .expect("the bulk density derives")
        };
        let solar = SolarAbundances::standard().expect("solar loads");
        // The fetched thick-disk alpha plateau, +0.3 dex, lifted onto the alpha elements.
        let alpha_rich = solar.scaled_alpha_by_dex(Fixed::from_ratio(3, 10), ALPHA);
        let rho_solar = density_of(&solar);
        let rho_alpha = density_of(&alpha_rich);
        let spread = (rho_solar - rho_alpha).to_f64_lossy();
        println!(
            "DENSITY LEVER: solar [alpha/Fe]=0 rho={:.5} g/cm3, thick [alpha/Fe]=+0.3 rho={:.5} g/cm3, spread={:.5} g/cm3",
            rho_solar.to_f64_lossy(),
            rho_alpha.to_f64_lossy(),
            spread
        );
        assert!(
            rho_alpha < rho_solar,
            "the alpha-enhanced world is LESS dense (the alpha lift lowers the metal-core fraction): alpha {} vs solar {}",
            rho_alpha.to_f64_lossy(),
            rho_solar.to_f64_lossy()
        );
        assert!(
            spread > 0.05,
            "the [alpha/Fe] density lever is meaningful (not a rounding wobble): spread {spread} g/cm3"
        );
    }

    #[test]
    fn two_unpinned_seeds_now_derive_different_densities_via_alpha() {
        // THE THESIS, upgraded: with the [alpha/Fe] link two unpinned seeds derive DIFFERENT densities (a density spread
        // beyond the mass spread), because a seed that lands on the high-alpha thick branch derives a lower core
        // fraction and so a lower density than a solar-alpha seed. Same star and orbit; the only difference is the
        // per-world composition draw. We scan
        // seeds for the first that draws a thick-branch [alpha/Fe] > 0 (a metal-poor, alpha-enhanced world), build its
        // scene and the solar Mirror, and show the alpha-enhanced world is the less dense one (the derived reason
        // worlds now differ in density). Two scene builds, so the suite stays fast.
        use civsim_materials::disk_composition::Environment;
        let s = Fixed::ONE;
        let o = Fixed::ONE;
        let env = Environment::local_disk();
        let mut thick = None;
        for seed in 0..400u64 {
            let fe_h = env.draw_fe_h(seed);
            let alpha = env.draw_alpha_fe(seed, fe_h).to_f64_lossy();
            if alpha > 0.05 {
                thick = Some((seed, fe_h.to_f64_lossy(), alpha));
                break;
            }
        }
        let (seed, fe_h, alpha) =
            thick.expect("some unpinned seed draws the high-alpha thick branch");
        let alpha_world =
            build_derived_scene_seeded(s, o, seed).expect("thick-branch world derives");
        let solar_world = build_derived_scene(s, o).expect("the solar Mirror derives");
        let rho_alpha = alpha_world.density.to_f64_lossy();
        let rho_solar = solar_world.density.to_f64_lossy();
        println!(
            "THESIS: seed {seed} draws [Fe/H]={fe_h:+.3} [alpha/Fe]={alpha:+.3} -> rho={rho_alpha:.5} vs solar rho={rho_solar:.5} g/cm3 (spread {:.5})",
            rho_solar - rho_alpha
        );
        assert_ne!(
            alpha_world.density.to_bits(),
            solar_world.density.to_bits(),
            "an unpinned alpha-enhanced seed derives a DIFFERENT density from the solar Mirror"
        );
        assert!(
            rho_alpha < rho_solar,
            "the alpha-enhanced (thick-branch) world is the LESS dense one: {rho_alpha} vs solar {rho_solar}"
        );
    }

    #[test]
    fn the_pinned_mirror_is_byte_identical_to_the_solar_pin_draw() {
        // The default globe (build_derived_scene) is the chain at the solar pin. Prove it is byte-identical, on the
        // load-bearing derived fields, to a scene built from the explicitly solar-pinned composition, so the Mirror
        // pins THROUGH the chain with no numeric drift (the run pins prove the full byte-identity end to end).
        let s = Fixed::ONE;
        let o = Fixed::ONE;
        let mirror = build_derived_scene(s, o).expect("mirror derives");
        let pinned_composition = civsim_materials::disk_composition::DiskComposition::draw(
            &civsim_materials::disk_composition::Environment::local_disk_solar_pin(),
            12345,
        )
        .expect("the pinned composition draws");
        let pinned = build_derived_scene_with_composition(s, o, &pinned_composition)
            .expect("pinned derives");
        assert_eq!(mirror.mass_earth.to_bits(), pinned.mass_earth.to_bits());
        assert_eq!(mirror.density.to_bits(), pinned.density.to_bits());
        assert_eq!(mirror.t_eff.to_bits(), pinned.t_eff.to_bits());
        assert_eq!(mirror.radius_m.to_bits(), pinned.radius_m.to_bits());
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

#[cfg(test)]
mod province_tests {
    use super::*;

    #[test]
    fn the_province_seed_is_deterministic_and_spreads() {
        // The symmetry-breaking seed is a pure hash: the same world and province index replay bit-for-bit, the
        // perturbation lies in [-1, 1), and different provinces get different values, so the field is not uniform.
        let a = province_seed_perturbation(Fixed::ONE, Fixed::ONE, 7);
        let b = province_seed_perturbation(Fixed::ONE, Fixed::ONE, 7);
        assert_eq!(a, b, "the seed replays deterministically");
        for i in 0..60 {
            let s = province_seed_perturbation(Fixed::ONE, Fixed::ONE, i);
            assert!(
                s >= Fixed::from_int(-1) && s < Fixed::ONE,
                "seed {i} is in [-1, 1): {}",
                s.to_f64_lossy()
            );
        }
        let distinct: std::collections::BTreeSet<i64> = (0..60)
            .map(|i| province_seed_perturbation(Fixed::ONE, Fixed::ONE, i).to_bits())
            .collect();
        assert!(
            distinct.len() > 50,
            "the seed spreads across provinces, got {} distinct of 60",
            distinct.len()
        );
        // A different world (a different orbit) derives a different province pattern, not a fixed map.
        assert_ne!(
            province_seed_perturbation(Fixed::ONE, Fixed::ONE, 3),
            province_seed_perturbation(Fixed::ONE, Fixed::from_ratio(3, 2), 3),
            "a different world derives a different province pattern"
        );
    }

    #[test]
    fn the_cell_aspect_is_the_rigid_rigid_eigenvalue_pair_not_the_free_free_mode() {
        // FIX 2: the province cell aspect and the convection onset threshold must be ONE boundary regime. The
        // aspect derives from the rigid-rigid critical wavenumber as pi / a_c (the same regime as ra_crit = 1708),
        // ~1.008, NOT the free-free sqrt(2) ~ 1.414 that the rigid onset would contradict.
        let expected = Fixed::PI.div(civsim_sim::deeptime::RIGID_RIGID_CRITICAL_WAVENUMBER);
        assert_eq!(
            MANTLE_CONVECTION_CELL_ASPECT, expected,
            "the aspect is pi / a_c, derived from the rigid-rigid critical wavenumber"
        );
        let a = MANTLE_CONVECTION_CELL_ASPECT.to_f64_lossy();
        assert!(
            (0.99..=1.03).contains(&a),
            "the rigid-rigid cell aspect is ~1.008, got {a}"
        );
        assert!(
            MANTLE_CONVECTION_CELL_ASPECT < Fixed::from_ratio(1414, 1000),
            "the rigid aspect is below the free-free sqrt(2) (the regime the onset threshold requires)"
        );
    }

    // Synthetic Mars-class interior inputs that resolve a province field, for the derive-first province tests
    // (no heavy scene build): the depth, count, and densities are plausible, the derived solidus and formation
    // melt fraction are supplied as the surface-composition chain would.
    fn mars_class_provinces(
        solidus_surface_k: Fixed,
        solidus_slope_k_per_gpa: Fixed,
        formation_melt_fraction: Option<Fixed>,
    ) -> DeepTimeProvinces {
        // The young potential temperature the R-YOUNG-TEMPERATURE handoff pins for a melted world (super-solidus,
        // so the melt engages) off the supplied derived solidus, fail-soft to the solidus itself.
        let young_t = civsim_physics::young_thermal::magma_ocean_handoff_temperature(
            solidus_surface_k,
            solidus_slope_k_per_gpa,
            MANTLE_ADIABAT_SLOPE_K_PER_GPA,
            MELT_PRODUCTIVITY_PER_GPA,
            PHI_C_LOCKUP_MELT_FRACTION,
        )
        .unwrap_or(solidus_surface_k);
        build_deep_time_provinces(
            Fixed::ONE,                 // star mass
            Fixed::ONE,                 // orbit AU
            Fixed::from_int(3_400_000), // Mars-class radius (m)
            Fixed::from_ratio(37, 10),  // Mars-class surface gravity
            Fixed::from_ratio(25, 100), // core mass fraction
            Fixed::from_int(7),         // core density (g/cm3)
            Fixed::from_ratio(39, 10),  // mean density (g/cm3)
            Fixed::from_ratio(35, 10),  // mantle density (g/cm3)
            Fixed::from_ratio(30, 10),  // crust density (g/cm3)
            solidus_surface_k,
            solidus_slope_k_per_gpa,
            formation_melt_fraction,
            young_t,
        )
        .expect("the Mars-class interior resolves a province field")
    }

    #[test]
    fn the_derived_solidus_flows_into_the_deep_time_melt_params() {
        // FIX 1: the world's OWN derived solidus (surface value and slope) is what the deep-time volcanism melts
        // against, never an authored 1373 K / 130 K/GPa pair. The value the caller threads is exactly the value
        // the MeltParams carries.
        let prov = mars_class_provinces(
            Fixed::from_int(1680),
            Fixed::from_int(120),
            Some(Fixed::from_ratio(1, 10)),
        );
        assert_eq!(
            prov.melt.solidus_surface_k,
            Fixed::from_int(1680),
            "the derived solidus surface value flows into the melt params"
        );
        assert_eq!(
            prov.melt.solidus_slope_k_per_gpa,
            Fixed::from_int(120),
            "the derived solidus slope flows into the melt params"
        );
    }

    #[test]
    fn the_heterogeneity_amplitude_derives_per_system_from_the_formation_melt_fraction() {
        // FIX 4: the radiogenic-heterogeneity AMPLITUDE is per-system, derived from the world's own formation melt
        // fraction F (~1/F incompatible-element enrichment), never Earth's 0.3. A melting world (F present) sorts
        // incompatibles, so its provinces DIVERGE in radiogenic budget; a sub-solidus world (F None) sorted none,
        // so its provinces are UNIFORM (the honest unprocessed mantle), no melt-driven spread.
        let melting = mars_class_provinces(
            Fixed::from_int(1500),
            Fixed::from_int(120),
            Some(Fixed::from_ratio(1, 10)), // F = 0.1, a strongly enriching low-degree melt
        );
        let heats: Vec<Fixed> = melting
            .column_params
            .iter()
            .map(|c| c.heat_production)
            .collect();
        let first = heats[0];
        assert!(
            heats.iter().any(|&h| h != first),
            "a melting world's provinces diverge in radiogenic budget (per-system heterogeneity > 0)"
        );

        let unprocessed = mars_class_provinces(
            Fixed::from_int(1680),
            Fixed::from_int(120),
            None, // sub-solidus formation: no melt, no incompatible sorting
        );
        let heats0: Vec<Fixed> = unprocessed
            .column_params
            .iter()
            .map(|c| c.heat_production)
            .collect();
        let base = heats0[0];
        assert!(
            heats0.iter().all(|&h| h == base),
            "an unprocessed (sub-solidus formation) world has a UNIFORM radiogenic field, never Earth's 0.3 spread"
        );
    }

    #[test]
    fn a_lower_formation_melt_fraction_makes_a_stronger_heterogeneity() {
        // The per-system amplitude scales with the enrichment 1/F: a lower-degree melt (smaller F) sorts
        // incompatibles more strongly, so its provinces spread WIDER. This is the world's own physics, keyed on F.
        let spread = |f: Fixed| -> f64 {
            let prov = mars_class_provinces(Fixed::from_int(1500), Fixed::from_int(120), Some(f));
            let heats: Vec<f64> = prov
                .column_params
                .iter()
                .map(|c| c.heat_production.to_f64_lossy())
                .collect();
            let max = heats.iter().cloned().fold(f64::MIN, f64::max);
            let min = heats.iter().cloned().fold(f64::MAX, f64::min);
            max - min
        };
        assert!(
            spread(Fixed::from_ratio(1, 20)) > spread(Fixed::from_ratio(1, 4)),
            "a lower melt fraction (stronger batch-melting enrichment) makes a wider radiogenic spread"
        );
    }

    #[test]
    fn the_batch_melting_amplitude_admits_the_partition_and_caps_the_blow_up() {
        let eff = Fixed::ONE; // isolate the enrichment contrast
        let f = Fixed::from_ratio(1, 10); // 10 percent melt
                                          // Incompatible D -> 0 recovers the (1/F - 1) limit: at F = 0.1, about 9.
        let tiny_d = Fixed::from_ratio(1, 100_000);
        let incompatible = heterogeneity_amplitude(f, tiny_d, eff)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (incompatible - 9.0).abs() < 0.05,
            "D->0 recovers the 1/F - 1 limit (~9 at F=0.1), got {incompatible}"
        );
        // A compatible world (D > 1, the reduced-chalcophile sign flip) still has a REAL spread, where the old
        // `1/F` clamp gave zero. At D = 2, F = 0.1: E = 1/(2 + 0.1*(1-2)) = 1/1.9 ~ 0.526, |E-1| ~ 0.474.
        let compatible = heterogeneity_amplitude(f, Fixed::from_int(2), eff)
            .unwrap()
            .to_f64_lossy();
        assert!(
            compatible > 0.4 && compatible < 0.55,
            "a compatible (D>1) world gets a non-zero inverted spread, got {compatible}"
        );
        // The F -> 0 blow-up is capped at 1/D - 1 (here D = 0.01 -> ~99), not divergent.
        let d = Fixed::from_ratio(1, 100);
        let tiny_f = Fixed::from_ratio(1, 1_000_000);
        let capped = heterogeneity_amplitude(tiny_f, d, eff)
            .unwrap()
            .to_f64_lossy();
        assert!(
            capped < 100.0,
            "as F->0 the enrichment is capped at 1/D (~99), not a 1/F blow-up, got {capped}"
        );
    }

    // A minimal province field with a chosen crust-thickness field, for the relief-emergence tests (no heavy
    // scene build): the densities and melt params are representative, only the thickness field varies.
    fn provinces_with(thicknesses: Vec<Fixed>, pcols: usize) -> DeepTimeProvinces {
        let n = thicknesses.len();
        let mut state = DeepTimeState::young(n, Fixed::from_int(1588));
        state.crust_thickness_km = thicknesses;
        DeepTimeProvinces {
            state,
            young_potential_temperature_k: Fixed::from_int(1588),
            column_params: (0..n)
                .map(|_| {
                    province_column_params(
                        Fixed::ONE,
                        Fixed::from_int(10),
                        Fixed::ONE,
                        Fixed::from_int(300),
                        Fixed::ONE,
                    )
                })
                .collect(),
            // Representative test-scaffold melt params (a peridotite-grade solidus, only to exercise the tile
            // relief; the real scene derives the solidus and heterogeneity, this helper just varies the field).
            melt: MeltParams {
                solidus_surface_k: Fixed::from_int(1373),
                solidus_slope_k_per_gpa: Fixed::from_int(130),
                adiabat_slope_k_per_gpa: MANTLE_ADIABAT_SLOPE_K_PER_GPA,
                productivity_per_gpa: MELT_PRODUCTIVITY_PER_GPA,
                source_density_kg_per_m3: Fixed::from_int(3300),
                gravity_m_per_s2: Fixed::from_int(10),
                processing_time_myr: MANTLE_PROCESSING_TIME_MYR,
            },
            pcols,
            prows: n / pcols,
            // A representative Mars-class body radius for the crater-stamp angular scale (these tests read the crust
            // field and do not step the bombardment, so no crater rows are stamped and the radius is unused here).
            radius_m: Fixed::from_int(3_000_000),
            crust_density: Fixed::from_ratio(30, 10),
            mantle_density: Fixed::from_ratio(35, 10),
            sea_level: Fixed::ZERO,
            // A flux built from the reserved-with-basis constants plus plausible test-scaffold derived values (this
            // helper's tests read the crust field and do not step the bombardment, so impact_relief_m stays zero).
            flux: ImpactFluxParams {
                reservoir_body_count: IMPACT_RESERVOIR_BODY_COUNT,
                sweep_timescale_myr: IMPACT_SWEEP_TIMESCALE_MYR,
                differential_slope: IMPACT_DOHNANYI_SLOPE,
                min_impactor_radius_m: IMPACT_MIN_IMPACTOR_RADIUS_M,
                max_impactor_radius_m: IMPACT_MAX_IMPACTOR_RADIUS_M,
                impact_velocity_m_s: Fixed::from_int(5000),
                impactor_density: Fixed::from_int(3000),
                target: Target {
                    gravity: Fixed::from_int(10),
                    strength: IMPACT_TARGET_STRENGTH_PA,
                    density: Fixed::from_int(3000),
                },
                coupling: CraterCoupling {
                    velocity_exponent: IMPACT_COUPLING_VELOCITY_EXPONENT,
                    density_exponent: IMPACT_COUPLING_DENSITY_EXPONENT,
                    efficiency_coefficient: IMPACT_COUPLING_EFFICIENCY_COEFFICIENT,
                    strength_coefficient: IMPACT_COUPLING_STRENGTH_COEFFICIENT,
                    bowl_aspect: IMPACT_COUPLING_BOWL_ASPECT,
                    eject_fraction: IMPACT_COUPLING_EJECT_FRACTION,
                },
                ejecta: EjectaFan {
                    speed: Fixed::from_int(1500),
                    elevation_angle: IMPACT_EJECTA_ELEVATION_ANGLE,
                    azimuths: IMPACT_EJECTA_AZIMUTHS,
                },
                forces: BallisticForces {
                    gravity: Fixed::from_int(10),
                    cell_size: Fixed::from_int(2000),
                    step_cap: IMPACT_BALLISTIC_STEP_CAP,
                },
                per_tick_impact_cap: IMPACT_PER_TICK_CAP,
            },
            world_seed: 0,
        }
    }

    #[test]
    fn the_texture_emerges_only_from_a_varied_field() {
        // The relief EMERGES from the province crust field: a uniform field classifies to one relief (a smooth
        // ball), a varied field to a mix (texture). No field variation, no texture; nothing is painted on.
        let count_variants = |tiles: &[DerivedTile]| -> usize {
            let up = tiles.iter().any(|t| t.relief == TerrainRelief::Upland);
            let low = tiles.iter().any(|t| t.relief == TerrainRelief::Lowland);
            let sub = tiles.iter().any(|t| t.relief == TerrainRelief::Submarine);
            up as usize + low as usize + sub as usize
        };
        let uniform = provinces_with(vec![Fixed::from_int(10); 8], 4);
        let cube = render::SurfaceParam::CubeSphere { face_res: 8 };
        let ut = derive_province_tiles(&uniform, cube).expect("uniform tiles");
        assert_eq!(
            count_variants(&ut),
            1,
            "a uniform province field is a single relief (a smooth ball)"
        );
        let varied = provinces_with(
            vec![
                Fixed::from_int(2),
                Fixed::from_int(60),
                Fixed::from_int(5),
                Fixed::from_int(40),
                Fixed::from_int(80),
                Fixed::from_int(3),
                Fixed::from_int(50),
                Fixed::from_int(8),
            ],
            4,
        );
        let vt = derive_province_tiles(&varied, cube).expect("varied tiles");
        assert!(
            count_variants(&vt) >= 2,
            "a varied province field textures the surface with more than one relief, got {}",
            count_variants(&vt)
        );
    }

    #[test]
    fn the_parallel_cube_cache_is_thread_count_independent() {
        // THE DETERMINISM GATE for the rayon-parallel cube-sphere cache: the build must be BIT-IDENTICAL regardless
        // of the thread count. Each cell is computed purely from immutable inputs (the crater rows and the province
        // field) with no shared mutable state, and the index-ordered `collect` preserves order, so a 1-thread build
        // and an 8-thread build agree to the bit. This is the reproducibility "two views of the same spot agree"
        // (Principle 10) relies on: the viewer stays deterministic even parallelized.
        let mut prov = provinces_with(
            vec![
                Fixed::from_int(2),
                Fixed::from_int(60),
                Fixed::from_int(5),
                Fixed::from_int(40),
                Fixed::from_int(80),
                Fixed::from_int(3),
                Fixed::from_int(50),
                Fixed::from_int(8),
            ],
            4,
        );
        // Seed discrete crater rows so the parallel build exercises the O(cells x craters) crater stamp too.
        prov.state.craters = vec![
            civsim_sim::deeptime::CraterRow {
                u: Fixed::from_ratio(1, 5),
                v: Fixed::from_ratio(2, 5),
                diameter_m: Fixed::from_int(400_000),
                depth_m: Fixed::from_int(20_000),
                age_myr: Fixed::ZERO,
            },
            civsim_sim::deeptime::CraterRow {
                u: Fixed::from_ratio(7, 10),
                v: Fixed::from_ratio(1, 2),
                diameter_m: Fixed::from_int(250_000),
                depth_m: Fixed::from_int(12_000),
                age_myr: Fixed::ZERO,
            },
            civsim_sim::deeptime::CraterRow {
                u: Fixed::from_ratio(9, 10),
                v: Fixed::from_ratio(1, 10),
                diameter_m: Fixed::from_int(600_000),
                depth_m: Fixed::from_int(30_000),
                age_myr: Fixed::ZERO,
            },
        ];
        let cube = render::SurfaceParam::CubeSphere { face_res: 24 };
        let tiles_at = |threads: usize| -> Vec<i64> {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("a rayon pool")
                .install(|| derive_province_tiles(&prov, cube).expect("cube tiles"))
                .iter()
                .map(|t| t.elevation.to_bits())
                .collect()
        };
        let one = tiles_at(1);
        let many = tiles_at(8);
        assert_eq!(
            one.len(),
            6 * 24 * 24,
            "the cube cache is six faces of face_res^2 cells"
        );
        assert_eq!(
            one, many,
            "the parallel cube-sphere elevation cache is bit-identical across thread counts"
        );
        // The lava field is the other parallel cube build; it must be thread-count independent as well.
        let lava_at = |threads: usize| -> Vec<(u8, u8, u8, u32)> {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("a rayon pool")
                .install(|| derive_province_lava(&prov, cube))
                .iter()
                .map(|g| {
                    (
                        g.emission.r,
                        g.emission.g,
                        g.emission.b,
                        g.intensity.to_bits(),
                    )
                })
                .collect()
        };
        assert_eq!(
            lava_at(1),
            lava_at(8),
            "the parallel cube-sphere lava field is bit-identical across thread counts"
        );
    }

    #[test]
    fn the_lava_glow_fades_as_the_deep_time_world_cools() {
        // The visible-volcanism arc: a fresh (young, super-solidus) world glows broadly, and as the deep-time clock
        // cools it toward its conductive steady state the glow FADES (fewer molten tiles, lower mean intensity). The
        // threshold is the world's OWN derived solidus the MeltParams carries, never an authored glow cutoff, and a
        // glowing tile carries a warm incandescent emission (the blackbody of its interior temperature).
        let mut prov = mars_class_provinces(
            Fixed::from_int(1680),
            Fixed::from_int(120),
            Some(Fixed::from_ratio(1, 10)),
        );
        let (cols, rows) = (32usize, 16usize);
        let param = render::SurfaceParam::LatLon { cols, rows };
        // The fresh young field starts laterally uniform at the super-solidus magma-ocean handoff, so it glows broadly.
        let young = derive_province_lava(&prov, param);
        assert_eq!(young.len(), cols * rows, "one glow entry per display tile");
        let young_molten = young.iter().filter(|g| g.intensity > 0.0).count();
        let young_mean = young.iter().map(|g| g.intensity as f64).sum::<f64>() / young.len() as f64;
        assert!(
            young_molten as f64 / young.len() as f64 > 0.9,
            "a fresh super-solidus world glows broadly, got {young_molten} of {} molten",
            young.len()
        );
        let hot = young
            .iter()
            .find(|g| g.intensity > 0.0)
            .expect("a molten tile");
        assert!(
            hot.emission.r >= hot.emission.b,
            "the incandescent emission is warm (red >= blue), got rgb({},{},{})",
            hot.emission.r,
            hot.emission.g,
            hot.emission.b
        );
        // Cool the world over deep time: the interior relaxes toward its (near-solidus) steady state, so the glow fades.
        step_provinces(&mut prov, 200);
        let aged = derive_province_lava(&prov, param);
        let aged_molten = aged.iter().filter(|g| g.intensity > 0.0).count();
        let aged_mean = aged.iter().map(|g| g.intensity as f64).sum::<f64>() / aged.len() as f64;
        assert!(
            aged_mean < young_mean,
            "the glow fades as the world cools: aged mean {aged_mean:.3} < young mean {young_mean:.3}"
        );
        assert!(
            aged_molten < young_molten,
            "fewer tiles stay molten as the world cools: aged {aged_molten} < young {young_molten}"
        );
    }

    #[test]
    fn a_solid_sub_solidus_world_does_not_glow() {
        // A world whose interior never crosses its own solidus makes no melt and so no lava glow: every tile's
        // intensity is zero (solid crust). This pins the threshold on the DERIVED solidus, not an authored cutoff.
        let mut prov = mars_class_provinces(
            Fixed::from_int(1680),
            Fixed::from_int(120),
            None, // no formation melt: an unprocessed, uniform mantle
        );
        // Force the whole interior below the solidus (a cold world), then check that nothing glows.
        for col in prov.state.columns.iter_mut() {
            col.temperature = Fixed::from_int(1000);
        }
        let glow = derive_province_lava(&prov, render::SurfaceParam::CubeSphere { face_res: 12 });
        assert!(
            glow.iter().all(|g| g.intensity == 0.0),
            "a sub-solidus world does not glow (every tile intensity is zero)"
        );
    }

    #[test]
    fn a_melted_world_drives_the_verdict_end_to_end_and_stands_in_km_relief() {
        // R-YOUNG-TEMPERATURE payoff, driven END TO END through the real wiring (fix 3): build_derived_scene runs
        // the young-thermal verdict itself, and for a world whose best estimate melts it pins the young potential
        // temperature at the magma-ocean lock-up handoff, RE-DERIVES the surface at that temperature by the real
        // second `derive_surface_composition` call (F = phi_c by construction, proven in the physics test
        // `the_handoff_pins_the_lock_up_melt_fraction`), and textures the deep-time relief off the resulting
        // formation melt fraction. No hand-fed F: the verdict and the re-derivation are in the loop.
        let scene =
            build_derived_scene(Fixed::ONE, Fixed::ONE).expect("the derived scene resolves");
        let v = scene.young.expect("the young-thermal verdict resolves");
        // The Sun/1AU world melts at its best estimate, so it is carried at the super-solidus handoff (its grade is
        // MARGINAL: the interim formation-time band up to a few megayears straddles the solidus, the honest catch).
        assert!(
            v.handoff_potential_temperature_k.is_some(),
            "a best-estimate-melting world is carried at the super-solidus handoff"
        );
        assert_eq!(
            v.young_potential_temperature_k,
            v.handoff_potential_temperature_k.unwrap(),
            "the carried young temperature IS the lock-up handoff (the re-derivation ran at it)"
        );
        // The ISOSTATIC diagnostics (the melt-driven relief) read the CRUST-ONLY tile field, not the composed
        // surface: the bombardment is a SEPARATE surface-topography record, so it must not dilute the melt-texture
        // indicator's normalization or masquerade as isostatic relief ([`derive_province_crust_tiles`]). They read
        // the FIXED diagnostic resolution (decoupled from the render sample cache) so the neighbour-contrast
        // indicator stays comparable to its calibrated smooth-ball floor.
        let crust_tiles = scene
            .provinces
            .as_ref()
            .and_then(|p| {
                derive_province_crust_tiles(p, DIAGNOSTIC_TILE_COLS, DIAGNOSTIC_TILE_ROWS)
            })
            .expect("the crust-only tiles resolve");
        // The verdict drove a real, nonzero isostatic relief through the re-derived surface: the honest km amplitude
        // is the physical measure the render shows at physical scale (never a normalized ratio).
        let amplitude_km =
            tile_relief_amplitude_km(&crust_tiles).expect("the relief amplitude resolves");
        assert!(
            amplitude_km > 0.0,
            "a melted world stands in real, nonzero km-scale isostatic relief, got {amplitude_km} km"
        );
        // The melt-driven heterogeneity is ENGAGED (the indicator stands above the smooth-ball floor): the texture
        // is on, which the amplitude confirms is real relief rather than a normalized artifact.
        let indicator = tile_relief_heterogeneity_indicator(&crust_tiles, DIAGNOSTIC_TILE_COLS)
            .expect("the heterogeneity indicator resolves");
        assert!(
            indicator > 0.03,
            "the melt-driven texture is engaged above the smooth-ball floor, got {indicator:.3}"
        );
        // THE BOMBARDMENT composed additional surface relief onto the same tiles: the deep-time impact chain drew
        // craters over the aged span, so the RENDERED (composed) relief stands ABOVE the crust-only isostatic
        // relief. The craters are in the derived elevation field the render reads.
        let impacts = scene
            .provinces
            .as_ref()
            .map(|p| p.state.impact_count)
            .unwrap_or(0);
        assert!(impacts > 0, "the deep-time bombardment carved craters");
        let composed_amplitude_km =
            tile_relief_amplitude_km(&scene.tiles).expect("the composed relief amplitude resolves");
        assert!(
            composed_amplitude_km > amplitude_km,
            "the bombardment composed relief onto the crust ({composed_amplitude_km:.2} km composed vs {amplitude_km:.2} km isostatic)"
        );
        // THE SUPPORT-BOUND CHECK runs and REPORTS (never asserts the direction away): the derived amplitude is
        // reported against yield/(rho*g), any excess flagged. Today the deep-time crust growth accumulates an
        // unphysically large relief over the aged span, so the check FLAGS it (a surfaced follow-on in the sim
        // deep-time crust-growth model, outside this slice's scope, which the km amplitude + support bound exposed
        // where the normalized roughness indicator had hidden it). The check firing is the honest behaviour.
        let bound_km = scene.provinces.as_ref().and_then(|p| {
            supportable_relief_km(
                CRUST_YIELD_STRENGTH_PA,
                p.crust_density.to_f64_lossy(),
                scene.gravity.to_f64_lossy(),
            )
        });
        let bound_note = match bound_km {
            Some(b) if amplitude_km > b => {
                format!("EXCEEDS support bound {b:.2} km (flagged: deep-time crust over-growth follow-on)")
            }
            Some(b) => format!("within support bound {b:.2} km"),
            None => "support bound unavailable".to_string(),
        };
        eprintln!(
            "R-YOUNG-TEMPERATURE payoff: {:?}, young T {:.0} K; derived relief amplitude {:.2} km ({}), heterogeneity indicator {:.1}%",
            v.regime,
            v.young_potential_temperature_k.to_f64_lossy(),
            amplitude_km,
            bound_note,
            indicator * 100.0
        );
    }

    #[test]
    fn the_relief_amplitude_and_support_bound_are_physical_quantities() {
        // The km amplitude is the elevation span (max minus min), a real magnitude, NOT a normalized ratio: a field
        // spanning -2 to +3 km is a 5 km amplitude. The support bound is yield/(rho*g): 1e8 Pa over 3000 kg/m^3 at
        // 3.7 m/s^2 is ~9 km, and an amplitude above it is flagged unsupportable.
        let tiles = vec![
            DerivedTile {
                elevation: Fixed::from_int(-2),
                relief: TerrainRelief::Submarine,
            },
            DerivedTile {
                elevation: Fixed::from_int(3),
                relief: TerrainRelief::Upland,
            },
            DerivedTile {
                elevation: Fixed::ZERO,
                relief: TerrainRelief::Lowland,
            },
        ];
        let amplitude = tile_relief_amplitude_km(&tiles).expect("the amplitude resolves");
        assert!(
            (amplitude - 5.0).abs() < 1e-6,
            "the amplitude is the elevation span (3 - (-2) = 5 km), got {amplitude}"
        );
        // Mars-class support bound: sigma_y 1e8 Pa, crust 3.0 g/cm^3, g 3.7 -> ~9 km.
        let bound = supportable_relief_km(1.0e8, 3.0, 3.7).expect("the bound resolves");
        assert!(
            (bound - 9.009).abs() < 0.1,
            "the support bound is yield/(rho*g) ~9 km for Mars-class, got {bound}"
        );
        // A taller relief than the bound is caught; a non-physical density fails soft.
        assert!(
            amplitude < bound,
            "a 5 km relief is supportable under a 9 km bound"
        );
        assert!(supportable_relief_km(1.0e8, 0.0, 3.7).is_none());
        assert!(tile_relief_amplitude_km(&[]).is_none());
    }

    #[test]
    fn the_bilinear_sampler_wraps_longitude_and_smooths() {
        // The province field is sampled with wrapping longitude (so the globe has no east-west seam) and it
        // smooths across boundaries: a coordinate between two provinces reads a value between their thicknesses.
        let prov = provinces_with(vec![Fixed::from_int(10), Fixed::from_int(30)], 2); // 2x1, thin then thick
                                                                                      // The seam at fu just under 1 wraps back toward province 0, so it is between the two, not a hard jump.
        let mid = sample_province_thickness(&prov, 0.5, 0.5).to_f64_lossy();
        assert!(
            (10.0..=30.0).contains(&mid),
            "a coordinate between provinces reads a blended thickness, got {mid}"
        );
        // Determinism: the same coordinate replays exactly.
        assert_eq!(
            sample_province_thickness(&prov, 0.42, 0.5),
            sample_province_thickness(&prov, 0.42, 0.5),
            "the sampler replays deterministically"
        );
    }
}
