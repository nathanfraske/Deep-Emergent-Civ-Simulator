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

//! The thermochemical instantiation of the Verdict kernel contract: the proposer, disposer, and freezer for
//! chemical selection (`MATERIALS_ORACLE_SPEC.md` stages 2, 4, 5).
//!
//! This is a plugin over the generic kernel ([`crate::verdict`], [`crate::contract`]): it supplies the physics
//! (the free-energy content) while the kernel owns the selection discipline. Slice a of the Stage-2 proposer
//! ([`proposer`]) is here; the disposer and freezer land in following slices.

pub mod proposer;

pub use proposer::{
    charge_neutral_primitives, Composition, Environment, Stoichiometry, ThermochemicalProposer,
};
