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

//! Stage 5, part 2, the quench: the freezer's OUTPUT side, turning the equilibrium assemblage into the realized
//! (frozen) assemblage as kinetics race the world's cooling rate. The core is the Dodson (1973) closure
//! temperature `T_c`, the temperature below which diffusive re-equilibration of an exchange can no longer keep
//! pace with cooling, so the composition freezes in. Gate-ruled design on #188.

use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;
const ONE: Fixed = Fixed::ONE;

/// The Dodson (1973) closure temperature `T_c` (K): the temperature at which diffusive re-equilibration of an
/// exchange reaction can no longer keep pace with cooling, so the composition freezes in (an exchange with `T_c`
/// above the current temperature is frozen and inherits, "diamond persists"). Dodson's relation is IMPLICIT in
/// `T_c`,
/// ```text
///   E*/(R*T_c) = ln( A * R * T_c^2 * D_0 / (E* * a^2 * |dT/dt|) ),
/// ```
/// solved here by a bounded FIXED-POINT iteration `T_c <- (E*/R) / (ln(g) + 2*ln(T_c))`, where `g = A*R*D_0 /
/// (E**a^2*|dT/dt|)` is the constant part of the log argument. The iteration is a contraction (its derivative
/// near the fixed point is `~2/ln(...)`, far below one, so it converges in a handful of steps); it runs under a
/// FIXED CAP with an integer `Fixed` tolerance, never an unbounded loop, so replay and worker-invariance hold
/// (the Kepler/assemblage pattern). The log is split as `ln(g*T_c^2) = ln(g) + 2*ln(T_c)` because the raw
/// product `g*T_c^2` reaches `~1e10` and overflows Q32.32, while `g` and `T_c` each fit; both operands feed the
/// built `Fixed::ln`, integer-only and pinned.
///
/// Provenance, to the gate's #188 rulings: `E*` is the exchange barrier (the freezer's Form-B barrier); `D_0`
/// is the pre-exponential diffusivity `a^2 * nu` (built on #187); the cooling rate `|dT/dt|` is READ from the
/// environment path as a `[W]` datum the world supplies, never reconstructed here and never reserved; the
/// geometry constant `A` is a MATHEMATICAL constant of the diffusion equation (55 sphere, 27 cylinder, 8.7
/// plane, a derivable-math value on a par with a tabulated Bessel zero, keyed off the phase morphology, sphere
/// the isotropic default), NOT a per-world reserved value; and `a` is the diffusion length (the exchange-length
/// scale now, to be refined to the grain size when the grain slice lands, the named inner coupling). All inputs
/// are at a consistent working scale so the log argument is dimensionless and representable (the caller folds
/// once at the cited scale, as the `T_m` and `nu` derivations do). A non-positive input, a degenerate log
/// argument (`<= 1`, no positive closure), or an overflow yields zero (no closure temperature: the exchange
/// never freezes at this precision).
#[allow(clippy::too_many_arguments)]
pub fn dodson_closure_temperature(
    barrier: Fixed,
    gas_constant: Fixed,
    d0: Fixed,
    diffusion_length: Fixed,
    cooling_rate: Fixed,
    geometry_constant: Fixed,
) -> Fixed {
    if barrier <= ZERO
        || gas_constant <= ZERO
        || d0 <= ZERO
        || diffusion_length <= ZERO
        || cooling_rate <= ZERO
        || geometry_constant <= ZERO
    {
        return ZERO;
    }
    // T_a = E*/R, the activation temperature (the numerator of the fixed point).
    let t_a = match barrier.checked_div(gas_constant) {
        Some(x) if x > ZERO => x,
        _ => return ZERO,
    };
    // g = A*R*D_0 / (E* * a^2 * |dT/dt|), the constant part of the log argument.
    let a_sq = match diffusion_length.checked_mul(diffusion_length) {
        Some(x) if x > ZERO => x,
        _ => return ZERO,
    };
    let num = geometry_constant
        .checked_mul(gas_constant)
        .and_then(|x| x.checked_mul(d0));
    let den = barrier
        .checked_mul(a_sq)
        .and_then(|x| x.checked_mul(cooling_rate));
    let g = match (num, den) {
        (Some(n), Some(d)) if d > ZERO => match n.checked_div(d) {
            Some(x) if x > ZERO => x,
            _ => return ZERO,
        },
        _ => return ZERO,
    };
    let ln_g = g.ln();
    // Initial guess: T_a / 25 (a nominal log of ~25, near the physical closure range), then iterate. The
    // contraction converges from any sane guess; the cap is a determinism backstop, not a tuning knob.
    let mut t_c = match t_a.checked_div(Fixed::from_int(25)) {
        Some(x) if x > ZERO => x,
        _ => return ZERO,
    };
    let tol = ONE; // 1 K: well below the Dodson model's own accuracy, a convergence gate not a physical claim
    for _ in 0..16 {
        if t_c <= ONE {
            return ZERO; // no positive closure temperature in range
        }
        // ln(g*T_c^2) = ln(g) + 2*ln(T_c), avoiding the overflowing product g*T_c^2.
        let ln_arg = ln_g.saturating_add(t_c.ln().checked_mul(Fixed::from_int(2)).unwrap_or(ZERO));
        if ln_arg <= ZERO {
            return ZERO; // degenerate: the exchange never closes (always re-equilibrates)
        }
        let next = match t_a.checked_div(ln_arg) {
            Some(x) if x > ZERO => x,
            _ => return ZERO,
        };
        let delta = abs_diff(next, t_c);
        t_c = next;
        if delta < tol {
            break;
        }
    }
    t_c
}

