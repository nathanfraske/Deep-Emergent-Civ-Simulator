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

use cubecl::prelude::*;

/// The legacy draw-key counter mixer on native `u64`.
#[cube]
pub(crate) fn splitmix64(x: u64) -> u64 {
    let mut z = x + 0x9E3779B97F4A7C15u64;
    z = (z ^ (z >> 30u32)) * 0xBF58476D1CE4E5B9u64;
    z = (z ^ (z >> 27u32)) * 0x94D049BB133111EBu64;
    z ^ (z >> 31u32)
}
