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

//! The pinned fixed-point transcendentals (R-GPU-CANON-PIN): the integer-only CPU oracle for exp,
//! ln, sin, cos, atan, asin, and pow, sibling to Fixed::sqrt. These tests confirm the known values,
//! the inverse identities, the domain guards, monotonicity, determinism (a pure function of the
//! input, bit-identical on re-evaluation), and the achieved accuracy against a high-precision f64
//! reference (the reference is test-only; no float touches the canonical path). This is the oracle
//! every GPU backend must reproduce bit-for-bit.

use civsim_core::Fixed;

fn f(n: i64, d: i64) -> Fixed {
    Fixed::from_ratio(n, d)
}
fn near(got: Fixed, want: f64, tol: f64, msg: &str) {
    let g = got.to_f64_lossy();
    assert!(
        (g - want).abs() < tol,
        "{msg}: got {g}, want {want}, |err|={}",
        (g - want).abs()
    );
}

const TOL: f64 = 1e-7; // full Q32.32 precision is ~2.3e-10; this leaves generous headroom

#[test]
fn exp_matches_known_values_and_is_monotone() {
    near(Fixed::ZERO.exp(), 1.0, TOL, "exp(0)");
    near(Fixed::ONE.exp(), std::f64::consts::E, TOL, "exp(1)");
    near(
        Fixed::from_int(2).ln().exp(),
        2.0,
        1e-6,
        "exp(ln 2) round trip approx",
    );
    near(f(-1, 1).exp(), (-1.0f64).exp(), TOL, "exp(-1)");
    near(f(5, 1).exp(), 5.0f64.exp(), 1e-4, "exp(5)");
    assert!(f(3, 1).exp() > f(2, 1).exp(), "exp is increasing");
    // Representability window: far out saturates rather than wrapping or panicking.
    assert_eq!(
        Fixed::from_int(100).exp(),
        Fixed::MAX,
        "exp overflow saturates"
    );
    assert_eq!(
        Fixed::from_int(-100).exp(),
        Fixed::ZERO,
        "exp underflow to zero"
    );
}

#[test]
fn ln_matches_known_values_and_inverts_exp() {
    near(Fixed::ONE.ln(), 0.0, TOL, "ln(1)");
    near(Fixed::from_int(2).ln(), 2.0f64.ln(), TOL, "ln(2)");
    near(f(1, 2).ln(), 0.5f64.ln(), TOL, "ln(0.5)");
    near(Fixed::ONE.exp().ln(), 1.0, 1e-6, "ln(e) round trip");
    // ln(a*b) = ln a + ln b (the additivity Nernst and pH rely on).
    let a = f(7, 2);
    let b = f(3, 1);
    near(
        a.mul(b).ln(),
        a.ln().to_f64_lossy() + b.ln().to_f64_lossy(),
        1e-6,
        "ln additivity",
    );
    // exp(ln x) recovers x across a range.
    for &x in &[0.1, 0.5, 1.0, 2.0, 7.5, 100.0, 1000.0] {
        near(
            f((x * 1000.0) as i64, 1000).ln().exp(),
            x,
            x.max(1.0) * 1e-6,
            "exp(ln x)",
        );
    }
    // Non-positive input is the fail-loud sentinel.
    assert_eq!(Fixed::ZERO.ln(), Fixed::MIN, "ln(0) sentinel");
    assert_eq!(f(-3, 1).ln(), Fixed::MIN, "ln(negative) sentinel");
}

#[test]
fn sin_cos_match_known_values_and_the_pythagorean_identity() {
    near(Fixed::ZERO.sin(), 0.0, TOL, "sin 0");
    near(Fixed::ZERO.cos(), 1.0, TOL, "cos 0");
    near(Fixed::HALF_PI.sin(), 1.0, 1e-6, "sin pi/2");
    near(Fixed::HALF_PI.cos(), 0.0, 1e-6, "cos pi/2");
    near(Fixed::PI.sin(), 0.0, 1e-6, "sin pi");
    near(Fixed::PI.cos(), -1.0, 1e-6, "cos pi");
    near(
        (Fixed::PI.div(Fixed::from_int(6))).sin(),
        0.5,
        1e-6,
        "sin pi/6 = 1/2",
    );
    near(
        (Fixed::PI.div(Fixed::from_int(3))).cos(),
        0.5,
        1e-6,
        "cos pi/3 = 1/2",
    );
    // sin^2 + cos^2 = 1 across a sweep, including several full turns and negatives.
    for i in -20..=20 {
        let theta = f(i, 3); // steps of 1/3 rad
        let (s, c) = theta.sin_cos();
        let id = s.mul(s).to_f64_lossy() + c.mul(c).to_f64_lossy();
        assert!((id - 1.0).abs() < 1e-5, "sin^2+cos^2=1 at {i}/3: {id}");
    }
}

