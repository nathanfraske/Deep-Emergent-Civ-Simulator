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

use civsim_core::splitmix64;
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

            // The organisms on this tile, drawn as marks in a small grid within the block.
            let occ = living.occupants.occupants(coord);
            if occ.is_empty() {
                continue;
            }
            let mark = (tile_px / 3).max(2);
            let gap = (tile_px - mark) / 2;
            for (i, o) in occ.iter().enumerate().take(4) {
                let color = living
                    .occupant_info
                    .get(o)
                    .map(|info| organism_color(info.layer, info.species.0))
                    .unwrap_or(Rgb::new(240, 240, 240));
                // Up to four marks: quadrant offsets so several occupants stay distinct.
                let (qx, qy) = match i {
                    0 => (gap, gap),
                    1 => (gap.saturating_sub(mark / 2), gap.saturating_sub(mark / 2)),
                    2 => (gap + mark / 2, gap + mark / 2),
                    _ => (gap + mark / 2, gap.saturating_sub(mark / 2)),
                };
                fill_rect(&mut buf, w, px0 + qx, py0 + qy, mark, mark, color.pack());
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
