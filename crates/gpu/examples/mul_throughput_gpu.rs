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

//! QUARANTINED DEV-FIXTURE HARNESS (not canonical). Authored sizes and seeds for a throughput
//! demonstration only; it produces no canonical state (design Principle 11). The canonical guarantee
//! it exercises is that both transports return the identical Fixed::mul bits (checked here against the
//! CPU oracle on a sample and by full-buffer equality of the two transports).
//!
//! Before/after benchmark for the Stage 0 limb multiply on CUDA. Both transports run the identical
//! sign-magnitude limb product; they differ only in how the `i64` Fixed bit patterns cross the
//! host/device boundary:
//!
//!   - `gpu_mul_limb_u32` (the u32-limb transport): the host splits every `i64` into low/high `u32`
//!     halves in a per-element loop, uploads four input buffers, and rejoins two output buffers.
//!   - `gpu_mul_native_i64` (the native-i64 transport): the host reinterprets each `i64` slice as raw
//!     bytes (zero per-element arithmetic), uploads two input buffers, and reads one output buffer;
//!     the `i64`->limb split happens in-register on the device.
//!
//! Run (a CUDA device required):
//!
//!   CUDA_PATH=$HOME/.local/cuda \
//!   LD_LIBRARY_PATH=$HOME/.local/cuda/lib:/usr/lib/wsl/lib \
//!   cargo run -p civsim-gpu --release --example mul_throughput_gpu
//!
//! The timing wraps the whole `gpu_mul_*` call; `read_one_unchecked` fences on the device (it waits
//! for the kernel and copies the result back), so the wall clock around it is the end-to-end device
//! time including transfers and the host-side work, which is exactly where the finding located the
//! bottleneck.

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_mul_limb_u32, gpu_mul_native_i64};
use std::time::Instant;

const N: usize = 64_000_000;
const REPS: usize = 5;

/// XOR checksum of a buffer (order-independent equality witness).
fn checksum(v: &[i64]) -> i64 {
    let mut c = 0i64;
    for &x in v {
        c ^= x;
    }
    c
}

/// The per-element host split the u32-limb transport performs before dispatch (four passes over the
/// two input slices), isolated so its cost can be attributed. Mirrors `stage0::split` exactly. Returns
/// the best (minimum) of a few reps in ms; the vectors are checksummed through `black_box` so the maps
/// materialize (a bare `.len()` would let the optimizer skip the loop).
fn host_split_ms(a: &[i64], b: &[i64]) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..REPS {
        let t = Instant::now();
        let a_lo: Vec<u32> = a.iter().map(|&x| x as u64 as u32).collect();
        std::hint::black_box(a_lo.as_ptr());
        let a_hi: Vec<u32> = a.iter().map(|&x| ((x as u64) >> 32) as u32).collect();
        std::hint::black_box(a_hi.as_ptr());
        let b_lo: Vec<u32> = b.iter().map(|&x| x as u64 as u32).collect();
        std::hint::black_box(b_lo.as_ptr());
        let b_hi: Vec<u32> = b.iter().map(|&x| ((x as u64) >> 32) as u32).collect();
        std::hint::black_box(b_hi.as_ptr());
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
    }
    best
}

/// Best (minimum) end-to-end time over `REPS` reps of `f`, in seconds. A warmup call precedes the loop
/// so the CubeCL JIT compile is not in the measured window.
fn best_secs(mut f: impl FnMut() -> Vec<i64>) -> f64 {
    let _ = std::hint::black_box(f()); // warmup / JIT
    let mut best = f64::INFINITY;
    for _ in 0..REPS {
        let t = Instant::now();
        let out = f();
        let el = t.elapsed().as_secs_f64();
        std::hint::black_box(checksum(&out));
        best = best.min(el);
    }
    best
}

fn main() {
    // Deterministic i64 Fixed bit patterns (same shape as the CPU `mul_throughput` bench).
    let a: Vec<i64> = (0..N)
        .map(|i| (i as i64).wrapping_mul(2_654_435_761) ^ 0x1234_5678)
        .collect();
    let b: Vec<i64> = (0..N)
        .map(|i| (i as i64).wrapping_mul(40_503) ^ 0x7EDC_BA98)
        .collect();

    let client = cuda_client();

    // Warm up both kernels (CubeCL JIT-compiles and caches on the first launch).
    let warm_u32 = gpu_mul_limb_u32(&client, &a, &b);
    let warm_i64 = gpu_mul_native_i64(&client, &a, &b);

    // Bit-identity self-check: the two transports must agree with each other and with the CPU oracle.
    assert_eq!(
        checksum(&warm_u32),
        checksum(&warm_i64),
        "the two transports disagree (checksum)"
    );
    for i in (0..N).step_by(N / 64 + 1) {
        let oracle = Fixed::from_bits(a[i]).mul(Fixed::from_bits(b[i])).to_bits();
        assert_eq!(
            warm_i64[i], oracle,
            "native-i64 transport disagrees with the oracle at {i}"
        );
        assert_eq!(
            warm_u32[i], oracle,
            "u32-limb transport disagrees with the oracle at {i}"
        );
    }

    println!("Stage 0 limb multiply throughput, N = {N} elements, CUDA (RTX 5090)");
    println!("(end-to-end per call: host prep + uploads + kernel + readback; readback fences)\n");

    // Measure each transport in its own tight loop (not interleaved) to avoid cross-path memory-pool
    // interference; report the best (minimum) end-to-end time of REPS reps after a warmup.
    let best_u32 = best_secs(|| gpu_mul_limb_u32(&client, &a, &b));
    let best_i64 = best_secs(|| gpu_mul_native_i64(&client, &a, &b));

    let mops_u32 = N as f64 / best_u32 / 1e6;
    let mops_i64 = N as f64 / best_i64 / 1e6;
    let host_ms = host_split_ms(&a, &b);

    println!(
        "u32-limb   (before): {:8.1} ms   {:7.0} Mops/s",
        best_u32 * 1e3,
        mops_u32
    );
    println!(
        "native-i64 (after) : {:8.1} ms   {:7.0} Mops/s",
        best_i64 * 1e3,
        mops_i64
    );
    println!("speedup (best)     : {:.2}x", best_u32 / best_i64);
    println!(
        "host-side split isolated (the per-element i64->u32 loop the u32 path removes): {:.1} ms",
        host_ms
    );
}
