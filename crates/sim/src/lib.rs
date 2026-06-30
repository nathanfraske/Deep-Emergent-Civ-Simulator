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

//! # civsim-sim: the staged simulation layer
//!
//! This crate holds the parts the runbook lists as buildable now at the structural
//! and determinism level, with tuned behaviour held until the owner sets the
//! reserved numbers:
//!
//! - [`calibration`]: the calibration manifest loader. Every reserved value loads
//!   as a fail-loud sentinel, so a system that reads an unset required value errors
//!   rather than running on a fabricated default. This is the operational form of
//!   the prime directive that the project never fabricates a value (runbook
//!   section 4, design Principle 11).
//! - [`conservation`]: the conserved-projection registry of design Part 58. What
//!   must be conserved is not a fixed list but a registry each two-tier subsystem
//!   declares for itself, so a future subsystem is covered the moment it registers
//!   its own projection.
//! - [`lod`]: a minimal two-tier world (individuals and aggregate pools) with
//!   promotion, demotion, merge, and split, used to exercise conservation and
//!   referential integrity (design Parts 11, 54, 58).
//! - [`substrate`]: data-driven substrate definitions with round-trip loading, the
//!   schema-and-loader plumbing the runbook says is buildable now while the content
//!   stays data.
//! - [`tom`]: recursive theory of mind (design Part 37, the resolved R-TOM-UPDATE
//!   work). The evidence engine run recursively on whether a target believes a thing,
//!   with a typed anti-projection guarantee (a nested frame admits only access evidence
//!   about its target) and a data-driven access-channel registry, so a false belief
//!   and a seen-through lie come from one mechanism without a closed enum of evidence.

pub mod calibration;
pub mod conservation;
pub mod evidence;
pub mod lod;
pub mod substrate;
pub mod tom;

pub use calibration::{CalibrationError, CalibrationManifest, Profile, ReservedValue};
pub use conservation::{ConservationError, ConservationRegistry};
pub use evidence::{AttrKindId, EvidenceRef, InferenceFrame, InferenceParams};
pub use lod::{Individual, Pool, TwoTierWorld};
pub use substrate::Substrate;
pub use tom::{
    detects_deception, AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights,
    EvidenceOrder, NestedFrame, ProjectionRejected,
};
