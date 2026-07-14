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

/// The Julian year in seconds (365.25 days * 86400 s = 31557600 s exactly), a cited definitional constant: the
/// seconds-per-year the accretion-rate argument (expressed in solar masses per megayear) is scaled by to reach
/// kg/s. A unit conversion, not a per-world value.
pub const JULIAN_YEAR_S: &str = "31557600";

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

/// The IRRADIATED-DISK (surface-equilibrium) TEMPERATURE `T_irr(r)` (K) at an orbital distance, DERIVED from
/// irradiation balance: the disk annulus at distance `r` intercepts the stellar flux `F(r) = L/(4*pi*r^2)`
/// ([`stellar_flux`], the same flux a world at that orbit receives), absorbs a geometry-set fraction of it, and
/// re-radiates in thermal equilibrium, so `sigma*T^4 = reprocessing_factor*F(r)` and
/// `T_irr(r) = (reprocessing_factor*F(r)/sigma)^(1/4)`. This is the SURFACE term of the two-regime disk-thermal
/// profile: irradiation heats the disk SURFACE, so it keeps this optically-thin equilibrium form and is not
/// boosted by the interior optical depth (the viscous term is, in [`disk_effective_temperature`] and the
/// optically-thick midplane closure). It falls with distance as `F^(1/4) ~ r^(-1/2)`, the outer-disk slope. Named
/// for the irradiation term rather than the midplane, correcting the earlier misnomer: this is `T_irr`, not the
/// full midplane temperature. `sigma` is the CODATA-derived Stefan-Boltzmann constant
/// ([`crate::physiology::derived_stefan_boltzmann`]), never authored.
///
/// Every per-world input is a scenario-set ARGUMENT (the admit-the-alien test): `mass_ratio`, `luminosity_exponent`
/// (the star's mass and its mass-luminosity residue, together fixing `L`), `distance_au` (the orbit), and
/// `reprocessing_factor`. The reprocessing factor is the reserved closure-residue of the disk's absorb-to-reradiate
/// GEOMETRY: `1/4` for a body that absorbs on its cross-section and re-emits isotropically (the fast-rotator /
/// spherical-grain equilibrium, the value that reproduces a planet's blackbody equilibrium temperature), a
/// grazing-and-flaring factor for a passive flared disk that intercepts starlight at a shallow angle and radiates
/// from two faces. Its basis is the disk (or grain) geometry of the world's regime, so a different disk structure
/// is a data row, never a rewrite. `t_max` is the representable ceiling the fourth-root read caps at (an engine
/// bound). At Earth's orbit (`mass_ratio = 1`, `distance_au = 1`, `reprocessing_factor = 1/4`) this derives the
/// ~278 K blackbody equilibrium temperature from `L_sun`, the AU, and the derived `sigma` alone, the derive-not-fit
/// anchor. `None` on a non-positive distance or a flux past the representable range.
pub fn irradiated_disk_temperature(
    mass_ratio: Fixed,
    luminosity_exponent: Fixed,
    distance_au: Fixed,
    reprocessing_factor: Fixed,
    t_max: Fixed,
) -> Option<Fixed> {
    // The flux the annulus at r intercepts is the same L/(4*pi*r^2) a world at that orbit receives.
    let flux = stellar_flux(mass_ratio, luminosity_exponent, distance_au)?;
    // The absorbed-and-reradiated balance sigma*T^4 = reprocessing_factor*F, inverted by the proven two-sqrt root.
    let absorbed = reprocessing_factor.checked_mul(flux)?;
    let sigma = crate::physiology::derived_stefan_boltzmann();
    Some(civsim_physics::laws::radiative_equilibrium(
        absorbed,
        Fixed::ONE,
        sigma,
        t_max,
    ))
}

