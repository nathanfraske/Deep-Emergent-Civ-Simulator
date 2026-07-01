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
        0 => (46, 176, 74),   // producers: the plants, green
        1 => (214, 176, 58),  // first consumers: herbivores, amber
        _ => (206, 74, 58),   // higher consumers: carnivores, red
    };
    let h = splitmix64(species_id as u64 ^ 0x9E37_79B9_7F4A_7C15);
    let jitter = |base: i32, shift: u32| -> u8 {
        let d = ((h >> shift) & 0x3f) as i32 - 32; // -32..31
        (base + d).clamp(0, 255) as u8
    };
    Rgb::new(jitter(br, 0), jitter(bg, 8), jitter(bb, 16))
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
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100, 0b01000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        ':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '|' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        _ => [0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100], // '?'
    }
}

/// Draw a text label with a filled backing panel, so the cursor readout is legible on the map
/// (the on-canvas names of what the selector points at). `scale` is pixels per font pixel.
/// The panel is clamped to stay on screen.
#[allow(clippy::too_many_arguments)]
pub fn draw_label(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, text: &str, scale: usize, fg: Rgb, bg: Rgb) {
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
pub fn draw_outline(buf: &mut [u32], w: usize, x0: usize, y0: usize, rw: usize, rh: usize, color: Rgb) {
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
            // Biome background, or the empty-space colour off the map.
            let tile_color = if topo.contains(coord) {
                living
                    .map
                    .tile(coord)
                    .map(|t| biomes.color(t.biome))
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
                let bm = info.map(|inf| inf.morphology.body_mass).unwrap_or(Fixed::from_ratio(1, 2));
                let span = Fixed::from_int((tile_px * 3 / 5) as i32);
                let extra = bm.checked_mul(span).map(|v| v.to_int().max(0) as usize).unwrap_or(0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_sim::genesis::{genesis, GenesisParams};

    #[test]
    fn organism_colour_is_deterministic_and_layer_keyed() {
        assert_eq!(organism_color(0, 7), organism_color(0, 7), "same inputs, same colour");
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
        let living = genesis(0xEA27, &params);
        let (w, h, tile_px) = (240usize, 160usize, 18usize);
        // Centre on an occupied tile so at least one organism mark is drawn.
        let center = living.occupants.occupied().next().expect("an occupied tile");
        let buf = super::superfine(&living, &BiomeSet::dev_default(), center, tile_px, w, h, Rgb::new(8, 9, 14));
        assert_eq!(buf.len(), w * h, "one word per pixel");
        assert_eq!(buf, super::superfine(&living, &BiomeSet::dev_default(), center, tile_px, w, h, Rgb::new(8, 9, 14)), "a pure read replays");
        // The centre tile's block carries a mark distinct from the background colour.
        assert!(buf.iter().any(|&p| p != Rgb::new(8, 9, 14).pack()), "something is drawn");
    }
}
