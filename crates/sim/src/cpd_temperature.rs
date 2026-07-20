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

//! The COUPLED CPD surface-and-midplane TEMPERATURE solve: the layer that turns the heat fluxes and the surface
//! density into the midplane temperature at which the satellite-forming condensation runs. This is the rung the
//! moon-composition slice descended to, and it is where the derive-first pipeline meets a genuine implicit
//! coupling (the blocking review's P1): the temperature sets the opacity, and the opacity sets the temperature.
//!
//! THE COUPLING, and why it is not two one-way modules. The grey vertical structure (Makalkin and Dorofeeva 1995,
//! as reproduced by Schneeberger and Mousis 2025) is
//!
//! - surface:  `T_s^4 = (1 + 1/(2 kappa_p Sigma_g)) F_total/sigma + T_neb^4`   (their Eq. 18)
//! - midplane: `T_m^4 = T_s^4 + (3/8) tau_R F_vis/sigma`, `tau_R = kappa_R Sigma_g / 2`  (Eq. 24, grey-integrated)
//!
//! with `F_total = F_vis + F_acc + F_p` the three per-face heat fluxes, `Sigma_g` the surface density, `kappa_p`
//! the Planck-mean and `kappa_R` the Rosseland-mean opacity. Because `kappa_p` and `kappa_R` are functions of the
//! very temperatures they set (`kappa_p(T_s)`, `kappa_R(T_m)`), this is an implicit fixed point, not a closed form.
//!
//! HOW IT IS SOLVED, and why not JFNK. Schneeberger and Mousis solve it with a Jacobian-Free Newton-Krylov
//! iteration; the review's ruling was NOT to copy that merely because the paper used it. This module solves the
//! fixed point by a DETERMINISTIC bounded Picard iteration: seed at the effective temperature, then alternately
//! update the surface and midplane temperatures with the opacity re-evaluated at each, until the midplane
//! temperature settles within a caller tolerance, and REFUSE (return `None`) if it does not settle within a caller
//! iteration bound. Fixed-point arithmetic throughout; the wide-magnitude `T^4` and `F/sigma` quantities are
//! carried as base-ten logs with a stable log-sum-exp for the two-term sums, so a 100 K cold trap and a 4500 K
//! inner disc are the same representable arithmetic.
//!
//! THE OPACITY IS A CLOSURE, not a scalar lookup (the review's other P1). The Planck and Rosseland opacities enter
//! as closures `kappa(T)`, exactly the boundary the engine's own Rosseland assembly
//! ([`civsim_physics::opacity::total_gas_and_grain_rosseland_opacity`]) uses to keep the grain term
//! composition-agnostic and off the materials dependency cycle. In the live wire the caller supplies that
//! assembly, evaluated over the realized condensate population (the materials proposer-disposer-freezer), so the
//! inner material/opacity fixed point the review asked for lives in the iteration and the opacity cliff at the ice
//! line is an emergent input, never a coded `kappa(T)` regime boundary. This module stays a clean primitive that
//! consumes the opacity as a function, testable against analytic constant- and power-law-opacity cases.
//!
//! DORMANT: no run-path caller (the composition consumer and assembly wiring are gated follow-ons), so the two run
//! pins hold bit-exact. ADMITS THE ALIEN: every input (the fluxes, the surface density, the nebular temperature,
//! the opacity laws) is an argument, so an alien disc chemistry is a different opacity closure, never a rewrite.

use civsim_core::Fixed;

/// The optical-depth regime the converged solve sits in, typed so a consumer reads the branch rather than
/// re-deriving it, and so a degenerate surface is named rather than reported as a plausible temperature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpdThermalBranch {
    /// The midplane is optically thick to its own thermal radiation (`tau_R > 1`): viscous heat is trapped, so the
    /// midplane runs hotter than the surface. The satellite-forming regime of the inner disc.
    OpticallyThick,
    /// The midplane is optically thin (`tau_R <= 1`) but a photosurface is still defined: the midplane temperature
    /// is close to the surface temperature, the irradiation-dominated outer disc.
    OpticallyThin,
    /// No well-defined photosurface (`tau_p < 2/3`): the column is too tenuous to reprocess the flux through a
    /// surface, so the surface relation is a limiting form, not a photosphere. Named, not silently trusted.
    NoPhotosurface,
}

