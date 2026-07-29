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

//! The GPU being-kernel gate: `gpu_body_thermal` must reproduce the runner's `phase_body_exchange`
//! (discrete Newton cooling of each being toward its cell temperature) bit-for-bit. The CPU reference
//! is the exact same update written against `civsim_core::Fixed`. Self-skips unless `CIVSIM_GPU` is set.

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_activate, gpu_body_thermal, gpu_metabolize, gpu_sat_mul};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

/// The exact CPU reference: `next = bt + exchange * (env - bt)` per being.
fn cpu_body_thermal(field: &[i64], cell: &[u32], body_temp: &[i64], exchange: Fixed) -> Vec<i64> {
    body_temp
        .iter()
        .zip(cell)
        .map(|(&bt, &c)| {
            let bt = Fixed::from_bits(bt);
            let env = Fixed::from_bits(field[c as usize]);
            (bt + exchange.mul(env - bt)).to_bits()
        })
        .collect()
}

#[test]
fn body_thermal_is_bit_identical_to_phase_body_exchange() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping being gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    // A field with a spread of positive and negative temperatures, so `env - bt` takes both signs and
    // exercises the pinned multiply's sign handling.
    let (w, h) = (20usize, 12usize);
    let field: Vec<i64> = (0..w * h)
        .map(|i| Fixed::from_ratio(((i * 7) % 40) as i64 - 20, 3).to_bits())
        .collect();
    let n = 500usize;
    let cell: Vec<u32> = (0..n).map(|i| ((i * 13) % (w * h)) as u32).collect();
    let body_temp: Vec<i64> = (0..n)
        .map(|i| Fixed::from_ratio(((i * 5) % 30) as i64 - 10, 4).to_bits())
        .collect();
    let exchange = Fixed::from_ratio(1, 8);

    let got = gpu_body_thermal(&client, &field, &cell, &body_temp, exchange.to_bits());
    let want = cpu_body_thermal(&field, &cell, &body_temp, exchange);
    assert_eq!(got.len(), want.len());
    let mism = got.iter().zip(&want).filter(|(a, b)| a != b).count();
    assert_eq!(
        mism, 0,
        "GPU body-thermal must equal phase_body_exchange over all {n} beings"
    );
}

/// The CPU oracle for the controller's saturating product: the checked Q32.32 product, or the signed
/// extreme by the operands' signs on overflow.
fn cpu_sat_mul(a: i64, b: i64) -> i64 {
    match Fixed::from_bits(a).checked_mul(Fixed::from_bits(b)) {
        Some(p) => p.to_bits(),
        None => {
            if (a < 0) ^ (b < 0) {
                Fixed::MIN.to_bits()
            } else {
                Fixed::MAX.to_bits()
            }
        }
    }
}

#[test]
fn sat_mul_is_bit_identical_to_the_controller_sat_mul() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping sat_mul gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let one = Fixed::ONE.to_bits();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let mut push = |x: i64, y: i64| {
        a.push(x);
        b.push(y);
    };

    // Corners: zero, +/-one, +/-two, i64 extremes.
    let corners = [
        0i64,
        one,
        -one,
        i64::MAX,
        i64::MIN,
        1,
        -1,
        Fixed::from_int(2).to_bits(),
        Fixed::from_int(-2).to_bits(),
    ];
    for &x in &corners {
        for &y in &corners {
            push(x, y);
        }
    }
    // Boundary: integer products straddling 2^31 (where the Q32.32 magnitude straddles 2^63).
    for k1 in 46338i32..46344 {
        for k2 in 46338i32..46344 {
            push(Fixed::from_int(k1).to_bits(), Fixed::from_int(k2).to_bits());
            push(
                Fixed::from_int(-k1).to_bits(),
                Fixed::from_int(k2).to_bits(),
            );
            push(
                Fixed::from_int(-k1).to_bits(),
                Fixed::from_int(-k2).to_bits(),
            );
        }
    }
    // Regression (a converged blind-audit finding): products whose low-96 magnitude is exactly 2^63
    // (w1 == 0, w2 == 0x80000000) but with a nonzero discarded low word w0, so the true shifted value is
    // -(2^63 + 1) and a differing-sign product must saturate to i64::MIN. The earlier overflow check
    // omitted the `w0 == 0` conjunct and returned i64::MAX, the opposite extreme. Every boundary case the
    // integer sweep above generates has w0 == 0, which is exactly why it missed this. Both operands fit
    // i64 (a is near 2^32, b near 2^63).
    push(4294967297i64, -9223372034707292161i64); // the auditors' exact counterexample
    push(-4294967297i64, 9223372034707292161i64);
    push(4294967298i64, -9223372032559808514i64);
    push(-4294967298i64, 9223372032559808514i64);
    push(4294967299i64, -9223372030412324869i64);
    push(-4294967299i64, 9223372030412324869i64);
    push(4294967300i64, -9223372028264841224i64);
    push(-4294967300i64, 9223372028264841224i64);
    push(4294967301i64, -9223372026117357581i64);
    push(-4294967301i64, 9223372026117357581i64);
    push(4294967302i64, -9223372023969873938i64);
    push(-4294967302i64, 9223372023969873938i64);
    push(4294967305i64, -9223372017527423017i64);
    push(-4294967305i64, 9223372017527423017i64);

    // Pseudo-random sweep (splitmix64): full-range pairs (mostly saturate) and reduced-magnitude pairs
    // (mostly fit), so both the overflow and the fitting paths are stressed.
    let mut s = 0x1234_5678_9ABC_DEF0u64;
    let mut nxt = || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as i64
    };
    for _ in 0..6000 {
        push(nxt(), nxt());
    }
    for _ in 0..3000 {
        push(nxt() >> 20, nxt() >> 20);
    }

    let got = gpu_sat_mul(&client, &a, &b);
    let (mut mism, mut n_sat, mut n_fit) = (0u64, 0u64, 0u64);
    for i in 0..a.len() {
        let want = cpu_sat_mul(a[i], b[i]);
        if want == i64::MAX || want == i64::MIN {
            n_sat += 1;
        } else {
            n_fit += 1;
        }
        if got[i] != want {
            mism += 1;
            if mism <= 8 {
                eprintln!(
                    "sat_mul mismatch a={:#018x} b={:#018x} got={:#018x} want={:#018x}",
                    a[i], b[i], got[i], want
                );
            }
        }
    }
    assert!(
        n_sat > 0 && n_fit > 0,
        "sweep must hit both saturation and fit (sat={n_sat} fit={n_fit})"
    );
    assert_eq!(
        mism, 0,
        "GPU sat_mul must equal the CPU sat_mul over all {} pairs (saturating {n_sat}, fitting {n_fit})",
        a.len()
    );
}

