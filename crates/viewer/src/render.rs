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

//! The superfine render: the zoom level where a tile is large enough to show the individual
//! organisms standing on it (design Parts 14, 1; R-VIEW-ELAB done as a pure read of canon).
//!
//! At the overview levels the viewer draws the biome quadtree; here, once the camera has
//! zoomed past per-tile, each tile is painted as a block of its biome colour with the located
//! organisms drawn on it as marks, coloured by trophic layer (plants green, herbivores amber,
//! carnivores red) and individualised per species. This reads the [`LivingWorld`]'s tile map
//! and located occupants and never writes them, so the superfine view is an observer of the
//! world, not an author of it (Principle 10).

use civsim_core::{splitmix64, Fixed};
use civsim_sim::genesis::LivingWorld;
use civsim_world::{BiomeSet, Coord3, Rgb, TopologySpace};

/// The colour of an organism mark: a base hue by trophic layer, jittered per species so each
/// kind is a distinct individual form. Presentation only, never canonical state.
pub fn organism_color(layer: u16, species_id: u32) -> Rgb {
    let (br, bg, bb) = match layer {
        0 => (46, 176, 74),  // producers: the plants, green
        1 => (214, 176, 58), // first consumers: herbivores, amber
        _ => (206, 74, 58),  // higher consumers: carnivores, red
    };
    let h = splitmix64(species_id as u64 ^ 0x9E37_79B9_7F4A_7C15);
    let jitter = |base: i32, shift: u32| -> u8 {
        let d = ((h >> shift) & 0x3f) as i32 - 32; // -32..31
        (base + d).clamp(0, 255) as u8
    };
    Rgb::new(jitter(br, 0), jitter(bg, 8), jitter(bb, 16))
}

/// A physics-derived terrain colour: the tile's own `elevation`, `moisture`, and `temperature`
/// fields (the physical quantities worldgen computed, each in `[0, 1]`) mapped to colour, so terrain
/// looks like its physics rather than an authored biome swatch. Presentation only, a pure read of
/// canon (Principle 10). This is the first slice of the visual-projection substrate: the palette
/// anchors below (sea level, the tan/green/snow/ochre/rock endpoints) are the tunable projection, an
/// aesthetic call the owner reserves, not physics.
pub fn physics_terrain_color(elevation: Fixed, moisture: Fixed, temperature: Fixed) -> Rgb {
    fn unit_to_255(f: Fixed) -> i32 {
        f.checked_mul(Fixed::from_int(255))
            .map(|v| v.to_int())
            .unwrap_or(0)
            .clamp(0, 255)
    }
    fn mix(a: i32, b: i32, num: i32, den: i32) -> i32 {
        a + (b - a) * num.clamp(0, den) / den.max(1)
    }
    let (e, m, t) = (
        unit_to_255(elevation),
        unit_to_255(moisture),
        unit_to_255(temperature),
    );
    const SEA: i32 = 77; // elevation 0.30: below it the cell is water
    if e < SEA {
        // Water: teal at the warm shallows deepening to cold abyssal blue.
        let d = SEA - e;
        return Rgb::new(
            mix(22, 6, d, SEA) as u8,
            mix(104, 44, d, SEA) as u8,
            mix(176, 92, d, SEA) as u8,
        );
    }
    // Land base: dry tan to wet green by moisture.
    let mut r = mix(196, 58, m, 255);
    let mut g = mix(176, 132, m, 255);
    let mut b = mix(120, 58, m, 255);
    // Cold tints toward snow; heat tints toward arid ochre.
    if t < SEA {
        let c = SEA - t;
        r = mix(r, 236, c, SEA);
        g = mix(g, 240, c, SEA);
        b = mix(b, 246, c, SEA);
    } else if t > 255 - SEA {
        let hh = t - (255 - SEA);
        r = mix(r, 206, hh, SEA);
        g = mix(g, 150, hh, SEA);
        b = mix(b, 92, hh, SEA);
    }
    // High ground lightens toward rock, quadratic so lowlands keep their colour.
    let land = e - SEA;
    let hl = (land * land) / (255 - SEA);
    r = mix(r, 206, hl, 255 - SEA);
    g = mix(g, 210, hl, 255 - SEA);
    b = mix(b, 214, hl, 255 - SEA);
    Rgb::new(
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
    )
}

