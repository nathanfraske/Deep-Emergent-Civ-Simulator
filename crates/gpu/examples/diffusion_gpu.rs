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

//! GPU counterpart of `crates/core`'s `diffusion_bench` (the Part 5.5 workload): the identical
//! canonical fixed-point Jacobi heat-diffusion stencil, run on the GPU via CubeCL. It prints the same
//! XOR checksum, which must match the CPU bench bit-for-bit (the determinism contract), and the
//! throughput. Run (a CUDA device required):
//!
//!   CUDA_PATH=$HOME/.local/cuda \
//!   LD_LIBRARY_PATH=$HOME/.local/cuda/lib:/usr/lib/wsl/lib \
//!   cargo run -p civsim-gpu --release --example diffusion_gpu
//!
//! This is a QUARANTINED dev-fixture harness (authored sizes and a fixed initial field for a
//! demonstration): it is not on the canonical path and produces no canonical state. The canonical
//! guarantee it exercises is that the printed checksum equals the CPU bench's.

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_diffuse};
use std::time::Instant;

const N: u32 = 1024;
const ITERS: u32 = 200;

fn main() {
    // Identical initial field to the CPU bench: ((x*7 + y*13) % 100) as a whole Q32.32 value.
    let n = N as usize;
    let mut field = vec![0i64; n * n];
    for y in 0..n {
        for x in 0..n {
            field[y * n + x] = Fixed::from_int(((x * 7 + y * 13) % 100) as i32).to_bits();
        }
    }
    let k = Fixed::from_ratio(1, 5).to_bits(); // 0.2, below the 0.25 stability bound

    let client = cuda_client();

    // Warm up (kernel JIT-compiles and caches on the first launch), then time the run.
    let _ = gpu_diffuse(&client, &field, N, N, 1, k);
    let t = Instant::now();
    let out = gpu_diffuse(&client, &field, N, N, ITERS, k);
    let el = t.elapsed().as_secs_f64();

    let mut checksum: i64 = 0;
    for v in &out {
        checksum ^= *v;
    }
    let cells = (n * n * ITERS as usize) as f64;
    println!(
        "GPU diffusion {N}x{N} x{ITERS}: {:.0} Mcell-updates/s ({el:.3}s end-to-end), checksum {checksum:#x}",
        cells / el / 1e6
    );
    println!("(compare the checksum against `cargo run -p civsim-core --release --example diffusion_bench`)");
}
