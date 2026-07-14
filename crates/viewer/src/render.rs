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