/// The steady-disk viscous DISSIPATION FLUX `D(r)` (W/m^2) at an orbital distance: the Shakura-Sunyaev
/// `D = (3/(8*pi)) * Mdot * Omega_K^2 * inner_boundary_factor`, with the Keplerian frequency
/// `Omega_K^2 = G*M_star/r^3`. This is the accretional heating rate the viscous-inner disk radiates (each face
/// radiates `sigma*T^4 = D`), the source term the viscous temperature and the two-regime combination read.
///
/// `accretion_rate_msun_myr` is the mass-accretion rate `Mdot` in solar masses per megayear, the reserved
/// closure-residue (Mirror's ~0.01, that is ~1e-8 M_sun/yr, is order-one at this scale, keeping full fixed-point
/// precision; its basis the observed class-II disk accretion rate). `mass_ratio` sets `M_star = mass_ratio*M_sun`,
/// `distance_au` the orbit, `inner_boundary_factor` the `(1 - sqrt(R_in/r))` inner-edge suppression (~1 in the
/// bulk disk where the condensation fronts sit, its basis the inner truncation radius, retiring when `R_in`
/// derives). `G` is the CODATA gravitational constant read from the fundamentals register (single source), and
/// `M_sun` and the Julian year are the cited unit anchors. The wide-magnitude product (`Mdot`, `G`, `M_star`,
/// `r^3` overflow or underflow Q32.32 while the ~few W/m^2 result fits) runs in exact BigRat and rounds once.
/// `None` on a non-positive distance or a dissipation past the representable range.
fn viscous_dissipation_flux(
    accretion_rate_msun_myr: Fixed,
    mass_ratio: Fixed,
    distance_au: Fixed,
    inner_boundary_factor: Fixed,
) -> Option<Fixed> {
    if distance_au <= Fixed::ZERO {
        return None;
    }
    let m_sun = BigRat::from_decimal_str(SOLAR_MASS_KG).ok()?;
    // Mdot [kg/s] = accretion_rate [M_sun/Myr] * M_sun / (1e6 * Julian year).
    let megayear = BigRat::from_decimal_str(JULIAN_YEAR_S)
        .ok()?
        .mul(&BigRat::from_i64(1_000_000));
    let mdot = nonneg_fixed_to_bigrat(accretion_rate_msun_myr)
        .mul(&m_sun)
        .div(&megayear);
    // Omega_K^2 [1/s^2] = G * M_star / r^3, with M_star = mass_ratio*M_sun and r = distance_au*AU.
    let g =
        BigRat::from_decimal_str(civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value).ok()?;
    let m_star = nonneg_fixed_to_bigrat(mass_ratio).mul(&m_sun);
    let au = BigRat::from_decimal_str(ASTRONOMICAL_UNIT_M).ok()?;
    let r = nonneg_fixed_to_bigrat(distance_au).mul(&au);
    let r3 = r.mul(&r).mul(&r);
    let omega_k2 = g.mul(&m_star).div(&r3);
    // D = (3/(8*pi)) * Mdot * Omega_K^2 * inner_boundary_factor.
    let three_over_eight_pi =
        BigRat::from_i64(3).div(&BigRat::from_i64(8).mul(&compute::pi(FLUX_PI_DIGITS)));
    let d = three_over_eight_pi
        .mul(&mdot)
        .mul(&omega_k2)
        .mul(&nonneg_fixed_to_bigrat(inner_boundary_factor));
    let bits = d.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

/// The VISCOUS-DISK EFFECTIVE TEMPERATURE `T_visc(r)` (K) at an orbital distance, DERIVED from the accretional
/// heating: each face of the disk radiates `sigma*T_visc^4 = D(r)`, so `T_visc = (D(r)/sigma)^(1/4)`, the same
/// Stefan-Boltzmann inversion the irradiated regime uses ([`radiative_equilibrium`], the proven two-sqrt fourth
/// root). `D(r)` is the viscous dissipation ([`viscous_dissipation_flux`]), `sigma` the CODATA-derived
/// Stefan-Boltzmann constant. This is the VISCOUS-INNER term of the two-regime disk-thermal profile: it falls
/// with distance as `D^(1/4) ~ r^(-3/4)`, steeper than the irradiated `r^(-1/2)`, so it dominates the inner disk
/// and the two cross at an emergent transition radius (no authored boundary). Every per-world input is a
/// scenario-set ARGUMENT (the admit-the-alien test): the accretion rate, the mass ratio, the orbit, the
/// inner-edge factor, all data rows for a different disk. `t_max` is the representable ceiling the fourth-root
/// read caps at. `None` on a non-positive distance or a dissipation past the representable range.
pub fn viscous_disk_temperature(
    accretion_rate_msun_myr: Fixed,
    mass_ratio: Fixed,
    distance_au: Fixed,
    inner_boundary_factor: Fixed,
    t_max: Fixed,
) -> Option<Fixed> {
    let dissipation = viscous_dissipation_flux(
        accretion_rate_msun_myr,
        mass_ratio,
        distance_au,
        inner_boundary_factor,
    )?;
    let sigma = crate::physiology::derived_stefan_boltzmann();
    Some(civsim_physics::laws::radiative_equilibrium(
        dissipation,
        Fixed::ONE,
        sigma,
        t_max,
    ))
}

/// The DISK EFFECTIVE TEMPERATURE `T_eff(r)` (K) of the completed two-regime profile, combining the viscous-inner
/// and irradiated-outer heat sources. The two sources add in FLUX (`sigma*T_eff^4 = sigma*T_visc^4 + sigma*T_irr^4`),
/// so the combination is done at the flux level (the viscous dissipation `D(r)` plus the absorbed irradiation
/// `reprocessing_factor*F(r)`) and inverted once through [`radiative_equilibrium`], which also sidesteps the
/// unrepresentable `T^4` (`T_irr^4 ~ 6e9` overflows Q32.32 while the fluxes ~340 and ~3 W/m^2 do not). Viscous
/// dominates the inner disk (steep `r^(-3/4)`), irradiation the outer (`r^(-1/2)`), and the profile transitions
/// between them at the radius where the two fluxes cross, an EMERGENT boundary (no authored transition, Principle 8).
///
/// This is the SURFACE effective temperature (the optically-thick midplane boost is slice 3c). Every per-world
/// input is a scenario-set ARGUMENT (the admit-the-alien test): the accretion rate, the mass ratio and its
/// mass-luminosity exponent (fixing `L`), the orbit, the reprocessing factor, and the inner-edge factor. With no
/// accretion (`accretion_rate = 0`) the viscous flux vanishes and this reduces to [`irradiated_disk_temperature`]
/// exactly. `None` on a non-positive distance or a flux past the representable range.
#[allow(clippy::too_many_arguments)]
pub fn disk_effective_temperature(
    accretion_rate_msun_myr: Fixed,
    mass_ratio: Fixed,
    luminosity_exponent: Fixed,
    distance_au: Fixed,
    reprocessing_factor: Fixed,
    inner_boundary_factor: Fixed,
    t_max: Fixed,
) -> Option<Fixed> {
    let dissipation = viscous_dissipation_flux(
        accretion_rate_msun_myr,
        mass_ratio,
        distance_au,
        inner_boundary_factor,
    )?;
    let absorbed_irradiation = reprocessing_factor.checked_mul(stellar_flux(
        mass_ratio,
        luminosity_exponent,
        distance_au,
    )?)?;
    let total_flux = dissipation.checked_add(absorbed_irradiation)?;
    let sigma = crate::physiology::derived_stefan_boltzmann();
    Some(civsim_physics::laws::radiative_equilibrium(
        total_flux,
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

    #[test]
    fn earth_orbit_derives_the_blackbody_equilibrium_temperature() {
        // A body at 1 AU absorbing on its cross-section and re-emitting isotropically (reprocessing_factor = 1/4)
        // reaches sigma*T^4 = F/4 with F ~1361 W/m^2, so T = (1361/(4 sigma))^(1/4) ~278 K, Earth's textbook
        // blackbody equilibrium temperature (the ~255 K real value is 278 K reduced by the ~0.3 albedo, which the
        // atmosphere arc supplies later; here the airless blackbody value is the DERIVED anchor). Nothing tuned:
        // it falls out of L_sun, the AU, and the CODATA-derived sigma.
        let t_max = Fixed::from_int(100_000);
        let t = irradiated_disk_temperature(
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            Fixed::ONE,
            Fixed::from_ratio(1, 4),
            t_max,
        )
        .expect("the disk temperature derives");
        let k = t.to_f64_lossy();
        assert!(
            (k - 278.0).abs() < 3.0,
            "a body at 1 AU derives the ~278 K blackbody equilibrium temperature, got {k}"
        );
    }

    #[test]
    fn the_disk_temperature_falls_as_inverse_root_distance() {
        // F ~ r^-2 and T ~ F^(1/4), so T ~ r^(-1/2): four times the distance is half the temperature. The radial
        // slope that places the snow lines (a volatile's condensation front is where T(r) crosses its threshold).
        let (alpha, factor, t_max) = (
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(1, 4),
            Fixed::from_int(100_000),
        );
        let near =
            irradiated_disk_temperature(Fixed::ONE, alpha, Fixed::ONE, factor, t_max).unwrap();
        let far = irradiated_disk_temperature(Fixed::ONE, alpha, Fixed::from_int(4), factor, t_max)
            .unwrap();
        let ratio = near.to_f64_lossy() / far.to_f64_lossy();
        assert!(
            (ratio - 2.0).abs() < 0.05,
            "four times the distance halves the temperature (T ~ r^-1/2), got ratio {ratio}"
        );
    }

    #[test]
    fn a_brighter_star_warms_the_disk() {
        // A more luminous star warms its disk at the same orbit: T ~ L^(1/4) ~ mass^(alpha/4), so a two-solar-mass
        // star's disk at 1 AU is hotter than the Sun's.
        let (alpha, factor, t_max) = (
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(1, 4),
            Fixed::from_int(100_000),
        );
        let sun =
            irradiated_disk_temperature(Fixed::ONE, alpha, Fixed::ONE, factor, t_max).unwrap();
        let heavy =
            irradiated_disk_temperature(Fixed::from_int(2), alpha, Fixed::ONE, factor, t_max)
                .unwrap();
        assert!(
            heavy > sun,
            "a brighter star warms the disk at the same orbit"
        );
    }

    #[test]
    fn the_reprocessing_factor_scales_the_temperature() {
        // T ~ reprocessing_factor^(1/4): a sixteen-fold larger factor is a two-fold hotter disk, the geometry
        // residue entering as a fourth root (so its uncertainty is strongly damped in the temperature).
        let (alpha, t_max) = (Fixed::from_ratio(35, 10), Fixed::from_int(100_000));
        let low = irradiated_disk_temperature(
            Fixed::ONE,
            alpha,
            Fixed::ONE,
            Fixed::from_ratio(1, 16),
            t_max,
        )
        .unwrap();
        let high =
            irradiated_disk_temperature(Fixed::ONE, alpha, Fixed::ONE, Fixed::ONE, t_max).unwrap();
        let ratio = high.to_f64_lossy() / low.to_f64_lossy();
        assert!(
            (ratio - 2.0).abs() < 0.05,
            "a sixteen-fold larger reprocessing factor doubles the temperature, got {ratio}"
        );
    }

    #[test]
    fn a_non_positive_disk_distance_routes_to_none() {
        assert_eq!(
            irradiated_disk_temperature(
                Fixed::ONE,
                Fixed::from_ratio(35, 10),
                Fixed::ZERO,
                Fixed::from_ratio(1, 4),
                Fixed::from_int(100_000)
            ),
            None
        );
    }

    #[test]
    fn a_mirror_disk_at_one_au_derives_the_viscous_temperature() {
        // Mirror's disk at 1 AU: a solar-mass star, an accretion rate of 0.01 M_sun/Myr (~1e-8 M_sun/yr, the
        // observed class-II value), no inner-edge suppression. The Shakura-Sunyaev dissipation
        // D = (3/8pi) Mdot G M_sun / r^3 ~3 W/m^2 gives T_visc = (D/sigma)^(1/4) ~85 K. This is DERIVED from the
        // accretion rate, G, M_sun, and the AU; nothing tuned. ~85 K is BELOW the ~278 K irradiation at 1 AU, so
        // irradiation leads there (the regime the gate noted); the viscous term dominates well inside 1 AU.
        let t_max = Fixed::from_int(100_000);
        let t = viscous_disk_temperature(
            Fixed::from_ratio(1, 100), // 0.01 M_sun/Myr
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE, // inner-edge factor ~1 in the bulk disk
            t_max,
        )
        .expect("the viscous temperature derives");
        let k = t.to_f64_lossy();
        assert!(
            (k - 85.0).abs() < 4.0,
            "Mirror's disk at 1 AU derives T_visc ~85 K, got {k}"
        );
    }

    #[test]
    fn the_viscous_temperature_falls_as_r_to_the_minus_three_quarters() {
        // D ~ Omega_K^2 ~ r^-3 and T ~ D^(1/4), so T ~ r^(-3/4): four times the distance is 4^(3/4) ~2.83 times
        // cooler. This is STEEPER than the irradiated r^(-1/2), which is why the viscous term dominates the inner
        // disk and the two regimes cross at an emergent transition radius.
        let (mdot, factor, t_max) = (
            Fixed::from_ratio(1, 100),
            Fixed::ONE,
            Fixed::from_int(100_000),
        );
        let near = viscous_disk_temperature(mdot, Fixed::ONE, Fixed::ONE, factor, t_max).unwrap();
        let far =
            viscous_disk_temperature(mdot, Fixed::ONE, Fixed::from_int(4), factor, t_max).unwrap();
        let ratio = near.to_f64_lossy() / far.to_f64_lossy();
        assert!(
            (ratio - 4.0_f64.powf(0.75)).abs() < 0.05,
            "four times the distance is 4^(3/4) ~2.83 times cooler, got {ratio}"
        );
    }

    #[test]
    fn a_higher_accretion_rate_warms_the_viscous_disk() {
        // T_visc ~ Mdot^(1/4): a sixteen-fold higher accretion rate is a two-fold hotter viscous disk, the
        // accretion residue entering as a fourth root (strongly damped).
        let (factor, t_max) = (Fixed::ONE, Fixed::from_int(100_000));
        let low = viscous_disk_temperature(
            Fixed::from_ratio(1, 100),
            Fixed::ONE,
            Fixed::ONE,
            factor,
            t_max,
        )
        .unwrap();
        let high = viscous_disk_temperature(
            Fixed::from_ratio(16, 100),
            Fixed::ONE,
            Fixed::ONE,
            factor,
            t_max,
        )
        .unwrap();
        let ratio = high.to_f64_lossy() / low.to_f64_lossy();
        assert!(
            (ratio - 2.0).abs() < 0.05,
            "a sixteen-fold higher accretion rate doubles the viscous temperature, got {ratio}"
        );
    }

    #[test]
    fn the_inner_boundary_factor_suppresses_the_viscous_temperature() {
        // The (1 - sqrt(R_in/r)) factor multiplies the dissipation, so a smaller factor is a cooler annulus, and
        // it enters as a fourth root: a sixteen-fold smaller factor halves T_visc.
        let (mdot, t_max) = (Fixed::from_ratio(1, 100), Fixed::from_int(100_000));
        let full =
            viscous_disk_temperature(mdot, Fixed::ONE, Fixed::ONE, Fixed::ONE, t_max).unwrap();
        let suppressed = viscous_disk_temperature(
            mdot,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::from_ratio(1, 16),
            t_max,
        )
        .unwrap();
        let ratio = full.to_f64_lossy() / suppressed.to_f64_lossy();
        assert!(
            (ratio - 2.0).abs() < 0.05,
            "a sixteen-fold smaller inner-edge factor halves the temperature, got {ratio}"
        );
    }

    #[test]
    fn a_non_positive_viscous_distance_routes_to_none() {
        assert_eq!(
            viscous_disk_temperature(
                Fixed::from_ratio(1, 100),
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::ONE,
                Fixed::from_int(100_000)
            ),
            None
        );
    }

    #[test]
    fn the_disk_effective_temperature_sums_the_two_regimes() {
        // At 1 AU irradiation leads (~278 K) and the viscous term (~85 K) adds a little, so the flux-summed
        // effective temperature sits just above pure irradiation and above pure viscous: T_eff^4 = T_irr^4 + T_visc^4.
        let t_max = Fixed::from_int(100_000);
        let (mdot, mass, alpha, reproc, inner) = (
            Fixed::from_ratio(1, 100),
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(1, 4),
            Fixed::ONE,
        );
        let eff = disk_effective_temperature(mdot, mass, alpha, Fixed::ONE, reproc, inner, t_max)
            .unwrap();
        let irr = irradiated_disk_temperature(mass, alpha, Fixed::ONE, reproc, t_max).unwrap();
        let visc = viscous_disk_temperature(mdot, mass, Fixed::ONE, inner, t_max).unwrap();
        assert!(eff > irr, "the two-regime sum exceeds pure irradiation");
        assert!(eff > visc, "the two-regime sum exceeds pure viscous");
        assert!(
            (eff.to_f64_lossy() - 278.6).abs() < 2.0,
            "at 1 AU the sum is ~278.6 K, got {}",
            eff.to_f64_lossy()
        );
    }

    #[test]
    fn the_two_regime_sum_reduces_to_irradiation_with_no_accretion() {
        // With no accretion the viscous flux vanishes, so the two-regime profile is pure irradiation, EXACTLY the
        // same bits as irradiated_disk_temperature (the flux sum adds zero).
        let t_max = Fixed::from_int(100_000);
        let (mass, alpha, reproc, inner) = (
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(1, 4),
            Fixed::ONE,
        );
        let eff =
            disk_effective_temperature(Fixed::ZERO, mass, alpha, Fixed::ONE, reproc, inner, t_max)
                .unwrap();
        let irr = irradiated_disk_temperature(mass, alpha, Fixed::ONE, reproc, t_max).unwrap();
        assert_eq!(
            eff, irr,
            "no accretion reduces the two-regime profile to pure irradiation"
        );
    }

    #[test]
    fn the_viscous_regime_dominates_the_close_inner_disk() {
        // A high accretion rate (10 M_sun/Myr, ~1e-5 M_sun/yr, an early disk) at a close orbit (0.05 AU): the
        // viscous dissipation overwhelms the irradiation, so the effective temperature tracks the viscous term.
        // The viscous-inner regime the completed profile adds, dominating where accretional heating is strong.
        let t_max = Fixed::from_int(100_000);
        let (mdot, mass, alpha, reproc, inner, dist) = (
            Fixed::from_int(10),
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            Fixed::from_ratio(1, 4),
            Fixed::ONE,
            Fixed::from_ratio(5, 100),
        );
        let eff =
            disk_effective_temperature(mdot, mass, alpha, dist, reproc, inner, t_max).unwrap();
        let irr = irradiated_disk_temperature(mass, alpha, dist, reproc, t_max).unwrap();
        let visc = viscous_disk_temperature(mdot, mass, dist, inner, t_max).unwrap();
        assert!(
            eff > irr,
            "with strong accretion the effective temperature exceeds pure irradiation"
        );
        let d_eff_visc = (eff.to_f64_lossy() - visc.to_f64_lossy()).abs();
        let d_eff_irr = (eff.to_f64_lossy() - irr.to_f64_lossy()).abs();
        assert!(
            d_eff_visc < d_eff_irr,
            "in the strongly-accreting inner disk T_eff tracks the viscous term"
        );
    }

    #[test]
    fn a_non_positive_effective_temperature_distance_routes_to_none() {
        assert_eq!(
            disk_effective_temperature(
                Fixed::from_ratio(1, 100),
                Fixed::ONE,
                Fixed::from_ratio(35, 10),
                Fixed::ZERO,
                Fixed::from_ratio(1, 4),
                Fixed::ONE,
                Fixed::from_int(100_000)
            ),
            None
        );
    }
}