#[test]
fn atan_and_asin_match_known_values_and_gate_the_domain() {
    near(Fixed::ZERO.atan(), 0.0, TOL, "atan 0");
    near(
        Fixed::ONE.atan(),
        std::f64::consts::FRAC_PI_4,
        1e-6,
        "atan 1 = pi/4",
    );
    near(
        f(-1, 1).atan(),
        -std::f64::consts::FRAC_PI_4,
        1e-6,
        "atan -1",
    );
    near(
        Fixed::from_int(1000).atan(),
        std::f64::consts::FRAC_PI_2,
        1e-3,
        "atan large -> pi/2",
    );
    near(Fixed::ZERO.asin(), 0.0, TOL, "asin 0");
    near(f(1, 2).asin(), (0.5f64).asin(), 1e-5, "asin 1/2 = pi/6");
    near(
        Fixed::ONE.asin(),
        std::f64::consts::FRAC_PI_2,
        1e-6,
        "asin 1 = pi/2",
    );
    near(
        f(-1, 1).asin(),
        -std::f64::consts::FRAC_PI_2,
        1e-6,
        "asin -1",
    );
    // Total-internal-reflection boundary: outside [-1,1] saturates to the right angle.
    assert_eq!(f(3, 2).asin(), Fixed::HALF_PI, "asin past 1 saturates");
    // asin is the inverse of sin over the principal range.
    for i in -5..=5 {
        let x = f(i, 6); // in [-5/6, 5/6]
        near(x.asin().sin(), x.to_f64_lossy(), 1e-5, "sin(asin x) = x");
    }
}

#[test]
fn pow_integer_and_real() {
    assert_eq!(
        Fixed::from_int(2).powi(10),
        Fixed::from_int(1024),
        "2^10 exact"
    );
    near(Fixed::from_int(2).powi(-2), 0.25, TOL, "2^-2");
    assert_eq!(f(5, 1).powi(0), Fixed::ONE, "x^0 = 1");
    near(
        Fixed::from_int(2).powf(f(1, 2)),
        2.0f64.sqrt(),
        1e-6,
        "2^0.5 = sqrt 2",
    );
    near(Fixed::from_int(8).powf(f(1, 3)), 2.0, 1e-5, "8^(1/3) = 2");
    near(Fixed::from_int(10).powf(f(2, 1)), 100.0, 1e-3, "10^2 = 100");
}

#[test]
fn the_transcendentals_are_deterministic() {
    // A pure function of the input: re-evaluation is bit-identical, the property the GPU oracle
    // contract stands on.
    let xs = [
        f(1, 7),
        f(22, 10),
        Fixed::PI,
        f(-13, 10),
        Fixed::from_int(5),
    ];
    for x in xs {
        assert_eq!(x.exp(), x.exp());
        assert_eq!(x.sin_cos(), x.sin_cos());
        assert_eq!(x.atan(), x.atan());
        if x > Fixed::ZERO {
            assert_eq!(x.ln(), x.ln());
        }
    }
}

#[test]
fn the_deferred_physics_transcendentals_now_compose() {
    // Spot-check that the shapes the deferred wave-2/3 laws need are now expressible in canon.
    // Beer-Lambert transmitted fraction exp(-tau).
    near(
        f(12, 10).exp().div(Fixed::ONE).ln(),
        1.2,
        1e-6,
        "ln . exp identity for Beer-Lambert path",
    );
    let tau = f(7, 10);
    near(
        (Fixed::ZERO - tau).exp(),
        (-0.7f64).exp(),
        1e-6,
        "exp(-tau) transmittance",
    );
    // Nernst-style ln of a ratio.
    near(f(50, 1).ln(), 50.0f64.ln(), 1e-6, "ln(Q)");
    // Snell asin of a ratio, with the TIR gate.
    assert_eq!(
        f(15, 10).asin(),
        Fixed::HALF_PI,
        "Snell TIR gate past critical angle"
    );
}
