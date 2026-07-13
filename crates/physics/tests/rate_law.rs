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

//! The numerical twin for the thermally-activated rate law (`laws::arrhenius_rate`), sited in a test FILE
//! rather than inline in `laws.rs`: a numerical-differentiation twin uses float (`to_f64_lossy`) to form the
//! central-difference slope, and the integer-only steering scan rejects any float inside the canonical
//! kernel module, test code included. The kernel PATH stays integer-only; the twin's float lives here, where
//! it is sanctioned. This is the RUNBOOK section-5 pairing of the numerical-twin rule and the no-float scan.

use civsim_core::Fixed;
use civsim_physics::laws::{arrhenius_rate, reduced_barrier};

/// The numerical-twin rule (RUNBOOK section 5): the analytic Arrhenius slope is
/// `d ln(rate)/d(1/T) = -E*/k_B`, a straight line whose slope IS the barrier. Working at a per-particle scale
/// where `k_B` folds to one, `E* = G` is the activation temperature, so the recovered slope must be `-G`.
/// Because the Arrhenius ln is EXACTLY linear in `1/T` (`ln(rate) = ln(A) - G/T`), the central difference
/// carries no truncation error, so the recovered slope equals `-G` to fixed-point rounding at EVERY step
/// size: the `h^2` plateau is the whole range, shown here with a wide and a narrow temperature pair.
#[test]
fn arrhenius_numerical_twin_recovers_minus_activation_temperature() {
    let a = Fixed::from_int(1000);
    let g = Fixed::from_int(10); // E*/k_B, the activation temperature

    // ln(rate) at a temperature, from the REAL kernel output (exp then ln, the full round trip). The float
    // read (`to_f64_lossy`) is a sanctioned boundary helper and is allowed here, in a test file.
    let ln_rate_at = |t: i32| -> f64 {
        arrhenius_rate(a, reduced_barrier(g, Fixed::from_int(t)))
            .ln()
            .to_f64_lossy()
    };
    // The central-difference slope of ln(rate) against 1/T between two temperatures.
    let slope = |t_hi: i32, t_lo: i32| -> f64 {
        let d_ln = ln_rate_at(t_hi) - ln_rate_at(t_lo);
        let d_inv_t = 1.0 / (t_hi as f64) - 1.0 / (t_lo as f64);
        d_ln / d_inv_t
    };
    // Wide pair (T = 20, 5; reduced barriers 0.5 and 2.0, both inside the exp window).
    let wide = slope(20, 5);
    // Narrow pair (T = 10, 8; reduced barriers 1.0 and 1.25).
    let narrow = slope(10, 8);
    assert!(
        (wide + 10.0).abs() < 0.05,
        "the wide-pair slope recovers -E*/k_B = -10: {wide}"
    );
    assert!(
        (narrow + 10.0).abs() < 0.05,
        "the narrow-pair slope also recovers -10 (step-size independent, the h^2 plateau): {narrow}"
    );
    // The two step sizes agree, confirming the plateau (the recovered barrier does not drift with h).
    assert!(
        (wide - narrow).abs() < 0.05,
        "the recovered slope is step-size independent: wide {wide} vs narrow {narrow}"
    );
}
