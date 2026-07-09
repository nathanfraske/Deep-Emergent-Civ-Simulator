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

/// The conventional ORDINAL ROLES for the terrain axes the engine's built-in consumers read (Arc 5 T4): the
/// hydrology routing reads elevation, the biome classifier reads all three. A world's tile-axis vector
/// carries these three first, in this order, then any additional niche axes it declares.
///
/// HONEST LIMIT (the end-of-arc audit flagged this, recorded not hidden): T4 makes the axis-generation SHAPE
/// data (a world declares how many axes it carries and how each is generated, via `WorldgenParams.axes`), but
/// the ROLE-to-ordinal binding is still an engine convention, these `pub const` ordinals, NOT world-remappable
/// data. So a world with no temperature, or with elevation at a different ordinal, cannot yet express that
/// through data alone; the hydrology and biome consumers read ordinals 0/1/2. Lifting the role binding into
/// world data (a `WorldgenParams.elevation_axis: Option<usize>` role map read by the consumers) is the
/// follow-on that would make a truly alien terrain a data row here too; the axis-vector SHAPE generalization
/// is the step that unblocks it. Light is likewise not a tile axis (it enters through the abiotic registry,
/// Arc 5 T1, and the environ latitude-light field), so a lightless world expresses that through its abiotic
/// sources, not the tile-axis vector.
pub const AXIS_ELEVATION: usize = 0;
pub const AXIS_MOISTURE: usize = 1;
pub const AXIS_TEMPERATURE: usize = 2;

/// How ONE tile axis is generated (Arc 5 T4): the value SHAPE, not a domain name, so a world declares its own
/// axis set as data rather than the engine hardcoding elevation, moisture, temperature. The dev fixture
/// declares the Earth terrain triad; a world with a pressure layer, a radiation gradient, or a mana field
/// declares its own rows.
#[derive(Clone, Copy, Debug)]
pub enum AxisGenSpec {
    /// A fractal-noise field keyed by a salt (the elevation and moisture shape).
    Fractal { field: u32 },
    /// A fractal-noise field blended with a latitude gradient, warm equator to cold poles (the temperature
    /// shape); `latitude_weight` in `[0, ONE]` weighs the gradient against the noise.
    LatitudeBlendedFractal { field: u32, latitude_weight: Fixed },
    /// A uniform constant everywhere (a world axis with no spatial variation).
    Uniform(Fixed),
}

/// One generated tile (Arc 5 T4): its AXIS VECTOR (the terrain field values in the world's declared axis
/// order, the conventional first three being elevation, moisture, temperature) and the biome they classify
/// to. The vector replaces the fixed elevation/moisture/temperature triad so a world's env dimensions are
/// data, not an engine-hardcoded three.
#[derive(Clone, Debug)]
pub struct Tile {
    pub axes: Vec<Fixed>,
    pub biome: BiomeId,
}

impl Tile {
    /// The value on a tile axis by ordinal; a tile that carries fewer axes reads zero (the absence
    /// convention), so a consumer keying an optional axis never panics.
    pub fn axis(&self, i: usize) -> Fixed {
        self.axes.get(i).copied().unwrap_or(Fixed::ZERO)
    }

    /// The conventional elevation axis (ordinal 0), the hydrology and biome consumers' terrain height.
    pub fn elevation(&self) -> Fixed {
        self.axis(AXIS_ELEVATION)
    }

    /// The conventional moisture axis (ordinal 1), the terrain wetness fraction (floored on
    /// `fluid.moisture_content`).
    pub fn moisture(&self) -> Fixed {
        self.axis(AXIS_MOISTURE)
    }

    /// The conventional temperature axis (ordinal 2).
    pub fn temperature(&self) -> Fixed {
        self.axis(AXIS_TEMPERATURE)
    }
}

/// Worldgen tuning. DEVELOPMENT FIXTURE: these shape the generated map's look and are
/// scaffolding for the first viewable slice, not owner-reserved canonical values. The
/// authoritative worldgen calibration is an owner choice; this lets the map be generated
/// and seen now, the way the dev syllable pool lets language run before the owner sets
/// the language calibrations.
#[derive(Clone, Debug)]
pub struct WorldgenParams {
    /// The coarsest lattice spacing in tiles; finer octaves halve it.
    pub base_period: i32,
    /// How many octaves of noise are summed.
    pub octaves: u32,
    /// How strongly latitude (warm equator, cold poles) weighs against temperature noise,
    /// in `[0, ONE]`. Retained as the dev-fixture source for the temperature axis's blend weight.
    pub latitude_weight: Fixed,
    /// The world's TILE-AXIS GENERATION SPEC (Arc 5 T4): one [`AxisGenSpec`] row per tile axis, in order, so
    /// which env dimensions a world carries and how they are generated is DATA, not the engine's hardcoded
    /// elevation/moisture/temperature triad. The dev fixture declares that triad (the conventional first
    /// three roles the hydrology and biome consumers read); a world declares its own.
    pub axes: Vec<AxisGenSpec>,
}

