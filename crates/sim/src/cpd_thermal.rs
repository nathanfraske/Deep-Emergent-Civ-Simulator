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

//! The CIRCUMPLANETARY-DISK (CPD) THERMAL STRUCTURE, the layer below the moon Branch A composition
//! (`docs/working/CPD_THERMAL_STRUCTURE_SCOPE.md`): the gas-starved viscous-plus-irradiated disk whose midplane
//! temperature sets where water ice condenses, and so the satellites' rock-versus-ice composition. The full
//! midplane temperature is a COUPLED implicit solve (the temperature sets the viscosity, surface density, and
//! opacity that set the temperature); this module builds the grounded STANDALONE primitives the solve composes,
//! one focused concern at a time, never a grab bag. This slice is the VISCOUS HEATING flux, the dominant heat
//! source of the inner disc. The gas-flux geometry factor that feeds it is a FLAGGED research question (its
//! printed branches do not meet at the centrifugal radius; see the note above the flux), so it enters as a caller
//! input here rather than a kernel, pending the primary equations.
//!
//! Grounded in the gas-starved CPD model of Canup and Ward (2002, 2006) with the Makalkin and Dorofeeva thermal
//! structure, as reproduced by Schneeberger and Mousis (2025, PSJ 6:23, DOI 10.3847/PSJ/ad9de1; preprint
//! arXiv:2411.13351; receipts in the scope doc). The
//! Keplerian frequency around the planet is REUSED from [`crate::astro::kepler_orbital_period_seconds`] and the
//! CPD outer (centrifugal) radius from [`crate::astro::centrifugal_radius_au`], never re-derived here.
//!
//! DORMANT: no run-path caller, so the two run pins hold bit-exact. Determinism (Principle 3): fixed-point
//! throughout, the wide-magnitude flux carried in the LOG domain (an accretion rate in kg/s overflows Q32.32), a
//! degenerate input failing soft to `None`. The value-authoring line (Principle 6): no number authored; the `3/8`,
//! the `1/5`, `4/5` of the geometry factor, and the `2 pi` of the frequency are the standard algebra of the cited
//! equations, and the disk parameters (accretion rate, the geometry radii) are per-CPD data on the argument list,
//! so a hotter or colder disc, or an alien giant's CPD, is a data row (admits the alien).

use civsim_core::Fixed;

use crate::astro::kepler_orbital_period_seconds;

// FLAGGED RESEARCH QUESTION (not built): the GAS-FLUX GEOMETRY FACTOR `Lambda(r)/l` (Schneeberger and Mousis 2025
// Eqs. 4, 5, 6), the radial profile of the viscously-spreading gas that multiplies the surface density and this
// flux. As PRINTED, the inside-`R_c` branch (Eq. 4) and the outside-`R_c` branch (Eq. 5) do NOT meet at the
// centrifugal radius: at `r = R_c` the inside carries `sqrt(R_p/R_c)` and `(4/5)sqrt(R_c/r_d)` where the outside
// carries `R_p/R_c` and `(4/5)(R_c/r_d)` (a square-root against a linear term, plus a mismatched `R_p/r_d` group),
// so `Lambda` jumps by about 0.13 at Jovian scales. The surface density must be continuous there (mass
// conservation), so this is either a typesetting slip in the reproduction or an unstated model feature, and it is
// NOT resolvable from this single reproduction. Resolving it needs the primary equations (Canup and Ward 2002;
// Makalkin and Dorofeeva 2014, 1995), or a re-derivation from the continuity equation (Eq. 1). Until then the
// geometry factor is a caller input to [`viscous_heating_flux_log10`] (about 1 in the mid-disc), not a kernel here.
// Recorded in `docs/working/CPD_THERMAL_STRUCTURE_SCOPE.md` as the first research flag of the CPD thermal sub-arc.