/// Blend colour `a` toward `b` by `num/den`. Presentation only.
fn blend(a: Rgb, b: Rgb, num: i32, den: i32) -> Rgb {
    let mix = |x: u8, y: u8| -> u8 {
        (x as i32 + (y as i32 - x as i32) * num.clamp(0, den) / den.max(1)).clamp(0, 255) as u8
    };
    Rgb::new(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b))
}

#[inline]
fn fill_rect(buf: &mut [u32], w: usize, x0: usize, y0: usize, rw: usize, rh: usize, color: u32) {
    for y in y0..(y0 + rh).min(buf.len() / w.max(1)) {
        let row = y * w;
        for x in x0..(x0 + rw).min(w) {
            buf[row + x] = color;
        }
    }
}

/// The 5x7 rows of a glyph (low 5 bits per row, leftmost column is bit 4). A compact bitmap
/// font covering the characters the selector readout uses; lowercase maps to uppercase.
fn glyph_rows(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        ' ' => [0; 7],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '#' => [
            0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010,
        ],
        '(' => [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ],
        ')' => [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ],
        ',' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100, 0b01000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ':' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '|' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        _ => [
            0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ], // '?'
    }
}

/// Draw a text label with a filled backing panel, so the cursor readout is legible on the map
/// (the on-canvas names of what the selector points at). `scale` is pixels per font pixel.
/// The panel is clamped to stay on screen.
#[allow(clippy::too_many_arguments)]
pub fn draw_label(
    buf: &mut [u32],
    w: usize,
    h: usize,
    x: i32,
    y: i32,
    text: &str,
    scale: usize,
    fg: Rgb,
    bg: Rgb,
) {
    let scale = scale.max(1);
    let cw = (5 + 1) * scale; // glyph width plus one-column gap
    let pad = scale * 2;
    let panel_w = text.chars().count() * cw + pad * 2;
    let panel_h = 7 * scale + pad * 2;
    // Clamp the panel onto the screen.
    let px = x.clamp(0, (w as i32 - panel_w as i32).max(0)) as usize;
    let py = y.clamp(0, (h as i32 - panel_h as i32).max(0)) as usize;
    fill_rect(buf, w, px, py, panel_w, panel_h, bg.pack());
    let fgp = fg.pack();
    let mut cx = px + pad;
    let ty = py + pad;
    for ch in text.chars() {
        let rows = glyph_rows(ch);
        for (r, bits) in rows.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    fill_rect(buf, w, cx + col * scale, ty + r * scale, scale, scale, fgp);
                }
            }
        }
        cx += cw;
    }
}

/// Draw a one-pixel outline around a cell rectangle, the cursor the tile selector uses to
/// indicate the hovered cell. Presentation only.
pub fn draw_outline(
    buf: &mut [u32],
    w: usize,
    x0: usize,
    y0: usize,
    rw: usize,
    rh: usize,
    color: Rgb,
) {
    let h = buf.len() / w.max(1);
    let c = color.pack();
    let x1 = (x0 + rw).min(w);
    let y1 = (y0 + rh).min(h);
    if x0 >= w || y0 >= h || x1 == 0 || y1 == 0 {
        return;
    }
    for x in x0..x1 {
        buf[y0 * w + x] = c;
        buf[(y1 - 1) * w + x] = c;
    }
    for y in y0..y1 {
        buf[y * w + x0] = c;
        buf[y * w + (x1 - 1)] = c;
    }
}

