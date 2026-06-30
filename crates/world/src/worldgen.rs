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

//! Worldgen pass one and the tile map (design Part 12, Part 6, Part 14).
//!
//! A deterministic CPU pass turns a seed and a [`FlatBounded`] world into a tile grid:
//! per cell, fixed-point fractal noise gives elevation, moisture, and a temperature noise
//! that blends with a latitude gradient, and the [`BiomeSet`] classifies the cell. Every
//! field is fixed-point integer, so the same seed yields a bit-identical map on any
//! machine with no GPU. A headless glyph render makes the map viewable; the multi-scale
//! GPU zoom view (Part 14) and the chunk-and-quadtree LOD structure (Part 6) layer on
//! later and are not needed to look at the world.

use crate::noise::{fractal, FIELD_ELEVATION, FIELD_MOISTURE, FIELD_TEMPERATURE};
use crate::terrain::{BiomeId, BiomeSet};
use crate::topology::{Coord3, FlatBounded, TopologySpace};
use civsim_core::{Fixed, StateHasher};

/// One generated tile: its normalised fields and the biome they classify to.
#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub elevation: Fixed,
    pub moisture: Fixed,
    pub temperature: Fixed,
    pub biome: BiomeId,
}

/// Worldgen tuning. DEVELOPMENT FIXTURE: these shape the generated map's look and are
/// scaffolding for the first viewable slice, not owner-reserved canonical values. The
/// authoritative worldgen calibration is an owner choice; this lets the map be generated
/// and seen now, the way the dev syllable pool lets language run before the owner sets
/// the language calibrations.
#[derive(Clone, Copy, Debug)]
pub struct WorldgenParams {
    /// The coarsest lattice spacing in tiles; finer octaves halve it.
    pub base_period: i32,
    /// How many octaves of noise are summed.
    pub octaves: u32,
    /// How strongly latitude (warm equator, cold poles) weighs against temperature noise,
    /// in `[0, ONE]`.
    pub latitude_weight: Fixed,
}

impl WorldgenParams {
    /// A labelled development fixture, not an owner value.
    pub fn dev_default() -> Self {
        WorldgenParams {
            base_period: 32,
            octaves: 5,
            latitude_weight: Fixed::from_ratio(3, 5),
        }
    }
}

/// A generated world: a [`FlatBounded`] shape and one [`Tile`] per ground-layer cell,
/// stored in row-major (y then x) order.
#[derive(Clone, Debug)]
pub struct TileMap {
    topo: FlatBounded,
    tiles: Vec<Tile>,
}

impl TileMap {
    /// Generate the ground layer of a world deterministically from a seed.
    pub fn generate(
        seed: u64,
        topo: FlatBounded,
        biomes: &BiomeSet,
        params: &WorldgenParams,
    ) -> TileMap {
        let w = topo.width.max(0) as usize;
        let h = topo.height.max(0) as usize;
        let mut tiles = Vec::with_capacity(w * h);
        for y in 0..topo.height {
            for x in 0..topo.width {
                let (xi, yi) = (x, y);
                let elevation = fractal(
                    seed,
                    xi,
                    yi,
                    FIELD_ELEVATION,
                    params.base_period,
                    params.octaves,
                );
                let moisture = fractal(
                    seed,
                    xi,
                    yi,
                    FIELD_MOISTURE,
                    params.base_period,
                    params.octaves,
                );
                let temp_noise = fractal(
                    seed,
                    xi,
                    yi,
                    FIELD_TEMPERATURE,
                    params.base_period,
                    params.octaves,
                );
                let temperature =
                    blend_latitude(yi, topo.height, temp_noise, params.latitude_weight);
                let biome = biomes.classify(elevation, moisture, temperature);
                tiles.push(Tile {
                    elevation,
                    moisture,
                    temperature,
                    biome,
                });
            }
        }
        TileMap { topo, tiles }
    }

    /// The world's shape.
    pub fn topo(&self) -> FlatBounded {
        self.topo
    }

