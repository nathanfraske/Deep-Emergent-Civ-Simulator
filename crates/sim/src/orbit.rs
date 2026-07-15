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

//! The orbital-mechanics geometry: where a body actually sits on its orbit at a moment in time
//! (design Parts 14.6, 32, the "orbits arc", task #78). The Kepler PERIOD is derived by
//! [`crate::astro::kepler_orbital_period_seconds`]; this module is the PHASE, the analytic position
//! along that orbit, which the sun-direction lighting and the projecting camera read.
//!
//! The position is ANALYTIC, not stepped: given the orbit's shape (eccentricity) and how far through
//! the period the body is (the mean anomaly, a phase), it solves Kepler's equation `M = E - e*sin(E)`
//! for the eccentric anomaly `E` and reads the position off it in closed form. There is no per-orbit
//! integration and no accumulated dt, so a body's position is exact at any time and never drifts. This
//! is the design the deep-time timestepping calls for: the fast orbital phase is computed on demand
//! (this module), while only the slow secular drift of the orbital elements is stepped (task #44). The
//! mean anomaly is unit-free (it is `2*pi` times the fraction of the period elapsed), so this phase
//! computation is independent of whether the period is carried in seconds, years, or reference units.

use civsim_core::Fixed;

/// The Newton-iteration count for the Kepler-equation solve. Kepler's equation `M = E - e*sin(E)` has
/// no closed form, so `E` is found by Newton's method, which converges quadratically for the bound
/// (`e < 1`) case. A fixed count (never a convergence-dependent early exit) keeps the solve
/// deterministic and bit-identical on every machine, an engine-mechanics constant (Principle 11
/// exemption), not a physical value. Twenty steps reach full Q32.32 precision for eccentricities up to
/// the ~0.9 range; the near-parabolic limit (`e` approaching 1) converges more slowly and is the
/// high-eccentricity follow-on (a starting-guess refinement), flagged not faked.
const KEPLER_NEWTON_STEPS: u32 = 20;

/// A body's position on its orbit at one moment, all lengths in units of the semi-major axis `a` so the
/// geometry is scale-free (multiply by the derived `a` for metres). The frame is perifocal: the star
/// sits at the origin (the focus), the `x` axis points toward perihelion, the `y` axis along the
/// semi-minor axis in the direction of motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrbitalState {
    /// The eccentric anomaly `E` (radians), the solved auxiliary angle.
    pub eccentric_anomaly: Fixed,
    /// The true anomaly `nu` (radians), the actual angular position measured from perihelion at the
    /// star. This is the angle a viewer at the star would see the body swing through.
    pub true_anomaly: Fixed,
    /// The star-to-body distance in units of `a`: `r/a = 1 - e*cos(E)`, ranging from `1 - e` at
    /// perihelion to `1 + e` at aphelion.
    pub radius_over_semimajor: Fixed,
    /// The body's position along the perihelion axis, in units of `a`: `cos(E) - e`.
    pub position_x_over_a: Fixed,
    /// The body's position along the semi-minor axis, in units of `a`: `sqrt(1 - e^2)*sin(E)`.
    pub position_y_over_a: Fixed,
}

/// Solve Kepler's equation `M = E - e*sin(E)` for the eccentric anomaly `E`, by Newton's method from
/// the standard starting guess `E0 = M + e*sin(M)`. `mean_anomaly` is `M` in radians (`2*pi` times the
/// fraction of the period elapsed since perihelion); `eccentricity` is `e`. `None` on `e < 0` or
/// `e >= 1` (an unbound or parabolic orbit is out of scope here), the honest bound-orbit limit.
pub fn solve_eccentric_anomaly(mean_anomaly: Fixed, eccentricity: Fixed) -> Option<Fixed> {
    if eccentricity < Fixed::ZERO || eccentricity >= Fixed::ONE {
        return None;
    }
    let (sin_m, _) = mean_anomaly.sin_cos();
    // E0 = M + e*sin(M), the guess that keeps Newton in the right branch.
    let mut ecc = mean_anomaly.checked_add(eccentricity.checked_mul(sin_m)?)?;
    for _ in 0..KEPLER_NEWTON_STEPS {
        let (sin_e, cos_e) = ecc.sin_cos();
        // f(E) = E - e*sin(E) - M, f'(E) = 1 - e*cos(E). For e < 1, f' >= 1 - e > 0, so the step is
        // always well-defined and the fixed iteration count never divides by zero.
        let f = ecc
            .checked_sub(eccentricity.checked_mul(sin_e)?)?
            .checked_sub(mean_anomaly)?;
        let f_prime = Fixed::ONE.checked_sub(eccentricity.checked_mul(cos_e)?)?;
        ecc = ecc.checked_sub(f.checked_div(f_prime)?)?;
    }
    Some(ecc)
}