/// The VISCOUS HEATING FLUX `F_vis = (3 / (8 pi)) (Lambda/l) Mdot Omega_K^2` of the CPD (Schneeberger and Mousis
/// 2025, PSJ 6:23, Equation 19, after Makalkin and Dorofeeva 2014), returned as `log10(F_vis / (W m^-2))`. This is the
/// accretional dissipation the CPD's own viscously-spreading gas releases, the dominant heat source of the inner
/// disc and one of the three fluxes that set the surface temperature (the accretion and planet-irradiation fluxes
/// are the siblings, the latter awaiting the planet's formation luminosity, a named further-down rung).
///
/// The Keplerian angular frequency around the planet is DERIVED by reusing [`kepler_orbital_period_seconds`]:
/// `Omega_K = 2 pi / P`. Inputs: `geometry_factor` the dimensionless `Lambda/l` gas-flux profile (a caller input,
/// about 1 in the mid-disc; its radial form is the flagged research question noted above);
/// `log10_mdot_kg_s` the CPD gas accretion rate as `log10(Mdot / (kg s^-1))` (a bare accretion rate overflows
/// Q32.32, so it enters as its base-10 log, the gas-starved value being reserved-with-basis at the call site,
/// order `1e-7` Jupiter masses per year); `sat_orbit_au` the satellite's distance from the planet (AU); and
/// `planet_mass_solar` the planet mass in solar masses, the central mass of the Keplerian frequency.
///
/// The result is a base-10 log because the flux spans a wide range and `Omega_K^2` with a large `Mdot` would not be
/// representable directly; it is a weighted sum of logs with no exponentiation. `None` on a non-positive input, if
/// the reused Kepler period does not resolve, or on an overflow.
// @sources: schneeberger_mousis_2025_cpd_thermal
pub fn viscous_heating_flux_log10(
    geometry_factor: Fixed,
    log10_mdot_kg_s: Fixed,
    sat_orbit_au: Fixed,
    planet_mass_solar: Fixed,
) -> Option<Fixed> {
    if geometry_factor <= Fixed::ZERO
        || sat_orbit_au <= Fixed::ZERO
        || planet_mass_solar <= Fixed::ZERO
    {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10 = |x: Fixed| -> Option<Fixed> { x.ln().checked_div(ln10) };
    let two = Fixed::from_int(2);
    // Omega_K = 2 pi / P, so log10(Omega_K) = log10(2 pi) - log10(P). P from the reused Kepler kernel.
    let period_s = kepler_orbital_period_seconds(sat_orbit_au, planet_mass_solar)?;
    if period_s <= Fixed::ZERO {
        return None;
    }
    let two_pi = Fixed::PI.checked_add(Fixed::PI)?;
    let log10_omega = log10(two_pi)?.checked_sub(log10(period_s)?)?;
    // log10(F_vis) = log10(3/(8 pi)) + log10(Lambda/l) + log10(Mdot) + 2 log10(Omega_K).
    let prefactor = Fixed::from_int(3).checked_div(Fixed::from_int(8).checked_mul(Fixed::PI)?)?;
    log10(prefactor)?
        .checked_add(log10(geometry_factor)?)?
        .checked_add(log10_mdot_kg_s)?
        .checked_add(two.checked_mul(log10_omega)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    /// The viscous flux reproduces the Jovian CPD viscous heating to fixed-point tolerance, cross-checked against an
    /// INDEPENDENT f64 reference computed from Equation 19 directly. With a gas-starved accretion rate
    /// `Mdot ~ 1.2e13 kg/s` (about `2e-7` Jupiter masses per year) at Ganymede's orbit (`~15 R_J`), the viscous
    /// dissipation is of order `10^2 W/m^2`, a black-body effective temperature near 200 K, the physical scale of
    /// the satellite-forming region.
    #[test]
    fn the_viscous_flux_matches_an_independent_reference_at_ganymede() {
        let orbit_au = r(716, 100_000); // ~15 R_J in AU
        let planet_mass_solar = r(954, 1_000_000); // ~1 M_Jup in solar masses
        let mdot = 1.2e13_f64;
        let geom = 1.0_f64; // Lambda/l ~ 1 in the mid-disc
                            // Independent f64 reference from Eq. 19: F_vis = (3/(8 pi)) (Lambda/l) Mdot Omega_K^2.
        let g = 6.674e-11_f64;
        let m_sun = 1.989e30_f64;
        let au = 1.495_978_707e11_f64;
        let a_m = orbit_au.to_f64_lossy() * au;
        let m_planet_kg = planet_mass_solar.to_f64_lossy() * m_sun;
        let omega_k = (g * m_planet_kg / a_m.powi(3)).sqrt();
        let f_ref = (3.0 / (8.0 * std::f64::consts::PI)) * geom * mdot * omega_k * omega_k;
        let log10_ref = f_ref.log10();
        let got = viscous_heating_flux_log10(
            Fixed::ONE,
            r((mdot.log10() * 1e6) as i64, 1_000_000),
            orbit_au,
            planet_mass_solar,
        )
        .expect("the viscous flux resolves at Ganymede's orbit");
        assert!(
            (got.to_f64_lossy() - log10_ref).abs() < 1e-2,
            "kernel log10(F_vis)={} vs independent Eq.19 reference log10={}",
            got.to_f64_lossy(),
            log10_ref
        );
        // Physical anchor: the viscous flux sits near 10^2 W/m^2 (an effective temperature of a few hundred K).
        assert!(
            got.to_f64_lossy() > 0.0 && got.to_f64_lossy() < 4.0,
            "the Jovian CPD viscous flux is of order 10^0 to 10^4 W/m^2, got 10^{}",
            got.to_f64_lossy()
        );
    }

    /// Determinism (Principle 3) and fail-soft: identical inputs give the identical log, and a non-positive
    /// geometry factor, orbit, or planet mass returns `None`, never a fabricated flux.
    #[test]
    fn the_viscous_flux_is_deterministic_and_fails_soft() {
        let args = (Fixed::ONE, r(13, 1), r(716, 100_000), r(954, 1_000_000));
        assert_eq!(
            viscous_heating_flux_log10(args.0, args.1, args.2, args.3),
            viscous_heating_flux_log10(args.0, args.1, args.2, args.3)
        );
        assert!(viscous_heating_flux_log10(Fixed::ZERO, args.1, args.2, args.3).is_none());
        assert!(viscous_heating_flux_log10(args.0, args.1, Fixed::ZERO, args.3).is_none());
        assert!(viscous_heating_flux_log10(args.0, args.1, args.2, Fixed::ZERO).is_none());
    }
}
