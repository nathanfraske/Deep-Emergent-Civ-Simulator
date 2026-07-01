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

//! CPU throughput baseline for the canonical Q32.32 multiply (single thread), for the GPU-vs-CPU
//! comparison. Run: `cargo run -p civsim-core --release --example mul_throughput`. The GPU counterpart
//! is `docs/working/gpu_mul_throughput.py`; both compute the identical fixed-point product, so the
//! comparison is of raw throughput on the same canonical kernel, not of two different arithmetics.

use civsim_core::Fixed;
use std::time::Instant;

fn main() {
    let n: usize = 64_000_000;
    let reps: usize = 5;
    let a: Vec<Fixed> = (0..n)
        .map(|i| Fixed::from_bits((i as i64).wrapping_mul(2_654_435_761) ^ 0x1234_5678))
        .collect();
    let b: Vec<Fixed> = (0..n)
        .map(|i| Fixed::from_bits((i as i64).wrapping_mul(40_503) ^ 0x7EDC_BA98))
        .collect();
    let mut out = vec![Fixed::ZERO; n];
    // Warm up.
    for i in 0..n {
        out[i] = a[i].mul(b[i]);
    }
    let t = Instant::now();
    for _ in 0..reps {
        for i in 0..n {
            out[i] = a[i].mul(b[i]);
        }
    }
    let el = t.elapsed().as_secs_f64();
    let ops = (n * reps) as f64;
    let mut checksum: i64 = 0;
    for v in &out {
        checksum ^= v.to_bits();
    }
    println!(
        "CPU 1-thread Fixed::mul: {:.0} Mops/s ({ops:.0} ops in {el:.3}s), checksum {checksum:#x}",
        ops / el / 1e6
    );
}
