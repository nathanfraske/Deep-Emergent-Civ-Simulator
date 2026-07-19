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

//! # civsim-foundation: the leaf substrates of the simulation layer
//!
//! This crate holds the modules that used to live in `civsim-sim` and reached no
//! other module of it. Nothing here changed when it moved: the extraction was a
//! pure relocation, gated on both run pins staying bit-exact.
//!
//! ## Why it was split out
//!
//! `civsim-sim` had grown past 100,000 lines in one compilation unit, and most of
//! the active work sits in a handful of large modules (the runner, the world
//! spine, the geodynamics and planetary paths). Every edit to any of those
//! recompiled the whole crate, these substrates included, though none of them
//! reads a line of it. Moving them to a sibling crate takes them off that path.
//! The dependency runs one way, `civsim-sim` on `civsim-foundation` and never the
//! reverse, which is what makes the split hold. It continues the split that
//! `civsim-bio` began.
//!
//! ## How the membership was decided
//!
//! Not by topic. The set is exactly the modules with NO outgoing intra-crate
//! edge, derived by parsing every `crate::` and `super::` path out of the
//! production code of `civsim-sim` (with `#[cfg(test)]` blocks and comments
//! stripped) and taking the modules with out-degree zero. That is why the move
//! needed no trait indirection, no feature flag, and no reordering: a module with
//! no outgoing edge drags nothing behind it. Being outside a cycle is not the
//! same as being free to leave, and the out-degree test is the form of the
//! question that has an answer.
//!
//! ## What it carries
//!
//! - [`material`]: the material substrate, the located, structured matter in the
//!   ground, and the matter-cycle fields over it.
//! - [`decompose`]: the decomposer-driver substrate, what makes matter rot,
//!   carried as world data rather than an engine law.
//! - [`value`]: value profiles and the distance between them (design Part 21, the
//!   resolved R-VALUE-METRIC work, record 62.3).
//! - [`sensorium`]: data-driven, channel-gated perception (the R-SENSORIUM gap).
//! - [`affect`]: transient affect, the fast event-driven emotional state of a
//!   being (the R-EMOTION gap).
//! - [`clock`]: the simulation time model and the observer's playback control
//!   (design Parts 14.6, 32, 54; Principles 3 and 10).
//! - [`conservation`]: the conserved-projection registry of design Part 58. What
//!   must be conserved is a registry each two-tier subsystem declares for itself
//!   rather than a fixed list, so a future subsystem is covered the moment it
//!   registers its own projection.
//! - [`transmission`]: the knowledge-transmission substrate (design Parts 20, 23,
//!   25, 41). A culture copies opaque content-addressed designs, the copy drifts a
//!   fidelity-scaled amount, and an under-practised design is lost. The kernels
//!   are race-blind, reading per-race perception and memory rather than an
//!   authored per-race fidelity table (Principle 9).
//! - [`breeding`]: the sex and breeding-system substrate (design Part 25, R-REPRO).
//! - [`stocks`]: stocks and flows, the ecological substrate (design Part 15).
//! - [`contact_transfer`]: the contact-energy-transfer registry, the data-defined
//!   binding from a contact channel to the physics-floor law it delivers through.
//! - [`surface_transport`] and [`surface_drivers`]: the surface-mass-transport
//!   substrate and its driver kernels (genesis-forward Stage 3).
//! - [`tectonic_regime`]: the descriptive tectonic-regime readout, the
//!   observer-side half of the corrected framing.
//! - [`orbit`]: the orbital-mechanics geometry, where a body sits on its orbit at
//!   a moment in time (design Parts 14.6, 32).
//! - [`located`]: the located-identity join, a tile to its occupants (design Parts
//!   6, 56).
//! - [`sequence`]: the executed-primitive-sequence substrate, one step of a sequence
//!   and the eligibility trace over recently executed sequences. It came here to cut
//!   the `learn` and `locomotion` cycle at its narrowest point.
//! - [`scenario`]: the scenario loader, reading a starting world from data.
//! - [`substrate`]: data-driven substrate definitions and their loader.
//! - [`unified_provenance`]: the joined provenance register.
//! - [`profile`]: coarse per-phase wall-clock profiling, off the canonical path by
//!   construction.
//!
//! ## Cross-crate references in these docs
//!
//! Several modules here describe how a `civsim-sim` module consumes them. Those
//! names are written as BACKTICKED PROSE with the crate named
//! (`civsim_sim::runner::Runner`), never as rustdoc links. A rustdoc link is a
//! dependency in miniature, and this crate must not acquire one on `civsim-sim`:
//! the whole point of the split is that the arrow runs one way. The same
//! convention already governs the references from `civsim-compose` and
//! `civsim-materials` into `civsim-sim`.
//!
//! ## What this crate does NOT carry, and why
//!
//! `physiology` is a leaf by the same test and stayed behind on the owner's call:
//! it belongs to the biology arc that is being overhauled, and a biology decision
//! is kept separate from a structural one. Everything else outside this crate has
//! at least one outgoing edge into `civsim-sim` and cannot move until the module
//! it reaches does.

pub mod affect;
pub mod breeding;
pub mod calibration;
pub mod clock;
pub mod conservation;
pub mod contact_transfer;
pub mod decision;
pub mod decompose;
pub mod located;
pub mod material;
pub mod orbit;
pub mod profile;
pub mod scenario;
pub mod sensorium;
pub mod sequence;
pub mod stocks;
pub mod substrate;
pub mod surface_drivers;
pub mod surface_transport;
pub mod tectonic_regime;
pub mod transmission;
pub mod unified_provenance;
pub mod value;
