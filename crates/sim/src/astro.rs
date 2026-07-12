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
//! from the star's luminosity and the world's orbital distance, `flux = L / (4*pi*d^2)`, rather than
//! authored as a solar-constant number. The star's luminosity itself derives from its mass by the
//! main-sequence mass-luminosity relation `L = L_sun * (M_star/M_sun)^exponent`. This retires the inline
//! `solar_constant` literal (`environ.rs` `DiurnalSky`) to a read of this derivation.
//!
//! The value-authoring line: the three astronomical ANCHORS (the solar luminosity, the solar mass, and the
//! astronomical unit) are cited reference DATA, not per-world and not fundamentals (they do not belong on the
//! closed c/k_B/h/e/eps_0/N_A list); they are the measured constants the ratio form is scaled by, the same
//! standing as the CIAAW atomic weights in the periodic table. The per-world LEAVES (the star's mass as a
//! fraction of the sun, and the orbital distance) are reserved-with-basis; the mass-luminosity exponent is a
//! reserved closure-residue (opacity-regime dependent, near 3 to 5). Mirror pins one solar mass at one AU, so
//! its luminosity ratio is exactly one and its flux is `L_sun / (4*pi*AU^2)`, Earth's ~1361 W/m^2.
//!
//! The determinism and scale discipline: `L_sun` (~3.828e26 W) and `d^2` (~2.24e22 m^2 at one AU) overflow
//! Q32.32, and the RESULT (~1361) is what fits, so the derivation runs in exact rational arithmetic
//! (`civsim_units::bignum::BigRat`, the same integer-only path the Stefan-Boltzmann sigma uses) with pi from
//! Machin's formula (`civsim_units::compute::pi`), rounding ONCE to the fixed-point scale at the end. The
//! dimensionless mass-luminosity ratio stays in `Fixed` (it is order one, and `Fixed::powf` is the pinned
//! transcendental), applied to the base flux after the wide-magnitude divide. No floating point reaches
//! canonical state.

use civsim_core::Fixed;
use civsim_units::bignum::{BigRat, BigUint};
use civsim_units::compute;

/// The solar luminosity `L_sun` in watts, the IAU nominal value (IAU 2015 Resolution B3: 3.828e26 W). A cited
/// astronomical reference constant, the anchor the mass-luminosity ratio scales.
pub const SOLAR_LUMINOSITY_W: &str = "3.828e26";

/// The astronomical unit in metres, the IAU 2012 definition (149597870700 m exactly). A cited reference
/// constant; Mirror's orbital distance is one AU.
pub const ASTRONOMICAL_UNIT_M: &str = "149597870700";

/// The solar mass `M_sun` in kilograms, the IAU nominal value (~1.989e30 kg). Cited reference data, the
/// denominator of the per-world mass ratio `M_star/M_sun` the mass-luminosity relation reads.
pub const SOLAR_MASS_KG: &str = "1.989e30";

/// The number of decimal digits pi is computed to for the flux derivation. Far above the ~10 significant
/// figures the Q32.32 result carries (a `2^-32` epsilon near a ~1361 magnitude is a relative ~1.7e-13), so
/// the pi truncation never reaches the result's low bit. An engine-accuracy bound, not a world value.
pub const FLUX_PI_DIGITS: u32 = 40;

/// A non-negative `Fixed` (its bits over `2^FRAC_BITS`) as an exact rational, for multiplying a dimensionless
/// `Fixed` ratio into a wide-magnitude `BigRat` without leaving exact arithmetic. The caller passes a
/// non-negative value (a flux and a mass-luminosity ratio are both non-negative).
fn nonneg_fixed_to_bigrat(value: Fixed) -> BigRat {
    let bits = value.to_bits();
    let num = BigUint::from_u64(bits.max(0) as u64);
    let den = BigUint::from_u64(1).shl_bits(Fixed::FRAC_BITS);
    BigRat::new(false, num, den)
}

