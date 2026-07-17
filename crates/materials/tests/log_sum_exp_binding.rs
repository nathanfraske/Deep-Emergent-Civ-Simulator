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

//! THE `log_sum_exp` BINDING TEST: step one of the coherence protocol (owner ruling 2026-07-16), executed for
//! the diamond the gate reports on every run. Two providers implement one primitive, `log(sum_i exp(x_i))`, under
//! two names in two crates: `civsim_physics::saha::log_sum_exp`, a PAIRWISE `hi + ln(1 + exp(lo - hi))` used as a
//! left fold (saha.rs:247), and `civsim_materials::creep`'s N-ARY `m + ln(sum_i exp(x_i - m))` with the shifted
//! terms summed in canonical (sorted) order, reached here through the public `creep_total_log_rate(_, Parallel)`
//! that routes straight through the private `logsumexp_canonical` (creep.rs:245). They CANNOT agree bit-for-bit:
//! the fold rounds at every step (each step its own exp, ln, add), the n-ary form rounds once, so saha's
//! order-independence is a CLAIM while creep's holds by construction. This test binds them WITHIN THE DERIVED
//! REASSOCIATION RESIDUE and moves NO bytes: it changes neither implementation, so both determinism pins hold
//! byte-exact. Step two (the delegation that unifies the two behind one provider) moves bytes and rides a
//! scheduled re-pin window with the owner's signature; it is not done here, and the diamond stays reported until
//! it lands (a scheduled arbitration is still an unarbitrated diamond).
//!
//! Sited in a materials test file, which is the ONLY legal home. `civsim_materials` depends on `civsim_physics`
//! (never the reverse), so only a materials-side test can see both providers, and `logsumexp_canonical` is
//! private, reachable solely through the public `Parallel` entry. Float here is an oracle for the derivation and
//! the kernel-error caps; it is sanctioned in a test file and reaches no canonical state (the integer-only
//! steering scan reads `physics/src/laws.rs` alone).

use civsim_core::Fixed;
use civsim_materials::creep::{creep_total_log_rate, CreepComposition};
use civsim_physics::saha::log_sum_exp;

/// The pairwise provider composed as saha composes it: a left fold of the binary `log_sum_exp`, exactly the
/// `.reduce(log_sum_exp)` at saha.rs:247. This is the fold whose per-step rounding the residue accounts for.
fn pairwise_fold(values: &[Fixed]) -> Fixed {
    values.iter().copied().reduce(log_sum_exp).unwrap()
}

/// The N-ary provider, reached through the public `Parallel` entry that dispatches to `logsumexp_canonical`.
fn nary_canonical(values: &[Fixed]) -> Fixed {
    creep_total_log_rate(values, CreepComposition::Parallel)
}

// THE PER-CALL KERNEL ERROR CAPS, in bits (ULPs), and they are DERIVED by counting the truncating fixed-point ops
// each kernel runs in `core::fixed`, NOT chosen tolerances. Every count is read off the kernel source at this
// commit, and `the_kernel_error_caps_are_valid_upper_bounds` re-confirms each cap holds by a dense sweep every
// build, so a kernel regression fails there rather than silently widening the residue below.
//
//   E_exp (arg <= 0, the only sign both paths feed it): `Fixed::exp` runs 39 truncating ops: 1 forming `k`, 1
//   forming `r`, 36 in the 18-step Maclaurin Horner (a mul and a div each), and 1 in `scale_pow2`. Because the
//   arg is <= 0, `k <= 0`, so `scale_pow2` DOWN-shifts and every Horner step multiplies by `r/i` with `|r| < ln2`,
//   so no truncation is AMPLIFIED except the two reduction truncs, which enter through `r` at sensitivity
//   `|d e^r/dr| = e^r < 2`. So E_exp <= 4 + 36 + 1 truncation-bits + a sub-ULP 18-term Maclaurin tail (`< 2^-32`
//   on `|r| < ln2`) <= 42.
//
//   E_ln (arg in [1, N]): `Fixed::ln` runs ~32 truncating ops: 1 in the mantissa reduction, 1 forming `u`, 1
//   forming `w`, 26 in the 13-step atanh Horner, 2 forming `ln_m`, 1 in `e * ln2`. `u = (m-1)/(m+1) < 1/3` and
//   `w = u^2 < 1/9`, so the Horner contracts; `u` enters the output at sensitivity `2/(1-u^2) < 2.25`. With the
//   `ln2`-constant error (`<= 0.5` bit per binary exponent, `e <= log2 N`) and a sub-ULP 13-term atanh tail, E_ln
//   <= 1 + 3 + 1 + 26 + 2 + 1 + ceil(0.5 log2 N) <= 40 for N <= 256.
//
// These are LOOSE upper bounds: the sweep below measures the true errors at ~1.3 (exp) and ~4.1 (ln) bits, an
// order of magnitude under the caps. A wide DERIVED bound is the honest price of the byte-neutral door; a narrow
// CHOSEN one would be the defect this project convicts.
const E_EXP_ULPS: i128 = 42;
const E_LN_ULPS: i128 = 40;

