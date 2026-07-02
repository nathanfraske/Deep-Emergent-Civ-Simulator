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

//! The cross-backend determinism gate (R-GPU-CANON-PIN, the multi-vendor Stage 0 residual). The
//! canonical Stage 0 arithmetic (the pinned multiply and divide) is run on two independent CubeCL
//! backends, CUDA (NVRTC-compiled) and the CPU backend (MLIR/LLVM-compiled), and each output is
//! checked against the `crates/core` `Fixed` oracle. Agreement of both with the oracle means they
//! agree with each other: two independent integer codegen paths produce bit-identical output, the
//! evidence the "only CUDA is proven" note wanted. Self-skips unless `CIVSIM_GPU` is set (it uses the
//! CUDA client, so it needs a device present).
//!
//! Scope limit: the transcendental kernels (`exp` and its siblings) do NOT yet run on the CPU backend.
//! Their more complex control flow (the barrel shifter, the manually unrolled series) trips a panic in
//! `cubecl-cpu`'s constant-propagation pass (`cubecl-opt`), which appears to be an upstream cubecl
//! optimizer bug rather than a kernel defect: the panic is inside cubecl's own pass, and the CUDA
//! backend proves the transcendentals bit-identical (`transcendental_gate.rs`). It is not filed
//! upstream or reduced to a reproducer here, so "appears" is the honest word.
//! Cross-backend confirmation of the transcendentals awaits a cubecl fix or a wgpu backend, so this
//! gate covers the load-bearing arithmetic.

use civsim_core::Fixed;
use civsim_gpu::{cpu_client, cuda_client, gpu_div, gpu_mul};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

fn corners() -> Vec<i64> {
    vec![
        0,
        1,
        -1,
        2,
        -2,
        i64::MAX,
        i64::MIN,
        i64::MIN + 1,
        i64::MAX - 1,
        1 << 32,
        -(1 << 32),
        (1 << 32) + 1,
        1 << 31,
        -(1 << 31),
        1i64 << 62,
        -(1i64 << 62),
    ]
}

/// Corner pairs plus a modest deterministic sweep (kept small: the CPU backend JIT-compiles and runs
/// on the host, so it is far slower than CUDA).
fn cases(seed: u64, sweep: usize) -> (Vec<i64>, Vec<i64>) {
    let cs = corners();
    let mut a = Vec::new();
    let mut b = Vec::new();
    for &x in &cs {
        for &y in &cs {
            a.push(x);
            b.push(y);
        }
    }
    let mut s = seed;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s as i64
    };
    for _ in 0..sweep {
        a.push(next());
        b.push(next());
    }
    (a, b)
}

#[test]
fn stage0_arithmetic_agrees_across_cuda_and_cpu_backends() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping cross-backend gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let cuda = cuda_client();
    let cpu = cpu_client();

    // Multiply.
    let (a, b) = cases(0x1234_5678_9ABC_DEF1, 20_000);
    let m_cuda = gpu_mul(&cuda, &a, &b);
    let m_cpu = gpu_mul(&cpu, &a, &b);
    let mut mism = 0u64;
    for i in 0..a.len() {
        let oracle = Fixed::from_bits(a[i]).mul(Fixed::from_bits(b[i])).to_bits();
        if m_cuda[i] != oracle || m_cpu[i] != oracle {
            mism += 1;
            if mism <= 5 {
                eprintln!(
                    "mul a={:#x} b={:#x} cuda={:#x} cpu={:#x} oracle={:#x}",
                    a[i], b[i], m_cuda[i], m_cpu[i], oracle
                );
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "mul: CUDA and CPU backends must both equal the oracle over {} cases",
        a.len()
    );

    // Divide (drop zero divisors).
    let (da, db) = cases(0x0FED_CBA9_8765_4321, 20_000);
    let mut ka = Vec::new();
    let mut kb = Vec::new();
    for i in 0..da.len() {
        if db[i] != 0 {
            ka.push(da[i]);
            kb.push(db[i]);
        }
    }
    let d_cuda = gpu_div(&cuda, &ka, &kb);
    let d_cpu = gpu_div(&cpu, &ka, &kb);
    let mut dmism = 0u64;
    for i in 0..ka.len() {
        let oracle = Fixed::from_bits(ka[i])
            .div(Fixed::from_bits(kb[i]))
            .to_bits();
        if d_cuda[i] != oracle || d_cpu[i] != oracle {
            dmism += 1;
        }
    }
    assert_eq!(
        dmism,
        0,
        "div: CUDA and CPU backends must both equal the oracle over {} cases",
        ka.len()
    );
}