impl WorldgenParams {
    /// A labelled development fixture, not an owner value.
    pub fn dev_default() -> Self {
        let latitude_weight = Fixed::from_ratio(3, 5);
        WorldgenParams {
            base_period: 32,
            octaves: 5,
            latitude_weight,
            // The Earth terrain triad, byte-identical to the pre-T4 fixed generation: elevation and moisture
            // fractal fields, then a latitude-blended temperature. A world adds or replaces rows here.
            axes: vec![
                AxisGenSpec::Fractal {
                    field: FIELD_ELEVATION,
                },
                AxisGenSpec::Fractal {
                    field: FIELD_MOISTURE,
                },
                AxisGenSpec::LatitudeBlendedFractal {
                    field: FIELD_TEMPERATURE,
                    latitude_weight,
                },
            ],
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
                // Generate each declared tile axis by its spec (Arc 5 T4). The dev spec reproduces the old
                // fixed elevation/moisture/temperature calls exactly, so the axis vector is byte-identical.
                let axes: Vec<Fixed> = params
                    .axes
                    .iter()
                    .map(|spec| match *spec {
                        AxisGenSpec::Fractal { field } => {
                            fractal(seed, xi, yi, field, params.base_period, params.octaves)
                        }
                        AxisGenSpec::LatitudeBlendedFractal {
                            field,
                            latitude_weight,
                        } => {
                            let noise =
                                fractal(seed, xi, yi, field, params.base_period, params.octaves);
                            blend_latitude(yi, topo.height, noise, latitude_weight)
                        }
                        AxisGenSpec::Uniform(v) => v,
                    })
                    .collect();
                // The biome classifier reads the conventional terrain roles (elevation, moisture,
                // temperature) by their designated ordinals off the axis vector.
                let biome = biomes.classify(
                    axes.get(AXIS_ELEVATION).copied().unwrap_or(Fixed::ZERO),
                    axes.get(AXIS_MOISTURE).copied().unwrap_or(Fixed::ZERO),
                    axes.get(AXIS_TEMPERATURE).copied().unwrap_or(Fixed::ZERO),
                );
                tiles.push(Tile { axes, biome });
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
            // Hash the axis vector in order (Arc 5 T4). A three-axis Earth tile hashes elevation, moisture,
            // temperature in that order, byte-identical to the pre-T4 named-field hash.
            for a in &t.axes {
                h.write_fixed(*a);
            }
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
    fn a_world_declares_its_own_tile_axes_as_data_not_the_hardcoded_terran_triad() {
        // Arc 5 T4: a tile's axis vector is generated from the world's AxisGenSpec, not a fixed
        // elevation/moisture/temperature triad. Proof (1): the dev triad produces exactly three axes whose
        // first three match the conventional roles. Proof (2): a world declaring a FOURTH axis (a uniform
        // stand-in for, say, a radiation gradient or a mana field) gets four-axis tiles the engine carries
        // with no code change, and its state hash differs because the extra axis folds in.
        let m = gen(0xEA27);
        let t = m.tile(Coord3::ground(3, 3)).unwrap();
        assert_eq!(
            t.axes.len(),
            3,
            "the dev fixture declares the three terrain axes"
        );
        assert_eq!(t.axis(AXIS_ELEVATION), t.elevation());
        assert_eq!(t.axis(AXIS_MOISTURE), t.moisture());
        assert_eq!(t.axis(AXIS_TEMPERATURE), t.temperature());

        let mut alien = WorldgenParams::dev_default();
        alien
            .axes
            .push(AxisGenSpec::Uniform(Fixed::from_ratio(1, 3)));
        let am = TileMap::generate(
            0xEA27,
            FlatBounded::new(48, 24, 1),
            &BiomeSet::dev_default(),
            &alien,
        );
        let at = am.tile(Coord3::ground(3, 3)).unwrap();
        assert_eq!(
            at.axes.len(),
            4,
            "a world may declare a fourth env axis as data"
        );
        assert_eq!(
            at.axis(3),
            Fixed::from_ratio(1, 3),
            "the declared uniform axis reads its constant everywhere"
        );
        // The first three axes are unchanged (same seed, same specs), so a consumer reading the terrain roles
        // sees identical values; only the added axis is new.
        assert_eq!(at.elevation(), t.elevation());
        assert_eq!(at.moisture(), t.moisture());
        assert_ne!(
            am.state_hash(),
            m.state_hash(),
            "the extra declared axis folds into the world hash"
        );
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
