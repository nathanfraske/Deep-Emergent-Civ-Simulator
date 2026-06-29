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

//! # civsim-core: the determinism bedrock
//!
//! This crate is the foundation described in design Part 60, Stage 1, and listed
//! in the runbook as buildable and fully testable now because it carries no
//! reserved numbers. It holds, with no external dependencies:
//!
//! - [`Fixed`]: the Q32.32 fixed-point newtype that is the canonical numeric type
//!   (design Part 3.1). It is a newtype rather than a bare `i64` so that the typed
//!   canonical-state boundary of Part 58 is a compile-time property: a float can
//!   never silently stand in for authoritative state.
//! - [`Rng`]: the per-entity counter-based RNG keyed on
//!   `(master_seed, entity, phase, counter)` (design Part 3.2). Every draw is a
//!   pure function of its coordinate, so results never depend on thread schedule.
//! - [`StableId`] and [`Registry`]: process-wide identity that survives promotion,
//!   demotion, save, and load (design Part 2.1, Part 11).
//! - [`Arena`] and [`Slab`]: contiguous arena allocation and a generationally
//!   guarded slab (design Part 2.3).
//! - [`CacheLine`]: the 64-byte alignment wrapper that prevents false sharing
//!   (design Part 2.5).
//! - [`EventLog`]: the append-only history with never-reused identifiers and a
//!   `StableId`-keyed provenance index (design Part 7). The event schema is kept
//!   open per the unresolved R-EVENT backlog item: the kind is a data-defined
//!   identifier, not a closed Rust enum.
//! - [`StateHasher`]: the deterministic fixed-order state hash (design Part 3.5).
//! - The typed canonical-state boundary in [`canonical`].

pub mod arena;
pub mod cacheline;
pub mod canonical;
pub mod event;
pub mod fixed;
pub mod hash;
pub mod id;
pub mod rng;

pub use arena::{Arena, Slab, SlabHandle};
pub use cacheline::CacheLine;
pub use canonical::{quantize_depth_mm, quantize_unit, Canonical, NonCanonical};
pub use event::{Event, EventId, EventKindId, EventLog};
pub use fixed::{Fixed, FRAC_BITS};
pub use hash::StateHasher;
pub use id::{EntityHandle, EntityLocation, PoolId, Registry, StableId, StableRef};
pub use rng::{splitmix64, Rng};