/// The converged CPD vertical temperature structure: the surface and midplane temperatures, the midplane optical
/// depth, the regime branch, and the iteration count. Produced by [`solve_cpd_midplane_temperature`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpdMidplaneTemperature {
    /// The surface (photosurface) temperature in kelvin.
    pub surface_k: Fixed,
    /// The midplane temperature in kelvin, the temperature the satellite-forming condensation runs at.
    pub midplane_k: Fixed,
    /// The Rosseland midplane optical depth `tau_R = kappa_R Sigma_g / 2` at convergence.
    pub midplane_optical_depth: Fixed,
    /// The optical-depth regime the converged solve sits in.
    pub branch: CpdThermalBranch,
    /// The number of Picard iterations taken to converge (a diagnostic, and a witness the solve is bounded).
    pub iterations: u32,
}

/// `log10(10^a + 10^b)`, the stable two-term log-sum-exp: the sum is formed around the larger term so the smaller
/// enters as `10^(-|a-b|) <= 1`, never overflowing. This is how the two `T^4` additions (the surface heat plus the
/// nebular floor, and the midplane's viscous term on the surface value) are taken without exponentiating a wide
/// magnitude.
fn log10_add(a: Fixed, b: Fixed) -> Option<Fixed> {
    let ln10 = Fixed::from_int(10).ln();
    let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
    let diff = lo.checked_sub(hi)?; // <= 0
    let ten_pow = diff.checked_mul(ln10)?.exp(); // 10^diff in (0, 1]
    let one_plus = Fixed::from_int(1).checked_add(ten_pow)?;
    hi.checked_add(one_plus.ln().checked_div(ln10)?)
}

