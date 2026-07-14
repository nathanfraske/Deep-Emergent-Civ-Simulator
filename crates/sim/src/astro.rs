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

//! The stellar-source derivation (genesis-forward Stage 1): the surface flux a world receives is DERIVED
//! from its star's mass and the world's orbital distance, `flux = L / (4*pi*d^2)` with the luminosity from the
//! main-sequence mass-luminosity relation `L = L_sun * (M_star/M_sun)^exponent`, rather than authored as a
//! solar-constant number. This retires the inline `solar_constant` literal (`environ.rs` `DiurnalSky`) to a
//! read of this derivation.
//!
//! The value-authoring line and the admit-the-alien test. This kernel is fixed Rust. The two authored things
//! it holds are cited REFERENCE ANCHORS, not world content: the solar luminosity and mass (the Sun-anchored
//! scale of the mass-luminosity relation, so at `M = M_sun` it returns `L_sun` exactly) and the astronomical
//! unit (the metres-per-AU conversion). Every PER-WORLD input arrives as an ARGUMENT set by the scenario: the
//! star's mass as a fraction of the sun (`mass_ratio`), the orbital distance in AU (`distance_au`), and the
//! mass-luminosity exponent. So an alien world with a heavier star, a wider orbit, or a different opacity
//! regime is a data row (different arguments), never a rewrite: nothing Mirror-specific is hardcoded here, the
//! Mirror values live in the scenario that calls this.
//!
//! The determinism and scale discipline: `L_sun` (~3.828e26 W) and `d^2` (~2.24e22 m^2 at one AU) overflow
//! Q32.32, and the RESULT (~1361) is what fits, so the wide-magnitude divide runs in exact rational arithmetic
//! (`civsim_units::bignum::BigRat`, the same integer-only path the Stefan-Boltzmann sigma uses) with pi from
//! Machin's formula (`civsim_units::compute::pi`), rounding ONCE to the fixed-point scale at the end. The
//! order-one arguments (the mass ratio, the exponent, the distance in AU) stay `Fixed`; the mass-luminosity
//! power is `Fixed::powf`, the pinned transcendental. No floating point reaches canonical state.

use civsim_core::Fixed;
use civsim_units::bignum::{BigRat, BigUint};
use civsim_units::compute;

/// The solar luminosity `L_sun` in watts, the IAU 2015 Resolution B3 nominal value (3.828e26 W). A cited
/// REFERENCE ANCHOR (the Sun-anchored scale of the mass-luminosity relation), not a per-world value.
pub const SOLAR_LUMINOSITY_W: &str = "3.828e26";

/// The astronomical unit in metres, the IAU 2012 definition (149597870700 m exactly). A cited reference
/// anchor: the metres-per-AU conversion the distance argument (in AU) is scaled by.
pub const ASTRONOMICAL_UNIT_M: &str = "149597870700";

/// The solar mass `M_sun` in kilograms, the IAU nominal value (~1.989e30 kg). A cited reference anchor: the
/// denominator of the per-world mass ratio `M_star/M_sun`. The scenario passes the ratio directly, so this is
/// the documented reference for computing it, not read in the kernel.
pub const SOLAR_MASS_KG: &str = "1.989e30";

/// The solar radius `R_sun` in metres, the IAU 2015 Resolution B3 nominal value (6.957e8 m). A cited reference
/// anchor: the Sun-anchored scale of the mass-radius relation, so at `M = M_sun` the star's radius returns
/// `R_sun`. Consumed by the effective-temperature solve, not the flux (a world receives flux at its orbit, not
/// at the stellar surface).
pub const SOLAR_RADIUS_M: &str = "6.957e8";

/// The number of decimal digits pi is computed to for the flux derivation. Far above the ~10 significant
/// figures the Q32.32 result carries (a `2^-32` epsilon near a ~1361 magnitude is a relative ~1.7e-13), so
/// the pi truncation never reaches the result's low bit. An engine-accuracy bound, not a world value.
pub const FLUX_PI_DIGITS: u32 = 40;