/// The base surface flux `L / (4*pi*d^2)` in W/m^2 for a luminosity `l_watts` and an orbital distance
/// `distance_m`, both cited decimal strings, DERIVED in exact rational arithmetic and rounded once to
/// Q32.32. `None` if a string does not parse, the distance is zero, or the result does not fit the
/// fixed-point range (a flux past the representable magnitude routes to `None` rather than wrapping).
pub fn base_flux(l_watts: &str, distance_m: &str) -> Option<Fixed> {
    let l = BigRat::from_decimal_str(l_watts).ok()?;
    let d = BigRat::from_decimal_str(distance_m).ok()?;
    let d2 = d.mul(&d);
    if d2.round_to_scale(Fixed::FRAC_BITS)? == 0 {
        return None;
    }
    let four_pi = BigRat::from_i64(4).mul(&compute::pi(FLUX_PI_DIGITS));
    let denom = four_pi.mul(&d2);
    let flux = l.div(&denom);
    let bits = flux.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

/// The full stellar-source flux for a world: the base flux `L_sun / (4*pi*d^2)` at the world's distance,
/// scaled by the main-sequence mass-luminosity ratio `(M_star/M_sun)^exponent`. `mass_ratio` is the star's
/// mass as a fraction of the sun (Mirror = 1), `exponent` the reserved mass-luminosity exponent (~3.5),
/// `distance_m` the world's orbital distance in metres (Mirror = one AU). The dimensionless ratio is applied
/// in `Fixed` (it is order one, `Fixed::powf` is the pinned power); the wide-magnitude divide is the exact
/// rational base flux. `None` on a non-parsing distance or an out-of-range flux.
pub fn stellar_flux(mass_ratio: Fixed, exponent: Fixed, distance_m: &str) -> Option<Fixed> {
    let base = base_flux(SOLAR_LUMINOSITY_W, distance_m)?;
    let luminosity_ratio = mass_ratio.powf(exponent);
    // flux = base * (M/M_sun)^exponent. Both non-negative; multiply exactly through BigRat and round once so
    // the ratio does not lose bits to an intermediate Fixed truncation.
    let product = nonneg_fixed_to_bigrat(base).mul(&nonneg_fixed_to_bigrat(luminosity_ratio));
    let bits = product.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

/// The Mirror world's surface flux: one solar mass at one AU, so the mass-luminosity ratio is exactly one and
/// the flux is `L_sun / (4*pi*AU^2)`. This is the derived value that replaces the inline solar-constant
/// literal. `exponent` is unused at unit mass ratio (one to any power is one) but is threaded so the same
/// derivation serves an off-Mirror world.
pub fn mirror_flux(exponent: Fixed) -> Option<Fixed> {
    stellar_flux(Fixed::ONE, exponent, ASTRONOMICAL_UNIT_M)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < 1e-2
    }

    #[test]
    fn the_mirror_flux_derives_earths_solar_constant() {
        // L_sun / (4 pi AU^2) = 3.828e26 / (4 pi (149597870700)^2) = ~1361.17 W/m^2, Earth's real total
        // solar irradiance (the measured TSI is ~1361, varying ~1360.8 to 1362 over the solar cycle), close
        // to but not the exact-integer 1361 the inline literal carried. The derivation is what fixes the
        // watt-scale; the small offset from 1361 is the real difference, never tuned back to the integer.
        let flux = mirror_flux(Fixed::from_ratio(35, 10)).expect("the Mirror flux derives");
        assert!(
            close(flux, 1361.166),
            "the derived Mirror flux is ~1361.17 W/m^2, got {}",
            flux.to_f64_lossy()
        );
    }

    #[test]
    fn the_flux_is_independent_of_the_exponent_at_unit_mass_ratio() {
        // One to any power is one, so a world at Mirror's mass and distance derives the same flux whatever
        // the reserved exponent, the invariance that keeps Mirror's value from riding on the exponent.
        let a = mirror_flux(Fixed::from_ratio(30, 10)).unwrap();
        let b = mirror_flux(Fixed::from_ratio(50, 10)).unwrap();
        assert_eq!(
            a, b,
            "at unit mass ratio the exponent does not move the flux"
        );
    }

    #[test]
    fn a_more_massive_star_is_brighter_by_the_mass_luminosity_law() {
        // A two-solar-mass star at one AU: L scales as 2^exponent, so the flux is ~2^3.5 = ~11.3 times
        // Mirror's. The ordering and rough magnitude are what the mass-luminosity relation asserts.
        let exponent = Fixed::from_ratio(35, 10);
        let mirror = mirror_flux(exponent).unwrap();
        let heavy = stellar_flux(Fixed::from_int(2), exponent, ASTRONOMICAL_UNIT_M).unwrap();
        assert!(heavy > mirror, "a heavier star delivers more flux");
        let ratio = heavy.to_f64_lossy() / mirror.to_f64_lossy();
        assert!(
            (ratio - 2.0_f64.powf(3.5)).abs() < 0.1,
            "the flux ratio tracks 2^exponent (~11.3), got {ratio}"
        );
    }

    #[test]
    fn a_farther_orbit_is_dimmer_by_the_inverse_square() {
        // Twice the distance is a quarter the flux (inverse-square), the geometry the derivation carries.
        let near = base_flux(SOLAR_LUMINOSITY_W, ASTRONOMICAL_UNIT_M).unwrap();
        let far = base_flux(SOLAR_LUMINOSITY_W, "299195741400").unwrap(); // two AU
        let ratio = near.to_f64_lossy() / far.to_f64_lossy();
        assert!(
            (ratio - 4.0).abs() < 0.05,
            "doubling the distance quarters the flux, got {ratio}"
        );
    }

    #[test]
    fn a_zero_distance_routes_to_none_rather_than_dividing_by_zero() {
        assert_eq!(base_flux(SOLAR_LUMINOSITY_W, "0"), None);
    }
}