/// The full orbital position at a moment: solve Kepler's equation for `E`, then read the perifocal
/// position, the star distance, and the true anomaly off it in closed form. `mean_anomaly` is `M`
/// (radians), `eccentricity` is `e`. `None` on an unbound orbit (`e` outside `[0, 1)`) or an
/// intermediate past the representable range.
pub fn orbital_state(mean_anomaly: Fixed, eccentricity: Fixed) -> Option<OrbitalState> {
    let eccentric_anomaly = solve_eccentric_anomaly(mean_anomaly, eccentricity)?;
    let (sin_e, cos_e) = eccentric_anomaly.sin_cos();
    // The perifocal position in units of a, focus (star) at the origin.
    let position_x_over_a = cos_e.checked_sub(eccentricity)?;
    let one_minus_e2 = Fixed::ONE.checked_sub(eccentricity.checked_mul(eccentricity)?)?;
    let position_y_over_a = one_minus_e2.sqrt().checked_mul(sin_e)?;
    // r/a = 1 - e*cos(E), equal to sqrt(x^2 + y^2) by construction.
    let radius_over_semimajor = Fixed::ONE.checked_sub(eccentricity.checked_mul(cos_e)?)?;
    let true_anomaly = atan2(position_y_over_a, position_x_over_a)?;
    Some(OrbitalState {
        eccentric_anomaly,
        true_anomaly,
        radius_over_semimajor,
        position_x_over_a,
        position_y_over_a,
    })
}

