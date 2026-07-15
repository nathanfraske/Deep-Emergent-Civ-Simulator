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
use civsim_sim::geodynamics::DerivedTile;
use civsim_world::terrain::TerrainRelief;
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

/// The approximate DISPLAY colour of a blackbody at effective temperature `t_eff_k` (kelvin): the observability
/// non-canon projection of a star's DERIVED `T_eff` (from [`civsim_sim::astro::stellar_effective_temperature`])
/// onto a screen colour. A cool ~3000 K star reads orange-red, the Sun (~5772 K) a warm near-white, a hot
/// ~10000 K star blue-white, tracking the Planckian locus. The mapping is the piecewise fit of Tanner Helland
/// ("How to Convert Temperature (K) to RGB", 2012), itself a regression to Mitchell Charity's blackbody-colour
/// datafile (`bbr_color.txt`, computed from the CIE 1931 colour-matching functions). Display-only: it reads a
/// derived scalar and returns pixels, writes no canonical state (Principle 10), and uses `f64` because a screen
/// colour needs no fixed-point rigour past per-run determinism.
pub fn blackbody_rgb(t_eff_k: Fixed) -> Rgb {
    // The fit is defined on temperature/100, valid roughly 1000..40000 K; clamp into that band so a derived T_eff
    // past the fit returns its nearest sensible colour rather than a wild extrapolation.
    let temp = (t_eff_k.to_f64_lossy() / 100.0).clamp(10.0, 400.0);
    let clamp255 = |v: f64| v.clamp(0.0, 255.0) as u8;
    let red = if temp <= 66.0 {
        255.0
    } else {
        329.698_727_446 * (temp - 60.0).powf(-0.133_204_759_2)
    };
    let green = if temp <= 66.0 {
        99.470_802_586_1 * temp.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (temp - 60.0).powf(-0.075_514_849_2)
    };
    let blue = if temp >= 66.0 {
        255.0
    } else if temp <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (temp - 10.0).ln() - 305.044_792_730_7
    };
    Rgb::new(clamp255(red), clamp255(green), clamp255(blue))
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
        '^' => [
            0b00100, 0b01010, 0b10001, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '~' => [
            0b00000, 0b00000, 0b01101, 0b10110, 0b00000, 0b00000, 0b00000,
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

/// The glyph a DERIVED tile shows, keyed by its relief class (the R1-override terrain projected to a mark in the
/// Dwarf-Fortress-spirit glyph view): submarine reads as water `~`, lowland as flat ground `.`, upland as raised
/// `^`. Presentation only, a one-way read of the derived relief, never canonical state (Principle 10).
pub fn derived_tile_glyph(relief: TerrainRelief) -> char {
    match relief {
        TerrainRelief::Submarine => '~',
        TerrainRelief::Lowland => '.',
        TerrainRelief::Upland => '^',
    }
}

/// The colour a DERIVED tile paints in the window, keyed by its relief class. The palette (deep water blue,
/// basaltic lowland grey, a lighter upland grey) is the tunable visual projection, an aesthetic call the owner
/// reserves: authored ONLY here in the non-canon renderer, byte-neutral on canon (Principle 10). The relief it
/// keys off is derived (the substrate's elevation crossing the derived references), so what varies across the
/// frame is physics; only the swatch is authored.
pub fn derived_tile_color(relief: TerrainRelief) -> Rgb {
    match relief {
        TerrainRelief::Submarine => Rgb::new(28, 78, 156), // deep water
        TerrainRelief::Lowland => Rgb::new(74, 68, 62),    // basaltic lowland
        TerrainRelief::Upland => Rgb::new(124, 113, 102),  // lighter raised rock
    }
}

/// Draw one glyph centred in a `size` by `size` cell at `(x0, y0)`, the font scaled to the cell. Presentation only.
fn draw_glyph_centered(
    buf: &mut [u32],
    w: usize,
    x0: usize,
    y0: usize,
    size: usize,
    ch: char,
    fg: Rgb,
) {
    let scale = (size / 8).max(1);
    let gw = 5 * scale;
    let gh = 7 * scale;
    let ix = x0 + size.saturating_sub(gw) / 2;
    let iy = y0 + size.saturating_sub(gh) / 2;
    let fgp = fg.pack();
    for (r, bits) in glyph_rows(ch).iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                fill_rect(buf, w, ix + col * scale, iy + r * scale, scale, scale, fgp);
            }
        }
    }
}