    fn index(&self, c: Coord3) -> Option<usize> {
        if c.z == 0 && self.topo.contains(c) {
            Some(c.y as usize * self.topo.width as usize + c.x as usize)
        } else {
            None
        }
    }

    /// The tile at a ground-layer coordinate, if it is on the world.
    pub fn tile(&self, c: Coord3) -> Option<&Tile> {
        self.index(c).map(|i| &self.tiles[i])
    }

    /// The deterministic state hash of the generated world, walked in canonical
    /// (y then x) order, so the same seed hashes identically on any machine.
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        h.write_u32(self.topo.width.max(0) as u32);
        h.write_u32(self.topo.height.max(0) as u32);
        for t in &self.tiles {
            h.write_fixed(t.elevation);
            h.write_fixed(t.moisture);
            h.write_fixed(t.temperature);
            h.write_u32(t.biome.0 as u32);
        }
        h.finish()
    }

    /// A headless glyph render of the map: one biome glyph per cell, one row per line.
    pub fn render_glyphs(&self, biomes: &BiomeSet) -> String {
        let w = self.topo.width.max(0) as usize;
        let h = self.topo.height.max(0) as usize;
        let mut s = String::with_capacity((w + 1) * h);
        for y in 0..self.topo.height {
            for x in 0..self.topo.width {
                let glyph = self
                    .tile(Coord3::ground(x, y))
                    .map(|t| biomes.glyph(t.biome))
                    .unwrap_or('?');
                s.push(glyph);
            }
            s.push('\n');
        }
        s
    }
}

/// Blend a latitude gradient (warm at the equator row, cold at the poles) with temperature
/// noise, in `[0, ONE]`.
fn blend_latitude(y: i32, height: i32, noise: Fixed, weight: Fixed) -> Fixed {
    let h = height;
    let mid = h / 2;
    let lat = if mid > 0 {
        let dist = (y - mid).abs();
        (Fixed::ONE - Fixed::from_ratio(dist as i64, mid as i64)).clamp(Fixed::ZERO, Fixed::ONE)
    } else {
        Fixed::ONE
    };
    weight.mul(lat) + (Fixed::ONE - weight).mul(noise)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen(seed: u64) -> TileMap {
        TileMap::generate(
            seed,
            FlatBounded::new(48, 24, 1),
            &BiomeSet::dev_default(),
            &WorldgenParams::dev_default(),
        )
    }

    #[test]
    fn the_same_seed_generates_a_bit_identical_world() {
        assert_eq!(gen(0xEA27).state_hash(), gen(0xEA27).state_hash());
    }

    #[test]
    fn a_different_seed_generates_a_different_world() {
        assert_ne!(gen(0xEA27).state_hash(), gen(0x1234).state_hash());
    }

    #[test]
    fn the_render_has_the_world_dimensions() {
        let m = gen(0xEA27);
        let glyphs = m.render_glyphs(&BiomeSet::dev_default());
        let lines: Vec<&str> = glyphs.lines().collect();
        assert_eq!(lines.len(), 24, "one line per row");
        assert!(
            lines.iter().all(|l| l.chars().count() == 48),
            "one glyph per column"
        );
    }

    #[test]
    fn a_generated_world_has_a_mix_of_biomes() {
        let m = gen(0xEA27);
        let mut seen = std::collections::BTreeSet::new();
        for y in 0..24i32 {
            for x in 0..48i32 {
                seen.insert(m.tile(Coord3::ground(x, y)).unwrap().biome);
            }
        }
        assert!(
            seen.len() >= 2,
            "the map is not a single biome: {} kinds",
            seen.len()
        );
    }

    #[test]
    fn tiles_off_the_world_are_none() {
        let m = gen(0xEA27);
        assert!(m.tile(Coord3::ground(0, 0)).is_some());
        assert!(m.tile(Coord3::ground(47, 23)).is_some());
        assert!(m.tile(Coord3::ground(48, 0)).is_none());
        assert!(m.tile(Coord3::new(0, 0, 1)).is_none());
    }
}