/// Paint the superfine view centred on `center`, each tile drawn as a `tile_px` square: its
/// biome colour, then the organisms on it as centred marks. Returns a `w` by `h` RGB buffer.
pub fn superfine(
    living: &LivingWorld,
    biomes: &BiomeSet,
    center: Coord3,
    tile_px: usize,
    w: usize,
    h: usize,
    bg: Rgb,
) -> Vec<u32> {
    let tile_px = tile_px.max(3);
    let cols = (w / tile_px).max(1) as i32;
    let rows = (h / tile_px).max(1) as i32;
    let ox = center.x - cols / 2;
    let oy = center.y - rows / 2;
    let topo = living.map.topo();
    let mut buf = vec![bg.pack(); w * h];

    for r in 0..rows {
        for c in 0..cols {
            let coord = Coord3::ground(ox + c, oy + r);
            let px0 = c as usize * tile_px;
            let py0 = r as usize * tile_px;
            // Physics-derived terrain colour (the tile's own elevation, moisture, and temperature),
            // with a light accent of the biome swatch for identity, or the empty-space colour off
            // the map. Terrain looks like its physics rather than an authored swatch.
            let tile_color = if topo.contains(coord) {
                living
                    .map
                    .tile(coord)
                    .map(|t| {
                        let physics =
                            physics_terrain_color(t.elevation(), t.moisture(), t.temperature());
                        blend(physics, biomes.color(t.biome), 46, 255) // about 18% biome accent
                    })
                    .unwrap_or(bg)
            } else {
                bg
            };
            fill_rect(&mut buf, w, px0, py0, tile_px, tile_px, tile_color.pack());

            // The organisms on this tile, drawn as marks sized by body mass and coloured by
            // kind, so anatomy shows: a big carnivore is a large red mark, a small plant a
            // small green one.
            let occ = living.occupants.occupants(coord);
            if occ.is_empty() {
                continue;
            }
            for (i, o) in occ.iter().enumerate().take(4) {
                let info = living.occupant_info.get(o);
                let color = info
                    .map(|inf| organism_color(inf.layer, inf.species.0))
                    .unwrap_or(Rgb::new(240, 240, 240));
                // Mark size scales with body mass: a quarter-tile at the smallest up to about
                // eight-tenths of a tile at the largest (integer, via the Fixed body-mass value).
                let bm = info
                    .map(|inf| inf.body_mass)
                    .unwrap_or(Fixed::from_ratio(1, 2));
                let span = Fixed::from_int((tile_px * 3 / 5) as i32);
                let extra = bm
                    .checked_mul(span)
                    .map(|v| v.to_int().max(0) as usize)
                    .unwrap_or(0);
                let mark = (tile_px / 4 + extra).clamp(2, tile_px);
                // Centre a lone occupant; nudge several so they stay distinct.
                let base = (tile_px.saturating_sub(mark)) / 2;
                let nudge = tile_px / 6;
                let (ox, oy) = match i {
                    0 => (base, base),
                    1 => (base.saturating_sub(nudge), base.saturating_sub(nudge)),
                    2 => (base + nudge, base + nudge),
                    _ => (base + nudge, base.saturating_sub(nudge)),
                };
                fill_rect(&mut buf, w, px0 + ox, py0 + oy, mark, mark, color.pack());
            }
        }
    }
    buf
}

// ---------------------------------------------------------------------------
// The outer, powers-of-ten zoom (design Part 14): past the whole surface, the planet as a
// single disc, then the solar system. Two level-of-detail tiers beyond the cell surface, chosen
// by the on-screen zoom scale, drawn as pure reads of canon (Principle 10). The individual cells
// stop being worth drawing once they fall below a pixel each, so at that scale the surface is
// shown as the body it is (a globe), and past that the planet is small enough that its orbit
// around its star is worth revealing.

/// Which level-of-detail tier a zoom scale falls in, the whole of the outer-zoom tier decision.
/// `px_per_cell` is the on-screen pixels per world cell of the whole-surface projection: at one
/// pixel per cell or more the cells are worth drawing (the surface tiers, unchanged); below one
/// pixel per cell the cells are sub-pixel and alias, so the planet is drawn as one disc; below a
/// further threshold the planet is small enough on screen that its solar system is drawn around
/// it. A pure, testable mapping.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lod {
    /// The cell surface (the existing overview and superfine reads); cells are at least a pixel.
    Surface,
    /// The whole planet as a single downsampled disc; cells are sub-pixel.
    Planet,
    /// The solar-system frame: the sun, the orbit line, and the planet as a dot on it.
    SolarSystem,
}

/// At or above this many on-screen pixels per world cell the cells are worth drawing; below it
/// they are sub-pixel, so the planet is drawn as a disc. A view-side legibility threshold, an
/// aesthetic call, not physics.
pub const PLANET_SCALE: f64 = 1.0;
/// Below this scale the planet is small enough on screen that its solar system is drawn around it
/// (the sun, the orbit, and the planet as a dot). A view-side legibility threshold.
pub const SOLAR_SCALE: f64 = 0.1;