/// Paint a field of DERIVED tiles as a `w` by `h` frame: each tile a `tile_px` block of its relief colour
/// ([`derived_tile_color`]) with its relief glyph ([`derived_tile_glyph`]) centred, laid out `cols` to a row in
/// generation order. This is the capstone's visible spine reaching the window: the terrain in the frame is what
/// the substrate derived (composition -> elevation -> relief), never fractal noise or an authored biome swatch
/// (the R1 override, end to end). A pure, deterministic read of the derived field, one-way canon -> view, so it
/// writes no canonical state and adds nothing to the canon hash (Principle 10).
pub fn paint_derived_tiles(
    tiles: &[DerivedTile],
    cols: usize,
    tile_px: usize,
    w: usize,
    h: usize,
    bg: Rgb,
) -> Vec<u32> {
    let tile_px = tile_px.max(3);
    let cols = cols.max(1);
    let mut buf = vec![bg.pack(); w * h];
    for (i, t) in tiles.iter().enumerate() {
        let cx = (i % cols) * tile_px;
        let cy = (i / cols) * tile_px;
        if cx >= w || cy >= h {
            continue;
        }
        let color = derived_tile_color(t.relief);
        fill_rect(&mut buf, w, cx, cy, tile_px, tile_px, color.pack());
        // A readable glyph over the block: dark on a light tile, light on a dark one.
        let fg = if color.luminance() > 128 {
            Rgb::new(20, 20, 24)
        } else {
            Rgb::new(228, 232, 236)
        };
        draw_glyph_centered(
            &mut buf,
            w,
            cx,
            cy,
            tile_px,
            derived_tile_glyph(t.relief),
            fg,
        );
    }
    buf
}

/// The on-screen radius (pixels) a planet's DERIVED radius projects to at a view scale of `m_per_px` metres per
/// pixel: `radius_px = radius_m / m_per_px`. This is the seeable-world size law, so a denser (smaller-radius)
/// planet draws a smaller globe and a larger one draws bigger, straight from [`civsim_sim::astro::planet_radius_m`].
/// All fixed-point, deterministic; a non-positive or overflowing input yields `0` (nothing to draw). Display-only,
/// a one-way read of the derived radius (Principle 10).
pub fn globe_radius_px(radius_m: Fixed, m_per_px: Fixed) -> usize {
    if radius_m <= Fixed::ZERO || m_per_px <= Fixed::ZERO {
        return 0;
    }
    radius_m
        .checked_div(m_per_px)
        .map(|v| v.to_int().max(0) as usize)
        .unwrap_or(0)
}

/// Normalise a 3-vector for display lighting, returning the +z unit vector for a zero input (a safe default facing
/// the viewer). Non-canon display math, `f32` is fine.
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if m <= 0.0 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / m, v[1] / m, v[2] / m]
    }
}

/// Sample the DERIVED-tile surface colour at longitude `u` and latitude `v` (each in `[0, 1)`), an orthographic
/// read of the derived relief field ([`derived_tile_color`]) wrapped onto the globe. An empty field falls back to a
/// deep-ocean stand-in so the sphere still draws. Display-only.
fn sample_derived_surface(tiles: &[DerivedTile], cols: usize, u: f32, v: f32) -> Rgb {
    if tiles.is_empty() || cols == 0 {
        return Rgb::new(40, 72, 120);
    }
    let rows = tiles.len().div_ceil(cols);
    let cu = ((u.clamp(0.0, 0.999_9) * cols as f32) as usize).min(cols - 1);
    let cv = ((v.clamp(0.0, 0.999_9) * rows as f32) as usize).min(rows.saturating_sub(1));
    let idx = (cv * cols + cu).min(tiles.len() - 1);
    derived_tile_color(tiles[idx].relief)
}