/// Solve the coupled CPD surface-and-midplane temperature at one radius by a deterministic bounded Picard
/// iteration over the opacity fixed point.
///
/// Inputs: the three per-face heat fluxes `flux_vis_wm2`, `flux_acc_wm2`, `flux_planet_wm2` (W/m^2, non-negative,
/// at least one positive); `surface_density_g_cm2` the gas surface density `Sigma_g` in g/cm^2 (the cgs unit the
/// opacity closures return `kappa` against); `nebular_temperature_k` the ambient nebular floor `T_neb` (K,
/// non-negative); `planck_opacity_at` and `rosseland_opacity_at` the opacity closures `kappa(T)` in cm^2/g (the
/// engine's Rosseland assembly in the live wire); `max_iterations` the Picard bound (a performance bound, reserved
/// with basis at the call site); `tolerance_k` the midplane convergence tolerance in K (reserved with basis, the
/// temperature resolution below which condensation is unaffected).
///
/// Returns the converged [`CpdMidplaneTemperature`], or `None` on a degenerate input, an opacity closure that
/// fails or returns a non-positive value, an overflow, or a failure to converge within `max_iterations` (the
/// refusal that names an unresolved branch topology rather than reporting a partial iterate).
#[allow(clippy::too_many_arguments)]
pub fn solve_cpd_midplane_temperature(
    flux_vis_wm2: Fixed,
    flux_acc_wm2: Fixed,
    flux_planet_wm2: Fixed,
    surface_density_g_cm2: Fixed,
    nebular_temperature_k: Fixed,
    planck_opacity_at: impl Fn(Fixed) -> Option<Fixed>,
    rosseland_opacity_at: impl Fn(Fixed) -> Option<Fixed>,
    max_iterations: u32,
    tolerance_k: Fixed,
) -> Option<CpdMidplaneTemperature> {
    if flux_vis_wm2 < Fixed::ZERO
        || flux_acc_wm2 < Fixed::ZERO
        || flux_planet_wm2 < Fixed::ZERO
        || surface_density_g_cm2 <= Fixed::ZERO
        || nebular_temperature_k < Fixed::ZERO
        || max_iterations == 0
        || tolerance_k <= Fixed::ZERO
    {
        return None;
    }
    let flux_total = flux_vis_wm2
        .checked_add(flux_acc_wm2)?
        .checked_add(flux_planet_wm2)?;
    if flux_total <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10 = |x: Fixed| -> Option<Fixed> {
        if x <= Fixed::ZERO {
            return None;
        }
        x.ln().checked_div(ln10)
    };
    let pow10 = |y: Fixed| -> Option<Fixed> { Some(y.checked_mul(ln10)?.exp()) };
    // Read the Stefan-Boltzmann constant from the units floor through the sanctioned decimal-log helper (the same
    // path astro reads its anchors through), never an inline decimal parse: log10(sigma) = ln(sigma) / ln 10.
    let log10_sigma_sb =
        civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::STEFAN_BOLTZMANN.value)?
            .checked_div(ln10)?;
    // log10(F/sigma) for the total and (if present) the viscous flux; these are the T^4-scale drivers, in K^4.
    let lf_total = log10(flux_total)?.checked_sub(log10_sigma_sb)?;
    let lf_vis = if flux_vis_wm2 > Fixed::ZERO {
        Some(log10(flux_vis_wm2)?.checked_sub(log10_sigma_sb)?)
    } else {
        None
    };
    let t_neb4_log = if nebular_temperature_k > Fixed::ZERO {
        Some(Fixed::from_int(4).checked_mul(log10(nebular_temperature_k)?)?)
    } else {
        None
    };
    let log10_3_8 = log10(Fixed::from_ratio(3, 8))?;
    let two_thirds = Fixed::from_ratio(2, 3);

    // Seed both temperatures at the effective temperature T_eff = (F_total/sigma)^(1/4).
    let mut t_s = pow10(lf_total.checked_div(Fixed::from_int(4))?)?;
    let mut t_m = t_s;
    let mut last_tau_r = Fixed::ZERO;
    let mut last_tau_p = Fixed::ZERO;
    let mut converged = false;
    let mut iterations = 0u32;

    for i in 1..=max_iterations {
        iterations = i;
        // Surface: T_s^4 = (1 + 1/(2 tau_p)) F_total/sigma + T_neb^4, with kappa_p at the current surface T.
        let kappa_p = planck_opacity_at(t_s)?;
        if kappa_p <= Fixed::ZERO {
            return None;
        }
        let tau_p = kappa_p.checked_mul(surface_density_g_cm2)?;
        last_tau_p = tau_p;
        let inv = Fixed::from_int(1).checked_div(Fixed::from_int(2).checked_mul(tau_p)?)?;
        let factor = Fixed::from_int(1).checked_add(inv)?;
        let log10_ts4_heat = lf_total.checked_add(log10(factor)?)?;
        let log10_ts4 = match t_neb4_log {
            Some(fl) => log10_add(log10_ts4_heat, fl)?,
            None => log10_ts4_heat,
        };
        t_s = pow10(log10_ts4.checked_div(Fixed::from_int(4))?)?;

        // Midplane: T_m^4 = T_s^4 + (3/8) tau_R F_vis/sigma, with kappa_R at the current midplane T.
        let kappa_r = rosseland_opacity_at(t_m)?;
        if kappa_r <= Fixed::ZERO {
            return None;
        }
        let tau_r = kappa_r
            .checked_mul(surface_density_g_cm2)?
            .checked_div(Fixed::from_int(2))?;
        last_tau_r = tau_r;
        let log10_tm4 = match lf_vis {
            Some(fv) => {
                let c_log = log10_3_8.checked_add(log10(tau_r)?)?.checked_add(fv)?;
                log10_add(log10_ts4, c_log)?
            }
            None => log10_ts4,
        };
        let t_m_new = pow10(log10_tm4.checked_div(Fixed::from_int(4))?)?;

        let delta = if t_m_new >= t_m {
            t_m_new.checked_sub(t_m)?
        } else {
            t_m.checked_sub(t_m_new)?
        };
        t_m = t_m_new;
        if delta < tolerance_k {
            converged = true;
            break;
        }
    }
    if !converged {
        return None;
    }
    let branch = if last_tau_p < two_thirds {
        CpdThermalBranch::NoPhotosurface
    } else if last_tau_r > Fixed::from_int(1) {
        CpdThermalBranch::OpticallyThick
    } else {
        CpdThermalBranch::OpticallyThin
    };
    Some(CpdMidplaneTemperature {
        surface_k: t_s,
        midplane_k: t_m,
        midplane_optical_depth: last_tau_r,
        branch,
        iterations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    // The independent f64 reference for the CONSTANT-opacity fixed point (which has a closed form because the
    // opacity does not move with T): T_s^4 = (1 + 1/(2 tau_p)) F_total/sigma + T_neb^4,
    // T_m^4 = T_s^4 + (3/8) tau_R F_vis/sigma, with tau_p = k Sigma, tau_R = k Sigma/2.
    fn constant_opacity_ref(
        f_vis: f64,
        f_acc: f64,
        f_p: f64,
        sigma_g: f64,
        t_neb: f64,
        kappa: f64,
    ) -> (f64, f64) {
        let sigma_sb = 5.670374419e-8;
        let f_total = f_vis + f_acc + f_p;
        let tau_p = kappa * sigma_g;
        let tau_r = kappa * sigma_g / 2.0;
        let ts4 = (1.0 + 1.0 / (2.0 * tau_p)) * f_total / sigma_sb + t_neb.powi(4);
        let tm4 = ts4 + (3.0 / 8.0) * tau_r * f_vis / sigma_sb;
        (ts4.powf(0.25), tm4.powf(0.25))
    }

    // A constant opacity: the fixed point is reached in one step, and the fixed-point solve must reproduce the
    // closed-form surface and midplane temperatures. Optically thick (tau_R > 1), so the midplane runs hotter.
    #[test]
    fn constant_opacity_matches_the_closed_form() {
        let (f_vis, f_acc, f_p, sigma_g, t_neb, kappa) = (100.0, 0.0, 0.0, 50.0, 20.0, 1.0);
        let (ts_ref, tm_ref) = constant_opacity_ref(f_vis, f_acc, f_p, sigma_g, t_neb, kappa);
        let out = solve_cpd_midplane_temperature(
            r((f_vis * 100.0) as i64, 100),
            Fixed::ZERO,
            Fixed::ZERO,
            r((sigma_g * 100.0) as i64, 100),
            r((t_neb * 100.0) as i64, 100),
            |_t| Some(r((kappa * 1000.0) as i64, 1000)),
            |_t| Some(r((kappa * 1000.0) as i64, 1000)),
            50,
            r(1, 100),
        )
        .expect("the constant-opacity solve converges");
        assert!(
            (out.surface_k.to_f64_lossy() - ts_ref).abs() < 1.0,
            "surface {} vs ref {}",
            out.surface_k.to_f64_lossy(),
            ts_ref
        );
        assert!(
            (out.midplane_k.to_f64_lossy() - tm_ref).abs() < 1.0,
            "midplane {} vs ref {}",
            out.midplane_k.to_f64_lossy(),
            tm_ref
        );
        // tau_R = 1.0 * 50 / 2 = 25 > 1: optically thick, midplane hotter than surface.
        assert_eq!(out.branch, CpdThermalBranch::OpticallyThick);
        assert!(out.midplane_k > out.surface_k);
    }

    // A temperature-DEPENDENT opacity (a Pollack-like power law kappa = kappa0 (T/100)^beta) exercises the real
    // fixed point: the solve must still converge and land on the self-consistent temperature, cross-checked by
    // substituting the converged T back into the closed form with kappa(T).
    #[test]
    fn power_law_opacity_reaches_a_self_consistent_fixed_point() {
        let kappa0 = 0.5_f64;
        let beta = 1.0_f64; // kappa grows with T
        let kappa_law = move |t: Fixed| -> Option<Fixed> {
            let tf = t.to_f64_lossy();
            let k = kappa0 * (tf / 100.0).powf(beta);
            Some(r((k * 100_000.0) as i64, 100_000))
        };
        let out = solve_cpd_midplane_temperature(
            r(50000, 100), // 500 W/m^2 viscous
            Fixed::ZERO,
            Fixed::ZERO,
            r(2000, 100), // 20 g/cm^2
            r(1000, 100), // 10 K nebular floor
            kappa_law,
            kappa_law,
            80,
            r(1, 100),
        )
        .expect("the power-law solve converges");
        // Self-consistency: at the converged T_m, tau_R and the midplane relation reproduce T_m.
        let tm = out.midplane_k.to_f64_lossy();
        let ts = out.surface_k.to_f64_lossy();
        let kappa_tm = 0.5 * (tm / 100.0).powf(beta);
        let tau_r = kappa_tm * 20.0 / 2.0;
        let tm4_check = ts.powi(4) + (3.0 / 8.0) * tau_r * 500.0 / 5.670374419e-8;
        assert!(
            (tm - tm4_check.powf(0.25)).abs() < 2.0,
            "converged midplane {} not self-consistent with {}",
            tm,
            tm4_check.powf(0.25)
        );
    }

    // Irradiation-dominated: with no viscous heating and a planet-irradiation flux only, the midplane equals the
    // surface (no trapped internal heat), the outer-disc limit.
    #[test]
    fn irradiation_only_gives_midplane_equal_to_surface() {
        let out = solve_cpd_midplane_temperature(
            Fixed::ZERO,
            Fixed::ZERO,
            r(1000, 100), // 10 W/m^2 planet irradiation
            r(500, 100),  // 5 g/cm^2
            Fixed::ZERO,
            |_t| Some(r(1, 10)),
            |_t| Some(r(1, 10)),
            50,
            r(1, 100),
        )
        .expect("the irradiation-only solve converges");
        assert_eq!(out.midplane_k, out.surface_k);
    }

    // Non-convergence REFUSES: an opacity closure that oscillates without settling returns `None`, the "refusal
    // names the gap" discipline, never a partial iterate reported as a temperature.
    #[test]
    fn a_non_converging_opacity_refuses() {
        // An opacity that flips wildly with a tiny temperature change never lets the midplane settle.
        let flip = |t: Fixed| -> Option<Fixed> {
            let cents = (t.to_f64_lossy() as i64) % 2;
            Some(if cents == 0 { r(1, 100) } else { r(50, 1) })
        };
        let out = solve_cpd_midplane_temperature(
            r(100000, 100),
            Fixed::ZERO,
            Fixed::ZERO,
            r(10000, 100),
            Fixed::ZERO,
            flip,
            flip,
            12,
            r(1, 1000),
        );
        assert!(
            out.is_none(),
            "a non-settling opacity must refuse, not report a partial iterate"
        );
    }

    #[test]
    fn degenerate_inputs_fail_soft() {
        let k = |_t: Fixed| Some(r(1, 1));
        // All fluxes zero.
        assert!(solve_cpd_midplane_temperature(
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            r(1, 1),
            Fixed::ZERO,
            k,
            k,
            10,
            r(1, 100)
        )
        .is_none());
        // Non-positive surface density.
        assert!(solve_cpd_midplane_temperature(
            r(100, 1),
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            k,
            k,
            10,
            r(1, 100)
        )
        .is_none());
        // Zero iteration bound.
        assert!(solve_cpd_midplane_temperature(
            r(100, 1),
            Fixed::ZERO,
            Fixed::ZERO,
            r(1, 1),
            Fixed::ZERO,
            k,
            k,
            0,
            r(1, 100)
        )
        .is_none());
    }

    #[test]
    fn the_solve_is_deterministic() {
        let k = |_t: Fixed| Some(r(1, 1));
        let call = || {
            solve_cpd_midplane_temperature(
                r(50000, 100),
                Fixed::ZERO,
                Fixed::ZERO,
                r(2000, 100),
                r(1000, 100),
                k,
                k,
                50,
                r(1, 100),
            )
        };
        assert_eq!(call(), call());
    }
}
