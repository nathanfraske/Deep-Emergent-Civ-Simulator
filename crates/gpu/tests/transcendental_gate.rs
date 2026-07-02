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

//! The device bit-identity gate for the GPU transcendentals (R-GPU-CANON-PIN): each CubeCL kernel
//! must reproduce the `crates/core` `Fixed` oracle bit-for-bit on the 5090, over its representable
//! domain and the saturating edges. Self-skips unless `CIVSIM_GPU` is set.

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_exp, gpu_ln, gpu_powf};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

/// A dense sweep of `x` bit patterns across `[-24, 24]` (the exp domain `[-22, 22]` plus the guard
/// edges and the saturating top), together with named corners.
fn exp_inputs() -> Vec<i64> {
    let mut xs = Vec::new();
    for f in [
        Fixed::ZERO,
        Fixed::ONE,
        Fixed::from_int(-1),
        Fixed::from_int(10),
        Fixed::from_int(-10),
        Fixed::from_int(21),
        Fixed::from_int(22),
        Fixed::from_int(-22),
        Fixed::from_int(23),
        Fixed::from_ratio(1, 2),
        Fixed::from_ratio(-1, 3),
    ] {
        xs.push(f.to_bits());
    }
    let start = -24i64 << 32;
    let end = 24i64 << 32;
    let step = (1i64 << 32) / 1000; // 0.001 in Q32.32
    let mut b = start;
    while b <= end {
        xs.push(b);
        b += step;
    }
    xs
}

#[test]
fn exp_is_bit_identical_to_fixed_oracle() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping exp device gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let xs = exp_inputs();
    let got = gpu_exp(&client, &xs);
    let mut mism = 0u64;
    for i in 0..xs.len() {
        let want = Fixed::from_bits(xs[i]).exp().to_bits();
        if got[i] != want {
            mism += 1;
            if mism <= 5 {
                eprintln!(
                    "exp mismatch x={:#x} got={:#x} want={:#x}",
                    xs[i], got[i], want
                );
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "GPU exp must equal Fixed::exp over all {} inputs",
        xs.len()
    );
}

/// Positive `x` across every exponent (a multiplicative sweep exercising the leading-bit
/// normalization), plus corners and the non-positive sentinel cases.
fn ln_inputs() -> Vec<i64> {
    let mut xs = Vec::new();
    for f in [
        Fixed::ZERO,
        Fixed::from_int(-1),
        Fixed::from_int(-100),
        Fixed::ONE,
        Fixed::from_int(2),
        Fixed::from_int(10),
        Fixed::from_ratio(1, 2),
        Fixed::from_int(1_000_000),
    ] {
        xs.push(f.to_bits());
    }
    let mut b = 1i64 << 20; // ~0.00024
    while b < (1i64 << 62) {
        xs.push(b);
        b += (b >> 6) + 1; // grow ~1.5% per step, covering all exponents
    }
    xs
}

#[test]
fn ln_is_bit_identical_to_fixed_oracle() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping ln device gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let xs = ln_inputs();
    let got = gpu_ln(&client, &xs);
    let mut mism = 0u64;
    for i in 0..xs.len() {
        let want = Fixed::from_bits(xs[i]).ln().to_bits();
        if got[i] != want {
            mism += 1;
            if mism <= 5 {
                eprintln!(
                    "ln mismatch x={:#x} got={:#x} want={:#x}",
                    xs[i], got[i], want
                );
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "GPU ln must equal Fixed::ln over all {} inputs",
        xs.len()
    );
}

#[test]
fn powf_is_bit_identical_to_fixed_oracle() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping powf device gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let bases = [
        Fixed::from_ratio(1, 4),
        Fixed::from_ratio(1, 2),
        Fixed::ONE,
        Fixed::from_int(2),
        Fixed::from_ratio(5, 2),
        Fixed::from_int(10),
        Fixed::from_int(100),
        Fixed::ZERO,         // sentinel -> 0
        Fixed::from_int(-3), // sentinel -> 0
    ];
    let ys = [
        Fixed::from_int(-4),
        Fixed::from_int(-2),
        Fixed::from_int(-1),
        Fixed::ZERO,
        Fixed::from_ratio(1, 2),
        Fixed::ONE,
        Fixed::from_int(2),
        Fixed::from_int(4),
        Fixed::from_ratio(7, 3),
    ];
    let mut base = Vec::new();
    let mut y = Vec::new();
    for b in bases {
        for yy in ys {
            base.push(b.to_bits());
            y.push(yy.to_bits());
        }
    }
    let got = gpu_powf(&client, &base, &y);
    let mut mism = 0u64;
    for i in 0..base.len() {
        let want = Fixed::from_bits(base[i])
            .powf(Fixed::from_bits(y[i]))
            .to_bits();
        if got[i] != want {
            mism += 1;
            if mism <= 8 {
                eprintln!(
                    "powf mismatch base={:#x} y={:#x} got={:#x} want={:#x}",
                    base[i], y[i], got[i], want
                );
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "GPU powf must equal Fixed::powf over all {} pairs",
        base.len()
    );
}