/// Draw the planet as a lit sphere: a filled disk of on-screen radius `radius_px` centred at `(cx, cy)`, its
/// surface textured from the DERIVED tiles (an orthographic sphere map of the relief field) and shaded by a Lambert
/// diffuse term against the star direction `star_dir`. The sunlit hemisphere is bright and tinted by `light_tint`
/// (the star's [`blackbody_rgb`]); the night side falls to a faint neutral ambient; the cosine falloff between them
/// is the soft day/night terminator. Pixels outside the disk are left untouched (the caller paints space and, in a
/// later slice, the atmosphere limb). A pure, deterministic read of the derived radius, tiles, and star direction,
/// one-way canon -> pixels, so it writes no canonical state (Principle 10).
#[allow(clippy::too_many_arguments)]
pub fn draw_globe(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    tiles: &[DerivedTile],
    tile_cols: usize,
    star_dir: [f32; 3],
    light_tint: Rgb,
) {
    use std::f32::consts::PI;
    if radius_px == 0 || w == 0 || h == 0 {
        return;
    }
    let r = radius_px as f32;
    let l = normalize3(star_dir);
    let tint = [
        light_tint.r as f32 / 255.0,
        light_tint.g as f32 / 255.0,
        light_tint.b as f32 / 255.0,
    ];
    // A faint neutral ambient so the night hemisphere reads dark but not pure black (skyglow and starlight).
    const AMBIENT: f32 = 0.10;
    let rp = radius_px as i32;
    let x0 = (cx - rp).max(0);
    let x1 = (cx + rp).min(w as i32 - 1);
    let y0 = (cy - rp).max(0);
    let y1 = (cy + rp).min(h as i32 - 1);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let nx = (px - cx) as f32 / r;
            let ny = (py - cy) as f32 / r;
            let d2 = nx * nx + ny * ny;
            if d2 > 1.0 {
                continue; // outside the disk
            }
            let nz = (1.0 - d2).sqrt(); // the front-hemisphere normal, toward the viewer
                                        // Orthographic sphere map. Screen y points down, so world up is -ny; longitude wraps the meridian.
            let lat = (-ny).clamp(-1.0, 1.0).asin(); // -pi/2..pi/2
            let lon = nx.atan2(nz); // -pi..pi
            let u = (lon + PI) / (2.0 * PI);
            let v = (0.5 - lat / PI).clamp(0.0, 1.0);
            let base = sample_derived_surface(tiles, tile_cols, u, v);
            // Lambert diffuse: dot of the surface normal with the star direction, clamped at the terminator.
            let lambert = (nx * l[0] + ny * l[1] + nz * l[2]).max(0.0);
            let shade = |c: u8, t: f32| -> u8 {
                let day = AMBIENT + (1.0 - AMBIENT) * lambert * t;
                (c as f32 * day).clamp(0.0, 255.0) as u8
            };
            let color = Rgb::new(
                shade(base.r, tint[0]),
                shade(base.g, tint[1]),
                shade(base.b, tint[2]),
            );
            buf[py as usize * w + px as usize] = color.pack();
        }
    }
}

/// Draw the star: a solid disk of `color` (its [`blackbody_rgb`]) with a soft radial glow fading to the background
/// over about three radii, so the star's on-screen colour reads as its temperature. Display-only.
fn draw_star(buf: &mut [u32], w: usize, h: usize, sx: i32, sy: i32, radius_px: usize, color: Rgb) {
    if radius_px == 0 || w == 0 || h == 0 {
        return;
    }
    let core = radius_px as i32;
    let glow = core * 3;
    let cr = color.r as f32;
    let cg = color.g as f32;
    let cb = color.b as f32;
    let x0 = (sx - glow).max(0);
    let x1 = (sx + glow).min(w as i32 - 1);
    let y0 = (sy - glow).max(0);
    let y1 = (sy + glow).min(h as i32 - 1);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = (px - sx) as f32;
            let dy = (py - sy) as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = py as usize * w + px as usize;
            if dist <= core as f32 {
                buf[idx] = color.pack();
            } else if dist <= glow as f32 {
                // The glow falls off quadratically from the core edge and blends over whatever is already there.
                let t = 1.0 - (dist - core as f32) / (glow - core).max(1) as f32;
                let a = (t * t).clamp(0.0, 1.0) * 0.8;
                let word = buf[idx];
                let er = (word >> 16) as u8 as f32;
                let eg = (word >> 8) as u8 as f32;
                let eb = word as u8 as f32;
                let mix = |e: f32, c: f32| -> u8 { (e + (c - e) * a).clamp(0.0, 255.0) as u8 };
                buf[idx] = Rgb::new(mix(er, cr), mix(eg, cg), mix(eb, cb)).pack();
            }
        }
    }
}