/// The full-circle arctangent `atan2(y, x)` in `[-pi, pi]`, built from the single-argument [`Fixed::atan`]
/// (which returns `[-pi/2, pi/2]`) plus the quadrant correction from the signs of `x` and `y`. `None`
/// only on an intermediate past the representable range.
fn atan2(y: Fixed, x: Fixed) -> Option<Fixed> {
    let pi = Fixed::PI;
    if x > Fixed::ZERO {
        Some(y.checked_div(x)?.atan())
    } else if x < Fixed::ZERO {
        let base = y.checked_div(x)?.atan();
        if y >= Fixed::ZERO {
            base.checked_add(pi)
        } else {
            base.checked_sub(pi)
        }
    } else if y > Fixed::ZERO {
        pi.checked_div(Fixed::from_int(2))
    } else if y < Fixed::ZERO {
        Fixed::ZERO.checked_sub(pi.checked_div(Fixed::from_int(2))?)
    } else {
        Some(Fixed::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tau() -> Fixed {
        Fixed::PI.checked_add(Fixed::PI).unwrap()
    }

    #[test]
    fn a_circular_orbit_has_the_eccentric_anomaly_equal_the_mean() {
        // At e = 0 Kepler's equation is M = E, so E = nu = M and the body rides a unit circle (r/a = 1).
        for &frac in &[1i64, 3, 5, 7] {
            let m = tau().checked_mul(Fixed::from_ratio(frac, 8)).unwrap();
            let s = orbital_state(m, Fixed::ZERO).unwrap();
            assert!(
                (s.eccentric_anomaly.to_f64_lossy() - m.to_f64_lossy()).abs() < 1e-4,
                "e=0: E equals M, got E={} M={}",
                s.eccentric_anomaly.to_f64_lossy(),
                m.to_f64_lossy()
            );
            assert!(
                (s.radius_over_semimajor.to_f64_lossy() - 1.0).abs() < 1e-4,
                "e=0: r/a is one, got {}",
                s.radius_over_semimajor.to_f64_lossy()
            );
            // For e=0 the true anomaly equals M modulo 2*pi, but atan2 returns the principal value in
            // [-pi, pi], so reduce M to that branch before comparing.
            let two_pi = 2.0 * std::f64::consts::PI;
            let m_principal = if m.to_f64_lossy() > std::f64::consts::PI {
                m.to_f64_lossy() - two_pi
            } else {
                m.to_f64_lossy()
            };
            assert!(
                (s.true_anomaly.to_f64_lossy() - m_principal).abs() < 1e-4,
                "e=0: true anomaly equals M (principal branch), got {} expected {}",
                s.true_anomaly.to_f64_lossy(),
                m_principal
            );
        }
    }

    #[test]
    fn perihelion_and_aphelion_are_the_closest_and_farthest_points() {
        let e = Fixed::from_ratio(3, 10); // e = 0.3
                                          // Perihelion at M = 0: E = 0, nu = 0, r/a = 1 - e, on the perihelion axis.
        let peri = orbital_state(Fixed::ZERO, e).unwrap();
        assert!(
            (peri.eccentric_anomaly.to_f64_lossy()).abs() < 1e-4,
            "E=0 at perihelion"
        );
        assert!(
            (peri.true_anomaly.to_f64_lossy()).abs() < 1e-4,
            "nu=0 at perihelion"
        );
        assert!(
            (peri.radius_over_semimajor.to_f64_lossy() - 0.7).abs() < 1e-4,
            "r/a = 1-e = 0.7 at perihelion, got {}",
            peri.radius_over_semimajor.to_f64_lossy()
        );
        // Aphelion at M = pi: E = pi, nu = pi, r/a = 1 + e.
        let apo = orbital_state(Fixed::PI, e).unwrap();
        assert!(
            (apo.true_anomaly.to_f64_lossy().abs() - std::f64::consts::PI).abs() < 1e-3,
            "nu = pi at aphelion, got {}",
            apo.true_anomaly.to_f64_lossy()
        );
        assert!(
            (apo.radius_over_semimajor.to_f64_lossy() - 1.3).abs() < 1e-4,
            "r/a = 1+e = 1.3 at aphelion, got {}",
            apo.radius_over_semimajor.to_f64_lossy()
        );
    }

    #[test]
    fn the_kepler_equation_residual_is_solved() {
        // The defining check: the solved E must satisfy M = E - e*sin(E) to fixed-point precision, for a
        // genuinely eccentric case at an off-axis phase.
        let e = Fixed::from_ratio(45, 100); // e = 0.45
        let m = Fixed::from_ratio(11, 10); // M = 1.1 rad
        let ecc = solve_eccentric_anomaly(m, e).unwrap();
        let (sin_e, _) = ecc.sin_cos();
        let residual =
            ecc.to_f64_lossy() - e.to_f64_lossy() * sin_e.to_f64_lossy() - m.to_f64_lossy();
        assert!(
            residual.abs() < 1e-5,
            "Kepler residual near zero, got {residual}"
        );
    }

    #[test]
    fn the_position_magnitude_matches_the_radius() {
        // A closed geometric invariant, independent of the solve: x^2 + y^2 must equal (r/a)^2 exactly by
        // construction (both equal (1 - e*cos E)^2), so this catches any position or radius slip.
        let e = Fixed::from_ratio(6, 10); // e = 0.6
        let m = Fixed::from_ratio(23, 10); // M = 2.3 rad
        let s = orbital_state(m, e).unwrap();
        let mag2 =
            s.position_x_over_a.to_f64_lossy().powi(2) + s.position_y_over_a.to_f64_lossy().powi(2);
        let r2 = s.radius_over_semimajor.to_f64_lossy().powi(2);
        assert!(
            (mag2 - r2).abs() < 1e-4,
            "|position|^2 equals (r/a)^2, got {mag2} vs {r2}"
        );
    }

    #[test]
    fn the_solve_is_deterministic_and_fails_loud_on_unbound_orbits() {
        let e = Fixed::from_ratio(2, 10);
        let m = Fixed::from_ratio(9, 10);
        assert_eq!(
            orbital_state(m, e),
            orbital_state(m, e),
            "same inputs, same state"
        );
        // Parabolic and hyperbolic and negative eccentricities are out of the bound-orbit scope.
        assert!(
            orbital_state(m, Fixed::ONE).is_none(),
            "e = 1 (parabolic) fails loud"
        );
        assert!(
            orbital_state(m, Fixed::from_ratio(15, 10)).is_none(),
            "e > 1 (hyperbolic) fails loud"
        );
        assert!(
            orbital_state(
                m,
                Fixed::ZERO.checked_sub(Fixed::from_ratio(1, 10)).unwrap()
            )
            .is_none(),
            "e < 0 fails loud"
        );
    }
}