/// The tier for a whole-surface zoom scale (pixels per world cell). Coarsens monotonically as the
/// scale falls: surface, then planet disc, then solar system. Presentation only.
pub fn lod_for_scale(px_per_cell: f64) -> Lod {
    if px_per_cell >= PLANET_SCALE {
        Lod::Surface
    } else if px_per_cell >= SOLAR_SCALE {
        Lod::Planet
    } else {
        Lod::SolarSystem
    }
}

/// Where the planet sits on its orbit: the angle in radians, measured from the +x axis, given the
/// elapsed time and the orbital period in the SAME units (the plain circular law `2*pi*(t mod T)/T`).
/// This is the whole of the orbital-position model for the first version: a circle.
///
/// THE ORBITAL-ELEMENTS UPGRADE SEAM. The full orbital elements (semi-major axis, eccentricity,
/// obliquity) are being built on a separate branch and will replace this circular approximation.
/// When they land, THIS is the one function to change: return an `(x, y)` offset on the ellipse
/// (Kepler's equation for the eccentric anomaly) instead of an angle on a circle, and the callers
/// place the dot at that offset rather than at `(r cos a, r sin a)`. Pure, float-only, a display
/// value that never enters canonical state (Principle 10).
pub fn planet_orbit_angle(elapsed_seconds: f64, period_seconds: f64) -> f64 {
    // A non-positive or non-finite period, or a non-finite elapsed time, has no orbital phase; the
    // positive-form guard also rejects NaN (a NaN comparison is false), so no divide by zero.
    if period_seconds > 0.0 && elapsed_seconds.is_finite() {
        let phase = elapsed_seconds.rem_euclid(period_seconds) / period_seconds;
        phase * std::f64::consts::TAU
    } else {
        0.0
    }
}

/// The planet's disc colour: the downsample (average) of the physics-derived surface colours over
/// every on-world tile, so the globe reads as an average of its own terrain rather than an authored
/// swatch. A pure read of canon (Principle 10); the terrain is fixed, so a caller computes this
/// once. Off-world tiles are skipped; an empty world falls back to a neutral ocean blue.
pub fn average_terrain_color(living: &LivingWorld) -> Rgb {
    let topo = living.map.topo();
    let (mut sr, mut sg, mut sb, mut n): (u64, u64, u64, u64) = (0, 0, 0, 0);
    for y in 0..topo.height {
        for x in 0..topo.width {
            if let Some(t) = living.map.tile(Coord3::ground(x, y)) {
                let c = physics_terrain_color(t.elevation(), t.moisture(), t.temperature());
                sr += c.r as u64;
                sg += c.g as u64;
                sb += c.b as u64;
                n += 1;
            }
        }
    }
    if n == 0 {
        return Rgb::new(46, 92, 140);
    }
    Rgb::new((sr / n) as u8, (sg / n) as u8, (sb / n) as u8)
}

/// Fill a shaded sphere of radius `r` centred at `(cx, cy)` in `color`, lit from the upper left so
/// it reads as a globe rather than a flat coin (a Lambert term over the sphere normal). The maths is
/// float, a display value only (Principle 10). Presentation only.
fn fill_globe(buf: &mut [u32], w: usize, h: usize, cx: i32, cy: i32, r: i32, color: Rgb) {
    if r <= 0 {
        return;
    }
    let rf = r as f64;
    // A fixed light direction (upper-left, tilted toward the viewer), normalised.
    let (lx, ly, lz) = {
        let (a, b, c) = (-0.5_f64, -0.5, 0.7);
        let inv = 1.0 / (a * a + b * b + c * c).sqrt();
        (a * inv, b * inv, c * inv)
    };
    for dy in -r..=r {
        let y = cy + dy;
        if y < 0 || y as usize >= h {
            continue;
        }
        for dx in -r..=r {
            let x = cx + dx;
            if x < 0 || x as usize >= w {
                continue;
            }
            let (nx, ny) = (dx as f64 / rf, dy as f64 / rf);
            let d2 = nx * nx + ny * ny;
            if d2 > 1.0 {
                continue;
            }
            let nz = (1.0 - d2).sqrt();
            let lambert = (nx * lx + ny * ly + nz * lz).clamp(0.18, 1.0);
            let shade = |ch: u8| (ch as f64 * lambert).clamp(0.0, 255.0) as u8;
            buf[y as usize * w + x as usize] =
                Rgb::new(shade(color.r), shade(color.g), shade(color.b)).pack();
        }
    }
}