/// Build a fixture from integer numerators over a common denominator, so the log-domain inputs are exact `Fixed`.
fn fixture(nums: &[i64], den: i64) -> Vec<Fixed> {
    nums.iter().map(|&n| Fixed::from_ratio(n, den)).collect()
}

#[test]
fn the_two_log_sum_exp_providers_bind_within_the_derived_reassociation_residue() {
    // THE RESIDUE IS DERIVED FROM THE TWO PATHS' OP COUNTS, NEVER AN AUTHORED TOLERANCE. Both compute the exact
    // real `L = ln(sum_i exp(x_i))` from the SAME `Fixed::exp` and `Fixed::ln` kernels; they part only in how the
    // rounding accumulates. Bounding each path against `L` with `E_exp`, `E_ln` the per-call caps above:
    //
    //   n-ary (creep): N `exp` calls, summed EXACTLY (fixed-point add is integer add, associative and order-free
    //     until it saturates, which the (0,1] shifted terms never do), then ONE `ln`. The shifted sum carries
    //     `<= N*E_exp`; `ln` is 1-Lipschitz on [1, inf) (the sum is `>= exp(0) = 1`), so it neither amplifies that
    //     nor adds more than `E_ln`:  |B - L| <= N*E_exp + E_ln.
    //   pairwise fold (saha): N-1 binary steps, each `hi + ln(1 + exp(lo - hi))`, one `exp` + one `ln` per step
    //     (per-step kernel error kappa <= E_exp + E_ln, since `1 + exp(.)` is in [1,2] and `ln` is 1-Lipschitz
    //     there). `lse2(p,q) = ln(e^p + e^q)` is 1-Lipschitz in its running argument, so the step errors ADD
    //     without amplifying down the fold:  |A - L| <= (N-1)*(E_exp + E_ln).
    //   |A - B| <= |A - L| + |B - L| = (2N-1)*E_exp + N*E_ln.
    //
    // The `(2N-1)` and `N` ARE the two paths' `exp`/`ln` call counts. The bound GROWS WITH N precisely because the
    // fold rounds per step while the n-ary form rounds once, which is the diamond's own thesis: creep's reduction
    // is permutation-independent by construction, saha's is a fold that accumulates. If the two ever parted by
    // more than this, it would mean they disagree about PHYSICS rather than about rounding, a finding bigger than
    // the diamond; measured, they part by 0 to 14 bits across N = 2..16, deep inside the residue.
    //
    // BLINDNESS SET, stated beside the discriminating power and measured by mutation (see the report):
    //  - ANY DEVIATION BELOW THE RESIDUE, BY CONSTRUCTION: the residue IS the reassociation the byte-neutral door
    //    licenses, so a sub-residue mutant (a 1-ULP perturbation, a re-sort of an already-associative integer sum,
    //    a `max`-tie comparison flip) is indistinguishable from the rounding this test exists to permit. That is
    //    the price step two pays down, not a hole here.
    //  - WHETHER EITHER PATH IS RIGHT: the agreement assertion certifies the two providers AGREE, not that either
    //    matches the physics, the shared-source blindness every agreement check carries. The SECOND assertion
    //    below narrows it: each path is separately bound to the f64 oracle `L` within its own derived per-path
    //    bound, so a shared-mode drift that moved BOTH away from `L` together is caught here even though the gap
    //    between them stayed small.
    //  - ANY N OR INPUT NOT SAMPLED: it binds the two where tested. N > 2 is sampled deliberately; a two-term
    //    fixture tests the one case where a fold and an n-ary reduction do the LEAST differing.
    let scale = (1i64 << 32) as f64;

    // Fixtures chosen for mutation reach, not convenience: max NOT first (a mutant reading values[0] for the shift
    // breaks), unsorted order (a mutant folding the other way parts), negatives, all-equal (the worst
    // reassociation), a wide gap (the underflow branch), and N up to 16 (the paths differ MORE as N grows).
    //
    // WIDE-ASCENDING FIXTURES ARE IN THE SET ON PURPOSE, and mutation is why, the sibling of the strain-rate
    // test's sinking flows. With only descending or narrow inputs this test SURVIVED a mutant that DROPPED the
    // hi/lo swap in `saha::log_sum_exp` (`if a >= b {(a,b)} else {(b,a)}` -> `(a, b)`): the fold's accumulator
    // never fell far below a later element, so `lo - hi` stayed non-positive and the swap never mattered. The swap
    // exists precisely for the overflow regime, `lo - hi` large positive, where `exp` saturates without it; a case
    // like `[2, 45]` forces the fold's running value 43 below the next element, so the un-swapped path overflows
    // `exp` and parts from the n-ary form by ~22 in value. Log-domain inputs span such ranges (ln number
    // densities, log rates), so this is a convention the binding must bind, not an edge.
    let cases: Vec<Vec<Fixed>> = vec![
        fixture(&[3, 1], 1),       // N=2 (the least-differing case, kept for coverage)
        fixture(&[5, 2, 1], 1),    // N=3
        fixture(&[7, 3, 3, 1], 1), // N=4, a repeat
        fixture(&[10, 9, 8, 7, 6, 5], 1), // N=6, descending
        fixture(&[1, 1, 1, 1, 1, 1, 1, 1], 1), // N=8, all equal (worst reassociation)
        fixture(&[20, 1], 1),      // N=2, wide gap (exp underflow branch)
        fixture(&[2, 45], 1),      // N=2, wide ASCENDING (binds the hi/lo swap)
        fixture(&[1, 3, 48], 1),   // N=3, wide ascending, small accumulator then a jump
        fixture(&[6, 15, 22, 9, 30, 2, 19, 12, 27, 5], 1), // N=10, max in the interior, unsorted
        fixture(&[7, 5, 11, 3, 9, 1], 2), // N=6, fractional, unsorted, max interior
        fixture(&[0, -3, -6, -9, -12], 1), // N=5, all non-positive
        (0..16).map(|i| Fixed::from_ratio(i, 3)).collect(), // N=16
    ];

    for (idx, c) in cases.iter().enumerate() {
        let n = c.len() as i128;
        let a = pairwise_fold(c);
        let b = nary_canonical(c);

        // PRIMARY: the two providers agree within the derived reassociation residue. Integer bits, exact.
        let residue = (2 * n - 1) * E_EXP_ULPS + n * E_LN_ULPS;
        let gap = (a.to_bits() as i128 - b.to_bits() as i128).abs();
        assert!(
            gap <= residue,
            "case {idx} N={n}: the pairwise fold and the n-ary canonical logsumexp part by {gap} bits, past the \
             derived reassociation residue {residue}"
        );

        // SECONDARY: each path is within its OWN derived bound of the exact real logsumexp, computed by an f64
        // oracle OUTSIDE both integer implementations. This closes the shared-source blindness of the agreement
        // check: a defect that moved both paths off `L` together would pass the gap test but fails one of these.
        let mx = c
            .iter()
            .map(|f| f.to_f64_lossy())
            .fold(f64::NEG_INFINITY, f64::max);
        let sum_shifted: f64 = c.iter().map(|f| (f.to_f64_lossy() - mx).exp()).sum();
        let true_lse_bits = (mx + sum_shifted.ln()) * scale;
        let a_err = (a.to_bits() as f64 - true_lse_bits).abs();
        let b_err = (b.to_bits() as f64 - true_lse_bits).abs();
        let a_bound = ((n - 1) * (E_EXP_ULPS + E_LN_ULPS)) as f64;
        let b_bound = (n * E_EXP_ULPS + E_LN_ULPS) as f64;
        assert!(
            a_err <= a_bound,
            "case {idx} N={n}: the pairwise fold parts from the real logsumexp by {a_err:.2} bits, past its \
             derived per-path bound {a_bound}"
        );
        assert!(
            b_err <= b_bound,
            "case {idx} N={n}: the n-ary canonical logsumexp parts from the real logsumexp by {b_err:.2} bits, \
             past its derived per-path bound {b_bound}"
        );
    }
}

