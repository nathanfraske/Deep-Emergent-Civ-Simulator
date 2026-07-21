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

//! # civsim-world: abiotic spatial operators
//!
//! - [`topology`]: the coordinate and topology model (design Part 56). A [`topology::Coord3`]
//!   over the 2.5D stacked model and a [`topology::TopologySpace`] trait every spatial
//!   system routes through, so a flat, cylindrical, or spherical world stays selectable;
//!   distance is an exact integer squared distance, so no float or square root enters
//!   canonical state. [`topology::Coord3::key`] folds a cell into a draw-key locus, so the
//!   spatial layer plugs straight into the canonical RNG keying of `civsim-core`.
//!
//! - [`geology`]: sparse, deterministic resident fields at the interior-to-surface
//!   boundary, including effective elevation deltas and per-column geodynamic state.
//! - [`terrain`]: generic relief classification over references derived by the physical
//!   pipeline. It carries no biome catalog or authored terrain selector.

pub mod ballistic;
pub mod crater;
pub mod drag_flight;
pub mod eruption;
pub mod flood;
pub mod geology;
pub mod impact_event;
pub mod impact_flux;
pub mod label;
pub mod momentum;
pub mod redistribute;
pub mod runout;
pub mod solve;
pub mod surface_coupling;
pub mod terrain;
pub mod topology;

pub use geology::{EarthworkField, GeodynamicColumn, GeodynamicField};
pub use terrain::{classify_relief, relief_datum, TerrainRelief};
pub use topology::{Coord3, FlatBounded, Topology, TopologySpace};
