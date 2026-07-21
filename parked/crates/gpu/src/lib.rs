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

//! Parked compatibility for retired world and organism GPU kernels.
//!
//! The root `civsim-gpu` package retains fixed-point Stage 0, field, and
//! transcendental kernels. This facade restores seed-driven world generation,
//! globe presentation, and biology-facing kernels for archived callers.

pub use civsim_gpu_abiotic::*;

#[path = "../../../../crates/gpu/src/prim.rs"]
mod prim;
mod rng_prim;

pub mod being;
pub mod globe;
pub mod perceive;
pub mod worldgen;

pub use being::{gpu_activate, gpu_body_thermal, gpu_metabolize, gpu_sat_mul};
pub use perceive::gpu_notice;
pub use worldgen::{gpu_worldgen, gpu_worldgen_noise};
