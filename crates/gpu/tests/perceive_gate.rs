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

//! The GPU perceive notice-roll gate: the GPU hit decision must equal the CPU `perceive` decision
//! bit-for-bit over a batch of (being, trace) pairs, proving a canonical GPU offload of the
//! memory-bound gather is possible (the draw-keyed RNG and the pinned Q32.32 arithmetic reproduce the
//! oracle exactly). Self-skips unless `CIVSIM_GPU` is set.

use civsim_core::{DrawKey, Fixed, Phase};
use civsim_gpu::{cuda_client, gpu_notice};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

/// The CPU oracle decision for one (being, trace) pair: the exact `perceive` computation.
fn cpu_hit(being: u64, trace: u64, acuity: i64, salience: i64, clock: u64, seed: u64) -> u32 {
    let roll = DrawKey::pair(being, trace, clock, Phase::PERCEPTION)
        .rng(seed)
        .unit_fixed(0);
    let chance = Fixed::from_bits(salience)
        .mul(Fixed::from_bits(acuity))
        .clamp(Fixed::ZERO, Fixed::ONE);
    if roll < chance {
        1
    } else {
        0
    }
}

#[test]
fn notice_roll_is_bit_identical_to_the_cpu_perceive_decision() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping perceive gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let clock = 42u64;
    let seed = 0xC0FF_EE00_1234_5678u64;

    // A grid of (being, trace) pairs with a spread of acuity and salience, chosen so the notice roll
    // bites (some pairs hit, some do not), the case that makes the RNG comparison meaningful.
    let (mut being, mut trace, mut acuity, mut salience) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for b in 0..120u64 {
        for t in 0..60u64 {
            being.push(b + 1);
            trace.push(1000 + t);
            // acuity 1/8 .. 8/8, salience 1/6 .. 6/6, so chance sweeps below and to one.
            acuity.push(Fixed::from_ratio(((b % 8) + 1) as i64, 8).to_bits());
            salience.push(Fixed::from_ratio(((t % 6) + 1) as i64, 6).to_bits());
        }
    }

    let got = gpu_notice(&client, &being, &trace, &acuity, &salience, clock, seed);
    assert_eq!(got.len(), being.len());
    let mut mism = 0u64;
    let mut hits = 0u64;
    for i in 0..being.len() {
        let want = cpu_hit(being[i], trace[i], acuity[i], salience[i], clock, seed);
        hits += want as u64;
        if got[i] != want {
            mism += 1;
            if mism <= 5 {
                eprintln!(
                    "notice mismatch being={} trace={} acuity={:#x} salience={:#x} got={} want={}",
                    being[i], trace[i], acuity[i], salience[i], got[i], want
                );
            }
        }
    }
    // The gate is only meaningful if a good fraction hit (not all-zero or all-one).
    assert!(
        hits > 0 && hits < being.len() as u64,
        "the roll must bite (hits={hits} of {})",
        being.len()
    );
    assert_eq!(
        mism,
        0,
        "GPU notice roll must equal the CPU perceive decision over all {} pairs",
        being.len()
    );
}