/// A STAND-IN sky colour for the atmosphere limb: a pale blue placeholder, NOT a derived value.
// TODO(atmosphere): the real limb colour derives from the Stage-8 gas-mix Rayleigh scattering (the manager is
// building that substrate); until it lands this pale-blue fixture stands in, clearly labelled so it is not mistaken
// for physics. When the gas mix is available, replace this constant with a read of the scattered-sky spectrum.
pub const PLACEHOLDER_SKY: Rgb = Rgb::new(150, 190, 235);

/// Draw a soft atmosphere haze around the globe's limb: a thin glow just outside the disk (fading out over
/// `HALO_FRAC` of the radius) plus a faint rim just inside it, brighter on the day side (where the limb faces the
/// star) and dim on the night side. `sky` is the haze colour (a STAND-IN placeholder, see [`PLACEHOLDER_SKY`]; the
/// real colour derives from the Stage-8 gas-mix Rayleigh scattering when that substrate lands). Blends over whatever
/// is already drawn, so it tints the globe's edge and glows against space. Display-only, one-way canon -> pixels.
#[allow(clippy::too_many_arguments)]
fn draw_atmosphere_limb(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    radius_px: usize,
    star_dir: [f32; 3],
    sky: Rgb,
) {
    if radius_px == 0 || w == 0 || h == 0 {
        return;
    }
    // The haze extends this fraction of the radius beyond the limb, and tints this fraction just inside it.
    const HALO_FRAC: f32 = 0.14;
    const RIM_FRAC: f32 = 0.10;
    let r = radius_px as f32;
    let l = normalize3(star_dir);
    let sr = sky.r as f32;
    let sg = sky.g as f32;
    let sb = sky.b as f32;
    let outer = (r * (1.0 + HALO_FRAC)) as i32;
    let x0 = (cx - outer).max(0);
    let x1 = (cx + outer).min(w as i32 - 1);
    let y0 = (cy - outer).max(0);
    let y1 = (cy + outer).min(h as i32 - 1);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let nx = (px - cx) as f32 / r;
            let ny = (py - cy) as f32 / r;
            let d = (nx * nx + ny * ny).sqrt(); // radial distance in radius units
                                                // A band peaking at the limb (d = 1): ramps up over the inner rim, fades out over the outer halo.
            let profile = if d <= 1.0 {
                ((d - (1.0 - RIM_FRAC)) / RIM_FRAC).max(0.0)
            } else {
                (1.0 - (d - 1.0) / HALO_FRAC).max(0.0)
            };
            if profile <= 0.0 {
                continue;
            }
            // The limb point's outward direction, and how much it faces the star (day limb bright, night limb dim).
            let inv = if d > 0.0 { 1.0 / d } else { 0.0 };
            let facing = (nx * inv * l[0] + ny * inv * l[1]).max(0.0);
            let day = 0.15 + 0.85 * facing; // a faint glow survives on the night limb
            let a = (profile * day * 0.6).clamp(0.0, 1.0);
            let idx = py as usize * w + px as usize;
            let word = buf[idx];
            let er = (word >> 16) as u8 as f32;
            let eg = (word >> 8) as u8 as f32;
            let eb = word as u8 as f32;
            let mix = |e: f32, c: f32| -> u8 { (e + (c - e) * a).clamp(0.0, 255.0) as u8 };
            buf[idx] = Rgb::new(mix(er, sr), mix(eg, sg), mix(eb, sb)).pack();
        }
    }
}