#[test]
fn the_kernel_error_caps_are_valid_upper_bounds() {
    // The residue rests on E_EXP_ULPS and E_LN_ULPS being TRUE per-call upper bounds. The op-count derivation
    // beside the constants is one proof; this sweep is the other, and it guards the caps against a kernel change:
    // it measures the real error of `Fixed::exp` and `Fixed::ln` against an f64 oracle over the exact windows the
    // two paths use (exp: arg <= 0; ln: arg in [1, N]) and asserts each stays at or under its cap. The sweep is a
    // regression guard, not the rigorous bound (the op count is, needing no sampling); it demonstrates the caps
    // are valid and conservative rather than tuned.
    let scale = (1i64 << 32) as f64;
    let step = 1i64 << 16; // 2^-16 in value; dense enough to exhibit the multi-ULP ln peak

    let mut max_exp_err = 0f64;
    let mut bits = -22i64 << 32; // exp underflows to 0 below about -22, an honest Q32.32 limit
    while bits <= 0 {
        let z = Fixed::from_bits(bits);
        let err = (z.exp().to_bits() as f64 - z.to_f64_lossy().exp() * scale).abs();
        if err > max_exp_err {
            max_exp_err = err;
        }
        bits += step;
    }
    assert!(
        max_exp_err <= E_EXP_ULPS as f64,
        "Fixed::exp error {max_exp_err:.2} bits over [-22, 0] exceeds the derived cap E_EXP_ULPS={E_EXP_ULPS}"
    );

    let mut max_ln_err = 0f64;
    let mut bits = 1i64 << 32; // y = 1
    while bits <= 64i64 << 32 {
        // covers N up to 64, above the sampled fixtures
        let y = Fixed::from_bits(bits);
        let err = (y.ln().to_bits() as f64 - y.to_f64_lossy().ln() * scale).abs();
        if err > max_ln_err {
            max_ln_err = err;
        }
        bits += step;
    }
    assert!(
        max_ln_err <= E_LN_ULPS as f64,
        "Fixed::ln error {max_ln_err:.2} bits over [1, 64] exceeds the derived cap E_LN_ULPS={E_LN_ULPS}"
    );
}
