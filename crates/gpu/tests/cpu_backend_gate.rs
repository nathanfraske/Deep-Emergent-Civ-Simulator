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

#![cfg(feature = "cpu-backend")]

//! The device-free Stage 0 bit-identity gate (R-GPU-CANON-PIN). Unlike `stage0_gate.rs` and
//! `cross_backend.rs`, which need a CUDA device and self-skip without `CIVSIM_GPU`, this gate runs the
//! pinned Q32.32 multiply and divide on the CubeCL CPU backend (MLIR/LLVM, no GPU) and checks each
//! output against the `civsim_core::Fixed` oracle. It runs in the sparse CPU evidence lane with
//! `--features cpu-backend`, proving the kernels' codegen and bit-identity on a second independent
//! implementation without adding Tracel LLVM to every pull request.
//!
//! Scope: only the pinned arithmetic runs here. The transcendental kernels (the CORDIC `sin`/`cos` and
//! the `exp` family) still trip the `cubecl-opt` constant-propagation panic on the CPU backend
//! (`constant_prop.rs`, "attempt to subtract with overflow"), so their bit-identity is proven on CUDA
//! only (`transcendental_gate.rs`). The i128-direct multiply is not an option at all: CubeCL 0.10 has
//! no 128-bit integer type (its `IntKind` is `I8`/`I16`/`I32`/`I64`), so the limb multiply gated here
//! is the load-bearing form, not a workaround awaiting an i128 replacement.

use civsim_core::Fixed;
use civsim_gpu::{cpu_client, gpu_div, gpu_mul};

/// Zero, the units, the i64 extremes, and the Q32.32 landmarks (the `stage0_gate.rs` corner set).
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
        0x0000_0001_FFFF_FFFF,
        -0x0000_0001_0000_0000,
    ]
}

/// Every corner pair, then a deterministic xorshift64 sweep. The sweep stays modest because the CPU
/// backend JIT-compiles and executes on the host (far slower than CUDA), and bit-identity is a property
/// of the exact integer algorithm rather than of sample count: the corners pin the sign and narrowing
/// boundaries, the sweep guards against a systematic codegen fault.
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
fn cpu_backend_multiply_is_bit_identical_to_fixed_oracle() {
    let client = cpu_client();
    let (a, b) = cases(0x9E37_79B9_7F4A_7C15, 5_000);
    let got = gpu_mul(&client, &a, &b);
    let mut mism = 0u64;
    for i in 0..a.len() {
        let want = Fixed::from_bits(a[i]).mul(Fixed::from_bits(b[i])).to_bits();
        if got[i] != want {
            mism += 1;
            if mism <= 5 {
                eprintln!(
                    "cpu mul mismatch a={:#x} b={:#x} got={:#x} want={:#x}",
                    a[i], b[i], got[i], want
                );
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "CPU-backend limb multiply must equal Fixed::mul over all {} cases",
        a.len()
    );
}

#[test]
fn cpu_backend_divide_is_bit_identical_to_fixed_oracle() {
    let client = cpu_client();
    let (a, b) = cases(0x2545_F491_4F6C_DD1D, 5_000);
    // Divide-by-zero is a precondition; drop the zero-divisor cases (as the oracle test does).
    let mut ka = Vec::with_capacity(a.len());
    let mut kb = Vec::with_capacity(b.len());
    for i in 0..a.len() {
        if b[i] != 0 {
            ka.push(a[i]);
            kb.push(b[i]);
        }
    }
    let got = gpu_div(&client, &ka, &kb);
    let mut mism = 0u64;
    for i in 0..ka.len() {
        let want = Fixed::from_bits(ka[i])
            .div(Fixed::from_bits(kb[i]))
            .to_bits();
        if got[i] != want {
            mism += 1;
            if mism <= 5 {
                eprintln!(
                    "cpu div mismatch a={:#x} b={:#x} got={:#x} want={:#x}",
                    ka[i], kb[i], got[i], want
                );
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "CPU-backend limb divide must equal Fixed::div over all {} cases",
        ka.len()
    );
}
