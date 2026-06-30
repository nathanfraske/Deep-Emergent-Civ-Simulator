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
//! The terrain and biome substrate, the fixed-point worldgen passes, the chunk grid and
//! quadtree, and the headless glyph view land here next.

pub mod topology;

pub use topology::{Coord3, FlatBounded, Topology, TopologySpace};