/// `|a - b|` for two `Fixed`, via the checked signed difference (saturating on the unreachable overflow).
fn abs_diff(a: Fixed, b: Fixed) -> Fixed {
    let d = a.checked_sub(b).unwrap_or(Fixed::MAX);
    if d.to_bits() >= 0 {
        d
    } else {
        ZERO.checked_sub(d).unwrap_or(Fixed::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn dodson_closure_solves_the_implicit_temperature() {
        // A hand-verified case. E* = 200 kJ/mol, R = 0.008314 kJ/(mol K) -> T_a = E*/R ~ 24056 K. Choose the
        // working-scale D_0, a, |dT/dt|, and the sphere geometry A = 55 so that g = A*R*D_0/(E**a^2*rate)
        // ~ 26000: A=55, a=2 (a^2=4), rate=1, D_0 = 45_480_000 give g ~ 26000. The fixed point
        // T_c = T_a / (ln(g) + 2 ln(T_c)) then lands near 1003 K (ln(26000) ~ 10.17, 2 ln(1003) ~ 13.82,
        // sum ~ 23.99, 24056/23.99 ~ 1003).
        let t_c = dodson_closure_temperature(
            Fixed::from_int(200),
            Fixed::from_ratio(8314, 1_000_000),
            Fixed::from_int(45_480_000),
            Fixed::from_int(2),
            ONE,
            Fixed::from_int(55),
        );
        assert!(
            close(t_c, 1003.0, 15.0),
            "the Dodson fixed point lands the implicit closure temperature ~1003 K: {t_c:?}"
        );
    }

    #[test]
    fn dodson_closure_rises_with_the_barrier_and_falls_with_faster_cooling() {
        let base = dodson_closure_temperature(
            Fixed::from_int(200),
            Fixed::from_ratio(8314, 1_000_000),
            Fixed::from_int(45_480_000),
            Fixed::from_int(2),
            ONE,
            Fixed::from_int(55),
        );
        // A higher barrier closes higher (a stiffer exchange freezes in earlier on cooling).
        let higher_barrier = dodson_closure_temperature(
            Fixed::from_int(300),
            Fixed::from_ratio(8314, 1_000_000),
            Fixed::from_int(45_480_000),
            Fixed::from_int(2),
            ONE,
            Fixed::from_int(55),
        );
        assert!(
            higher_barrier > base,
            "a higher exchange barrier closes at a higher temperature"
        );
        // Faster cooling closes higher (less time to re-equilibrate, so freezing catches the system hotter).
        let faster_cooling = dodson_closure_temperature(
            Fixed::from_int(200),
            Fixed::from_ratio(8314, 1_000_000),
            Fixed::from_int(45_480_000),
            Fixed::from_int(2),
            Fixed::from_int(1000),
            Fixed::from_int(55),
        );
        assert!(
            faster_cooling > base,
            "faster cooling closes at a higher temperature (less time to re-equilibrate)"
        );
    }

    #[test]
    fn dodson_closure_guards_and_is_deterministic() {
        let r = Fixed::from_ratio(8314, 1_000_000);
        // Non-positive inputs: no closure.
        assert_eq!(
            dodson_closure_temperature(ZERO, r, ONE, ONE, ONE, Fixed::from_int(55)),
            ZERO,
            "no barrier: no closure"
        );
        assert_eq!(
            dodson_closure_temperature(
                Fixed::from_int(200),
                r,
                ONE,
                ONE,
                ZERO,
                Fixed::from_int(55)
            ),
            ZERO,
            "no cooling rate: no closure"
        );
        // Deterministic (Principle 3): the same inputs return the same bits.
        let a = dodson_closure_temperature(
            Fixed::from_int(200),
            r,
            Fixed::from_int(45_480_000),
            Fixed::from_int(2),
            ONE,
            Fixed::from_int(55),
        );
        let b = dodson_closure_temperature(
            Fixed::from_int(200),
            r,
            Fixed::from_int(45_480_000),
            Fixed::from_int(2),
            ONE,
            Fixed::from_int(55),
        );
        assert_eq!(a, b);
    }
}
