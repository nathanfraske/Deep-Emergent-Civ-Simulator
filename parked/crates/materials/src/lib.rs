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

//! Parked compatibility for the retired caller-supplied disk composition path.
//!
//! The root `civsim-materials` package retains the abiotic material kernels. This
//! facade re-exports them for archived callers and keeps the former per-world
//! composition generator available only in the parked workspace.

pub use civsim_materials_abiotic::*;

pub mod disk_composition;
