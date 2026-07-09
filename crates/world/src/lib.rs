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

//! # civsim-world: the spatial and worldgen layer
//!
//! The generated world the civilizations inhabit (design Parts 6, 12, 14, 56), built on
//! the determinism bedrock of `civsim-core` and kept separate from the cognition layer
//! (`civsim-sim`) so the spatial substrate stands on its own. It is being built in the
//! order of roadmap M1:
//!
//! - [`topology`]: the coordinate and topology model (design Part 56). A [`topology::Coord3`]
//!   over the 2.5D stacked model and a [`topology::TopologySpace`] trait every spatial
//!   system routes through, so a flat, cylindrical, or spherical world stays selectable;
//!   distance is an exact integer squared distance, so no float or square root enters
//!   canonical state. [`topology::Coord3::key`] folds a cell into a draw-key locus, so the
//!   spatial layer plugs straight into the canonical RNG keying of `civsim-core`.
//!
//! - [`noise`]: deterministic fixed-point fractal value noise, the field generator
//!   worldgen samples (design Part 12).
//! - [`terrain`]: the terrain and biome substrate (design Part 12), biomes recognised
//!   over data-defined field ranges rather than a closed enum.
//! - [`worldgen`]: worldgen pass one and the tile map (design Parts 12, 6, 14), a seed
//!   turned into a bit-identical tile grid with a headless glyph render to view it.
//! - [`lod`]: the chunk grid and the canonical level-of-detail quadtree (design Part 6),
//!   one tree over the tile map summarising each region by its dominant biome.
//! - [`view`]: the camera and the multi-zoom glyph view (design Parts 14, 11, 54), a pure
//!   read of the quadtree that draws the world at any zoom without writing canon.
//! - [`celestial`]: a world's orbital elements (design Parts 14.6, 32), the year and day
//!   lengths in fixed-point world-seconds that the canonical time cadences derive from, so a
//!   world's calendar is a property of its orbit rather than a hardcoded Earth year.
//!
//! The multi-scale GPU view (Part 14) is a later swap of the same [`view`] reads.

pub mod celestial;
pub mod lod;
pub mod noise;
pub mod structure;
pub mod terrain;
pub mod topology;
pub mod view;
pub mod worldgen;

pub use celestial::OrbitalElements;
pub use lod::{ChunkCoord, NodeSummary, QuadTree, CHUNK};
pub use structure::{WorldStructure, EARTH_STRUCTURE};
pub use terrain::{BiomeDef, BiomeId, BiomeSet, Rgb};
pub use topology::{Coord3, FlatBounded, Topology, TopologySpace};
pub use view::Camera;
pub use worldgen::{Tile, TileMap, WorldgenParams};
