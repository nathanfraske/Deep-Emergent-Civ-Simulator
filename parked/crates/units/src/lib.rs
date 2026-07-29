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

//! Parked compatibility for the retired emic and civilization measurement surface.
//!
//! The root `civsim-units` package owns the abiotic dimensional substrate. This
//! facade re-exports that substrate for archived callers and restores the emic
//! registry without exposing it to the canonical workspace.

pub use civsim_units_abiotic::*;

pub mod emic;

/// Integer division rounded to nearest, ties to even, for a positive divisor.
///
/// This is the former crate-private helper used by the parked emic registry.
pub(crate) fn idiv_round_half_even(num: i128, den: i128) -> i128 {
    debug_assert!(den > 0);
    let q = num.div_euclid(den);
    let r = num.rem_euclid(den);
    let twice = r * 2;
    if twice < den {
        q
    } else if twice > den {
        q + 1
    } else if q % 2 == 0 {
        q
    } else {
        q + 1
    }
}