/// A non-negative `Fixed` (its bits over `2^FRAC_BITS`) as an exact rational, so an order-one `Fixed` argument
/// multiplies into the wide-magnitude `BigRat` without leaving exact arithmetic. The caller passes a
/// non-negative value (a distance, a flux, and a mass-luminosity ratio are all non-negative).
fn nonneg_fixed_to_bigrat(value: Fixed) -> BigRat {
    let bits = value.to_bits();
    let num = BigUint::from_u64(bits.max(0) as u64);
    let den = BigUint::from_u64(1).shl_bits(Fixed::FRAC_BITS);
    BigRat::new(false, num, den)
}

/// The stellar-source flux a world receives, in W/m^2: `L_sun * (mass_ratio)^exponent / (4*pi*d^2)`, with
/// `d = distance_au * AU`. `mass_ratio` is the star's mass as a fraction of the sun (Mirror = 1), `exponent`
/// the mass-luminosity exponent (a reserved closure-residue, ~3.5), `distance_au` the world's orbital distance
/// in astronomical units (Mirror = 1). All three are scenario-set arguments (the admit-the-alien test); the
/// derivation and the cited anchors are the only fixed parts.
///
/// The wide-magnitude divide (`L_sun / (4*pi*d^2)`, whose operands overflow Q32.32 while the ~1361 result
/// fits) runs in exact rational arithmetic and rounds once to the fixed-point scale; the order-one mass ratio
/// enters through `Fixed::powf`. `None` on a non-positive distance or a flux past the representable range (it
/// routes to the extreme rather than wrapping).
pub fn stellar_flux(mass_ratio: Fixed, exponent: Fixed, distance_au: Fixed) -> Option<Fixed> {
    if distance_au <= Fixed::ZERO {
        return None;
    }
    let au = BigRat::from_decimal_str(ASTRONOMICAL_UNIT_M).ok()?;
    let d = nonneg_fixed_to_bigrat(distance_au).mul(&au);
    let d2 = d.mul(&d);
    let four_pi = BigRat::from_i64(4).mul(&compute::pi(FLUX_PI_DIGITS));
    let denom = four_pi.mul(&d2);
    let l_sun = BigRat::from_decimal_str(SOLAR_LUMINOSITY_W).ok()?;
    let luminosity = l_sun.mul(&nonneg_fixed_to_bigrat(mass_ratio.powf(exponent)));
    let flux = luminosity.div(&denom);
    let bits = flux.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

/// The stellar EFFECTIVE TEMPERATURE `T_eff` (K) a star radiates at, DERIVED from its mass through the
/// Stefan-Boltzmann law: `T_eff = (L / (4*pi*R_star^2*sigma))^(1/4)`, the luminosity from the mass-luminosity
/// relation `L = L_sun*(mass_ratio)^luminosity_exponent` and the radius from the mass-radius relation
/// `R_star = R_sun*(mass_ratio)^radius_exponent`. `sigma` is the Stefan-Boltzmann constant DERIVED from the
/// CODATA fundamentals (`k_B`, `h`, `c`) through [`crate::physiology::derived_stefan_boltzmann`], never authored.
///
/// Every per-world input is a scenario-set ARGUMENT (the admit-the-alien test): `mass_ratio` (Mirror = 1), and
/// the TWO relation exponents, each a reserved closure-residue passed by the caller so a different opacity or
/// structure regime is a data row, never a rewrite. `luminosity_exponent` is the mass-luminosity exponent (the
/// same residue [`stellar_flux`] carries, ~3.5 in the solar regime); `radius_exponent` is the mass-radius
/// exponent (a SECOND residue this solve needs that the flux does not, ~0.8 on the upper main sequence), its
/// basis the main-sequence mass-radius slope of the star's regime. `t_max` is the representable ceiling the
/// fourth-root read caps at (an engine bound the caller sets, not a physical knob). The only fixed parts are the
/// derivation, the cited anchors (`L_sun`, `R_sun`), and the derived `sigma`.
///
/// At `mass_ratio = 1` both exponents drop out (one to any power is one) and `T_eff` returns the Sun's effective
/// temperature (~5772 K) from `L_sun`, `R_sun`, and `sigma` alone: the derive-not-fit anchor, nothing tuned to
/// hit it. The stellar surface flux `F = L/(4*pi*R_star^2)` (whose `L` and `R_star^2` overflow Q32.32 while the
/// ~6.3e7 W/m^2 result fits) runs the wide divide in exact rational arithmetic and rounds once; the fourth root
/// reuses [`civsim_physics::laws::radiative_equilibrium`] (two nested integer square roots, so the
/// unrepresentable `T^4` never forms), with emissivity one because a star radiates as a blackbody at its
/// effective temperature by the definition of `T_eff`. `None` on a non-positive mass ratio or a surface flux past
/// the representable range.
pub fn stellar_effective_temperature(
    mass_ratio: Fixed,
    luminosity_exponent: Fixed,
    radius_exponent: Fixed,
    t_max: Fixed,
) -> Option<Fixed> {
    if mass_ratio <= Fixed::ZERO {
        return None;
    }
    // The stellar radius R_star = R_sun*mass_ratio^beta, and the surface flux F = L/(4*pi*R_star^2).
    let r_sun = BigRat::from_decimal_str(SOLAR_RADIUS_M).ok()?;
    let r_star = r_sun.mul(&nonneg_fixed_to_bigrat(mass_ratio.powf(radius_exponent)));
    let r2 = r_star.mul(&r_star);
    let four_pi = BigRat::from_i64(4).mul(&compute::pi(FLUX_PI_DIGITS));
    let denom = four_pi.mul(&r2);
    let l_sun = BigRat::from_decimal_str(SOLAR_LUMINOSITY_W).ok()?;
    let luminosity = l_sun.mul(&nonneg_fixed_to_bigrat(
        mass_ratio.powf(luminosity_exponent),
    ));
    let surface_flux_bits = luminosity.div(&denom).round_to_scale(Fixed::FRAC_BITS)?;
    let surface_flux = Fixed::from_bits_i128(surface_flux_bits)?;
    // T_eff = (F / sigma)^(1/4): the Stefan-Boltzmann inversion, the proven two-sqrt fourth root.
    let sigma = crate::physiology::derived_stefan_boltzmann();
    Some(civsim_physics::laws::radiative_equilibrium(
        surface_flux,
        Fixed::ONE,
        sigma,
        t_max,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < 1e-2
    }

    #[test]
    fn a_sun_at_one_au_derives_earths_solar_constant() {
        // mass_ratio = 1, distance = 1 AU: L returns L_sun exactly (one to any power is one), and
        // L_sun / (4 pi AU^2) = 3.828e26 / (4 pi (149597870700)^2) = ~1361.17 W/m^2, Earth's real total solar
        // irradiance (the measured TSI is ~1361, varying ~1360.8 to 1362 over the solar cycle). Close to but
        // not the exact-integer 1361 the retired literal carried; the small offset is the real difference, the
        // integer literal's rounding, never tuned back.
        let flux =
            stellar_flux(Fixed::ONE, Fixed::from_ratio(35, 10), Fixed::ONE).expect("derives");
        assert!(
            close(flux, 1361.166),
            "a solar-mass star at one AU derives ~1361.17 W/m^2, got {}",
            flux.to_f64_lossy()
        );
    }

    #[test]
    fn the_flux_is_independent_of_the_exponent_at_unit_mass_ratio() {
        // One to any power is one, so a solar-mass star derives the same flux whatever the reserved exponent,
        // the invariance (L at M_sun is L_sun exactly) that keeps Mirror anchored on Earth's real value.
        let a = stellar_flux(Fixed::ONE, Fixed::from_ratio(30, 10), Fixed::ONE).unwrap();
        let b = stellar_flux(Fixed::ONE, Fixed::from_ratio(50, 10), Fixed::ONE).unwrap();
        assert_eq!(
            a, b,
            "at unit mass ratio the exponent does not move the flux"
        );
    }

    #[test]
    fn a_more_massive_star_is_brighter_by_the_mass_luminosity_law() {
        // A two-solar-mass star at one AU: L scales as 2^exponent, so the flux is ~2^3.5 = ~11.3 times a
        // solar-mass star's. The ordering and rough magnitude are what the mass-luminosity relation asserts.
        let exponent = Fixed::from_ratio(35, 10);
        let sun = stellar_flux(Fixed::ONE, exponent, Fixed::ONE).unwrap();
        let heavy = stellar_flux(Fixed::from_int(2), exponent, Fixed::ONE).unwrap();
        assert!(heavy > sun, "a heavier star delivers more flux");
        let ratio = heavy.to_f64_lossy() / sun.to_f64_lossy();
        assert!(
            (ratio - 2.0_f64.powf(3.5)).abs() < 0.1,
            "the flux ratio tracks 2^exponent (~11.3), got {ratio}"
        );
    }

    #[test]
    fn a_farther_orbit_is_dimmer_by_the_inverse_square() {
        // Twice the distance is a quarter the flux (inverse-square), the geometry the derivation carries.
        let exponent = Fixed::from_ratio(35, 10);
        let near = stellar_flux(Fixed::ONE, exponent, Fixed::ONE).unwrap();
        let far = stellar_flux(Fixed::ONE, exponent, Fixed::from_int(2)).unwrap();
        let ratio = near.to_f64_lossy() / far.to_f64_lossy();
        assert!(
            (ratio - 4.0).abs() < 0.05,
            "doubling the distance quarters the flux, got {ratio}"
        );
    }

    #[test]
    fn a_non_positive_distance_routes_to_none() {
        assert_eq!(
            stellar_flux(Fixed::ONE, Fixed::from_ratio(35, 10), Fixed::ZERO),
            None
        );
    }

    #[test]
    fn a_sun_derives_its_effective_temperature() {
        // mass_ratio = 1: the exponents drop out and T_eff = (L_sun/(4 pi R_sun^2 sigma))^(1/4), which is the
        // Sun's effective temperature ~5772 K (IAU nominal 5772). This is DERIVED from L_sun, R_sun, and the
        // CODATA-derived sigma, never fit: nothing here was tuned to land 5772. The measured value is ~5769 K, a
        // ~3 K (0.05%) offset from the coarse Q32.32 sigma (~8 fractional bits) and the integer-root
        // discretization, not a knob.
        let t_max = Fixed::from_int(100_000); // an engine ceiling above any main-sequence T_eff
        let t_eff = stellar_effective_temperature(
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(8, 10),
            t_max,
        )
        .expect("the sun derives a temperature");
        let k = t_eff.to_f64_lossy();
        assert!(
            (k - 5772.0).abs() < 20.0,
            "a solar-mass star derives T_eff ~5772 K, got {k}"
        );
    }

    #[test]
    fn the_effective_temperature_is_exponent_independent_at_unit_mass() {
        // One to any power is one, so at the solar mass ratio neither exponent moves T_eff: the anchor stays on
        // the Sun's real value whatever the reserved residues, mirroring the flux invariance.
        let t_max = Fixed::from_int(100_000);
        let a = stellar_effective_temperature(
            Fixed::ONE,
            Fixed::from_ratio(30, 10),
            Fixed::from_ratio(6, 10),
            t_max,
        )
        .unwrap();
        let b = stellar_effective_temperature(
            Fixed::ONE,
            Fixed::from_ratio(50, 10),
            Fixed::from_ratio(10, 10),
            t_max,
        )
        .unwrap();
        assert_eq!(a, b, "at unit mass ratio the exponents do not move T_eff");
    }

    #[test]
    fn a_more_massive_star_is_hotter_when_luminosity_outpaces_area() {
        // A heavier star: L scales as mass^alpha and the emitting area as mass^(2*beta), so T_eff scales as
        // mass^((alpha - 2*beta)/4). With alpha = 3.5 and beta = 0.8 the exponent is positive (0.475), so a
        // two-solar-mass star is hotter, by ~2^0.475 = ~1.39. The ordering and rough magnitude are what the
        // mass-luminosity and mass-radius relations together assert.
        let (alpha, beta) = (Fixed::from_ratio(35, 10), Fixed::from_ratio(8, 10));
        let t_max = Fixed::from_int(100_000);
        let sun = stellar_effective_temperature(Fixed::ONE, alpha, beta, t_max).unwrap();
        let heavy = stellar_effective_temperature(Fixed::from_int(2), alpha, beta, t_max).unwrap();
        assert!(heavy > sun, "a heavier star radiates hotter");
        let ratio = heavy.to_f64_lossy() / sun.to_f64_lossy();
        assert!(
            (ratio - 2.0_f64.powf(0.475)).abs() < 0.03,
            "the T_eff ratio tracks mass^((alpha-2beta)/4) (~1.39), got {ratio}"
        );
    }

    #[test]
    fn a_non_positive_mass_ratio_routes_to_none() {
        assert_eq!(
            stellar_effective_temperature(
                Fixed::ZERO,
                Fixed::from_ratio(35, 10),
                Fixed::from_ratio(8, 10),
                Fixed::from_int(100_000)
            ),
            None
        );
    }
}
