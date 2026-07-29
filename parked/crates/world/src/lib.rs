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

//! Parked compatibility for retired authored world generation and fixtures.
//!
//! The root `civsim-world` package retains only abiotic spatial operators. This
//! facade restores the old seed-driven map, biome catalog, quadtree, camera,
//! caller-supplied structure, and orbital fixture outside the canonical graph.

pub use civsim_world_abiotic::{
    ballistic, classify_relief, crater, drag_flight, eruption, flood, geology, impact_event,
    impact_flux, label, momentum, redistribute, relief_datum, runout, solve, surface_coupling,
    topology, Coord3, EarthworkField, FlatBounded, GeodynamicColumn, GeodynamicField,
    TerrainRelief, Topology, TopologySpace,
};

pub mod celestial;
pub mod lod;
pub mod noise;
pub mod structure;
pub mod terrain;
pub mod view;
pub mod worldgen;

pub use celestial::OrbitalElements;
pub use lod::{ChunkCoord, NodeSummary, QuadTree, CHUNK};
pub use structure::{WorldStructure, EARTH_STRUCTURE};
pub use terrain::{BiomeDef, BiomeId, BiomeSet, Rgb};
pub use view::Camera;
pub use worldgen::{Tile, TileMap, WorldgenParams};