/// The CPU oracle for the controller's activation: the saturating sum of the saturating products,
/// clamped to [-1, 1]. Mirrors `activate` in `controller.rs`.
fn cpu_activate(weights: &[i64], inputs: &[i64]) -> i64 {
    let neg_one = Fixed::from_int(-1);
    let terms = weights.iter().zip(inputs).map(|(&w, &x)| {
        match Fixed::from_bits(w).checked_mul(Fixed::from_bits(x)) {
            Some(p) => p,
            None => {
                if (w < 0) ^ (x < 0) {
                    Fixed::MIN
                } else {
                    Fixed::MAX
                }
            }
        }
    });
    Fixed::saturating_sum(terms)
        .clamp(neg_one, Fixed::ONE)
        .to_bits()
}

#[test]
fn activate_is_bit_identical_to_the_controller_activate() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping activate gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let n_terms = 12usize;
    let mut s = 0xAC71_5A7E_0BAD_F00Du64;
    let mut nxt = || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as i64
    };
    let (mut w, mut x) = (Vec::new(), Vec::new());
    // Batch A: moderate weights (~[-4,4]) and unit-ish inputs, so sums straddle +/-1 and clamp often.
    // Batch B: full-range weights, so sat_mul saturates and the 128-bit sum overflows i64.
    // Batch C: tiny weights and inputs, so the sum stays inside [-1, 1] (the pass-through path).
    for _ in 0..1500 {
        for _ in 0..n_terms {
            w.push(nxt() >> 30);
            x.push(nxt() >> 33);
        }
    }
    for _ in 0..1500 {
        for _ in 0..n_terms {
            w.push(nxt());
            x.push(nxt() >> 20);
        }
    }
    for _ in 0..1500 {
        for _ in 0..n_terms {
            w.push(nxt() >> 40);
            x.push(nxt() >> 40);
        }
    }

    let got = gpu_activate(&client, &w, &x, n_terms as u32);
    let n_acts = w.len() / n_terms;
    assert_eq!(got.len(), n_acts);
    let one = Fixed::ONE.to_bits();
    let neg_one = Fixed::from_int(-1).to_bits();
    let (mut mism, mut hi_clamp, mut lo_clamp, mut pass) = (0u64, 0u64, 0u64, 0u64);
    for a in 0..n_acts {
        let ww = &w[a * n_terms..(a + 1) * n_terms];
        let xx = &x[a * n_terms..(a + 1) * n_terms];
        let want = cpu_activate(ww, xx);
        if want == one {
            hi_clamp += 1;
        } else if want == neg_one {
            lo_clamp += 1;
        } else {
            pass += 1;
        }
        if got[a] != want {
            mism += 1;
            if mism <= 8 {
                eprintln!(
                    "activate mismatch act={a} got={:#018x} want={:#018x}",
                    got[a], want
                );
            }
        }
    }
    assert!(
        hi_clamp > 0 && lo_clamp > 0 && pass > 0,
        "sweep must hit both clamps and the pass-through (hi={hi_clamp} lo={lo_clamp} pass={pass})"
    );
    assert_eq!(
        mism, 0,
        "GPU activate must equal the CPU activate over all {n_acts} activations (hi={hi_clamp} lo={lo_clamp} pass={pass})"
    );
}

