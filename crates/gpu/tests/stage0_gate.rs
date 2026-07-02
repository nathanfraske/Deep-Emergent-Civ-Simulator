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

//! The Stage 0 device bit-identity gate for R-GPU-CANON-PIN: the pinned CubeCL limb multiply and
//! divide, run on the actual GPU, compared bit-for-bit against the `civsim_core::Fixed` oracle over
//! every corner pair and a large deterministic sweep. This is the device confirmation of the
//! backend-general software gate in `crates/core/tests/gpu_emulation.rs`. It self-skips (a pass with a
//! printed note) unless `CIVSIM_GPU` is set, so the workspace stays green on a machine with no device.

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_div, gpu_fixed_mul, gpu_mul};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

/// The corner set from `gpu_emulation.rs`: zero, units, the i64 extremes, and the Q32.32 landmarks.
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

/// Every corner pair, then a deterministic xorshift64 sweep across the full i64 range.
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
fn stage0_multiply_is_bit_identical_to_fixed_oracle() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping stage0 multiply device gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let (a, b) = cases(0x9E37_79B9_7F4A_7C15, 1_000_000);
    let got = gpu_mul(&client, &a, &b);
    let mut mism = 0u64;
    for i in 0..a.len() {
        let want = Fixed::from_bits(a[i]).mul(Fixed::from_bits(b[i])).to_bits();
        if got[i] != want {
            mism += 1;
            if mism <= 5 {
                eprintln!("mul mismatch a={:#x} b={:#x} got={:#x} want={:#x}", a[i], b[i], got[i], want);
            }
        }
    }
    assert_eq!(mism, 0, "GPU limb multiply must equal Fixed::mul over all {} cases", a.len());
}

#[test]
fn stage0_divide_is_bit_identical_to_fixed_oracle() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping stage0 divide device gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let (mut a, mut b) = cases(0x2545_F491_4F6C_DD1D, 1_000_000);
    // divide-by-zero is a precondition; drop the zero-divisor cases (same as the oracle test).
    let mut ka = Vec::with_capacity(a.len());
    let mut kb = Vec::with_capacity(b.len());
    for i in 0..a.len() {
        if b[i] != 0 {
            ka.push(a[i]);
            kb.push(b[i]);
        }
    }
    a = ka;
    b = kb;
    let got = gpu_div(&client, &a, &b);
    let mut mism = 0u64;
    for i in 0..a.len() {
        let want = Fixed::from_bits(a[i]).div(Fixed::from_bits(b[i])).to_bits();
        if got[i] != want {
            mism += 1;
            if mism <= 5 {
                eprintln!("div mismatch a={:#x} b={:#x} got={:#x} want={:#x}", a[i], b[i], got[i], want);
            }
        }
    }
    assert_eq!(mism, 0, "GPU limb divide must equal Fixed::div over all {} cases", a.len());
}

#[test]
fn field_q32_mul_is_bit_identical_to_fixed_oracle() {
    // The diffusion kernel's coefficient multiply (field.rs q32_mul, the i64-boundary form) is a
    // second copy of the multiply, exercised in a diffusion run only over narrow field values. Gate it
    // over the same corners + 1M sweep as the Stage 0 limb multiply, so both copies are proven over the
    // full range (including the i64::MIN corners), not just where they happen to agree.
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping field q32_mul device gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let (a, b) = cases(0x9E37_79B9_7F4A_7C15, 1_000_000);
    let got = gpu_fixed_mul(&client, &a, &b);
    let mut mism = 0u64;
    for i in 0..a.len() {
        let want = Fixed::from_bits(a[i]).mul(Fixed::from_bits(b[i])).to_bits();
        if got[i] != want {
            mism += 1;
            if mism <= 5 {
                eprintln!("q32_mul mismatch a={:#x} b={:#x} got={:#x} want={:#x}", a[i], b[i], got[i], want);
            }
        }
    }
    assert_eq!(mism, 0, "field q32_mul must equal Fixed::mul over all {} cases", a.len());
}
