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

//! QUARANTINED DEV-FIXTURE HARNESS (not canonical). This example uses authored, dev-fixture numbers
//! (calibrations, seeds, scenario values) to produce a result for demonstration and testing only, and
//! its behaviour is not authoritative (design Principle 11, the reserved-value discipline: an authored
//! constant in the path of world content is a defect until it earns its place). The canonical runner
//! is manifest-driven and fail-loud with zero unapproved authored features; see docs/QUARANTINE.md.
//!
//! CPU baseline for a canonical fixed-point heat-diffusion field (a Jacobi stencil, the Part 5.5 GPU
//! workload), the compute-bound counterpart to the elementwise multiply bench. Each cell reuses its
//! four toroidal neighbours, so this is arithmetic-dense rather than bandwidth-bound, the regime where
//! the GPU pulls furthest ahead. Run `cargo run -p civsim-core --release --example diffusion_bench`.
//! The GPU counterpart is `docs/working/gpu_diffusion.py`; both run the identical fixed-point stencil,
//! so the printed checksum must match bit-for-bit (the determinism contract) and the times compare
//! raw throughput. The production kernel is a CubeCL `#[cube]` function (design Part 5).

use civsim_core::Fixed;
use std::time::Instant;

const N: usize = 1024;
const ITERS: usize = 200;

fn main() {
    let k = Fixed::from_ratio(1, 5); // diffusion coefficient 0.2, below the 0.25 stability bound
    let mut f = vec![Fixed::ZERO; N * N];
    for y in 0..N {
        for x in 0..N {
            f[y * N + x] = Fixed::from_int(((x * 7 + y * 13) % 100) as i32);
        }
    }
    let mut g = f.clone();
    let t = Instant::now();
    for _ in 0..ITERS {
        for y in 0..N {
            let yu = ((y + N - 1) % N) * N;
            let yd = ((y + 1) % N) * N;
            let yc = y * N;
            for x in 0..N {
                let xl = (x + N - 1) % N;
                let xr = (x + 1) % N;
                let c = f[yc + x];
                let lap = f[yu + x] + f[yd + x] + f[yc + xl] + f[yc + xr]
                    - Fixed::from_bits(c.to_bits() * 4);
                g[yc + x] = c + k.mul(lap);
            }
        }
        std::mem::swap(&mut f, &mut g);
    }
    let el = t.elapsed().as_secs_f64();
    let cells = (N * N * ITERS) as f64;
    let mut checksum: i64 = 0;
    for v in &f {
        checksum ^= v.to_bits();
    }
    println!(
        "CPU 1-thread diffusion {N}x{N} x{ITERS}: {:.1} Mcell-updates/s ({el:.2}s), checksum {checksum:#x}",
        cells / el / 1e6
    );
}
