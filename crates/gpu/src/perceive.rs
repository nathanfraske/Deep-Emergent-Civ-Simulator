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

//! The canonical perceive notice-roll on the GPU (the spike for a bit-identical GPU offload of the
//! memory-bound perceive gather). Each per-(being, trace) notice decision is a pure function of the
//! draw-keyed counter RNG (R-RNG-COORD) and the pinned Q32.32 arithmetic (R-GPU-CANON-PIN), so the
//! whole computation ports to a `#[cube]` kernel that reproduces the `crates/core` oracle bit-for-bit:
//! the same `splitmix64` counter RNG, the same `DrawKey::pair` coordinate fold, the same
//! `unit_fixed`, and the same `Fixed` multiply and clamp. Bit-identity then makes this a canonical
//! offload (an authoritative replacement), not a quantized approximation. This is exactly the
//! massively-parallel, memory-bound, draw-keyed, fixed-point workload the GPU wins hardest on.

use crate::prim::{q32_mul, splitmix64};
use crate::stage0::CudaClient;
use cubecl::cuda::CudaRuntime;
use cubecl::prelude::*;

/// The perceive notice decision for one (being, trace) pair, bit-identical to the oracle: fold the
/// draw key `[ABSENT, being, trace, clock, PERCEPTION, slot]` (`Rng::for_coords`), draw the unit roll
/// (`unit_fixed(0)`), form the chance `clamp(salience * acuity, 0, 1)`, and return whether the roll is
/// below the chance. `acuity` is already the being's acuity times its channel acuity (`Fixed` bits);
/// `salience` is the trace's salience (`Fixed` bits).
// The DSL exposes no rotate_left intrinsic, so the coordinate rotates are written by hand.
#[allow(clippy::manual_rotate)]
#[cube]
fn notice_hit(being: u64, trace: u64, clock: u64, seed: u64, acuity: i64, salience: i64) -> u32 {
    // Rng::for_coords(seed, [ABSENT, being, trace, clock, 0x01, 0]) with rotate_left((i % 63) + 1).
    let mut key = splitmix64(seed);
    let absent = 0xFFFF_FFFF_FFFF_FFFFu64; // u64::MAX
    key = splitmix64(key ^ ((absent << 1u32) | (absent >> 63u32))); // i=0, rotate_left 1
    key = splitmix64(key ^ ((being << 2u32) | (being >> 62u32))); // i=1, rotate_left 2
    key = splitmix64(key ^ ((trace << 3u32) | (trace >> 61u32))); // i=2, rotate_left 3
    key = splitmix64(key ^ ((clock << 4u32) | (clock >> 60u32))); // i=3, rotate_left 4
    let phase = 1u64;
    key = splitmix64(key ^ ((phase << 5u32) | (phase >> 59u32))); // i=4, phase PERCEPTION
    let slot = 0u64;
    key = splitmix64(key ^ ((slot << 6u32) | (slot >> 58u32))); // i=5, slot 0
                                                                // at(0) = splitmix64(key ^ 0); unit_fixed(0) = at(0) >> FRAC_BITS (the high 32 bits, in [0, 1)).
    let at0 = splitmix64(key);
    let roll = i64::cast_from(at0 >> 32u32);
    // chance = clamp(salience * acuity, 0, ONE), Q32.32 multiply (floor), then compare.
    let one = 4294967296i64;
    let raw = q32_mul(salience, acuity);
    let hi = select(raw > one, one, raw);
    let chance = select(hi < 0i64, 0i64, hi);
    select(roll < chance, 1u32, 0u32)
}

/// Elementwise perceive notice roll on the GPU: `out[i]` is 1 if being `being[i]` notices trace
/// `trace[i]` this tick, else 0, bit-identical to the CPU oracle. The per-pair inputs are the being id,
/// the trace id, the effective acuity (being acuity times channel acuity, `Fixed` bits), and the trace
/// salience (`Fixed` bits); `clock` and `seed` are the world's, shared by every pair.
#[cube(launch)]
fn notice_kernel(
    being: &Array<u64>,
    trace: &Array<u64>,
    acuity: &Array<i64>,
    salience: &Array<i64>,
    clock: u64,
    seed: u64,
    out: &mut Array<u32>,
) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = notice_hit(
            being[pos],
            trace[pos],
            clock,
            seed,
            acuity[pos],
            salience[pos],
        );
    }
}

/// Run the perceive notice roll on the GPU over a batch of (being, trace) pairs, returning the hit
/// mask (1 = the being notices the trace), bit-identical to the CPU `perceive` decision (CUDA backend).
#[allow(clippy::too_many_arguments)]
pub fn gpu_notice(
    client: &CudaClient,
    being: &[u64],
    trace: &[u64],
    acuity: &[i64],
    salience: &[i64],
    clock: u64,
    seed: u64,
) -> Vec<u32> {
    let n = being.len();
    assert_eq!(n, trace.len(), "gpu_notice: mismatched lengths");
    assert_eq!(n, acuity.len(), "gpu_notice: mismatched lengths");
    assert_eq!(n, salience.len(), "gpu_notice: mismatched lengths");
    if n == 0 {
        return Vec::new();
    }
    let bin = client.create_from_slice(u64::as_bytes(being));
    let tin = client.create_from_slice(u64::as_bytes(trace));
    let ain = client.create_from_slice(i64::as_bytes(acuity));
    let sin = client.create_from_slice(i64::as_bytes(salience));
    let out = client.empty(n * core::mem::size_of::<u32>());
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        notice_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(bin.clone(), n),
            ArrayArg::from_raw_parts(tin.clone(), n),
            ArrayArg::from_raw_parts(ain.clone(), n),
            ArrayArg::from_raw_parts(sin.clone(), n),
            clock,
            seed,
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    u32::from_bytes(&bytes).to_vec()
}