/// The CPU oracle for `Stock::regen_increment`: the logistic per-step regen.
fn cpu_regen(amount: Fixed, capacity: Fixed, regen_rate: Fixed) -> Fixed {
    let z = Fixed::ZERO;
    if capacity <= z || amount <= z {
        return z;
    }
    let ratio = match amount.checked_div(capacity) {
        Some(r) => r,
        None => return z,
    };
    let gap = Fixed::ONE - ratio;
    let og = match amount.checked_mul(gap) {
        Some(v) => v,
        None => return z,
    };
    regen_rate.checked_mul(og).unwrap_or(z)
}

/// The CPU oracle for one being-and-axis drain: `Homeostasis::metabolize` then `Stock::step`.
#[allow(clippy::too_many_arguments)]
fn cpu_metabolize(
    base_drain: i64,
    exertion_drain: i64,
    exertion: i64,
    amount: i64,
    capacity: i64,
    regen_rate: i64,
) -> i64 {
    let z = Fixed::ZERO;
    let one = Fixed::ONE;
    let ex = Fixed::from_bits(exertion).clamp(z, one);
    let coupling = Fixed::from_bits(exertion_drain)
        .checked_mul(ex)
        .unwrap_or(z);
    let frac = Fixed::from_bits(base_drain).saturating_add(coupling);
    let cap = Fixed::from_bits(capacity);
    let draw = frac.checked_mul(cap).unwrap_or(z);
    let amt = Fixed::from_bits(amount);
    let regen = cpu_regen(amt, cap, Fixed::from_bits(regen_rate));
    let after_regen = amt.saturating_add(regen).clamp(z, cap);
    let drawn = if draw < z { z } else { draw };
    (after_regen - drawn).clamp(z, cap).to_bits()
}

#[test]
fn metabolize_is_bit_identical_to_the_drain_and_step() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping metabolize gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let n_axes = 3usize;
    let n_beings = 2500usize;
    let mut s = 0x5A17_0BAD_C0DE_1234u64;
    let mut nxt = || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let base_drain: Vec<i64> = (0..n_axes)
        .map(|a| Fixed::from_ratio(1 + a as i64, 300).to_bits())
        .collect();
    let exertion_drain: Vec<i64> = (0..n_axes)
        .map(|a| Fixed::from_ratio(1 + a as i64, 150).to_bits())
        .collect();
    // Exertion spans below 0 and above 1 to exercise the clamp on both ends.
    let exertion: Vec<i64> = (0..n_beings)
        .map(|b| Fixed::from_ratio((b as i64 % 7) - 2, 3).to_bits())
        .collect();
    let (mut amount, mut capacity, mut regen_rate) = (Vec::new(), Vec::new(), Vec::new());
    for b in 0..n_beings {
        for a in 0..n_axes {
            let r = nxt();
            // Capacity: sometimes 0 (dead reserve), occasionally large (stress the checked products),
            // else a small-to-moderate value.
            let cap = if (b + a) % 11 == 0 {
                Fixed::ZERO
            } else if (b + a) % 17 == 0 {
                Fixed::from_int(1 << 26)
            } else {
                Fixed::from_int(1 + (r % 4000) as i32)
            };
            // Amount: a fraction of capacity in [0, cap], including 0 and full.
            let frac_k = (r >> 10) % 11;
            let amt = cap.mul(Fixed::from_ratio(frac_k as i64, 10));
            let rate = Fixed::from_ratio(1 + ((r >> 20) % 30) as i64, 500);
            amount.push(amt.to_bits());
            capacity.push(cap.to_bits());
            regen_rate.push(rate.to_bits());
        }
    }

    let got = gpu_metabolize(
        &client,
        &base_drain,
        &exertion_drain,
        &exertion,
        &amount,
        &capacity,
        &regen_rate,
        n_axes as u32,
    );
    let n = amount.len();
    assert_eq!(got.len(), n);
    let (mut mism, mut regened, mut drained, mut zero_cap) = (0u64, 0u64, 0u64, 0u64);
    for i in 0..n {
        let being = i / n_axes;
        let axis = i % n_axes;
        let want = cpu_metabolize(
            base_drain[axis],
            exertion_drain[axis],
            exertion[being],
            amount[i],
            capacity[i],
            regen_rate[i],
        );
        if capacity[i] == 0 {
            zero_cap += 1;
        } else if want > amount[i] {
            regened += 1;
        } else if want < amount[i] {
            drained += 1;
        }
        if got[i] != want {
            mism += 1;
            if mism <= 8 {
                eprintln!(
                    "metabolize mismatch i={i} got={:#018x} want={:#018x}",
                    got[i], want
                );
            }
        }
    }
    assert!(
        regened > 0 && drained > 0 && zero_cap > 0,
        "sweep must hit regen, drain, and zero-capacity (regen={regened} drain={drained} zerocap={zero_cap})"
    );
    assert_eq!(
        mism, 0,
        "GPU metabolize must equal the CPU drain+step over all {n} reserves (regen={regened} drain={drained} zerocap={zero_cap})"
    );
}