/// Fill a flat disc of radius `r` centred at `(cx, cy)` in `color` (a self-luminous body, the star).
/// Presentation only.
fn fill_disk(buf: &mut [u32], w: usize, h: usize, cx: i32, cy: i32, r: i32, color: Rgb) {
    if r <= 0 {
        return;
    }
    let c = color.pack();
    let rr = (r * r) as i64;
    for dy in -r..=r {
        let y = cy + dy;
        if y < 0 || y as usize >= h {
            continue;
        }
        for dx in -r..=r {
            let x = cx + dx;
            if x < 0 || x as usize >= w {
                continue;
            }
            if (dx * dx + dy * dy) as i64 <= rr {
                buf[y as usize * w + x as usize] = c;
            }
        }
    }
}

/// Draw a one-and-a-half-pixel ring of radius `r` centred at `(cx, cy)` (the orbit line). A pixel is
/// on the ring when its distance from the centre is within half the line width of `r`. Presentation
/// only.
fn draw_ring(buf: &mut [u32], w: usize, h: usize, cx: i32, cy: i32, r: i32, color: Rgb) {
    if r <= 0 {
        return;
    }
    let rf = r as f64;
    let c = color.pack();
    for dy in (-r - 1)..=(r + 1) {
        let y = cy + dy;
        if y < 0 || y as usize >= h {
            continue;
        }
        for dx in (-r - 1)..=(r + 1) {
            let x = cx + dx;
            if x < 0 || x as usize >= w {
                continue;
            }
            let d = ((dx * dx + dy * dy) as f64).sqrt();
            if (d - rf).abs() <= 1.0 {
                buf[y as usize * w + x as usize] = c;
            }
        }
    }
}

/// Paint the PLANET tier: the whole planet as one filled disc of `diameter_px`, centred in the
/// window, its colour the downsampled surface average (a small globe, shaded to read as a sphere).
/// Returns a `win_w` by `win_h` RGB buffer. A pure read of canon (Principle 10).
pub fn planet_disk(
    planet_color: Rgb,
    diameter_px: usize,
    win_w: usize,
    win_h: usize,
    bg: Rgb,
) -> Vec<u32> {
    let mut buf = vec![bg.pack(); win_w * win_h];
    let r = (diameter_px / 2).max(1) as i32;
    let cx = (win_w / 2) as i32;
    let cy = (win_h / 2) as i32;
    fill_globe(&mut buf, win_w, win_h, cx, cy, r, planet_color);
    buf
}

/// The window-pixel position of the planet on the drawn orbit, given the frame centre, the orbit
/// radius, and the orbital angle. Shared by the solar-system paint and its label so the dot and its
/// name stay together. When the elliptical elements land ([`planet_orbit_angle`]'s seam), this
/// becomes the ellipse point.
pub fn planet_orbit_xy(cx: i32, cy: i32, orbit_radius: usize, angle: f64) -> (i32, i32) {
    let r = orbit_radius as f64;
    (
        cx + (r * angle.cos()).round() as i32,
        cy + (r * angle.sin()).round() as i32,
    )
}