/// Compose the zoomed-out solar-system / planet-object view: a `w` by `h` frame of the star and the lit planet
/// globe over space. The star draws as a [`blackbody_rgb`]-coloured disk at `star_px` (its on-screen position, the
/// caller's projection of the orbit, so the orbital phase sets which hemisphere is day). The planet sits at the view
/// centre, its on-screen size the DERIVED radius at this scale ([`globe_radius_px`]), lit from the star direction
/// with the sunlight tinted by the star's colour ([`draw_globe`]): the star-facing hemisphere bright, the far side
/// dark, a soft terminator between. This is the seeable-world payoff entry point: hand it the derived radius, the
/// star's derived `T_eff`, the derived tiles, and the star's projected position, and it draws the star-lit planet.
/// A pure, deterministic read of the derived planet and star (Principle 10); it writes no canonical state.
#[allow(clippy::too_many_arguments)]
pub fn render_solar_system_view(
    radius_m: Fixed,
    t_eff_k: Fixed,
    tiles: &[DerivedTile],
    tile_cols: usize,
    w: usize,
    h: usize,
    m_per_px: Fixed,
    star_px: (i32, i32),
    star_radius_px: usize,
    bg: Rgb,
) -> Vec<u32> {
    let mut buf = vec![bg.pack(); w.max(1) * h.max(1)];
    if w == 0 || h == 0 {
        return buf;
    }
    let star_color = blackbody_rgb(t_eff_k);
    let planet_cx = (w / 2) as i32;
    let planet_cy = (h / 2) as i32;
    let planet_radius_px = globe_radius_px(radius_m, m_per_px);
    // The star direction is the on-screen vector from the planet to the star, lifted out of the screen plane so the
    // lit hemisphere tilts toward the viewer (a readable terminator rather than an edge-on sliver). The in-plane
    // part carries the orbit's projected direction; the fixed z-lift is a display framing, not physics.
    let dx = (star_px.0 - planet_cx) as f32;
    let dy = (star_px.1 - planet_cy) as f32;
    let plane = (dx * dx + dy * dy).sqrt();
    let star_dir = if plane <= 0.0 {
        [0.0, 0.0, 1.0]
    } else {
        [0.72 * dx / plane, 0.72 * dy / plane, 0.70]
    };
    draw_star(
        &mut buf,
        w,
        h,
        star_px.0,
        star_px.1,
        star_radius_px,
        star_color,
    );
    draw_globe(
        &mut buf,
        w,
        h,
        planet_cx,
        planet_cy,
        planet_radius_px,
        tiles,
        tile_cols,
        star_dir,
        star_color,
    );
    // The atmosphere haze around the limb, a STAND-IN sky colour until the Stage-8 gas mix derives it.
    draw_atmosphere_limb(
        &mut buf,
        w,
        h,
        planet_cx,
        planet_cy,
        planet_radius_px,
        star_dir,
        PLACEHOLDER_SKY,
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
    fn blackbody_colour_tracks_effective_temperature() {
        // The star colour is a deterministic pure read of the derived T_eff: the same temperature replays the same
        // colour, and the chromaticity walks the Planckian locus from red through white to blue.
        let sun = blackbody_rgb(Fixed::from_int(5772));
        assert_eq!(
            blackbody_rgb(Fixed::from_int(5772)),
            sun,
            "a pure read replays"
        );
        // The Sun (~5772 K) reads a warm near-white: every channel high, red the strongest, blue a shade lower.
        assert!(
            sun.r > 240 && sun.g > 220 && sun.b > 200,
            "the Sun is near-white, got {sun:?}"
        );
        assert!(sun.r >= sun.g && sun.g >= sun.b, "the Sun leans warm");
        // A cool M dwarf (~3000 K) reads reddish: red dominant, little blue.
        let m_dwarf = blackbody_rgb(Fixed::from_int(3000));
        assert!(
            m_dwarf.r > m_dwarf.b && m_dwarf.r >= m_dwarf.g,
            "an M dwarf is reddish, got {m_dwarf:?}"
        );
        assert!(m_dwarf.b < 160, "an M dwarf carries little blue");
        // A hot early-type star (~10000 K) reads blue-white: blue overtakes red.
        let hot = blackbody_rgb(Fixed::from_int(10000));
        assert!(hot.b > hot.r, "a hot star is bluish, got {hot:?}");
    }

    /// A small hand-built DERIVED-tile field for the globe-texture tests: a 6-wide grid banded by relief, so the
    /// sphere has a surface to wrap without loading the petrology registry.
    fn demo_globe_tiles() -> (Vec<DerivedTile>, usize) {
        let cols = 6usize;
        let rows = 6usize;
        let mut tiles = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            let relief = match r {
                0 | 1 => TerrainRelief::Upland,
                2 | 3 => TerrainRelief::Lowland,
                _ => TerrainRelief::Submarine,
            };
            for _ in 0..cols {
                tiles.push(DerivedTile {
                    elevation: Fixed::from_int(r as i32),
                    relief,
                });
            }
        }
        (tiles, cols)
    }

    /// The mean luminance of the disk pixels on one side of the vertical centre line at `cx`.
    fn half_luminance(buf: &[u32], w: usize, cx: i32, cy: i32, r: i32, right: bool) -> f64 {
        let mut sum = 0f64;
        let mut n = 0f64;
        for py in (cy - r).max(0)..=(cy + r) {
            for px in (cx - r).max(0)..=(cx + r) {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                if right != (dx > 0) {
                    continue;
                }
                let word = buf[py as usize * w + px as usize];
                let rgb = Rgb::new((word >> 16) as u8, (word >> 8) as u8, word as u8);
                sum += rgb.luminance() as f64;
                n += 1.0;
            }
        }
        if n == 0.0 {
            0.0
        } else {
            sum / n
        }
    }

    #[test]
    fn the_globe_is_a_lit_sphere_sized_from_the_derived_radius() {
        use civsim_sim::astro;
        // The on-screen size scales from the DERIVED planet radius: a denser planet of the same mass has a smaller
        // derived radius and draws a smaller disk at the same view scale (a pure read of planet_radius_m).
        let m_per_px = Fixed::from_int(30_000);
        let earth = astro::planet_radius_m(Fixed::ONE, Fixed::from_ratio(5514, 1000))
            .expect("earth radius");
        let dense = astro::planet_radius_m(Fixed::ONE, Fixed::from_int(8)).expect("dense radius");
        let earth_px = globe_radius_px(earth, m_per_px);
        let dense_px = globe_radius_px(dense, m_per_px);
        assert!(
            earth_px > 0 && dense_px > 0,
            "both globes have an on-screen size"
        );
        assert!(
            earth_px > dense_px,
            "a denser, smaller planet draws a smaller globe"
        );
        assert_eq!(
            globe_radius_px(Fixed::ZERO, m_per_px),
            0,
            "no radius, no globe"
        );

        // Draw the Earth globe lit from the right (+x). The star-facing (right) hemisphere reads brighter than the
        // night (left) side, the terminator running down the middle, and the render replays byte for byte.
        let (w, h) = (200usize, 160usize);
        let bg = Rgb::new(8, 9, 14);
        let (tiles, cols) = demo_globe_tiles();
        let (cx, cy) = (100i32, 80i32);
        let radius = 64usize;
        let mut buf = vec![bg.pack(); w * h];
        draw_globe(
            &mut buf,
            w,
            h,
            cx,
            cy,
            radius,
            &tiles,
            cols,
            [1.0, 0.0, 0.0],
            Rgb::new(255, 255, 255),
        );
        assert!(buf.iter().any(|&p| p != bg.pack()), "the globe is drawn");
        let right = half_luminance(&buf, w, cx, cy, radius as i32, true);
        let left = half_luminance(&buf, w, cx, cy, radius as i32, false);
        assert!(
            right > left * 1.5,
            "the sunlit hemisphere is brighter than the night side (right {right:.1} vs left {left:.1})"
        );
        let mut replay = vec![bg.pack(); w * h];
        draw_globe(
            &mut replay,
            w,
            h,
            cx,
            cy,
            radius,
            &tiles,
            cols,
            [1.0, 0.0, 0.0],
            Rgb::new(255, 255, 255),
        );
        assert_eq!(buf, replay, "a pure read replays byte for byte");
    }

    #[test]
    fn the_solar_view_lights_the_globe_from_the_star_and_tints_by_temperature() {
        use civsim_sim::astro;
        let (w, h) = (240usize, 180usize);
        let bg = Rgb::new(6, 7, 12);
        let (tiles, cols) = demo_globe_tiles();
        let radius_m = astro::planet_radius_m(Fixed::ONE, Fixed::from_ratio(5514, 1000))
            .expect("earth radius");
        // A view scale that draws Earth's globe at a legible size, the star off to the left of the planet.
        let m_per_px = Fixed::from_int(80_000);
        let sun_t = astro::stellar_effective_temperature(
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(8, 10),
            Fixed::from_int(50_000),
        )
        .expect("sun T_eff");
        let star_px = (24i32, 40i32); // upper-left of the centred planet
        let frame = render_solar_system_view(
            radius_m, sun_t, &tiles, cols, w, h, m_per_px, star_px, 10, bg,
        );
        assert_eq!(frame.len(), w * h, "one word per pixel");
        // The star disk carries the derived blackbody colour at its core.
        let star_core = frame[star_px.1 as usize * w + star_px.0 as usize];
        assert_eq!(
            star_core,
            blackbody_rgb(sun_t).pack(),
            "the star reads its blackbody colour"
        );
        // The day side faces the star: with the star upper-left, the globe's LEFT hemisphere is brighter.
        let (pcx, pcy) = ((w / 2) as i32, (h / 2) as i32);
        let pr = globe_radius_px(radius_m, m_per_px) as i32;
        let left = half_luminance(&frame, w, pcx, pcy, pr, false);
        let right = half_luminance(&frame, w, pcx, pcy, pr, true);
        assert!(
            left > right * 1.3,
            "the star-facing (left) hemisphere is the day side (left {left:.1} vs right {right:.1})"
        );
        // Deterministic pure read.
        let replay = render_solar_system_view(
            radius_m, sun_t, &tiles, cols, w, h, m_per_px, star_px, 10, bg,
        );
        assert_eq!(frame, replay, "a pure read replays byte for byte");

        // The star's blackbody colour tints the sunlight: a cool ~3200 K star warms the day side (a higher
        // red-to-blue ratio) versus a hot ~9000 K star, at the same geometry.
        let cool = astro::stellar_effective_temperature(
            Fixed::from_ratio(6, 10),
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(8, 10),
            Fixed::from_int(50_000),
        )
        .expect("cool T_eff");
        let hot = astro::stellar_effective_temperature(
            Fixed::from_int(3),
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(8, 10),
            Fixed::from_int(50_000),
        )
        .expect("hot T_eff");
        let day_ratio = |t_eff: Fixed| -> f64 {
            let f = render_solar_system_view(
                radius_m, t_eff, &tiles, cols, w, h, m_per_px, star_px, 10, bg,
            );
            let mut sr = 0f64;
            let mut sb = 0f64;
            for py in (pcy - pr).max(0)..=(pcy + pr) {
                for px in (pcx - pr).max(0)..=(pcx + pr) {
                    let dx = px - pcx;
                    let dy = py - pcy;
                    if dx * dx + dy * dy > pr * pr || dx > 0 {
                        continue; // the lit (left) hemisphere
                    }
                    let word = f[py as usize * w + px as usize];
                    sr += (word >> 16) as u8 as f64;
                    sb += (word & 0xff) as f64;
                }
            }
            (sr + 1.0) / (sb + 1.0)
        };
        assert!(
            day_ratio(cool) > day_ratio(hot),
            "a cool star warms the day side more than a hot star"
        );
    }

    #[test]
    fn the_atmosphere_limb_is_a_day_bright_haze_ring() {
        let unpack = |word: u32| Rgb::new((word >> 16) as u8, (word >> 8) as u8, word as u8);
        let (w, h) = (200usize, 200usize);
        let bg = Rgb::new(6, 7, 12);
        let (cx, cy) = (100i32, 100i32);
        let radius = 60usize;
        let mut buf = vec![bg.pack(); w * h];
        // Star to the right (+x): the day limb is on the right, the night limb on the left.
        draw_atmosphere_limb(
            &mut buf,
            w,
            h,
            cx,
            cy,
            radius,
            [1.0, 0.0, 0.2],
            PLACEHOLDER_SKY,
        );
        // Just outside the day (right) limb the pixel is tinted toward the sky colour: bluer and brighter than space.
        let day = unpack(buf[cy as usize * w + (cx + radius as i32 + 3) as usize]);
        assert!(
            day.b > bg.b + 10 && day.b > day.r,
            "the day limb glows sky-blue, got {day:?}"
        );
        // The night (left) limb at the same offset is dimmer than the day limb.
        let night = unpack(buf[cy as usize * w + (cx - radius as i32 - 3) as usize]);
        assert!(
            day.b > night.b,
            "the day limb is brighter than the night limb"
        );
        // The far background is untouched: the haze is confined to the limb.
        assert_eq!(buf[5 * w + 5], bg.pack(), "the haze stays at the limb");
        // Deterministic pure read.
        let mut replay = vec![bg.pack(); w * h];
        draw_atmosphere_limb(
            &mut replay,
            w,
            h,
            cx,
            cy,
            radius,
            [1.0, 0.0, 0.2],
            PLACEHOLDER_SKY,
        );
        assert_eq!(buf, replay, "a pure read replays byte for byte");
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
    fn the_derived_tile_glyph_and_colour_key_off_relief() {
        // The render mapping is a pure, distinct read of the relief class: three classes, three glyphs, three
        // colours. Water reads bluest; the upland reads lighter than the lowland (the raised-rock swatch), so the
        // frame's contrast tracks the derived relief.
        assert_eq!(derived_tile_glyph(TerrainRelief::Submarine), '~');
        assert_eq!(derived_tile_glyph(TerrainRelief::Lowland), '.');
        assert_eq!(derived_tile_glyph(TerrainRelief::Upland), '^');
        let sub = derived_tile_color(TerrainRelief::Submarine);
        let low = derived_tile_color(TerrainRelief::Lowland);
        let up = derived_tile_color(TerrainRelief::Upland);
        assert!(sub.b > sub.r && sub.b > sub.g, "submarine reads blue");
        assert!(
            up.luminance() > low.luminance(),
            "the upland reads lighter than the lowland"
        );
        assert!(
            sub != low && low != up && sub != up,
            "each relief has a distinct swatch"
        );
    }

    #[test]
    fn paint_derived_tiles_replays_and_shows_the_relief() {
        // The paint is a deterministic pure read: the same derived field paints the same frame, byte for byte. A
        // hand-built render-test field (labelled test-only) of one submarine and one upland tile shows both relief
        // swatches in the frame.
        let field = [
            DerivedTile {
                elevation: Fixed::from_int(-5),
                relief: TerrainRelief::Submarine,
            },
            DerivedTile {
                elevation: Fixed::from_int(9),
                relief: TerrainRelief::Upland,
            },
        ];
        let (cols, tile_px, w, h) = (2usize, 16usize, 32usize, 16usize);
        let bg = Rgb::new(8, 9, 14);
        let frame = paint_derived_tiles(&field, cols, tile_px, w, h, bg);
        assert_eq!(frame.len(), w * h, "one word per pixel");
        assert_eq!(
            frame,
            paint_derived_tiles(&field, cols, tile_px, w, h, bg),
            "a pure read replays byte for byte"
        );
        assert!(
            frame.contains(&derived_tile_color(TerrainRelief::Submarine).pack()),
            "the submarine swatch is in the frame"
        );
        assert!(
            frame.contains(&derived_tile_color(TerrainRelief::Upland).pack()),
            "the upland swatch is in the frame"
        );
    }

    #[test]
    fn an_authored_composition_yields_a_visible_frame_whose_terrain_is_derived() {
        // THE VISIBLE SPINE, END TO END: the labelled Slice-0 demo field (its per-tile composition the only authored
        // input) drives the real substrate to derived elevations, the field datum, and the relief by crossing it,
        // and that DERIVED field paints a frame. The light silica band floats to Upland, the forsterite to Lowland,
        // the dense periclase below the datum to Submarine, so the terrain in the window is what the material is,
        // never fractal noise (the R1 override reaching the viewer). Colour is authored only in the swatch; the
        // relief that selects it is derived. Generation lives in the sim lane; the viewer only reads and paints.
        let tiles =
            civsim_sim::geodynamics::slice0_demo_field(6, 6).expect("the derived demo field");
        // The frame carries all three DERIVED relief classes.
        let has = |r: TerrainRelief| tiles.iter().any(|t| t.relief == r);
        assert!(has(TerrainRelief::Upland), "a light band derives upland");
        assert!(has(TerrainRelief::Lowland), "a middle band derives lowland");
        assert!(
            has(TerrainRelief::Submarine),
            "a dense band derives submarine"
        );
        let bg = Rgb::new(8, 9, 14);
        let frame = paint_derived_tiles(&tiles, 6, 16, 96, 96, bg);
        assert_eq!(
            frame,
            paint_derived_tiles(&tiles, 6, 16, 96, 96, bg),
            "the derived-terrain frame is a deterministic pure read"
        );
        // The frame shows the three DERIVED relief swatches: the terrain reached the window from composition alone.
        assert!(frame.contains(&derived_tile_color(TerrainRelief::Upland).pack()));
        assert!(frame.contains(&derived_tile_color(TerrainRelief::Lowland).pack()));
        assert!(frame.contains(&derived_tile_color(TerrainRelief::Submarine).pack()));
        assert!(
            frame.iter().any(|&p| p != bg.pack()),
            "a derived frame is painted"
        );
    }
}
