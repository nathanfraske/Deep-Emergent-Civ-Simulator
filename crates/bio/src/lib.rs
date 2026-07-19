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

//! # civsim-bio: the parked biology arc
//!
//! This crate holds the biology and epistemic modules that used to live in
//! `civsim-sim`. Nothing here changed when it moved: the extraction was a pure
//! relocation, gated on both run pins staying bit-exact.
//!
//! ## Why it was parked
//!
//! `civsim-sim` had grown to about 112,000 lines in one compilation unit, and a
//! large share of the active work is physics: the geodynamics, planetary
//! assembly, stellar, orbital, and materials paths. Every edit to any of those
//! recompiled the whole crate, biology included, though biology reads none of
//! it. Splitting the biology arc into a sibling crate takes it off that path, so
//! a physics change recompiles the physics side alone. The dependency runs one
//! way, `civsim-sim` on `civsim-bio` and never the reverse, which is what makes
//! the split hold.
//!
//! ## What it carries
//!
//! The modules moved as a downward-closed set: every module here depends only on
//! other modules here, on `civsim-core`, `civsim-physics`, and `civsim-compose`.
//! That is why the move needed no trait indirection and no feature flag.
//!
//! - [`genome`]: the genetic substrate. Alleles, haplotypes, linkage, dominance
//!   modes, the gene pool, and the expressed channels a phenotype reads.
//! - [`anatomy`]: the body-plan substrate, deriving what a body affords from its
//!   own parts rather than from an authored creature table.
//! - [`lineage`]: descent records.
//! - [`mate_choice`]: preference and realised fitness over genetic distance.
//! - [`evidence`]: the evidence engine, the inference frame, and diffusion.
//! - [`belief`]: the belief pool, facet strength, and the prevailing belief.
//! - [`tom`]: recursive theory of mind (design Part 37, the resolved R-TOM-UPDATE
//!   work), with its typed anti-projection guarantee and data-driven
//!   access-channel registry.
//! - [`agent`]: the [`agent::Mind`] composing belief and theory of mind into the
//!   epistemic core of an agent.
//! - [`calibration`]: the calibration manifest loader. Every reserved value loads
//!   as a fail-loud sentinel, so a system that reads an unset required value
//!   errors rather than running on a fabricated default (design Principle 11).
//! - [`decision`]: the action, drive, and consideration definitions the belief
//!   and agent paths read.
//!
//! ## What this crate does NOT carry, and why
//!
//! The biology arc is wider than this set. `evolve` (the R-BEHAVIOR-EVOLVE
//! implementation, design Part 8.4, record 62.19), `biosphere`, `discovery`,
//! `edibility`, and `conviction_experience` are still in `civsim-sim`.
//!
//! An earlier revision of this note said each was held by a production
//! dependency cycle. Measured over the production import graph, that holds for
//! `evolve` alone: it imports `crate::runner`, which cycles with `environ` and
//! `genesis`, so its closure is 39 modules. The `learn` and `locomotion` cycle
//! that the note also named is now broken (the shared types moved to
//! `civsim_foundation::sequence`), and the other three were never inside a
//! cycle at all: `conviction_experience` closes over 3 modules, `biosphere`
//! over 4, `edibility` over 6, and `discovery` over 18, none of them cyclic.
//!
//! What holds them is closure cost rather than cyclicity: each drags modules
//! that stay behind, so the move is a decision about where those modules
//! belong and not a mechanical relocation. Being outside a cycle is not the
//! same as being free to leave. They are parked here on the owner's call while
//! the biology arc is overhauled, so the placement question is answered by that
//! work rather than by the import graph.
//!
//! ## A note on [`calibration`]
//!
//! The calibration manifest is not biology. It is here because the epistemic
//! modules read it and it is a leaf, so the closure demanded it. It belongs in a
//! crate of its own, or in a shared lower crate, once one exists; it cannot go
//! to `civsim-core`, whose manifest deliberately carries no external
//! dependencies and so cannot take serde or toml.

pub mod agent;
pub mod anatomy;
pub mod belief;
pub mod evidence;
pub mod genome;
pub mod lineage;
pub mod mate_choice;
pub mod tom;