/// Paint the SOLAR-SYSTEM tier: the sun as a bright disc at the orbital focus (the frame centre),
/// the orbit as a ring, and the planet as a small disc on the orbit at `planet_angle`. The sun and
/// orbit grow from nothing as the view pulls back (the caller passes growing radii), so the reveal
/// out of the planet disc is continuous. Returns a `win_w` by `win_h` RGB buffer, a pure read of
/// canon (Principle 10).
#[allow(clippy::too_many_arguments)]
pub fn solar_system(
    planet_color: Rgb,
    sun_color: Rgb,
    sun_radius: usize,
    orbit_radius: usize,
    planet_radius: usize,
    planet_angle: f64,
    win_w: usize,
    win_h: usize,
    bg: Rgb,
) -> Vec<u32> {
    let mut buf = vec![bg.pack(); win_w * win_h];
    let cx = (win_w / 2) as i32;
    let cy = (win_h / 2) as i32;
    // The orbit line first, so the sun and the planet sit on top of it.
    draw_ring(
        &mut buf,
        win_w,
        win_h,
        cx,
        cy,
        orbit_radius as i32,
        Rgb::new(74, 84, 116),
    );
    // The star at the focus: a bright, self-luminous disc.
    fill_disk(&mut buf, win_w, win_h, cx, cy, sun_radius as i32, sun_color);
    // The planet on its orbit at the current angular position, shaded like the globe it was.
    let (px, py) = planet_orbit_xy(cx, cy, orbit_radius, planet_angle);
    fill_globe(
        &mut buf,
        win_w,
        win_h,
        px,
        py,
        planet_radius.max(1) as i32,
        planet_color,
    );
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_sim::genesis::{genesis, GenesisParams};

    #[test]
    fn organism_colour_is_deterministic_and_layer_keyed() {
        assert_eq!(
            organism_color(0, 7),
            organism_color(0, 7),
            "same inputs, same colour"
        );
        // Plants (layer 0) are greener than carnivores (layer 2): more green, less red.
        let plant = organism_color(0, 1);
        let carnivore = organism_color(2, 1);
        assert!(plant.g > plant.r, "a plant is green-dominant");
        assert!(carnivore.r > carnivore.g, "a carnivore is red-dominant");
    }

    #[test]
    fn superfine_paints_the_requested_size_and_marks_occupants() {
        let mut params = GenesisParams::dev_default();
        params.width = 48;
        params.height = 32;
        let living = genesis(
            0xEA27,
            &params,
            &civsim_sim::environ::AbioticSourceRegistry::earth_dev(),
            None,
        );
        let (w, h, tile_px) = (240usize, 160usize, 18usize);
        // Centre on an occupied tile so at least one organism mark is drawn.
        let center = living
            .occupants
            .occupied()
            .next()
            .expect("an occupied tile");
        let buf = super::superfine(
            &living,
            &BiomeSet::dev_default(),
            center,
            tile_px,
            w,
            h,
            Rgb::new(8, 9, 14),
        );
        assert_eq!(buf.len(), w * h, "one word per pixel");
        assert_eq!(
            buf,
            super::superfine(
                &living,
                &BiomeSet::dev_default(),
                center,
                tile_px,
                w,
                h,
                Rgb::new(8, 9, 14)
            ),
            "a pure read replays"
        );
        // The centre tile's block carries a mark distinct from the background colour.
        assert!(
            buf.iter().any(|&p| p != Rgb::new(8, 9, 14).pack()),
            "something is drawn"
        );
    }
    #[test]
    fn physics_terrain_colour_reflects_the_fields() {
        let p = |n: i64| Fixed::from_ratio(n, 100);
        // A deterministic pure read of the tile's physical fields.
        assert_eq!(
            physics_terrain_color(p(60), p(50), p(50)),
            physics_terrain_color(p(60), p(50), p(50)),
        );
        // Low elevation is water: blue-dominant.
        let water = physics_terrain_color(p(10), p(50), p(50));
        assert!(water.b > water.r && water.b > water.g, "water reads blue");
        // Wet temperate land is green-dominant; drier ground at the same elevation is warmer.
        let meadow = physics_terrain_color(p(50), p(80), p(50));
        assert!(
            meadow.g > meadow.r && meadow.g > meadow.b,
            "wet temperate land reads green"
        );
        let dry = physics_terrain_color(p(50), p(10), p(50));
        assert!(dry.r > meadow.r, "drier ground is warmer than a meadow");
        // A cold high peak lightens toward snow and rock.
        let peak = physics_terrain_color(p(95), p(40), p(10));
        assert!(
            peak.r > 180 && peak.g > 180 && peak.b > 180,
            "a cold peak reads pale"
        );
    }

    #[test]
    fn lod_tiers_by_zoom_scale() {
        // Cells worth drawing at a pixel or more per cell; below that, the planet disc; below a
        // further threshold, the solar system. The boundaries are inclusive on the coarse side.
        assert_eq!(lod_for_scale(4.0), Lod::Surface);
        assert_eq!(
            lod_for_scale(PLANET_SCALE),
            Lod::Surface,
            "1.0 is still surface"
        );
        assert_eq!(lod_for_scale(0.5), Lod::Planet);
        assert_eq!(
            lod_for_scale(SOLAR_SCALE),
            Lod::Planet,
            "0.1 is still the disc"
        );
        assert_eq!(lod_for_scale(0.05), Lod::SolarSystem);
        assert_eq!(lod_for_scale(0.001), Lod::SolarSystem);
        // As the on-screen scale falls, the tier only ever coarsens (never sharpens back).
        let order = |l: Lod| match l {
            Lod::Surface => 0,
            Lod::Planet => 1,
            Lod::SolarSystem => 2,
        };
        let mut prev = 0;
        let mut s = 8.0_f64;
        while s > 0.0005 {
            let t = order(lod_for_scale(s));
            assert!(
                t >= prev,
                "the tier coarsens monotonically as the scale falls"
            );
            prev = t;
            s *= 0.5;
        }
    }

    #[test]
    fn orbit_angle_advances_with_time() {
        use std::f64::consts::{PI, TAU};
        // One Earth-year of world-seconds, the dev-earth orbital period.
        let p = 31_536_000.0_f64;
        assert_eq!(
            planet_orbit_angle(0.0, p),
            0.0,
            "t=0 sits at the start of the orbit"
        );
        assert!(
            (planet_orbit_angle(p / 4.0, p) - PI / 2.0).abs() < 1e-9,
            "a quarter period is a quarter turn"
        );
        assert!(
            (planet_orbit_angle(p / 2.0, p) - PI).abs() < 1e-9,
            "half a period is half a turn"
        );
        // A full period returns to the start, and time past a period wraps.
        assert!(
            planet_orbit_angle(p, p).abs() < 1e-9,
            "a full period is back to zero"
        );
        assert!(
            (planet_orbit_angle(p * 1.25, p) - PI / 2.0).abs() < 1e-9,
            "past a period wraps around"
        );
        // The angle stays in [0, TAU).
        for k in 0..20 {
            let a = planet_orbit_angle(p * (k as f64) * 0.137, p);
            assert!(
                (0.0..TAU).contains(&a),
                "the angle is a valid orbital phase"
            );
        }
        // A degenerate or non-finite period is safe (no divide by zero, no NaN out).
        assert_eq!(planet_orbit_angle(5.0, 0.0), 0.0);
        assert_eq!(planet_orbit_angle(5.0, -1.0), 0.0);
        assert_eq!(planet_orbit_angle(f64::INFINITY, p), 0.0);
    }

    #[test]
    fn outer_tiers_draw_their_bodies() {
        let (w, h) = (200usize, 160usize);
        let bg = Rgb::new(8, 9, 14);
        let planet = Rgb::new(60, 120, 90);
        // Planet tier: a globe centred, its centre lit (not background), a corner still empty.
        let disk = planet_disk(planet, 80, w, h, bg);
        assert_eq!(disk.len(), w * h, "one word per pixel");
        let center = (h / 2) * w + (w / 2);
        assert_ne!(disk[center], bg.pack(), "the globe centre is drawn");
        assert_eq!(disk[0], bg.pack(), "a corner is empty space");
        assert_eq!(
            disk,
            planet_disk(planet, 80, w, h, bg),
            "the disc is a pure read and replays"
        );

        // Solar tier: the sun at the frame centre, the planet a dot out along +x at angle 0.
        let sun = Rgb::new(255, 240, 200);
        let orbit_r = 50usize;
        let sys = solar_system(planet, sun, 12, orbit_r, 6, 0.0, w, h, bg);
        let cx = (w / 2) as i32;
        let cy = (h / 2) as i32;
        assert_eq!(
            sys[(cy as usize) * w + cx as usize],
            sun.pack(),
            "the sun sits at the orbital focus"
        );
        let (px, py) = planet_orbit_xy(cx, cy, orbit_r, 0.0);
        assert_eq!(
            (px, py),
            (cx + orbit_r as i32, cy),
            "angle zero puts the planet at +x on the orbit"
        );
        let dot = sys[(py as usize) * w + px as usize];
        assert_ne!(dot, bg.pack(), "the planet dot is drawn on the orbit");
        assert_ne!(dot, sun.pack(), "the planet is not the star");
        assert_eq!(
            sys,
            solar_system(planet, sun, 12, orbit_r, 6, 0.0, w, h, bg),
            "the solar-system frame is a pure read and replays"
        );
    }
}
