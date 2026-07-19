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

/// The Earth mass in kilograms, the IAU nominal terrestrial mass (~5.9722e24 kg). A cited reference anchor: the
/// scale a DERIVED planet mass is reported against (the accretion feeding-zone integral yields a mass; a planet's
/// mass in Earth masses times this anchor is its mass in kg), and the anchor the derived planet radius reads. Not a
/// per-world value; the derived planet mass is the per-world quantity.
pub const EARTH_MASS_KG: &str = "5.9722e24";

/// The Earth mean radius in metres, the IUGG/IAU arithmetic mean radius `R_1 = (2a + b)/3 = 6371.0 km`. A cited
/// reference anchor (not a per-world value): the honest gravity gate is `g_ref = G M_earth / R_earth^2` computed from
/// this and [`EARTH_MASS_KG`], which lands ~9.82 m/s^2, the value Earth's own mass and radius give. This anchor
/// replaces the standard-gravity CONVENTION `9.80665` (the 1901 CGPM sea-level-45-degree definition), which is a
/// bureaucratic datum, not Earth's derived surface gravity; a derived quantity must be checked against the physics,
/// not against a convention. Held as an integer-metre value so it constructs exactly in fixed-point.
pub const EARTH_MEAN_RADIUS_M: i32 = 6_371_000;

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
    // The mass-luminosity power law fixes the luminosity `L = L_sun * mass_ratio^exponent`, then the flux
    // derivation is shared with the direct-luminosity door: this delegates so the two forms cannot drift apart,
    // and is byte-identical to computing the luminosity inline (the same `Fixed` enters the same wide divide).
    stellar_flux_from_luminosity_lsun(mass_ratio.powf(exponent), distance_au)
}

/// The stellar-source flux from a DIRECTLY SUPPLIED bolometric luminosity (in `L_sun`), the door for a luminosity
/// the `mass^exponent` power law cannot express: `L_sun * luminosity_lsun / (4*pi*d^2)`, with `d = distance_au *
/// AU`. The load-bearing case is a PRE-MAIN-SEQUENCE star, which at a solar mass is several times brighter than
/// its main-sequence instance while `mass_ratio^exponent` at `mass_ratio = 1` is exactly one, so no exponent can
/// carry the pre-main-sequence brightness at the Mirror mass. `luminosity_lsun` is a scenario-set argument (the
/// star's own bolometric luminosity, however derived), so a star of any track or composition is a data row. This
/// is byte-identical to [`stellar_flux`] when `luminosity_lsun` equals `mass_ratio^exponent` (the same `Fixed`
/// bits enter the same wide-magnitude divide, which is why [`stellar_flux`] delegates here). The wide divide runs
/// in exact rational arithmetic and rounds once. `None` on a non-positive distance or a flux past the
/// representable range.
pub fn stellar_flux_from_luminosity_lsun(
    luminosity_lsun: Fixed,
    distance_au: Fixed,
) -> Option<Fixed> {
    if distance_au <= Fixed::ZERO {
        return None;
    }
    let au = BigRat::from_decimal_str(ASTRONOMICAL_UNIT_M).ok()?;
    let d = nonneg_fixed_to_bigrat(distance_au).mul(&au);
    let d2 = d.mul(&d);
    let four_pi = BigRat::from_i64(4).mul(&compute::pi(FLUX_PI_DIGITS));
    let denom = four_pi.mul(&d2);
    let l_sun = BigRat::from_decimal_str(SOLAR_LUMINOSITY_W).ok()?;
    let luminosity = l_sun.mul(&nonneg_fixed_to_bigrat(luminosity_lsun));
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
    // The Stefan-Boltzmann inversion in SUN-RELATIVE form, so a massive star whose surface flux overflows fixed
    // point still derives its T_eff. T_eff = T_sun*(F/F_sun)^(1/4) = T_sun*M^(alpha/4 - beta/2): the flux RATIO to
    // the Sun (a representable M^~1.9) scales the solar anchor, and the wide-magnitude stellar flux (which crosses
    // the Q32.32 ceiling near 6.4 M_sun) is never formed, the log-space-census discipline. Mathematically identical
    // to (F/sigma)^(1/4) and byte-identical at unit mass, but Betelgeuse-mass safe.
    let r_sun = BigRat::from_decimal_str(SOLAR_RADIUS_M).ok()?;
    let l_sun = BigRat::from_decimal_str(SOLAR_LUMINOSITY_W).ok()?;
    let four_pi = BigRat::from_i64(4).mul(&compute::pi(FLUX_PI_DIGITS));
    // The Sun's OWN surface flux F_sun = L_sun/(4*pi*R_sun^2), ~6.3e7 W/m^2, which IS representable.
    let solar_flux_bits = l_sun
        .div(&four_pi.mul(&r_sun.mul(&r_sun)))
        .round_to_scale(Fixed::FRAC_BITS)?;
    let solar_flux = Fixed::from_bits_i128(solar_flux_bits)?;
    let sigma = crate::physiology::derived_stefan_boltzmann();
    let t_sun = civsim_physics::laws::radiative_equilibrium(solar_flux, Fixed::ONE, sigma, t_max);
    // The mass scaling M^(alpha/4 - beta/2), the Stefan-Boltzmann inversion of L ~ M^alpha and R ~ M^beta, a
    // representable power at any mass. T_sun already carries the t_max fourth-root ceiling; re-cap the scaled result
    // so a hot star still saturates at t_max as before.
    let exponent = luminosity_exponent
        .checked_div(Fixed::from_int(4))?
        .checked_sub(radius_exponent.checked_div(Fixed::from_int(2))?)?;
    let t_eff = t_sun.checked_mul(mass_ratio.powf(exponent))?;
    Some(if t_eff > t_max { t_max } else { t_eff })
}

/// The BLACK, ISOTHERMAL SPHERE REDISTRIBUTION FACTOR, DERIVED from geometry rather than reserved: a body that
/// absorbs starlight on its circular cross-section `pi r^2` and re-emits isotropically over its full sphere
/// `4 pi r^2` reprocesses `pi r^2 / (4 pi r^2)` of the incident flux per unit emitting area. The `pi` and `r^2`
/// cancel analytically, so the factor is the EXACT rational `1/4`, the cross-section-to-sphere ratio that
/// reproduces a body's blackbody equilibrium temperature (the ~278 K at 1 AU anchor). ADMITS THE ALIEN: it is pure
/// geometry, independent of the body's composition or the star's spectrum.
///
/// SCOPE, stated so it is never over-read: this is the BLACK (zero-albedo) case, so the 278 K anchor is the airless
/// blackbody value a real body's albedo reduces (Earth's ~255 K is 278 K times `(1-A)^(1/4)`); and it is the
/// ISOTHERMAL / FULL-REDISTRIBUTION case, the `1/4` assuming the absorbed flux spreads uniformly over the whole
/// sphere (a fast rotator or a high thermal-inertia body), where a slow rotator radiating mostly from its dayside
/// carries a larger factor. A CALLER supplies its own albedo and redistribution when the body departs from black
/// and isothermal.
///
/// NOT THE DISK DEFAULT. This is a SPHERICAL-BODY redistribution factor, NOT the passive-disk reprocessing
/// solution. A passive FLARED DISK intercepts starlight at a shallow grazing angle and radiates from two faces, so
/// its reprocessing factor is the grazing-and-flaring geometry (of order a few percent, a much SMALLER number),
/// which needs a disk vertical-structure (scale-height flaring) substrate the engine does not yet carry. The
/// canonical disk therefore passes its OWN flared factor to [`irradiated_disk_temperature`] and
/// [`disk_effective_temperature`] (the derived-disk tests use about `0.05`); this `1/4` must never be wired in as
/// the disk default, and is exercised only for a spherical grain or a fast-rotating body's equilibrium temperature.
pub fn spherical_reprocessing_factor() -> Fixed {
    // pi r^2 (absorbing cross-section) over 4 pi r^2 (isotropic emitting sphere): pi and r^2 cancel to the exact 1/4.
    Fixed::from_ratio(1, 4)
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
/// `reprocessing_factor`. The reprocessing factor is the disk's absorb-to-reradiate GEOMETRY, DERIVED not reserved:
/// the spherical-grain / fast-rotator case is [`spherical_reprocessing_factor`], the exact cross-section-to-sphere
/// `1/4` that reproduces a planet's blackbody equilibrium temperature; a passive flared disk that intercepts
/// starlight at a shallow angle and radiates from two faces takes the grazing-and-flaring factor instead, keyed on
/// the disk's own flaring (a derive-later data row when the vertical-structure substrate lands). Its basis is the
/// disk (or grain) geometry of the world's regime, so a different disk structure is a data row, never a rewrite. `t_max` is the representable ceiling the fourth-root read caps at (an engine
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
/// Stefan-Boltzmann inversion the irradiated regime uses ([`civsim_physics::laws::radiative_equilibrium`], the proven two-sqrt fourth
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
/// `reprocessing_factor*F(r)`) and inverted once through [`civsim_physics::laws::radiative_equilibrium`], which also sidesteps the
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

/// Kepler's third law REFERENCE PERIOD: the orbital period in world-seconds at ONE AU around a ONE-solar-mass
/// star, DERIVED from the cited astronomical unit, solar mass, and the fundamental gravitational constant.
/// `T_ref = 2*pi*sqrt(AU^3/(G*M_sun))`, the sidereal-year anchor (~3.156e7 s, ~365.25 days). It is computed, not
/// fit: this is the Kepler period the floor gives, distinct from the round 365-day calendar fixture the run path
/// currently carries and that this arc retires. The wide radicand `AU^3/(G*M_sun)` (~2.5e13, over the Q32.32
/// ceiling) is formed in exact rational arithmetic and rooted once by the scale-aware Tier-2 integer square root
/// ([`civsim_units::tier2::isqrt`]), so no float and no unrepresentable intermediate enters, the same
/// wide-magnitude discipline the stellar flux uses. `None` only if the derivation exceeds the representable
/// range, which it does not for the solar reference.
fn reference_orbital_period_seconds() -> Option<Fixed> {
    let au = BigRat::from_decimal_str(ASTRONOMICAL_UNIT_M).ok()?;
    let m_sun = BigRat::from_decimal_str(SOLAR_MASS_KG).ok()?;
    let g =
        BigRat::from_decimal_str(civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value).ok()?;
    let pi = compute::pi(FLUX_PI_DIGITS);
    let four_pi2 = BigRat::from_i64(4).mul(&pi).mul(&pi);
    let a3 = au.mul(&au).mul(&au);
    // The squared period T^2 = 4*pi^2*AU^3/(G*M_sun), in seconds^2 (~9.96e14). Rounded to the integer-second^2
    // scale (its ~15 significant figures are far finer than a period needs) and rooted once at Q32.32.
    let t_squared = four_pi2.mul(&a3).div(&g.mul(&m_sun));
    let t2_bits = i64::try_from(t_squared.round_to_scale(0)?).ok()?;
    let t_bits = civsim_units::tier2::isqrt(t2_bits, 0, Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(t_bits as i128)
}

/// The ORBITAL PERIOD in world-seconds of a planet at `orbit_au` around a star of `star_mass_ratio` solar
/// masses, DERIVED from Kepler's third law `T^2 = 4*pi^2*a^3/(G*M_star)`. It factors as
/// `T = T_ref*sqrt(orbit_au^3/star_mass_ratio)` around the derived one-AU one-solar-mass reference
/// ([`reference_orbital_period_seconds`]), so the per-orbit factor stays in Q32.32 (the cube of the orbit and
/// the mass ratio are order-one across the terrestrial zone) while the wide constant is derived once. The
/// factorisation is the exact Kepler identity, not an approximation. This is the year a world's time cadences
/// derive from, retiring the reserved year scalar in the celestial substrate.
///
/// Every input is a scenario-set ARGUMENT (the admit-the-alien test): a different orbit or a different star mass
/// is a data row, never a rewrite. `None` on a non-positive orbit or star mass, or a period past the
/// representable range: a far orbit whose year in seconds crosses the Q32.32 ceiling (~16 AU around a solar-mass
/// star) fails loud rather than wrapping, the log-space period representation being the units follow-on, flagged
/// not faked.
pub fn kepler_orbital_period_seconds(orbit_au: Fixed, star_mass_ratio: Fixed) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO || star_mass_ratio <= Fixed::ZERO {
        return None;
    }
    let t_ref = reference_orbital_period_seconds()?;
    let a3 = orbit_au.checked_mul(orbit_au)?.checked_mul(orbit_au)?;
    let factor = a3.checked_div(star_mass_ratio)?;
    // sqrt of an order-one-to-order-thousand Q32.32 value; the wide magnitude lives only in T_ref.
    let root = factor.sqrt();
    t_ref.checked_mul(root)
}

/// The ORBITAL PERIOD in YEARS (sidereal), the representable-across-the-whole-system companion to
/// [`kepler_orbital_period_seconds`]. Kepler's third law in astronomical units (AU, solar mass, sidereal year)
/// is `T^2 = a^3 / M`, so `T[yr] = sqrt(orbit_au^3 / star_mass_ratio)` with Earth (`a = 1`, `M = 1`) at exactly
/// one year, the natural unit the sidereal year already anchors. It is computed in LOG-SPACE (the
/// [`planet_radius_m`] cube-root precedent), `ln T = (3*ln(orbit_au) - ln(star_mass_ratio)) / 2`, so `orbit_au^3`
/// never forms: the period stays representable out past the Oort cloud (about a million years at 1e5 AU) where
/// the seconds form overflows Q32.32 near 16 AU. This is the unit the multi-body system map carries, the fix for
/// the seconds ceiling the orbits arc surfaced.
///
/// Every input is a scenario ARGUMENT (admit the alien). `None` on a non-positive orbit or star mass, or a period
/// past the representable-years ceiling (about 1.6e6 AU, where the result reaches the Q32.32 maximum): it fails
/// loud rather than saturating, the honest units bound (the log-space exp window and the fixed-point ceiling
/// coincide there). The ceiling is `ln(2^31)`, the log of the representable maximum itself, an engine
/// representability bound derived from the representation, not a physical value.
pub fn kepler_orbital_period_years(orbit_au: Fixed, star_mass_ratio: Fixed) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO || star_mass_ratio <= Fixed::ZERO {
        return None;
    }
    let ln_period = Fixed::from_int(3)
        .checked_mul(orbit_au.ln())?
        .checked_sub(star_mass_ratio.ln())?
        .checked_div(Fixed::from_int(2))?;
    // Fail loud past the representable ceiling rather than let `exp` saturate to the maximum: the Q32.32 positive
    // ceiling is ~2^31, so a result fits only while `ln_period < ln(2^31) = 31*ln(2)`. This is the log of the
    // representation's own maximum, an engine bound, not an owner value.
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_period >= ln_ceiling {
        return None;
    }
    Some(ln_period.exp())
}

/// The Earth-to-Sun mass ratio `M_earth / M_sun` (~3.0e-6) as a `Fixed`, folded once from the two cited reference
/// anchors [`EARTH_MASS_KG`] and [`SOLAR_MASS_KG`]. It is the dimensionless bridge a body's mass in Earth masses
/// crosses to reach a fraction of the star mass, the ratio the Hill radius and the resonance-overlap survival time
/// both need. Not a per-world value: a view of the two cited anchors, exactly the fold [`hill_radius_au`] forms
/// inline. `None` only on a decimal-parse or scale miss, which the fixed anchors do not produce.
pub fn earth_to_sun_mass_ratio() -> Option<Fixed> {
    let earth = BigRat::from_decimal_str(EARTH_MASS_KG).ok()?;
    let sun = BigRat::from_decimal_str(SOLAR_MASS_KG).ok()?;
    Fixed::from_bits_i128(earth.div(&sun).round_to_scale(Fixed::FRAC_BITS)?)
}

/// The HILL RADIUS in AU, the reach of a body's own gravity against the star's tide: the distance out to which
/// the body dominates, `R_H = a * (M_planet / (3*M_star))^(1/3)`. It is the ruler the whole multi-body system is
/// built on: the feeding zone a planet clears (a few `R_H`, the isolation mass), the spacing between neighbouring
/// planets (mutual Hill radii, dynamical stability), and the sphere within which a moon stays bound (satellite
/// capture, task #75) are all measured in it. `orbit_au` the orbit, `planet_mass_earth` the body mass in Earth
/// masses, `star_mass_ratio` the star mass in solar masses.
///
/// The mass ratio `M_planet / M_star` is formed in consistent units by folding the cited Earth-to-Sun mass ratio
/// once (`EARTH_MASS_KG / SOLAR_MASS_KG`, ~3.0e-6, from the two cited anchors), so no wide intermediate forms and
/// the cube root runs on an order-`1e-6` Q32.32 value. At one Earth mass, 1 AU, one solar mass this derives
/// ~0.0098 AU (Earth's real Hill radius), and Jupiter (318 Earth masses, 5.2 AU) derives ~0.35 AU, both matched
/// without a fit. `None` on a non-positive input or a register miss.
pub fn hill_radius_au(
    orbit_au: Fixed,
    planet_mass_earth: Fixed,
    star_mass_ratio: Fixed,
) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO || planet_mass_earth <= Fixed::ZERO || star_mass_ratio <= Fixed::ZERO
    {
        return None;
    }
    // The Earth-to-Sun mass ratio from the two cited anchors, folded once (~3.0e-6, well inside Q32.32).
    let earth_per_sun = {
        let earth = BigRat::from_decimal_str(EARTH_MASS_KG).ok()?;
        let sun = BigRat::from_decimal_str(SOLAR_MASS_KG).ok()?;
        Fixed::from_bits_i128(earth.div(&sun).round_to_scale(Fixed::FRAC_BITS)?)?
    };
    // M_planet / M_star = (planet_mass_earth * (M_earth/M_sun)) / star_mass_ratio, then the (.../3)^(1/3) factor.
    let mass_ratio = planet_mass_earth
        .checked_mul(earth_per_sun)?
        .checked_div(star_mass_ratio)?;
    let cube_argument = mass_ratio.checked_div(Fixed::from_int(3))?;
    orbit_au.checked_mul(cube_argument.cbrt())
}

/// The oligarchic ISOLATION MASS in Earth masses: the mass a growing embryo reaches once it has swept its feeding
/// zone clear, DERIVED self-consistently from the Hill radius, so the feeding-zone WIDTH is no longer a reserved
/// geometry input. An embryo accretes a zone `Delta a = C*R_H` wide (`R_H` the Hill radius), and the zone mass
/// `M_iso = 2*pi*a*Delta a*Sigma` and `R_H` both depend on `M`; the self-consistent solve closes to
/// `M_iso = (2*pi*C*a^2*Sigma)^(3/2) / sqrt(3*M_star)`, the Kokubo-Ida oligarchic isolation mass. This retires the
/// reserved feeding-zone width [`feeding_zone_mass`] carried: the width now DERIVES as `C` Hill radii.
///
/// `orbit_au` the orbit, `star_mass_ratio` the star mass in solar masses, `sigma_kg_m2` the disk SOLID surface
/// density at the orbit ([`disk_surface_density`], kg/m^2), `feeding_zone_hill_widths` the width `C` in Hill radii
/// (a reserved-with-basis residue, its basis the oligarchic feeding-zone width, a few to ~10 mutual Hill radii,
/// Kokubo-Ida 1998/2000). The wide product runs in LOG-SPACE (the [`planet_radius_m`] precedent): the AU, solar
/// mass, and Earth mass anchors enter as their decimal-string logs, so no unrepresentable intermediate forms. The
/// honest result is SUB-EARTH at Earth's orbit (a Mars-class oligarch, which is why Earth needs oligarch mergers
/// to reach one mass, the Layer-4 giant-impact tier). `None` on a non-positive input or a register miss.
pub fn isolation_mass_earth(
    orbit_au: Fixed,
    star_mass_ratio: Fixed,
    sigma_kg_m2: Fixed,
    feeding_zone_hill_widths: Fixed,
) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO
        || star_mass_ratio <= Fixed::ZERO
        || sigma_kg_m2 <= Fixed::ZERO
        || feeding_zone_hill_widths <= Fixed::ZERO
    {
        return None;
    }
    let two_pi_c = Fixed::PI
        .checked_add(Fixed::PI)?
        .checked_mul(feeding_zone_hill_widths)?;
    let three_halves = Fixed::from_ratio(3, 2);
    let half = Fixed::from_ratio(1, 2);
    // ln(a[m]) = ln(orbit_au) + ln(AU_m); ln(M_star[kg]) = ln(star_mass_ratio) + ln(M_sun_kg), the wide anchors
    // entering as their decimal-string logs.
    let ln_a_m = orbit_au
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?)?;
    let ln_m_star = star_mass_ratio
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
    // ln M_iso[kg] = 1.5*ln(2*pi*C) + 3*ln(a) + 1.5*ln(Sigma) - 0.5*(ln 3 + ln M_star).
    let ln_m_iso_kg = three_halves
        .checked_mul(two_pi_c.ln())?
        .checked_add(Fixed::from_int(3).checked_mul(ln_a_m)?)?
        .checked_add(three_halves.checked_mul(sigma_kg_m2.ln())?)?
        .checked_sub(half.checked_mul(Fixed::from_int(3).ln().checked_add(ln_m_star)?)?)?;
    let ln_m_iso_earth =
        ln_m_iso_kg.checked_sub(civsim_physics::saha::ln_of_decimal(EARTH_MASS_KG)?)?;
    Some(ln_m_iso_earth.exp())
}

/// The DISK SURFACE DENSITY `Sigma(r)` (in the normalization's units) at an orbital distance, the Lynden-Bell and
/// Pringle self-similar profile `Sigma(r) = Sigma_c * (r/r_c)^(-gamma) * exp(-(r/r_c)^(2-gamma))`: a power-law
/// interior steepened by an exponential cutoff beyond the characteristic radius `r_c`. This is the second half of
/// the stage-2 disk structure and the column the disk optical depth integrates (`tau_R = kappa_R * Sigma / 2`),
/// which the optically-thick midplane closure (slice 3c-iii) reads.
///
/// Every per-world input is a scenario-set ARGUMENT (the admit-the-alien test): `distance_au` (the orbit),
/// `characteristic_radius_au` the cutoff radius `r_c` (a reserved residue, its basis the disk's viscous-spreading /
/// angular-momentum radius), `gamma` the surface-density slope (the viscous-spreading exponent, a reserved residue
/// ~1, its basis the viscosity power law `nu ~ r^gamma`), and `normalization` the scale `Sigma_c` (a reserved
/// residue, its basis the disk-mass fraction, `Sigma_c ~ M_disk*(2-gamma)/(2*pi*r_c^2)`). The profile SHAPE is the
/// fixed physics; the three residues are the caller's, so a different disk is a data row. `gamma` must be below 2
/// (the finite-mass condition, `2-gamma > 0` giving the outer cutoff), else `None`. The order-one ratio
/// `x = r/r_c` keeps the powers and the exponential in `Fixed`; far beyond `r_c` the `exp` argument passes the
/// window floor and saturates to zero, the physical disk edge. `None` on a non-positive distance or radius,
/// `gamma >= 2`, or an intermediate past the representable range.
pub fn disk_surface_density(
    distance_au: Fixed,
    characteristic_radius_au: Fixed,
    gamma: Fixed,
    normalization: Fixed,
) -> Option<Fixed> {
    if distance_au <= Fixed::ZERO || characteristic_radius_au <= Fixed::ZERO {
        return None;
    }
    let two = Fixed::from_int(2);
    if gamma >= two {
        return None; // the finite-mass condition 2 - gamma > 0 (an outer cutoff exists)
    }
    let x = distance_au.checked_div(characteristic_radius_au)?;
    // The power-law interior x^(-gamma) = 1 / x^gamma.
    let power = Fixed::ONE.checked_div(x.powf(gamma))?;
    // The exponential cutoff exp(-(x^(2-gamma))); beyond r_c the argument passes the exp window floor and the
    // exponential saturates to zero, the disk's physical outer edge.
    let cutoff = Fixed::ZERO
        .checked_sub(x.powf(two.checked_sub(gamma)?))?
        .exp();
    let density = normalization.checked_mul(power)?.checked_mul(cutoff)?;
    Some(density)
}

/// The DISK GAS SURFACE DENSITY `Sigma(r)` (kg/m^2) at an orbital distance, DERIVED from the STEADY-STATE VISCOUS
/// SIMILARITY rather than read as a free normalization. A steady accretion disk carries the same mass-flux `Mdot`
/// through every radius, so `Sigma = Mdot / (3*pi*nu)` with the kinematic viscosity `nu = alpha*c_s*H`
/// (Shakura-Sunyaev 1973), the isothermal sound speed `c_s^2 = k_B*T/(mu*m_H)`, the scale height `H = c_s/Omega`,
/// and the Keplerian frequency `Omega = sqrt(G*M_star/r^3)`. Composing these the sound speed cancels
/// (`nu = alpha*k_B*T/(mu*m_H*Omega)`), leaving
/// `Sigma(r) = Mdot*mu*m_H*Omega(r) / (3*pi*alpha*k_B*T(r))`. This retires the Lynden-Bell and Pringle residues
/// `Sigma_c`, `gamma`, and `r_c` to VIEWS of the disk realization (the accretion rate, the viscosity, the star
/// mass, and the derived disk temperature): zero new per-system initial conditions (the R-ASSEMBLY gate-G target).
///
/// The surface-density slope `gamma ~ 1` is now EMERGENT, not authored: `Sigma ~ Omega/T ~ r^(-3/2)/r^(-1/2) =
/// r^(-1)` wherever the disk temperature follows the irradiated `T ~ r^(-1/2)` (the inner viscous regime
/// `T ~ r^(-3/4)` steepens it toward `r^(-3/4)`), so the ~1 slope falls out of the viscous physics rather than
/// being a residue (a test asserts the `r^(-1)` fall-off under an irradiated `T(r)`).
///
/// DERIVED / read from the floor: `Mdot` from `accretion_rate_msun_myr` (the same `M_sun/Myr -> kg/s` conversion
/// [`viscous_dissipation_flux`] uses), `Omega(r)` from the CODATA `G` (`fundamentals::GRAVITATIONAL_CONSTANT`), the
/// star mass ([`SOLAR_MASS_KG`] times `star_mass_ratio`) and `r = orbit_au * AU`, `k_B`
/// (`fundamentals::BOLTZMANN`), and `m_H` as one atomic mass unit (`1e-3 / N_A` kg, one gram-per-mole per amu, from
/// `fundamentals::AVOGADRO`); `disk_temperature_k` is the caller's derived disk temperature `T(r)`.
/// RESERVED-with-basis, surfaced rather than fabricated: `alpha_viscosity`, the Shakura-Sunyaev viscosity parameter,
/// and `mean_molecular_weight` `mu` (basis ~2.34 for a solar H2+He mix; a per-composition datum, so a carbon-rich or
/// a metal-poor disk is a data row). ALPHA IS A CHORD THAT MUST DECLARE ITS METHOD (research-agent re-scope): the
/// letter covers TWO quantities that diverge in practice, the EFFECTIVE TRANSPORT coefficient the accretion clock
/// consumes (calibrated by accretion-rate and disk-lifetime demographics) and the LOCALLY-MEASURED turbulence
/// coefficient (ALMA linewidths, MRI simulations), which part company in dead zones and non-ideal-MHD or weakly
/// hydrodynamic regimes. This clock consumes the TRANSPORT-side quantity, so the basis is the transport-side
/// observable (accretion-inferred `alpha ~ 1e-3 to 1e-2`), NOT the turbulence measurement; a per-disk datum, so a
/// quiescent dead-zone disk and an MRI-active disk are data rows. The full census (method, region and regime,
/// mechanism class, with regime-conditioned banded rows) is the alpha arc's first deliverable, not this interim.
///
/// The product spans many decades (`Mdot ~ 1e15 kg/s`, `Omega ~ 1e-7 rad/s`, `m_H ~ 1e-27 kg`, `k_B ~ 1e-23 J/K`),
/// so the whole assembly runs in LOG-SPACE (the [`isolation_mass_earth`] precedent): `ln Sigma = ln Mdot + ln mu +
/// ln m_H + ln Omega - ln(3*pi) - ln alpha - ln k_B - ln T`, with `ln Omega = 0.5*(ln G + ln M_star - 3*ln r)`, the
/// decimal-string constants entering as [`civsim_physics::saha::ln_of_decimal`] logs and the order-one `Fixed`
/// inputs through `Fixed::ln`, exponentiated once at the end. No wide-magnitude product is ever formed outside the
/// log domain. `None` on a non-positive input or a result past the representable exp window (it fails loud rather
/// than saturating, the honest units bound, the same style as the neighbours).
pub fn viscous_similarity_surface_density(
    orbit_au: Fixed,
    star_mass_ratio: Fixed,
    accretion_rate_msun_myr: Fixed,
    disk_temperature_k: Fixed,
    alpha_viscosity: Fixed,
    mean_molecular_weight: Fixed,
) -> Option<Fixed> {
    if orbit_au <= Fixed::ZERO
        || star_mass_ratio <= Fixed::ZERO
        || accretion_rate_msun_myr <= Fixed::ZERO
        || disk_temperature_k <= Fixed::ZERO
        || alpha_viscosity <= Fixed::ZERO
        || mean_molecular_weight <= Fixed::ZERO
    {
        return None;
    }
    // ln Mdot [kg/s] = ln(accretion_rate) + ln(M_sun) - ln(1e6 * Julian year), the M_sun/Myr -> kg/s conversion in
    // the log domain (the same conversion `viscous_dissipation_flux` forms in BigRat).
    let ln_megayear_s = civsim_physics::saha::ln_of_decimal(JULIAN_YEAR_S)?
        .checked_add(civsim_physics::saha::ln_of_decimal("1e6")?)?;
    let ln_mdot = accretion_rate_msun_myr
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?
        .checked_sub(ln_megayear_s)?;
    // ln Omega = 0.5*(ln G + ln M_star - 3*ln r), with M_star = star_mass_ratio*M_sun and r = orbit_au*AU.
    let ln_m_star = star_mass_ratio
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
    let ln_r = orbit_au
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?)?;
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    let ln_omega = Fixed::from_ratio(1, 2).checked_mul(
        ln_g.checked_add(ln_m_star)?
            .checked_sub(Fixed::from_int(3).checked_mul(ln_r)?)?,
    )?;
    // ln m_H = ln(1e-3) - ln(N_A): one atomic mass unit, one gram-per-mole per amu (`fundamentals::AVOGADRO`).
    let ln_m_h = civsim_physics::saha::ln_of_decimal("1e-3")?.checked_sub(
        civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::AVOGADRO.value)?,
    )?;
    let ln_k_b = civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::BOLTZMANN.value)?;
    let ln_three_pi = Fixed::from_int(3).checked_mul(Fixed::PI)?.ln();
    // ln Sigma = ln Mdot + ln mu + ln m_H + ln Omega - ln(3*pi) - ln alpha - ln k_B - ln T.
    let ln_sigma = ln_mdot
        .checked_add(mean_molecular_weight.ln())?
        .checked_add(ln_m_h)?
        .checked_add(ln_omega)?
        .checked_sub(ln_three_pi)?
        .checked_sub(alpha_viscosity.ln())?
        .checked_sub(ln_k_b)?
        .checked_sub(disk_temperature_k.ln())?;
    // Fail loud past the representable exp ceiling rather than let `exp` saturate to the maximum (the
    // `kepler_orbital_period_years` precedent): `ln(2^31) = 31*ln2` is the log of the representation's own maximum,
    // an engine bound, not an owner value.
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_sigma >= ln_ceiling {
        return None;
    }
    Some(ln_sigma.exp())
}

/// A protostellar CORE-COLLAPSE MODEL: the dimensionless mass-accretion eigenvalue `m0` of an inside-out collapse,
/// carried as data so which collapse solution the birth rate uses is a DECLARED CHOICE, the sibling of the
/// [`XrayWindFit`] wind ensemble (distinct physics claims banded, not a settled law). The rate is `Mdot = m0*c_s^3/G`
/// ([`shu_inside_out_collapse_accretion_rate_msun_myr`]), so a larger `m0` is a faster, more violent collapse. The
/// mechanism applies whichever model is passed; an alien collapse physics is a new constructor, a data row, not a
/// rewrite (admit-the-alien).
///
/// `m0` IS A CHORD THAT DECLARES ITS ABSCISSA. The classic isothermal-sphere collapse solutions are a CONTINUUM
/// FAMILY parameterized by the instability parameter `A` (the initial central over-density relative to the
/// hydrostatic singular isothermal sphere), with `m0` conditioning on `A` within Shu 1977's own Table 1 (Hunter
/// 1977; Whitworth and Summers 1985). The two shipped rows are the DECLARED ENDPOINTS of that measured continuum, a
/// factor ~48 apart, the Owen-versus-Sellek band exactly: (1) [`CollapseModel::shu_1977`], the hydrostatic edge
/// `A = 2`, `m0 = 0.975`, the slowest, quasi-static expansion-wave collapse, VENDORED (Shu 1977, and independently
/// corroborated by Hunter 1977 p.838 which prints the same `0.975`); (2) [`CollapseModel::larson_penston`], the
/// dynamical edge `A = 8.854`, `m0 = 46.915`, the fastest collapse, VENDORED (Hunter 1977, ApJ 218, 834, read
/// source-verbatim, with Whitworth-Summers 1985 as a dual-channel corroboration at `w0 = 46.84`). A caller needing
/// one number gets the BAND, not a default.
///
/// THE CENTRAL-MEMBER CHOICE IS A CONVENTION with a recorded stability note (VENDORED and CORRECTED at the primary,
/// Ori and Piran 1988, MNRAS 234, 821, receipt `968e318b...`): the paper proves only a NECESSARY condition, so the
/// carried claim is NOT "Larson-Penston is the only stable solution". Read verbatim: the primary-direction family
/// (including homogeneous collapse) is UNSTABLE and ruled out, and the secondary-direction family (whose best-known
/// member is Larson-Penston) SATISFIES the necessary criterion, but the paper states outright it "does not show that
/// the secondary-direction ... solution is stable". So LP is the surviving candidate, not a proven-unique stable
/// solution; Shu is the widely-used quasi-static convention; the debate continues on the failure of either endpoint
/// post-core-formation. RULED
/// (research agent, owner-signed): the end state is PURE BAND, NO DEFAULT, since a default here sits in the giant
/// verdict's path with a factor-48 alternative and an open selection debate. STAGED: today the Shu member rides as a
/// DEFAULTS-TAKEN interim (the convention line in the provenance readout, the stability note an annotation never a
/// selector); the collapse-band interval propagation through the race is its own slice; then the default dies and
/// the band ships. Choosing a member because a solar hindcast prefers it would be a licensed-calibration event
/// (ledger, spent row, owner signature), which nothing here licenses. The factor-48 framing follows from two
/// VENDORED endpoints (46.915 / 0.975), and the Ori-Piran stability note is now vendored-and-corrected above.
///
/// NAMED DEBT (flagged, not built): the REALISTIC time-dependent infall history is not constant, VENDORED and
/// CORRECTED at the primary (Foster and Chevalier 1993, ApJ 416, 303, receipt `dfd6f006...`): the central mass
/// accretion rate PEAKS at `~47 c_s^3/G` at `r = 0` immediately after core formation and declines sharply
/// thereafter (NOT the `~13` the channel relayed, which is nowhere in the paper), and opacity is NOT in the collapse
/// dynamics (the hydro is isothermal; opacity enters only the line-profile diagnostics). This [`CollapseModel`]
/// carries a single constant-rate eigenvalue, so the contract is kept wide enough to admit a rate-LAW `Mdot(t)`
/// member later, a fetch-flagged debt. NAMED OPPORTUNITY (not a debt): the eigenvalue family's own floor is the
/// similarity ODE, so `m0(A)` is derivable in-engine, at which rung Shu Table 1 (the vendored `m0(A)` row,
/// `A = 2.00 -> 4.00` giving `m0 = 0.975 -> 5.58`, with LP on Hunter's separate secondary branch far outside that
/// range) demotes to a concordance check. The fetch specs live in
/// `docs/working/DISK_ARC_FETCH_VALUES.md`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CollapseModel {
    /// The dimensionless collapse mass-accretion eigenvalue `m0` (`Mdot = m0 * c_s^3 / G`).
    pub collapse_coefficient_m0: Fixed,
    /// The instability parameter `A` the eigenvalue conditions on (Shu 1977 Table 1), the ABSCISSA of the `m0`
    /// chord: `A = 2` is the hydrostatic singular isothermal sphere, larger `A` a more over-dense, faster collapse.
    /// Carried so the coefficient never travels without the condition it was read at.
    pub instability_parameter_a: Fixed,
}

impl CollapseModel {
    /// The Shu (1977) EXPANSION-WAVE inside-out collapse, the SLOW (hydrostatic) endpoint: `m0 = 0.975` at the
    /// instability parameter `A = 2` (Shu 1977, ApJ 214, 488, Table 1 and Table 2, the `x -> 0` core-mass
    /// eigenvalue; vendored primary sha256 `af390700604cd491d36b9dfbf9a5e767611b4f7880ae360a6d2258c224fd29d2`). A
    /// CONVENTION, not a neutral default: the widely-used quasi-static value carrying the Ori-Piran 1988 caveat that
    /// the Larson-Penston branch is the stability-SURVIVING candidate, not a proven-unique stable solution (the
    /// corrected reading on [`CollapseModel`]). A consumer needing one number reads the band.
    pub fn shu_1977() -> Self {
        CollapseModel {
            collapse_coefficient_m0: Fixed::from_ratio(975, 1000), // Shu 1977 Table 1/2 (x -> 0, A = 2)
            instability_parameter_a: Fixed::from_int(2),           // the hydrostatic SIS
        }
    }

    /// The Larson-Penston DYNAMICAL collapse, the FAST endpoint: `m0 = 46.915` at the instability parameter
    /// `A = 8.854`, ~48 times the Shu rate, the faster edge of the collapse-model band.
    ///
    /// VENDORED (the channel-relayed flag retired). Read source-verbatim from the primary: Hunter 1977, ApJ 218,
    /// 834, p.838 (`"Values of m0 are 46.915, ... for the Larson-Penston ... solutions"`) for the eigenvalue, and
    /// p.837 (`"values of P(0) ... being 8.854, ..."`) for the abscissa, which is Hunter's central-density
    /// coefficient `P(0)` (so `A = 2` for Shu and `A = 8.854` for LP both land on it). Hunter's convention (eqs. 1
    /// and 14, `M = a^3 t m(zeta)/G` with `m -> m0`) gives `Mdot = m0 c_s^3/G`, matching ours. Vendored primary
    /// sha256 `9e187e6d69cccf733734b75c7b974f287532163692514084eb511828d6a70e0f`. DUAL-CHANNEL CONFIRMED on Hunter's
    /// own pages (the scanned-typography OCR-flip guard): an independent re-read of the rendered page crops agrees
    /// with the OCR text layer exactly, `46.915` and `8.854`, no digit flip, same hash. Separately, the cross-source
    /// spread is a CLASSIFIED ROW, not absorbed by the word corroboration: Whitworth and Summers 1985 (MNRAS 214, 1,
    /// receipt `ba57e11c...`) print the same member as `w0 = 46.84` under their `(z0, w0)` parametrization
    /// (`z0 = 1.672`, no `P(0)` given, so our `A = 8.854` rests on Hunter alone). The two carry BOTH facts: they
    /// corroborate the PHYSICS (the Larson-Penston member exists at ~47x the Shu rate) and DISAGREE on the NUMBER at
    /// the third digit (46.915 versus 46.84, a 0.16% spread). CLASSIFICATION: presumed numerical-integration
    /// precision (two independent integrations of the same similarity ODE), unexplained beyond that, far below the
    /// factor-48 band so decision-irrelevant; the carried value tracks Hunter's printed `46.915`.
    pub fn larson_penston() -> Self {
        CollapseModel {
            collapse_coefficient_m0: Fixed::from_ratio(46915, 1000), // 46.915 (Hunter 1977 p.838; W&S give 46.84)
            instability_parameter_a: Fixed::from_ratio(8854, 1000), // A = P(0) = 8.854 (Hunter 1977 p.837)
        }
    }
}

/// THE BIRTH ACCRETION RATE `Mdot_0` (solar masses per Myr), DERIVED from the cloud core's own collapse rather than
/// reserved as a number. This retires the disk clock's `Mdot_0` from a tagged solar interim to a derived quantity:
/// the inside-out collapse of a singular isothermal sphere delivers mass onto the forming star-plus-disk at
/// `Mdot = m0 * c_s^3 / G`, where `c_s = (k_B*T / (mu*m_H))^(1/2)` is the ISOTHERMAL sound speed of the molecular
/// cloud core (the same `c_s` [`viscous_similarity_surface_density`] uses) and `m0` is the [`CollapseModel`] the
/// caller declares. So the birth rate falls out of the core TEMPERATURE and the gas mean molecular weight, both more
/// fundamental than an authored accretion rate: `Mdot ~ c_s^3 ~ T^(3/2)`, a warmer core collapsing faster.
///
/// THE SOUND SPEED IS ISOTHERMAL, DECLARED AND ASSERTED. `c_s = (k_B*T / (mu*m_H))^(1/2)` carries NO adiabatic
/// index: an `a = (gamma*k_B*T/(mu*m_H))^(1/2)` would inflate the rate by `gamma^(3/2)` (a factor 2.15 at
/// `gamma = 5/3`), invisible to any T-scaling test, so the isothermal form is asserted by the absolute-magnitude
/// oracle (the 10 K solar value lands near `1.5`, not the `~3.3` an adiabatic `c_s` would give). TERMS DROPPED, each
/// where it lives: isothermality is physically justified at the prestellar stage (efficient line-and-dust cooling
/// holds the core near constant `T`); spherical symmetry neglects rotation, magnetic fields, and dynamical
/// turbulence, and ROTATION re-enters downstream through the disk-size derivation (`R_1`, `t_visc`), so the dropped
/// angular momentum is relocated rather than lost, named here rather than buried.
///
/// THE MEAN MOLECULAR WEIGHT IS A CHORD OVER PHASE AND COUNTING, both fixed to core conditions. It MUST be the
/// MOLECULAR value (hydrogen as `H2` at cold core conditions, a 2.5x rate lever against atomic hydrogen) and PER
/// FREE PARTICLE (against pure-`H2` counting, a 1.26x lever), which is what [`derive_disk_gas_mean_molecular_weight`]
/// returns when passed `hydrogen_atoms_per_molecule = 2` (`mu ~ 2.34` at solar). SAME-FACT-TWO-DOORS: the core `mu`
/// and the disk `mu` are ONE ROW, the same molecular per-free-particle derivation, not two routes to arbitrate; the
/// caller passes the world's single derived `mu` here and to the disk clock alike, so no second door opens. (That
/// function's `disk_gas` NAME serves a cloud-core consumer here by SHARED SCOPE, both being cold molecular
/// `H2`-dominated gas, not a proximity grab.) TERMS DROPPED: this is valid where hydrogen is MOLECULAR; a hot-inner-
/// disk consumer, where `H2` dissociates, needs a phase dispatch before it may read this row, a named debt, flag
/// only.
///
/// THE COLLAPSE COEFFICIENT is the model-structure choice, carried on [`CollapseModel`] (Shu `A = 2` versus the
/// faster Larson-Penston `A = 8.85` endpoint, a factor ~48 band), never authored inline. The cloud-core TEMPERATURE
/// is the remaining input: a per-system birth condition (`disk_clock.cloud_core_temperature_k` in the calibration
/// manifest, interim `10 K` with a DEFAULTS-TAKEN basis naming it the COLD EDGE of the Milky-Way present-epoch
/// measured medians, owner-signature-pending), reserved until the layer-4 birth draw supplies it per-star. It bottoms
/// out at a population draw, not a further derivation: a core cannot be colder than the CMB floor, and above it the
/// equilibrium is set by the birth environment (the Layer-4 terminus). The draw conditions on environment class
/// (cluster versus field, ~2.2x in rate), cosmic epoch (the CMB floor scaling `(1+z)`, a named debt: no epoch draw
/// exists yet), and metallicity (present today as the drawn abundances), admit-the-alien a data row per system.
///
/// DORMANT: the derived replacement the slice-2 wire's `Mdot_0` interim graduates to; the giant gate reads it
/// through [`crate::giants::giant_formation_on_derived_clock`]. The wide-magnitude product (`Mdot ~ 1e17 kg/s`,
/// `m_H ~ 1e-27 kg`, `k_B ~ 1e-23 J/K`) is computed entirely in the log domain, the
/// [`viscous_similarity_surface_density`] precedent, so no unrepresentable intermediate forms and a fail-loud past
/// the `exp` ceiling. `None` on a non-physical temperature, molecular weight, or collapse coefficient, or an
/// overflow.
///
// @derives: the protostellar disk birth accretion rate Mdot_0 <- the inside-out collapse rate m0*c_s^3/G over the cloud-core temperature, the disk-gas mean molecular weight, and the declared collapse model
pub fn shu_inside_out_collapse_accretion_rate_msun_myr(
    cloud_core_temp_k: Fixed,
    mean_molecular_weight: Fixed,
    collapse: &CollapseModel,
) -> Option<Fixed> {
    if cloud_core_temp_k <= Fixed::ZERO
        || mean_molecular_weight <= Fixed::ZERO
        || collapse.collapse_coefficient_m0 <= Fixed::ZERO
    {
        return None;
    }
    // ln c_s = 0.5*(ln k_B + ln T - ln mu - ln m_H), the isothermal sound speed (SI: m/s).
    let ln_k_b = civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::BOLTZMANN.value)?;
    // ln m_H = ln(1e-3) - ln(N_A): one atomic mass unit, one gram-per-mole per amu (`fundamentals::AVOGADRO`).
    let ln_m_h = civsim_physics::saha::ln_of_decimal("1e-3")?.checked_sub(
        civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::AVOGADRO.value)?,
    )?;
    let ln_c_s = Fixed::from_ratio(1, 2).checked_mul(
        ln_k_b
            .checked_add(cloud_core_temp_k.ln())?
            .checked_sub(mean_molecular_weight.ln())?
            .checked_sub(ln_m_h)?,
    )?;
    // ln Mdot [kg/s] = ln m0 + 3*ln c_s - ln G.
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    let ln_mdot_kg_s = collapse
        .collapse_coefficient_m0
        .ln()
        .checked_add(Fixed::from_int(3).checked_mul(ln_c_s)?)?
        .checked_sub(ln_g)?;
    // ln Mdot [M_sun/Myr] = ln Mdot [kg/s] + ln(1e6 * Julian year) - ln(M_sun), the kg/s -> M_sun/Myr conversion in
    // the log domain (the `derive_disk_gas_surface_density` conversion run the other way).
    let ln_megayear_s = civsim_physics::saha::ln_of_decimal(JULIAN_YEAR_S)?
        .checked_add(civsim_physics::saha::ln_of_decimal("1e6")?)?;
    let ln_mdot_msun_myr = ln_mdot_kg_s
        .checked_add(ln_megayear_s)?
        .checked_sub(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
    // Fail loud past the representable exp ceiling rather than saturate (the surface-density precedent):
    // `ln(2^31) = 31*ln2` is the log of the representation's own maximum, an engine bound, not an owner value.
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_mdot_msun_myr >= ln_ceiling {
        return None;
    }
    Some(ln_mdot_msun_myr.exp())
}

/// The CENTRIFUGAL RADIUS `R_c` (AU): the disk BIRTH radius `R_1`, DERIVED from the collapsing core's specific
/// angular momentum rather than drawn on its own axis. A fluid element falling in from the core conserves its
/// specific angular momentum `j` and settles onto the forming disk where rotation supports it against gravity,
/// which is where `j` equals the Keplerian circular value `sqrt(G M_star r)`. Equating them gives
/// `R_c = j^2 / (G M_star)`, the classical rotating-collapse landing radius (Ulrich 1976; Cassen and Moosman
/// 1981; Terebey, Shu and Cassen 1984). This is why `R_1` is DERIVABLE and not a root (LAYER4_ROOT_CENSUS): the
/// disk's birth size follows from the core-angular-momentum root, and the resolved-disk-size demographics
/// (Tazzari 2017 gas `R_c`, the Tripathi 2017 / Andrews 2020 dust size relations) demote to VALIDATION of the
/// derived distribution. Drawing `R_1` independently while the engine owns core rotation would be two doors to one
/// fact and would author away the correlation between disk size and everything else the core's `j` sets.
///
/// The value line: ZERO reserved numbers of its own. It reads the specific angular momentum (the census's ROOT,
/// whose measured velocity-gradient distribution is the pending core-angular-momentum draw, not this kernel's to
/// set) and the stellar mass, and composes them with the fundamental `G`, solar mass, and astronomical unit. Every
/// input is a per-core ARGUMENT (the admit-the-alien test): a slower or faster core, a heavier or lighter star, is
/// a data row, never a rewrite.
///
/// The specific angular momentum enters as its NATURAL LOG in SI (`m^2 s^-1`), not as a bare `Fixed`: a
/// star-forming core carries `j ~ 1e16` to `1e18 m^2 s^-1` (Goodman et al. 1993 velocity gradients), which
/// overflows the Q32.32 range the way the wide astronomical constants do, so the caller forms `ln j` (the
/// log-valued-parameter idiom the wind coefficients already use) and the whole derivation stays in the log domain.
/// `ln R_c[AU] = 2 ln j - ln G - ln M_sun - ln(star_mass_ratio) - ln AU`, then a single `exp`.
///
/// TERMS DROPPED, named rather than hidden. First and load-bearing: MAGNETIC BRAKING is omitted, so this is the
/// pure HYDRODYNAMIC centrifugal radius and therefore an UPPER BOUND. A collapsing core threaded by field loses
/// angular momentum to the envelope during infall (the classical magnetic-braking problem), which lands the
/// material at a smaller radius than `j^2/(G M_star)` alone, so a braking-efficiency term is the named debt that a
/// magnetized-collapse follow-on multiplies in; until then the derived `R_c` is the no-braking ceiling and the
/// demographics validate whatever braking the drawn `j` distribution already folds in. Second: it is a SINGLE-SHELL
/// instantaneous radius, the landing radius of material carrying THIS `j`. In a real collapse successively outer
/// shells carry higher `j` and land farther out, so the disk outer edge grows over the accretion phase (the
/// Terebey-Shu-Cassen time dependence); the caller selects which shell's `j` defines `R_1` (the outer,
/// disk-defining shell), and the growth history is the named follow-on. Third: `M_star` is the enclosed central
/// mass, disk self-gravity dropped, valid for `M_disk << M_star`. `None` on a non-positive stellar mass or a
/// result past the representable `exp` range (it fails loud rather than saturating).
///
// @derives: the protostellar disk birth radius R_1 <- the centrifugal radius j^2/(G M_star) of the collapsing core's specific angular momentum over the enclosed stellar mass
pub fn centrifugal_radius_au(
    ln_specific_angular_momentum_si: Fixed,
    star_mass_ratio: Fixed,
) -> Option<Fixed> {
    if star_mass_ratio <= Fixed::ZERO {
        return None;
    }
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    let ln_m_sun = civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?;
    let ln_au = civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?;
    // ln R_c[AU] = 2 ln j - ln G - ln M_sun - ln(star_mass_ratio) - ln AU (the m -> AU conversion in the log domain).
    let ln_rc = Fixed::from_int(2)
        .checked_mul(ln_specific_angular_momentum_si)?
        .checked_sub(ln_g)?
        .checked_sub(ln_m_sun)?
        .checked_sub(star_mass_ratio.ln())?
        .checked_sub(ln_au)?;
    // Fail loud past the representable exp ceiling rather than saturate (the Shu-rate precedent): `ln(2^31)` is the
    // log of the representation's own maximum, an engine bound, not an owner value.
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_rc >= ln_ceiling {
        return None;
    }
    Some(ln_rc.exp())
}

/// The GOLDSMITH THERMAL-BALANCE MODEL: the cited scalar coefficients of the dark-cloud-core gas-temperature
/// balance (Goldsmith 2001, ApJ 557, 736), the fixed physics [`cloud_core_thermal_balance_temperature_k`] solves.
/// The FORM is fixed Rust; these coefficients are the paper's own numbers, a declared model the way
/// [`CollapseModel`] and [`SpinDownModel`] carry theirs, so a recalibration is a data row. The wide coefficients
/// are stored as their base-ten log (the [`XrayWindFit`] idiom) because the raw rates underflow the Q32.32 range.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GoldsmithThermalModel {
    /// `log10` of the cosmic-ray heating energy deposited per ionization `dQ` (erg). Goldsmith adopts `dQ ~ 20 eV`
    /// (`= 3.204e-11 erg`), so the heating rate is `Gamma = zeta * dQ * n(H2)` (eq. 3, linear in density, no
    /// temperature dependence). The ionization rate `zeta` is the caller's explicit argument, not baked here.
    pub cr_energy_log10_erg: Fixed,
    /// `log10` of the gas-dust collisional-coupling coefficient (eq. 15): `Lambda_gd = 2e-33 * n(H2)^2 * (T_gas -
    /// T_dust) * (T_gas/10)^0.5` erg cm^-3 s^-1, the `n^2` term that pins the gas to the dust above `n ~ 1e4`.
    pub gas_dust_coeff_log10: Fixed,
    /// The gas-dust cooling's temperature power, `0.5` (eq. 15's `(T_gas/10)^0.5`).
    pub gas_dust_temp_power: Fixed,
    /// The `10 K` reference temperature both the line-cooling `(T/10)^b` (eq. 1) and the gas-dust `(T/10)^0.5`
    /// (eq. 15) are normalized to.
    pub reference_temp_k: Fixed,
    /// `log10` of the dust radiative-cooling coefficient (eq. 13): `Lambda_dust = 6.8e-33 * (T_d/K)^6 * n(H2)`
    /// erg cm^-3 s^-1, the steep `T_d^6` thermal emission the dust balance ([`cloud_core_coupled_temperatures`])
    /// balances against the external heating.
    pub dust_cooling_coeff_log10: Fixed,
    /// `log10` of the external dust-heating coefficient (eq. 7): `Gamma_dust,ext = 3.9e-24 * n(H2) * chi`
    /// erg cm^-3 s^-1, the attenuated interstellar radiation field scaled by the caller's flux factor `chi`.
    pub dust_heating_coeff_log10: Fixed,
    /// The dust-cooling temperature power, `6` (eq. 13's `(T_d/10)`... `T_d^6`, the Planck-integrated emission).
    pub dust_cooling_temp_power: Fixed,
}

impl GoldsmithThermalModel {
    /// Goldsmith 2001, the vendored coefficients (ApJ 557, 736; DOI 10.1086/322255; citation-plus-witness in the
    /// disk_arc_literature manifest, no bytes held per the AAS licence ruling, the equations read against the
    /// Internet Archive witness): the cosmic-ray heating `dQ ~ 20 eV` (eq. 3), the gas-dust coupling `2e-33`
    /// (eq. 15), the dust cooling `6.8e-33 T_d^6 n` (eq. 13), the external dust heating `3.9e-24 n chi` (eq. 7),
    /// the `10 K` reference.
    pub fn goldsmith_2001() -> Self {
        Self {
            cr_energy_log10_erg: Fixed::from_ratio(-104942, 10_000), // log10(3.204e-11), dQ = 20 eV
            gas_dust_coeff_log10: Fixed::from_ratio(-326990, 10_000), // log10(2e-33), eq. 15
            gas_dust_temp_power: Fixed::from_ratio(1, 2),            // (T/10)^0.5, eq. 15
            reference_temp_k: Fixed::from_int(10),
            dust_cooling_coeff_log10: Fixed::from_ratio(-321675, 10_000), // log10(6.8e-33), eq. 13
            dust_heating_coeff_log10: Fixed::from_ratio(-234089, 10_000), // log10(3.9e-24), eq. 7
            dust_cooling_temp_power: Fixed::from_int(6),                  // T_d^6, eq. 13
        }
    }
}

/// The LINE-COOLING FIT `Lambda_line = a (T/10)^b` (Goldsmith 2001 eq. 1): the abundance-and-depletion-conditioned
/// power-law fit the caller reads from the vendored Table 2 (undepleted) or Table 4 (depleted) for its density and
/// depletion regime. `log10_a` is `log10` of the coefficient `a` (erg cm^-3 s^-1, which underflows Q32.32 raw), `b`
/// the temperature index. This is the abundance-set input to the balance, carried as a fit rather than a raw
/// abundance because the cooling is the tabulated CO-network result, not a closed form.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LineCoolingFit {
    /// `log10` of the line-cooling coefficient `a` in `Lambda_line = a (T/10)^b`.
    pub log10_a: Fixed,
    /// The temperature index `b`.
    pub b: Fixed,
}

/// The MOLECULAR CLOUD-CORE gas temperature `T_core` (K), DERIVED by SOLVING the Goldsmith (2001) thermal balance
/// rather than drawn from a distribution. In a well-shielded dark core the gas temperature settles where the
/// volumetric cosmic-ray heating equals the volumetric cooling: `Gamma_CR(zeta, n) = Lambda_line(n, T) +
/// Lambda_gd(n, T, T_dust)`. This is the DERIVE-FIRST terminus (route two) of the reserved
/// `disk_clock.cloud_core_temperature_k`: at this rung the birth temperature the Shu collapse reads stops being a
/// drawn interim and becomes SOLVED from the core's own conditions, and the measured Jijina distribution demotes to
/// a validation hindcast (the derived `T` at dark-core inputs must land inside the surveyed ~12 to 20 K spread).
///
/// The value line, and why it needs no schema: every physical input is an EXPLICIT named argument, the
/// [`centrifugal_radius_au`] precedent, so the kernel owns the balance and nothing else and depends on no data
/// layout. The cosmic-ray ionization rate enters in units of `1e-17 s^-1` (its dark-core scale, so the order-one
/// argument stays representable), the H2 number density in `cm^-3`, the dust temperature in K (the radiation-field
/// coupled sink the gas-dust term pulls toward), the line-cooling fit `(log10 a, b)` of `Lambda_line = a
/// (T/10)^b` (eq. 1, the abundance-and-depletion-conditioned Table 2 / Table 4 fit the caller supplies for its
/// regime), and the CMB floor. The cited scalar coefficients live in [`GoldsmithThermalModel`]. Zero fabricated
/// values: the model numbers are Goldsmith's own (vendored), the state is per-core data (admit the alien).
///
/// Solved by bounded bisection over `[cmb_floor, t_hi]` (the [`disk_midplane_temperature`] pattern): at a trial
/// `T` the net `Gamma - Lambda_line - Lambda_gd` is positive when heating wins (the gas wants to be hotter, raise
/// the floor) and negative when cooling wins. Each rate is formed in the log domain and exponentiated into a
/// common scaled linear domain (units of `1e-24 erg cm^-3 s^-1`) so the tiny CGS rates neither underflow nor lose
/// their SIGN: the gas-dust term is signed by `T - T_dust`, so below the dust temperature it HEATS the gas rather
/// than cooling it, which a pure-log sum could not represent. The result is clamped at the CMB floor (a core
/// cannot be colder than the microwave background; the epoch `(1+z)` scaling of that floor is the named debt).
///
/// TERMS DROPPED, named at the site. Photoelectric and UV heating are omitted, valid for a well-shielded core and
/// the named debt at the irradiated edge or PDR. Turbulent and compressional (adiabatic) heating are omitted,
/// valid for a quiescent core and named at the collapsing or shocked edge. The line cooling is the dominant CO
/// network folded into the caller's `(a, b)` fit; the fuller coolant set and the optical-depth moderation of
/// depletion (Goldsmith's finding that a hundredfold abundance drop cuts the cooling only a few fold) live in that
/// fit, which the caller reads from the vendored table. The dust temperature is taken as an explicit input rather
/// than co-solved from the attenuated interstellar field through the dust balance (eqs. 13, 18). That co-solve now
/// exists as [`cloud_core_coupled_temperatures`], which makes the radiation-field scaling `chi` the argument and
/// derives `T_dust` too; this gas-only entry stays for the caller that already knows its dust temperature. `None`
/// on a non-physical input, a bracket that does not straddle the floor, or a rate past the representable range.
///
// @derives: the molecular cloud-core gas temperature T_core <- the Goldsmith thermal balance of cosmic-ray heating against gas-dust coupling and molecular line cooling, over the ionization rate, density, dust temperature, line-cooling fit, and the CMB floor
pub fn cloud_core_thermal_balance_temperature_k(
    cosmic_ray_ionization_rate_per_1e17_s: Fixed,
    h2_number_density_cm3: Fixed,
    dust_temperature_k: Fixed,
    line_cooling: LineCoolingFit,
    cmb_floor_k: Fixed,
    model: &GoldsmithThermalModel,
    t_hi: Fixed,
) -> Option<Fixed> {
    if cosmic_ray_ionization_rate_per_1e17_s <= Fixed::ZERO
        || h2_number_density_cm3 <= Fixed::ZERO
        || dust_temperature_k <= Fixed::ZERO
        || cmb_floor_k <= Fixed::ZERO
        || line_cooling.b <= Fixed::ZERO
        || t_hi <= cmb_floor_k
    {
        return None;
    }
    fn exp_guarded(x: Fixed, ln_ceiling: Fixed) -> Option<Fixed> {
        if x >= ln_ceiling {
            None
        } else {
            Some(x.exp())
        }
    }
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    let ln10 = Fixed::from_int(10).ln();
    let ln_n = h2_number_density_cm3.ln();
    let two_ln_n = Fixed::from_int(2).checked_mul(ln_n)?;
    // The common scale R = 1e-24 erg cm^-3 s^-1: the dark-core cooling magnitude, so every rate divided by it is
    // order one to order a few thousand across n = 1e3 to 1e6, representable in Q32.32.
    let ln_scale = civsim_physics::saha::ln_of_decimal("1e-24")?;
    let ln_ref_t = model.reference_temp_k.ln();
    // ln Gamma_CR = ln(zeta) + ln(dQ) + ln(n), with zeta = argument * 1e-17 s^-1 and ln(dQ) = log10(dQ) * ln(10).
    let ln_zeta = cosmic_ray_ionization_rate_per_1e17_s
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal("1e-17")?)?;
    let ln_gamma = ln_zeta
        .checked_add(model.cr_energy_log10_erg.checked_mul(ln10)?)?
        .checked_add(ln_n)?;
    let gamma_scaled = exp_guarded(ln_gamma.checked_sub(ln_scale)?, ln_ceiling)?;
    let ln_a = line_cooling.log10_a.checked_mul(ln10)?;
    let ln_gd_coeff = model.gas_dust_coeff_log10.checked_mul(ln10)?;
    // net(T) = (Gamma - Lambda_line - Lambda_gd) / R, the sign the bisection reads.
    let net = |t: Fixed| -> Option<Fixed> {
        if t <= Fixed::ZERO {
            return None;
        }
        let ln_t_over_ref = t.ln().checked_sub(ln_ref_t)?;
        let ln_line = ln_a.checked_add(line_cooling.b.checked_mul(ln_t_over_ref)?)?;
        let line_scaled = exp_guarded(ln_line.checked_sub(ln_scale)?, ln_ceiling)?;
        let dt = t.checked_sub(dust_temperature_k)?;
        let gd_scaled = if dt == Fixed::ZERO {
            Fixed::ZERO
        } else {
            let abs_dt = if dt < Fixed::ZERO {
                Fixed::ZERO.checked_sub(dt)?
            } else {
                dt
            };
            let ln_gd = ln_gd_coeff
                .checked_add(two_ln_n)?
                .checked_add(abs_dt.ln())?
                .checked_add(model.gas_dust_temp_power.checked_mul(ln_t_over_ref)?)?;
            let mag = exp_guarded(ln_gd.checked_sub(ln_scale)?, ln_ceiling)?;
            if dt < Fixed::ZERO {
                Fixed::ZERO.checked_sub(mag)? // below the dust temperature the coupling HEATS the gas
            } else {
                mag
            }
        };
        gamma_scaled
            .checked_sub(line_scaled)?
            .checked_sub(gd_scaled)
    };
    let mut lo = cmb_floor_k;
    let mut hi = t_hi;
    for _ in 0..60 {
        let mid = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
        if net(mid)? > Fixed::ZERO {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
    Some(if t < cmb_floor_k { cmb_floor_k } else { t })
}

/// The coupled gas AND dust temperatures of a dark cloud core, both DERIVED.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CoupledCoreTemperatures {
    /// The derived gas temperature `T_gas` (K), the birth temperature the Shu collapse reads.
    pub gas_temperature_k: Fixed,
    /// The derived dust temperature `T_dust` (K), no longer a supplied input.
    pub dust_temperature_k: Fixed,
}

/// The COUPLED gas-and-dust thermal balance: `T_gas` AND `T_dust` both DERIVED from the radiation field, closing
/// the seam the gas-only [`cloud_core_thermal_balance_temperature_k`] left open (its `T_dust` was an explicit
/// input). It solves the two Goldsmith (2001) balances together: the gas balance (eq. 17)
/// `Gamma_CR = Lambda_line + Lambda_gd`, and the dust balance (eq. 18)
/// `Gamma_dust,ext - Lambda_dust + Lambda_gd = 0`, so the radiation-field scaling `chi` (the attenuated
/// interstellar field, `~1e-4` to `1e-5` for a well-shielded dark core) becomes the argument and `T_dust` is
/// derived rather than supplied, converting a handed-in quantity into a solved one.
///
/// The dust heating is the attenuated ISRF (eq. 7) `Gamma_dust,ext = 3.9e-24 * n * chi`; the dust cooling is the
/// steep thermal re-emission (eq. 13) `Lambda_dust = 6.8e-33 * T_d^6 * n`; the gas-dust coupling (eq. 15) carries
/// energy from gas to dust when `T_gas > T_dust`. NESTED solve: bisect on `T_dust`, and at each trial solve the gas
/// balance for `T_gas` on that dust temperature, then read the dust residual. The steep `T_d^6` cooling makes the
/// residual monotone in `T_dust`, so the bracket converges cleanly. Every rate is formed in the log domain and
/// combined in the scaled linear domain (the gas-solver discipline). Zero fabricated values: the coefficients are
/// Goldsmith's own, read against the licenced Internet Archive witness (no bytes shipped).
///
/// TERMS DROPPED: the reradiation of far-infrared photons between grains is omitted (valid for a dark core where
/// reradiation is at long wavelengths, Goldsmith's stated approximation), and `chi` is the caller's per-environment
/// argument (its derivation from the visual extinction and column density is the named debt). `None` on a
/// non-physical input or a solve that does not bracket.
///
// @derives: the coupled cloud-core gas and dust temperatures <- the Goldsmith gas-plus-dust thermal balance over the ionization rate, density, radiation-field chi, line-cooling fit, and CMB floor
pub fn cloud_core_coupled_temperatures(
    cosmic_ray_ionization_rate_per_1e17_s: Fixed,
    h2_number_density_cm3: Fixed,
    radiation_field_chi: Fixed,
    line_cooling: LineCoolingFit,
    cmb_floor_k: Fixed,
    model: &GoldsmithThermalModel,
    t_hi: Fixed,
) -> Option<CoupledCoreTemperatures> {
    if cosmic_ray_ionization_rate_per_1e17_s <= Fixed::ZERO
        || h2_number_density_cm3 <= Fixed::ZERO
        || radiation_field_chi <= Fixed::ZERO
        || cmb_floor_k <= Fixed::ZERO
        || line_cooling.b <= Fixed::ZERO
        || t_hi <= cmb_floor_k
    {
        return None;
    }
    fn exp_guarded(x: Fixed, ln_ceiling: Fixed) -> Option<Fixed> {
        if x >= ln_ceiling {
            None
        } else {
            Some(x.exp())
        }
    }
    let solve_gas = |t_dust: Fixed| -> Option<Fixed> {
        cloud_core_thermal_balance_temperature_k(
            cosmic_ray_ionization_rate_per_1e17_s,
            h2_number_density_cm3,
            t_dust,
            line_cooling,
            cmb_floor_k,
            model,
            t_hi,
        )
    };
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    let ln10 = Fixed::from_int(10).ln();
    let ln_n = h2_number_density_cm3.ln();
    let two_ln_n = Fixed::from_int(2).checked_mul(ln_n)?;
    let ln_scale = civsim_physics::saha::ln_of_decimal("1e-24")?;
    let ln_ref_t = model.reference_temp_k.ln();
    // Gamma_dust,ext = 3.9e-24 * n * chi (eq. 7), constant in T_dust.
    let ln_gamma_dust = model
        .dust_heating_coeff_log10
        .checked_mul(ln10)?
        .checked_add(ln_n)?
        .checked_add(radiation_field_chi.ln())?;
    let gamma_dust_scaled = exp_guarded(ln_gamma_dust.checked_sub(ln_scale)?, ln_ceiling)?;
    let ln_dust_cool_coeff = model.dust_cooling_coeff_log10.checked_mul(ln10)?;
    let ln_gd_coeff = model.gas_dust_coeff_log10.checked_mul(ln10)?;
    // The dust-balance residual at a trial T_dust: positive means net dust heating (T_dust wants higher). The steep
    // T_d^6 cooling makes it monotone decreasing in T_dust, so the bracket converges.
    let dust_residual = |t_dust: Fixed| -> Option<Fixed> {
        if t_dust <= Fixed::ZERO {
            return None;
        }
        let t_gas = solve_gas(t_dust)?;
        // Lambda_dust = 6.8e-33 * T_d^6 * n (eq. 13).
        let ln_dust_cool = ln_dust_cool_coeff
            .checked_add(model.dust_cooling_temp_power.checked_mul(t_dust.ln())?)?
            .checked_add(ln_n)?;
        let lambda_dust_scaled = exp_guarded(ln_dust_cool.checked_sub(ln_scale)?, ln_ceiling)?;
        // Lambda_gd, signed by T_gas - T_dust: it HEATS the dust (positive) when the gas is hotter.
        let dt = t_gas.checked_sub(t_dust)?;
        let gd_scaled = if dt == Fixed::ZERO {
            Fixed::ZERO
        } else {
            let abs_dt = if dt < Fixed::ZERO {
                Fixed::ZERO.checked_sub(dt)?
            } else {
                dt
            };
            let ln_gd = ln_gd_coeff
                .checked_add(two_ln_n)?
                .checked_add(abs_dt.ln())?
                .checked_add(
                    model
                        .gas_dust_temp_power
                        .checked_mul(t_gas.ln().checked_sub(ln_ref_t)?)?,
                )?;
            let mag = exp_guarded(ln_gd.checked_sub(ln_scale)?, ln_ceiling)?;
            if dt < Fixed::ZERO {
                Fixed::ZERO.checked_sub(mag)?
            } else {
                mag
            }
        };
        gamma_dust_scaled
            .checked_sub(lambda_dust_scaled)?
            .checked_add(gd_scaled)
    };
    let mut lo = cmb_floor_k;
    let mut hi = t_hi;
    for _ in 0..60 {
        let mid = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
        if dust_residual(mid)? > Fixed::ZERO {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t_dust_raw = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
    let dust_temperature_k = if t_dust_raw < cmb_floor_k {
        cmb_floor_k
    } else {
        t_dust_raw
    };
    let gas_temperature_k = solve_gas(dust_temperature_k)?;
    Some(CoupledCoreTemperatures {
        gas_temperature_k,
        dust_temperature_k,
    })
}

/// The VISUAL EXTINCTION `A_V` (magnitudes) to a cloud-core center, DERIVED from the core's own hydrogen column
/// density and the cited gas-to-extinction ratio: `A_V = N_H / (N_H/A_V)`. The column (order `1e22` per cm^2) and
/// the ratio (order `1e21` per cm^2 per mag) both overflow fixed point, so the caller passes their base-10
/// LOGARITHMS and the division is a subtraction in the log domain, `log10(A_V) = log10(N_H) - log10(N_H/A_V)`, then
/// exponentiated to the representable `A_V` (order 1 to 100 for a core). DERIVED, no authored value: `log10_column`
/// is the core's own column (a drawn or derived environment quantity) and `log10_gas_to_extinction_ratio` is the
/// cited Bohlin, Savage and Drake 1978 / Guver and Ozel 2009 constant (`N_H/A_V ~ 1.87e21` to `2.21e21`, vendored in
/// `disk_arc_literature`). ADMITS THE ALIEN: keyed on the core's own column against a cited dust law, so a
/// different dust-to-gas world is a data row. `None` if the log-domain result overflows the representable range.
// @derives: the visual extinction A_V to a cloud-core center <- the core's hydrogen column density over the cited gas-to-extinction ratio (Bohlin 1978 / Guver-Ozel 2009)
pub fn visual_extinction_magnitudes(
    log10_column_h_cm2: Fixed,
    log10_gas_to_extinction_ratio_cm2_per_mag: Fixed,
) -> Option<Fixed> {
    let ln10 = Fixed::from_int(10).ln();
    let log10_a_v = log10_column_h_cm2.checked_sub(log10_gas_to_extinction_ratio_cm2_per_mag)?;
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    let ln_a_v = log10_a_v.checked_mul(ln10)?;
    if ln_a_v >= ln_ceiling {
        return None;
    }
    Some(ln_a_v.exp())
}

/// The BROADBAND DUST-HEATING ATTENUATION ESTIMATOR: the effective attenuation efficiency `k` per magnitude of
/// visual extinction, held as a BAND `[k_lo, k_hi]` rather than a single value because it is `A_V`-dependent and no
/// single constant is the physical model. Zucconi, Walmsley and Galli 2001 (vendored) give a per-frequency
/// radiative-transfer solution whose EFFECTIVE broadband `k` runs about `0.05` (deep cores, where the penetrating
/// far-IR takes over) to about `0.13` (shallow cores). This model carries that range so the estimator it feeds
/// returns a band, never a laundered point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExtinctionChiEstimator {
    /// The shallow-attenuation edge (per magnitude), the smaller `k` toward which deep cores flatten.
    pub k_lo: Fixed,
    /// The steep-attenuation edge (per magnitude), the larger `k` of shallow cores.
    pub k_hi: Fixed,
}

impl ExtinctionChiEstimator {
    /// Zucconi, Walmsley and Galli 2001's broadband dust-heating range, `k` about `0.05` to `0.13` per magnitude
    /// (vendored receipt in `disk_arc_literature`). NOT the V-band `0.4`: the V-band flux attenuation
    /// `10^(-A_V/2.5)` grossly over-attenuates the DUST-HEATING field, which is broadband (optical-NIR, far-IR,
    /// mid-IR, the cosmic background) and penetrates a dark core from frequencies where the optical depth is of
    /// order one, far shallower than the V band.
    pub fn zucconi_2001() -> Self {
        Self {
            k_lo: Fixed::from_ratio(5, 100),
            k_hi: Fixed::from_ratio(13, 100),
        }
    }
}

/// A banded log-domain radiation-field scaling `chi`, the [`ExtinctionChiEstimator`] output. Held as `log10(chi)`
/// so a deep core (a `chi` far below the fixed-point floor) stays REPRESENTABLE rather than underflowing to a
/// false zero or a refusal: the linear value is recovered only on demand, and only that recovery can fail.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChiEstimateBand {
    /// `log10(chi)` at the deepest attenuation (the larger `k`), the band's lower `chi` edge.
    pub log10_chi_lo: Fixed,
    /// `log10(chi)` at the shallowest attenuation (the smaller `k`), the band's upper `chi` edge.
    pub log10_chi_hi: Fixed,
}

impl ChiEstimateBand {
    /// The linear `chi` upper edge, `10^(log10_chi_hi)`. `None` if it overflows the representable range.
    pub fn linear_hi(&self) -> Option<Fixed> {
        let ln10 = Fixed::from_int(10).ln();
        let ln_val = self.log10_chi_hi.checked_mul(ln10)?;
        let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
        if ln_val >= ln_ceiling {
            return None;
        }
        Some(ln_val.exp())
    }

    /// The linear `chi` lower edge, `10^(log10_chi_lo)`. `None` if a deep core drives it below the fixed-point
    /// floor: the honest limit of a LINEAR carrier, surfaced here rather than swallowed, since the log edges above
    /// stay live for a consumer that carries `log10(chi)` through the thermal balance.
    pub fn linear_lo(&self) -> Option<Fixed> {
        let ln10 = Fixed::from_int(10).ln();
        let chi = self.log10_chi_lo.checked_mul(ln10)?.exp();
        (chi > Fixed::ZERO).then_some(chi)
    }
}

/// A LAYER-3 ESTIMATE of the radiation-field scaling `chi` from the visual extinction, NOT a derivation and NOT a
/// retirement of the reserved `chi` the coupled cloud-core temperature solve ([`cloud_core_coupled_temperatures`])
/// reads. The interstellar field that heats core dust is attenuated by the core's extinction, modelled here to
/// FIRST ORDER as `chi = 10^(-A_V * k)` over the [`ExtinctionChiEstimator`] band. This is a coordinate change on a
/// single effective `k`, a stand-in for Zucconi's per-frequency `exp(-tau_nu)` radiative transfer, so it is graded
/// an estimator: the true `chi(A_V)` is spectral (the far-IR takes over as the core deepens, flattening `k`), and
/// deriving it needs the broadband dust-heating integral over the external SED, the dust opacity, the column, and
/// the geometry, a substrate this does not build. The band `[k_lo, k_hi]` is carried through so the output is an
/// interval, and it is returned in the LOG domain ([`ChiEstimateBand`]) so a deep core does not vanish into a false
/// zero: the representation-liveness rule says a large extinction convicts a linear carrier, not the core's
/// existence. `None` only on a negative extinction or an inverted `k` band.
pub fn radiation_field_chi_estimate_from_extinction(
    a_v_magnitudes: Fixed,
    estimator: &ExtinctionChiEstimator,
) -> Option<ChiEstimateBand> {
    if a_v_magnitudes < Fixed::ZERO
        || estimator.k_lo <= Fixed::ZERO
        || estimator.k_hi < estimator.k_lo
    {
        return None;
    }
    // log10(chi) = -A_V * k. The steeper k (k_hi) gives the deeper attenuation, so it forms the lower chi edge.
    let log10_chi_lo = Fixed::ZERO.checked_sub(a_v_magnitudes.checked_mul(estimator.k_hi)?)?;
    let log10_chi_hi = Fixed::ZERO.checked_sub(a_v_magnitudes.checked_mul(estimator.k_lo)?)?;
    Some(ChiEstimateBand {
        log10_chi_lo,
        log10_chi_hi,
    })
}

/// The DISK ACCRETION-RATE CLOCK (the disk-evolution arc, slice 1): the Lynden-Bell-Pringle self-similar decline
/// (Hartmann et al. 1998), `Mdot(t) = Mdot_0 * (1 + t / t_visc) ^ (-p)` with the decline exponent
/// `p = (5/2 - gamma) / (2 - gamma)` set by the viscous-spreading exponent `gamma` (the same `gamma ~ 1` gate-G
/// retired the disk profile onto, giving `p = 3/2`). At `t = 0` the rate is `Mdot_0` (the hot accreting formation
/// epoch); it declines monotonically toward zero as `t` grows, so the disk lifetime becomes a derived output of
/// the clock rather than a consulted constant.
///
/// The value line, per the arc ruling: NONE of these is an owner-set scalar. `mdot_0_msun_myr` is the per-disk
/// `M_star`-conditioned draw (interim: the solar pin, loudly tagged; destination: the layer-4 accretion draw),
/// `t_visc_myr` is DERIVED from the disk's alpha closure and thermal structure (`R_1^2 / (3 nu(R_1))`), and
/// `gamma` is the disk's own viscous-spreading exponent. This function is fixed Rust, PARAMETRIC over all three,
/// with the sources upgraded behind the signature. It is a viscous-transport instance of the declared
/// model-structure band (the MHD wind-driven rival carries a different decline); the caller owns which branch.
///
/// TERMS DROPPED: the Lynden-Bell-Pringle similarity assumes the viscosity `nu(r)` is STATIONARY in time. A real
/// disk's viscous heating declines with the accretion rate, so the temperature and therefore `nu` decline too,
/// which steepens the true decline past this similarity form. The omission is VALID where irradiation sets the
/// temperature, the outer disk where the scale radius `R_1` lives, and INVALID in the viscously-heated inner
/// disk, which contributes little to `t_visc` at the scale radius. The assumption is chosen out loud here,
/// alongside the already-declared wind-driven model-structure band, so the domain is stated rather than implied.
///
/// Computed in the log domain for determinism (the `viscous_similarity_surface_density` precedent): `base >= 1`
/// so `base^p >= 1` and the rate never exceeds `Mdot_0`, so the only bound is underflow. Past the representable
/// `exp` ceiling the rate has fallen below what the fixed-point format can hold, so it returns `ZERO` rather than
/// a saturated value. That ceiling is a REPRESENTATION-FLOOR event, not physical dispersal: it sits about six
/// orders of magnitude below any physical disk-clearing threshold and is unreachable in real operation, and
/// dispersal (the disk clearing) is the slice-2 wind-versus-accretion race, never the number format. The
/// `ZERO` return is graceful arithmetic degradation; a fully declined rate is a readable physical zero, whereas an
/// out-of-domain orbit in the surface-density function is an error, which is why this returns `ZERO` and that one
/// returns `None`. At `age = 0` the rate is `Mdot_0` exactly, special-cased so the identity is exact by
/// construction rather than within the ln/exp round-trip. `None` on a non-positive `Mdot_0` or `t_visc`, a
/// negative `age`, or a `gamma` outside `[0, 2)` (where the exponent is undefined).
pub fn viscous_similarity_accretion_rate(
    mdot_0_msun_myr: Fixed,
    t_visc_myr: Fixed,
    gamma: Fixed,
    age_myr: Fixed,
) -> Option<Fixed> {
    if mdot_0_msun_myr <= Fixed::ZERO
        || t_visc_myr <= Fixed::ZERO
        || age_myr < Fixed::ZERO
        || gamma < Fixed::ZERO
        || gamma >= Fixed::from_int(2)
    {
        return None;
    }
    // Identity by construction at t = 0: base would be 1 and the rate Mdot_0, but the ln/exp round-trip of 1 is
    // exact only to a ULP, so the zero-age case returns Mdot_0 directly.
    if age_myr == Fixed::ZERO {
        return Some(mdot_0_msun_myr);
    }
    // p = (5/2 - gamma) / (2 - gamma); p = 3/2 at gamma = 1.
    let p = Fixed::from_ratio(5, 2)
        .checked_sub(gamma)?
        .checked_div(Fixed::from_int(2).checked_sub(gamma)?)?;
    // base = 1 + age / t_visc >= 1, so ln(base) >= 0 and base^p >= 1.
    let base = Fixed::ONE.checked_add(age_myr.checked_div(t_visc_myr)?)?;
    let exponent = p.checked_mul(base.ln())?;
    // A REPRESENTATION-FLOOR guard, not physical dispersal (which the slice-2 race owns): past the exp ceiling the
    // rate is below what the format can hold, so return ZERO rather than a saturated value. `ln(2^31) = 31 * ln 2`
    // is the representation's own bound (the surface-density precedent). Unreachable in real operation, ~6 orders
    // below any physical dispersal threshold.
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if exponent >= ln_ceiling {
        return Some(Fixed::ZERO);
    }
    mdot_0_msun_myr.checked_div(exponent.exp())
}

/// One (epoch, rate) LANDMARK the accretion clock is hindcast against, with the fractional band it must sit
/// within. A landmark must be a GENUINE RATE MEASUREMENT: the mature endpoint is the observed class-II accretion
/// rate at the age of the sample it came from (a fetchable rate-versus-age locus, Hartmann or Manara-class
/// compilations, with sample conditioning, not one synthetic point). The formation RATE is NOT a landmark here:
/// the 0.19 is a partition share excluded by owner directive (see [`accretion_clock_hindcasts`]); the formation
/// constraint enters as a derived-root condition instead. `band_frac` is the observational band on the rate,
/// carried with the fetched measurement, not authored here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AccretionLandmark {
    /// The epoch (Myr since `t = 0`) at which the landmark rate was measured.
    pub epoch_myr: Fixed,
    /// The measured accretion rate at that epoch (solar masses per Myr).
    pub rate_msun_myr: Fixed,
    /// The fractional band the derived rate must fall within (for example `0.1` for ten percent).
    pub band_frac: Fixed,
}

/// The INVERTED two-landmark HINDCAST gate (the arc ruling): with `Mdot_0` DRAWN and `t_visc` DERIVED, the two
/// landmarks no longer PIN the parameters, they VALIDATE them. This asks whether the clock built from the
/// independently-sourced `(mdot_0, t_visc, gamma)` passes through every landmark within its band, returning
/// `Some(true)` if so. It is a stronger check than the joint pin, which would have made the parameters exactly as
/// reserved as the numbers they retire (a change of coordinates wearing a derivation's clothes); as a consistency
/// check over independent inputs it can fail. `None` if the clock cannot be evaluated at a landmark epoch or a
/// band is non-positive.
///
/// BLINDNESS SET (rule 1): discriminating power is that it convicts any `(mdot_0, t_visc, gamma)` whose curve
/// misses a landmark epoch by more than its band. Blind, first, to a joint error that shifts `Mdot_0` and every
/// landmark rate together by the same factor (a wrong overall normalization consistent with the whole chord),
/// covered by anchoring `Mdot_0` to the independently-observed class-0/I peak-accretion band rather than to the
/// landmarks. Blind, second, to the family SHAPE: two landmarks cannot distinguish the decline family, since a
/// different `gamma` with refit `(mdot_0, t_visc)` can pass the same two points, covered by `gamma`'s own
/// provenance from gate-G (the exponent is derived, not free to refit) and by the model-structure band being
/// DECLARED rather than inferred from these landmarks. There is no third blind spot for a formation-RATE
/// landmark, because there is no such landmark: the 0.19 formation rate is EXCLUDED from the validation set by
/// owner directive. It was never a measurement, only a PARTITION SHARE: the ~1400 K condensation front fixes only
/// the PRODUCT of accretion rate, dust column, and opacity (the formation-era slice records this at its
/// `FORMATION_ACCRETION_RATE_MSUN_MYR`), so a hindcast on `Mdot(t_formation) = 0.19` could pass on a compensating
/// dust error and fail on a correct `Mdot_0`, which is a referee that convicts the right answer, not a referee.
/// The formation constraint enters instead as the DERIVED-ROOT condition it always physically was: `t_formation`
/// is the root of `T_mid(1 AU, t) = T_condensation`, solved on `Mdot(t)` through the disk's own thermal
/// structure (the same condensation-temperature module that consumed the 0.19), which now convicts `Mdot` because
/// two of the three product factors (the dust column and the opacity) have since been derived. That derived-root
/// referee is a follow-on build; this gate takes only landmarks that are genuine rate measurements.
pub fn accretion_clock_hindcasts(
    mdot_0_msun_myr: Fixed,
    t_visc_myr: Fixed,
    gamma: Fixed,
    landmarks: &[AccretionLandmark],
) -> Option<bool> {
    for landmark in landmarks {
        if landmark.band_frac <= Fixed::ZERO || landmark.rate_msun_myr <= Fixed::ZERO {
            return None;
        }
        let derived = viscous_similarity_accretion_rate(
            mdot_0_msun_myr,
            t_visc_myr,
            gamma,
            landmark.epoch_myr,
        )?;
        // |derived - stated| <= band_frac * stated: the derived rate sits within the landmark's band. Take the
        // gap as high-minus-low so no negation is needed (Fixed carries no abs).
        let (hi, lo) = if derived >= landmark.rate_msun_myr {
            (derived, landmark.rate_msun_myr)
        } else {
            (landmark.rate_msun_myr, derived)
        };
        let deviation = hi.checked_sub(lo)?;
        let allowed = landmark.band_frac.checked_mul(landmark.rate_msun_myr)?;
        if deviation > allowed {
            return Some(false);
        }
    }
    Some(true)
}

/// The DERIVED DISK-GAS MEAN MOLECULAR WEIGHT `mu` (dimensionless), mass-weighted over the world's OWN elemental
/// abundances rather than the authored solar `2.34`. It reads the drawn abundance pattern and derives
/// `mu = (total mass) / (total particles)` per hydrogen nucleus: each element contributes its number relative to
/// hydrogen `n_X/n_H = 10^(log_eps(X) - 12)` times its standard atomic weight to the mass, and its particle count
/// to the denominator, with HYDROGEN counted as a molecule of `hydrogen_atoms_per_molecule` atoms (2 for the cold
/// molecular disk, so `n_H/2` particles) and every other element counted atomically. So a metal-rich world carries
/// a heavier gas and a slightly larger `mu`, and the solar pattern reproduces the `2.34` the fixture carried, now
/// as the solar INSTANCE of a per-world derivation. `hydrogen_atoms_per_molecule` is the disk-gas regime input
/// (2 for `H2`, 1 for an atomic-hydrogen disk), keyed so an alien gas is a data row (Principle 7).
///
/// DERIVE-FIRST, and a TRAP AVOIDED: it walks the abundance ROWS (`elements` plus `preferred`), which the `[Fe/H]`
/// draw (`SolarAbundances::scaled_metals_by_dex`) correctly scales, NOT the `x_mass_fraction`/`y_mass_fraction`/
/// `z_mass_fraction` getters, which that draw leaves at the SOLAR strings (a stale fixture, surfaced separately),
/// so reading them would fix `mu` at solar for every world. The atomic weights are the periodic table's cited
/// standard values (a physics floor read, never authored here). `None` on a non-positive
/// `hydrogen_atoms_per_molecule`, an element in the pattern absent from the periodic table (a data
/// inconsistency, surfaced rather than silently dropped), or no particles.
pub fn derive_disk_gas_mean_molecular_weight(
    abundances: &civsim_physics::solar_abundances::SolarAbundances,
    periodic: &civsim_physics::periodic::PeriodicTable,
    hydrogen_atoms_per_molecule: Fixed,
) -> Option<Fixed> {
    if hydrogen_atoms_per_molecule <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let twelve = Fixed::from_int(12);
    let mut total_mass = Fixed::ZERO;
    let mut total_particles = Fixed::ZERO;
    for symbol in abundances.elements() {
        let log_eps = match abundances.preferred(symbol) {
            Some(v) => v,
            None => continue, // a row with no cited abundance carries no gas
        };
        let element = periodic.element(symbol)?;
        // The number relative to hydrogen, n_X/n_H = 10^(log_eps - 12), in the log-epsilon convention.
        let n_rel = log_eps.checked_sub(twelve)?.checked_mul(ln10)?.exp();
        total_mass = total_mass.checked_add(n_rel.checked_mul(element.standard_atomic_weight)?)?;
        // Hydrogen (Z = 1) forms molecules of `hydrogen_atoms_per_molecule` atoms, so it is that many fewer
        // particles; every other element is atomic in the gas.
        let particles = if element.z == 1 {
            n_rel.checked_div(hydrogen_atoms_per_molecule)?
        } else {
            n_rel
        };
        total_particles = total_particles.checked_add(particles)?;
    }
    if total_particles <= Fixed::ZERO {
        return None;
    }
    total_mass.checked_div(total_particles)
}

/// The DERIVED VISCOUS TIME `t_visc` (megayears), the characteristic time of the accretion clock's decline (the
/// `t_visc` argument of [`viscous_similarity_accretion_rate`]), DERIVED from the disk's own structure rather than
/// reserved. The arc ruling: this is not a free scalar; the Lynden-Bell-Pringle family defines it as the viscous
/// time at the scale radius, `t_visc = R_1^2 / (3 * nu(R_1))`, with the Shakura-Sunyaev viscosity
/// `nu = alpha * c_s * H`. Reducing the disk's own relations (`H = c_s / Omega`, `c_s^2 = k_B * T / (mu * m_H)`,
/// `Omega = sqrt(G * M_star / R_1^3)`) collapses it to
/// `t_visc = sqrt(R_1) * sqrt(G * M_star) * mu * m_H / (3 * alpha * k_B * T(R_1))`, so it reads the already-banked
/// `alpha`, the disk temperature at the scale radius, and the mean molecular weight, and derives from them.
///
/// FOUR of the five inputs have named sources (the banked `alpha`, the disk temperature, the mean molecular
/// weight from the composition chain, and the stellar mass). THE FIFTH, `scale_radius_au` (`R_1`), HAS NONE, and
/// it cannot be derived inside this arc: `R_1` is the similarity solution's INITIAL CONDITION, the disk's birth
/// size, so it is a per-system DRAW from measured disk-size demographics conditioned on stellar mass (the
/// resolved-disk size distributions, the fetch target), with the solar pin as the interim exactly like `Mdot_0`.
/// It is `r_c` today, and landing `r_c` in the gas density is this arc's own finding-1 closure, so `R_1` is on
/// slice 1's closure list draw-pending, not a settled derivation. Computed in the log domain (the
/// `viscous_similarity_surface_density` precedent), converting seconds to megayears at the end. `None` on a
/// non-positive input or an intermediate past the representable range.
pub fn derive_viscous_time_myr(
    scale_radius_au: Fixed,
    star_mass_ratio: Fixed,
    disk_temperature_k: Fixed,
    alpha_viscosity: Fixed,
    mean_molecular_weight: Fixed,
) -> Option<Fixed> {
    if scale_radius_au <= Fixed::ZERO
        || star_mass_ratio <= Fixed::ZERO
        || disk_temperature_k <= Fixed::ZERO
        || alpha_viscosity <= Fixed::ZERO
        || mean_molecular_weight <= Fixed::ZERO
    {
        return None;
    }
    // ln R_1 [m] = ln(scale_radius_au) + ln(AU).
    let ln_r1 = scale_radius_au
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?)?;
    // ln(G * M_star) = ln G + ln(star_mass_ratio) + ln(M_sun).
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    let ln_g_m_star = ln_g
        .checked_add(star_mass_ratio.ln())?
        .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
    // ln m_H = ln(1e-3) - ln(N_A): one atomic mass unit in kg (one gram per mole per amu).
    let ln_m_h = civsim_physics::saha::ln_of_decimal("1e-3")?.checked_sub(
        civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::AVOGADRO.value)?,
    )?;
    let ln_k_b = civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::BOLTZMANN.value)?;
    let half = Fixed::from_ratio(1, 2);
    // ln t_visc [s] = 0.5 ln R_1 + 0.5 ln(G M_star) + ln mu + ln m_H - ln 3 - ln alpha - ln k_B - ln T.
    let ln_t_s = half
        .checked_mul(ln_r1)?
        .checked_add(half.checked_mul(ln_g_m_star)?)?
        .checked_add(mean_molecular_weight.ln())?
        .checked_add(ln_m_h)?
        .checked_sub(Fixed::from_int(3).ln())?
        .checked_sub(alpha_viscosity.ln())?
        .checked_sub(ln_k_b)?
        .checked_sub(disk_temperature_k.ln())?;
    // Convert seconds to megayears: subtract ln(1e6 * Julian year) in the log domain.
    let ln_megayear_s = civsim_physics::saha::ln_of_decimal(JULIAN_YEAR_S)?
        .checked_add(civsim_physics::saha::ln_of_decimal("1e6")?)?;
    let ln_t_myr = ln_t_s.checked_sub(ln_megayear_s)?;
    // Fail loud past the representable exp ceiling rather than saturate (the surface-density precedent).
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_t_myr >= ln_ceiling {
        return None;
    }
    Some(ln_t_myr.exp())
}

/// The ROCHE-LOBE RADIUS (AU) of the disk-hosting star in a binary, the Eggleton 1983 fit to the Roche-potential
/// volume radius: `R_L/a = c_num * q^(2/3) / (c_log * q^(2/3) + ln(1 + q^(1/3)))`, `q = M_host/M_companion`,
/// accurate to about one percent over all `q`.
///
/// MODALITY: this is the HARD UPPER EDGE on the disk's outer radius, NOT the expected truncation radius. A
/// circumstellar disk tidally truncates INSIDE its Roche lobe, at the outermost non-overlapping Lindblad resonance
/// (Paczynski 1977, Papaloizou and Pringle 1977, Artymowicz and Lubow 1994), at `R_t = f * R_L` with the
/// resonance-truncation FRACTION `f` now VENDORED ([`resonant_truncation_fraction`], the Pichardo 2005 fit
/// `f = 0.733 (1 - e)^1.20 q^0.07`, `q` the companion mass fraction), so [`tidally_capped_scale_radius_au`] caps
/// `R_1` at the expected `f * R_L`
/// rather than this conservative edge. `t_visc` and `tau_disk` inherit the resulting `sqrt(f)` band (a class effect,
/// wider at high eccentricity) through [`derive_viscous_time_myr`] (`t_visc ~ sqrt(R_1)`), the machinery already
/// built rather than a new path. This Roche-lobe radius stays the outer bound of that band.
///
/// SEPARATION CONVENTION: `separation_au` is the SEMI-MAJOR AXIS `a`, so this is the Roche lobe at the mean
/// separation. In an eccentric binary the tide is strongest at periastron, where the instantaneous lobe is
/// smaller (the periastron lobe sits inside the semi-major lobe), and the real truncation sits inside that, so a
/// semi-major evaluation is the OUTER, most conservative edge: eccentricity only LOOSENS this bound further above
/// the true truncation, never tightens it past the true value. The bound's conservative character therefore
/// SURVIVES eccentricity, which is why the eccentricity dependence is a doc convention here and not a code term.
/// The tightening arrives with the fetched `f(q, e, viscosity)` fraction, which turns the one-sided bound into a
/// band.
///
/// ZERO new per-system free values: the Roche fraction derives from the mass ratio, and the truncation fraction
/// `f` enters as a fetched (q, e, viscosity)-conditioned banded class row, not an owner scalar. `c_num` (~0.49)
/// and `c_log` (~0.6) are Eggleton's cited fit to the Roche-potential volume radius: cited-universal and
/// mass-ratio-only (material-free, not an owner tunable), but a FIT accurate to about one percent, so it carries
/// its own accuracy band, unlike an exact constant such as `pi`. Passed as parameters. ADMITS THE ALIEN: it keys
/// on the mass ratio and separation, the binary's own data, no Terran assumption. `None` on a non-positive input.
pub fn roche_lobe_radius_au(
    separation_au: Fixed,
    mass_ratio_host_to_companion: Fixed,
    eggleton_numerator_coeff: Fixed,
    eggleton_log_coeff: Fixed,
) -> Option<Fixed> {
    if separation_au <= Fixed::ZERO || mass_ratio_host_to_companion <= Fixed::ZERO {
        return None;
    }
    let q = mass_ratio_host_to_companion;
    let q_third = q.powf(Fixed::from_ratio(1, 3)); // q^(1/3)
    let q_two_thirds = q.powf(Fixed::from_ratio(2, 3)); // q^(2/3)
                                                        // R_L/a = c_num * q^(2/3) / (c_log * q^(2/3) + ln(1 + q^(1/3))).
    let denom = eggleton_log_coeff
        .checked_mul(q_two_thirds)?
        .checked_add(Fixed::ONE.checked_add(q_third)?.ln())?;
    let fraction = eggleton_numerator_coeff
        .checked_mul(q_two_thirds)?
        .checked_div(denom)?;
    separation_au.checked_mul(fraction)
}

/// The RESONANT DISK-TRUNCATION FRACTION `f = R_t / R_L`: the fraction of the host's Roche-lobe radius at which a
/// circumstellar disk truncates under the companion's resonant torques, the cited closed fit
/// `f = c (1 - e)^p_e q^p_f` (Pichardo, Sparke and Aguilar 2005, Eq. 6, VERIFIED source-verbatim against the held
/// scan p.524: `R_d ~ R_d,Egg * 0.733 (1 - e)^1.20 q^0.07`). The symbol `q` is the COMPANION MASS FRACTION
/// `M_2 / (M_1 + M_2)` (Pichardo Eq. 5 and Figs 3 to 4), NOT a mass ratio, over `q in [0.01, 0.99]`, `e in [0, 0.9]`,
/// to about 6.5 percent. The disk edge sits INSIDE the Roche lobe: `f` runs from about 0.53 (a low mass fraction) to
/// 0.73 (a companion-dominated split), weakly increasing with the mass fraction (the exponent is 0.07, about 17
/// percent per decade of `q`, weak but NOT negligible), and tightening sharply with eccentricity (`f ~ 0.32` at
/// `e = 0.5`, about `0.046` at `e = 0.9`, at the high-mass-fraction end) as the companion's closest approach carves
/// the disk back.
///
/// CORRECTION OF RECORD: the `q` exponent is 0.07, not the 0.01 an earlier OCR text-layer read reported (a
/// superscript misread, caught in the derive-first audit and corrected against the held page image). The coefficients
/// are CITED data (Principle 11), carried by [`DiskTruncationFit`]; the mechanism is fixed Rust. ADMITS THE ALIEN:
/// keyed on the binary's own eccentricity and mass fraction.
///
/// THIS IS ONE RUNG, NOT THE UNIVERSAL EDGE. Pichardo is the COPLANAR, DISSIPATIONLESS invariant-loop determination
/// (an estimator ceiling, no viscosity). Two SEPARATE rungs stay distinct and are NOT folded into these
/// coefficients: the Artymowicz and Lubow 1994 SPH edge sits LOWER and moves OUTWARD with disk viscosity (the edge
/// is the outermost resonance the viscous torque cannot overflow), and the Manara / Papaloizou-Pringle
/// viscous-torque fit (a distinct `h mu^k` form that consumes `alpha_viscosity` and `H/r` through a Reynolds state)
/// is its own type, built only when a consumer needs the viscosity-conditioned edge. A viscous disk's true
/// truncation is a band between the SPH edge and this dissipationless bound. The fit's OWN validity travels with
/// its coefficients (the [`ConvectiveTurnoverFit`] precedent): the source fitted `e in [0, 0.90]` and `q in
/// [0.01, 0.99]` to about 6.5 percent, so an evaluation outside that box REFUSES rather than extrapolating the fit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiskTruncationFit {
    /// The fit coefficient `c` (the `q -> 1` circular-orbit limit of `f`). Cited (Pichardo 2005 Eq. 6, 0.733).
    pub circular_fraction: Fixed,
    /// The exponent on `(1 - e)`, the eccentricity tightening. Cited (1.20).
    pub eccentricity_exponent: Fixed,
    /// The exponent on the companion mass fraction `q = M_2/(M_1+M_2)`, weakly positive. Cited (0.07, NOT the 0.01
    /// an OCR text layer misread; corrected against the held scan p.524).
    pub mass_fraction_exponent: Fixed,
    /// The fractional fit accuracy, carried so the fraction ships as a BAND `f * (1 +/- this)` rather than a point.
    /// Cited (Pichardo 2005 p.524, `+/- 6.5 percent`).
    pub fit_error_fraction: Fixed,
    /// The highest eccentricity the fit was measured over (Pichardo p.524, `0.90`); above it the evaluation refuses.
    pub valid_ecc_max: Fixed,
    /// The lowest companion mass fraction the fit was measured over (Pichardo p.524, `0.01`).
    pub valid_mass_fraction_min: Fixed,
    /// The highest companion mass fraction the fit was measured over (Pichardo p.524, `0.99`).
    pub valid_mass_fraction_max: Fixed,
}

impl DiskTruncationFit {
    /// The Pichardo, Sparke and Aguilar 2005 invariant-loop fit (MNRAS 359, 521, Eq. 6, held-scan p.524), as cited
    /// data: `f = 0.733 (1 - e)^1.20 q^0.07`, `q` the companion mass fraction, fitted over `e in [0, 0.90]` and
    /// `q in [0.01, 0.99]` to `+/- 6.5 percent`. Vendored in `disk_arc_literature` (`pichardo_2005`).
    pub fn pichardo_2005() -> Self {
        DiskTruncationFit {
            circular_fraction: Fixed::from_ratio(733, 1000), // 0.733
            eccentricity_exponent: Fixed::from_ratio(120, 100), // 1.20
            mass_fraction_exponent: Fixed::from_ratio(7, 100), // 0.07 (verified p.524, not the OCR-misread 0.01)
            fit_error_fraction: Fixed::from_ratio(65, 1000),   // +/- 6.5 percent (p.524)
            valid_ecc_max: Fixed::from_ratio(90, 100),         // e in [0, 0.90]
            valid_mass_fraction_min: Fixed::from_ratio(1, 100), // q in [0.01, ...]
            valid_mass_fraction_max: Fixed::from_ratio(99, 100), // q in [..., 0.99]
        }
    }
}

/// Which circumstellar disc a truncation radius describes. Pichardo Eq. 6 (the invariant-loop fit) is the disc
/// around the PRIMARY (the accretor hosting the modelled disc); the paper's separate secondary-disc rule
/// (`0.4 +/- 0.03` of the Lagrange radius times `(1 - e)`, p.526) is a DIFFERENT relation. Keying the evaluation on
/// the component stops the two from being silently swapped, the ambiguity equal-mass tests cannot expose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CircumstellarComponent {
    /// The disc around the primary (host) star, the regime Pichardo Eq. 6 fits.
    Primary,
    /// The disc around the secondary (companion) star, governed by the distinct secondary-disc rule.
    Secondary,
}

/// The physical MODALITY a truncation radius is drawn from: these are SEPARATE rungs, not one number. The
/// dissipationless invariant-loop UPPER bound (Pichardo 2005) sits above the viscous SPH resonant edge
/// (Artymowicz-Lubow 1994), which sits below the tidal-torque-balance radius (Papaloizou-Pringle 1977, the
/// near-Roche-lobe high-viscosity limit). A consumer that needs the viscous edge reads a different rung, not this
/// one relabelled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TruncationModality {
    /// Pichardo 2005 coplanar dissipationless invariant-loop determination: an UPPER bound (no viscosity).
    InvariantLoopUpperBound,
    /// Artymowicz-Lubow 1994 viscous SPH resonant edge: LOWER, moves outward with disk viscosity.
    ViscousSphResonant,
    /// Papaloizou-Pringle 1977 tidal-torque-balance radius: the near-Roche-lobe high-viscosity limit.
    TidalTorqueBalance,
}

/// A banded truncation FRACTION `f = R_t / R_L` (dimensionless), the Pichardo central value widened by its stated
/// fit error. `lo <= hi`, both in `(0, 1]` for a physical truncation inside the Roche lobe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TruncationFractionBand {
    /// The lower fraction edge, `f_central * (1 - fit_error_fraction)`.
    pub lo: Fixed,
    /// The upper fraction edge, `f_central * (1 + fit_error_fraction)`.
    pub hi: Fixed,
}

impl TruncationFractionBand {
    /// The central fraction, the midpoint of the symmetric band (a readout, not the consumed value).
    pub fn central(&self) -> Option<Fixed> {
        self.lo
            .checked_add(self.hi)?
            .checked_div(Fixed::from_int(2))
    }
}

/// A banded truncation RADIUS (AU), the fraction band scaled by the Roche-lobe radius.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TruncationRadiusBand {
    /// The lower radius edge (AU).
    pub lo_au: Fixed,
    /// The upper radius edge (AU).
    pub hi_au: Fixed,
}

impl TruncationRadiusBand {
    /// The central radius (AU), the midpoint of the band (a readout for a consumer that wants a single value).
    pub fn central(&self) -> Option<Fixed> {
        self.lo_au
            .checked_add(self.hi_au)?
            .checked_div(Fixed::from_int(2))
    }
}

/// The TYPED truncation evaluation: the banded fraction and radius, tagged with the modality it was drawn from and
/// the circumstellar component it describes, so a scalar Pichardo output can never be mistaken for the canonical
/// disk state. Built by [`pichardo_truncation_evaluation`]; consumed by [`tidally_capped_scale_radius_au`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TruncationEvaluation {
    /// The truncation fraction band `f = R_t / R_L`.
    pub fraction: TruncationFractionBand,
    /// The truncation radius band `R_t = f * R_L` (AU).
    pub radius_au: TruncationRadiusBand,
    /// Which model rung this radius is drawn from (Pichardo: the invariant-loop upper bound).
    pub modality: TruncationModality,
    /// Which disc this radius describes (Pichardo Eq. 6: the circumprimary disc).
    pub component: CircumstellarComponent,
}

/// Derive the [`DiskTruncationFit`] fraction BAND `f = c (1 - e)^p_e q^p_f`, widened by the fit's stated accuracy,
/// at a binary's eccentricity and companion mass fraction `q = M_2/(M_1+M_2)`. The band is the point fraction times
/// `(1 -/+ fit_error_fraction)`, so no downstream consumer reads a laundered point. The fit's OWN measured domain is
/// enforced (Pichardo `e in [0, 0.90]`, `q in [0.01, 0.99]`): outside it the function REFUSES rather than
/// extrapolating an invariant-loop fit into a regime it never covered. `None` if `e` is negative or above the fit's
/// `valid_ecc_max`, or the mass fraction is outside `[valid_mass_fraction_min, valid_mass_fraction_max]`.
pub fn resonant_truncation_fraction(
    eccentricity: Fixed,
    companion_mass_fraction: Fixed,
    fit: &DiskTruncationFit,
) -> Option<TruncationFractionBand> {
    if eccentricity < Fixed::ZERO
        || eccentricity > fit.valid_ecc_max
        || companion_mass_fraction < fit.valid_mass_fraction_min
        || companion_mass_fraction > fit.valid_mass_fraction_max
    {
        return None;
    }
    let one_minus_e = Fixed::ONE.checked_sub(eccentricity)?;
    let ecc_term = one_minus_e.powf(fit.eccentricity_exponent);
    let q_term = companion_mass_fraction.powf(fit.mass_fraction_exponent);
    let central = fit
        .circular_fraction
        .checked_mul(ecc_term)?
        .checked_mul(q_term)?;
    let lo = central.checked_mul(Fixed::ONE.checked_sub(fit.fit_error_fraction)?)?;
    let hi = central.checked_mul(Fixed::ONE.checked_add(fit.fit_error_fraction)?)?;
    Some(TruncationFractionBand { lo, hi })
}

/// Compose the Pichardo [`TruncationEvaluation`]: the banded fraction from [`resonant_truncation_fraction`] scaled
/// to a radius band by the Roche-lobe radius, tagged [`TruncationModality::InvariantLoopUpperBound`] and
/// [`CircumstellarComponent::Primary`] (Pichardo Eq. 6 is the circumprimary dissipationless upper bound). This is
/// the typed object the cap consumes, so the modality and component travel with the number rather than being
/// implied. `None` on a non-positive Roche lobe or a domain refusal from the fraction.
pub fn pichardo_truncation_evaluation(
    eccentricity: Fixed,
    companion_mass_fraction: Fixed,
    roche_lobe_au: Fixed,
    fit: &DiskTruncationFit,
) -> Option<TruncationEvaluation> {
    if roche_lobe_au <= Fixed::ZERO {
        return None;
    }
    let fraction = resonant_truncation_fraction(eccentricity, companion_mass_fraction, fit)?;
    Some(TruncationEvaluation {
        radius_au: TruncationRadiusBand {
            lo_au: roche_lobe_au.checked_mul(fraction.lo)?,
            hi_au: roche_lobe_au.checked_mul(fraction.hi)?,
        },
        fraction,
        modality: TruncationModality::InvariantLoopUpperBound,
        component: CircumstellarComponent::Primary,
    })
}

/// Cap a disk's birth scale radius `R_1` at the companion's resonant TRUNCATION radius band, returning the effective
/// `R_1` as a BAND `[min(birth, R_t_lo), min(birth, R_t_hi)]`. A disk inside its truncation radius is untouched (a
/// wide or absent companion leaves the birth radius on both edges); a disk that would spill past it is truncated,
/// and the fit's 6.5 percent band propagates to `t_visc` and `tau_disk` rather than collapsing to one conservative
/// point. The cap consumes the TYPED [`TruncationEvaluation`], not an arbitrary scalar fraction, and REFUSES a
/// non-physical fraction (a truncation radius cannot exceed the Roche lobe, so `f > 1` is rejected). `None` on a
/// non-positive birth radius or an unphysical fraction band.
pub fn tidally_capped_scale_radius_au(
    birth_r1_au: Fixed,
    evaluation: &TruncationEvaluation,
) -> Option<TruncationRadiusBand> {
    if birth_r1_au <= Fixed::ZERO
        || evaluation.fraction.lo <= Fixed::ZERO
        || evaluation.fraction.hi > Fixed::ONE
    {
        return None;
    }
    let cap = |r_t: Fixed| -> Fixed {
        if birth_r1_au < r_t {
            birth_r1_au
        } else {
            r_t
        }
    };
    Some(TruncationRadiusBand {
        lo_au: cap(evaluation.radius_au.lo_au),
        hi_au: cap(evaluation.radius_au.hi_au),
    })
}

/// Wright et al. 2011's empirical convective-turnover fit: the polynomial coefficients AND the stellar-mass range
/// over which it was measured. The range travels with the coefficients because outside it the fit is not merely
/// less accurate, the underlying PHYSICS changes: above the high-mass edge the star has a radiative envelope and
/// no rotation-activity dynamo at all (A stars are X-ray dark), so an extrapolation returns a confident and wrong
/// answer with no symptom. Reserved-with-basis measured data (Wright et al. 2011: `0.09 < M/M_sun < 1.36`), not
/// authored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ConvectiveTurnoverFit {
    /// `log10(tau)` polynomial constant term.
    pub log_tau_c0: Fixed,
    /// `log10(tau)` polynomial linear coefficient (on `log10 M`).
    pub log_tau_c1: Fixed,
    /// `log10(tau)` polynomial quadratic coefficient (on `log10^2 M`).
    pub log_tau_c2: Fixed,
    /// Fit validity lower bound (solar masses).
    pub mass_min_msun: Fixed,
    /// Fit validity upper bound (solar masses); above it the radiative-envelope regime, no dynamo.
    pub mass_max_msun: Fixed,
}

/// Why [`convective_turnover_time_days`] declined to return a turnover: ONE TYPED DOOR PER REASON, so a consumer
/// can never read three distinct refusals through one channel. A bare `None` welded three cases together (an
/// invalid input, a fit-domain refusal, and an engine representation limit), and the expansion's radiative-envelope
/// wind dispatch is precisely the consumer that would read them as one, sending a negative mass or an overflow to
/// the EUV branch. This is the value-reads-two-ways defect class (the `friction` rename, the `Delta` unit error) in
/// a return channel, so the channel is typed. Only [`TurnoverRefusal::AboveFitDomain`] is a dispatch seam.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TurnoverRefusal {
    /// The input is not a star (a non-positive mass): an error, never a branch.
    InvalidInput,
    /// The mass is BELOW the fit's low-mass edge (a sub-fit regime, its own future door, NOT the radiative branch).
    BelowFitDomain,
    /// The mass is ABOVE the fit's high-mass edge: the DISPATCH SEAM, where the convective dynamo ends and the
    /// radiative-envelope (Herbig Ae/Be) wind branch takes over (A stars are X-ray dark). The only door here.
    AboveFitDomain,
    /// An intermediate exceeded the representable range: an engine limit, never a branch.
    Unrepresentable,
}

/// The CONVECTIVE TURNOVER TIME (days) as a function of stellar mass, the denominator of the Rossby number and
/// half of the shared rotation state (the L_X slice). An empirical polynomial in `log10(M/M_sun)`:
/// `log10(tau) = c0 + c1*log10(M) + c2*log10(M)^2`, from Wright et al. 2011 (the mass fit, `0.09 < M/M_sun < 1.36`,
/// RMS 0.028 dex, alongside their V-Ks colour fit; the engine keys on mass, the drawn physical variable, rather
/// than derive a colour to look one up). The coefficients are reserved-with-basis measured data (the third floor
/// pillar), not authored. Longer for lower masses, so an M dwarf sits at a longer turnover and a lower Rossby
/// number at fixed rotation, which is why M dwarfs stay saturated for gigayears (the convicting population for a
/// mass-universal formulation).
///
/// DOMAIN GUARD (both ends), returned as a TYPED REFUSAL so "every `None` is a door" holds BY CONSTRUCTION rather
/// than by the caller's good behaviour (the gate ruling before the dispatch is built). The convective-dynamo
/// paradigm ends at the fit's high-mass edge: a star above it has a radiative envelope and no rotation-activity
/// dynamo (A STARS ARE X-RAY DARK, the convicting population this function would otherwise light up like young
/// suns), so a mass above `mass_max` returns [`TurnoverRefusal::AboveFitDomain`], the one door, where the
/// radiative-envelope wind branch takes over. A mass below `mass_min` is a separate door
/// ([`TurnoverRefusal::BelowFitDomain`], a sub-fit regime, not the radiative branch); a non-positive mass is
/// [`TurnoverRefusal::InvalidInput`] (an error, never a branch); an intermediate past the representable range is
/// [`TurnoverRefusal::Unrepresentable`] (an engine limit, never a branch). The dispatch keys on `AboveFitDomain`
/// ALONE, so an invalid mass or an overflow can never be misread as a physical seam.
pub fn convective_turnover_time_days(
    mass_ratio: Fixed,
    fit: &ConvectiveTurnoverFit,
) -> Result<Fixed, TurnoverRefusal> {
    if mass_ratio <= Fixed::ZERO {
        return Err(TurnoverRefusal::InvalidInput);
    }
    if mass_ratio < fit.mass_min_msun {
        return Err(TurnoverRefusal::BelowFitDomain);
    }
    if mass_ratio > fit.mass_max_msun {
        return Err(TurnoverRefusal::AboveFitDomain);
    }
    let ln10 = Fixed::from_int(10).ln();
    let compute = || -> Option<Fixed> {
        let log10_m = mass_ratio.ln().checked_div(ln10)?;
        let log10_tau = fit
            .log_tau_c0
            .checked_add(fit.log_tau_c1.checked_mul(log10_m)?)?
            .checked_add(fit.log_tau_c2.checked_mul(log10_m.checked_mul(log10_m)?)?)?;
        let ln_tau = log10_tau.checked_mul(ln10)?;
        let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
        if ln_tau >= ln_ceiling {
            return None;
        }
        Some(ln_tau.exp())
    };
    // A `None` here is arithmetic overflow past the format, the representation limit, never a physical door.
    compute().ok_or(TurnoverRefusal::Unrepresentable)
}

/// The star's ENVELOPE STRUCTURAL STATE for the wind-branch dispatch: does a convection zone operate a
/// rotation-coupled magnetic dynamo (the CONVECTIVE branch, the X-ray wind, where
/// [`convective_turnover_time_days`] returns a value), or is the photosphere radiative and dynamo-dark (the
/// RADIATIVE branch, the Herbig Ae/Be EUV-photoevaporation wind, where the turnover refuses)? This is the
/// STRUCTURE-KEYED line the turnover's high-mass refusal ([`TurnoverRefusal::AboveFitDomain`]) is the
/// main-sequence INSTANCE of: rather than a mass cut (the `mass_max_msun` fit edge, ~1.36 M_sun), the dispatch
/// keys on the star's own effective temperature against the KRAFT BREAK, the `T_eff` at which the surface
/// convection zone (driven by the hydrogen and helium ionization layers) vanishes. Below the break a star of any
/// mass hosts a convective dynamo, whether fully convective (a low-mass M dwarf, a cool T Tauri star on the
/// Hayashi track) or a radiative core under a convective envelope (the Sun); above it the photosphere is
/// radiative and dynamo-dark (an A or B star, a Herbig Ae/Be pre-main-sequence star).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnvelopeStructure {
    /// A convection zone operates a rotation-coupled dynamo: the X-ray wind branch, where the convective turnover
    /// time is defined. Covers both the fully convective and convective-envelope cases (both host a dynamo, the
    /// distinction between them being a phase question the dispatch does not need, flagged below).
    Convective,
    /// A radiative photosphere with no dynamo: the Herbig Ae/Be EUV-photoevaporation branch, where the turnover
    /// refuses with [`TurnoverRefusal::AboveFitDomain`] and the ionizing luminosity is the spectrum's own tail.
    Radiative,
}

/// Derive the [`EnvelopeStructure`] from the star's current effective temperature against the Kraft break. Keying
/// on `T_eff`, the drawn physical variable, generalizes where a fixed mass cut fails: an intermediate-mass star
/// early on the Hayashi track is cool and fully convective (Convective), and heats past the break onto the
/// radiative Henyey track as it evolves (Radiative), a transition its own `T_eff` tracks and a 1.36 M_sun cut,
/// which would call it Radiative at every age, cannot. The caller passes the star's CURRENT `T_eff` (the Hayashi
/// wall temperature while it is on the track, the main-sequence [`stellar_effective_temperature`] once it has
/// arrived), so the same function serves both tracks.
///
/// ADMITS THE ALIEN: the boundary is a temperature read against the star's own derived `T_eff`, so a star of any
/// composition dispatches through its own photosphere rather than a Terran mass. The honest limit: the Kraft
/// break is the hydrogen and helium ionization boundary, so a radically different photospheric composition would
/// move it, a per-star datum override if a world's chemistry ever demands one. A non-positive input is not a
/// star (`None`, an error, never a branch).
///
/// SCOPE (flagged, not conflated): this keys the dynamo/wind dispatch, one boundary (the break). The `L_bol`
/// track selection (pre-main-sequence contraction luminosity versus the main-sequence law) is a PHASE question,
/// orthogonal to envelope structure: a Herbig star is radiative AND pre-main-sequence at once. The fully
/// convective versus convective-envelope sub-distinction that would key the phase is a sibling on a second
/// boundary (the fully convective limit), left to the `L_bol` wire rather than overloaded onto this axis.
///
/// SUPERSEDED for the live dispatch by the BAND-AWARE [`kraft_band_dispatch`]: the Kraft break is not one
/// temperature but a band (the classic and modern determinations disagree by a few hundred K), so a point cut
/// asserts a certainty the measurement does not have. This point form is kept as the CERTAIN-cut classifier (a
/// star far from the band resolves the same either way) and as the main-sequence instance the structural
/// criterion demotes to once the track exposes envelope structure directly.
pub fn stellar_envelope_structure(
    t_eff_k: Fixed,
    kraft_break_k: Fixed,
) -> Option<EnvelopeStructure> {
    if t_eff_k <= Fixed::ZERO || kraft_break_k <= Fixed::ZERO {
        return None;
    }
    Some(if t_eff_k > kraft_break_k {
        EnvelopeStructure::Radiative
    } else {
        EnvelopeStructure::Convective
    })
}

/// The Kraft band's METALLICITY CONDITIONING, a SUM TYPE so a consumer can never read one meaning for another. A
/// plain zero shift conflates three distinct states (no slope claimed, a measured zero slope, a solar-reference
/// offset of zero), and a bare signed scalar would admit a NEGATIVE slope the data forbid. The break shifts UP in
/// `T_eff` with rising `[Fe/H]` (a metal-rich star sustains surface convection to higher mass, Amard and Matt
/// 2020), so the sign is fixed POSITIVE; the magnitude is under-constrained by the present data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KraftMetallicityConditioning {
    /// No conditioning: the band sits at its solar-reference placement. Distinct from a measured-zero slope, this is
    /// the state before any metallicity-dependent determination is consulted at all.
    SolarReference,
    /// The SIGN is known (positive) but no magnitude is applied, because the data fix the direction (Amard and Matt
    /// 2020's break-mass shift) without a `T_eff`-per-dex slope, and Avallone et al. 2022 find the near-break
    /// rotation-metallicity correlation below detection. A consumer may dispatch on the known direction; the edges do
    /// not move, and no value is fabricated. This is the current honest default of the ratified band.
    SignOnly,
    /// A BANDED `K`-per-dex slope, applied once a metallicity-dependent Kraft determination is fetched. Both bounds
    /// are non-negative (the fixed positive sign), and an uncertain slope WIDENS the near-degenerate band rather than
    /// asserting a point shift. Built through [`KraftMetallicityConditioning::banded_slope`].
    BandedSlope {
        /// The shallow slope edge (K per dex), non-negative.
        lo_k_per_dex: Fixed,
        /// The steep slope edge (K per dex), at least the shallow edge.
        hi_k_per_dex: Fixed,
    },
    /// The shift DERIVED from the star's own convective-envelope structure (a future structure-first route) rather
    /// than an empirical slope: a single non-negative `K`-per-dex. Built through
    /// [`KraftMetallicityConditioning::structure_derived`].
    StructureDerived {
        /// The derived slope (K per dex), non-negative.
        k_per_dex: Fixed,
    },
}

impl KraftMetallicityConditioning {
    /// A banded positive slope. `None` unless `0 <= lo <= hi` (the sign is fixed positive, and the band is ordered).
    pub fn banded_slope(lo_k_per_dex: Fixed, hi_k_per_dex: Fixed) -> Option<Self> {
        (lo_k_per_dex >= Fixed::ZERO && hi_k_per_dex >= lo_k_per_dex).then_some(Self::BandedSlope {
            lo_k_per_dex,
            hi_k_per_dex,
        })
    }

    /// A structure-derived positive slope. `None` if the slope is negative (the sign is fixed positive).
    pub fn structure_derived(k_per_dex: Fixed) -> Option<Self> {
        (k_per_dex >= Fixed::ZERO).then_some(Self::StructureDerived { k_per_dex })
    }

    /// The shift (K) applied to the band's LOWER edge at a metallicity `offset`. The no-magnitude states apply zero;
    /// a point slope shifts rigidly; a banded slope takes the shift that pushes the lower edge furthest DOWN, so an
    /// uncertain slope WIDENS the near-degenerate zone rather than asserting it narrower.
    fn lower_edge_shift(self, offset: Fixed) -> Option<Fixed> {
        match self {
            Self::SolarReference | Self::SignOnly => Some(Fixed::ZERO),
            Self::StructureDerived { k_per_dex } => k_per_dex.checked_mul(offset),
            Self::BandedSlope {
                lo_k_per_dex,
                hi_k_per_dex,
            } => {
                let a = lo_k_per_dex.checked_mul(offset)?;
                let b = hi_k_per_dex.checked_mul(offset)?;
                Some(if a < b { a } else { b })
            }
        }
    }

    /// The shift (K) applied to the band's UPPER edge at a metallicity `offset`; the banded slope takes the shift
    /// that pushes the upper edge furthest UP (the mirror of [`Self::lower_edge_shift`]).
    fn upper_edge_shift(self, offset: Fixed) -> Option<Fixed> {
        match self {
            Self::SolarReference | Self::SignOnly => Some(Fixed::ZERO),
            Self::StructureDerived { k_per_dex } => k_per_dex.checked_mul(offset),
            Self::BandedSlope {
                lo_k_per_dex,
                hi_k_per_dex,
            } => {
                let a = lo_k_per_dex.checked_mul(offset)?;
                let b = hi_k_per_dex.checked_mul(offset)?;
                Some(if a > b { a } else { b })
            }
        }
    }
}

/// THE KRAFT-BREAK BAND: the envelope-structure boundary as a BAND rather than a point, per the ratified ruling.
/// The classic primary-read Kraft break (`classic_edge_k`, the LOWER edge) and the modern determination
/// (`modern_center_k` plus or minus `modern_halfwidth_k`, whose upper reach is the band's UPPER edge) disagree by
/// a few hundred K, and that disagreement is a real dispatch ambiguity, not a value to average away: below the
/// lower edge a surface convection zone certainly operates (the dynamo branch), above the upper edge the
/// photosphere is certainly radiative (the EUV branch), and between them lies the NEAR-DEGENERATE zone the Gap
/// Law carries rather than asserts. `conditioning` moves the whole band with composition (the hydrogen and helium
/// ionization boundary depends on the metal-line opacity), a typed [`KraftMetallicityConditioning`] state rather
/// than a bare scalar, so the pre-fetch honest state (sign known, magnitude under-constrained) is distinct from a
/// measured zero and a negative slope is unrepresentable.
///
/// The edges are RESERVED-with-basis data (Principle 11): the mechanism (a three-zone dispatch on `T_eff`) is
/// fixed Rust; the edge temperatures and the Z-shift are data the caller supplies. ADMITS THE ALIEN: the band is
/// read against the star's own derived `T_eff` and its own metallicity offset, so a star of any composition
/// dispatches through its own photosphere.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KraftBreakBand {
    /// The classic Kraft-break `T_eff` (K), the band's LOWER edge: below it the convective dynamo branch is
    /// certain. RESERVED-with-basis (the primary-read classic determination, ~6200 K).
    pub classic_edge_k: Fixed,
    /// The modern determination's CENTER `T_eff` (K). RESERVED-with-basis (the search-grade modern edge, ~6550 K).
    pub modern_center_k: Fixed,
    /// The modern determination's half-width (K): `modern_center_k + modern_halfwidth_k` is the band's UPPER edge,
    /// above which the radiative EUV branch is certain. RESERVED-with-basis (~200 K).
    pub modern_halfwidth_k: Fixed,
    /// The band's METALLICITY CONDITIONING, a typed [`KraftMetallicityConditioning`] rather than a bare K-per-dex
    /// scalar. The literature fixes the SIGN but not the magnitude: the break shifts UP in `T_eff` with rising
    /// [Fe/H], since a metal-rich star bears a deeper surface convection zone and sustains it to higher mass (Amard
    /// and Matt 2020, the break mass falling from ~1.3 to ~1.0 M_sun from [Fe/H] 0.0 to -1.0, ~+0.3 M_sun per dex;
    /// vendored). No source gives a `T_eff` break versus [Fe/H] slope in K per dex, and Avallone et al. 2022 find
    /// the near-break rotation-metallicity correlation below detection in a hot main-sequence sample (an empirical
    /// upper bound; vendored). So the honest pre-fetch state is [`KraftMetallicityConditioning::SignOnly`] (the sign
    /// carried, no magnitude applied), distinct from a measured zero and unable to hold a negative slope.
    pub conditioning: KraftMetallicityConditioning,
}

impl KraftBreakBand {
    /// The band's effective LOWER edge (K) at a metallicity `metallicity_log10_offset` dex from the sampled
    /// composition: `classic_edge_k` plus the conditioning's lower-edge shift. `None` if the shift or an overflow
    /// drives the edge non-positive (a non-physical band).
    pub fn lower_edge_k(self, metallicity_log10_offset: Fixed) -> Option<Fixed> {
        let shifted = self.classic_edge_k.checked_add(
            self.conditioning
                .lower_edge_shift(metallicity_log10_offset)?,
        )?;
        (shifted > Fixed::ZERO).then_some(shifted)
    }
    /// The band's effective UPPER edge (K): `modern_center_k + modern_halfwidth_k` plus the conditioning's
    /// upper-edge shift. `None` on overflow or a non-positive result.
    pub fn upper_edge_k(self, metallicity_log10_offset: Fixed) -> Option<Fixed> {
        let base = self.modern_center_k.checked_add(self.modern_halfwidth_k)?;
        let shifted = base.checked_add(
            self.conditioning
                .upper_edge_shift(metallicity_log10_offset)?,
        )?;
        (shifted > Fixed::ZERO).then_some(shifted)
    }
}

/// The KRAFT-BAND VERDICT: which wind branch a star's photosphere takes, with the near-degenerate band CARRIED
/// rather than asserted (the Gap Law). A consumer that needs a single clock and reads [`Self::NearDegenerate`]
/// must evaluate BOTH branches and carry the pair as a bracket (the way the EUV branch ships a bracket), never
/// silently pick a side, so a few hundred K of dispatch ambiguity cannot masquerade as a definite branch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KraftVerdict {
    /// Below the band's lower edge: the X-ray dynamo branch, certain.
    Convective,
    /// Above the band's upper edge: the EUV branch, certain.
    Radiative,
    /// Inside the Kraft band: near-degenerate, both branches plausible (the Gap Law). The consumer carries both.
    NearDegenerate,
}

/// Dispatch a star's envelope structure against the KRAFT-BREAK BAND, the band-aware successor to the point
/// [`stellar_envelope_structure`]. The star's current `T_eff` against the band's effective edges (each shifted
/// for the star's metallicity offset): [`KraftVerdict::Convective`] below the lower edge,
/// [`KraftVerdict::Radiative`] above the upper edge, [`KraftVerdict::NearDegenerate`] inside (the boundaries
/// themselves belong to the ambiguous band, so the certain branches are the strict outside). Per the ratified
/// ruling the in-band case is a distinct verdict the consumer carries, never asserted to one side. `None` on a
/// non-star (non-positive `T_eff`) or an invalid band (an edge non-positive, or the lower edge above the upper).
pub fn kraft_band_dispatch(
    t_eff_k: Fixed,
    band: KraftBreakBand,
    metallicity_log10_offset: Fixed,
) -> Option<KraftVerdict> {
    if t_eff_k <= Fixed::ZERO {
        return None;
    }
    let lower = band.lower_edge_k(metallicity_log10_offset)?;
    let upper = band.upper_edge_k(metallicity_log10_offset)?;
    if lower > upper {
        return None;
    }
    Some(if t_eff_k < lower {
        KraftVerdict::Convective
    } else if t_eff_k > upper {
        KraftVerdict::Radiative
    } else {
        KraftVerdict::NearDegenerate
    })
}

/// The STAR'S EVOLUTIONARY PHASE, the axis ORTHOGONAL to envelope structure that decides which luminosity law a
/// star obeys. A star is on the PRE-MAIN-SEQUENCE while it is still contracting and shining on the released
/// gravitational energy (the Hayashi-Henyey contraction, brighter than its zero-age main-sequence instance), and
/// on the MAIN SEQUENCE once hydrogen ignition has halted the contraction. The distinction is a PHASE question, not
/// a structural one: a Herbig Ae/Be star is radiative-envelope AND pre-main-sequence at once, and a solar analogue
/// is convective-envelope on BOTH sides of its arrival, so this never substitutes for [`KraftVerdict`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EvolutionaryPhase {
    /// Still contracting: the star shines above its main-sequence luminosity on gravitational energy, so the
    /// `L_bol` track is the pre-main-sequence contraction law ([`pre_main_sequence_luminosity_lsun`]) and the
    /// convective turnover is the fully-convective pre-main-sequence value
    /// ([`pre_main_sequence_convective_turnover_days`]).
    PreMainSequence,
    /// Arrived: contraction has fallen to the zero-age main-sequence luminosity, so the `L_bol` track is the
    /// main-sequence mass-luminosity law and the turnover is the main-sequence polynomial
    /// ([`convective_turnover_time_days`]).
    MainSequence,
}

/// Derive the [`EvolutionaryPhase`] from the CROSSING of the two luminosity laws the star obeys in turn: the falling
/// pre-main-sequence contraction luminosity (`L ~ t^(-2/3)` on the Hayashi track) and the zero-age main-sequence
/// luminosity (the mass-luminosity law, `L_sun * mass_ratio^exponent`). A pre-main-sequence star begins
/// super-luminous and dims as it contracts, crossing the main-sequence value FROM ABOVE, and the crossing IS the
/// arrival: while `L_pms > L_MS` the star is still contracting (PreMainSequence), and once the contraction
/// luminosity has fallen to or below the main-sequence value it has reached the zero-age main sequence
/// (MainSequence). Defining the boundary as the crossing makes `L_bol` CONTINUOUS across the phase switch (the two
/// laws are equal there by construction), so a consumer selecting the phase-appropriate luminosity never sees a
/// discontinuity at arrival, the same-fact-two-doors hazard defused at its root.
///
/// DERIVED, no authored value: both luminosities are the star's own derived quantities (each carries the star's
/// mass, its Hayashi wall temperature, and, through the mass-luminosity exponent, its opacity), so the phase is a
/// comparison of two derived numbers, not a threshold on age. ADMITS THE ALIEN: a star of any composition or energy
/// route dispatches on ITS OWN two luminosities, never a Terran arrival age. The HONEST LIMIT: the Hayashi
/// `t^(-2/3)` law overstates the late-contraction brightness (a real track flattens before the crossing) and a
/// massive star's late pre-main-sequence is the Henyey track rather than the Hayashi wall, so the crossing slightly
/// OVERESTIMATES the arrival age for those stars, a bias to correct when a per-track pre-main-sequence luminosity
/// lands. `None` if either luminosity is non-positive (not a star).
pub fn evolutionary_phase(
    pre_main_sequence_luminosity_lsun: Fixed,
    main_sequence_luminosity_lsun: Fixed,
) -> Option<EvolutionaryPhase> {
    if pre_main_sequence_luminosity_lsun <= Fixed::ZERO
        || main_sequence_luminosity_lsun <= Fixed::ZERO
    {
        return None;
    }
    Some(
        if pre_main_sequence_luminosity_lsun > main_sequence_luminosity_lsun {
            EvolutionaryPhase::PreMainSequence
        } else {
            EvolutionaryPhase::MainSequence
        },
    )
}

/// THE STRUCTURE-KEYED DISPATCH STATE: the star's envelope structure and evolutionary phase as ONE derived state,
/// the single node every branch downstream keys on. The two axes are orthogonal and both load-bearing: the
/// [`KraftVerdict`] envelope (from [`kraft_band_dispatch`] on the star's current `T_eff`) selects the WIND branch (a
/// convective envelope runs the X-ray dynamo clock, a radiative one the EUV-photoevaporation branch, a
/// near-degenerate one carries both per the Gap Law), and the [`EvolutionaryPhase`] (from [`evolutionary_phase`] on
/// the luminosity crossing) selects the `L_bol` TRACK (pre-main-sequence contraction versus the main-sequence law)
/// and, with it, which convective turnover the Rossby number reads. Holding both in one state is what lets a
/// consumer route a star correctly without re-deriving either: a Herbig star reads Radiative and PreMainSequence, a
/// young solar analogue Convective and PreMainSequence, an arrived Sun Convective and MainSequence.
///
/// This supersedes a mass cut (the demoted `1.4 M_sun` figure was the main-sequence instance of a structure-keyed
/// line): the dispatch keys on the star's own derived `T_eff` and its own two luminosities, so it is fully
/// convective on the pre-main-sequence and mass-dependent on the main sequence WITHOUT reading a mass threshold.
/// DERIVED throughout, no authored value; the mechanism is fixed Rust and the Kraft band edges are the only data,
/// carried by [`KraftBreakBand`]. ADMITS THE ALIEN: every input is the star's own derived quantity.
///
/// HONEST LIMIT (a flagged sibling, not built here): the Kraft band is a MAIN-SEQUENCE-instance calibration, and a
/// pre-main-sequence star keeps a convective envelope to a higher `T_eff` than its arrived instance, so the true
/// envelope boundary shifts with phase. That shift is a future conditioning field on [`KraftBreakBand`] (the
/// sibling of its metallicity shift), reserved until a pre-main-sequence Kraft determination is fetched; until then
/// the envelope dispatches on the phase-correct `T_eff` against the main-sequence band, the conservative reading.
/// `None` if either sub-dispatch refuses (a non-star `T_eff`, an invalid band, or a non-positive luminosity).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StellarStructuralState {
    /// The envelope-structure verdict against the Kraft band: which wind branch the star's photosphere takes.
    pub envelope: KraftVerdict,
    /// The evolutionary phase: which luminosity track and convective turnover the star obeys.
    pub phase: EvolutionaryPhase,
}

/// Derive the [`StellarStructuralState`] by composing the two sub-dispatches: the envelope from
/// [`kraft_band_dispatch`] (the star's current `T_eff` against the metallicity-shifted Kraft band) and the phase
/// from [`evolutionary_phase`] (the pre-main-sequence contraction luminosity against the main-sequence luminosity,
/// both in `L_sun`). Returns `None` if either refuses, so an ill-posed star never yields a half-formed state.
pub fn stellar_structural_state(
    t_eff_k: Fixed,
    band: KraftBreakBand,
    metallicity_log10_offset: Fixed,
    pre_main_sequence_luminosity_lsun: Fixed,
    main_sequence_luminosity_lsun: Fixed,
) -> Option<StellarStructuralState> {
    let envelope = kraft_band_dispatch(t_eff_k, band, metallicity_log10_offset)?;
    let phase = evolutionary_phase(
        pre_main_sequence_luminosity_lsun,
        main_sequence_luminosity_lsun,
    )?;
    Some(StellarStructuralState { envelope, phase })
}

/// A LOG10 BAND `[lo, hi]` (both already `log10` of the underlying quantity), the RIDER 2 output form for a value
/// whose model uncertainty spans orders of magnitude: carrying the range in the log domain keeps a quantity that
/// sits outside the fixed-point window (an ionizing photon rate of order `1e45` per second, an ionizing luminosity
/// of order `1e33` erg/s) representable, and makes the width a subtraction. A point value is `lo == hi`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Log10Band {
    /// The lower bound (`log10` of the quantity).
    pub lo: Fixed,
    /// The upper bound (`log10` of the quantity).
    pub hi: Fixed,
}

impl Log10Band {
    /// The band WIDTH in dex (`hi - lo`), readable before any consumer reads the bounds (RIDER 2). `None` only if
    /// the subtraction leaves the representable range.
    pub fn width_dex(self) -> Option<Fixed> {
        self.hi.checked_sub(self.lo)
    }
}

/// Which spectral model an ionizing evaluation came from, carried so a consumer never mistakes the LTE estimator
/// for a real atmosphere.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AtmosphereBranch {
    /// The LTE blackbody Wien-tail integral: a self-consistent ESTIMATOR and upper bound (a real hot atmosphere's
    /// line blanketing and ionization edges suppress the ionizing flux below it).
    Blackbody,
    /// An NLTE line-blanketed atmosphere grid (Sternberg, Hoffmann and Pauldrach 2003): the photon-number departure
    /// applied to the blackbody photon rate IN PHOTON SPACE, never an energy departure divided by a mean energy.
    NlteLineBlanketed,
}

/// ONE IONIZING-SPECTRUM EVALUATION, the same-spectrum-correct object the EUV wind consumes: the hydrogen-ionizing
/// photon rate `Q_H` (photons/s, `log10`) with the ionizing luminosity `L_ion` and the mean ionizing photon energy
/// `<E>` from the SAME spectral branch when that branch supplies them, so `L_ion / Q_H = <E>` holds within one
/// spectrum. This is the fix for the same-spectrum violation: the wind reads `Q_H` directly, never an NLTE-adjusted
/// energy bracket divided by an LTE blackbody mean energy (three correlated errors, since line blanketing changes
/// spectral shape so the departures in integrated energy, photon number, and mean energy are not the same number).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IonizingSpectrumEvaluation {
    /// `log10(Q_H)` in photons per second, the quantity the EUV wind law consumes.
    pub photon_rate_log10_s: Log10Band,
    /// `log10(L_ion)` in erg/s, present only when the branch supplies it self-consistently (the blackbody branch).
    pub ionizing_luminosity_log10_erg_s: Option<Log10Band>,
    /// `log10(<E>)` in erg, the mean ionizing photon energy, present only from the same branch as `L_ion`.
    pub mean_photon_energy_log10_erg: Option<Log10Band>,
    /// Which spectral model produced this evaluation.
    pub branch: AtmosphereBranch,
    /// The effective temperature the evaluation was formed at (the minimal atmosphere-state anchor).
    pub t_eff_k: Fixed,
}

/// The BLACKBODY IONIZING FRACTION `f_BB(T_eff)`: the fraction of a blackbody's radiant exitance emitted above the
/// hydrogen ionization edge (13.6 eV), the Wien-tail upper-incomplete integral `(15/pi^4) exp(-x)(x^3 + 3x^2 +
/// 6x + 6)` with `x = T_ion/T_eff` and `T_ion = E_edge/k_B`. Computed in LOG space (`C * exp(ln(poly) - x)`, one
/// exp at the end) so the tiny tail stays representable where a bare `exp(-x)` would underflow fixed point. This
/// is the LTE-blackbody BASELINE that the real hot-photosphere EUV departs from by orders of magnitude; the
/// departure is the atmosphere-model band the caller brackets, not this baseline. DERIVED from `T_eff`, no
/// authored value; `T_ion` is a derived physical constant (the hydrogen edge over Boltzmann, ~157821 K). It rises
/// steeply with `T_eff` (roughly eight dex from a solar photosphere to a 30000 K one), which is why a hotter
/// radiative star photoevaporates harder.
///
/// VALIDITY (GUARDED, not just documented): the Wien tail (dropping the `-1` in the Planck denominator) is exact
/// to a fraction of a percent for `x >~ 3`, which holds at the hydrogen edge for every star cooler than ~50000 K.
/// A hotter photosphere (`x < wien_x_min`) needs the full Planck denominator, so this REFUSES (`None`, the
/// domain door) rather than extrapolating the approximation into a regime it does not describe: the code now
/// enforces the flag its own doc claims, the second edge of a domain that was one-ended. `wien_x_min` (~3) is the
/// caller's reserved-with-basis value (the Wien-tail validity edge). A non-positive input also returns `None`.
pub fn blackbody_ionizing_fraction(
    t_eff_k: Fixed,
    t_ion_k: Fixed,
    wien_x_min: Fixed,
) -> Option<Fixed> {
    if t_eff_k <= Fixed::ZERO || t_ion_k <= Fixed::ZERO || wien_x_min <= Fixed::ZERO {
        return None;
    }
    let x = t_ion_k.checked_div(t_eff_k)?;
    if x < wien_x_min {
        return None; // above the Wien-tail validity T_eff: the full-Planck-denominator regime, a separate door
    }
    let c = Fixed::from_int(15).checked_div(Fixed::PI.powi(4))?; // 15/pi^4, the Planck-integral normalization
                                                                 // CHECKED powers (not `powi`, which multiplies through the unchecked `Fixed::mul` and wraps silently): a
                                                                 // large `x` (a photosphere below ~122 K, non-stellar) must REFUSE with `None`, the total-kernel contract, not
                                                                 // wrap `x^3` to garbage. This is the one unchecked-arithmetic hole the audit caught in an otherwise checked
                                                                 // function.
    let x2 = x.checked_mul(x)?;
    let x3 = x2.checked_mul(x)?;
    let poly = x3
        .checked_add(Fixed::from_int(3).checked_mul(x2)?)?
        .checked_add(Fixed::from_int(6).checked_mul(x)?)?
        .checked_add(Fixed::from_int(6))?;
    // f_BB = C * poly * exp(-x), formed as C * exp(ln(poly) - x) so the tiny tail never underflows.
    let tail = poly.ln().checked_sub(x)?.exp();
    c.checked_mul(tail)
}

/// The MEAN IONIZING PHOTON ENERGY as a multiple of the hydrogen edge energy, DERIVED from the star's `T_eff`: the
/// energy-weighted mean energy of a photon above the 13.6 eV ionization edge, `<E> / E_edge`. This is what converts
/// the ionizing LUMINOSITY (energy per second) into the ionizing photon RATE `Q_H` (photons per second) inside the
/// same-branch [`blackbody_ionizing_spectrum`], and it is DERIVED rather than reserved: it falls out of
/// the same Wien-tail integral as [`blackbody_ionizing_fraction`]. The mean energy above the edge is the energy flux
/// over the photon-number flux, `<E> = kT * Gamma(4,x) / Gamma(3,x)` with `x = T_ion/T_eff`, which reduces (using
/// `kT = E_edge/x`) to `<E>/E_edge = (x^3 + 3x^2 + 6x + 6) / (x (x^2 + 2x + 2))`. The numerator is the SAME
/// `Gamma(4,x)` polynomial the ionizing fraction carries (the energy integral); the denominator adds the
/// `Gamma(3,x) = x^2 + 2x + 2` number integral. The ratio is 1 at the cold limit (every ionizing photon sits just
/// above the edge) and rises as the photosphere heats and the tail hardens (about 1.11 at `x = 10`, 1.28 at `x = 5`).
///
/// DERIVED, NO reserved value: it keys ONLY on the star's own `T_eff` against the derived edge, so a hot star of any
/// composition converts its own luminosity to its own photon rate (ADMITS THE ALIEN). HONEST LIMIT: this is the LTE
/// BLACKBODY mean, the same baseline the EUV luminosity bracket departs from; a real atmosphere-model spectrum
/// hardens the tail further, and that departure is carried by the luminosity bracket's band rather than a second
/// number here. `None` above the Wien-tail validity edge (`x < wien_x_min`, the full-Planck regime) or on a non-star
/// input, the same domain door the ionizing fraction guards.
pub fn mean_ionizing_photon_energy_over_edge(
    t_eff_k: Fixed,
    t_ion_k: Fixed,
    wien_x_min: Fixed,
) -> Option<Fixed> {
    if t_eff_k <= Fixed::ZERO || t_ion_k <= Fixed::ZERO || wien_x_min <= Fixed::ZERO {
        return None;
    }
    let x = t_ion_k.checked_div(t_eff_k)?;
    if x < wien_x_min {
        return None; // above the Wien-tail validity T_eff, the full-Planck regime, the shared domain door
    }
    let x2 = x.checked_mul(x)?;
    let x3 = x2.checked_mul(x)?;
    // Gamma(4,x) = x^3 + 3x^2 + 6x + 6, the energy integral (the ionizing-fraction numerator).
    let gamma4 = x3
        .checked_add(Fixed::from_int(3).checked_mul(x2)?)?
        .checked_add(Fixed::from_int(6).checked_mul(x)?)?
        .checked_add(Fixed::from_int(6))?;
    // Gamma(3,x) = x^2 + 2x + 2, the photon-number integral.
    let gamma3 = x2
        .checked_add(Fixed::from_int(2).checked_mul(x)?)?
        .checked_add(Fixed::from_int(2))?;
    // <E>/E_edge = Gamma(4,x) / (x * Gamma(3,x)).
    gamma4.checked_div(x.checked_mul(gamma3)?)
}

/// The BLACKBODY IONIZING SPECTRUM: `L_ion`, `Q_H`, and `<E>` for a radiative-envelope star's photosphere, all
/// three from the SAME LTE Wien-tail integral so they are self-consistent (`L_ion / Q_H = <E>` by construction).
/// The ionizing luminosity is `L_ion = L_bol * f_BB(T_eff)` ([`blackbody_ionizing_fraction`]); the mean ionizing
/// photon energy is `<E> = (<E>/E_edge) * k_B * T_ion` ([`mean_ionizing_photon_energy_over_edge`] times the edge
/// energy, both floor); the photon rate is the ONE division `Q_H = L_ion / <E>`, done here in the log domain (the
/// erg-scale photon energy of order `3e-11` sits below fixed-point resolution and `Q_H ~ 1e45` above its range, so
/// everything is `log10`). This is the LTE ESTIMATOR and UPPER BOUND (branch [`AtmosphereBranch::Blackbody`]); a
/// real atmosphere departs below it, and that departure is applied in PHOTON space by
/// [`nlte_departed_ionizing_spectrum`], never as an energy multiplier divided by this mean energy. `None` on a
/// non-positive `L_bol`, a `T_eff` past the Wien-tail edge (the shared domain door), or an intermediate overflow.
pub fn blackbody_ionizing_spectrum(
    t_eff_k: Fixed,
    l_bol_lsun: Fixed,
    t_ion_k: Fixed,
    wien_x_min: Fixed,
) -> Option<IonizingSpectrumEvaluation> {
    if l_bol_lsun <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10 = |x: Fixed| -> Option<Fixed> { x.ln().checked_div(ln10) };
    let f_bb = blackbody_ionizing_fraction(t_eff_k, t_ion_k, wien_x_min)?;
    // log10(L_ion in erg/s) = log10(L_bol/L_sun) + log10(f_BB) + log10(L_sun in erg/s).
    let log10_lsun_erg_s = civsim_physics::saha::ln_of_decimal(SOLAR_LUMINOSITY_W)?
        .checked_div(ln10)?
        .checked_add(Fixed::from_int(7))?; // W to erg/s
    let log10_l_ion = log10(l_bol_lsun)?
        .checked_add(log10(f_bb)?)?
        .checked_add(log10_lsun_erg_s)?;
    // log10(<E> in erg) = log10(<E>/E_edge) + log10(k_B in erg/K) + log10(T_ion).
    let photon_over_edge = mean_ionizing_photon_energy_over_edge(t_eff_k, t_ion_k, wien_x_min)?;
    let log10_kb_erg = civsim_physics::saha::ln_of_decimal("1.380649e-16")?.checked_div(ln10)?;
    let log10_mean_e = log10(photon_over_edge)?
        .checked_add(log10_kb_erg)?
        .checked_add(log10(t_ion_k)?)?;
    // The ONE division, self-consistent: log10(Q_H) = log10(L_ion) - log10(<E>).
    let log10_q_h = log10_l_ion.checked_sub(log10_mean_e)?;
    Some(IonizingSpectrumEvaluation {
        photon_rate_log10_s: Log10Band {
            lo: log10_q_h,
            hi: log10_q_h,
        },
        ionizing_luminosity_log10_erg_s: Some(Log10Band {
            lo: log10_l_ion,
            hi: log10_l_ion,
        }),
        mean_photon_energy_log10_erg: Some(Log10Band {
            lo: log10_mean_e,
            hi: log10_mean_e,
        }),
        branch: AtmosphereBranch::Blackbody,
        t_eff_k,
    })
}

/// The NLTE-DEPARTED IONIZING SPECTRUM: the real hot atmosphere's photon rate, formed by applying a PHOTON-NUMBER
/// departure band `[departure_lo, departure_hi]` to the blackbody `Q_H` IN PHOTON SPACE, `log10(Q_H) =
/// log10(Q_H,BB) + log10(departure)`. This is the same-spectrum-correct placement of the atmosphere-model band: the
/// Sternberg, Hoffmann and Pauldrach 2003 grid tabulates `q_H` as a photon flux, so its departure below the
/// same-`T_eff` blackbody is a photon-number suppression (within about 0.1 to 0.2 dex above 45000 K, of order 1 dex
/// at the 26000 to 30000 K edge; deeper and UNCONSTRAINED below 25000 K, the Herbig regime). The cooler grid is now
/// HELD as a witness (the BSTAR2006 NLTE B-star atmospheres, Lanz and Hubeny 2007, `bstar2006_lanz_hubeny` in
/// `sources/registry.toml`, Teff 15000 to 30000 K, `model.flux` from soft X-ray to far-IR so the Lyman continuum is
/// computed), but reading its paper found that its Fig. 6 EUV statement is NLTE-versus-LTE(Kurucz), NOT the
/// model-versus-blackbody departure this branch consumes, so it does not yet supply the Herbig number: that requires
/// integrating the grid's `model.flux` SEDs against the blackbody Wien tail, a data fetch named as the deeper rung
/// (the paper's factor is a different comparison and must not be conflated with the departure).
/// The departure is NOT applied to an energy and then divided by a mean energy: that would cross an NLTE energy with
/// an LTE mean. `L_ion` and `<E>` are left absent because this branch does not reconstruct the NLTE energy integral,
/// so no self-consistent energy pair is claimed. `None` if the input is not a blackbody evaluation, on a
/// non-positive or inverted departure, or an overflow.
pub fn nlte_departed_ionizing_spectrum(
    blackbody: &IonizingSpectrumEvaluation,
    departure_lo: Fixed,
    departure_hi: Fixed,
) -> Option<IonizingSpectrumEvaluation> {
    if blackbody.branch != AtmosphereBranch::Blackbody
        || departure_lo <= Fixed::ZERO
        || departure_hi < departure_lo
    {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10 = |x: Fixed| -> Option<Fixed> { x.ln().checked_div(ln10) };
    // The departure suppresses in PHOTON space: the lower departure gives the lower photon rate.
    let lo = blackbody
        .photon_rate_log10_s
        .lo
        .checked_add(log10(departure_lo)?)?;
    let hi = blackbody
        .photon_rate_log10_s
        .hi
        .checked_add(log10(departure_hi)?)?;
    Some(IonizingSpectrumEvaluation {
        photon_rate_log10_s: Log10Band { lo, hi },
        ionizing_luminosity_log10_erg_s: None,
        mean_photon_energy_log10_erg: None,
        branch: AtmosphereBranch::NlteLineBlanketed,
        t_eff_k: blackbody.t_eff_k,
    })
}

/// The PRE-MAIN-SEQUENCE LUMINOSITY `L_bol / L_sun` at a stellar age, the disk-era bolometric luminosity the L_X
/// chain reads (and the race's wind rate runs through). A disk-hosting star is not a main-sequence object: it is
/// a pre-main-sequence star still descending the Hayashi track, fully convective and BRIGHTER than its
/// main-sequence instance, contracting under gravity. It sits at the H-minus opacity wall's own effective
/// temperature (the [`crate::stellar_evolution::hayashi_effective_temperature`] band, direction-agnostic: the same
/// wall serves the pre-main-sequence descent and the post-main-sequence giant ascent), so `L = 4 pi sigma T_H^4 R^2`
/// with `T_H` fixed and `R` shrinking.
///
/// FULLY DERIVED, ZERO NEW VALUES. The star is an `n = 3/2` polytrope (a fully convective, adiabatic monatomic
/// envelope, `gamma = 5/3` giving `n = 1/(gamma - 1) = 3/2`), whose total energy `E = -(3/(2(5-n))) G M^2 / R =
/// -(3/7) G M^2 / R` carries the structure coefficient `3/7`, DERIVED from the polytrope index, not fetched.
/// Kelvin-Helmholtz balance `L = -dE/dt` with `L = 4 pi sigma T_H^4 R^2` gives `dR/dt = -(28 pi sigma T_H^4 /
/// (3 G M^2)) R^4`, whose solution once the birth radius is forgotten is `R^3 = G M^2 / (28 pi sigma T_H^4 t)`, so
/// `R ~ t^(-1/3)` and `L ~ t^(-2/3)` in closed form. The luminosity reads only the stellar mass, the Hayashi wall
/// temperature (the existing banded anchor), the age, and the floor constants `G`, `sigma` (the derived
/// Stefan-Boltzmann), and `M_sun`, `L_sun`; the `n = 3/2` index and its `3/7` coefficient are bare-algebra physics
/// results. Computed in the log domain (`L ~ 1e26 W` overflows the format; `L / L_sun` is order one).
///
/// This is the CONSUMER the race's wind rate needed: `L_X = plateau * L_bol` in the saturated disk era, so the
/// `t^(-2/3)` decline of `L_bol` (a factor ~5 across a 1-to-10 Myr window) is the wind's own time dependence, the
/// term the race's constant-wind statement had dropped (see [`derive_disk_lifetime_myr`]).
///
/// DOMAIN: valid while the star is FULLY CONVECTIVE on the Hayashi track and past its initial contraction (the
/// birth radius forgotten, which for the disk era holds since the Kelvin-Helmholtz time is well under a megayear).
/// The boundary is where the star leaves full convection for the radiative Henyey leg; for the disk era at FGKM
/// masses that boundary stays comfortably distant, and for the A-class masses it hands off to the radiative-envelope
/// wind branch (a named future dispatch, its own gate). Past the representable range (an unphysically small age
/// where the forgotten-birth-radius asymptote diverges) it returns `None`. `None` also on a non-positive input.
pub fn pre_main_sequence_luminosity_lsun(
    mass_ratio: Fixed,
    hayashi_temp_k: Fixed,
    age_myr: Fixed,
) -> Option<Fixed> {
    if mass_ratio <= Fixed::ZERO || hayashi_temp_k <= Fixed::ZERO || age_myr <= Fixed::ZERO {
        return None;
    }
    let ln_pi = Fixed::PI.ln();
    let ln_sigma = crate::physiology::derived_stefan_boltzmann().ln();
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    let ln_m = mass_ratio
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
    let ln_t_h = hayashi_temp_k.ln();
    // Age in seconds: age(Myr) * 1e6 * Julian year.
    let ln_t = age_myr
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal("1e6")?)?
        .checked_add(civsim_physics::saha::ln_of_decimal(JULIAN_YEAR_S)?)?;
    // ln R^3 = ln(G M^2 / (28 pi sigma T_H^4 t)) = ln G + 2 ln M - ln 28 - ln pi - ln sigma - 4 ln T_H - ln t.
    let four_ln_t_h = Fixed::from_int(4).checked_mul(ln_t_h)?;
    let ln_r3 = ln_g
        .checked_add(Fixed::from_int(2).checked_mul(ln_m)?)?
        .checked_sub(Fixed::from_int(28).ln())?
        .checked_sub(ln_pi)?
        .checked_sub(ln_sigma)?
        .checked_sub(four_ln_t_h)?
        .checked_sub(ln_t)?;
    let ln_r = ln_r3.checked_div(Fixed::from_int(3))?;
    // ln L = ln(4 pi sigma T_H^4 R^2) = ln 4 + ln pi + ln sigma + 4 ln T_H + 2 ln R.
    let ln_l = Fixed::from_int(4)
        .ln()
        .checked_add(ln_pi)?
        .checked_add(ln_sigma)?
        .checked_add(four_ln_t_h)?
        .checked_add(Fixed::from_int(2).checked_mul(ln_r)?)?;
    let ln_l_over_lsun =
        ln_l.checked_sub(civsim_physics::saha::ln_of_decimal(SOLAR_LUMINOSITY_W)?)?;
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_l_over_lsun >= ln_ceiling {
        return None;
    }
    Some(ln_l_over_lsun.exp())
}

/// The PRE-MAIN-SEQUENCE CONVECTIVE TURNOVER TIME (days), the Rossby denominator a DISK-ERA star truly needs,
/// which the main-sequence Wright polynomial ([`convective_turnover_time_days`]) gets wrong. This is the RIDER-1
/// finding's fix and the second founding case of the MAIN-SEQUENCE-INSTANCE SWEEP: the arc's stars are
/// pre-main-sequence, but the chain was built on main-sequence instances, and here the main-sequence polynomial
/// (calibrated on shallow outer convection zones) UNDERESTIMATES the turnover of a FULLY convective
/// pre-main-sequence star by roughly a decade, which flips the computed Rossby number from saturated (true) to
/// unsaturated (wrong) for `M >~ 0.5 M_sun` at disk-locked rotation. Left unfixed it makes `L_X` read on the decay
/// branch instead of the plateau, so the wind rate and `tau_disk` are wrong, so the #73 giant verdict races a
/// wrong clock.
///
/// DERIVED on the SAME Hayashi substrate the contraction luminosity stands on. Global mixing-length theory gives a
/// fully-convective turnover `tau ~ C (R^2 M / L)^(1/3)` (the convective velocity `v ~ (L/(4 pi R^2 rho))^(1/3)`
/// carrying the luminosity, over a mixing length `~ R`). On the Hayashi track `L = 4 pi sigma T_H^4 R^2`, so the
/// radius CANCELS and `tau ~ C (M / (4 pi sigma T_H^4))^(1/3)`, a function of stellar mass and the H-minus wall
/// temperature alone. The cancellation is what makes this SPECIFIC to Hayashi-track stars: it does not misfire on a
/// main-sequence star (whose `L` is not set by `T_H`), so the two turnovers stay distinct. The mass dependence is
/// now correct (INCREASING weakly with mass, since a pre-main-sequence star is fully convective at every mass),
/// the opposite of the main-sequence polynomial's decrease as the envelope thins, which is exactly why the
/// polynomial failed at the high-mass end.
///
/// `mlt_coefficient` (`C`) is reserved-with-basis: the mixing-length `alpha` (solar-calibrated `~1.5 to 2.0`) times
/// the order-unity global-mixing-length numerical factors, anchorable to a pre-main-sequence model turnover (a
/// solar-mass pre-main-sequence `tau_conv ~ 250 to 400` days, Landin et al. 2010 / Gregory et al. 2016 class). Its
/// precision does not decide the arc's answer: at disk-locked rotation the derived turnover clears the saturation
/// knee by a factor of several (the saturation assertion test), so the conclusion survives the coefficient's
/// uncertainty, which is the blindness restored on the CORRECT substrate. `None` on a non-positive input or an
/// intermediate past the representable range.
pub fn pre_main_sequence_convective_turnover_days(
    mass_ratio: Fixed,
    hayashi_temp_k: Fixed,
    mlt_coefficient: Fixed,
) -> Option<Fixed> {
    if mass_ratio <= Fixed::ZERO || hayashi_temp_k <= Fixed::ZERO || mlt_coefficient <= Fixed::ZERO
    {
        return None;
    }
    let ln_pi = Fixed::PI.ln();
    let ln_sigma = crate::physiology::derived_stefan_boltzmann().ln();
    let ln_m = mass_ratio
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
    // ln(tau in s) = ln C + (1/3)(ln M - ln 4 - ln pi - ln sigma - 4 ln T_H).
    let inner = ln_m
        .checked_sub(Fixed::from_int(4).ln())?
        .checked_sub(ln_pi)?
        .checked_sub(ln_sigma)?
        .checked_sub(Fixed::from_int(4).checked_mul(hayashi_temp_k.ln())?)?;
    let ln_tau_s = mlt_coefficient
        .ln()
        .checked_add(inner.checked_div(Fixed::from_int(3))?)?;
    // Seconds to days: subtract ln(86400).
    let ln_tau_days = ln_tau_s.checked_sub(Fixed::from_int(86_400).ln())?;
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_tau_days >= ln_ceiling {
        return None;
    }
    Some(ln_tau_days.exp())
}

/// The ROTATION PERIOD `P_rot` (days) at a stellar age, the SPIN-DOWN that closes the last interim in the L_X
/// chain: it supplies the numerator of the Rossby number ([`stellar_rossby_number`]) the whole activity-and-wind
/// chain reads. Magnetized stars shed angular momentum in their winds and spin down, and the empirical law is
/// Skumanich's `P_rot ~ t^n` with `n` near one half (Skumanich 1972; refined by Barnes 2007 and Mamajek and
/// Hillenbrand 2008 to the ~0.5 to 0.57 range). So `P_rot(t) = P_ref * (t / t_ref)^n`, a power law in age, the
/// same shape the accretion clock uses in time. Computed in the log domain (the ratio may be below one for a star
/// younger than the reference, so the log is signed).
///
/// THE CARRIER IS MASS-AGNOSTIC BY CONSTRUCTION, and that is the design: the age evolution `(t/t_ref)^n` carries
/// no colour or mass, so the star does not synthesize a colour to look one up (the keying resolution the
/// convective-turnover fit already took, keying on the drawn physical variable). The MASS DEPENDENCE lives
/// entirely in the reference rotation `P_ref`, which a gyrochrone supplies as a function of the star (the classic
/// Barnes / Mamajek-Hillenbrand gyrochrones key that normalization on `B-V` colour, and a mass-keyed calibration
/// is the follow-on that resolves the colour-versus-mass seam there, not here). `P_ref` is DRAW-PENDING, the
/// interim-plus-destination pattern `Mdot_0` takes: the destination is the mass gyrochrone (converged old stars)
/// over the layer-4 `Omega_star_0` birth rotation (young stars, before the gyrochrone erases the initial
/// condition), and until that draw lands the caller passes a solar-interim reference (the Sun near `25.4` days at
/// `4570` Myr). So this builds the spin-down SHAPE, the last mechanism the L_X chain needed, and leaves only that
/// one normalization draw.
///
/// The `skumanich_exponent` is reserved-with-basis (the Skumanich-to-gyrochrone band, its basis the chosen
/// gyrochrone's age index), not authored inline. `None` on a non-positive age, reference period, reference age, or
/// exponent, or an intermediate past the representable range.
///
/// CONSUMER SPLIT (a domain statement, the gate ruling). This function's two honest limits cancel by their
/// domains: the Skumanich law's validity begins where rotation CONVERGES onto the gyrochrone (gigayears in), and
/// the disk arc's window ends at dispersal (a few Myr), so they do not overlap, and that is fine. Within the disk
/// era every plausible rotation sits DEEP in saturation (`Ro` well below `ro_sat`), so the whole Rossby chain is
/// plateau-pinned: the DISK ARC consumes only the saturated branch and needs nothing from this function but the
/// confirmation that `Ro` is below the knee, which any disk-era rotation supplies. The PRECISION machinery, both
/// this spin-down law and the unsaturated activity slope, serves the ATMOSPHERE-ESCAPE arc gigayears later, where
/// stars leave saturation. So this and [`convective_turnover_time_days`] are built for that future consumer,
/// correctly dormant and correctly labelled, and the disk arc's insensitivity to their precision is a declared
/// blindness with its consumer named.
pub fn stellar_rotation_period_days(
    age_myr: Fixed,
    reference_period_days: Fixed,
    reference_age_myr: Fixed,
    skumanich_exponent: Fixed,
) -> Option<Fixed> {
    if age_myr <= Fixed::ZERO
        || reference_period_days <= Fixed::ZERO
        || reference_age_myr <= Fixed::ZERO
        || skumanich_exponent <= Fixed::ZERO
    {
        return None;
    }
    // ln P_rot = ln P_ref + n * (ln age - ln age_ref); the age ratio may be below one, so the log is signed.
    let ln_ratio = age_myr.ln().checked_sub(reference_age_myr.ln())?;
    let ln_p = reference_period_days
        .ln()
        .checked_add(skumanich_exponent.checked_mul(ln_ratio)?)?;
    // Fail loud past the representable exp ceiling rather than saturate (the surface-density precedent).
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_p >= ln_ceiling {
        return None;
    }
    Some(ln_p.exp())
}

/// The stellar ROSSBY NUMBER `Ro = P_rot / tau_conv`, the SHARED ROTATION STATE both high-energy bands read (the
/// L_X slice, ruled). Rotation is the causal upstream (the dynamo is driven by rotation against convection), the
/// activity bands are windows on the dynamo's output, so the Rossby number is the state variable and each band
/// maps it to a luminosity by its OWN measured law. It is mass-universal by construction: the mass enters only
/// through `tau_conv` ([`convective_turnover_time_days`]), so two stars of different mass at the same Rossby
/// number show the same fractional activity, which is the admit-the-alien property that gets the M dwarfs (the
/// galaxy's commonest planet hosts) right. `P_rot` is the draw-pending rotation state (the layer-4 `Omega_star_0`
/// draw plus the gyrochronology `Omega(t)` spin-down, design-only today, solar interim). `None` on a non-positive
/// input.
pub fn stellar_rossby_number(rotation_period_days: Fixed, tau_conv_days: Fixed) -> Option<Fixed> {
    if rotation_period_days <= Fixed::ZERO || tau_conv_days <= Fixed::ZERO {
        return None;
    }
    rotation_period_days.checked_div(tau_conv_days)
}

/// The GYROCHRONOLOGICAL SPIN-DOWN MODEL: the age-scaling exponent `n` of the magnetic-braking law that ages a
/// star's rotation forward, `P(t) ~ t^n` (equivalently Skumanich's `v ~ t^(-n)` with `n = 1/2`, since a longer
/// period is a slower rotation). The FORM is fixed dynamo physics, a wind-braked star spins down as a power law of
/// age; the MEMBER is the cited exponent, a declared ensemble the way [`CollapseModel`] and [`XrayWindFit`] carry
/// their measured members, so a different calibration is a data row, never a rewrite. The band runs from
/// Skumanich's canonical `1/2` through the modern gyrochronology recalibrations (`0.5189`, `0.566`), a real
/// measured spread rather than one authored point, which the caller propagates as an interval the way the collapse
/// and wind bands already flow.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SpinDownModel {
    /// The braking-law age exponent `n` in `P(t) ~ t^n`. Skumanich `0.5`; Barnes `0.5189`; Mamajek-Hillenbrand
    /// `0.566`. Carried as the ensemble member so the exponent never travels without the calibration it was read
    /// from.
    pub braking_exponent: Fixed,
}

impl SpinDownModel {
    /// Skumanich (1972): the canonical `v ~ t^(-1/2)` braking law, age exponent `n = 1/2` EXACTLY (the value is the
    /// power law's own, not a fit coefficient). Skumanich 1972, ApJ 171, 565 (SHA256 `9c4f2d4a...`); the exponent
    /// is conditioned on the adopted 0.4 Gyr Hyades age, and the normalization is set by the Sun plus the
    /// Pleiades/UMa/Hyades sequence, no printed coefficient.
    pub fn skumanich_1972() -> Self {
        Self {
            braking_exponent: Fixed::from_ratio(1, 2),
        }
    }

    /// Barnes (2007) separable gyrochronology, the age term `g(t) = t^0.5189`, exponent fixed by the solar anchor
    /// (`P = 26.09` d at `B-V = 0.642`, 4.566 Gyr). Barnes 2007, ApJ 669, 1167 (SHA256 `1b6e3a14...`). Valid for
    /// I-sequence rotators only, NOT the disk-locked birth population (segregated at the ~100 Myr gyrochrone), the
    /// validity window the kernel guards.
    pub fn barnes_2007() -> Self {
        Self {
            braking_exponent: Fixed::from_ratio(5189, 10_000),
        }
    }

    /// Mamajek and Hillenbrand (2008) revised gyrochronology, age exponent `n = 0.566`. Mamajek and Hillenbrand
    /// 2008, ApJ 687, 1264 (SHA256 `9e407163...`). Motivated by Barnes over-predicting Hyades periods by up to 50
    /// percent, so the recalibrated exponent is the model-choice high edge of the band against Skumanich's `0.5`;
    /// same I-sequence validity limits, anchored 130 Myr to 4.566 Gyr.
    pub fn mamajek_hillenbrand_2008() -> Self {
        Self {
            braking_exponent: Fixed::from_ratio(566, 1000),
        }
    }
}

/// The STELLAR ROTATION at a target age (days), DERIVED by aging a reference rotation forward along the
/// gyrochronological spin-down `P(t) = P_ref * (t / t_ref)^n` rather than drawn on its own axis. A wind-braked star
/// loses angular momentum as a power law of age, so once its birth rotation and one reference epoch are fixed, the
/// rotation at any later age is the braking law evaluated between them. This is why `Omega_star` at disk dispersal
/// is DERIVABLE and not an independent root (LAYER4_ROOT_CENSUS): birth rotation is regulated by disk locking
/// (correlated with the engine's own `tau_disk`), and after release the spin-down law ages it forward, so a marginal
/// `Omega` draw would author away both correlations. The young-cluster rotation distributions (Herbst 2001, Rebull
/// 2018) then validate the JOINT statistics rather than seeding an independent marginal.
///
/// The value line: ZERO reserved numbers of its own. The exponent `n` is a cited member of [`SpinDownModel`]
/// (Skumanich `1/2`, Barnes `0.5189`, Mamajek-Hillenbrand `0.566`), the reference rotation and both epochs are
/// per-star ARGUMENTS, and the spin-down onset is the caller's validity boundary. Every input is a data row (the
/// admit-the-alien test): a faster or slower birth rotator, a different braking calibration, is a data row, never a
/// rewrite. Computed in the log domain (`ln P = ln P_ref + n (ln t - ln t_ref)`) with a fail-loud ceiling, the
/// sibling discipline of [`centrifugal_radius_au`] and the Shu rate.
///
/// TERMS DROPPED, named rather than hidden. First, the VALIDITY WINDOW is enforced, not assumed: the power-law
/// calibration holds only AFTER the star leaves the disk-locked / C-sequence regime (the ~100 Myr gyrochrone), so
/// both the reference and the target epoch must sit at or past `spin_down_onset_myr`, or the kernel REFUSES
/// (`None`) rather than extrapolating the law into the birth window where it is invalid. Second, the COLOR (mass)
/// dependence is dropped: the full gyrochronology forms carry a `f(B-V)` prefactor (a redder, lower-mass star
/// brakes on a different track), and this kernel ages a GIVEN period forward color-free, so the mass dependence of
/// the braking is the named debt that a color-axis follow-on multiplies in. Third, the STALLED-BRAKING and
/// weak-braking regimes at old age and at the fast/slow extremes (the van Saders class) are omitted, valid across
/// the main-sequence I-sequence and named at the far-age edge. `None` on a non-positive input, an epoch inside the
/// birth window, or a result past the representable range.
///
// @derives: the stellar rotation period at a target age Omega_star(t) <- the gyrochronological spin-down P_ref*(t/t_ref)^n aged forward from a reference epoch, over the cited braking exponent, valid only after the disk-release onset
pub fn spin_down_rotation_period_days(
    reference_period_days: Fixed,
    reference_age_myr: Fixed,
    target_age_myr: Fixed,
    spin_down_onset_myr: Fixed,
    model: &SpinDownModel,
) -> Option<Fixed> {
    if reference_period_days <= Fixed::ZERO
        || reference_age_myr <= Fixed::ZERO
        || target_age_myr <= Fixed::ZERO
        || spin_down_onset_myr <= Fixed::ZERO
        || model.braking_exponent <= Fixed::ZERO
    {
        return None;
    }
    // The validity window: the braking law is invalid inside the disk-locked / C-sequence birth window, so both
    // epochs must sit at or past the onset. Refuse rather than extrapolate the law outside its domain.
    if reference_age_myr < spin_down_onset_myr || target_age_myr < spin_down_onset_myr {
        return None;
    }
    // ln P(t) = ln P_ref + n * (ln t - ln t_ref), the period lengthening as the star brakes.
    let ln_ratio = target_age_myr.ln().checked_sub(reference_age_myr.ln())?;
    let ln_period = reference_period_days
        .ln()
        .checked_add(model.braking_exponent.checked_mul(ln_ratio)?)?;
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_period >= ln_ceiling {
        return None;
    }
    Some(ln_period.exp())
}

/// The ACTIVITY BAND MAPPING: a high-energy band's luminosity-to-bolometric ratio `L_band / L_bol` from the
/// Rossby number, one window on the shared rotation state. Two regimes: SATURATED below the critical Rossby
/// `ro_sat`, at a constant `10^saturated_log10_fraction`; and UNSATURATED above it, declining as
/// `(Ro / ro_sat)^beta`. It is BAND-AGNOSTIC by construction: the FORM (a saturated power law) is the shared
/// dynamo physics, and the BAND is entirely in the coefficient set the caller passes. That is the literal shape
/// of the welded-bands cure, one state with two MEASURED mappings DERIVES the `L_X / L_EUV` ratio (it evolves
/// because the coefficients differ), where one shared exponent would have AUTHORED it. To get an absolute
/// luminosity, multiply by the bolometric luminosity ([`crate::stellar::luminosity_ratio`] times `L_sun`); this
/// returns the dimensionless ratio.
///
/// TWO COEFFICIENT SETS, each reserved-with-basis and cited, are the two measured mappings on this one form:
///
/// X-RAY (the disk-wind consumer, Owen X-ray photoevaporation): Wright et al. 2011 (arXiv:1109.4634, ar5iv HTML,
/// native text so no OCR risk), 824 solar and late-type F-M stars: `ro_sat = 0.13 +/- 0.02`;
/// `saturated_log10_fraction = -3.13 +/- 0.22`; `beta` a SOURCE-INTERNAL SELECTION DICHOTOMY declared as a band
/// per the V-star precedent, `-2.70 +/- 0.13` (unbiased sub-sample, SERVES) to `-2.55 +/- 0.15` (full sample), the
/// band `[-2.70, -2.55]`, both rejecting the canonical -2 at ~5 sigma. The X-ray age decay is DERIVED: with
/// `P_rot ~ t^(1/2)` (Skumanich), `L_X / L_bol ~ Ro^beta ~ t^(beta/2)`, so the index is `beta/2 ~ -1.35`, matching
/// the independent `-1.37 +/- 0.47` (Aldarondo Quinones et al. 2025), a cross-check. TERMS DROPPED on that
/// cross-check: it assumes unsaturated Skumanich throughout, so `beta/2` describes the POST-SATURATION era only;
/// the `Omega(t)` structure renders that moot by producing plateau-then-decline from `Ro` crossing `ro_sat`.
///
/// EUV (the atmospheric-escape consumer, a NAMED SIBLING this slice does not wire): France et al. 2024, saturated
/// `L_EUV / L_bol = 9.7e-5 +/- 1.6e-5` (`saturated_log10_fraction ~ -4.01`). TWO EUV ANOMALIES surfaced, not
/// silently resolved: (1) France measures EUV against AGE, not the Rossby number, so its Rossby slope is INFERRED
/// through Skumanich, `beta_EUV = 2 * age_index = 2 * (-1.12 +/- 0.06) ~ -2.24`, a modeling assumption the X-ray
/// band did not need; (2) the EUV values are PROXY-RECONSTRUCTED (N V line and DEM, EUV being ISM-absorbed), the
/// reconstruction-modality flag on every EUV row. Open design call for the gate: whether the EUV shares the X-ray
/// dynamo `ro_sat = 0.13` or carries a band-specific threshold (its age breakpoint is 73 +/- 16 Myr against the
/// X-ray's ~100 Myr). Until ruled, the EUV coefficients are surfaced, not settled.
///
/// `None` on a non-positive Rossby or `ro_sat`.
pub fn activity_luminosity_fraction(
    rossby: Fixed,
    ro_sat: Fixed,
    saturated_log10_fraction: Fixed,
    beta: Fixed,
) -> Option<Fixed> {
    if rossby <= Fixed::ZERO || ro_sat <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let ln_saturated = saturated_log10_fraction.checked_mul(ln10)?;
    let ln_fraction = if rossby <= ro_sat {
        ln_saturated
    } else {
        // Unsaturated: multiply the saturated level by (Ro / ro_sat)^beta, in the log domain.
        let ln_ratio = rossby.ln().checked_sub(ro_sat.ln())?;
        ln_saturated.checked_add(beta.checked_mul(ln_ratio)?)?
    };
    Some(ln_fraction.exp())
}

/// The ABSOLUTE X-RAY LUMINOSITY as `log10(L_X in erg/s)`, the destination the wind rate's interim `log10(L_X)`
/// retires onto (the L_X-chain composition). It folds two dimensionless ratios the star already carries into an
/// absolute luminosity: the bolometric ratio `L_bol/L_sun` ([`crate::stellar::luminosity_ratio`]) and the
/// activity fraction `L_X/L_bol` ([`activity_luminosity_fraction`] on the star's Rossby number), through the solar
/// luminosity, `L_X = (L_bol/L_sun) * L_sun * (L_X/L_bol)`. Returned as a `log10` because `L_X ~ 1e30 erg/s`
/// overflows the fixed-point range outright, and because the wind rate consumes exactly this `log10(L_X)`, so the
/// two compose without ever forming the raw value.
///
/// ZERO NEW VALUES: `L_sun` is the floor's solar-luminosity constant ([`SOLAR_LUMINOSITY_W`], in watts, folded to
/// erg/s by the `1 W = 1e7 erg/s` decade), and the two ratios are derived upstream. This retires the
/// L_bol-times-fraction step of the wind rate's interim; the LAST remaining interim is the Rossby number's own
/// input, the rotation period `P_rot(age)` through the gyrochronology spin-down, which stays draw-pending (the
/// `Omega_star_0` birth rotation is a layer-4 spec, not yet built, the same interim-plus-destination status as
/// `Mdot_0`). So this closes the composition down to that one remaining draw. `None` on a non-positive ratio.
pub fn stellar_xray_luminosity_log10_erg_s(
    bolometric_ratio: Fixed,
    activity_fraction: Fixed,
) -> Option<Fixed> {
    if bolometric_ratio <= Fixed::ZERO || activity_fraction <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    // log10(L_sun in erg/s) = log10(L_sun in W) + 7 (the watt-to-erg/s decade).
    let log10_l_sun_erg_s = civsim_physics::saha::ln_of_decimal(SOLAR_LUMINOSITY_W)?
        .checked_div(ln10)?
        .checked_add(Fixed::from_int(7))?;
    let log10_bol = bolometric_ratio.ln().checked_div(ln10)?;
    let log10_fraction = activity_fraction.ln().checked_div(ln10)?;
    // log10(L_X) = log10(L_bol/L_sun) + log10(L_sun) + log10(L_X/L_bol); it is a log, so it stays in range.
    log10_bol
        .checked_add(log10_l_sun_erg_s)?
        .checked_add(log10_fraction)
}

/// The FORMATION EPOCH `t_formation` (Myr): the DERIVED ROOT of `T_mid(1 AU, t) = T_condensation`, the referee
/// that replaces the retired 0.19 formation-rate landmark (slice 1's closure). The formation-era midplane
/// temperature RISES with the accretion rate, and the clock's `Mdot(t)` DECLINES with age, so the midplane cools
/// monotonically and crosses the condensation front exactly once; this bisects for that crossing. ZERO NEW
/// VALUES: `condensation_temperature_k` is the banked condensation front (the ~1400 K forsterite-enstatite
/// landmark), `Mdot(t)` is [`viscous_similarity_accretion_rate`], and `midplane_temp_at_rate` maps a rate to the
/// 1 AU midplane temperature (the caller composes [`formation_midplane_temperature`] with its fixed disk
/// parameters, keeping the many-argument disk state out of this signature). Unlike a hindcast on the 0.19 rate,
/// this convicts `Mdot` because the dust column and opacity inside the temperature map are now derived, so the
/// front fixes a temperature rather than a degenerate product.
///
/// DETERMINISM: a fixed-iteration bisection (no unbounded loop), all fixed-point. `None` on a degenerate bracket,
/// a non-positive condensation temperature, or a bracket that does not STRADDLE the front (temperature at `t_lo`
/// below it or at `t_hi` above it means no crossing in range, surfaced rather than extrapolated).
#[allow(clippy::too_many_arguments)]
pub fn derive_formation_epoch_myr(
    mdot_0_msun_myr: Fixed,
    t_visc_myr: Fixed,
    decline_gamma: Fixed,
    condensation_temperature_k: Fixed,
    midplane_temp_at_rate: impl Fn(Fixed) -> Option<Fixed>,
    t_lo_myr: Fixed,
    t_hi_myr: Fixed,
    iterations: u32,
) -> Option<Fixed> {
    if t_lo_myr < Fixed::ZERO || t_hi_myr <= t_lo_myr || condensation_temperature_k <= Fixed::ZERO {
        return None;
    }
    let temp_at = |age: Fixed| -> Option<Fixed> {
        let rate =
            viscous_similarity_accretion_rate(mdot_0_msun_myr, t_visc_myr, decline_gamma, age)?;
        midplane_temp_at_rate(rate)
    };
    // Temperature declines with age, so the bracket must straddle: T(t_lo) >= T_cond >= T(t_hi).
    if temp_at(t_lo_myr)? < condensation_temperature_k
        || temp_at(t_hi_myr)? > condensation_temperature_k
    {
        return None;
    }
    let mut lo = t_lo_myr;
    let mut hi = t_hi_myr;
    let two = Fixed::from_int(2);
    for _ in 0..iterations {
        let mid = lo.checked_add(hi)?.checked_div(two)?;
        // Still too hot at the midpoint: the crossing is at a later (larger) age.
        if temp_at(mid)? > condensation_temperature_k {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo.checked_add(hi)?.checked_div(two)
}

/// THE BASIS GRADE of a draw-pending interim: whether it is independently motivated enough that a validation
/// reading it is meaningful rather than circular. A consistency check compares a DERIVED quantity against a
/// landmark; if the interims feeding the derivation were CHOSEN to reproduce that landmark, the agreement is true
/// by construction and the check is worthless (the replacement-circularity trap). This grade lets a check refuse
/// exactly that case. The two qualifying grades are a real layer-4 draw ([`InterimBasis::DrawGrade`]) and a value
/// cited to a documented population ([`InterimBasis::CitedToPopulation`], for instance a birth-accretion band or a
/// disk-size demographic); a value picked without a documented basis
/// ([`InterimBasis::ChosenWithoutBasis`]) never qualifies, because nothing stops it from being fit to the answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InterimBasis {
    /// A layer-4 draw, the real population sample: qualifies.
    DrawGrade,
    /// Cited to a documented population (a birth-accretion band, a disk-size demographic): qualifies, so a
    /// consistency check can run meaningfully BEFORE the draw lands, since the value did not come from the answer.
    CitedToPopulation,
    /// Picked without a documented basis: NEVER qualifies, because it could be fit to whatever the check compares
    /// against, which is the circularity the gate exists to refuse.
    ChosenWithoutBasis,
}

impl InterimBasis {
    /// True if the interim is independently motivated (a draw or cited to a population), so a validation reading
    /// it is meaningful rather than circular. A [`InterimBasis::ChosenWithoutBasis`] interim is never independent.
    pub fn is_independent(self) -> bool {
        matches!(
            self,
            InterimBasis::DrawGrade | InterimBasis::CitedToPopulation
        )
    }
}

/// A draw-pending interim VALUE paired with its [`InterimBasis`], so a consumer can refuse to run a validation
/// that its inputs would make meaningless. The mechanism is fixed Rust; the value and its basis are data the
/// caller supplies (Principle 11), the basis field carrying the provenance the gate reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProvenancedInterim {
    /// The interim value.
    pub value: Fixed,
    /// The basis grade of the value, the provenance a validation gates on.
    pub basis: InterimBasis,
}

/// The verdict of the formation-rate consistency check: whether the derived epoch's accretion rate agrees with
/// the retired formation-rate landmark. A verdict is returned ONLY when the check was meaningful (its interims
/// were independent); a circular configuration returns `None` from [`formation_rate_consistency`] instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FormationRateConsistency {
    /// `Mdot(t_formation)` sits within tolerance of the landmark: the derived clock and the retired landmark agree.
    Consistent,
    /// `Mdot(t_formation)` is outside tolerance: a Residual to surface, not a failure to tune away.
    Inconsistent,
}

/// PROVENANCE-GATED consistency check for the retired 0.19 formation-rate landmark. Once the formation epoch is a
/// DERIVED root ([`derive_formation_epoch_myr`]), the old landmark demotes to a check: does the derived epoch's
/// accretion rate `Mdot(t_formation)` land near the landmark the epoch used to be pinned to? That agreement is
/// only MEANINGFUL if the clock's interims (`Mdot_0`, `t_visc`) are independently motivated; fitting them to
/// reproduce the landmark (for instance `Mdot_0 = 1`, `t_visc = 0.5`, which lands `Mdot(1 Myr) = 0.192` exactly)
/// makes the agreement true by construction, the circularity this arc convicts. So this REFUSES (`None`) unless
/// EVERY interim [`InterimBasis::is_independent`], a check that reads its own inputs' provenance and knows when it
/// would be worthless. On independent interims it returns [`FormationRateConsistency`], a verdict rather than a
/// silent pass, so an inconsistency surfaces as a Residual instead of pressure to tune. `None` on a
/// chosen-without-basis interim, a non-positive landmark or tolerance, or a clock that refuses the rate.
pub fn formation_rate_consistency(
    mdot_0: ProvenancedInterim,
    t_visc: ProvenancedInterim,
    gamma: Fixed,
    t_formation_myr: Fixed,
    landmark_rate_msun_myr: Fixed,
    tolerance_frac: Fixed,
) -> Option<FormationRateConsistency> {
    // THE PROVENANCE GATE: a chosen-without-basis interim makes the whole check circular, so refuse rather than
    // report a meaningless verdict. This is what makes interim-fitting unconstructible: the fitted exploit cannot
    // even run the check, because its basis is not independent.
    if !mdot_0.basis.is_independent() || !t_visc.basis.is_independent() {
        return None;
    }
    if landmark_rate_msun_myr <= Fixed::ZERO || tolerance_frac < Fixed::ZERO {
        return None;
    }
    let rate =
        viscous_similarity_accretion_rate(mdot_0.value, t_visc.value, gamma, t_formation_myr)?;
    let diff = if rate >= landmark_rate_msun_myr {
        rate.checked_sub(landmark_rate_msun_myr)?
    } else {
        landmark_rate_msun_myr.checked_sub(rate)?
    };
    let frac = diff.checked_div(landmark_rate_msun_myr)?;
    Some(if frac <= tolerance_frac {
        FormationRateConsistency::Consistent
    } else {
        FormationRateConsistency::Inconsistent
    })
}

/// The GRAVITATIONAL RADIUS `r_g` (AU): the disk radius beyond which the photoevaporative wind's thermal energy
/// exceeds the star's gravitational binding, so the heated gas escapes (the slice-2 dispersal race). It DERIVES
/// from the stellar mass and the wind's sound speed, `r_g = G * M_star / c_s^2` with `c_s^2 = k_B * T / (mu * m_H)`,
/// so `r_g = G * M_star * mu * m_H / (k_B * T_wind)`. No reserved number of its own: it reads the stellar mass,
/// the wind temperature (a banded class value, the EUV-heated ~1e4 K wind or the harder X-ray-heated wind, per the
/// band the giant arc flagged), and the mean molecular weight of the launched gas. The GAP RADIUS where the wind
/// first opens a gap is `r_g` times a wind-physics prefactor (~0.1 to 0.2, the banded class constant the caller
/// supplies), so this returns `r_g` and the caller scales it. Computed in the log domain (the
/// `viscous_similarity_surface_density` precedent). `None` on a non-positive input or an intermediate past the
/// representable range.
pub fn gravitational_radius_au(
    star_mass_ratio: Fixed,
    wind_temperature_k: Fixed,
    mean_molecular_weight: Fixed,
) -> Option<Fixed> {
    if star_mass_ratio <= Fixed::ZERO
        || wind_temperature_k <= Fixed::ZERO
        || mean_molecular_weight <= Fixed::ZERO
    {
        return None;
    }
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    let ln_m_star = star_mass_ratio
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
    let ln_m_h = civsim_physics::saha::ln_of_decimal("1e-3")?.checked_sub(
        civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::AVOGADRO.value)?,
    )?;
    let ln_k_b = civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::BOLTZMANN.value)?;
    let ln_au = civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?;
    // ln r_g[AU] = ln G + ln M_star + ln mu + ln m_H - ln k_B - ln T - ln AU.
    let ln_rg = ln_g
        .checked_add(ln_m_star)?
        .checked_add(mean_molecular_weight.ln())?
        .checked_add(ln_m_h)?
        .checked_sub(ln_k_b)?
        .checked_sub(wind_temperature_k.ln())?
        .checked_sub(ln_au)?;
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_rg >= ln_ceiling {
        return None;
    }
    Some(ln_rg.exp())
}

/// The DISK LIFETIME `tau_disk` (Myr): the DERIVED output the whole arc is named for, the age at which the
/// wind-versus-accretion race tips. While the viscous accretion rate [`viscous_similarity_accretion_rate`]
/// exceeds the integrated photoevaporative wind loss the disk drains through the star; when the declining rate
/// falls TO the wind rate at the gap radius the wind opens a gap and clears the disk on the much shorter local
/// viscous time (Clarke, Gendrin and Sotomayor 2001; Alexander, Clarke and Pringle 2006). So `tau_disk` is the
/// crossing time, the root of `Mdot(t) = Mdot_wind`. Because the LBP decline is a monotone power law crossing a
/// CONSTANT wind rate, the crossing INVERTS in closed form rather than needing a root-finder:
/// `Mdot_0 / (1 + t/t_visc)^p = Mdot_wind` gives `t = t_visc * ((Mdot_0/Mdot_wind)^(1/p) - 1)`, with
/// `p = (5/2 - gamma)/(2 - gamma)` the same decline exponent the clock uses (so `1/p = (2 - gamma)/(5/2 - gamma)`).
///
/// This is a DERIVED output of `Mdot_0`, `t_visc`, `gamma`, and the wind rate, never a consulted constant: it is
/// what hands the #73 giant gate a real gas clock in place of the reserved `disk_gas_lifetime_myr` it retires, and
/// it validates against the Haisch-Lada / Mamajek disk-fraction-versus-age band as an OUTPUT (the
/// replacement-circularity rule: it never calibrates against the lifetime it replaces). Zero reserved values of
/// its own: it composes the clock's parameters with the wind rate the caller supplies.
///
/// TERMS-DROPPED, with the chord discipline the gate ruling requires. The wind rate is held CONSTANT across the
/// crossing, so the caller's `wind_rate_msun_myr` is the wind evaluated at ONE declared epoch: an undeclared
/// evaluation age is the chord class (a value standing at an implicit time), so the constant-wind instance must
/// state which age its wind was read at. Holding it constant is only HALF justified by the saturated plateau, and
/// naming the other half is the correction the gate caught: `L_X = plateau * L_bol`, and while the activity
/// fraction (the plateau) does vary slowly in the saturated disk era, the bolometric multiplicand does NOT.
/// A disk-era star is a pre-main-sequence contractor whose `L_bol ~ t^(-2/3)` ([`pre_main_sequence_luminosity_lsun`])
/// falls by a factor ~5 across a 1-to-10 Myr window, which through the wind's ~1.14 luminosity exponent moves the
/// rate by a comparable factor. That time dependence is therefore folded INTO the wind band, not dropped: the band
/// is already order-of-magnitude wide from the model-structure ensemble (see [`XrayWindFit`]), and a factor-five
/// decline belongs inside that statement rather than outside it. The self-consistent alternative, solving
/// `Mdot(t) = W(L_bol(t))` with the bisection pattern [`derive_formation_epoch_myr`] already carries (this closed
/// form is then the constant-wind instance the bisection brackets), is the sharper follow-on when the band is
/// tightened. EXTERNAL photoevaporation (birth-environment irradiation) is likewise omitted, its validity domain
/// the isolated star-forming environment, the dense-cluster term named for the environment-hook follow-on.
///
/// Returns `Fixed::ZERO` (immediate dispersal, no viscous era) when the wind rate already meets or exceeds the
/// peak accretion rate `Mdot_0`, since the crossing then sits at or before birth. `None` on a non-positive input,
/// a `gamma` outside `[0, 2)`, or a lifetime past the representable range.
pub fn derive_disk_lifetime_myr(
    mdot_0_msun_myr: Fixed,
    t_visc_myr: Fixed,
    decline_gamma: Fixed,
    wind_rate_msun_myr: Fixed,
) -> Option<Fixed> {
    if mdot_0_msun_myr <= Fixed::ZERO
        || t_visc_myr <= Fixed::ZERO
        || wind_rate_msun_myr <= Fixed::ZERO
        || decline_gamma < Fixed::ZERO
        || decline_gamma >= Fixed::from_int(2)
    {
        return None;
    }
    // The wind already meets or beats peak accretion: the gap opens at (or before) birth, so no viscous era.
    if wind_rate_msun_myr >= mdot_0_msun_myr {
        return Some(Fixed::ZERO);
    }
    // 1/p = (2 - gamma) / (5/2 - gamma); at gamma = 1 this is 2/3 (p = 3/2).
    let two = Fixed::from_int(2);
    let inv_p = two
        .checked_sub(decline_gamma)?
        .checked_div(Fixed::from_ratio(5, 2).checked_sub(decline_gamma)?)?;
    // factor = (Mdot_0 / Mdot_wind)^(1/p), computed in the log domain (the ratio exceeds 1 here, so ln > 0).
    let ln_ratio = mdot_0_msun_myr.ln().checked_sub(wind_rate_msun_myr.ln())?;
    let ln_factor = inv_p.checked_mul(ln_ratio)?;
    // A REPRESENTATION-FLOOR guard (the clock precedent): a lifetime past the exp ceiling exceeds what the format
    // can hold, so surface it as unrepresentable rather than a saturated value. Unreachable for physical ratios.
    let ln_ceiling = Fixed::from_int(31).checked_mul(two.ln())?;
    if ln_factor >= ln_ceiling {
        return None;
    }
    // tau_disk = t_visc * (factor - 1).
    t_visc_myr.checked_mul(ln_factor.exp().checked_sub(Fixed::ONE)?)
}

/// An X-ray photoevaporation wind-rate FIT: the coefficient, the two power-law exponents, the luminosity
/// normalization, and the stellar-mass range over which it holds. It carries what the wind rate the dispersal
/// race consumes needs, as data rather than an inline constant (the [`ConvectiveTurnoverFit`] precedent). The
/// coefficient is stored as a `log10` because the physical value (`~6e-9 M_sun/yr`) sits near the fixed-point
/// floor, and the luminosity normalization as a `log10` because `L_X ~ 1e30 erg/s` overflows the format outright,
/// so the whole rate is computed in the log domain.
///
/// THE FIT IS ONE INSTANCE OF A CONTESTED FAMILY (the model-structure band the arc declares, the same treatment
/// as the alpha-viscous-versus-MHD-wind transport dispute), so which coefficient set the caller supplies is a
/// declared choice, not a settled law. Three rows are on the table, each reserved-with-basis and cited: (1) the
/// Owen, Clarke and Ercolano 2012 APPENDIX-B population-synthesis fit, the near-linear
/// `Mdot_w = 6.25e-9 (M_star/M_sun)^-0.068 (L_X/1e30)^1.14 M_sun/yr` (the widely-used primordial-disc row); (2) the
/// same paper's EQUATION-9 analytic estimate, the strictly linear mass-independent `Mdot_w = 8e-9 (L_X/1e30)`
/// (`l_x_exponent = 1`, `mass_exponent = 0`), the paper's own order-of-magnitude form; (3) the Sellek, Grassi,
/// Picogna, Rab, Clarke and Ercolano 2024 PLUTO+PRIZMO radiation-hydro revision, which finds integrated rates
/// roughly an order of magnitude LOWER from enhanced molecular cooling (a live rival, a lower coefficient on the
/// same shape). The mechanism below applies whichever row is passed.
///
/// INTEGRATED RATES ARE CHORDS OVER THEIR INTEGRATION DOMAIN (the generalization the owner minted ruling the
/// Sellek rate pair): a photoevaporation rate is the wind integrated out to some outer radius, so a whole-disk
/// total and a rate truncated to a shorter radius are different quantities, and BAND MEMBERSHIP REQUIRES
/// DOMAIN-MATCHED ROWS. Sellek reports a PAIR: `4.32e-9 M_sun/yr` integrated to the model's 160 AU outer edge (the
/// whole-disk total, DOMAIN-MATCHED to Owen's total, so the band-serving low edge the owner ruled) and
/// `1.06e-9 M_sun/yr` truncated to 80 AU (the paper's own controlled-comparison statistic, a shorter chord, NOT
/// domain-matched to Owen's total and so NOT a band edge). Both are carried ([`XrayWindFit::sellek_2024`] and
/// [`XrayWindFit::sellek_2024_controlled_80au`]), each tagged with its [`WindIntegrationDomain`], so a consumer that
/// bands rows can refuse a domain mismatch rather than compare a total against a chord.
///
/// RULED (owner, the batch audit): all three rows ship as the DECLARED ENSEMBLE, not a single picked row, because
/// they are distinct physics claims (a population-synthesis fit, an analytic estimate, a radiation-hydro rival),
/// the radiative-conductivity dispute pattern. Their roles: the appendix-B fit is the CENTRAL instance (confirmed
/// verbatim at the primary, `arXiv:1112.1087`), equation 9 the same paper's order-of-magnitude cross-check, and
/// Sellek 2024 the LOW EDGE. THE COST, stated so no consumer is surprised: an order-of-magnitude wind band propagates
/// through the `(Mdot_0/Mdot_w)^(1/p)` inversion in [`derive_disk_lifetime_myr`] to roughly a factor `10^(1/p)`
/// band on `tau_disk`, about 4.64 at `gamma = 1` (`1/p = 2/3`), wide and honest. The Haisch-Lada and Mamajek
/// disk-fraction-versus-age data is the independent ensemble referee that discriminates WITHIN this band (legal
/// because it is independent data, never the retired `disk_gas_lifetime_myr` the replacement-circularity rule
/// forbids calibrating against).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct XrayWindFit {
    /// `log10` of the wind-rate coefficient (solar masses per YEAR) at the reference `L_X` and one solar mass.
    pub log10_coefficient_msun_yr: Fixed,
    /// `log10` of the reference X-ray luminosity (erg/s) the fit normalizes to (30 for the `1e30` normalization).
    pub log10_l_x_reference_erg_s: Fixed,
    /// The exponent on `(L_X / L_X_reference)` (near 1: the rate scales almost linearly with X-ray luminosity).
    pub l_x_exponent: Fixed,
    /// The exponent on `(M_star / M_sun)` (near 0: the rate depends only weakly on stellar mass).
    pub mass_exponent: Fixed,
    /// Fit validity lower bound (solar masses), the low-mass edge of the sample the fit was measured over.
    pub mass_min_msun: Fixed,
    /// Fit validity upper bound (solar masses); Owen's sample is low-mass stars, so above it the fit is unproven.
    pub mass_max_msun: Fixed,
    /// The metallicity `Z/Z_sun` the coefficients were fit at (solar = 1), the domain-of-validity marker in
    /// COMPOSITION, the sibling of `mass_min_msun`/`mass_max_msun`. The wind rate carries a NEGATIVE metallicity
    /// slope `Mdot ~ Z^s` (a distinct axis from which [`XrayWindFit`] row the caller picks), so a draw off this
    /// composition applies the slope through [`XrayWindFit::metallicity_rate_factor`] rather than the solar fit as if
    /// composition did not matter. This field records the SAMPLE the row was measured at;
    /// [`XrayWindFit::metallicity_domain`] classifies a draw against it and [`XrayWindFit::metallicity_rate_factor`] moves the
    /// rate.
    pub sample_metallicity: Fixed,
    /// The RADIAL INTEGRATION DOMAIN the coefficient's integrated rate was measured over: the scope marker on the
    /// integrated-rate axis, the sibling of the mass range and the sampled metallicity. An X-ray photoevaporation
    /// rate is a CHORD over the radius it is integrated to, so two rows are a legal band only when their domains
    /// match ([`WindIntegrationDomain::matches`], the domain-matched-rows rule). A whole-disk total against a
    /// rate truncated to a shorter radius would misstate the band width, which is why the Sellek 160 AU total
    /// (domain-matched to Owen's total) serves the band and the 80 AU controlled statistic does not.
    pub integration_domain: WindIntegrationDomain,
}

/// The RADIAL INTEGRATION DOMAIN a wind-rate coefficient was integrated over, the SCOPE of an integrated rate.
/// A photoevaporation rate is the wind integrated out to some radius, so it is a CHORD, and two integrated rates
/// are comparable (bandable) only when their chords span the same domain (the owner's domain-matched-rows rule).
/// The variant carries the radius where a source states one, so the axis is open rather than a fixed set of named
/// radii: a future row integrated to any radius is a data value, not a new arm.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindIntegrationDomain {
    /// The total integrated wind rate over the whole disk, with no radial truncation stated by the source (Owen's
    /// population-synthesis and analytic rows, and the Sellek total integrated to the model's own outer edge).
    TotalDisk,
    /// The rate integrated to a stated outer radius (AU): a truncated chord, NOT the whole-disk total. The Sellek
    /// 80 AU controlled-comparison statistic is this; it does not band against a [`WindIntegrationDomain::TotalDisk`]
    /// row.
    WithinRadiusAu(Fixed),
}

impl WindIntegrationDomain {
    /// Whether two integration domains are the SAME chord, so the rates they scope may form a band (the
    /// domain-matched-rows rule). Two totals match; two truncated rates match iff they share the outer radius; a
    /// total never matches a truncated chord. This is the guard a band-forming consumer runs before it treats two
    /// [`XrayWindFit`] rows as edges of one wind band.
    pub fn matches(self, other: Self) -> bool {
        match (self, other) {
            (WindIntegrationDomain::TotalDisk, WindIntegrationDomain::TotalDisk) => true,
            (
                WindIntegrationDomain::WithinRadiusAu(a),
                WindIntegrationDomain::WithinRadiusAu(b),
            ) => a == b,
            _ => false,
        }
    }
}

impl XrayWindFit {
    /// The Owen, Clarke and Ercolano 2012 APPENDIX-B population-synthesis fit, the CENTRAL row of the declared
    /// wind ensemble, as cited data: `Mdot_w = 6.25e-9 (M_star/M_sun)^-0.068 (L_X/1e30)^1.14 M_sun/yr`, a total
    /// disk rate over solar-metallicity low-mass (`0.1` to `1.5 M_sun`) stars. Confirmed verbatim against the
    /// primary (`arXiv:1112.1087`, equation B1). The coefficient is stored as `log10(6.25e-9) = -8.20412`.
    pub fn owen_appendix_b() -> Self {
        XrayWindFit {
            log10_coefficient_msun_yr: Fixed::from_ratio(-820412, 100_000), // log10(6.25e-9)
            log10_l_x_reference_erg_s: Fixed::from_int(30),                 // L_X_ref = 1e30 erg/s
            l_x_exponent: Fixed::from_ratio(114, 100),                      // 1.14
            mass_exponent: Fixed::from_ratio(-68, 1000),                    // -0.068
            mass_min_msun: Fixed::from_ratio(1, 10), // 0.1 M_sun (sample low-mass edge)
            mass_max_msun: Fixed::from_ratio(15, 10), // 1.5 M_sun (sample high edge)
            sample_metallicity: Fixed::ONE, // solar: the composition the coefficients were fit at
            integration_domain: WindIntegrationDomain::TotalDisk, // whole-disk total, no radial truncation stated
        }
    }

    /// The Owen, Clarke and Ercolano 2012 EQUATION-9 analytic estimate, the same paper's ORDER-OF-MAGNITUDE
    /// cross-check of appendix B, as cited data: `Mdot_w = 8e-9 (L_X/1e30) M_sun/yr`, strictly linear in `L_X`
    /// (`l_x_exponent = 1`) and mass-independent (`mass_exponent = 0`). Confirmed verbatim against the primary
    /// (`arXiv:1112.1087`, equation 9). The coefficient is stored as `log10(8e-9) = -8.09691`. The mass range is
    /// carried as the low-mass regime the X-ray-driven family is scoped to, even though the rate itself does not
    /// read mass, so the domain guard in [`photoevaporative_wind_rate_msun_myr`] holds the same stellar window as
    /// its sibling rows.
    pub fn owen_equation_9() -> Self {
        XrayWindFit {
            log10_coefficient_msun_yr: Fixed::from_ratio(-809691, 100_000), // log10(8e-9)
            log10_l_x_reference_erg_s: Fixed::from_int(30),                 // L_X_ref = 1e30 erg/s
            l_x_exponent: Fixed::ONE,   // strictly linear in L_X
            mass_exponent: Fixed::ZERO, // mass-independent
            mass_min_msun: Fixed::from_ratio(1, 10), // 0.1 M_sun (family scope, low-mass)
            mass_max_msun: Fixed::from_ratio(15, 10), // 1.5 M_sun (family scope, high edge)
            sample_metallicity: Fixed::ONE, // solar
            integration_domain: WindIntegrationDomain::TotalDisk, // whole-disk total
        }
    }

    /// The Sellek, Grassi, Picogna, Rab, Clarke and Ercolano 2024 PLUTO+PRIZMO radiation-hydro revision, the LOW
    /// EDGE of the declared wind band, as cited data: the WHOLE-DISK TOTAL `4.32e-9 M_sun/yr` integrated to the
    /// model's 160 AU outer edge, the value DOMAIN-MATCHED to Owen's total and so the band-serving edge the owner
    /// ruled (the domain-matched-rows rule). Sellek reports a total rate, not a re-fit of the mass and `L_X`
    /// exponents, so this row INHERITS the Owen appendix-B shape (`l_x_exponent = 1.14`, `mass_exponent = -0.068`)
    /// and supplies only the lower normalization, the honest interim stated so no consumer reads a Sellek-measured
    /// mass slope that does not exist. The coefficient is stored as `log10(4.32e-9) = -8.36452`. Its sibling
    /// [`XrayWindFit::sellek_2024_controlled_80au`] carries the paper's 80 AU controlled statistic, which is NOT a
    /// band edge because its chord does not match Owen's total.
    pub fn sellek_2024() -> Self {
        XrayWindFit {
            log10_coefficient_msun_yr: Fixed::from_ratio(-836452, 100_000), // log10(4.32e-9), total to 160 AU
            log10_l_x_reference_erg_s: Fixed::from_int(30),                 // L_X_ref = 1e30 erg/s
            l_x_exponent: Fixed::from_ratio(114, 100), // 1.14, inherited Owen appendix-B shape
            mass_exponent: Fixed::from_ratio(-68, 1000), // -0.068, inherited Owen appendix-B shape
            mass_min_msun: Fixed::from_ratio(1, 10),   // 0.1 M_sun (family scope, low-mass)
            mass_max_msun: Fixed::from_ratio(15, 10),  // 1.5 M_sun (family scope, high edge)
            sample_metallicity: Fixed::ONE, // solar: Sellek ran a solar-metallicity model
            integration_domain: WindIntegrationDomain::TotalDisk, // total over the model's 160 AU outer edge
        }
    }

    /// The Sellek et al. 2024 CONTROLLED-COMPARISON statistic: the rate the paper integrates only to 80 AU,
    /// `1.06e-9 M_sun/yr`, PRESERVED as cited data but explicitly NOT a band edge. Its integration domain is
    /// [`WindIntegrationDomain::WithinRadiusAu`] at 80 AU, a shorter chord than Owen's total, so
    /// [`WindIntegrationDomain::matches`] refuses to band it against the Owen rows; the band-serving Sellek edge is
    /// [`XrayWindFit::sellek_2024`] (the 160 AU total). This row exists so the paper's own controlled statistic is
    /// carried with its domain marked, for a future consumer that runs Sellek's like-for-like 80 AU comparison
    /// rather than the whole-disk band. The coefficient is stored as `log10(1.06e-9) = -8.97469`.
    pub fn sellek_2024_controlled_80au() -> Self {
        XrayWindFit {
            log10_coefficient_msun_yr: Fixed::from_ratio(-897469, 100_000), // log10(1.06e-9), truncated to 80 AU
            log10_l_x_reference_erg_s: Fixed::from_int(30),                 // L_X_ref = 1e30 erg/s
            l_x_exponent: Fixed::from_ratio(114, 100), // 1.14, inherited Owen appendix-B shape
            mass_exponent: Fixed::from_ratio(-68, 1000), // -0.068, inherited Owen appendix-B shape
            mass_min_msun: Fixed::from_ratio(1, 10),   // 0.1 M_sun (family scope, low-mass)
            mass_max_msun: Fixed::from_ratio(15, 10),  // 1.5 M_sun (family scope, high edge)
            sample_metallicity: Fixed::ONE,            // solar
            integration_domain: WindIntegrationDomain::WithinRadiusAu(Fixed::from_int(80)), // truncated chord
        }
    }
}

/// Where a drawn composition sits relative to a fit's sampled metallicity: the domain-of-validity classification
/// on the METALLICITY AXIS, the sibling of the mass-range guard. It reports POSITION only and moves no rate; the
/// rate move is [`XrayWindFit::metallicity_rate_factor`]. TWO AXES, KEPT SEPARATE: the metallicity axis (one model evaluated
/// across `Z`, the negative slope `Mdot ~ Z^s`) is ORTHOGONAL to the model-structure axis (which
/// [`XrayWindFit`] row the caller picks, Owen versus the Sellek 2024 thermochemistry revision, both at solar
/// `Z`). Sellek is a solar-metallicity model, not a low-`Z` instance, so a metal-rich draw does NOT mean "the
/// Sellek row"; it means a LOWER rate along the metallicity axis, whichever row is the base.
///
/// SIGN AND ITS DOMAIN (the audit's alien and Terran-shape notes, surfaced not hidden): the measured slope is
/// negative (a metal-poor draw runs a higher wind rate, a metal-rich draw a lower one, fetched `-0.4` to `-0.8`
/// dex per dex) because heavy-element line, molecular, and dust cooling scale with metallicity. That sign is the
/// PROTOPLANETARY-DISK regime's, NOT a universal, and the arithmetic is SIGN-GENERAL: an alien disk whose
/// composition-wind coupling differs passes its own slope (even a positive one) and is a data row, not a rewrite,
/// so `MetalPoor`/`MetalRich` name a SIDE of the sampled composition and the rate consequence follows from the
/// passed slope. HONEST LIMIT (Principle 7): the single scalar `Z/Z_sample` axis itself assumes an H-dominated,
/// metal-line-cooled disk with the FUV floor at H2 photodissociation; a disk not governed by that cooling has no
/// meaningful single metallicity axis, a residual Terran-shaped modelling choice this arc names rather than
/// buries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MetallicitySampleDomain {
    /// Metal-poor relative to the sample (`Z < sample`): a HIGHER wind rate (weaker molecular cooling), the
    /// negative slope on the metallicity axis, applied by [`XrayWindFit::metallicity_rate_factor`].
    MetalPoor,
    /// Metal-rich, or exactly on-sample (`Z >= sample`): a LOWER-or-equal wind rate (stronger cooling), the same
    /// slope, applied by [`XrayWindFit::metallicity_rate_factor`] (exactly on-sample gives a unit factor). There is
    /// no exact-equality arm: an exact-`Z == sample` case is unreachable by measure on a continuous draw, so it
    /// folds here rather than as a dead branch.
    MetalRich,
}

impl XrayWindFit {
    /// Classify a drawn metallicity `z_ratio` (`Z/Z_sun`) against the fit's `sample_metallicity`: which SIDE of
    /// the sampled composition it sits on, metal-poor (higher rate) or metal-rich (lower rate). Position only; it
    /// moves no rate, the rate move being [`XrayWindFit::metallicity_rate_factor`], and the fit-range guard lives
    /// there too. `None` on a non-positive draw or a non-positive sample.
    pub fn metallicity_domain(&self, z_ratio: Fixed) -> Option<MetallicitySampleDomain> {
        if z_ratio <= Fixed::ZERO || self.sample_metallicity <= Fixed::ZERO {
            return None;
        }
        Some(if z_ratio < self.sample_metallicity {
            MetallicitySampleDomain::MetalPoor
        } else {
            MetallicitySampleDomain::MetalRich
        })
    }
}

/// A dimensionless RATE-FACTOR BRACKET `[lo, hi]`: the band form for a multiplicative rate adjustment whose slope
/// is model-dependent, the same band-not-point discipline as the EUV luminosity bracket. `width_dex` states the
/// band width before a consumer reads the bounds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RateFactorBracket {
    lo: Fixed,
    hi: Fixed,
}

impl RateFactorBracket {
    /// The lower bound (dimensionless multiplier).
    pub fn lo(self) -> Fixed {
        self.lo
    }
    /// The upper bound (dimensionless multiplier).
    pub fn hi(self) -> Fixed {
        self.hi
    }
    /// The band width in dex (`log10(hi/lo)`). `None` on a degenerate bracket (a non-positive bound).
    pub fn width_dex(self) -> Option<Fixed> {
        if self.lo <= Fixed::ZERO || self.hi <= Fixed::ZERO {
            return None;
        }
        let ln10 = Fixed::from_int(10).ln();
        self.hi.checked_div(self.lo)?.ln().checked_div(ln10)
    }
}

impl XrayWindFit {
    /// The METALLICITY RATE FACTOR: the multiplicative adjustment `Mdot_w(Z)/Mdot_w(Z_sample) =
    /// (Z/Z_sample)^s` the photoevaporative wind rate carries with composition, a BAND because the slope `s` is
    /// model-dependent. This is the METALLICITY AXIS, ORTHOGONAL to the model-structure ([`XrayWindFit`] row)
    /// axis: it multiplies whatever base rate the chosen row gives, so Owen-versus-Sellek and
    /// metal-poor-versus-rich never weld. The slope is FIRMLY NEGATIVE (Ercolano and Clarke 2010 `Z^-0.77`,
    /// Nakatani 2018 `Z^-0.6` with X-rays and `Z^-0.4` without), so a metal-poor draw runs a HIGHER rate (a factor
    /// above one) and a metal-rich draw a LOWER one, matching the observed disc-lifetime-versus-metallicity trend
    /// (`t ~ Z^0.52`, tied to the rate by `t ~ Mdot^(-2/3)`, and `-0.77 * -2/3 ~ 0.51`).
    ///
    /// DOMAIN (TWO-ENDED, both edges from the source's own fit range, because a one-ended guard is a half-guard):
    /// the negative slope holds for `Z` between `z_floor_ratio` (~0.03 solar) and `z_ceiling_ratio` (~2 solar,
    /// the Ercolano-Clarke fit's upper edge). Below the floor the FUV-driven rate turns over, a SEPARATE regime;
    /// above the ceiling the draw is past the fitted range. A draw outside `[floor, ceiling]` REFUSES (`None`, the
    /// domain door) rather than extrapolating the slope into a regime it does not describe. `None` also on a
    /// non-positive draw, sample, floor, or ceiling, or an inverted `[floor, ceiling]`. The slope band edges and
    /// the two domain edges are the caller's reserved-with-basis-and-cited values; a Sellek-generation slope
    /// across `Z` does not exist in the literature (Sellek ran only solar), so it is not authored here.
    pub fn metallicity_rate_factor(
        &self,
        z_ratio: Fixed,
        slope_steep: Fixed,
        slope_shallow: Fixed,
        z_floor_ratio: Fixed,
        z_ceiling_ratio: Fixed,
    ) -> Option<RateFactorBracket> {
        if z_ratio <= Fixed::ZERO
            || self.sample_metallicity <= Fixed::ZERO
            || z_floor_ratio <= Fixed::ZERO
            || z_ceiling_ratio < z_floor_ratio
        {
            return None;
        }
        if z_ratio < z_floor_ratio || z_ratio > z_ceiling_ratio {
            return None; // outside the fitted slope domain: the FUV-turnover floor or the fit's upper edge
        }
        let z = z_ratio.checked_div(self.sample_metallicity)?;
        // (Z/Z_sample)^s at each slope edge; min/max orders the band whichever way the edges are passed.
        let f_a = z.powf(slope_steep);
        let f_b = z.powf(slope_shallow);
        let (lo, hi) = if f_a <= f_b { (f_a, f_b) } else { (f_b, f_a) };
        Some(RateFactorBracket { lo, hi })
    }
}

/// The PHOTOEVAPORATIVE WIND MASS-LOSS RATE (solar masses per Myr), the input the dispersal race
/// ([`derive_disk_lifetime_myr`]) crosses the declining accretion rate against. It wires to the star's own
/// high-energy output: the wind is X-ray-driven, so the rate is a power law on the stellar X-ray luminosity `L_X`
/// (and a weak power on stellar mass), read from the [`XrayWindFit`] the caller supplies:
/// `Mdot_w = C (M_star/M_sun)^a (L_X/L_X_ref)^b`, converted from the fit's per-year coefficient to the per-Myr
/// units the accretion clock uses (a factor `1e6`). Computed entirely in the log domain, since both `L_X` and the
/// coefficient sit outside the fixed-point range: `L_X` is passed as `log10(L_X in erg/s)` (about 30 for a young
/// solar analogue), never a raw value.
///
/// `L_X` ITSELF IS A DRAW-PENDING DERIVATION, the interim-plus-destination treatment `Mdot_0` takes: the
/// destination is `L_X = L_bol * (L_X/L_bol)` with the fraction from [`activity_luminosity_fraction`] on the
/// star's Rossby number and the bolometric luminosity from [`crate::stellar::luminosity_ratio`], the activity
/// chain this arc's L_X slice already built as dormant pieces; until that chain is composed and the rotation state
/// is drawn, the caller passes a solar-interim `log10(L_X)`. So this function homes the wind rate in the engine's
/// own activity physics rather than a reserved number, with only the fit coefficients (Owen 2012 or its rivals)
/// reserved-with-basis.
///
/// DOMAIN GUARD: the fit is measured over low-mass stars, so this returns `None` for a stellar mass outside
/// `[mass_min, mass_max]` rather than extrapolate the wind physics into the intermediate-mass regime where the
/// X-ray driver and the disc structure both change. `None` also on a non-positive mass or an intermediate past the
/// representable range.
pub fn photoevaporative_wind_rate_msun_myr(
    log10_l_x_erg_s: Fixed,
    star_mass_ratio: Fixed,
    fit: &XrayWindFit,
) -> Option<Fixed> {
    if star_mass_ratio <= Fixed::ZERO
        || star_mass_ratio < fit.mass_min_msun
        || star_mass_ratio > fit.mass_max_msun
    {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10_m = star_mass_ratio.ln().checked_div(ln10)?;
    // log10(Mdot in M_sun/yr) = log10(C) + a*log10(M_star) + b*(log10(L_X) - log10(L_X_ref)).
    let log10_l_x_term = fit
        .l_x_exponent
        .checked_mul(log10_l_x_erg_s.checked_sub(fit.log10_l_x_reference_erg_s)?)?;
    let log10_rate_yr = fit
        .log10_coefficient_msun_yr
        .checked_add(fit.mass_exponent.checked_mul(log10_m)?)?
        .checked_add(log10_l_x_term)?;
    // Convert per-year to per-Myr: add log10(1e6) = 6.
    let log10_rate_myr = log10_rate_yr.checked_add(Fixed::from_int(6))?;
    let ln_rate = log10_rate_myr.checked_mul(ln10)?;
    // Fail loud past the representable exp ceiling rather than saturate (the surface-density precedent).
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_rate >= ln_ceiling {
        return None;
    }
    Some(ln_rate.exp())
}

/// A PHOTOEVAPORATION RATE BRACKET (solar masses per Myr), the RIDER 2 output form for the radiative-envelope wind
/// rate. The ionizing photon rate that drives it is itself a band (the hot-photosphere atmosphere-model departure
/// spans decades, the [`IonizingSpectrumEvaluation`] photon-rate band), so the rate it produces is a bracket, never
/// a point: a consumer cannot read a decade-wide ignorance as a definite mass-loss rate. `[lo, hi]` in solar masses
/// per Myr, with
/// [`PhotoevaporationRateBracket::width_dex`] making the width machine-readable before any consumer reads the bounds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhotoevaporationRateBracket {
    lo_msun_myr: Fixed,
    hi_msun_myr: Fixed,
}

impl PhotoevaporationRateBracket {
    /// The lower bound (solar masses per Myr).
    pub fn lo_msun_myr(self) -> Fixed {
        self.lo_msun_myr
    }
    /// The upper bound (solar masses per Myr).
    pub fn hi_msun_myr(self) -> Fixed {
        self.hi_msun_myr
    }
    /// The bracket WIDTH in dex (`log10(hi/lo)`), the stated width RIDER 2 requires be readable before a consumer
    /// reads the bounds. `None` on a degenerate bracket (a non-positive bound).
    pub fn width_dex(self) -> Option<Fixed> {
        if self.lo_msun_myr <= Fixed::ZERO || self.hi_msun_myr <= Fixed::ZERO {
            return None;
        }
        let ln10 = Fixed::from_int(10).ln();
        self.hi_msun_myr
            .checked_div(self.lo_msun_myr)?
            .ln()
            .checked_div(ln10)
    }
}

/// A CLOSED STELLAR-MASS INTERVAL `[lo, hi]` in solar masses, the unit a fit's typed domain is built from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MassInterval {
    /// Lower edge (solar masses), inclusive.
    pub lo_solar: Fixed,
    /// Upper edge (solar masses), inclusive.
    pub hi_solar: Fixed,
}

impl MassInterval {
    /// Whether a stellar mass falls inside the closed interval.
    pub fn contains(&self, mass_solar: Fixed) -> bool {
        mass_solar >= self.lo_solar && mass_solar <= self.hi_solar
    }
}

/// The GRADE of an [`EuvWindFit`] evaluation at a given stellar mass, so a consumer can never mistake an
/// extrapolation into an unmeasured regime for an empirically grounded rate. This is the fix for the disjoint-
/// evidence defect: a single `[mass_min, mass_max]` pair turned the Herbig validation gap (about 2 to 15 solar
/// masses, between the low-mass T Tauri grounding and the massive-star numerical grounding) into ordinary in-domain
/// success. The grade travels with the rate instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FitReach {
    /// The mass sits inside the fit's empirically grounded interval: the rate carries the fit's own grade.
    Grounded,
    /// The mass sits in the analytic-extrapolation interval (the fit's power law reaching beyond where it was
    /// measured, principally the Herbig gap): ESTIMATOR grade. The carried value is the DERIVED extrapolation
    /// distance, `decades_beyond_grounded`, the base-10 magnitude in stellar mass from the nearest grounded edge,
    /// a monotone trust-decay proxy the consumer reads. It is NOT a rate-error band in dex: the rate uncertainty of
    /// bridging the unmeasured Herbig regime is not itself measured (the honest gap, pending a Herbig-regime grid),
    /// so no such width is fabricated here.
    AnalyticExtrapolation { decades_beyond_grounded: Fixed },
}

/// The TYPED DOMAIN of an EUV wind fit: the interval where its coefficients were empirically grounded, and the
/// adjacent interval its power law only reaches by analytic extrapolation, at estimator grade. Queried at a stellar
/// mass, it returns the graded [`FitReach`] or `None` (outside both intervals, a refusal). This replaces the old
/// `[mass_min, mass_max]` pair, which could not represent the disjoint support of the cited channels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EuvFitDomain {
    /// The empirically grounded interval (the channel's own measured or modelled masses).
    pub grounded: MassInterval,
    /// The analytic-extrapolation interval, adjacent to the grounded one, where the same scaling is extended into an
    /// unmeasured regime at estimator grade.
    pub extrapolation: MassInterval,
}

impl EuvFitDomain {
    /// The graded reach at a stellar mass: grounded wins where the intervals touch, then extrapolation, else `None`.
    /// The extrapolation distance is DERIVED as `|log10(mass) - log10(nearest grounded edge)|`, the decades of
    /// stellar mass past the grounded interval, so an ungrounded evaluation carries how far out on the limb it sits.
    pub fn reach_at(&self, mass_solar: Fixed) -> Option<FitReach> {
        if mass_solar <= Fixed::ZERO {
            return None;
        }
        if self.grounded.contains(mass_solar) {
            return Some(FitReach::Grounded);
        }
        if self.extrapolation.contains(mass_solar) {
            let ln10 = Fixed::from_int(10).ln();
            let log10_m = mass_solar.ln().checked_div(ln10)?;
            // The nearest grounded edge: the low edge if the mass sits below the grounded interval, else the high.
            let edge = if mass_solar < self.grounded.lo_solar {
                self.grounded.lo_solar
            } else {
                self.grounded.hi_solar
            };
            let log10_edge = edge.ln().checked_div(ln10)?;
            let decades = log10_m.checked_sub(log10_edge)?.abs();
            return Some(FitReach::AnalyticExtrapolation {
                decades_beyond_grounded: decades,
            });
        }
        None
    }
}

/// A GRADED EUV wind-rate evaluation: the rate bracket paired with the [`FitReach`] grade at the evaluated mass, so
/// a Herbig-gap extrapolation reaches the consumer tagged as estimator provenance rather than as a grounded point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhotoevaporationRateEvaluation {
    /// The mass-loss rate bracket (solar masses per Myr), the atmosphere-model band propagated through the wind law.
    pub rate: PhotoevaporationRateBracket,
    /// The grounded-or-extrapolated grade of this evaluation, from the fit's typed domain. Named `fit_reach`, not
    /// `reach`, so it does not read as the physics `laws::reach()` distance (a homonym: this is a fit-domain grade,
    /// not a runout reach), per the diamond gate's rename-when-not-one discharge.
    pub fit_reach: FitReach,
}

/// The EUV PHOTOEVAPORATION WIND-RATE FIT: the reserved-with-basis coefficients of the radiative-envelope branch's
/// mass-loss rate `Mdot = C (Phi/Phi_ref)^p (M_star/M_sun)^q` (solar masses per YEAR), where `Phi` is the star's own
/// ionizing (Lyman-continuum) photon rate in photons per second. The mechanism (the power law) is fixed Rust
/// (Principle 11); the coefficient, the reference photon rate, and the two exponents are cited data a constructor
/// supplies, one per literature channel, the way [`XrayWindFit`] carries the X-ray branch's rows. A consumer forms
/// the MODEL BAND by evaluating two channels, the analytic ceiling ([`EuvWindFit::hollenbach_1994`] or
/// [`EuvWindFit::alexander_2006`]) and the hydrodynamic floor ([`EuvWindFit::font_2004_hydrodynamic`]), the
/// analytic-versus-hydrodynamic sibling of the Owen-versus-Sellek band the X-ray branch forms. That model band is
/// ORTHOGONAL to the atmosphere-model band the EUV luminosity itself carries; both ship as brackets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EuvWindFit {
    /// `log10` of the wind-rate coefficient (solar masses per YEAR) at the reference photon rate and one solar mass.
    pub log10_coefficient_msun_yr: Fixed,
    /// `log10` of the reference ionizing photon rate (photons/s) the coefficient normalizes to (41 for the `1e41`
    /// per second T Tauri normalization the low-mass channels use).
    pub log10_phi_reference_per_s: Fixed,
    /// The exponent on `(Phi/Phi_ref)` (`1/2`: the rate scales as the square root of the ionizing photon rate).
    pub phi_exponent: Fixed,
    /// The exponent on `(M_star/M_sun)` (`1/2`: the rate scales as the square root of stellar mass).
    pub mass_exponent: Fixed,
    /// The TYPED mass domain: the grounded interval where this channel was measured, and the adjacent analytic-
    /// extrapolation interval it reaches at estimator grade. Replaces the old `[mass_min, mass_max]` pair, which
    /// could not represent that the Herbig regime (about 2 to 15 solar masses) is a validation gap, not in-domain.
    pub domain: EuvFitDomain,
}

impl EuvWindFit {
    /// The Hollenbach, Johnstone, Lizano and Shu 1994 analytic WEAK-WIND rate, the TRUE ORIGIN of the EUV
    /// photoevaporation normalization, as cited data: Eq. 3.14 `Mdot = 1.3e-5 Phi_49^(1/2) M_1^(1/2)` M_sun/yr,
    /// rescaled to the T Tauri normalization `4.1e-10 (Phi/1e41 s^-1)^(1/2) (M_star/M_sun)^(1/2)` M_sun/yr. This is
    /// the ANALYTIC rate (launch velocity `v = c_s`), a UPPER edge of the model band. Validity per the primary: the
    /// 1e4 K photoionized diffuse-field weak wind; the numerical models span 15 to 65 M_sun, the low-mass reach is
    /// the analytic scaling extended by Font 2004 and Alexander 2006. The coefficient is stored as `log10(4.1e-10)`.
    /// Vendored in `disk_arc_literature` (`hollenbach_1994`).
    pub fn hollenbach_1994() -> Self {
        EuvWindFit {
            log10_coefficient_msun_yr: Fixed::from_ratio(-938722, 100_000), // log10(4.1e-10)
            log10_phi_reference_per_s: Fixed::from_int(41),                 // Phi_ref = 1e41 s^-1
            phi_exponent: Fixed::from_ratio(1, 2),                          // 1/2
            mass_exponent: Fixed::from_ratio(1, 2),                         // 1/2
            // GROUNDED on the massive-star numerical models (about 15 to 65 solar masses, the primary's own grid);
            // the analytic scaling reaches DOWN to the T Tauri regime at estimator grade.
            domain: EuvFitDomain {
                grounded: MassInterval {
                    lo_solar: Fixed::from_int(15),
                    hi_solar: Fixed::from_int(65),
                },
                extrapolation: MassInterval {
                    lo_solar: Fixed::from_ratio(1, 10),
                    hi_solar: Fixed::from_int(15),
                },
            },
        }
    }

    /// The Alexander, Clarke and Pringle 2006 restatement of the HJLS94 rate, the CROSS-REFERENCE channel and the
    /// analytic ceiling, as cited data: Eq. 2 `Mdot = 4.4e-10 (Phi/1e41 s^-1)^(1/2) (M_star/M_sun)^(1/2)` M_sun/yr,
    /// attributed to Hollenbach et al. 1994. The 4.4e-10 sits about 7 percent above the HJLS94 rescaled 4.1e-10 (the
    /// exact analytic prefactor versus the numerically-fitted Eq. 3.14); the exponents agree exactly across both
    /// channels. The coefficient is stored as `log10(4.4e-10)`. Vendored in `disk_arc_literature` (`alexander_2006_ii`).
    pub fn alexander_2006() -> Self {
        EuvWindFit {
            log10_coefficient_msun_yr: Fixed::from_ratio(-935655, 100_000), // log10(4.4e-10)
            log10_phi_reference_per_s: Fixed::from_int(41),                 // Phi_ref = 1e41 s^-1
            phi_exponent: Fixed::from_ratio(1, 2),                          // 1/2
            mass_exponent: Fixed::from_ratio(1, 2),                         // 1/2
            // GROUNDED on the low-mass T Tauri regime (about 0.1 to 2 solar masses, where this restatement is
            // applied); the same scaling reaches UP through the Herbig gap to the massive-star edge at estimator grade.
            domain: EuvFitDomain {
                grounded: MassInterval {
                    lo_solar: Fixed::from_ratio(1, 10),
                    hi_solar: Fixed::from_int(2),
                },
                extrapolation: MassInterval {
                    lo_solar: Fixed::from_int(2),
                    hi_solar: Fixed::from_int(65),
                },
            },
        }
    }

    /// The Font, McCarthy, Johnstone and Ballantyne 2004 HYDRODYNAMIC correction, the LOW edge of the model band, as
    /// cited data: the 2D radiation-hydrodynamic simulations find the total mass-loss rate LOWER than the HJLS94
    /// analytic value by a factor of about 2.7, because the wind launches off the disk at about 0.3 to 0.4 `c_s`
    /// rather than the analytic `v = c_s` (a check at `v = c_s` recovers the analytic rate to within 10 percent).
    /// The correction is to the launch velocity, not the density normalization or the scaling, so the exponents and
    /// reference are unchanged and only the coefficient moves: the HJLS94 4.1e-10 reduced by 2.7, stored as
    /// `log10(4.1e-10 / 2.7) = log10(1.52e-10)`. Vendored in `disk_arc_literature` (`font_2004`).
    pub fn font_2004_hydrodynamic() -> Self {
        EuvWindFit {
            log10_coefficient_msun_yr: Fixed::from_ratio(-981860, 100_000), // log10(4.1e-10/2.7) = log10(1.52e-10)
            log10_phi_reference_per_s: Fixed::from_int(41),                 // Phi_ref = 1e41 s^-1
            phi_exponent: Fixed::from_ratio(1, 2),                          // 1/2
            mass_exponent: Fixed::from_ratio(1, 2),                         // 1/2
            // GROUNDED on the low-mass T Tauri regime (about 0.1 to 2 solar masses, the 2D radiation-hydrodynamic
            // simulations); the same scaling reaches UP through the Herbig gap at estimator grade.
            domain: EuvFitDomain {
                grounded: MassInterval {
                    lo_solar: Fixed::from_ratio(1, 10),
                    hi_solar: Fixed::from_int(2),
                },
                extrapolation: MassInterval {
                    lo_solar: Fixed::from_int(2),
                    hi_solar: Fixed::from_int(65),
                },
            },
        }
    }
}

/// The RADIATIVE-ENVELOPE EUV PHOTOEVAPORATION WIND RATE (solar masses per Myr), the Herbig-branch sibling of the
/// X-ray [`photoevaporative_wind_rate_msun_myr`]: the mass-loss rate a radiative-envelope star's ionizing luminosity
/// drives off its disk. A hot photosphere has no corona but is intrinsically EUV-bright, so the wind is EUV-driven
/// rather than X-ray-driven, and the rate is `Mdot = C (Phi/Phi_ref)^(1/2) (M_star/M_sun)^(1/2)` on the star's
/// ionizing photon rate `Phi` (Hollenbach et al. 1994 Eq. 3.14, cross-confirmed by Alexander et al. 2006 and Font
/// et al. 2004, all vendored), read from the [`EuvWindFit`] the caller supplies.
///
/// A BRACKET IN, A BRACKET OUT (RIDER 2). The input is the [`IonizingSpectrumEvaluation`] the radiative-envelope
/// branch derives, whose photon-rate band width is the hot-star atmosphere-model departure from the LTE baseline
/// (an NLTE evaluation) or zero (a blackbody evaluation). That band propagates through the square root to the rate,
/// so the output is a [`PhotoevaporationRateBracket`] whose width (halved in dex by the `1/2` exponent) is stated
/// before any consumer reads it. The analytic-versus-hydrodynamic MODEL band (which [`EuvWindFit`] the caller
/// passes) is the orthogonal second axis, a band the consumer forms from two fits.
///
/// THE SAME-SPECTRUM RULE. The wind consumes the photon rate `Q_H` DIRECTLY from the spectrum
/// (`photon_rate_log10_s`), never an ionizing luminosity divided here by a mean photon energy. The one
/// luminosity-to-photon-rate division lives inside [`blackbody_ionizing_spectrum`], self-consistent within the LTE
/// blackbody, and any NLTE departure is applied in PHOTON space by [`nlte_departed_ionizing_spectrum`]. So an
/// NLTE-adjusted energy is never crossed with an LTE mean energy: the three correlated departures (integrated
/// energy, photon number, mean energy) stay inside one branch. The photon rate (order `1e45` per second) and the
/// mass-loss rate (order `1e-10` solar masses per year) both sit outside the fixed-point range, so the whole rate
/// is carried as `log10` and exponentiated once at the end.
///
/// ADMITS THE ALIEN: the rate keys on the star's OWN ionizing photon rate and mass, so a hot star of any
/// composition photoevaporates its disk through its own spectrum, never a Terran template. SCOPE: the EUV
/// (hydrogen-ionizing) diffuse-field weak wind at 1e4 K, not the X-ray or FUV wind; the direct-field rate (about
/// 8.8 times higher, which dominates once the inner disk drains and turns optically thin) is a flagged later-phase
/// sibling this rate does not carry. The Herbig regime (about 2 to 15 M_sun), where the radiative-envelope dispatch
/// mostly lives, is a validation GAP bridged by the analytic scaling: the result is GRADED, so a mass in that gap
/// returns a [`PhotoevaporationRateEvaluation`] tagged [`FitReach::AnalyticExtrapolation`] (estimator provenance)
/// rather than the same rate a grounded mass would, and a mass outside the fit's whole reach REFUSES. `None` on a
/// non-star mass, a mass outside the fit's typed domain, or an intermediate past the representable range.
pub fn radiative_euv_photoevaporation_wind_rate_msun_myr(
    spectrum: &IonizingSpectrumEvaluation,
    star_mass_ratio: Fixed,
    fit: &EuvWindFit,
) -> Option<PhotoevaporationRateEvaluation> {
    if star_mass_ratio <= Fixed::ZERO {
        return None;
    }
    // The typed domain grades the mass: grounded, analytic-extrapolation (estimator), or outside (refusal).
    let reach = fit.domain.reach_at(star_mass_ratio)?;
    let ln10 = Fixed::from_int(10).ln();
    let log10 = |x: Fixed| -> Option<Fixed> { x.ln().checked_div(ln10) };
    let log10_m = log10(star_mass_ratio)?;
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    // The wind law on the photon rate Q_H read straight from the spectrum: no luminosity-to-photon division here.
    let rate_for = |log10_phi: Fixed| -> Option<Fixed> {
        // log10(Mdot in M_sun/yr) = log10(C) + p*(log10(Phi) - log10(Phi_ref)) + q*log10(M).
        let phi_term = fit
            .phi_exponent
            .checked_mul(log10_phi.checked_sub(fit.log10_phi_reference_per_s)?)?;
        let mass_term = fit.mass_exponent.checked_mul(log10_m)?;
        let log10_rate_yr = fit
            .log10_coefficient_msun_yr
            .checked_add(phi_term)?
            .checked_add(mass_term)?;
        // Per-year to per-Myr: add log10(1e6) = 6.
        let log10_rate_myr = log10_rate_yr.checked_add(Fixed::from_int(6))?;
        let ln_rate = log10_rate_myr.checked_mul(ln10)?;
        // Fail loud past the representable exp ceiling rather than saturate (the surface-density precedent).
        if ln_rate >= ln_ceiling {
            return None;
        }
        Some(ln_rate.exp())
    };
    // The photon-rate band propagates monotonically: the lower Q_H gives the lower rate, the higher the higher.
    let lo = rate_for(spectrum.photon_rate_log10_s.lo)?;
    let hi = rate_for(spectrum.photon_rate_log10_s.hi)?;
    Some(PhotoevaporationRateEvaluation {
        rate: PhotoevaporationRateBracket {
            lo_msun_myr: lo,
            hi_msun_myr: hi,
        },
        fit_reach: reach,
    })
}

/// Which EUV field drives the photoevaporative wind, a function of the disc's INNER OPTICAL-DEPTH TOPOLOGY rather
/// than a prose aside. While the inner disc is optically THICK the star's direct ionizing field cannot reach the
/// disc surface past the inner rim, so a recombination (DIFFUSE) field drives the wind; once the inner disc drains
/// and turns optically THIN the DIRECT field reaches the surface and drives a materially stronger wind (the
/// [`radiative_euv_photoevaporation_wind_rate_msun_myr`] rate is the diffuse branch, so the direct branch is the
/// load-bearing sibling this phase decides). The two branches also carry different radial scalings (the diffuse
/// integrated rate falls as `R^(-1/2)`, the direct rises as `R^(1/2)`, Alexander, Clarke and Pringle 2006, Eq. 14).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EuvDispersalPhase {
    /// The inner disc is optically thick: the recombination (diffuse) field drives the wind.
    Diffuse,
    /// The inner disc has drained optically thin: the direct field reaches the surface and drives the wind.
    Direct,
}

/// The DIFFUSE-TO-DIRECT EUV PHASE TRANSITION, cited data (Alexander, Clarke and Pringle 2006, MNRAS 369, 229;
/// vendored in `disk_arc_literature` as `alexander_2006_ii`): the inner-disc optical depth at which the direct
/// field breaks through, and the direct-to-diffuse mass-loss ratio at the solar reference. The MECHANISM (the
/// dispatch on optical depth) is fixed Rust (Principle 11); these are the paper's own numbers, a declared model the
/// way [`EuvWindFit`] carries the wind-rate rows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EuvPhaseTransition {
    /// The inner-disc optical depth at the diffuse-to-direct transition (Alexander 2006 Eq. 15: `tau = 4.61`, i.e.
    /// `exp(-tau) = 0.01`, the surface where the direct ionizing field has attenuated to one percent).
    pub transition_optical_depth: Fixed,
    /// The direct-to-diffuse INTEGRATED mass-loss ratio AT the reference stellar mass (Alexander 2006: `8.8` at
    /// `1 M_sun`). A SCOPED anchor for the "materially larger" direct branch, NOT a universal multiplier: away from
    /// the reference mass the full radial direct-field form (Eq. 3, coefficient `1.73e-9`, `C_D = 0.235`, radial
    /// exponent `a = 2.42`) sets the ratio through `Phi` and the gravitational radius, a deeper rung named here and
    /// not fabricated as a constant factor.
    pub reference_enhancement_ratio: Fixed,
    /// The stellar mass (solar masses) at which the enhancement ratio was computed (Alexander 2006: `1.0`).
    pub reference_mass_ratio: Fixed,
}

impl EuvPhaseTransition {
    /// Alexander, Clarke and Pringle 2006's diffuse-to-direct transition, as cited data (Eq. 14, 15; vendored
    /// receipt in `alexander_2006_ii`): the direct field breaks through at `tau = 4.61`, and the direct wind is
    /// about `8.8` times the diffuse at `1 M_sun`.
    pub fn alexander_2006() -> Self {
        Self {
            transition_optical_depth: Fixed::from_ratio(461, 100), // tau = 4.61 (Eq. 15)
            reference_enhancement_ratio: Fixed::from_ratio(88, 10), // 8.8x at 1 M_sun
            reference_mass_ratio: Fixed::ONE,
        }
    }
}

// @derives: the EUV dispersal phase (diffuse versus direct field) <- the disc's inner optical depth against the cited Alexander 2006 breakthrough transition tau=4.61, the topology state that decides which field drives the wind
/// Dispatch the EUV dispersal phase on the disc's INNER OPTICAL DEPTH: [`EuvDispersalPhase::Direct`] once the inner
/// disc has drained BELOW the transition optical depth (turned optically thin, so the direct field reaches the
/// surface), [`EuvDispersalPhase::Diffuse`] while it is still optically thick. Keyed on the disc's OWN inner optical
/// depth (admit-the-alien: any disc dispatches through its own topology, no Terran default), so the once-prose
/// "flagged later-phase sibling" becomes a typed branch on written state, the audit's load-bearing requirement.
/// `None` on a non-positive optical depth or transition.
pub fn euv_dispersal_phase(
    inner_disc_optical_depth: Fixed,
    transition: &EuvPhaseTransition,
) -> Option<EuvDispersalPhase> {
    if inner_disc_optical_depth <= Fixed::ZERO || transition.transition_optical_depth <= Fixed::ZERO
    {
        return None;
    }
    Some(
        if inner_disc_optical_depth < transition.transition_optical_depth {
            EuvDispersalPhase::Direct
        } else {
            EuvDispersalPhase::Diffuse
        },
    )
}

/// THE COMPOSED DISK CLOCK (Myr), CONVECTIVE (X-ray-driven) BRANCH: the disk lifetime `tau_disk` DERIVED end to
/// end from a disk-hosting star's own state, the payoff the whole arc built toward, turning `tau_disk` from a
/// consulted constant into a derived output. It chains the built pieces:
/// pre-main-sequence turnover ([`pre_main_sequence_convective_turnover_days`]) -> Rossby number
/// ([`stellar_rossby_number`]) -> X-ray activity fraction ([`activity_luminosity_fraction`]) folded with the
/// pre-main-sequence bolometric luminosity ([`pre_main_sequence_luminosity_lsun`]) into the absolute X-ray
/// luminosity ([`stellar_xray_luminosity_log10_erg_s`]) -> the photoevaporative wind rate
/// ([`photoevaporative_wind_rate_msun_myr`]) -> the accretion-versus-wind dispersal race
/// ([`derive_disk_lifetime_myr`]). This is the CONVECTIVE branch, a T Tauri star with a rotation-driven dynamo;
/// a radiative-envelope (Herbig) star has no dynamo and takes the EUV branch instead
/// ([`blackbody_ionizing_spectrum`]), dispatched on the star's envelope structure at the Kraft break, its
/// sibling.
///
/// DORMANT: no run-path caller yet; the consumer wire that feeds this `tau_disk` into the #73 giant gate and the
/// DiskGas opening lands behind a flag, presented for audit before it flips. Each link keeps its own domain
/// door, so a `None` here is one link refusing (a non-physical star, a mass outside the wind fit, an overflow),
/// propagated rather than swallowed. INTERIMS still standing, each interim-plus-destination: the rotation period
/// `P_rot` (the `Omega_star_0` birth-rotation gyrochronology, a layer-4 draw), `Mdot_0` (the disk's initial
/// accretion rate), and `t_visc` (derived from the scale radius `R_1`, itself the disk-size-demographics draw).
/// The activity fit (`ro_sat`, `saturated_log10_fraction`, `beta`) and the wind fit are the reserved-with-basis
/// data the base arc already carries. `None` if any link refuses.
#[allow(clippy::too_many_arguments)]
pub fn disk_era_xray_disk_lifetime_myr(
    mass_ratio: Fixed,
    hayashi_temp_k: Fixed,
    age_myr: Fixed,
    rotation_period_days: Fixed,
    mlt_coefficient: Fixed,
    ro_sat: Fixed,
    saturated_log10_fraction: Fixed,
    beta: Fixed,
    xray_fit: &XrayWindFit,
    mdot_0_msun_myr: Fixed,
    t_visc_myr: Fixed,
    decline_gamma: Fixed,
) -> Option<Fixed> {
    let tau_conv =
        pre_main_sequence_convective_turnover_days(mass_ratio, hayashi_temp_k, mlt_coefficient)?;
    let rossby = stellar_rossby_number(rotation_period_days, tau_conv)?;
    let activity_fraction =
        activity_luminosity_fraction(rossby, ro_sat, saturated_log10_fraction, beta)?;
    let bolometric_ratio = pre_main_sequence_luminosity_lsun(mass_ratio, hayashi_temp_k, age_myr)?;
    let log10_l_x = stellar_xray_luminosity_log10_erg_s(bolometric_ratio, activity_fraction)?;
    let wind_rate = photoevaporative_wind_rate_msun_myr(log10_l_x, mass_ratio, xray_fit)?;
    derive_disk_lifetime_myr(mdot_0_msun_myr, t_visc_myr, decline_gamma, wind_rate)
}

/// The FEEDING-ZONE (annulus) DISK MASS a planet accretes from, in `normalization`-units times AU-squared: the
/// integral `M = integral over [inner, outer] of 2*pi*r*Sigma(r) dr`, the disk mass in the orbital annulus
/// `[inner_au, outer_au]`. This is the ACCRETION-mass scaffold: the mass follows from the geometry and the surface
/// density ([`disk_surface_density`]) alone, so it needs no temperature or opacity. The COMPOSITION of that mass
/// (what condenses at the annulus) waits on the completed disk `T(r)` and the condensation sequence; this is the
/// how-much, not the what.
///
/// The integral is a BOUNDED midpoint Riemann sum over `steps` intervals (a fixed integration resolution, an
/// engine accuracy bound set by the caller, not a physical knob, so determinism holds by construction: a fixed
/// count, integer-only `Fixed`). Keeping `r` in AU (order-one) holds the `2*pi*r*Sigma*dr` accumulation in `Fixed`;
/// the physical mass scale (`AU^2` to `m^2`, the `normalization` units to `kg/m^2`) is a later unit fold. The
/// annulus bounds are the feeding-zone width, a reserved geometry input (its basis a few Hill radii of the
/// forming planet, retiring when the Hill-radius/isolation-mass closure lands). `None` on a non-positive inner
/// radius, `outer <= inner`, zero steps, a `Sigma` that fails to resolve, or an accumulation past the range.
pub fn feeding_zone_mass(
    inner_au: Fixed,
    outer_au: Fixed,
    characteristic_radius_au: Fixed,
    gamma: Fixed,
    normalization: Fixed,
    steps: u32,
) -> Option<Fixed> {
    if inner_au <= Fixed::ZERO || outer_au <= inner_au || steps == 0 {
        return None;
    }
    let span = outer_au.checked_sub(inner_au)?;
    let dr = span.checked_div(Fixed::from_int(steps as i32))?;
    let half_dr = dr.checked_div(Fixed::from_int(2))?;
    let two_pi = Fixed::PI.checked_add(Fixed::PI)?;
    let mut mass = Fixed::ZERO;
    for i in 0..steps {
        // The midpoint of interval i: inner + (i + 1/2)*dr.
        let offset = dr
            .checked_mul(Fixed::from_int(i as i32))?
            .checked_add(half_dr)?;
        let r = inner_au.checked_add(offset)?;
        let sigma = disk_surface_density(r, characteristic_radius_au, gamma, normalization)?;
        // The ring mass 2*pi*r*Sigma*dr for this interval, accumulated.
        let ring = two_pi.checked_mul(r)?.checked_mul(sigma)?.checked_mul(dr)?;
        mass = mass.checked_add(ring)?;
    }
    Some(mass)
}

/// The feeding-zone mass in EARTH MASSES, the accretion arc's mass output in a physical unit: the
/// [`feeding_zone_mass`] integral (in `normalization`-units times AU^2, the `normalization` being the surface-density
/// scale `Sigma_c` in kg/m^2) folded to kilograms by the AU-to-metre conversion, then to Earth masses. This is the
/// `M` the planet radius and the surface gravity read, so the whole accretion-to-gravity chain is now derived. The
/// caller passes the `feeding_zone_mass` result computed with `Sigma_c` in kg/m^2 (its basis the disk-mass fraction,
/// a reserved residue). The wide-magnitude fold (`AU^2 ~ 2.2e22 m^2`, `EARTH_MASS ~ 6e24 kg` overflow Q32.32 while
/// the order-one Earth-mass result fits) runs in exact rational arithmetic and rounds once, the same `BigRat` path
/// [`stellar_flux`] uses: `M_earth = output * AU_m^2 / EARTH_MASS_KG`. `None` on a non-positive input or a bad
/// anchor.
pub fn feeding_zone_mass_earth(feeding_zone_mass_output: Fixed) -> Option<Fixed> {
    if feeding_zone_mass_output <= Fixed::ZERO {
        return None;
    }
    let au = BigRat::from_decimal_str(ASTRONOMICAL_UNIT_M).ok()?;
    let au2 = au.mul(&au);
    let earth = BigRat::from_decimal_str(EARTH_MASS_KG).ok()?;
    let mass_kg = nonneg_fixed_to_bigrat(feeding_zone_mass_output).mul(&au2);
    let mass_earth = mass_kg.div(&earth);
    Fixed::from_bits_i128(mass_earth.round_to_scale(Fixed::FRAC_BITS)?)
}

/// The PLANET RADIUS `R` (metres) from its mass and bulk density, DERIVED by inverting the sphere volume
/// `M = (4/3) pi R^3 rho`, so `R = (3 M / (4 pi rho))^(1/3)`. This is the planet's SHAPE size, the accretion arc's
/// radius output the render draws the globe from, and the `R` the derived surface gravity `g = G M / R^2` reads
/// (closing the hardcoded-gravity retirement: the whole-planet `M` and `R` are now derived, so `g` is too).
/// `mass_earth` is the mass in Earth masses (the accretion integral's output, scaled by [`EARTH_MASS_KG`]);
/// `bulk_density_g_cm3` is the whole-planet mean density (the differentiated core-plus-mantle mean, ~5.51 for
/// Earth, NOT the silicate ~3.3), the materials arc's output.
///
/// The wide-magnitude cube root runs in LOG-SPACE (`M ~ 6e24 kg` and the `~1e21 m^3` volume overflow Q32.32 while
/// the ~6.4e6 m radius fits): `ln R = (1/3)(ln(3/(4 pi)) + ln M_kg - ln rho_kg_m3)`, each term assembled from the
/// register/anchor logs, then exponentiated. At one Earth mass and Earth's ~5.514 g/cm^3 mean density this derives
/// ~6371 km, the derive-not-fit anchor (the Hadean-gate radius target). `None` on a non-positive input or a
/// register miss.
pub fn planet_radius_m(mass_earth: Fixed, bulk_density_g_cm3: Fixed) -> Option<Fixed> {
    if mass_earth <= Fixed::ZERO || bulk_density_g_cm3 <= Fixed::ZERO {
        return None;
    }
    let four_pi = Fixed::PI.checked_mul(Fixed::from_int(4))?;
    let ln_3_over_4pi = Fixed::from_int(3).checked_div(four_pi)?.ln();
    // ln M[kg] = ln(mass_earth) + ln(EARTH_MASS_KG).
    let ln_m_kg = mass_earth
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(EARTH_MASS_KG)?)?;
    // ln rho[kg/m^3] = ln(rho[g/cm^3]) + ln(1000).
    let ln_rho_kg_m3 = bulk_density_g_cm3
        .ln()
        .checked_add(Fixed::from_int(1000).ln())?;
    let ln_r = ln_3_over_4pi
        .checked_add(ln_m_kg)?
        .checked_sub(ln_rho_kg_m3)?
        .checked_div(Fixed::from_int(3))?;
    Some(ln_r.exp())
}

/// The DISK MIDPLANE TEMPERATURE `T_mid` (K), the Stage-2 payoff: the optically-thick closure that lifts the disk's
/// EFFECTIVE (surface) temperature to the interior where condensation happens, and the T-versus-kappa FIXED POINT
/// that couples the opacity to the temperature. Only the VISCOUS heating is generated in the interior, so it is
/// boosted by the optical depth, while irradiation heats the surface and is not: the midplane balance is
/// `sigma T_mid^4 = (3/4) tau_R D_visc + F_irr`, with `tau_R = kappa_R(T_mid) Sigma / 2`. Because `kappa_R` depends
/// on `T_mid` (dust sublimates as the gas warms, dropping the opacity), this is a self-consistent fixed point,
/// solved by a BOUNDED BISECTION on `T_mid` (a fixed iteration count, so determinism holds): at each trial `T` the
/// equilibrium temperature `radiative_equilibrium((3/4) tau_R(T) D_visc + F_irr)` is compared to `T`, and the
/// bracket halves toward the crossing.
///
/// `viscous_flux` is the Shakura-Sunyaev dissipation `D(r)` (W/m^2, one face) and `absorbed_irradiation_flux` the
/// reprocessed stellar flux (W/m^2); both are the disk's own derived fluxes. `surface_density` and `kappa_of_t`
/// (the Rosseland opacity as a function of temperature, the #54 grain-plus-gas opacity) carry matching units so
/// `tau_R = kappa Sigma / 2` is dimensionless. The bracket `[t_lo, t_hi]` is the search interval (the surface
/// temperature below, an optically-thick ceiling above). HONEST LIMIT: near dust sublimation the opacity cliff can
/// make three fixed points (the thermal-instability S-curve, the FU-Orionis engine); this bisection returns the
/// single crossing in the given bracket, so the caller brackets the branch it wants. `None` on a bad input, a
/// kappa the closure cannot price, or an overflow.
pub fn disk_midplane_temperature(
    viscous_flux: Fixed,
    absorbed_irradiation_flux: Fixed,
    surface_density: Fixed,
    kappa_of_t: impl Fn(Fixed) -> Option<Fixed>,
    t_lo: Fixed,
    t_hi: Fixed,
) -> Option<Fixed> {
    if surface_density <= Fixed::ZERO || t_hi <= t_lo {
        return None;
    }
    let sigma = crate::physiology::derived_stefan_boltzmann();
    let three_quarters = Fixed::from_ratio(3, 4);
    let half = Fixed::from_ratio(1, 2);
    // The equilibrium temperature the disk settles to at a trial midplane temperature `t` (through its opacity).
    let equilibrium_t = |t: Fixed| -> Option<Fixed> {
        let tau_r = kappa_of_t(t)?
            .checked_mul(surface_density)?
            .checked_mul(half)?; // kappa Sigma / 2
        let lifted = three_quarters
            .checked_mul(tau_r)?
            .checked_mul(viscous_flux)?
            .checked_add(absorbed_irradiation_flux)?;
        Some(civsim_physics::laws::radiative_equilibrium(
            lifted,
            Fixed::ONE,
            sigma,
            t_hi,
        ))
    };
    // Bounded bisection: below the fixed point the disk wants to be hotter (equilibrium_t > t), above it cooler.
    let mut lo = t_lo;
    let mut hi = t_hi;
    for _ in 0..60 {
        let mid = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
        if equilibrium_t(mid)? > mid {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo.checked_add(hi)?.checked_div(Fixed::from_int(2))
}

/// The FORMATION-ERA disk midplane temperature `T_mid(r)` (K) at an orbital distance: the condensation epoch's
/// temperature, DISTINCT from the mature surface warmth [`disk_effective_temperature`] gives. THE EPOCH JOIN-LAW,
/// enforced by keeping the two derivations apart: the crust CONDENSES against THIS (the hot, optically-thick,
/// accreting formation midplane), and the FINISHED planet's surface reads the MATURE
/// [`disk_effective_temperature`] (the cooled irradiation equilibrium). The two are separate epochs and are never
/// conflated under one variable.
///
/// This assembles the three inputs the optically-thick midplane closure ([`disk_midplane_temperature`]) needs and
/// calls it: the viscous dissipation flux ([`viscous_dissipation_flux`], from the FORMATION-era accretion rate, so
/// it is the hot accreting disk), the absorbed irradiation flux (`reprocessing_factor` times [`stellar_flux`]), and
/// the dust surface density `Sigma_dust(r)` ([`disk_surface_density`], the optical column the depth integrates),
/// with the caller's Rosseland-opacity closure `kappa_of_t` (kept a parameter so this stays free of any
/// materials-crate dependency and admits any opacity law). Because the viscous dissipation steepens as `r^(-3)` and
/// `Sigma` falls with distance, `T_mid(r)` falls steeply with orbit, so a closer orbit condenses a more refractory
/// crust and a farther orbit a cooler assemblage: the condensation staircase is DERIVED, not authored. Every
/// per-world input is a scenario-set ARGUMENT (the admit-the-alien test); the reserved disk residues (the
/// formation accretion rate, the surface-density normalization, the reprocessing and inner-edge factors, the
/// bisection bracket) are the caller's, each surfaced with a basis. `None` if any link fails to resolve.
#[allow(clippy::too_many_arguments)]
pub fn formation_midplane_temperature(
    accretion_rate_msun_myr: Fixed,
    mass_ratio: Fixed,
    luminosity_exponent: Fixed,
    bolometric_luminosity_lsun: Option<Fixed>,
    distance_au: Fixed,
    reprocessing_factor: Fixed,
    inner_boundary_factor: Fixed,
    characteristic_radius_au: Fixed,
    gamma: Fixed,
    dust_surface_density_normalization: Fixed,
    kappa_of_t: impl Fn(Fixed) -> Option<Fixed>,
    t_lo: Fixed,
    t_hi: Fixed,
) -> Option<Fixed> {
    let viscous_flux = viscous_dissipation_flux(
        accretion_rate_msun_myr,
        mass_ratio,
        distance_au,
        inner_boundary_factor,
    )?;
    // The irradiation term reads the star's bolometric luminosity. `bolometric_luminosity_lsun` is the door for a
    // luminosity the mass-luminosity power law cannot express (the PRE-MAIN-SEQUENCE case, where a solar-mass star
    // is several times brighter than `mass_ratio^exponent = 1`): when supplied it drives the irradiation directly
    // ([`stellar_flux_from_luminosity_lsun`]), and when `None` the term falls back to the main-sequence power law
    // ([`stellar_flux`]) byte-for-byte, so a caller that does not opt in is unchanged.
    let stellar_flux_wm2 = match bolometric_luminosity_lsun {
        Some(l_bol_lsun) => stellar_flux_from_luminosity_lsun(l_bol_lsun, distance_au)?,
        None => stellar_flux(mass_ratio, luminosity_exponent, distance_au)?,
    };
    let absorbed_irradiation_flux = reprocessing_factor.checked_mul(stellar_flux_wm2)?;
    let dust_surface_density = disk_surface_density(
        distance_au,
        characteristic_radius_au,
        gamma,
        dust_surface_density_normalization,
    )?;
    disk_midplane_temperature(
        viscous_flux,
        absorbed_irradiation_flux,
        dust_surface_density,
        kappa_of_t,
        t_lo,
        t_hi,
    )
}

/// [`formation_midplane_temperature`] for a CONSTANT Rosseland opacity, in CLOSED FORM (no bisection). When the
/// opacity does not depend on temperature (a fixed-composition dust evaluated once at a reference, the viewer's
/// case), [`disk_midplane_temperature`]'s `equilibrium_t` is independent of the trial temperature, so its fixed
/// point is DIRECT: `T_mid = radiative_equilibrium((3/4)(kappa Sigma/2) F_visc + F_irr, sigma)`. This returns that
/// value in O(1) instead of the 60-step inner bisection, so a ROOT-FINDER that evaluates the midplane at many
/// rates for ONE composition (the formation-epoch root) is not quadratic in the two nested bisections. It
/// assembles the same three inputs as the parent from the same functions, so it matches the parent (with a
/// constant `kappa_of_t`) to within the bisection residual (well under a kelvin); the DISPLAYED midplane keeps the
/// exact bisection form, so nothing a consumer reads changes. `None` on the same domain refusals as the parent, or
/// a non-positive `kappa`.
#[allow(clippy::too_many_arguments)]
pub fn formation_midplane_temperature_constant_opacity(
    accretion_rate_msun_myr: Fixed,
    mass_ratio: Fixed,
    luminosity_exponent: Fixed,
    bolometric_luminosity_lsun: Option<Fixed>,
    distance_au: Fixed,
    reprocessing_factor: Fixed,
    inner_boundary_factor: Fixed,
    characteristic_radius_au: Fixed,
    gamma: Fixed,
    dust_surface_density_normalization: Fixed,
    kappa: Fixed,
    t_hi: Fixed,
) -> Option<Fixed> {
    if kappa <= Fixed::ZERO {
        return None;
    }
    let viscous_flux = viscous_dissipation_flux(
        accretion_rate_msun_myr,
        mass_ratio,
        distance_au,
        inner_boundary_factor,
    )?;
    let stellar_flux_wm2 = match bolometric_luminosity_lsun {
        Some(l_bol_lsun) => stellar_flux_from_luminosity_lsun(l_bol_lsun, distance_au)?,
        None => stellar_flux(mass_ratio, luminosity_exponent, distance_au)?,
    };
    let absorbed_irradiation_flux = reprocessing_factor.checked_mul(stellar_flux_wm2)?;
    let dust_surface_density = disk_surface_density(
        distance_au,
        characteristic_radius_au,
        gamma,
        dust_surface_density_normalization,
    )?;
    if dust_surface_density <= Fixed::ZERO {
        return None;
    }
    let sigma = crate::physiology::derived_stefan_boltzmann();
    // tau_r = kappa * Sigma / 2, the optical half-depth; lifted = (3/4) tau_r F_visc + F_irr, the same combination
    // disk_midplane_temperature's equilibrium_t forms. With kappa constant this does not depend on the midplane
    // temperature, so the fixed point is this value directly.
    let tau_r = kappa
        .checked_mul(dust_surface_density)?
        .checked_mul(Fixed::from_ratio(1, 2))?;
    let lifted = Fixed::from_ratio(3, 4)
        .checked_mul(tau_r)?
        .checked_mul(viscous_flux)?
        .checked_add(absorbed_irradiation_flux)?;
    Some(civsim_physics::laws::radiative_equilibrium(
        lifted,
        Fixed::ONE,
        sigma,
        t_hi,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_planet_radius_derives_earth_from_its_mass_and_density() {
        // The derive-not-fit shape anchor: one Earth mass at Earth's ~5.514 g/cm^3 whole-planet mean density derives
        // the ~6371 km radius, from the sphere volume and the mass anchor alone (the Hadean-gate radius target). The
        // log-space cube root keeps the wide-magnitude compute exact.
        let r = planet_radius_m(Fixed::ONE, Fixed::from_ratio(5514, 1000)).unwrap();
        assert!(
            (r.to_f64_lossy() - 6.371e6).abs() < 1.0e5,
            "Earth radius ~6371 km, got {:.0} km",
            r.to_f64_lossy() / 1000.0
        );
        // A denser, more refractory planet at the same mass is smaller (the metal-rich inner-planet direction).
        let dense = planet_radius_m(Fixed::ONE, Fixed::from_int(8)).unwrap();
        assert!(
            dense.to_f64_lossy() < r.to_f64_lossy(),
            "a denser planet of the same mass is smaller"
        );
    }

    #[test]
    fn the_kepler_period_derives_earths_year() {
        // Kepler's third law at 1 AU around 1 M_sun derives the sidereal-grade year (~3.156e7 s, ~365.25 days)
        // from the cited AU, solar mass, and G alone, the derive-not-fit year anchor. It matches the Julian year
        // (31,557,600 s, 365.25 days) to ~0.01%, distinct from the round 365.0-day calendar fixture (31,536,000 s)
        // the run path currently carries: the derivation gives the true orbital year, not the round approximation.
        let year = kepler_orbital_period_seconds(Fixed::ONE, Fixed::ONE).unwrap();
        let julian = 31_557_600.0;
        assert!(
            (year.to_f64_lossy() - julian).abs() < 50_000.0,
            "the 1 AU / 1 M_sun period is Earth's year ~3.156e7 s, got {}",
            year.to_f64_lossy()
        );
    }

    #[test]
    fn the_kepler_period_follows_the_third_law_scaling() {
        // T^2 proportional to a^3 / M_star: quadrupling the orbit multiplies the period by 4^1.5 = 8; quadrupling
        // the star mass divides it by sqrt(4) = 2. The scaling is the law, checked as ratios so the constants drop.
        let base = kepler_orbital_period_seconds(Fixed::ONE, Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        let wider = kepler_orbital_period_seconds(Fixed::from_int(4), Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        let heavier = kepler_orbital_period_seconds(Fixed::ONE, Fixed::from_int(4))
            .unwrap()
            .to_f64_lossy();
        assert!(
            (wider / base - 8.0).abs() < 0.02,
            "T scales as a^1.5 (4^1.5 = 8), got {}",
            wider / base
        );
        assert!(
            (heavier / base - 0.5).abs() < 0.005,
            "T scales as M^-0.5 (1/2), got {}",
            heavier / base
        );
    }

    #[test]
    fn the_kepler_period_matches_jupiter() {
        // An independent real-world check: Jupiter at 5.203 AU around the Sun has an 11.86-year period. The
        // derivation reproduces it from the orbit and the solar mass alone, with no fit to Jupiter.
        let t = kepler_orbital_period_seconds(Fixed::from_ratio(5203, 1000), Fixed::ONE).unwrap();
        let expected = 11.86 * 31_557_600.0;
        assert!(
            (t.to_f64_lossy() - expected).abs() / expected < 0.01,
            "Jupiter's period ~11.86 yr, got {} yr",
            t.to_f64_lossy() / 31_557_600.0
        );
    }

    #[test]
    fn the_kepler_period_fails_loud_on_bad_inputs_and_far_orbits() {
        // Non-positive orbit or star mass has no period; a far orbit whose year in seconds crosses the Q32.32
        // ceiling (here 100 AU, ~1000 years) fails loud rather than wrapping, the honest units limit. The
        // log-space period representation for the outer system is the timestepping-arc follow-on, flagged not faked.
        assert!(kepler_orbital_period_seconds(Fixed::ZERO, Fixed::ONE).is_none());
        assert!(kepler_orbital_period_seconds(Fixed::ONE, Fixed::ZERO).is_none());
        assert!(kepler_orbital_period_seconds(Fixed::from_int(100), Fixed::ONE).is_none());
    }

    #[test]
    fn the_period_in_years_derives_earths_year_and_the_outer_system() {
        // The years form derives Earth at exactly one year, and reaches the orbits the seconds form cannot: Neptune
        // (~30 AU, 165 yr) is past the ~16 AU seconds ceiling, and an Oort body at 1e4 AU (~1e6 yr) is far past it,
        // both representable in years. Jupiter cross-checks the real 11.86 yr.
        let earth = kepler_orbital_period_years(Fixed::ONE, Fixed::ONE).unwrap();
        assert!(
            (earth.to_f64_lossy() - 1.0).abs() < 1e-3,
            "Earth is one year, got {}",
            earth.to_f64_lossy()
        );
        let jupiter =
            kepler_orbital_period_years(Fixed::from_ratio(5203, 1000), Fixed::ONE).unwrap();
        assert!(
            (jupiter.to_f64_lossy() - 11.86).abs() / 11.86 < 0.01,
            "Jupiter ~11.86 yr, got {}",
            jupiter.to_f64_lossy()
        );
        // Neptune at ~30.07 AU: past where the SECONDS form overflows, but fine in years.
        assert!(
            kepler_orbital_period_seconds(Fixed::from_ratio(3007, 100), Fixed::ONE).is_none(),
            "seconds overflows at Neptune"
        );
        let neptune =
            kepler_orbital_period_years(Fixed::from_ratio(3007, 100), Fixed::ONE).unwrap();
        assert!(
            (neptune.to_f64_lossy() - 165.0).abs() / 165.0 < 0.02,
            "Neptune ~165 yr, got {}",
            neptune.to_f64_lossy()
        );
        // An Oort-cloud body at 1e4 AU: ~1e6 years, still representable.
        let oort = kepler_orbital_period_years(Fixed::from_int(10_000), Fixed::ONE).unwrap();
        assert!(
            (oort.to_f64_lossy() - 1.0e6).abs() / 1.0e6 < 0.02,
            "1e4 AU is ~1e6 yr, got {}",
            oort.to_f64_lossy()
        );
    }

    #[test]
    fn the_period_in_years_agrees_with_the_seconds_form_and_scales_and_fails_loud() {
        // Where both are valid, years times the sidereal year equals seconds (one physics, two units); the third-law
        // scaling holds; and past the representable-years ceiling (~1.6e6 AU) it fails loud rather than saturating.
        let a = Fixed::from_int(5);
        let secs = kepler_orbital_period_seconds(a, Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        let yrs = kepler_orbital_period_years(a, Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (yrs * 31_557_600.0 - secs).abs() / secs < 0.001,
            "years*sidereal == seconds at 5 AU"
        );
        let base = kepler_orbital_period_years(Fixed::ONE, Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        let wider = kepler_orbital_period_years(Fixed::from_int(4), Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (wider / base - 8.0).abs() < 0.05,
            "T scales as a^1.5 (4^1.5 = 8), got {}",
            wider / base
        );
        assert!(kepler_orbital_period_years(Fixed::ZERO, Fixed::ONE).is_none());
        assert!(kepler_orbital_period_years(Fixed::ONE, Fixed::ZERO).is_none());
        assert!(
            kepler_orbital_period_years(Fixed::from_int(2_000_000), Fixed::ONE).is_none(),
            "past the years ceiling fails loud"
        );
    }

    #[test]
    fn the_hill_radius_matches_earth_and_jupiter() {
        // Two independent real-world anchors: Earth (1 Earth mass, 1 AU, 1 solar mass) has a Hill radius of
        // ~0.0098 AU, and Jupiter (318 Earth masses, 5.203 AU) ~0.355 AU. The derivation reproduces both from the
        // mass and orbit alone, no fit.
        let earth = hill_radius_au(Fixed::ONE, Fixed::ONE, Fixed::ONE).unwrap();
        assert!(
            (earth.to_f64_lossy() - 0.0098).abs() < 0.0005,
            "Earth's Hill radius ~0.0098 AU, got {}",
            earth.to_f64_lossy()
        );
        let jupiter = hill_radius_au(
            Fixed::from_ratio(5203, 1000),
            Fixed::from_int(318),
            Fixed::ONE,
        )
        .unwrap();
        assert!(
            (jupiter.to_f64_lossy() - 0.355).abs() / 0.355 < 0.03,
            "Jupiter's Hill radius ~0.355 AU, got {}",
            jupiter.to_f64_lossy()
        );
    }

    #[test]
    fn the_hill_radius_scales_and_fails_loud() {
        // R_H grows with the orbit (linearly) and with the body mass (as the cube root), and shrinks as the star
        // mass grows (a heavier star's tide reaches in closer). Fail-loud on any non-positive input.
        let base = hill_radius_au(Fixed::ONE, Fixed::ONE, Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        let farther = hill_radius_au(Fixed::from_int(2), Fixed::ONE, Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (farther / base - 2.0).abs() < 0.01,
            "R_H scales linearly with orbit, got {}",
            farther / base
        );
        let heavier_planet = hill_radius_au(Fixed::ONE, Fixed::from_int(8), Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (heavier_planet / base - 2.0).abs() < 0.02,
            "R_H scales as the cube root of mass (8^(1/3)=2), got {}",
            heavier_planet / base
        );
        let heavier_star = hill_radius_au(Fixed::ONE, Fixed::ONE, Fixed::from_int(8))
            .unwrap()
            .to_f64_lossy();
        assert!(
            heavier_star < base,
            "a heavier star shrinks the Hill radius"
        );
        assert!(hill_radius_au(Fixed::ZERO, Fixed::ONE, Fixed::ONE).is_none());
        assert!(hill_radius_au(Fixed::ONE, Fixed::ZERO, Fixed::ONE).is_none());
        assert!(hill_radius_au(Fixed::ONE, Fixed::ONE, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_isolation_mass_is_sub_earth_at_one_au() {
        // The honest oligarchic result: at Earth's orbit with an MMSN-grade solid surface density (~266 kg/m^2)
        // and a few-Hill-radii feeding zone, the isolation mass is SUB-EARTH (a Mars-class oligarch, ~0.05 to 0.2
        // Earth masses). This is the physics of why Earth needed oligarch mergers to reach one mass, and it
        // exercises the wide-AU log in the fold.
        let m = isolation_mass_earth(
            Fixed::ONE,                // 1 AU
            Fixed::ONE,                // 1 solar mass
            Fixed::from_ratio(266, 1), // ~266 kg/m^2 MMSN-grade solid density
            Fixed::from_ratio(35, 10), // C = 3.5 Hill radii (a classic feeding-zone width)
        )
        .unwrap();
        assert!(
            m.to_f64_lossy() > 0.02 && m.to_f64_lossy() < 0.5,
            "the 1 AU isolation mass is a sub-Earth Mars-class oligarch, got {} M_earth",
            m.to_f64_lossy()
        );
    }

    #[test]
    fn the_isolation_mass_follows_its_power_laws_and_fails_loud() {
        // M_iso proportional to a^3, Sigma^(3/2), M_star^(-1/2), (2*pi*C)^(3/2): each checked as a ratio so the
        // wide unit fold drops out. Fail-loud on any non-positive input.
        let base = isolation_mass_earth(
            Fixed::ONE,
            Fixed::ONE,
            Fixed::from_int(200),
            Fixed::from_int(5),
        )
        .unwrap()
        .to_f64_lossy();
        let wider_orbit = isolation_mass_earth(
            Fixed::from_int(2),
            Fixed::ONE,
            Fixed::from_int(200),
            Fixed::from_int(5),
        )
        .unwrap()
        .to_f64_lossy();
        assert!(
            (wider_orbit / base - 8.0).abs() / 8.0 < 0.02,
            "M_iso scales as a^3 (2^3=8), got {}",
            wider_orbit / base
        );
        let denser = isolation_mass_earth(
            Fixed::ONE,
            Fixed::ONE,
            Fixed::from_int(800),
            Fixed::from_int(5),
        )
        .unwrap()
        .to_f64_lossy();
        assert!(
            (denser / base - 8.0).abs() / 8.0 < 0.02,
            "M_iso scales as Sigma^1.5 (4^1.5=8), got {}",
            denser / base
        );
        let heavier_star = isolation_mass_earth(
            Fixed::ONE,
            Fixed::from_int(4),
            Fixed::from_int(200),
            Fixed::from_int(5),
        )
        .unwrap()
        .to_f64_lossy();
        assert!(
            (heavier_star / base - 0.5).abs() < 0.02,
            "M_iso scales as M_star^-0.5 (1/2), got {}",
            heavier_star / base
        );
        assert!(isolation_mass_earth(
            Fixed::ZERO,
            Fixed::ONE,
            Fixed::from_int(200),
            Fixed::from_int(5)
        )
        .is_none());
        assert!(
            isolation_mass_earth(Fixed::ONE, Fixed::ONE, Fixed::ZERO, Fixed::from_int(5)).is_none()
        );
        assert!(
            isolation_mass_earth(Fixed::ONE, Fixed::ONE, Fixed::from_int(200), Fixed::ZERO)
                .is_none()
        );
    }

    #[test]
    fn the_feeding_zone_mass_folds_to_earth_masses() {
        // The accretion mass fold: a feeding-zone integral of ~266.5 (Sigma_c in kg/m^2 times AU^2) reaches one Earth
        // mass, EARTH_MASS / AU^2 = 5.97e24 / 2.24e22 = 266.5. This is the M the radius and gravity read, so the
        // accretion-to-gravity chain is fully derived.
        let m = feeding_zone_mass_earth(Fixed::from_ratio(2665, 10)).unwrap();
        assert!(
            (m.to_f64_lossy() - 1.0).abs() < 0.05,
            "the fold reaches ~1 Earth mass, got {}",
            m.to_f64_lossy()
        );
    }

    #[test]
    fn the_midplane_fixed_point_lifts_the_temperature_and_is_self_consistent() {
        // The Stage-2 payoff: the optically-thick midplane, hotter than the surface by the viscous optical-depth
        // lift, at the self-consistent T-versus-kappa fixed point. A dusty opacity that drops with temperature
        // (dust sublimation): kappa = 2 - T/1000, floored. The midplane balance sigma T^4 = (3/4)(kappa Sigma/2) D
        // + F must land above the irradiation-only surface temperature, and be self-consistent at T_mid.
        let sigma = crate::physiology::derived_stefan_boltzmann();
        let kappa = |t: Fixed| -> Option<Fixed> {
            let k = Fixed::from_int(2).checked_sub(t.checked_div(Fixed::from_int(1000))?)?;
            Some(if k < Fixed::from_ratio(1, 100) {
                Fixed::from_ratio(1, 100)
            } else {
                k
            })
        };
        let viscous = Fixed::from_int(50); // D_visc W/m^2
        let irradiation = Fixed::from_int(100); // F_irr W/m^2
        let sigma_density = Fixed::from_int(100); // Sigma g/cm^2
        let t_mid = disk_midplane_temperature(
            viscous,
            irradiation,
            sigma_density,
            kappa,
            Fixed::from_int(200),
            Fixed::from_int(2000),
        )
        .unwrap();
        let t_surface = civsim_physics::laws::radiative_equilibrium(
            irradiation,
            Fixed::ONE,
            sigma,
            Fixed::from_int(2000),
        );
        assert!(
            t_mid.to_f64_lossy() > t_surface.to_f64_lossy() + 50.0,
            "the optically-thick midplane is lifted above the surface: {} vs {}",
            t_mid.to_f64_lossy(),
            t_surface.to_f64_lossy()
        );
        let tau_r = kappa(t_mid).unwrap().to_f64_lossy() * 100.0 / 2.0;
        let lifted = 0.75 * tau_r * 50.0 + 100.0;
        let t_check = civsim_physics::laws::radiative_equilibrium(
            Fixed::from_ratio((lifted * 1000.0) as i64, 1000),
            Fixed::ONE,
            sigma,
            Fixed::from_int(2000),
        );
        assert!(
            (t_mid.to_f64_lossy() - t_check.to_f64_lossy()).abs() < 5.0,
            "the fixed point is self-consistent: {} vs {}",
            t_mid.to_f64_lossy(),
            t_check.to_f64_lossy()
        );
    }

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
    fn the_direct_luminosity_flux_byte_equals_the_power_law_at_the_same_luminosity() {
        // The delegation contract: `stellar_flux` IS `stellar_flux_from_luminosity_lsun` at the power-law
        // luminosity `mass^exponent`, byte-for-byte, across mass and distance. This is what makes the pre-MS
        // override a pure door (a caller passing `None` is unchanged) and pins that the refactor moved no bit.
        for &(m, e, d) in &[
            (Fixed::ONE, Fixed::from_ratio(35, 10), Fixed::ONE),
            (Fixed::from_int(2), Fixed::from_ratio(35, 10), Fixed::ONE),
            (Fixed::ONE, Fixed::from_ratio(35, 10), Fixed::from_int(2)),
            (
                Fixed::from_ratio(1, 2),
                Fixed::from_ratio(4, 1),
                Fixed::from_int(5),
            ),
        ] {
            assert_eq!(
                stellar_flux(m, e, d),
                stellar_flux_from_luminosity_lsun(m.powf(e), d),
                "the two flux forms agree bit-for-bit at L = M^e (M={}, e={}, d={})",
                m.to_f64_lossy(),
                e.to_f64_lossy(),
                d.to_f64_lossy()
            );
        }
        // The direct door carries a luminosity the power law cannot reach at unit mass: a 3x-brighter pre-MS solar
        // analogue delivers 3x the flux, where `mass^exponent = 1` at M = 1 could only ever give the solar value.
        let solar = stellar_flux_from_luminosity_lsun(Fixed::ONE, Fixed::ONE).unwrap();
        let pre_ms = stellar_flux_from_luminosity_lsun(Fixed::from_int(3), Fixed::ONE).unwrap();
        assert!(
            (pre_ms.to_f64_lossy() / solar.to_f64_lossy() - 3.0).abs() < 0.01,
            "a 3x-brighter luminosity is 3x the flux (got {})",
            pre_ms.to_f64_lossy() / solar.to_f64_lossy()
        );
        // Fail-loud on a non-positive distance, matching the power-law form.
        assert_eq!(
            stellar_flux_from_luminosity_lsun(Fixed::ONE, Fixed::ZERO),
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
        // it falls out of L_sun, the AU, and the CODATA-derived sigma. The reprocessing factor is DERIVED geometry
        // (cross-section over sphere), the exact 1/4, not an authored constant.
        assert_eq!(
            spherical_reprocessing_factor(),
            Fixed::from_ratio(1, 4),
            "the cross-section-to-sphere geometry is the exact 1/4"
        );
        let t_max = Fixed::from_int(100_000);
        let t = irradiated_disk_temperature(
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            Fixed::ONE,
            spherical_reprocessing_factor(),
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

    #[test]
    fn the_surface_density_at_the_characteristic_radius_is_the_normalization_over_e() {
        // At r = r_c (x = 1) the profile is Sigma_c * 1^(-gamma) * exp(-1^(2-gamma)) = Sigma_c/e, whatever gamma
        // (1 to any power is 1). With Sigma_c = 1000 that is ~367.9, the derived value at the characteristic radius.
        let d = disk_surface_density(
            Fixed::from_int(10), // r = r_c
            Fixed::from_int(10), // r_c
            Fixed::ONE,          // gamma = 1
            Fixed::from_int(1000),
        )
        .expect("the surface density derives");
        assert!(
            (d.to_f64_lossy() - 1000.0 / std::f64::consts::E).abs() < 2.0,
            "Sigma(r_c) is Sigma_c/e ~367.9, got {}",
            d.to_f64_lossy()
        );
    }

    #[test]
    fn the_surface_density_rises_toward_the_inner_disk_and_truncates_outside() {
        // The power-law interior makes the disk denser inward (x < 1), the exponential cutoff makes it fall off
        // steeply outward (x > 1): Sigma(0.5 r_c) > Sigma(r_c) > Sigma(2 r_c). The characteristic radius is the
        // knee between the two.
        let (rc, gamma, norm) = (Fixed::from_int(10), Fixed::ONE, Fixed::from_int(1000));
        let inner = disk_surface_density(Fixed::from_int(5), rc, gamma, norm).unwrap();
        let knee = disk_surface_density(Fixed::from_int(10), rc, gamma, norm).unwrap();
        let outer = disk_surface_density(Fixed::from_int(20), rc, gamma, norm).unwrap();
        assert!(inner > knee, "the inner disk is denser than the knee");
        assert!(knee > outer, "the outer disk is thinner than the knee");
    }

    #[test]
    fn the_surface_density_edge_saturates_to_zero() {
        // Far beyond r_c the exp argument (-(x^(2-gamma)) = -30 at 30 r_c) passes the window floor and the
        // exponential saturates to zero, the disk's physical outer edge (no negative or wrapped density).
        let (rc, gamma, norm) = (Fixed::from_int(10), Fixed::ONE, Fixed::from_int(1000));
        let edge = disk_surface_density(Fixed::from_int(300), rc, gamma, norm).unwrap();
        assert_eq!(
            edge,
            Fixed::ZERO,
            "the disk truncates to zero past the cutoff"
        );
    }

    #[test]
    fn the_surface_density_requires_a_finite_mass_slope_and_positive_geometry() {
        let (rc, norm) = (Fixed::from_int(10), Fixed::from_int(1000));
        // gamma >= 2 has no outer cutoff (infinite mass), routed to None.
        assert!(disk_surface_density(Fixed::from_int(10), rc, Fixed::from_int(2), norm).is_none());
        // Non-positive distance or characteristic radius routes to None.
        assert!(disk_surface_density(Fixed::ZERO, rc, Fixed::ONE, norm).is_none());
        assert!(disk_surface_density(Fixed::from_int(10), Fixed::ZERO, Fixed::ONE, norm).is_none());
    }

    #[test]
    fn the_feeding_zone_mass_matches_the_lynden_bell_pringle_analytic_total() {
        // For gamma = 1 the annulus mass has a closed form: integral of 2*pi*r*Sigma dr from a to b is
        // 2*pi*Sigma_c*r_c^2*(exp(-a/r_c) - exp(-b/r_c)), because 2*pi*r*Sigma = 2*pi*Sigma_c*r_c*exp(-r/r_c) when
        // gamma = 1. The bounded midpoint sum must reproduce this from the surface density alone (the mass-
        // integration scaffold), never a fitted mass.
        let (rc, gamma, norm) = (Fixed::from_int(10), Fixed::ONE, Fixed::from_int(1000));
        let (a, b) = (Fixed::from_ratio(1, 10), Fixed::from_int(100)); // 0.1 to 100 AU
        let mass =
            feeding_zone_mass(a, b, rc, gamma, norm, 1000).expect("the annulus mass integrates");
        let (rc_f, sc_f) = (10.0_f64, 1000.0_f64);
        let analytic = 2.0
            * std::f64::consts::PI
            * sc_f
            * rc_f
            * rc_f
            * ((-0.1 / rc_f).exp() - (-100.0 / rc_f).exp());
        let got = mass.to_f64_lossy();
        assert!(
            (got - analytic).abs() / analytic < 0.01,
            "the integrated annulus mass ~{analytic:.0} (Lynden-Bell-Pringle closed form), got {got:.0}"
        );
    }

    #[test]
    fn the_feeding_zone_mass_is_deterministic_and_grows_with_the_annulus() {
        let (rc, gamma, norm) = (Fixed::from_int(10), Fixed::ONE, Fixed::from_int(1000));
        let a = Fixed::from_ratio(1, 10);
        let narrow = feeding_zone_mass(a, Fixed::from_int(20), rc, gamma, norm, 400).unwrap();
        let wide = feeding_zone_mass(a, Fixed::from_int(50), rc, gamma, norm, 400).unwrap();
        // A pure bounded read replays, and a wider annulus captures more disk mass.
        assert_eq!(
            narrow,
            feeding_zone_mass(a, Fixed::from_int(20), rc, gamma, norm, 400).unwrap(),
            "the integration replays deterministically"
        );
        assert!(wide > narrow, "a wider annulus holds more mass");
    }

    #[test]
    fn a_narrow_feeding_zone_reduces_to_the_local_ring() {
        // Over a narrow annulus around r_c the mass is the local ring 2*pi*r*Sigma(r)*width: at r_c, Sigma = Sigma_c/e
        // ~367.9, so 2*pi*10*367.9*0.2 ~4623 over the width-0.2 annulus [9.9, 10.1].
        let (rc, gamma, norm) = (Fixed::from_int(10), Fixed::ONE, Fixed::from_int(1000));
        let mass = feeding_zone_mass(
            Fixed::from_ratio(99, 10),
            Fixed::from_ratio(101, 10),
            rc,
            gamma,
            norm,
            20,
        )
        .unwrap();
        let local = 2.0 * std::f64::consts::PI * 10.0 * (1000.0 / std::f64::consts::E) * 0.2;
        let got = mass.to_f64_lossy();
        assert!(
            (got - local).abs() / local < 0.01,
            "a narrow annulus is the local ring ~{local:.0}, got {got:.0}"
        );
    }

    #[test]
    fn the_feeding_zone_mass_guards() {
        let (rc, gamma, norm) = (Fixed::from_int(10), Fixed::ONE, Fixed::from_int(1000));
        // Non-positive inner radius, an inverted or degenerate annulus, and zero steps all route to None.
        assert!(
            feeding_zone_mass(Fixed::ZERO, Fixed::from_int(10), rc, gamma, norm, 100).is_none()
        );
        assert!(feeding_zone_mass(
            Fixed::from_int(10),
            Fixed::from_int(5),
            rc,
            gamma,
            norm,
            100
        )
        .is_none());
        assert!(feeding_zone_mass(Fixed::ONE, Fixed::from_int(10), rc, gamma, norm, 0).is_none());
    }

    #[test]
    fn the_formation_midplane_lands_in_the_condensation_window_and_falls_with_orbit() {
        // The SEAM-3 formation epoch: the FORMATION-era midplane temperature, DERIVED at the orbit from the
        // viscous-plus-irradiation optically-thick fixed point with a representative constant silicate-dust Rosseland
        // opacity. It must (1) land in the silicate condensation window (~1300 to 1500 K) at 1 AU with the reserved
        // disk residues, and (2) FALL with orbit (a closer orbit hotter, a farther one cooler), the driver of the
        // orbit-dependent condensation staircase the crust reads. With a constant opacity the midplane is the direct
        // optically-thick equilibrium; the bisection returns it in the bracket.
        let kappa = |_t: Fixed| Some(Fixed::from_int(600)); // representative silicate-dust Rosseland opacity, cm^2/g
        let mid = |orbit: Fixed| {
            formation_midplane_temperature(
                Fixed::from_ratio(19, 100), // formation accretion rate (reserved, pinned to the 1 AU front)
                Fixed::ONE,
                Fixed::from_ratio(35, 10),
                None, // main-sequence power-law luminosity (no pre-MS override), byte-identical to before
                orbit,
                Fixed::from_ratio(1, 4),
                Fixed::ONE,
                Fixed::from_int(30),
                Fixed::ONE,
                Fixed::from_ratio(586, 1000),
                kappa,
                Fixed::from_int(100),
                Fixed::from_int(1950),
            )
            .unwrap()
            .to_f64_lossy()
        };
        let close = mid(Fixed::from_ratio(8, 10));
        let one = mid(Fixed::ONE);
        let far = mid(Fixed::from_int(2));
        assert!(
            one > 1300.0 && one < 1500.0,
            "the 1 AU formation midplane lands in the condensation window (~1400 K), got {one}"
        );
        assert!(
            close > one,
            "a closer orbit condenses hotter: {close} vs {one} K"
        );
        assert!(
            one > far,
            "a farther orbit condenses cooler: {one} vs {far} K"
        );
    }

    #[test]
    fn the_formation_midplane_rises_with_the_accretion_rate() {
        // A higher FORMATION accretion rate is a hotter midplane (more viscous dissipation to trap), the monotone the
        // reserved formation rate is calibrated along to pin the 1 AU condensation front.
        let kappa = |_t: Fixed| Some(Fixed::from_int(600));
        let at = |mdot: Fixed| {
            formation_midplane_temperature(
                mdot,
                Fixed::ONE,
                Fixed::from_ratio(35, 10),
                None, // main-sequence power-law luminosity (no pre-MS override)
                Fixed::ONE,
                Fixed::from_ratio(1, 4),
                Fixed::ONE,
                Fixed::from_int(30),
                Fixed::ONE,
                Fixed::from_ratio(586, 1000),
                kappa,
                Fixed::from_int(100),
                Fixed::from_int(1950),
            )
            .unwrap()
            .to_f64_lossy()
        };
        assert!(
            at(Fixed::from_ratio(25, 100)) > at(Fixed::from_ratio(10, 100)),
            "a higher accretion rate warms the formation midplane"
        );
    }

    #[test]
    fn the_formation_midplane_pre_ms_override_warms_the_disk_and_none_is_byte_neutral() {
        // The slice-3b mechanism: the bolometric-luminosity override is a pure door. `None` returns exactly the
        // main-sequence power-law midplane (byte-for-byte), so a caller that does not opt in is unchanged; and a
        // Some carrying a BRIGHTER pre-MS luminosity warms the formation midplane (more irradiation to trap),
        // which is the whole point of the third-site fix (a pre-MS star is brighter, its disk warmer, condensation
        // earlier). Same star, same orbit: only the luminosity truth moves.
        let kappa = |_t: Fixed| Some(Fixed::from_int(600));
        let mid = |l_bol: Option<Fixed>| {
            formation_midplane_temperature(
                Fixed::from_ratio(19, 100),
                Fixed::ONE,
                Fixed::from_ratio(35, 10),
                l_bol,
                Fixed::ONE,
                Fixed::from_ratio(1, 4),
                Fixed::ONE,
                Fixed::from_int(30),
                Fixed::ONE,
                Fixed::from_ratio(586, 1000),
                kappa,
                Fixed::from_int(100),
                Fixed::from_int(1950),
            )
            .unwrap()
        };
        // None equals Some(mass^exponent): at unit mass the power law is L_sun, so an explicit unit luminosity is
        // the same disk bit-for-bit as the fallback, the byte-neutrality of the door at the Mirror mass.
        assert_eq!(
            mid(None),
            mid(Some(Fixed::ONE)),
            "the override at the power-law luminosity is byte-identical to the fallback"
        );
        // A pre-MS solar analogue is several times brighter (say 4x): the warmer irradiation lifts the midplane.
        assert!(
            mid(Some(Fixed::from_int(4))) > mid(None),
            "a brighter pre-MS luminosity warms the formation midplane (pre-MS {}, MS {})",
            mid(Some(Fixed::from_int(4))).to_f64_lossy(),
            mid(None).to_f64_lossy()
        );
    }

    #[test]
    fn the_constant_opacity_closed_form_matches_the_bisection_midplane() {
        // The CLOSED-FORM midplane (no bisection) must reproduce the 60-step bisection when the opacity is constant,
        // which is what lets the formation-epoch root evaluate the midplane cheaply without the nested bisection.
        // Across a rate sweep and the MS-versus-pre-MS luminosity door, the two forms agree to within the bisection
        // residual (well under a kelvin). The DISPLAYED midplane keeps the bisection form, so nothing a consumer
        // reads changes; this only backs the root's cheap inner evaluation.
        let kappa_val = Fixed::from_int(600);
        let kappa = |_t: Fixed| Some(kappa_val);
        for &rate in &[
            Fixed::from_ratio(19, 100),
            Fixed::ONE,
            Fixed::from_int(4),
            Fixed::from_ratio(1, 100),
        ] {
            for l_bol in [None, Some(Fixed::from_int(4))] {
                let bisected = formation_midplane_temperature(
                    rate,
                    Fixed::ONE,
                    Fixed::from_ratio(35, 10),
                    l_bol,
                    Fixed::ONE,
                    Fixed::from_ratio(1, 4),
                    Fixed::ONE,
                    Fixed::from_int(30),
                    Fixed::ONE,
                    Fixed::from_ratio(586, 1000),
                    kappa,
                    Fixed::from_int(100),
                    Fixed::from_int(1950),
                )
                .unwrap();
                let closed = formation_midplane_temperature_constant_opacity(
                    rate,
                    Fixed::ONE,
                    Fixed::from_ratio(35, 10),
                    l_bol,
                    Fixed::ONE,
                    Fixed::from_ratio(1, 4),
                    Fixed::ONE,
                    Fixed::from_int(30),
                    Fixed::ONE,
                    Fixed::from_ratio(586, 1000),
                    kappa_val,
                    Fixed::from_int(1950),
                )
                .unwrap();
                assert!(
                    (bisected.to_f64_lossy() - closed.to_f64_lossy()).abs() < 0.01,
                    "closed form matches the bisection at rate {} l_bol {:?} (bisected {}, closed {})",
                    rate.to_f64_lossy(),
                    l_bol.map(|l| l.to_f64_lossy()),
                    bisected.to_f64_lossy(),
                    closed.to_f64_lossy()
                );
            }
        }
        // Fail-loud on a non-positive opacity, the new domain edge.
        assert!(formation_midplane_temperature_constant_opacity(
            Fixed::ONE,
            Fixed::ONE,
            Fixed::from_ratio(35, 10),
            None,
            Fixed::ONE,
            Fixed::from_ratio(1, 4),
            Fixed::ONE,
            Fixed::from_int(30),
            Fixed::ONE,
            Fixed::from_ratio(586, 1000),
            Fixed::ZERO,
            Fixed::from_int(1950),
        )
        .is_none());
    }

    // A Mirror-grade viscous-similarity disk realization: Mdot ~ 0.01 M_sun/Myr (~1e-8 M_sun/yr), alpha 0.01, mu 2.34
    // (a solar H2+He mix). The temperature is passed as the caller's derived disk T(r).
    fn mirror_visc(orbit_au: Fixed, temperature_k: Fixed) -> Option<Fixed> {
        viscous_similarity_surface_density(
            orbit_au,
            Fixed::ONE,                // solar-mass star
            Fixed::from_ratio(1, 100), // Mdot = 0.01 M_sun/Myr
            temperature_k,
            Fixed::from_ratio(1, 100),   // alpha = 0.01
            Fixed::from_ratio(234, 100), // mu = 2.34
        )
    }

    #[test]
    fn the_viscous_similarity_gives_an_mmsn_grade_gas_column_at_one_au() {
        // A Mirror-grade steady viscous disk at 1 AU with T ~ 280 K derives ~1341 kg/m^2 of gas (about 134 g/cm^2),
        // an order 1e3 to 1e4 kg/m^2 minimum-mass-nebula-grade column, with no Sigma_c input: the scale is a VIEW of
        // the accretion rate, the viscosity, and the disk temperature.
        let sigma = mirror_visc(Fixed::ONE, Fixed::from_int(280)).expect("derives");
        let v = sigma.to_f64_lossy();
        assert!(
            (1.0e3..=1.0e4).contains(&v),
            "an MMSN-grade viscous disk carries an order 1e3 to 1e4 kg/m^2 gas column at 1 AU, got {v}"
        );
    }

    #[test]
    fn the_viscous_similarity_slope_derives_gamma_near_one() {
        // Sigma ~ Omega/T ~ r^(-3/2)/r^(-1/2) = r^(-1): where the disk temperature follows the irradiated
        // T ~ r^(-1/2) (T halves when the orbit quadruples: 280 K at 1 AU, 140 K at 4 AU), the surface density
        // should fall as r^(-1), a quarter over a 4x orbit, so the slope gamma ~ 1 DERIVES from the viscous physics
        // rather than being an authored residue.
        let inner = mirror_visc(Fixed::ONE, Fixed::from_int(280)).unwrap();
        let outer = mirror_visc(Fixed::from_int(4), Fixed::from_int(140)).unwrap();
        let ratio = inner.to_f64_lossy() / outer.to_f64_lossy();
        assert!(
            (ratio - 4.0).abs() < 0.05,
            "Sigma falls as r^(-1) under an irradiated T(r): the 1-AU to 4-AU ratio is ~4 (gamma ~ 1), got {ratio}"
        );
    }

    #[test]
    fn the_viscous_gas_column_rises_with_the_accretion_rate() {
        // The steady-state column is linear in the mass-flux (Sigma ~ Mdot at fixed T): doubling the accretion rate
        // doubles the gas surface density, so a denser disk is a higher-Mdot realization, not a bigger Sigma_c knob.
        let base = mirror_visc(Fixed::ONE, Fixed::from_int(280)).unwrap();
        let fed = viscous_similarity_surface_density(
            Fixed::ONE,
            Fixed::ONE,
            Fixed::from_ratio(2, 100), // 2x the accretion rate
            Fixed::from_int(280),
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(234, 100),
        )
        .unwrap();
        let ratio = fed.to_f64_lossy() / base.to_f64_lossy();
        assert!(
            (ratio - 2.0).abs() < 0.02,
            "doubling the accretion rate doubles the steady-state gas column (Sigma ~ Mdot), got {ratio}"
        );
    }

    #[test]
    fn the_viscous_similarity_fails_loud_on_bad_inputs() {
        // Each non-positive input routes to None rather than a wrapped or saturated density (the fail-loud units bound).
        let (o, m, mdot, t, a, mu) = (
            Fixed::ONE,
            Fixed::ONE,
            Fixed::from_ratio(1, 100),
            Fixed::from_int(280),
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(234, 100),
        );
        assert!(viscous_similarity_surface_density(Fixed::ZERO, m, mdot, t, a, mu).is_none());
        assert!(viscous_similarity_surface_density(o, Fixed::ZERO, mdot, t, a, mu).is_none());
        assert!(viscous_similarity_surface_density(o, m, Fixed::ZERO, t, a, mu).is_none());
        assert!(viscous_similarity_surface_density(o, m, mdot, Fixed::ZERO, a, mu).is_none());
        assert!(viscous_similarity_surface_density(o, m, mdot, t, Fixed::ZERO, mu).is_none());
        assert!(viscous_similarity_surface_density(o, m, mdot, t, a, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_accretion_clock_starts_at_mdot_0_and_declines() {
        // At t = 0 the rate is Mdot_0 BIT-EXACTLY (the zero-age special case), then it falls monotonically. Test
        // fixtures, not authored physics: the math is what is checked.
        let mdot_0 = Fixed::ONE;
        let t_visc = Fixed::ONE;
        let gamma = Fixed::ONE; // p = 3/2
        let at_zero =
            viscous_similarity_accretion_rate(mdot_0, t_visc, gamma, Fixed::ZERO).unwrap();
        assert_eq!(
            at_zero, mdot_0,
            "Mdot(0) is Mdot_0 exactly by construction, not within a round-trip tolerance"
        );
        let early = viscous_similarity_accretion_rate(mdot_0, t_visc, gamma, Fixed::ONE).unwrap();
        let late =
            viscous_similarity_accretion_rate(mdot_0, t_visc, gamma, Fixed::from_int(4)).unwrap();
        assert!(
            early < at_zero && late < early,
            "the rate declines monotonically (0: {}, 1: {}, 4: {})",
            at_zero.to_f64_lossy(),
            early.to_f64_lossy(),
            late.to_f64_lossy()
        );
    }

    #[test]
    fn the_accretion_clock_decline_matches_the_viscous_exponent() {
        // At gamma = 1 the exponent p = 3/2, so at one viscous time (base = 2) the rate is Mdot_0 / 2^(3/2).
        let mdot_0 = Fixed::from_int(4);
        let t_visc = Fixed::from_int(2);
        let gamma = Fixed::ONE;
        let at_t_visc = viscous_similarity_accretion_rate(mdot_0, t_visc, gamma, t_visc).unwrap();
        // The expected value is computed OUTSIDE the engine, by the f64 standard library (`2.0_f64.powf(1.5)`),
        // NOT by the fixed-point `exp` under test, so this is an external oracle rather than a self-comparison.
        let expected = 4.0 / 2.0_f64.powf(1.5);
        // DEFAULTS-TAKEN, the 1e-3 relative tolerance: a numerical-accuracy bound on the fixed-point ln/exp
        // round-trip against the f64 oracle, not a residue budget and not a physical band. Basis: the Q32.32
        // transcendentals hold roughly six to seven significant digits, so a thousandth is loose headroom over
        // their round-trip error at this magnitude.
        assert!(
            (at_t_visc.to_f64_lossy() - expected).abs() / expected < 1e-3,
            "Mdot(t_visc) = Mdot_0 / 2^1.5 (expected {expected}, got {})",
            at_t_visc.to_f64_lossy()
        );
    }

    #[test]
    fn the_hindcast_passes_external_landmarks_and_convicts_a_mutated_clock() {
        // Mutation testing (standing rule) with twin-independence. The landmarks are pinned from OUTSIDE the
        // engine by the analytic form, NOT sampled from the clock under test, so the pass case is not a
        // self-comparison: at gamma = 1 (p = 3/2), base = 4 gives rate 1/4^(3/2) = 1/8 and base = 9 gives
        // 1/9^(3/2) = 1/27, both exact rationals computed by hand. Then the clock is MUTATED (a wrong gamma) and
        // the gate must convict, which is what proves the gate tests the clock rather than agreeing with it. A
        // test never shown to fail has not been shown to test anything.
        let mdot_0 = Fixed::ONE;
        let t_visc = Fixed::ONE;
        let gamma = Fixed::ONE; // p = 3/2
        let band = Fixed::from_ratio(1, 100); // 1 percent: over the ln/exp round-trip, under any real mutation
        let external = [
            AccretionLandmark {
                epoch_myr: Fixed::from_int(3), // base = 4, rate = 1/4^(3/2) = 1/8
                rate_msun_myr: Fixed::from_ratio(1, 8),
                band_frac: band,
            },
            AccretionLandmark {
                epoch_myr: Fixed::from_int(8), // base = 9, rate = 1/9^(3/2) = 1/27
                rate_msun_myr: Fixed::from_ratio(1, 27),
                band_frac: band,
            },
        ];
        assert_eq!(
            accretion_clock_hindcasts(mdot_0, t_visc, gamma, &external),
            Some(true),
            "the true clock passes the external analytic landmarks within band"
        );
        // MUTATION: a wrong gamma of 3/2 gives p = 2, not 3/2, so at base = 4 the mutant produces 1/4^2 = 1/16,
        // half the 1/8 landmark and far outside the 1 percent band. The gate must convict the mutant.
        let mutant_gamma = Fixed::from_ratio(3, 2);
        assert_eq!(
            accretion_clock_hindcasts(mdot_0, t_visc, mutant_gamma, &external),
            Some(false),
            "a mutated clock (wrong gamma) is convicted against the external landmarks"
        );
    }

    #[test]
    fn the_accretion_clock_fails_loud_on_bad_inputs() {
        let (m, t, g, a) = (Fixed::ONE, Fixed::ONE, Fixed::ONE, Fixed::ONE);
        assert!(viscous_similarity_accretion_rate(Fixed::ZERO, t, g, a).is_none());
        assert!(viscous_similarity_accretion_rate(m, Fixed::ZERO, g, a).is_none());
        assert!(viscous_similarity_accretion_rate(m, t, Fixed::from_int(2), a).is_none());
        assert!(viscous_similarity_accretion_rate(m, t, g, Fixed::from_int(-1)).is_none());
    }

    #[test]
    fn the_viscous_time_matches_the_analytic_scale_time() {
        // External f64 oracle (twin-independence): t_visc = sqrt(R_1)*sqrt(G*M)*mu*m_H/(3*alpha*k_B*T), computed
        // OUTSIDE the fixed-point engine with the same constants the function reads. A solar disk (R_1 = 30 AU,
        // M = 1 M_sun) at 50 K with alpha = 0.01, mu = 2.34 gives ~0.145 Myr, a sub-Myr scale time consistent
        // with the observed class-II band.
        let au = 149597870700.0_f64;
        let m_sun = 1.989e30_f64;
        let g = 6.67430e-11_f64;
        let k_b = 1.380649e-23_f64;
        let m_h = 1e-3_f64 / 6.02214076e23_f64;
        let year = 31557600.0_f64;
        let r1 = 30.0 * au;
        let t_s = r1.sqrt() * (g * m_sun).sqrt() * 2.34 * m_h / (3.0 * 0.01 * k_b * 50.0);
        let expected_myr = t_s / (1e6 * year);
        let derived = derive_viscous_time_myr(
            Fixed::from_int(30),
            Fixed::ONE,
            Fixed::from_int(50),
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(234, 100),
        )
        .unwrap();
        // DEFAULTS-TAKEN, the 2 percent tolerance: a numerical-accuracy bound on the eight-term fixed-point log
        // chain against the f64 oracle, not a residue budget. Basis: roughly a thousandth per ln/exp times the
        // chain length, loose headroom.
        assert!(
            (derived.to_f64_lossy() - expected_myr).abs() / expected_myr < 0.02,
            "t_visc matches the analytic scale time (expected {expected_myr}, got {})",
            derived.to_f64_lossy()
        );
    }

    #[test]
    fn the_viscous_time_scales_inversely_with_alpha() {
        // t_visc is proportional to 1/alpha by the analytic form, an independent check on the alpha dependence
        // (a wrong power on alpha would break the ratio). Doubling alpha halves t_visc.
        let base = derive_viscous_time_myr(
            Fixed::from_int(30),
            Fixed::ONE,
            Fixed::from_int(50),
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(234, 100),
        )
        .unwrap();
        let double_alpha = derive_viscous_time_myr(
            Fixed::from_int(30),
            Fixed::ONE,
            Fixed::from_int(50),
            Fixed::from_ratio(2, 100),
            Fixed::from_ratio(234, 100),
        )
        .unwrap();
        let ratio = base.checked_div(double_alpha).unwrap().to_f64_lossy();
        assert!(
            (ratio - 2.0).abs() < 0.01,
            "doubling alpha halves t_visc (ratio {ratio}, expected 2.0)"
        );
    }

    #[test]
    fn the_viscous_time_fails_loud_on_bad_inputs() {
        let (r, m, t, a, mu) = (
            Fixed::from_int(30),
            Fixed::ONE,
            Fixed::from_int(50),
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(234, 100),
        );
        assert!(derive_viscous_time_myr(Fixed::ZERO, m, t, a, mu).is_none());
        assert!(derive_viscous_time_myr(r, Fixed::ZERO, t, a, mu).is_none());
        assert!(derive_viscous_time_myr(r, m, Fixed::ZERO, a, mu).is_none());
        assert!(derive_viscous_time_myr(r, m, t, Fixed::ZERO, mu).is_none());
        assert!(derive_viscous_time_myr(r, m, t, a, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_disk_gas_mu_derives_the_solar_value_and_shifts_with_metallicity() {
        // TWIN-INDEPENDENCE: the disk-gas mean molecular weight mass-weighted over the SOLAR abundance pattern (H
        // as H2) reproduces the ~2.34 the authored fixture carried, computed from the abundance rows and the
        // periodic atomic weights, not read off any stored 2.34. It is the solar INSTANCE of a per-world
        // derivation, so 2.34 demotes rather than vanishing.
        let solar = civsim_physics::solar_abundances::SolarAbundances::standard()
            .expect("solar abundances");
        let periodic = civsim_physics::periodic::PeriodicTable::standard().expect("periodic table");
        let h2 = Fixed::from_int(2);
        let mu_solar = derive_disk_gas_mean_molecular_weight(&solar, &periodic, h2).unwrap();
        assert!(
            (mu_solar.to_f64_lossy() - 2.34).abs() < 0.1,
            "the solar disk-gas mu reproduces ~2.34 (got {})",
            mu_solar.to_f64_lossy()
        );
        // A metal-rich draw (+0.3 dex) carries a heavier gas, so a LARGER mu, the per-world variation the
        // graduation gives. The drawn ROWS shift (which is what this reads), even though the x/y/z getters do not.
        let metal_rich = solar.scaled_metals_by_dex(Fixed::from_ratio(3, 10));
        let mu_rich = derive_disk_gas_mean_molecular_weight(&metal_rich, &periodic, h2).unwrap();
        assert!(
            mu_rich > mu_solar,
            "a metal-rich world carries a heavier disk gas (rich {}, solar {})",
            mu_rich.to_f64_lossy(),
            mu_solar.to_f64_lossy()
        );
        // ADMIT THE ALIEN: an atomic-hydrogen disk (1 atom per molecule) has more particles per unit mass, so a
        // LOWER mu than the molecular value; and a non-positive molecule size is not a gas.
        let mu_atomic =
            derive_disk_gas_mean_molecular_weight(&solar, &periodic, Fixed::ONE).unwrap();
        assert!(
            mu_atomic < mu_solar,
            "atomic hydrogen lowers mu below the molecular value (atomic {}, H2 {})",
            mu_atomic.to_f64_lossy(),
            mu_solar.to_f64_lossy()
        );
        assert!(derive_disk_gas_mean_molecular_weight(&solar, &periodic, Fixed::ZERO).is_none());
    }

    // Eggleton 1983 Roche-lobe coefficients as a test fixture (cited at the function docs), the ~0.49/~0.6 fit.
    fn eggleton() -> (Fixed, Fixed) {
        (Fixed::from_ratio(49, 100), Fixed::from_ratio(6, 10))
    }

    #[test]
    fn the_roche_lobe_matches_the_eggleton_oracle() {
        // External f64 oracle (twin-independence): R_L/a = 0.49 q^(2/3) / (0.6 q^(2/3) + ln(1 + q^(1/3))), the
        // Eggleton Roche lobe, computed outside the engine. At separation 20 AU: q = 1 -> 7.579, q = 2 -> 8.800,
        // q = 0.5 -> 6.415 AU. A more massive host keeps a larger disk.
        let (c_num, c_log) = eggleton();
        let a = Fixed::from_int(20);
        let cases = [
            (Fixed::ONE, 7.5788f64),
            (Fixed::from_int(2), 8.8003),
            (Fixed::from_ratio(1, 2), 6.4153),
        ];
        for (q, oracle) in cases {
            let r = roche_lobe_radius_au(a, q, c_num, c_log).unwrap();
            assert!(
                (r.to_f64_lossy() - oracle).abs() / oracle < 0.01,
                "Eggleton R_L ~ {oracle}, got {}",
                r.to_f64_lossy()
            );
        }
    }

    #[test]
    fn the_roche_lobe_grows_with_separation_and_host_mass() {
        let (c_num, c_log) = eggleton();
        // Linear in separation.
        let near = roche_lobe_radius_au(Fixed::from_int(10), Fixed::ONE, c_num, c_log).unwrap();
        let far = roche_lobe_radius_au(Fixed::from_int(20), Fixed::ONE, c_num, c_log).unwrap();
        assert!(
            (far.to_f64_lossy() / near.to_f64_lossy() - 2.0).abs() < 0.001,
            "the cap scales linearly with separation"
        );
        // Monotone in host mass ratio: a heavier host is less truncated.
        let light =
            roche_lobe_radius_au(Fixed::from_int(20), Fixed::from_ratio(1, 2), c_num, c_log)
                .unwrap();
        let heavy =
            roche_lobe_radius_au(Fixed::from_int(20), Fixed::from_int(2), c_num, c_log).unwrap();
        assert!(heavy > light, "a more massive host keeps a larger disk");
    }

    #[test]
    fn the_truncation_fraction_tightens_the_disk_inside_the_roche_lobe() {
        // The Pichardo fit f = 0.733 (1-e)^1.20 q^0.07, q the companion mass fraction M_2/(M_1+M_2). At an equal-mass
        // circular binary (q=0.5) the disk truncates near 0.70 R_L; the 0.733 coefficient is the q->1 limit.
        let fit = DiskTruncationFit::pichardo_2005();
        let f_eq = resonant_truncation_fraction(Fixed::ZERO, Fixed::from_ratio(1, 2), &fit)
            .unwrap()
            .central()
            .unwrap();
        assert!(
            (f_eq.to_f64_lossy() - 0.699).abs() < 0.01,
            "an equal-mass circular binary truncates the disk near 0.70 R_L (got {})",
            f_eq.to_f64_lossy()
        );
        // Eccentricity tightens the disk sharply: at q=0.5, e=0.5 gives ~0.30, e=0.9 gives ~0.044.
        let f_half =
            resonant_truncation_fraction(Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2), &fit)
                .unwrap()
                .central()
                .unwrap();
        let f_high =
            resonant_truncation_fraction(Fixed::from_ratio(9, 10), Fixed::from_ratio(1, 2), &fit)
                .unwrap()
                .central()
                .unwrap();
        assert!(
            f_high < f_half && f_half < f_eq,
            "higher eccentricity tightens the disk"
        );
        assert!(
            (f_half.to_f64_lossy() - 0.304).abs() < 0.02,
            "e=0.5 truncates near 0.30 R_L at equal mass (got {})",
            f_half.to_f64_lossy()
        );
        // The fraction ships as a BAND at the fit's stated +/- 6.5 percent, not a point.
        let band =
            resonant_truncation_fraction(Fixed::ZERO, Fixed::from_ratio(1, 2), &fit).unwrap();
        assert!(
            (band.hi.to_f64_lossy() / band.lo.to_f64_lossy() - 1.065 / 0.935).abs() < 0.001,
            "the fraction band spans the +/- 6.5 percent Pichardo fit accuracy"
        );
        // The mass-fraction dependence is WEAK BUT REAL (q^0.07, ~17.5 percent per decade), not negligible: a tenfold
        // mass fraction (0.05 -> 0.5) raises f by ~17.5 percent, the exact 0.07-versus-0.01 exponent the audit fixed.
        let f_low_q = resonant_truncation_fraction(Fixed::ZERO, Fixed::from_ratio(5, 100), &fit)
            .unwrap()
            .central()
            .unwrap();
        let f_hi_q = resonant_truncation_fraction(Fixed::ZERO, Fixed::from_ratio(5, 10), &fit)
            .unwrap()
            .central()
            .unwrap();
        let decade_ratio = f_hi_q.to_f64_lossy() / f_low_q.to_f64_lossy();
        assert!(
            (decade_ratio - 1.175).abs() < 0.01,
            "a tenfold mass fraction moves f by ~17.5 percent (10^0.07), got {decade_ratio}"
        );
        // Fail-loud on the SOURCE domain (Pichardo e in [0, 0.90], q in [0.01, 0.99]), not an extrapolation: an
        // eccentricity above 0.90 (even below the unbound-orbit 1.0) and a mass fraction outside [0.01, 0.99] refuse.
        assert!(
            resonant_truncation_fraction(Fixed::from_ratio(95, 100), Fixed::from_ratio(1, 2), &fit)
                .is_none(),
            "e = 0.95 is above the fit's measured e <= 0.90, so it refuses rather than extrapolating"
        );
        assert!(resonant_truncation_fraction(Fixed::ONE, Fixed::from_ratio(1, 2), &fit).is_none());
        assert!(
            resonant_truncation_fraction(Fixed::ZERO, Fixed::from_ratio(5, 1000), &fit).is_none(),
            "q = 0.005 is below the fit's measured q >= 0.01"
        );
        assert!(resonant_truncation_fraction(Fixed::ZERO, Fixed::ONE, &fit).is_none());
    }

    #[test]
    fn the_truncation_fit_fingerprints_the_pichardo_coefficients() {
        // Source-fingerprint (the audit's requirement): the exact Pichardo 2005 Eq. 6 coefficients, verified against
        // the held scan p.524 (f = 0.733 (1-e)^1.20 q^0.07). This pins them so no future edit can silently swap in a
        // different model family's coefficient (the Manara / Papaloizou-Pringle viscous-torque fit's 0.01, say, a
        // distinct type with its own validity frame). If this fails, the fit has crossed a model family.
        let fit = DiskTruncationFit::pichardo_2005();
        assert_eq!(
            fit.circular_fraction,
            Fixed::from_ratio(733, 1000),
            "coefficient 0.733"
        );
        assert_eq!(
            fit.eccentricity_exponent,
            Fixed::from_ratio(120, 100),
            "eccentricity exponent 1.20"
        );
        assert_eq!(
            fit.mass_fraction_exponent,
            Fixed::from_ratio(7, 100),
            "mass-fraction exponent 0.07 (NOT the OCR-misread 0.01)"
        );
        // The fit's own validity and accuracy travel with it (Pichardo p.524): the +/- 6.5 percent band and the
        // measured e in [0, 0.90], q in [0.01, 0.99] box, so a consumer cannot extrapolate the fit silently.
        assert_eq!(
            fit.fit_error_fraction,
            Fixed::from_ratio(65, 1000),
            "+/- 6.5 percent"
        );
        assert_eq!(fit.valid_ecc_max, Fixed::from_ratio(90, 100), "e <= 0.90");
        assert_eq!(
            fit.valid_mass_fraction_min,
            Fixed::from_ratio(1, 100),
            "q >= 0.01"
        );
        assert_eq!(
            fit.valid_mass_fraction_max,
            Fixed::from_ratio(99, 100),
            "q <= 0.99"
        );
    }

    #[test]
    fn the_cap_bounds_a_large_disk_at_the_truncation_radius() {
        // min(birth, f * roche_lobe): a disk larger than its resonant truncation radius is bounded to it, a smaller
        // one untouched. At a circular orbit the Pichardo fraction is ~0.733, so the 7.6 AU lobe truncates at ~5.6 AU.
        let lobe = Fixed::from_ratio(76, 10); // 7.6 AU Roche-lobe radius
        let eval = pichardo_truncation_evaluation(
            Fixed::ZERO,
            Fixed::from_ratio(1, 2),
            lobe,
            &DiskTruncationFit::pichardo_2005(),
        )
        .unwrap();
        // The evaluation is typed: the circumprimary invariant-loop upper bound, a band, not a bare scalar.
        assert_eq!(eval.component, CircumstellarComponent::Primary);
        assert_eq!(eval.modality, TruncationModality::InvariantLoopUpperBound);
        // A large disk is bounded to the truncation band (both edges are the truncation radius, birth is larger).
        let capped_large = tidally_capped_scale_radius_au(Fixed::from_int(30), &eval).unwrap();
        assert_eq!(
            capped_large, eval.radius_au,
            "a 30 AU disk in a tight binary is bounded at the resonant truncation band f * R_L"
        );
        // A small disk inside its truncation radius is untouched on BOTH edges (birth is below either bound).
        let capped_small = tidally_capped_scale_radius_au(Fixed::from_int(3), &eval).unwrap();
        assert_eq!(capped_small.lo_au, Fixed::from_int(3));
        assert_eq!(capped_small.hi_au, Fixed::from_int(3));
        assert!(tidally_capped_scale_radius_au(Fixed::ZERO, &eval).is_none());
        // A non-physical fraction (f > 1, a truncation radius beyond the Roche lobe) is refused.
        let bad_eval = TruncationEvaluation {
            fraction: TruncationFractionBand {
                lo: Fixed::from_ratio(9, 10),
                hi: Fixed::from_ratio(11, 10),
            },
            radius_au: TruncationRadiusBand {
                lo_au: Fixed::from_int(7),
                hi_au: Fixed::from_int(9),
            },
            modality: TruncationModality::InvariantLoopUpperBound,
            component: CircumstellarComponent::Primary,
        };
        assert!(tidally_capped_scale_radius_au(Fixed::from_int(30), &bad_eval).is_none());
        let (c_num, c_log) = eggleton();
        assert!(roche_lobe_radius_au(Fixed::ZERO, Fixed::ONE, c_num, c_log).is_none());
        assert!(roche_lobe_radius_au(Fixed::from_int(20), Fixed::ZERO, c_num, c_log).is_none());
    }

    #[test]
    fn the_roche_lobe_cap_shortens_the_viscous_time_through_the_existing_machinery() {
        // The payoff: binarity shortens tau_disk with NO new path. Cap a 30 AU birth disk at a q = 1, 20 AU
        // binary's Roche lobe (~7.58 AU), feed the capped R_1 to derive_viscous_time_myr (all else equal), and
        // t_visc falls, because t_visc ~ sqrt(R_1). Holding T fixed isolates the radius effect; the real cooler
        // T(R_1) at the smaller radius shortens it further.
        let (c_num, c_log) = eggleton();
        let (m, t, alpha, mu) = (
            Fixed::ONE,
            Fixed::from_int(50),
            Fixed::from_ratio(1, 100),
            Fixed::from_ratio(234, 100),
        );
        let birth = Fixed::from_int(30);
        let lobe = roche_lobe_radius_au(Fixed::from_int(20), Fixed::ONE, c_num, c_log).unwrap();
        let eval = pichardo_truncation_evaluation(
            Fixed::ZERO,
            Fixed::from_ratio(1, 2),
            lobe,
            &DiskTruncationFit::pichardo_2005(),
        )
        .unwrap();
        let capped_band = tidally_capped_scale_radius_au(birth, &eval).unwrap();
        // Read the central of the capped band as the single radius the (still point-wise) viscous clock consumes.
        let capped = capped_band.central().unwrap();
        let r_t = eval.radius_au.central().unwrap();
        assert_eq!(
            capped, r_t,
            "the 30 AU disk is bounded at the resonant truncation radius f * R_L"
        );
        let t_birth = derive_viscous_time_myr(birth, m, t, alpha, mu).unwrap();
        let t_capped = derive_viscous_time_myr(capped, m, t, alpha, mu).unwrap();
        assert!(
            t_capped < t_birth,
            "the disk bounded at the Roche lobe runs a shorter (upper-bound) viscous time ({} < {})",
            t_capped.to_f64_lossy(),
            t_birth.to_f64_lossy()
        );
        // t_visc ~ sqrt(R_1), so the ratio tracks sqrt(R_trunc/birth).
        let ratio = t_capped.to_f64_lossy() / t_birth.to_f64_lossy();
        let expected = (r_t.to_f64_lossy() / birth.to_f64_lossy()).sqrt();
        assert!(
            (ratio - expected).abs() < 0.02,
            "the viscous-time ratio tracks sqrt(R_trunc/birth), got {ratio} vs {expected}"
        );
    }

    // Wright et al. 2011 convective-turnover fit as a test fixture (cited at the function docs), coefficients and
    // the 0.09 to 1.36 M_sun validity range.
    fn tau_poly() -> ConvectiveTurnoverFit {
        ConvectiveTurnoverFit {
            log_tau_c0: Fixed::from_ratio(116, 100),    // c0 = 1.16
            log_tau_c1: Fixed::from_ratio(-149, 100),   // c1 = -1.49
            log_tau_c2: Fixed::from_ratio(-54, 100),    // c2 = -0.54
            mass_min_msun: Fixed::from_ratio(9, 100),   // 0.09 M_sun
            mass_max_msun: Fixed::from_ratio(136, 100), // 1.36 M_sun
        }
    }

    #[test]
    fn the_convective_turnover_matches_the_polynomial_and_lengthens_for_lower_mass() {
        // External f64 oracle (twin-independence): 10^(1.16 + c1 log M + c2 log^2 M), computed outside the engine.
        // At the solar mass log M = 0 so tau = 10^1.16 ~ 14.45 days.
        let fit = tau_poly();
        let solar = convective_turnover_time_days(Fixed::ONE, &fit).unwrap();
        let expected_solar = 10f64.powf(1.16);
        // DEFAULTS-TAKEN, 2 percent: numerical-accuracy on the ln/exp round-trip, not a residue budget.
        assert!(
            (solar.to_f64_lossy() - expected_solar).abs() / expected_solar < 0.02,
            "solar turnover ~14.45 d (expected {expected_solar}, got {})",
            solar.to_f64_lossy()
        );
        // An M dwarf sits at a LONGER turnover, so it stays saturated longer (the convicting population).
        let m_dwarf = convective_turnover_time_days(Fixed::from_ratio(3, 10), &fit).unwrap();
        assert!(
            m_dwarf > solar,
            "the M dwarf turnover exceeds the solar one (M dwarf {}, solar {})",
            m_dwarf.to_f64_lossy(),
            solar.to_f64_lossy()
        );
    }

    #[test]
    fn the_xray_fraction_is_mass_universal_at_fixed_rossby() {
        // Admit-the-alien: the X-ray fraction depends ONLY on the Rossby number, so two stars of different mass
        // that reach the same Rossby show the same fractional activity. Build the same Ro two ways and compare.
        let fit = tau_poly();
        let ro_sat = Fixed::from_ratio(13, 100);
        let sat = Fixed::from_ratio(-313, 100);
        let beta = Fixed::from_ratio(-27, 10);
        let tau_g = convective_turnover_time_days(Fixed::ONE, &fit).unwrap();
        let tau_m = convective_turnover_time_days(Fixed::from_ratio(3, 10), &fit).unwrap();
        // Choose rotation periods so both give the same Rossby number Ro = 1.0 (P_rot = tau).
        let ro_g = stellar_rossby_number(tau_g, tau_g).unwrap();
        let ro_m = stellar_rossby_number(tau_m, tau_m).unwrap();
        let frac_g = activity_luminosity_fraction(ro_g, ro_sat, sat, beta).unwrap();
        let frac_m = activity_luminosity_fraction(ro_m, ro_sat, sat, beta).unwrap();
        assert_eq!(
            frac_g, frac_m,
            "same Rossby gives the same fractional activity regardless of mass"
        );
    }

    #[test]
    fn the_xray_fraction_saturates_declines_and_convicts_a_mutated_slope() {
        // The band mapping, mutation-tested. External f64 oracle: saturated 10^-3.13 below ro_sat; unsaturated
        // 10^-3.13 * (Ro/ro_sat)^beta above. Then MUTATE the slope to the canonical -2, which sits OUTSIDE the
        // declared source-internal dichotomy band [-2.70, -2.55] (Wright rejects it at 5 sigma), so the mutation
        // proves the mapping depends on the slope it claims rather than merely tracking a value inside the band.
        let ro_sat = Fixed::from_ratio(13, 100);
        let sat = Fixed::from_ratio(-313, 100);
        let beta = Fixed::from_ratio(-27, 10); // the unbiased sub-sample fit, which serves
                                               // Saturated regime: Ro below ro_sat returns the plateau exactly.
        let saturated =
            activity_luminosity_fraction(Fixed::from_ratio(5, 100), ro_sat, sat, beta).unwrap();
        let expected_plateau = 10f64.powf(-3.13);
        assert!(
            (saturated.to_f64_lossy() - expected_plateau).abs() / expected_plateau < 0.02,
            "saturated plateau 10^-3.13 (expected {expected_plateau}, got {})",
            saturated.to_f64_lossy()
        );
        // Unsaturated, at a solar-like Rossby of 1.757, against the f64 oracle.
        let ro = Fixed::from_ratio(1757, 1000);
        let ro_f = 1.757_f64;
        let expected = 10f64.powf(-3.13) * (ro_f / 0.13).powf(-2.70);
        let derived = activity_luminosity_fraction(ro, ro_sat, sat, beta).unwrap();
        assert!(
            (derived.to_f64_lossy() - expected).abs() / expected < 0.02,
            "unsaturated fraction matches the oracle (expected {expected:e}, got {:e})",
            derived.to_f64_lossy()
        );
        assert!(
            derived < saturated,
            "the unsaturated fraction is below the plateau"
        );
        // MUTATION: the canonical -2 slope, which Wright rejects. It lands far outside the 2 percent band.
        let mutant = activity_luminosity_fraction(ro, ro_sat, sat, Fixed::from_int(-2)).unwrap();
        assert!(
            (mutant.to_f64_lossy() - expected).abs() / expected > 0.5,
            "a mutated slope is convicted, off by more than half (mutant {:e}, true {expected:e})",
            mutant.to_f64_lossy()
        );
    }

    #[test]
    fn the_xray_functions_fail_loud_on_bad_inputs() {
        let fit = tau_poly();
        assert_eq!(
            convective_turnover_time_days(Fixed::ZERO, &fit),
            Err(TurnoverRefusal::InvalidInput)
        );
        assert!(stellar_rossby_number(Fixed::ZERO, Fixed::ONE).is_none());
        assert!(stellar_rossby_number(Fixed::ONE, Fixed::ZERO).is_none());
        let (ro_sat, sat, beta) = (
            Fixed::from_ratio(13, 100),
            Fixed::from_ratio(-313, 100),
            Fixed::from_ratio(-27, 10),
        );
        assert!(activity_luminosity_fraction(Fixed::ZERO, ro_sat, sat, beta).is_none());
        assert!(activity_luminosity_fraction(Fixed::ONE, Fixed::ZERO, sat, beta).is_none());
    }

    #[test]
    fn the_turnover_refuses_the_radiative_envelope_domain() {
        // Domain guard (catch 1), now a TYPED refusal per door (the gate seam). Beyond the high-mass edge the star
        // is radiative-enveloped with no rotation-activity dynamo: an A star (2 M_sun) returns the AboveFitDomain
        // door, the ONE dispatch seam, not a bare refusal a consumer could confuse with an invalid input.
        let fit = tau_poly();
        assert_eq!(
            convective_turnover_time_days(Fixed::from_int(2), &fit),
            Err(TurnoverRefusal::AboveFitDomain),
            "a 2 M_sun A star is the radiative-envelope dispatch seam"
        );
        // The low-mass edge is a DIFFERENT door (a sub-fit regime), never the radiative branch.
        assert_eq!(
            convective_turnover_time_days(Fixed::from_ratio(5, 100), &fit),
            Err(TurnoverRefusal::BelowFitDomain),
            "0.05 M_sun is below the fit, its own door"
        );
        // A non-positive mass is an invalid input, never a door.
        assert_eq!(
            convective_turnover_time_days(Fixed::from_int(-1), &fit),
            Err(TurnoverRefusal::InvalidInput)
        );
        // Inside the range still resolves.
        assert!(convective_turnover_time_days(Fixed::ONE, &fit).is_ok());
    }

    #[test]
    fn the_envelope_structure_keys_on_the_kraft_break_not_a_mass() {
        // The structure-keyed dispatch state: below the Kraft break a convective dynamo (the X-ray wind branch),
        // above it a radiative dynamo-dark photosphere (the EUV branch). The boundary is a TEMPERATURE, read
        // against the star's own T_eff, so no mass enters the dispatch.
        let kraft = Fixed::from_int(6200); // reserved Kraft-break T_eff, the surface-convection cutoff
        assert_eq!(
            stellar_envelope_structure(Fixed::from_int(4200), kraft), // a cool T Tauri / Hayashi-wall star
            Some(EnvelopeStructure::Convective),
            "a cool star hosts a convective dynamo"
        );
        assert_eq!(
            stellar_envelope_structure(Fixed::from_int(9000), kraft), // a hot Herbig / A star
            Some(EnvelopeStructure::Radiative),
            "a hot star is radiative, dynamo-dark"
        );
        // The break itself resolves to the convective side (the surface zone survives at the boundary).
        assert_eq!(
            stellar_envelope_structure(kraft, kraft),
            Some(EnvelopeStructure::Convective)
        );
        // A non-positive T_eff is not a star, and a non-positive break is not a boundary: errors, never branches.
        assert_eq!(stellar_envelope_structure(Fixed::from_int(-1), kraft), None);
        assert_eq!(
            stellar_envelope_structure(Fixed::from_int(5000), Fixed::ZERO),
            None
        );
    }

    fn kraft_band() -> KraftBreakBand {
        // The ratified Kraft band as reserved-with-basis fixtures: the classic 6200 K lower edge, the modern
        // 6550 +/- 200 K determination (a 6750 K upper edge), and the honest pre-fetch conditioning (sign known
        // positive, no magnitude), distinct from a measured-zero slope.
        KraftBreakBand {
            classic_edge_k: Fixed::from_int(6200),
            modern_center_k: Fixed::from_int(6550),
            modern_halfwidth_k: Fixed::from_int(200),
            conditioning: KraftMetallicityConditioning::SignOnly,
        }
    }

    #[test]
    fn the_kraft_band_carries_the_near_degenerate_zone_rather_than_asserting_a_side() {
        // The Gap-Law dispatch: three zones, not two. A cool star is certainly convective, a hot star certainly
        // radiative, and a star inside the classic-to-modern band is NEAR-DEGENERATE, a verdict the consumer
        // carries (evaluating both branches) rather than a side the dispatch picks. This is the whole point of the
        // band over the point cut: the few-hundred-K disagreement between the classic and modern breaks is a real
        // dispatch ambiguity, surfaced, not averaged away.
        let band = kraft_band();
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(4200), band, Fixed::ZERO), // a cool T Tauri star
            Some(KraftVerdict::Convective),
            "well below the band the dynamo branch is certain"
        );
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(9000), band, Fixed::ZERO), // a hot A star
            Some(KraftVerdict::Radiative),
            "well above the band the EUV branch is certain"
        );
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(6400), band, Fixed::ZERO), // between 6200 and 6750
            Some(KraftVerdict::NearDegenerate),
            "inside the band the verdict is carried, not asserted"
        );
        // The boundaries themselves belong to the ambiguous band (the certain branches are the strict outside), so
        // both edges resolve NearDegenerate: a star exactly at a measured break is precisely where certainty fails.
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(6200), band, Fixed::ZERO),
            Some(KraftVerdict::NearDegenerate),
            "the lower edge is in-band"
        );
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(6750), band, Fixed::ZERO),
            Some(KraftVerdict::NearDegenerate),
            "the upper edge is in-band"
        );
    }

    #[test]
    fn the_kraft_band_shifts_with_metallicity() {
        // A STRUCTURE-DERIVED point slope moves the WHOLE band with composition (the ionization boundary depends on
        // the metal-line opacity). At +1 dex a 300 K/dex slope shifts the edges up to [6500, 7050], so a 6400 K star
        // that read NearDegenerate at solar sits BELOW the shifted lower edge and reads Convective. The sign is
        // fixed positive by the type; the magnitude here is a probe, not a fetched value.
        let mut band = kraft_band();
        band.conditioning =
            KraftMetallicityConditioning::structure_derived(Fixed::from_int(300)).unwrap();
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(6400), band, Fixed::ONE),
            Some(KraftVerdict::Convective),
            "a +1 dex shift lifts the band above a 6400 K star"
        );
        // At solar composition the same band leaves the star NearDegenerate: the shift, not the star, moved it.
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(6400), band, Fixed::ZERO),
            Some(KraftVerdict::NearDegenerate),
            "with no offset the band is unmoved"
        );
        // The effective edges track the point shift exactly.
        assert_eq!(band.lower_edge_k(Fixed::ONE), Some(Fixed::from_int(6500)));
        assert_eq!(band.upper_edge_k(Fixed::ONE), Some(Fixed::from_int(7050)));
        // The SIGN is fixed positive by the type: a negative slope is unrepresentable, not a runtime check.
        assert!(KraftMetallicityConditioning::structure_derived(Fixed::from_int(-1)).is_none());
        // The default conditioning (SignOnly) applies no magnitude, so the band does not move with metallicity, and
        // that is DISTINCT from a measured-zero slope: the sign is carried even though the edges stay put.
        let default_band = kraft_band();
        assert_eq!(
            default_band.lower_edge_k(Fixed::ONE),
            Some(Fixed::from_int(6200))
        );
        assert_eq!(
            default_band.conditioning,
            KraftMetallicityConditioning::SignOnly
        );
    }

    #[test]
    fn the_kraft_banded_slope_widens_the_near_degenerate_zone() {
        // A BANDED slope (uncertain magnitude) widens the near-degenerate band rather than asserting a point shift:
        // at +1 dex a [100, 300] K/dex slope pushes the lower edge up by the SHALLOW 100 (to 6300) and the upper by
        // the STEEP 300 (to 7050), so the ambiguous zone grows with the slope uncertainty.
        let mut band = kraft_band();
        band.conditioning =
            KraftMetallicityConditioning::banded_slope(Fixed::from_int(100), Fixed::from_int(300))
                .unwrap();
        assert_eq!(band.lower_edge_k(Fixed::ONE), Some(Fixed::from_int(6300)));
        assert_eq!(band.upper_edge_k(Fixed::ONE), Some(Fixed::from_int(7050)));
        // The banded constructor enforces the ordering and the fixed positive sign.
        assert!(
            KraftMetallicityConditioning::banded_slope(Fixed::from_int(300), Fixed::from_int(100))
                .is_none(),
            "an out-of-order band is rejected"
        );
        assert!(
            KraftMetallicityConditioning::banded_slope(Fixed::from_int(-1), Fixed::from_int(100))
                .is_none(),
            "a negative slope edge is rejected"
        );
    }

    #[test]
    fn the_kraft_band_refuses_a_non_star_and_an_invalid_band() {
        // Fail-loud: a non-positive T_eff is not a star, and a band whose lower edge is driven above its upper edge
        // (here by a metallicity shift that inverts them) is not a boundary. Errors, never a silent branch.
        let band = kraft_band();
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(-1), band, Fixed::ZERO),
            None
        );
        assert_eq!(kraft_band_dispatch(Fixed::ZERO, band, Fixed::ZERO), None);
        // A shift large enough to drive an edge non-positive refuses (the band ceases to be physical).
        let mut sunk = kraft_band();
        sunk.conditioning =
            KraftMetallicityConditioning::structure_derived(Fixed::from_int(10_000)).unwrap();
        assert_eq!(sunk.lower_edge_k(Fixed::from_int(-1)), None); // 6200 - 10000 < 0
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(5000), sunk, Fixed::from_int(-1)),
            None,
            "a shift that sinks an edge below zero is not a band"
        );
        // A CROSSED band (the classic lower edge above the modern upper reach) is malformed base data, not a shift
        // artifact (the point shift moves both edges equally and cannot invert them): the lower-above-upper guard refuses.
        let crossed = KraftBreakBand {
            classic_edge_k: Fixed::from_int(9000),
            modern_center_k: Fixed::from_int(6550),
            modern_halfwidth_k: Fixed::from_int(200),
            conditioning: KraftMetallicityConditioning::SignOnly,
        };
        assert_eq!(
            kraft_band_dispatch(Fixed::from_int(7000), crossed, Fixed::ZERO),
            None,
            "a lower edge above the upper edge is not a band"
        );
    }

    #[test]
    fn the_mass_cut_is_the_main_sequence_instance_of_the_structural_line() {
        // The demotion the refinement requires: the turnover's high-mass refusal at mass_max (1.36 M_sun) and the
        // T_eff Kraft break are the SAME boundary seen two ways. A main-sequence star at the fit's high-mass edge
        // has a T_eff above the break, so the mass-keyed refusal and the T_eff-keyed dispatch AGREE on the main
        // sequence, while the Sun sits below it (convective, turnover defined). The mass cut is the structural
        // line's main-sequence shadow, not an independent number.
        let (alpha, beta) = (Fixed::from_ratio(35, 10), Fixed::from_ratio(8, 10));
        let t_max = Fixed::from_int(100_000);
        let kraft = Fixed::from_int(6200);
        let fit = tau_poly();
        // At the fit's high-mass edge the main-sequence T_eff is radiative: the mass cut agrees with the break.
        let t_edge = stellar_effective_temperature(fit.mass_max_msun, alpha, beta, t_max).unwrap();
        assert_eq!(
            stellar_envelope_structure(t_edge, kraft),
            Some(EnvelopeStructure::Radiative),
            "a main-sequence star at mass_max is radiative, matching AboveFitDomain (T_eff {})",
            t_edge.to_f64_lossy()
        );
        // The Sun is convective by both keys (below mass_max, below the break).
        let t_sun = stellar_effective_temperature(Fixed::ONE, alpha, beta, t_max).unwrap();
        assert_eq!(
            stellar_envelope_structure(t_sun, kraft),
            Some(EnvelopeStructure::Convective),
            "the Sun hosts a convective dynamo (T_eff {})",
            t_sun.to_f64_lossy()
        );
        assert!(convective_turnover_time_days(Fixed::ONE, &fit).is_ok());
        assert_eq!(
            convective_turnover_time_days(Fixed::from_ratio(14, 10), &fit),
            Err(TurnoverRefusal::AboveFitDomain)
        );
    }

    #[test]
    fn the_structural_line_generalizes_where_a_mass_cut_fails() {
        // The generalization the mass cut cannot reach: a 2 M_sun star is ABOVE the 1.36 mass cut, so a mass-keyed
        // dispatch calls it radiative at every age. But early on the Hayashi track it is COOL (its T_eff is the
        // Hayashi wall) and fully convective, X-ray active; it turns radiative only once it heats onto the Henyey
        // track. The structural line, keyed on the star's CURRENT T_eff, gets both epochs right where the mass cut
        // gets the young one wrong.
        let kraft = Fixed::from_int(6200);
        // Epoch one: young, on the Hayashi wall (~4300 K), fully convective, whatever its main-sequence mass.
        let t_hayashi_wall = Fixed::from_int(4300);
        assert_eq!(
            stellar_envelope_structure(t_hayashi_wall, kraft),
            Some(EnvelopeStructure::Convective),
            "a young intermediate-mass star on the Hayashi wall is convective"
        );
        // Epoch two: arrived on the main sequence, hot, radiative. The same star at its 2 M_sun main-sequence T_eff.
        let (alpha, beta) = (Fixed::from_ratio(35, 10), Fixed::from_ratio(8, 10));
        let t_ms = stellar_effective_temperature(
            Fixed::from_int(2),
            alpha,
            beta,
            Fixed::from_int(100_000),
        )
        .unwrap();
        assert_eq!(
            stellar_envelope_structure(t_ms, kraft),
            Some(EnvelopeStructure::Radiative),
            "the same star on the main sequence is hot and radiative (T_eff {})",
            t_ms.to_f64_lossy()
        );
        // The mass cut, by contrast, refuses the 2 M_sun star at every age, getting the young epoch wrong.
        let fit = tau_poly();
        assert_eq!(
            convective_turnover_time_days(Fixed::from_int(2), &fit),
            Err(TurnoverRefusal::AboveFitDomain)
        );
    }

    #[test]
    fn the_phase_is_the_crossing_of_the_two_luminosity_laws() {
        // The evolutionary phase falls out of the luminosity crossing, from the star's OWN derived luminosities. A
        // solar analogue: its zero-age main-sequence luminosity is 1 L_sun (1^exponent), and its pre-main-sequence
        // contraction luminosity falls as t^(-2/3) from a super-solar value. Young, it outshines the main sequence
        // (PreMainSequence); old, the contraction has fallen well below it (MainSequence). The same T_H is used for
        // both so the t^(-2/3) decline is the only difference.
        let l_ms = Fixed::ONE.powf(Fixed::from_ratio(35, 10)); // 1^exponent = 1 L_sun, the ZAMS Sun.
        let t_h = Fixed::from_int(4300); // the Hayashi wall.
        let l_pms_young = pre_main_sequence_luminosity_lsun(Fixed::ONE, t_h, Fixed::ONE).unwrap();
        let l_pms_old =
            pre_main_sequence_luminosity_lsun(Fixed::ONE, t_h, Fixed::from_int(200)).unwrap();
        assert!(
            l_pms_young > l_ms,
            "a 1 Myr pre-main-sequence Sun outshines the ZAMS ({} > {})",
            l_pms_young.to_f64_lossy(),
            l_ms.to_f64_lossy()
        );
        assert!(
            l_pms_old < l_ms,
            "a 200 Myr contraction has fallen below the ZAMS ({} < {})",
            l_pms_old.to_f64_lossy(),
            l_ms.to_f64_lossy()
        );
        assert_eq!(
            evolutionary_phase(l_pms_young, l_ms),
            Some(EvolutionaryPhase::PreMainSequence),
            "still contracting above the main sequence"
        );
        assert_eq!(
            evolutionary_phase(l_pms_old, l_ms),
            Some(EvolutionaryPhase::MainSequence),
            "contraction fallen below the main sequence: arrived"
        );
        // The boundary is inclusive of arrival, so L_bol is CONTINUOUS across the switch: exactly at the crossing
        // (the two laws equal) the phase reads MainSequence, and a max-of-the-two selection meets without a jump.
        assert_eq!(
            evolutionary_phase(l_ms, l_ms),
            Some(EvolutionaryPhase::MainSequence),
            "at the crossing the star has arrived, and the two laws agree there"
        );
        // Fail-loud on a non-star luminosity.
        assert_eq!(evolutionary_phase(Fixed::ZERO, l_ms), None);
        assert_eq!(evolutionary_phase(l_ms, Fixed::from_int(-1)), None);
    }

    #[test]
    fn the_structural_state_keys_both_the_wind_branch_and_the_lbol_track() {
        // The one state carries the two orthogonal axes, and the three canonical stars separate cleanly. The
        // envelope is the wind branch (Kraft on T_eff), the phase the L_bol track (the luminosity crossing).
        let band = kraft_band();
        // A Herbig Ae/Be star: hot photosphere (radiative envelope, the EUV branch) AND still contracting above its
        // luminous main sequence. Radiative and PreMainSequence at once, the case the two axes must keep distinct.
        let herbig = stellar_structural_state(
            Fixed::from_int(10_000),
            band,
            Fixed::ZERO,
            Fixed::from_int(50), // still contracting above ...
            Fixed::from_int(47), // ... its ZAMS luminosity.
        )
        .unwrap();
        assert_eq!(herbig.envelope, KraftVerdict::Radiative);
        assert_eq!(herbig.phase, EvolutionaryPhase::PreMainSequence);
        // A young solar analogue: cool (convective dynamo, the X-ray branch) and still contracting. Convective and
        // PreMainSequence, the same envelope as its arrived self but the other phase.
        let young_sun = stellar_structural_state(
            Fixed::from_int(5772),
            band,
            Fixed::ZERO,
            Fixed::from_ratio(167, 100), // 1.67 L_sun contraction ...
            Fixed::ONE,                  // ... above the 1 L_sun ZAMS.
        )
        .unwrap();
        assert_eq!(young_sun.envelope, KraftVerdict::Convective);
        assert_eq!(young_sun.phase, EvolutionaryPhase::PreMainSequence);
        // The arrived Sun: same convective envelope, contraction now below the ZAMS. Convective and MainSequence.
        let arrived_sun = stellar_structural_state(
            Fixed::from_int(5772),
            band,
            Fixed::ZERO,
            Fixed::from_ratio(5, 10), // contraction fallen to 0.5 L_sun ...
            Fixed::ONE,               // ... below the 1 L_sun ZAMS.
        )
        .unwrap();
        assert_eq!(arrived_sun.envelope, KraftVerdict::Convective);
        assert_eq!(arrived_sun.phase, EvolutionaryPhase::MainSequence);
        // Fail-loud: a non-star T_eff or a non-positive luminosity refuses the whole state, never a half-formed one.
        assert_eq!(
            stellar_structural_state(Fixed::ZERO, band, Fixed::ZERO, Fixed::ONE, Fixed::ONE),
            None
        );
        assert_eq!(
            stellar_structural_state(
                Fixed::from_int(5772),
                band,
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::ONE
            ),
            None
        );
    }

    // T_ion = E_edge/k_B for the 13.6 eV hydrogen Lyman edge, a DERIVED physical constant (~157821 K), the
    // test's stand-in for the value a live caller derives from the floor.
    fn t_ion() -> Fixed {
        Fixed::from_int(157821)
    }

    fn wien_x_min() -> Fixed {
        Fixed::from_int(3) // the Wien-tail validity edge (x >~ 3)
    }

    #[test]
    fn the_blackbody_ionizing_fraction_matches_the_wien_tail_oracle() {
        // External f64 oracle (twin-independence): f_BB = (15/pi^4) exp(-x)(x^3+3x^2+6x+6), x = T_ion/T_eff,
        // computed OUTSIDE the engine in python. The fixed-point log-space form must reproduce it. Values:
        // T_eff=10000 -> 1.0297e-4, 20000 -> 4.214e-2, 30000 -> 2.128e-1.
        let cases = [
            (10000i32, 1.029720e-4f64),
            (20000, 4.213767e-2),
            (30000, 2.127986e-1),
        ];
        for (t_eff, oracle) in cases {
            let f =
                blackbody_ionizing_fraction(Fixed::from_int(t_eff), t_ion(), wien_x_min()).unwrap();
            let got = f.to_f64_lossy();
            assert!(
                (got - oracle).abs() / oracle < 0.02,
                "f_BB(T_eff={t_eff}) ~ {oracle:e}, got {got:e}"
            );
        }
    }

    #[test]
    fn the_ionizing_fraction_rises_steeply_with_temperature() {
        // The convicting behaviour: the EUV tail climbs orders of magnitude with T_eff, so a hot Herbig B star
        // photoevaporates far harder than an A star. Monotone, and the 10000 -> 20000 K step alone spans more
        // than two dex.
        let cool =
            blackbody_ionizing_fraction(Fixed::from_int(10000), t_ion(), wien_x_min()).unwrap();
        let warm =
            blackbody_ionizing_fraction(Fixed::from_int(20000), t_ion(), wien_x_min()).unwrap();
        let hot =
            blackbody_ionizing_fraction(Fixed::from_int(30000), t_ion(), wien_x_min()).unwrap();
        assert!(warm > cool && hot > warm, "f_BB rises with T_eff");
        assert!(
            warm.to_f64_lossy() / cool.to_f64_lossy() > 100.0,
            "the 10000 -> 20000 K step spans more than two dex"
        );
    }

    #[test]
    fn the_blackbody_spectrum_is_same_spectrum_self_consistent() {
        // THE SAME-SPECTRUM IDENTITY (the audit's required test): all three quantities come from ONE blackbody
        // integral, so L_ion / Q_H = <E> exactly, i.e. log10(L_ion) - log10(Q_H) = log10(<E>). No NLTE energy is
        // ever crossed with an LTE mean energy because there is one branch here.
        let bb = blackbody_ionizing_spectrum(
            Fixed::from_int(15000),
            Fixed::from_int(100), // L_bol ~ 100 L_sun, a Herbig
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        assert_eq!(bb.branch, AtmosphereBranch::Blackbody);
        let l_ion = bb.ionizing_luminosity_log10_erg_s.unwrap().lo;
        let q_h = bb.photon_rate_log10_s.lo;
        let mean_e = bb.mean_photon_energy_log10_erg.unwrap().lo;
        assert!(
            (l_ion.checked_sub(q_h).unwrap().to_f64_lossy() - mean_e.to_f64_lossy()).abs() < 1e-6,
            "L_ion / Q_H = <E> holds within the one blackbody branch"
        );
        // The blackbody is a POINT spectrum: zero-width photon-rate band.
        assert_eq!(bb.photon_rate_log10_s.width_dex(), Some(Fixed::ZERO));
    }

    #[test]
    fn the_nlte_departure_applies_in_photon_space_with_its_width_stated() {
        // RIDER 2: the NLTE branch's photon rate is a BAND whose width is readable before a consumer reads it. A
        // photon-number departure of [0.01, 1] (two dex of suppression below the blackbody, the atmosphere-model
        // ensemble spread) makes the Q_H band two dex wide, applied IN PHOTON SPACE, and width_dex reports it.
        let bb = blackbody_ionizing_spectrum(
            Fixed::from_int(15000),
            Fixed::from_int(100),
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        let nlte = nlte_departed_ionizing_spectrum(
            &bb,
            Fixed::from_ratio(1, 100), // 0.01, deep suppression edge
            Fixed::ONE,                // 1.0, the blackbody edge
        )
        .unwrap();
        assert_eq!(nlte.branch, AtmosphereBranch::NlteLineBlanketed);
        // The suppressed edge is below the blackbody photon rate; the unsuppressed edge equals it.
        assert!(nlte.photon_rate_log10_s.lo < bb.photon_rate_log10_s.lo);
        assert_eq!(nlte.photon_rate_log10_s.hi, bb.photon_rate_log10_s.hi);
        let width = nlte.photon_rate_log10_s.width_dex().unwrap().to_f64_lossy();
        assert!(
            (width - 2.0).abs() < 0.01,
            "the photon-rate band is the departure's two dex, got {width}"
        );
        // The NLTE branch does not claim a self-consistent energy pair it did not reconstruct.
        assert!(nlte.ionizing_luminosity_log10_erg_s.is_none());
        assert!(nlte.mean_photon_energy_log10_erg.is_none());
    }

    #[test]
    fn the_blackbody_photon_rate_scales_with_luminosity() {
        // Doubling L_bol doubles Q_H (the ionizing luminosity is linear in L_bol, and the one division by the
        // same-T_eff mean energy is unchanged), so log10(Q_H) rises by log10(2).
        let one = blackbody_ionizing_spectrum(
            Fixed::from_int(20000),
            Fixed::from_int(50),
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        let two = blackbody_ionizing_spectrum(
            Fixed::from_int(20000),
            Fixed::from_int(100),
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        let ratio = 10.0_f64.powf(
            two.photon_rate_log10_s.lo.to_f64_lossy() - one.photon_rate_log10_s.lo.to_f64_lossy(),
        );
        assert!(
            (ratio - 2.0).abs() < 0.001,
            "twice the L_bol, twice the photon rate, got {ratio}"
        );
    }

    #[test]
    fn the_ionizing_spectrum_refuses_bad_inputs() {
        // A non-positive luminosity is an error, never a spectrum.
        assert!(blackbody_ionizing_spectrum(
            Fixed::from_int(15000),
            Fixed::ZERO,
            t_ion(),
            wien_x_min()
        )
        .is_none());
        assert!(blackbody_ionizing_fraction(Fixed::from_int(-1), t_ion(), wien_x_min()).is_none());
        // Above the Wien-tail validity T_eff (x < wien_x_min ~ 3, i.e. T_eff > ~52600 K): the full-Planck-
        // denominator regime, a refusal not a silent extrapolation (the second edge of a once-one-ended domain).
        assert!(
            blackbody_ionizing_fraction(Fixed::from_int(60000), t_ion(), wien_x_min()).is_none(),
            "a 60000 K photosphere is past the Wien-tail validity edge: refuse, do not extrapolate"
        );
        assert!(blackbody_ionizing_spectrum(
            Fixed::from_int(60000),
            Fixed::from_int(100),
            t_ion(),
            wien_x_min()
        )
        .is_none());
        // The audit's checked-arithmetic fix: a sub-122 K photosphere (x > ~1290) would overflow the polynomial
        // `x^3`; the checked multiply REFUSES with None (the total-kernel contract) rather than wrapping to garbage.
        assert!(
            blackbody_ionizing_fraction(Fixed::from_int(100), t_ion(), wien_x_min()).is_none(),
            "a 100 K photosphere overflows x^3: refuse (checked), never a wrapped value"
        );
        // The NLTE departure refuses an inverted band and a non-blackbody input.
        let bb = blackbody_ionizing_spectrum(
            Fixed::from_int(15000),
            Fixed::from_int(100),
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        assert!(
            nlte_departed_ionizing_spectrum(&bb, Fixed::ONE, Fixed::from_ratio(1, 100)).is_none(),
            "an inverted departure band (hi < lo) refuses"
        );
        let already_nlte =
            nlte_departed_ionizing_spectrum(&bb, Fixed::from_ratio(1, 10), Fixed::ONE).unwrap();
        assert!(
            nlte_departed_ionizing_spectrum(&already_nlte, Fixed::from_ratio(1, 10), Fixed::ONE)
                .is_none(),
            "the departure only applies to a blackbody evaluation, never a re-departed one"
        );
        // A degenerate departure [d, d] is a valid point band of zero width.
        let point =
            nlte_departed_ionizing_spectrum(&bb, Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2))
                .unwrap();
        assert_eq!(
            point.photon_rate_log10_s.width_dex(),
            Some(Fixed::ZERO),
            "a point departure has zero width"
        );
    }

    #[test]
    fn the_mean_ionizing_photon_energy_derives_from_the_wien_tail() {
        // <E>/E_edge = Gamma(4,x)/(x Gamma(3,x)), x = T_ion/T_eff. At a 15000 K photosphere (x ~ 10.5) the ratio is
        // about 1.11 (the mean ionizing photon carries about 15 eV), DERIVED from T_eff with no reserved value.
        let ratio =
            mean_ionizing_photon_energy_over_edge(Fixed::from_int(15000), t_ion(), wien_x_min())
                .unwrap()
                .to_f64_lossy();
        let x = 157821.0_f64 / 15000.0;
        let oracle = (x * x * x + 3.0 * x * x + 6.0 * x + 6.0) / (x * (x * x + 2.0 * x + 2.0));
        assert!(
            (ratio / oracle - 1.0).abs() < 0.001,
            "the derived ratio matches the Wien-tail oracle: got {ratio}, oracle {oracle}"
        );
        // A hotter photosphere hardens the tail: the mean energy rises above the cold-limit edge value, and the
        // mean is always above the edge itself (the ratio exceeds 1).
        let hot =
            mean_ionizing_photon_energy_over_edge(Fixed::from_int(30000), t_ion(), wien_x_min())
                .unwrap();
        let cool =
            mean_ionizing_photon_energy_over_edge(Fixed::from_int(10000), t_ion(), wien_x_min())
                .unwrap();
        assert!(
            hot > cool,
            "a hotter photosphere has a harder mean ionizing photon"
        );
        assert!(
            cool > Fixed::ONE,
            "the mean is always above the edge energy"
        );
        // Past the Wien-tail validity edge (T_eff > T_ion/wien_x_min ~ 52600 K) it refuses, not extrapolates.
        assert!(mean_ionizing_photon_energy_over_edge(
            Fixed::from_int(60000),
            t_ion(),
            wien_x_min()
        )
        .is_none());
    }

    #[test]
    fn the_euv_wind_rate_matches_the_hollenbach_normalization_oracle() {
        // END-TO-END oracle (the audit's requirement: begin at a grid state, not an arbitrary ionizing luminosity):
        // start from (T_eff, L_bol), build the blackbody spectrum (L_ion, Q_H, <E> from one integral), feed Q_H to
        // the wind law, and match an independent f64 chain L_bol -> f_BB -> L_ion -> Q_H = L_ion/<E> -> Mdot.
        let bb = blackbody_ionizing_spectrum(
            Fixed::from_int(15000),
            Fixed::from_int(100), // L_bol ~ 100 L_sun, a Herbig
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        let out = radiative_euv_photoevaporation_wind_rate_msun_myr(
            &bb,
            Fixed::ONE,
            &EuvWindFit::hollenbach_1994(),
        )
        .unwrap();
        // The f64 oracle, computed OUTSIDE the code, replicates the same one-branch chain.
        let l_sun_erg_s = 3.828e26_f64 * 1e7; // solar luminosity in erg/s
        let (t_ion_f, k_b_erg) = (157821.0_f64, 1.380649e-16_f64);
        let x = t_ion_f / 15000.0;
        let poly = x * x * x + 3.0 * x * x + 6.0 * x + 6.0;
        let f_bb = (15.0 / std::f64::consts::PI.powi(4)) * (-x).exp() * poly;
        let l_ion_erg_s = 100.0 * f_bb * l_sun_erg_s;
        let e_photon_erg = k_b_erg * t_ion_f * poly / (x * (x * x + 2.0 * x + 2.0));
        let phi = l_ion_erg_s / e_photon_erg; // Q_H
        let mdot_myr = 4.1e-10 * (phi / 1e41).powf(0.5) * 1.0_f64.powf(0.5) * 1e6;
        let got = out.rate.lo_msun_myr().to_f64_lossy();
        assert!(
            (got / mdot_myr - 1.0).abs() < 0.03,
            "the end-to-end EUV wind rate matches the Hollenbach oracle: got {got}, oracle {mdot_myr}"
        );
        // A blackbody (point) spectrum gives a point rate: zero-width bracket.
        assert_eq!(
            out.rate.width_dex(),
            Some(Fixed::ZERO),
            "a point spectrum has zero rate width"
        );
        // One solar mass sits BELOW Hollenbach's grounded 15-to-65 massive-star grid, so the rate is graded an
        // analytic extrapolation (estimator), not a grounded point: the disjoint-evidence fix in action.
        assert!(
            matches!(out.fit_reach, FitReach::AnalyticExtrapolation { .. }),
            "a solar mass is an extrapolation for the Hollenbach massive-star grid"
        );
    }

    #[test]
    fn the_euv_wind_rate_scales_as_the_square_root_of_the_photon_rate() {
        // Mdot ~ Q_H^(1/2), so a hundredfold photon-rate band (two dex, an NLTE departure applied in photon space)
        // gives a tenfold rate band, and the rate bracket's width is HALF the photon band's in dex (RIDER 2).
        let bb = blackbody_ionizing_spectrum(
            Fixed::from_int(15000),
            Fixed::from_int(100),
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        // A [0.01, 1] photon-number departure is a two-dex Q_H band.
        let nlte =
            nlte_departed_ionizing_spectrum(&bb, Fixed::from_ratio(1, 100), Fixed::ONE).unwrap();
        let out = radiative_euv_photoevaporation_wind_rate_msun_myr(
            &nlte,
            Fixed::ONE,
            &EuvWindFit::hollenbach_1994(),
        )
        .unwrap();
        let ratio = out.rate.hi_msun_myr().to_f64_lossy() / out.rate.lo_msun_myr().to_f64_lossy();
        assert!(
            (ratio - 10.0).abs() < 0.1,
            "a 100x photon-rate band gives a 10x (sqrt) rate band, got {ratio}"
        );
        let width = out.rate.width_dex().unwrap().to_f64_lossy();
        assert!(
            (width - 1.0).abs() < 0.01,
            "the rate bracket is 1 dex, half the photon band's 2 dex, got {width}"
        );
    }

    #[test]
    fn the_euv_wind_rate_orders_the_analytic_and_hydrodynamic_channels() {
        // The three cited channels form the model band: the Font hydrodynamic floor (1.52e-10) below the Hollenbach
        // analytic (4.1e-10) below the Alexander analytic ceiling (4.4e-10), at identical inputs. This is the band a
        // consumer forms, the analytic-versus-hydrodynamic sibling of the X-ray Owen-versus-Sellek band.
        let bb = blackbody_ionizing_spectrum(
            Fixed::from_int(15000),
            Fixed::from_int(100),
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        let rate = |fit: &EuvWindFit| {
            radiative_euv_photoevaporation_wind_rate_msun_myr(&bb, Fixed::ONE, fit)
                .unwrap()
                .rate
                .lo_msun_myr()
        };
        let font = rate(&EuvWindFit::font_2004_hydrodynamic());
        let hollenbach = rate(&EuvWindFit::hollenbach_1994());
        let alexander = rate(&EuvWindFit::alexander_2006());
        assert!(
            font < hollenbach && hollenbach < alexander,
            "the model band orders floor < analytic < ceiling: {} < {} < {}",
            font.to_f64_lossy(),
            hollenbach.to_f64_lossy(),
            alexander.to_f64_lossy()
        );
    }

    #[test]
    fn the_euv_wind_rate_guards_its_domain() {
        // Fail-loud: a mass outside the fit's whole reach refuses, and a photosphere past the Wien-tail validity edge
        // refuses at the spectrum, so no wind rate can be built from it.
        let bb = blackbody_ionizing_spectrum(
            Fixed::from_int(15000),
            Fixed::from_int(100),
            t_ion(),
            wien_x_min(),
        )
        .unwrap();
        let fit = EuvWindFit::hollenbach_1994();
        // Below the 0.1 M_sun extrapolation floor and above the 65 M_sun grounded ceiling: outside the whole reach.
        assert!(radiative_euv_photoevaporation_wind_rate_msun_myr(
            &bb,
            Fixed::from_ratio(5, 100),
            &fit
        )
        .is_none());
        assert!(
            radiative_euv_photoevaporation_wind_rate_msun_myr(&bb, Fixed::from_int(100), &fit)
                .is_none()
        );
        // A 60000 K photosphere is past the Wien-tail validity edge (x = T_ion/T_eff < wien_x_min ~ 3), so the
        // SPECTRUM refuses to form, and there is no laundered rate downstream: the shared domain door moved up front.
        assert!(blackbody_ionizing_spectrum(
            Fixed::from_int(60000),
            Fixed::from_int(100),
            t_ion(),
            wien_x_min()
        )
        .is_none());
    }

    #[test]
    fn the_euv_fit_domain_grades_each_channels_disjoint_support() {
        // The disjoint-evidence fix: each channel grades a mass by ITS OWN grounded support, so the Herbig gap is
        // never laundered into in-domain success. A single [mass_min, mass_max] pair could not tell these apart.
        let hollenbach = EuvWindFit::hollenbach_1994();
        let font = EuvWindFit::font_2004_hydrodynamic();
        // Hollenbach is grounded on the 15-to-65 massive-star grid; a 30 M_sun star is in-domain.
        assert_eq!(
            hollenbach.domain.reach_at(Fixed::from_int(30)),
            Some(FitReach::Grounded)
        );
        // The same fit at 1 M_sun is an analytic extrapolation (about 1.18 decades below the 15 M_sun edge).
        match hollenbach.domain.reach_at(Fixed::ONE) {
            Some(FitReach::AnalyticExtrapolation {
                decades_beyond_grounded,
            }) => assert!(
                (decades_beyond_grounded.to_f64_lossy() - 15.0_f64.log10()).abs() < 0.02,
                "the extrapolation distance is log10(15) decades below the grounded edge"
            ),
            other => panic!("a solar mass is a Hollenbach extrapolation, got {other:?}"),
        }
        // Font is grounded on the low-mass T Tauri regime; a solar-mass T Tauri star is in-domain for it, and the
        // SAME 8 M_sun Herbig-gap star is an extrapolation for Font but never grounded by any channel.
        assert_eq!(font.domain.reach_at(Fixed::ONE), Some(FitReach::Grounded));
        assert!(matches!(
            font.domain.reach_at(Fixed::from_int(8)),
            Some(FitReach::AnalyticExtrapolation { .. })
        ));
        assert!(matches!(
            hollenbach.domain.reach_at(Fixed::from_int(8)),
            Some(FitReach::AnalyticExtrapolation { .. })
        ));
        // Outside every interval is a refusal, not a silent extrapolation.
        assert_eq!(hollenbach.domain.reach_at(Fixed::from_int(80)), None);
        assert_eq!(font.domain.reach_at(Fixed::from_ratio(5, 100)), None);
    }

    #[test]
    fn the_euv_dispersal_phase_branches_on_the_inner_optical_depth() {
        // The load-bearing P1-3 fix: the diffuse/direct field is a typed branch on the disc's OWN inner optical
        // depth against the cited Alexander transition (tau = 4.61), not a prose aside. An optically THICK inner
        // disc (tau well above 4.61) is diffuse-driven; a DRAINED, optically thin inner disc (tau below 4.61) is
        // direct-driven, the materially stronger wind.
        let t = EuvPhaseTransition::alexander_2006();
        assert_eq!(
            t.transition_optical_depth,
            Fixed::from_ratio(461, 100),
            "tau = 4.61 (Eq. 15)"
        );
        assert_eq!(
            euv_dispersal_phase(Fixed::from_int(20), &t),
            Some(EuvDispersalPhase::Diffuse),
            "an optically thick inner disc is diffuse-driven"
        );
        assert_eq!(
            euv_dispersal_phase(Fixed::from_ratio(1, 100), &t),
            Some(EuvDispersalPhase::Direct),
            "a drained, optically thin inner disc is direct-driven"
        );
        // The transition itself sits in the thick (diffuse) side: exactly at tau it has not yet broken through.
        assert_eq!(
            euv_dispersal_phase(t.transition_optical_depth, &t),
            Some(EuvDispersalPhase::Diffuse)
        );
        // The 8.8x enhancement is a SCOPED reference datum (at 1 M_sun), carried for the direct branch's magnitude.
        assert_eq!(t.reference_enhancement_ratio, Fixed::from_ratio(88, 10));
        assert_eq!(t.reference_mass_ratio, Fixed::ONE);
        // Fail-loud on a non-positive optical depth.
        assert_eq!(euv_dispersal_phase(Fixed::ZERO, &t), None);
        assert_eq!(euv_dispersal_phase(Fixed::from_int(-1), &t), None);
    }

    #[test]
    fn the_band_ratio_evolves_with_rossby_the_welded_bands_cure() {
        // The payoff test: the SAME band mapping on the X-ray and EUV coefficient sets gives a ratio L_X/L_EUV
        // that EVOLVES with the Rossby number, because the slopes differ (X-ray -2.70 steeper than EUV -2.24). A
        // welded single-exponent design would have pinned this ratio constant forever; one state with two
        // measured mappings derives it. (The EUV coefficients are surfaced-pending-gate, so this proves the
        // MECHANISM, not the exact crossover.)
        let ro_sat = Fixed::from_ratio(13, 100);
        let xray_sat = Fixed::from_ratio(-313, 100);
        let xray_beta = Fixed::from_ratio(-27, 10);
        let euv_sat = Fixed::from_ratio(-401, 100); // log10(9.7e-5) ~ -4.01, France 2024
        let euv_beta = Fixed::from_ratio(-224, 100); // 2 * -1.12, inferred via Skumanich
        let ratio_at = |ro: Fixed| {
            let lx = activity_luminosity_fraction(ro, ro_sat, xray_sat, xray_beta).unwrap();
            let le = activity_luminosity_fraction(ro, ro_sat, euv_sat, euv_beta).unwrap();
            lx.checked_div(le).unwrap().to_f64_lossy()
        };
        let ratio_young = ratio_at(Fixed::from_ratio(3, 10)); // Ro = 0.3
        let ratio_old = ratio_at(Fixed::from_int(3)); // Ro = 3.0
        assert!(
            ratio_old < ratio_young * 0.9,
            "L_X/L_EUV evolves with Rossby (X-ray fades faster), not welded (young {ratio_young:e}, old {ratio_old:e})"
        );
    }

    #[test]
    fn the_formation_epoch_root_reproduces_the_condensation_front() {
        // A monotone stub midplane map (viscous scaling T = 2000 * rate^(1/4)), so temperature declines with age
        // as the clock's rate declines and crosses the ~1400 K front once. The found t_formation must, fed back
        // through the same clock and map, reproduce the condensation temperature: that is what makes it a root.
        let midplane =
            |rate: Fixed| Fixed::from_int(2000).checked_mul(rate.powf(Fixed::from_ratio(1, 4)));
        let cond = Fixed::from_int(1400);
        let t_form = derive_formation_epoch_myr(
            Fixed::ONE, // mdot_0
            Fixed::ONE, // t_visc
            Fixed::ONE, // decline gamma (p = 3/2)
            cond,
            midplane,
            Fixed::ZERO,
            Fixed::from_int(10),
            48,
        )
        .unwrap();
        assert!(
            t_form > Fixed::ZERO && t_form < Fixed::from_int(10),
            "the root lands inside the bracket (t_form {})",
            t_form.to_f64_lossy()
        );
        let rate_at_form =
            viscous_similarity_accretion_rate(Fixed::ONE, Fixed::ONE, Fixed::ONE, t_form).unwrap();
        let temp_at_form = midplane(rate_at_form).unwrap();
        // DEFAULTS-TAKEN, 1 K: the 48-iteration bisection converges far tighter than a kelvin over this bracket.
        assert!(
            (temp_at_form.to_f64_lossy() - 1400.0).abs() < 1.0,
            "T_mid(t_formation) reproduces the condensation front (got {})",
            temp_at_form.to_f64_lossy()
        );
    }

    #[test]
    fn the_formation_epoch_refuses_a_non_straddling_bracket() {
        // If the front is never reached in range (here the map is always hotter than a 100 K target across the
        // bracket), there is no crossing, so None rather than an extrapolated root.
        let midplane =
            |rate: Fixed| Fixed::from_int(2000).checked_mul(rate.powf(Fixed::from_ratio(1, 4)));
        assert!(
            derive_formation_epoch_myr(
                Fixed::ONE,
                Fixed::ONE,
                Fixed::ONE,
                Fixed::from_int(100),
                midplane,
                Fixed::ZERO,
                Fixed::from_int(10),
                48,
            )
            .is_none(),
            "a bracket that never crosses the front returns None"
        );
    }

    #[test]
    fn the_consistency_check_refuses_a_fitted_interim_so_the_trap_is_unconstructible() {
        // THE ANTI-CIRCULARITY GATE (assert the defect cannot be constructed). The exploit: pick Mdot_0 = 1 and
        // t_visc = 0.5 so Mdot(1 Myr) = 1 / (1 + 1/0.5)^(3/2) = 3^(-3/2) = 0.192, landing the 0.19 landmark
        // EXACTLY by construction. If the check ran on these it would report a meaningless "Consistent". Tagged
        // ChosenWithoutBasis, the provenance gate REFUSES (None): the fitted agreement cannot even be evaluated,
        // which is what makes interim-fitting unconstructible rather than merely discouraged.
        let fitted_mdot_0 = ProvenancedInterim {
            value: Fixed::ONE,
            basis: InterimBasis::ChosenWithoutBasis,
        };
        let fitted_t_visc = ProvenancedInterim {
            value: Fixed::from_ratio(1, 2),
            basis: InterimBasis::ChosenWithoutBasis,
        };
        let landmark = Fixed::from_ratio(19, 100); // 0.19
        assert!(
            formation_rate_consistency(
                fitted_mdot_0,
                fitted_t_visc,
                Fixed::ONE,
                Fixed::ONE, // t_formation = 1 Myr
                landmark,
                Fixed::from_ratio(5, 100), // 5 percent tolerance, which the fitted value would pass
            )
            .is_none(),
            "a chosen-without-basis interim refuses the check even when its value fits the landmark exactly"
        );
        // One independent, one fitted: still refuses, since the gate needs EVERY interim independent.
        let independent_mdot_0 = ProvenancedInterim {
            value: Fixed::ONE,
            basis: InterimBasis::CitedToPopulation,
        };
        assert!(
            formation_rate_consistency(
                independent_mdot_0,
                fitted_t_visc,
                Fixed::ONE,
                Fixed::ONE,
                landmark,
                Fixed::from_ratio(5, 100),
            )
            .is_none(),
            "one chosen-without-basis interim is enough to refuse"
        );
    }

    #[test]
    fn the_consistency_check_reports_a_verdict_on_independent_interims() {
        // With independent interims (draw-grade or cited-to-population), the check RUNS and reports a verdict. The
        // same values that were refused above (Mdot(1 Myr) = 0.192) now, cited to a population, land Consistent
        // against the 0.19 landmark within 5 percent; the point is that the VERDICT is earned by provenance, not
        // that these particular numbers pass. A far-off epoch reports Inconsistent, a Residual to surface.
        let mdot_0 = ProvenancedInterim {
            value: Fixed::ONE,
            basis: InterimBasis::CitedToPopulation, // e.g. the class-0/I birth-accretion band
        };
        let t_visc = ProvenancedInterim {
            value: Fixed::from_ratio(1, 2),
            basis: InterimBasis::DrawGrade, // e.g. derived from an R_1 disk-size demographic
        };
        let landmark = Fixed::from_ratio(19, 100);
        assert_eq!(
            formation_rate_consistency(
                mdot_0,
                t_visc,
                Fixed::ONE,
                Fixed::ONE, // Mdot(1 Myr) = 0.192, within 5 percent of 0.19
                landmark,
                Fixed::from_ratio(5, 100),
            ),
            Some(FormationRateConsistency::Consistent),
            "independent interims landing near the landmark report Consistent"
        );
        // A much later epoch has Mdot far below 0.19: Inconsistent, surfaced not tuned.
        assert_eq!(
            formation_rate_consistency(
                mdot_0,
                t_visc,
                Fixed::ONE,
                Fixed::from_int(50), // Mdot(50 Myr) is orders below 0.19
                landmark,
                Fixed::from_ratio(5, 100),
            ),
            Some(FormationRateConsistency::Inconsistent),
            "an epoch far from the landmark reports Inconsistent, a Residual"
        );
    }

    #[test]
    fn the_consistency_check_and_basis_grade_fail_loud() {
        // is_independent: only a draw or a cited population qualifies.
        assert!(InterimBasis::DrawGrade.is_independent());
        assert!(InterimBasis::CitedToPopulation.is_independent());
        assert!(!InterimBasis::ChosenWithoutBasis.is_independent());
        // Fail-loud on a non-positive landmark and a negative tolerance, even with independent interims.
        let ind = ProvenancedInterim {
            value: Fixed::ONE,
            basis: InterimBasis::DrawGrade,
        };
        assert!(formation_rate_consistency(
            ind,
            ind,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ZERO, // non-positive landmark
            Fixed::from_ratio(5, 100),
        )
        .is_none());
        assert!(formation_rate_consistency(
            ind,
            ind,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::from_ratio(19, 100),
            Fixed::from_int(-1), // negative tolerance
        )
        .is_none());
    }

    #[test]
    fn the_gravitational_radius_matches_the_solar_euv_wind_oracle() {
        // Twin-independent oracle, computed OUTSIDE the code under test: for the solar EUV-heated wind
        // (M_star = 1 M_sun, T_wind = 1e4 K, mu = 1), r_g = G M_star mu m_H / (k_B T_wind) works out to
        // ~10.673 AU in an f64 hand-computation. The log-domain derivation must land on the same value.
        let r_g = gravitational_radius_au(Fixed::ONE, Fixed::from_int(10_000), Fixed::ONE).unwrap();
        // DEFAULTS-TAKEN, 0.05 AU: the log/exp round trip holds the ~10.67 AU result well inside a hundredth
        // of the radius; the tolerance is the log-table resolution, not a physical margin.
        assert!(
            (r_g.to_f64_lossy() - 10.672_862).abs() < 0.05,
            "solar EUV wind r_g reproduces the oracle (got {})",
            r_g.to_f64_lossy()
        );
    }

    #[test]
    fn the_gravitational_radius_scales_inverse_temperature_and_linear_mass() {
        // Two independent scaling laws the closed form must obey, each checked against the base case rather
        // than against a second hand-number: r_g is inverse in T_wind (a ten-times-colder wind unbinds ten
        // times farther out) and linear in M_star (half the mass binds half as far).
        let base = gravitational_radius_au(Fixed::ONE, Fixed::from_int(10_000), Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        let colder = gravitational_radius_au(Fixed::ONE, Fixed::from_int(1_000), Fixed::ONE)
            .unwrap()
            .to_f64_lossy();
        let lighter =
            gravitational_radius_au(Fixed::from_ratio(1, 2), Fixed::from_int(10_000), Fixed::ONE)
                .unwrap()
                .to_f64_lossy();
        assert!(
            (colder - base * 10.0).abs() < 0.5,
            "a ten-times-colder wind gives a ten-times-larger r_g (base {}, colder {})",
            base,
            colder
        );
        assert!(
            (lighter - base / 2.0).abs() < 0.05,
            "half the stellar mass halves r_g (base {}, lighter {})",
            base,
            lighter
        );
    }

    #[test]
    fn the_gravitational_radius_refuses_nonphysical_inputs() {
        // Fail-loud on each non-positive axis rather than returning a plausible-looking radius: a zero or
        // negative mass, wind temperature, or molecular weight has no gravitational radius.
        assert!(
            gravitational_radius_au(Fixed::ZERO, Fixed::from_int(10_000), Fixed::ONE).is_none()
        );
        assert!(gravitational_radius_au(Fixed::ONE, Fixed::ZERO, Fixed::ONE).is_none());
        assert!(
            gravitational_radius_au(Fixed::ONE, Fixed::from_int(10_000), Fixed::ZERO).is_none()
        );
        assert!(
            gravitational_radius_au(Fixed::from_int(-1), Fixed::from_int(10_000), Fixed::ONE)
                .is_none()
        );
    }

    #[test]
    fn the_disk_lifetime_inverts_the_race_to_a_clean_oracle() {
        // Twin-independent oracle: at gamma = 1 (p = 3/2, so 1/p = 2/3), Mdot_0 = 8, Mdot_wind = 1, t_visc = 1,
        // the closed form t_visc*((Mdot_0/Mdot_wind)^(1/p) - 1) is 1*(8^(2/3) - 1) = 1*(4 - 1) = 3. A second
        // point (Mdot_0 = 27, t_visc = 2) gives 2*(27^(2/3) - 1) = 2*(9 - 1) = 16, both integers computed
        // outside the code under test.
        let tau = derive_disk_lifetime_myr(Fixed::from_int(8), Fixed::ONE, Fixed::ONE, Fixed::ONE)
            .unwrap();
        assert!(
            (tau.to_f64_lossy() - 3.0).abs() < 0.01,
            "the race tips at the analytic crossing (got {})",
            tau.to_f64_lossy()
        );
        let tau2 = derive_disk_lifetime_myr(
            Fixed::from_int(27),
            Fixed::from_int(2),
            Fixed::ONE,
            Fixed::ONE,
        )
        .unwrap();
        assert!(
            (tau2.to_f64_lossy() - 16.0).abs() < 0.02,
            "the second oracle point matches (got {})",
            tau2.to_f64_lossy()
        );
    }

    #[test]
    fn the_disk_lifetime_is_the_rate_crossing() {
        // The deeper invariant: tau_disk is the age at which the clock's own rate equals the wind rate. Feeding
        // the derived lifetime back through the accretion clock (an INDEPENDENT function) must reproduce the wind
        // rate, so the closed form and the clock agree on where the race tips.
        let mdot_0 = Fixed::from_int(8);
        let t_visc = Fixed::ONE;
        let gamma = Fixed::ONE;
        let wind = Fixed::ONE;
        let tau = derive_disk_lifetime_myr(mdot_0, t_visc, gamma, wind).unwrap();
        let rate_at_tau = viscous_similarity_accretion_rate(mdot_0, t_visc, gamma, tau).unwrap();
        assert!(
            (rate_at_tau.to_f64_lossy() - wind.to_f64_lossy()).abs() < 0.01,
            "Mdot(tau_disk) reproduces the wind rate (got {}, wind {})",
            rate_at_tau.to_f64_lossy(),
            wind.to_f64_lossy()
        );
    }

    #[test]
    fn the_disk_lifetime_is_zero_when_the_wind_beats_peak_accretion() {
        // A wind rate at or above the peak accretion rate opens the gap at (or before) birth: no viscous era, so
        // the lifetime is zero rather than a negative or None. Both the equal and the exceeding case.
        assert_eq!(
            derive_disk_lifetime_myr(
                Fixed::from_int(8),
                Fixed::ONE,
                Fixed::ONE,
                Fixed::from_int(8)
            ),
            Some(Fixed::ZERO)
        );
        assert_eq!(
            derive_disk_lifetime_myr(
                Fixed::from_int(8),
                Fixed::ONE,
                Fixed::ONE,
                Fixed::from_int(10)
            ),
            Some(Fixed::ZERO)
        );
    }

    #[test]
    fn the_disk_lifetime_refuses_nonphysical_inputs() {
        // Fail-loud on each non-positive axis and on a gamma outside [0, 2): no race, no derived lifetime.
        assert!(
            derive_disk_lifetime_myr(Fixed::ZERO, Fixed::ONE, Fixed::ONE, Fixed::ONE).is_none()
        );
        assert!(
            derive_disk_lifetime_myr(Fixed::from_int(8), Fixed::ZERO, Fixed::ONE, Fixed::ONE)
                .is_none()
        );
        assert!(
            derive_disk_lifetime_myr(Fixed::from_int(8), Fixed::ONE, Fixed::ONE, Fixed::ZERO)
                .is_none()
        );
        assert!(derive_disk_lifetime_myr(
            Fixed::from_int(8),
            Fixed::ONE,
            Fixed::from_int(2),
            Fixed::ONE
        )
        .is_none());
    }

    #[test]
    fn the_disk_lifetime_band_propagates_the_wind_uncertainty() {
        // The owner's cost ruling made executable: the declared wind-rate ENSEMBLE spans about an order of
        // magnitude (Owen appendix-B central, Owen eq. 9, Sellek 2024 the low edge), and that band propagates
        // through the (Mdot_0/Mdot_w)^(1/p) inversion. At gamma = 1 the exponent is 1/p = 2/3, so a factor-ten
        // band on the wind rate becomes a factor 10^(2/3) ~ 4.64 band on tau_disk, checked here so the cost is
        // proven rather than asserted. Two wind rates a decade apart at a large accretion-to-wind ratio (where
        // the -1 term is negligible) must give tau_disk values a factor ~4.64 apart. The Haisch-Lada / Mamajek
        // disk-fraction-versus-age data is the independent ensemble referee that discriminates within this band.
        let mdot_0 = Fixed::ONE;
        let t_visc = Fixed::ONE;
        let gamma = Fixed::ONE;
        let tau_strong_wind =
            derive_disk_lifetime_myr(mdot_0, t_visc, gamma, Fixed::from_ratio(1, 10_000))
                .unwrap()
                .to_f64_lossy();
        let tau_weak_wind =
            derive_disk_lifetime_myr(mdot_0, t_visc, gamma, Fixed::from_ratio(1, 100_000))
                .unwrap()
                .to_f64_lossy();
        let band = tau_weak_wind / tau_strong_wind;
        // Oracle 10^(2/3) = 4.6416 computed outside the code; the large ratio keeps the -1 shift under a percent.
        assert!(
            (band - 4.641_589).abs() < 0.05,
            "a decade wind band propagates to a ~4.64 tau_disk band at gamma=1 (got {})",
            band
        );
    }

    fn owen_appendix_b_fit() -> XrayWindFit {
        // The Owen, Clarke and Ercolano 2012 appendix-B population-synthesis fit, the cited central row: the test
        // helper delegates to the run-path constructor so there is one source of truth for the coefficients.
        XrayWindFit::owen_appendix_b()
    }

    #[test]
    fn the_shu_collapse_derives_the_birth_accretion_rate_from_the_core_temperature() {
        // The DERIVE-FIRST retirement of the disk clock's Mdot_0 interim: the Shu (1977) inside-out collapse rate
        // Mdot = m0*c_s^3/G derives the birth accretion rate from the cloud-core temperature and the gas mean
        // molecular weight, so Mdot_0 is a DERIVED quantity, not a reserved number. Oracle: a ~10 K solar-composition
        // core (mu ~ 2.33) at Shu's m0 = 0.975 gives ~1.5e-6 M_sun/yr = ~1.5 M_sun/Myr (vendored Shu 1977 Table 2,
        // cross-checked against Fiorellino et al. 2023's class-I band), matching the tagged ~1 M_sun/Myr interim in
        // order of magnitude.
        let shu = CollapseModel::shu_1977(); // m0 = 0.975 at A = 2, the vendored expansion-wave eigenvalue
        assert_eq!(shu.collapse_coefficient_m0, Fixed::from_ratio(975, 1000));
        assert_eq!(
            shu.instability_parameter_a,
            Fixed::from_int(2),
            "the coefficient declares its abscissa A (Shu 1977 Table 1)"
        );
        let mu_solar = Fixed::from_ratio(233, 100);
        let solar =
            shu_inside_out_collapse_accretion_rate_msun_myr(Fixed::from_int(10), mu_solar, &shu)
                .expect("the collapse rate resolves for a solar-composition 10 K core");
        // ISOTHERMAL ASSERTION: the vendored band is ~1.5 (mu=2.33) to ~1.9 (mu=2.0) M_sun/Myr at 10 K. An adiabatic
        // sound speed (gamma=5/3) would inflate the rate by gamma^(3/2) ~ 2.15 to ~3.3, OUTSIDE this band, so the
        // absolute magnitude asserts the isothermal c_s that a silent-gamma bug would break. MUTATION RECEIPT TAKEN
        // once (audit of f369bdf): injecting the adiabatic gamma gave 3.36 M_sun/Myr and this assert went RED, so it
        // tests the form, not just the magnitude. The band is a residue window with an analytic twin (the isothermal
        // 1.55 against the adiabatic 3.3), not an authored epsilon: it discriminates the two forms by construction.
        assert!(
            solar.to_f64_lossy() > 1.4 && solar.to_f64_lossy() < 1.7,
            "the 10 K solar-core birth rate is ~1.5 M_sun/Myr, isothermal not adiabatic (got {})",
            solar.to_f64_lossy()
        );
        // MECHANISM, Mdot ~ T^(3/2): a warmer core collapses faster. Doubling T lifts the rate by 2^1.5 ~ 2.83.
        let warm =
            shu_inside_out_collapse_accretion_rate_msun_myr(Fixed::from_int(20), mu_solar, &shu)
                .unwrap();
        let t_ratio = warm.checked_div(solar).unwrap().to_f64_lossy();
        assert!(
            (t_ratio - 2.828_427).abs() < 0.02,
            "Mdot scales as T^(3/2): 2x temperature lifts the rate by 2^1.5 (got {})",
            t_ratio
        );
        // MECHANISM, Mdot ~ mu^(-3/2): a lighter gas has a higher sound speed, a faster collapse, so mu 2.0 < 2.33
        // gives a higher rate.
        let light = shu_inside_out_collapse_accretion_rate_msun_myr(
            Fixed::from_int(10),
            Fixed::from_int(2),
            &shu,
        )
        .unwrap();
        assert!(
            light > solar,
            "a lighter gas (lower mu) collapses faster (light {}, solar {})",
            light.to_f64_lossy(),
            solar.to_f64_lossy()
        );
        // THE MODEL-STRUCTURE BAND (the declared endpoints): the Larson-Penston endpoint (m0 = 46.9 at A = 8.85) is
        // ~48x the Shu rate, the faster edge. Mdot is linear in m0, so the rate ratio is exactly the eigenvalue
        // ratio, and the two constructors are the measured continuum's endpoints (Whitworth-Summers 1985).
        let lp = CollapseModel::larson_penston();
        assert_eq!(lp.instability_parameter_a, Fixed::from_ratio(8854, 1000)); // A = P(0) = 8.854 (Hunter 1977)
        let rapid =
            shu_inside_out_collapse_accretion_rate_msun_myr(Fixed::from_int(10), mu_solar, &lp)
                .unwrap();
        let m0_ratio = rapid.checked_div(solar).unwrap().to_f64_lossy();
        let expected_ratio = 46.915 / 0.975; // ~48.1, the vendored endpoint separation (Hunter 1977)
        assert!(
            (m0_ratio - expected_ratio).abs() < 0.1,
            "Mdot is linear in m0: the LP endpoint is ~48x the Shu rate (got {}, expected {})",
            m0_ratio,
            expected_ratio
        );
        // A non-physical temperature, molecular weight, or collapse coefficient fails soft, never a fabricated rate.
        assert!(
            shu_inside_out_collapse_accretion_rate_msun_myr(Fixed::ZERO, mu_solar, &shu).is_none()
        );
        assert!(shu_inside_out_collapse_accretion_rate_msun_myr(
            Fixed::from_int(10),
            Fixed::ZERO,
            &shu
        )
        .is_none());
        let zero_m0 = CollapseModel {
            collapse_coefficient_m0: Fixed::ZERO,
            instability_parameter_a: Fixed::from_int(2),
        };
        assert!(shu_inside_out_collapse_accretion_rate_msun_myr(
            Fixed::from_int(10),
            mu_solar,
            &zero_m0
        )
        .is_none());
    }

    #[test]
    fn the_centrifugal_radius_derives_the_disk_birth_size_from_the_core_angular_momentum() {
        // The DERIVE-FIRST retirement of R_1 as an independent draw: the centrifugal radius R_c = j^2/(G M_star)
        // derives the disk's birth size from the collapsing core's specific angular momentum, so R_1 is DERIVED off
        // the core-angular-momentum root, not an independent axis (LAYER4_ROOT_CENSUS). Oracle: a core with
        // j ~ 3e16 m^2/s around a 1 M_sun star lands its material at ~45 AU, inside the Tazzari 2017 observed bulk
        // (25 to 100 AU, gas self-similar R_c), which is VALIDATION of the derived value, never the mechanism
        // authored to it.
        let ln_j = civsim_physics::saha::ln_of_decimal("3e16").unwrap(); // disk-scale core specific angular momentum
        let r_c = centrifugal_radius_au(ln_j, Fixed::ONE)
            .expect("the centrifugal radius resolves for a solar-mass star");
        assert!(
            r_c.to_f64_lossy() > 44.0 && r_c.to_f64_lossy() < 47.0,
            "the j ~ 3e16 birth radius is ~45 AU, inside the observed 25-100 AU bulk (got {})",
            r_c.to_f64_lossy()
        );
        // MECHANISM, R_c ~ j^2: doubling the specific angular momentum quadruples the landing radius. Formed by
        // adding ln 2 to ln j so the log-parameter idiom is exercised, not re-derived from a second string.
        let ln_j_double = ln_j.checked_add(Fixed::from_int(2).ln()).unwrap();
        let r_c_double = centrifugal_radius_au(ln_j_double, Fixed::ONE).unwrap();
        let j_ratio = r_c_double.checked_div(r_c).unwrap().to_f64_lossy();
        assert!(
            (j_ratio - 4.0).abs() < 0.02,
            "R_c scales as j^2: 2x angular momentum lifts the radius by 4x (got {})",
            j_ratio
        );
        // MECHANISM, R_c ~ 1/M_star: a heavier star holds the material closer, so doubling M_star halves the radius.
        let r_c_heavy = centrifugal_radius_au(ln_j, Fixed::from_int(2)).unwrap();
        let m_ratio = r_c_heavy.checked_div(r_c).unwrap().to_f64_lossy();
        assert!(
            (m_ratio - 0.5).abs() < 0.005,
            "R_c scales as 1/M_star: 2x stellar mass halves the radius (got {})",
            m_ratio
        );
        // A lower-j core (a slower rotator) births a smaller disk: the monotone direction the census relies on.
        let ln_j_slow = civsim_physics::saha::ln_of_decimal("1e16").unwrap();
        let r_c_slow = centrifugal_radius_au(ln_j_slow, Fixed::ONE).unwrap();
        assert!(
            r_c_slow < r_c,
            "a slower core (lower j) births a smaller disk (slow {}, base {})",
            r_c_slow.to_f64_lossy(),
            r_c.to_f64_lossy()
        );
        // A non-physical stellar mass fails soft, never a fabricated radius.
        assert!(centrifugal_radius_au(ln_j, Fixed::ZERO).is_none());
        assert!(centrifugal_radius_au(ln_j, Fixed::from_int(-1)).is_none());
    }

    #[test]
    fn the_spin_down_ages_stellar_rotation_along_the_gyrochronology_band() {
        // The DERIVE-FIRST retirement of Omega_star as an independent draw: the gyrochronological spin-down
        // P(t) = P_ref*(t/t_ref)^n ages a reference rotation forward, so the rotation at disk dispersal (and at any
        // later age) is DERIVED off the birth rotation and the braking law, not an independent axis
        // (LAYER4_ROOT_CENSUS). The exponent is a cited ensemble member, not one authored point.
        let sku = SpinDownModel::skumanich_1972();
        assert_eq!(
            sku.braking_exponent,
            Fixed::from_ratio(1, 2),
            "Skumanich n = 1/2 exactly"
        );
        assert_eq!(
            SpinDownModel::barnes_2007().braking_exponent,
            Fixed::from_ratio(5189, 10_000),
            "Barnes 2007 age exponent 0.5189"
        );
        let mh = SpinDownModel::mamajek_hillenbrand_2008();
        assert_eq!(
            mh.braking_exponent,
            Fixed::from_ratio(566, 1000),
            "Mamajek-Hillenbrand 0.566"
        );
        // MECHANISM, P ~ t^n: from a 5-day rotator at 100 Myr to 400 Myr (a 4x age ratio), Skumanich's n = 1/2
        // gives a factor (4)^(1/2) = 2, so the period doubles to 10 days.
        let onset = Fixed::from_int(100); // the ~100 Myr gyrochrone, the disk-locked / C-sequence exit
        let aged = spin_down_rotation_period_days(
            Fixed::from_int(5),
            onset,
            Fixed::from_int(400),
            onset,
            &sku,
        )
        .expect("the spin-down resolves at and past the onset");
        assert!(
            (aged.to_f64_lossy() - 10.0).abs() < 0.03,
            "P ~ t^(1/2): a 4x age ratio doubles the 5-day period to ~10 (got {})",
            aged.to_f64_lossy()
        );
        // THE MODEL BAND: a steeper exponent brakes the star more, so at the same age ratio Mamajek-Hillenbrand
        // (0.566) lengthens the period past Skumanich (0.5). The band is the measured recalibration spread.
        let aged_mh = spin_down_rotation_period_days(
            Fixed::from_int(5),
            onset,
            Fixed::from_int(400),
            onset,
            &mh,
        )
        .unwrap();
        assert!(
            aged_mh > aged,
            "a steeper braking exponent spins down more (MH {}, Skumanich {})",
            aged_mh.to_f64_lossy(),
            aged.to_f64_lossy()
        );
        // THE VALIDITY WINDOW: the braking law is invalid inside the disk-locked birth window, so an epoch below
        // the onset REFUSES rather than extrapolating. Both the reference and the target are guarded.
        assert!(
            spin_down_rotation_period_days(
                Fixed::from_int(5),
                Fixed::from_int(50),
                Fixed::from_int(400),
                onset,
                &sku
            )
            .is_none(),
            "a reference epoch inside the birth window is refused, not extrapolated"
        );
        assert!(
            spin_down_rotation_period_days(
                Fixed::from_int(5),
                onset,
                Fixed::from_int(50),
                onset,
                &sku
            )
            .is_none(),
            "a target epoch inside the birth window is refused"
        );
        // SOLAR HINDCAST, reported never gated (the replacement-circularity discipline): a ~5-day solar-mass
        // Pleiades rotator (~125 Myr) aged to the solar age (4566 Myr) lands in the tens-of-days slow-rotator range,
        // the order of the observed solar ~26 days, across the three-model band. REPORTED as a mechanism sanity,
        // never asserted to the observed value it would otherwise calibrate against.
        let solar_sku = spin_down_rotation_period_days(
            Fixed::from_int(5),
            Fixed::from_int(125),
            Fixed::from_int(4566),
            onset,
            &sku,
        )
        .unwrap();
        let solar_mh = spin_down_rotation_period_days(
            Fixed::from_int(5),
            Fixed::from_int(125),
            Fixed::from_int(4566),
            onset,
            &mh,
        )
        .unwrap();
        assert!(
            solar_sku.to_f64_lossy() > 20.0 && solar_mh.to_f64_lossy() < 45.0,
            "the solar-age hindcast lands in the slow-rotator tens-of-days band (Skumanich {}, MH {}, observed ~26)",
            solar_sku.to_f64_lossy(),
            solar_mh.to_f64_lossy()
        );
        // Non-physical inputs fail soft, never a fabricated period.
        assert!(spin_down_rotation_period_days(
            Fixed::ZERO,
            onset,
            Fixed::from_int(400),
            onset,
            &sku
        )
        .is_none());
        assert!(spin_down_rotation_period_days(
            Fixed::from_int(5),
            Fixed::ZERO,
            Fixed::from_int(400),
            onset,
            &sku
        )
        .is_none());
    }

    #[test]
    fn the_thermal_balance_derives_the_cloud_core_temperature_against_goldsmith_table_five() {
        // The DERIVE-FIRST retirement (route two) of the reserved cloud-core temperature: the Goldsmith 2001
        // thermal balance (cosmic-ray heating = molecular line cooling + gas-dust coupling) SOLVES T_core from the
        // core's own conditions, so the birth temperature the Shu collapse reads becomes derived, not drawn, and the
        // measured Jijina distribution demotes to a validation hindcast. Coefficients are the vendored Goldsmith
        // numbers (disk_arc_literature manifest).
        let m = GoldsmithThermalModel::goldsmith_2001();
        assert_eq!(m.cr_energy_log10_erg, Fixed::from_ratio(-104942, 10_000)); // log10(20 eV in erg)
        assert_eq!(m.gas_dust_coeff_log10, Fixed::from_ratio(-326990, 10_000)); // log10(2e-33), eq. 15
                                                                                // The heating input: zeta ~ 3.12e-17 s^-1 reproduces the paper's intermediate 1e-27*n heating coefficient
                                                                                // (zeta * dQ, dQ = 20 eV), so the solve is comparable to Table 5 (which uses that heating).
        let zeta = Fixed::from_ratio(3121, 1000); // in units of 1e-17 s^-1
        let cmb = Fixed::from_ratio(273, 100); // present-epoch CMB floor 2.73 K
        let t_hi = Fixed::from_int(50);
        // The line-cooling fits are the Goldsmith Table 2 (undepleted) power-law members for each density.
        let fit_1e3 = LineCoolingFit {
            log10_a: Fixed::from_ratio(-239586, 10_000), // log10(1.1e-24)
            b: Fixed::from_ratio(24, 10),                // 2.4
        };
        let fit_1e4 = LineCoolingFit {
            log10_a: Fixed::from_ratio(-232518, 10_000), // log10(5.6e-24)
            b: Fixed::from_ratio(27, 10),                // 2.7
        };
        let fit_1e6 = LineCoolingFit {
            log10_a: Fixed::from_ratio(-223098, 10_000), // log10(4.9e-23)
            b: Fixed::from_ratio(34, 10),                // 3.4
        };
        // VALIDATION (reported, never gated, the replacement-circularity discipline): a dark core at n=1e4 cm^-3,
        // undepleted, T_dust=6.53 K settles near Table 5's 11.4 K. Reported inside a band that brackets the paper,
        // never asserted to the measured value it would otherwise calibrate against.
        let t_1e4 = cloud_core_thermal_balance_temperature_k(
            zeta,
            Fixed::from_int(10_000),
            Fixed::from_ratio(653, 100),
            fit_1e4,
            cmb,
            &m,
            t_hi,
        )
        .expect("the thermal balance resolves for a dark core");
        assert!(
            t_1e4.to_f64_lossy() > 10.0 && t_1e4.to_f64_lossy() < 13.0,
            "the n=1e4 dark core settles ~11 K, the order of Goldsmith Table 5's 11.4 K (got {})",
            t_1e4.to_f64_lossy()
        );
        // MECHANISM, more cosmic-ray heating raises T_gas: doubling zeta lifts the equilibrium.
        let t_hot = cloud_core_thermal_balance_temperature_k(
            Fixed::from_int(6),
            Fixed::from_int(10_000),
            Fixed::from_ratio(653, 100),
            fit_1e4,
            cmb,
            &m,
            t_hi,
        )
        .unwrap();
        assert!(
            t_hot > t_1e4,
            "more cosmic-ray heating raises the gas temperature ({} vs {})",
            t_hot.to_f64_lossy(),
            t_1e4.to_f64_lossy()
        );
        // MECHANISM, the n^2 gas-dust coupling pins the gas to the dust at high density: at n=1e6 the gas-dust term
        // dominates so |T_gas - T_dust| is small, where at n=1e3 the line cooling sets T_gas well above T_dust.
        let td_hi_n = Fixed::from_ratio(763, 100); // T_dust = 7.63 K (Table 5, n=1e6)
        let t_1e6 = cloud_core_thermal_balance_temperature_k(
            zeta,
            Fixed::from_int(1_000_000),
            td_hi_n,
            fit_1e6,
            cmb,
            &m,
            t_hi,
        )
        .unwrap();
        let gap_1e6 = t_1e6.checked_sub(td_hi_n).unwrap().to_f64_lossy();
        assert!(
            gap_1e6 < 1.5,
            "at n=1e6 the gas-dust coupling pins T_gas near T_dust (gap {} K)",
            gap_1e6
        );
        let td_lo_n = Fixed::from_ratio(618, 100); // T_dust = 6.18 K (Table 5, n=1e3)
        let t_1e3 = cloud_core_thermal_balance_temperature_k(
            zeta,
            Fixed::from_int(1_000),
            td_lo_n,
            fit_1e3,
            cmb,
            &m,
            t_hi,
        )
        .unwrap();
        let gap_1e3 = t_1e3.checked_sub(td_lo_n).unwrap().to_f64_lossy();
        assert!(
            gap_1e3 > gap_1e6,
            "a low-density core's gas sits farther above its dust (gap n=1e3 {} > n=1e6 {})",
            gap_1e3,
            gap_1e6
        );
        // THE CMB FLOOR: a core cannot be colder than the microwave background. A high-z floor (15 K = 2.73*(1+z),
        // z~4.5) that exceeds the equilibrium clamps the result at the floor rather than returning below it.
        let t_floored = cloud_core_thermal_balance_temperature_k(
            zeta,
            Fixed::from_int(10_000),
            Fixed::from_ratio(653, 100),
            fit_1e4,
            Fixed::from_int(15),
            &m,
            t_hi,
        )
        .unwrap();
        assert!(
            (t_floored.to_f64_lossy() - 15.0).abs() < 0.5,
            "the derived temperature clamps at the CMB floor when the floor exceeds the equilibrium (got {})",
            t_floored.to_f64_lossy()
        );
        // Non-physical inputs fail soft, never a fabricated temperature.
        assert!(cloud_core_thermal_balance_temperature_k(
            Fixed::ZERO,
            Fixed::from_int(10_000),
            Fixed::from_ratio(653, 100),
            fit_1e4,
            cmb,
            &m,
            t_hi,
        )
        .is_none());
        assert!(cloud_core_thermal_balance_temperature_k(
            zeta,
            Fixed::from_int(10_000),
            Fixed::from_ratio(653, 100),
            fit_1e4,
            cmb,
            &m,
            cmb, // t_hi = floor, no bracket
        )
        .is_none());
    }

    #[test]
    fn the_coupled_solve_derives_the_dust_temperature_from_the_radiation_field() {
        // The seam closed: the dust temperature is no longer a supplied input but DERIVED with the gas temperature
        // from the radiation-field scaling chi, solving Goldsmith's gas and dust balances together. Validation
        // against Table 5 (chi = 1e-4): a n=1e4 dark core settles near T_gas = 11.4 K, T_dust = 6.53 K.
        let m = GoldsmithThermalModel::goldsmith_2001();
        assert_eq!(
            m.dust_cooling_coeff_log10,
            Fixed::from_ratio(-321675, 10_000)
        ); // log10(6.8e-33), eq. 13
        assert_eq!(
            m.dust_heating_coeff_log10,
            Fixed::from_ratio(-234089, 10_000)
        ); // log10(3.9e-24), eq. 7
        let zeta = Fixed::from_ratio(3121, 1000);
        let cmb = Fixed::from_ratio(273, 100);
        let t_hi = Fixed::from_int(50);
        let chi = Fixed::from_ratio(1, 10_000); // 1e-4, the dark-cloud flux-scaling of Table 5
        let fit_1e4 = LineCoolingFit {
            log10_a: Fixed::from_ratio(-232518, 10_000),
            b: Fixed::from_ratio(27, 10),
        };
        let t = cloud_core_coupled_temperatures(
            zeta,
            Fixed::from_int(10_000),
            chi,
            fit_1e4,
            cmb,
            &m,
            t_hi,
        )
        .expect("the coupled balance resolves for a dark core");
        // VALIDATION (reported, never gated): both temperatures land near Goldsmith Table 5 (11.4 K / 6.53 K).
        assert!(
            t.gas_temperature_k.to_f64_lossy() > 10.0 && t.gas_temperature_k.to_f64_lossy() < 13.0,
            "the derived gas temperature is ~11 K (got {})",
            t.gas_temperature_k.to_f64_lossy()
        );
        assert!(
            t.dust_temperature_k.to_f64_lossy() > 5.5 && t.dust_temperature_k.to_f64_lossy() < 7.5,
            "the derived dust temperature is ~6.5 K (got {})",
            t.dust_temperature_k.to_f64_lossy()
        );
        // The dust is cooler than the gas (the cosmic rays heat the gas, the gas-dust coupling warms the dust).
        assert!(
            t.dust_temperature_k < t.gas_temperature_k,
            "the dust settles cooler than the gas ({} vs {})",
            t.dust_temperature_k.to_f64_lossy(),
            t.gas_temperature_k.to_f64_lossy()
        );
        // MECHANISM, more radiation raises the dust: a tenfold chi lifts T_dust (Table 5's chi=1e-3 gives ~9 K).
        let t_bright = cloud_core_coupled_temperatures(
            zeta,
            Fixed::from_int(10_000),
            Fixed::from_ratio(1, 1_000), // chi = 1e-3
            fit_1e4,
            cmb,
            &m,
            t_hi,
        )
        .unwrap();
        assert!(
            t_bright.dust_temperature_k > t.dust_temperature_k,
            "a stronger radiation field raises the dust temperature ({} vs {})",
            t_bright.dust_temperature_k.to_f64_lossy(),
            t.dust_temperature_k.to_f64_lossy()
        );
        // MECHANISM, the gas-dust coupling warms the dust at high density: T_dust rises with n (Table 5 6.18 K at
        // n=1e3 to 7.63 K at n=1e6), where the uncoupled dust temperature would be density-independent.
        let fit_1e3 = LineCoolingFit {
            log10_a: Fixed::from_ratio(-239586, 10_000),
            b: Fixed::from_ratio(24, 10),
        };
        let fit_1e6 = LineCoolingFit {
            log10_a: Fixed::from_ratio(-223098, 10_000),
            b: Fixed::from_ratio(34, 10),
        };
        let t_lo_n = cloud_core_coupled_temperatures(
            zeta,
            Fixed::from_int(1_000),
            chi,
            fit_1e3,
            cmb,
            &m,
            t_hi,
        )
        .unwrap();
        let t_hi_n = cloud_core_coupled_temperatures(
            zeta,
            Fixed::from_int(1_000_000),
            chi,
            fit_1e6,
            cmb,
            &m,
            t_hi,
        )
        .unwrap();
        assert!(
            t_hi_n.dust_temperature_k > t_lo_n.dust_temperature_k,
            "the gas-dust coupling warms the dust at high density ({} at n=1e6 vs {} at n=1e3)",
            t_hi_n.dust_temperature_k.to_f64_lossy(),
            t_lo_n.dust_temperature_k.to_f64_lossy()
        );
        // Non-physical inputs fail soft.
        assert!(cloud_core_coupled_temperatures(
            zeta,
            Fixed::from_int(10_000),
            Fixed::ZERO,
            fit_1e4,
            cmb,
            &m,
            t_hi
        )
        .is_none());
    }

    #[test]
    fn the_radiation_field_chi_estimator_is_a_banded_log_domain_stand_in() {
        // The extinction-to-chi map is graded an ESTIMATOR, not a derivation, so this test exercises its own
        // contract (a band, monotone in A_V, live in the log domain at deep extinction) and does NOT validate a
        // single chi against the Goldsmith dark-core value: selecting k to reproduce that value and then checking
        // against it would be the circular target reuse the audit flagged.
        let est = ExtinctionChiEstimator::zucconi_2001();
        // The A_V axis derives from the core column exactly as before (that step IS a derivation, the Bohlin ratio).
        let log10_ratio = Fixed::from_ratio(2127, 100); // log10(1.87e21) ~ 21.27, the cited Bohlin ratio
        let log10_column = Fixed::from_ratio(2297, 100); // log10(9.4e22) ~ 22.97, a factor ~50 over the ratio
        let a_v = visual_extinction_magnitudes(log10_column, log10_ratio).unwrap();
        assert!(
            (a_v.to_f64_lossy() - 50.0).abs() < 1.5,
            "the column over the ratio is ~50 mag of extinction (got {})",
            a_v.to_f64_lossy()
        );
        // The estimate is a BAND: the steep-k edge attenuates deeper (lower chi) than the shallow-k edge.
        let band = radiation_field_chi_estimate_from_extinction(a_v, &est).unwrap();
        assert!(
            band.log10_chi_lo < band.log10_chi_hi,
            "the steep-k edge is the lower chi ({} < {})",
            band.log10_chi_lo.to_f64_lossy(),
            band.log10_chi_hi.to_f64_lossy()
        );
        // At A_V ~ 50 the band spans about 10^(-50*0.13) to 10^(-50*0.05), i.e. log10(chi) in about -6.5 to -2.5,
        // straddling the observed dark-core regime WITHOUT being pinned to any one target inside it.
        assert!(
            band.log10_chi_hi.to_f64_lossy() > -3.0 && band.log10_chi_hi.to_f64_lossy() < -2.0,
            "the shallow-k edge log10(chi) ~ -2.5 (got {})",
            band.log10_chi_hi.to_f64_lossy()
        );
        assert!(
            band.log10_chi_lo.to_f64_lossy() > -7.0 && band.log10_chi_lo.to_f64_lossy() < -6.0,
            "the steep-k edge log10(chi) ~ -6.5 (got {})",
            band.log10_chi_lo.to_f64_lossy()
        );
        // No extinction leaves the field unattenuated: log10(chi) = 0 on both edges, chi = 1.
        let bare = radiation_field_chi_estimate_from_extinction(Fixed::ZERO, &est).unwrap();
        assert_eq!(bare.log10_chi_lo, Fixed::ZERO);
        assert_eq!(bare.log10_chi_hi, Fixed::ZERO);
        assert!((bare.linear_hi().unwrap().to_f64_lossy() - 1.0).abs() < 1e-6);
        // Monotone: less extinction is a brighter field on the matched edge.
        let thin = radiation_field_chi_estimate_from_extinction(Fixed::from_int(5), &est).unwrap();
        assert!(
            thin.log10_chi_hi > band.log10_chi_hi,
            "less extinction is brighter"
        );
        // REPRESENTATION LIVENESS: a deep core past the linear fixed-point floor stays live in the LOG domain (the
        // band is a valid, very negative log10(chi)); only the LINEAR recovery refuses, which convicts the carrier
        // and not the core's existence. This is the alien-general behaviour the A_V=200 refusal test lacked.
        let deep =
            radiation_field_chi_estimate_from_extinction(Fixed::from_int(200), &est).unwrap();
        assert!(
            deep.log10_chi_lo.to_f64_lossy() < -20.0,
            "a deep core has a live, very negative log10(chi) (got {})",
            deep.log10_chi_lo.to_f64_lossy()
        );
        assert!(
            deep.linear_lo().is_none(),
            "the linear recovery is the carrier that refuses, not the estimator"
        );
        // Fail-loud on a negative extinction or an inverted k band.
        assert!(radiation_field_chi_estimate_from_extinction(Fixed::from_int(-1), &est).is_none());
        let inverted = ExtinctionChiEstimator {
            k_lo: Fixed::from_ratio(13, 100),
            k_hi: Fixed::from_ratio(5, 100),
        };
        assert!(
            radiation_field_chi_estimate_from_extinction(Fixed::from_int(10), &inverted).is_none()
        );
    }

    #[test]
    fn the_declared_wind_rows_carry_their_cited_coefficients_and_integration_domains() {
        // The three-row wind ensemble as cited run-path data (stage 1 of the slice-2 wire), plus the Sellek
        // controlled statistic. Two facts are asserted: the coefficients match the primaries digit for digit, and
        // the integration domains encode the owner's domain-matched-rows ruling (the 160 AU total serves the band;
        // the 80 AU chord does not).
        let owen_b = XrayWindFit::owen_appendix_b();
        let owen_9 = XrayWindFit::owen_equation_9();
        let sellek = XrayWindFit::sellek_2024();
        let sellek_80 = XrayWindFit::sellek_2024_controlled_80au();

        // Coefficients, stored as log10 of M_sun/yr, against the cited values.
        assert!((owen_b.log10_coefficient_msun_yr.to_f64_lossy() - (-8.204_12)).abs() < 1e-4); // 6.25e-9
        assert!((owen_9.log10_coefficient_msun_yr.to_f64_lossy() - (-8.096_91)).abs() < 1e-4); // 8e-9
        assert!((sellek.log10_coefficient_msun_yr.to_f64_lossy() - (-8.364_52)).abs() < 1e-4); // 4.32e-9, 160 AU
        assert!((sellek_80.log10_coefficient_msun_yr.to_f64_lossy() - (-8.974_69)).abs() < 1e-4); // 1.06e-9, 80 AU

        // Equation 9 is strictly linear in L_X and mass-independent, the shape that distinguishes it.
        assert_eq!(owen_9.l_x_exponent, Fixed::ONE);
        assert_eq!(owen_9.mass_exponent, Fixed::ZERO);
        // Sellek inherits the Owen appendix-B shape (it re-normalizes, it does not re-fit the exponents).
        assert_eq!(sellek.l_x_exponent, owen_b.l_x_exponent);
        assert_eq!(sellek.mass_exponent, owen_b.mass_exponent);

        // Band ordering: Sellek's whole-disk total is the LOW edge (below Owen's central and its eq-9 cross-check).
        assert!(sellek.log10_coefficient_msun_yr < owen_b.log10_coefficient_msun_yr);
        assert!(owen_b.log10_coefficient_msun_yr < owen_9.log10_coefficient_msun_yr);
        // A shorter chord is a lower integrated rate: the 80 AU statistic sits below the 160 AU total, the physical
        // reason the two are distinct quantities, not interchangeable.
        assert!(sellek_80.log10_coefficient_msun_yr < sellek.log10_coefficient_msun_yr);

        // The domain-matched-rows rule: the three whole-disk totals band together; the 80 AU chord does not band
        // against a total, so it cannot be swapped in as the low edge.
        assert!(owen_b.integration_domain.matches(owen_9.integration_domain));
        assert!(owen_b.integration_domain.matches(sellek.integration_domain));
        assert!(!sellek
            .integration_domain
            .matches(sellek_80.integration_domain));
        assert_eq!(
            sellek_80.integration_domain,
            WindIntegrationDomain::WithinRadiusAu(Fixed::from_int(80)),
        );
        // Two truncated chords match iff they share the radius; a total never matches a chord.
        assert!(WindIntegrationDomain::WithinRadiusAu(Fixed::from_int(80))
            .matches(WindIntegrationDomain::WithinRadiusAu(Fixed::from_int(80))));
        assert!(!WindIntegrationDomain::WithinRadiusAu(Fixed::from_int(80))
            .matches(WindIntegrationDomain::WithinRadiusAu(Fixed::from_int(160))));
        assert!(!WindIntegrationDomain::TotalDisk
            .matches(WindIntegrationDomain::WithinRadiusAu(Fixed::from_int(160))));
    }

    #[test]
    fn the_metallicity_domain_flags_off_solar_draws_without_moving_a_rate() {
        // The position classifier on the metallicity axis: a metal-poor draw classifies MetalPoor (a higher wind
        // rate through weaker cooling), a metal-rich draw MetalRich (a lower rate). Position only, no rate moved
        // (that is `metallicity_rate_factor`). No exact-equality arm: exactly-solar folds into MetalRich with a
        // unit factor, since exact `Z == sample` is unreachable by measure on a continuous draw.
        let fit = owen_appendix_b_fit();
        assert_eq!(
            fit.sample_metallicity,
            Fixed::ONE,
            "the coefficients are solar-sampled"
        );
        assert_eq!(
            fit.metallicity_domain(Fixed::from_ratio(3, 10)), // 0.3 Z_sun
            Some(MetallicitySampleDomain::MetalPoor),
            "a metal-poor draw runs a higher rate"
        );
        assert_eq!(
            fit.metallicity_domain(Fixed::from_int(2)), // 2 Z_sun
            Some(MetallicitySampleDomain::MetalRich),
            "a metal-rich draw runs a lower rate"
        );
        // Exactly on-sample folds into MetalRich (the unit-factor side), no dead exact-equality branch.
        assert_eq!(
            fit.metallicity_domain(Fixed::ONE),
            Some(MetallicitySampleDomain::MetalRich)
        );
        // A non-positive composition is not a draw: an error, never a classification.
        assert_eq!(fit.metallicity_domain(Fixed::ZERO), None);
        assert_eq!(fit.metallicity_domain(Fixed::from_int(-1)), None);
    }

    #[test]
    fn the_metallicity_widening_applies_the_fetched_slope_band_with_the_correct_sign() {
        // The widening, to the fetched slope band [-0.8, -0.4] dex/dex (Ercolano-Clarke -0.77, Nakatani
        // -0.6/-0.4). External oracle: (Z/Z_sample)^s. At Z = 0.3 solar the band is [0.3^-0.4, 0.3^-0.8] =
        // [1.62, 2.62] (metal-poor runs FASTER, factor > 1); at Z = 2 it is [2^-0.8, 2^-0.4] = [0.574, 0.758]
        // (metal-rich runs SLOWER, factor < 1). Solar is exactly [1, 1]. The band width is the slope band times
        // |log10 Z|, the model-dependent ignorance stated, not a point.
        let fit = owen_appendix_b_fit();
        let (steep, shallow, floor, ceiling) = (
            Fixed::from_ratio(-8, 10), // -0.8
            Fixed::from_ratio(-4, 10), // -0.4
            Fixed::from_ratio(3, 100), // 0.03 solar floor
            Fixed::from_int(2),        // 2 solar ceiling (Ercolano-Clarke fit top)
        );
        // Metal-poor: factor above one, both bounds.
        let poor = fit
            .metallicity_rate_factor(Fixed::from_ratio(3, 10), steep, shallow, floor, ceiling)
            .unwrap();
        assert!(
            (poor.lo().to_f64_lossy() - 1.62).abs() < 0.02
                && (poor.hi().to_f64_lossy() - 2.62).abs() < 0.02,
            "0.3 solar widens to [1.62, 2.62], got [{}, {}]",
            poor.lo().to_f64_lossy(),
            poor.hi().to_f64_lossy()
        );
        assert!(poor.lo() > Fixed::ONE, "a metal-poor draw runs faster");
        // Metal-rich: factor below one.
        let rich = fit
            .metallicity_rate_factor(Fixed::from_int(2), steep, shallow, floor, ceiling)
            .unwrap();
        assert!(rich.hi() < Fixed::ONE, "a metal-rich draw runs slower");
        assert!(
            (rich.lo().to_f64_lossy() - 0.574).abs() < 0.01,
            "2 solar low bound ~0.574, got {}",
            rich.lo().to_f64_lossy()
        );
        // Solar is the identity: no adjustment on-sample.
        let solar = fit
            .metallicity_rate_factor(Fixed::ONE, steep, shallow, floor, ceiling)
            .unwrap();
        assert_eq!(solar.lo(), Fixed::ONE);
        assert_eq!(solar.hi(), Fixed::ONE);
        assert_eq!(solar.width_dex(), Some(Fixed::ZERO), "solar has zero width");
        // The stated width is the slope band times |log10 Z|: 0.4 dex-of-slope * |log10 0.3| ~ 0.209 dex.
        let width = poor.width_dex().unwrap().to_f64_lossy();
        assert!(
            (width - 0.209).abs() < 0.005,
            "the band width is the model-dependent slope band, got {width}"
        );
    }

    #[test]
    fn the_metallicity_widening_refuses_outside_the_two_ended_domain() {
        // DOMAIN (two-ended): the slope holds between the ~0.03-solar FUV floor and the 2-solar fit ceiling; outside
        // either edge the widening REFUSES rather than extrapolating the slope into physics it does not describe.
        let fit = owen_appendix_b_fit();
        let (steep, shallow, floor, ceiling) = (
            Fixed::from_ratio(-8, 10),
            Fixed::from_ratio(-4, 10),
            Fixed::from_ratio(3, 100),
            Fixed::from_int(2), // 2 solar ceiling
        );
        // Below the floor: the FUV-turnover regime, a separate door.
        assert!(
            fit.metallicity_rate_factor(Fixed::from_ratio(2, 100), steep, shallow, floor, ceiling)
                .is_none(),
            "0.02 solar is below the slope domain: the FUV-turnover door, not an extrapolated factor"
        );
        // Above the ceiling: past the fitted range, the second door (the two-ended guard).
        assert!(
            fit.metallicity_rate_factor(Fixed::from_int(5), steep, shallow, floor, ceiling)
                .is_none(),
            "5 solar is above the Ercolano-Clarke fit's 2-solar edge: refuse, not a silent extrapolation"
        );
        // Inside the domain (both ends) resolves.
        assert!(
            fit.metallicity_rate_factor(Fixed::from_ratio(5, 100), steep, shallow, floor, ceiling)
                .is_some(),
            "0.05 solar is within the slope domain"
        );
        assert!(
            fit.metallicity_rate_factor(Fixed::from_int(2), steep, shallow, floor, ceiling)
                .is_some(),
            "2 solar sits exactly on the ceiling, inclusive"
        );
        // A non-positive draw is an error, never a factor.
        assert!(fit
            .metallicity_rate_factor(Fixed::ZERO, steep, shallow, floor, ceiling)
            .is_none());
    }

    #[test]
    fn the_wind_rate_matches_the_owen_solar_oracle() {
        // Twin-independent oracle: for the solar analogue at the reference luminosity (log10 L_X = 30, M = 1) the
        // Owen fit gives 6.25e-9 M_sun/yr, which is 6.25e-3 M_sun/Myr in the clock's units, computed outside the
        // code under test. A half-solar star adds a weak mass factor 0.5^-0.068 ~ 1.048, giving ~6.552e-3.
        let fit = owen_appendix_b_fit();
        let solar =
            photoevaporative_wind_rate_msun_myr(Fixed::from_int(30), Fixed::ONE, &fit).unwrap();
        assert!(
            (solar.to_f64_lossy() - 6.25e-3).abs() / 6.25e-3 < 0.02,
            "solar wind rate ~6.25e-3 M_sun/Myr (got {})",
            solar.to_f64_lossy()
        );
        let half =
            photoevaporative_wind_rate_msun_myr(Fixed::from_int(30), Fixed::from_ratio(1, 2), &fit)
                .unwrap();
        assert!(
            (half.to_f64_lossy() - 6.551_64e-3).abs() / 6.551_64e-3 < 0.02,
            "half-solar wind rate ~6.552e-3 M_sun/Myr (got {})",
            half.to_f64_lossy()
        );
    }

    #[test]
    fn the_wind_rate_scales_near_linearly_with_luminosity() {
        // A ten-times-brighter X-ray star (log10 L_X 30 -> 31) raises the rate by 10^1.14 ~ 13.80, the near-linear
        // L_X scaling checked against the base case rather than a second hand-number.
        let fit = owen_appendix_b_fit();
        let base = photoevaporative_wind_rate_msun_myr(Fixed::from_int(30), Fixed::ONE, &fit)
            .unwrap()
            .to_f64_lossy();
        let bright = photoevaporative_wind_rate_msun_myr(Fixed::from_int(31), Fixed::ONE, &fit)
            .unwrap()
            .to_f64_lossy();
        let expected = 10f64.powf(1.14);
        assert!(
            (bright / base - expected).abs() / expected < 0.02,
            "a decade brighter in X-rays raises the rate by 10^1.14 ~ {} (got ratio {})",
            expected,
            bright / base
        );
    }

    #[test]
    fn the_wind_rate_guards_the_mass_domain_and_refuses_nonphysical_inputs() {
        // The fit is measured over low-mass stars, so an intermediate-mass star (above mass_max) returns None
        // rather than an extrapolated rate; a non-positive mass likewise.
        let fit = owen_appendix_b_fit();
        assert!(
            photoevaporative_wind_rate_msun_myr(Fixed::from_int(30), Fixed::from_int(2), &fit)
                .is_none(),
            "a 2 M_sun star is outside the low-mass fit domain"
        );
        assert!(photoevaporative_wind_rate_msun_myr(
            Fixed::from_int(30),
            Fixed::from_ratio(1, 100),
            &fit
        )
        .is_none());
        assert!(
            photoevaporative_wind_rate_msun_myr(Fixed::from_int(30), Fixed::ZERO, &fit).is_none()
        );
    }

    #[test]
    fn the_wind_rate_feeds_the_dispersal_race() {
        // The end-to-end slice-2 chain: a derived wind rate feeds the race and yields a finite disk lifetime.
        // With a solar wind rate of ~6.25e-3 M_sun/Myr well below a peak accretion of 0.1 M_sun/Myr, the race
        // tips at a positive, finite tau_disk (the arc's output), not immediate dispersal or overflow.
        let fit = owen_appendix_b_fit();
        let wind =
            photoevaporative_wind_rate_msun_myr(Fixed::from_int(30), Fixed::ONE, &fit).unwrap();
        let tau = derive_disk_lifetime_myr(
            Fixed::from_ratio(1, 10), // Mdot_0 = 0.1 M_sun/Myr
            Fixed::ONE,               // t_visc = 1 Myr
            Fixed::ONE,               // gamma = 1
            wind,
        )
        .unwrap();
        assert!(
            tau > Fixed::ZERO,
            "the wind rate feeding the race gives a finite positive lifetime (tau {})",
            tau.to_f64_lossy()
        );
    }

    #[test]
    fn the_absolute_xray_luminosity_folds_to_the_solar_oracle() {
        // Twin-independent oracle: a solar-bolometric star (L_bol/L_sun = 1) at the saturated young-sun activity
        // fraction L_X/L_bol = 1e-3 gives log10(L_X) = log10(L_sun in erg/s) - 3 = 33.583 - 3 = 30.583, a few
        // times 1e30 erg/s, the observed young-solar-analogue X-ray level. Computed outside the code under test.
        let saturated_young_sun =
            stellar_xray_luminosity_log10_erg_s(Fixed::ONE, Fixed::from_ratio(1, 1000)).unwrap();
        assert!(
            (saturated_young_sun.to_f64_lossy() - 30.582_972).abs() < 0.01,
            "the young sun sits at log10(L_X) ~ 30.583 (got {})",
            saturated_young_sun.to_f64_lossy()
        );
        // At the full bolometric luminosity (fraction 1) the result is log10(L_sun in erg/s) itself, 33.583.
        let full = stellar_xray_luminosity_log10_erg_s(Fixed::ONE, Fixed::ONE).unwrap();
        assert!(
            (full.to_f64_lossy() - 33.582_972).abs() < 0.01,
            "the full-bolometric fold reproduces log10(L_sun in erg/s) (got {})",
            full.to_f64_lossy()
        );
    }

    #[test]
    fn the_absolute_xray_luminosity_is_a_decade_per_decade() {
        // Each ratio enters as a log10, so a ten-times-brighter bolometric star and a ten-times-more-active star
        // each raise log10(L_X) by exactly one, checked against the base rather than a second hand-number.
        let base = stellar_xray_luminosity_log10_erg_s(Fixed::ONE, Fixed::from_ratio(1, 1000))
            .unwrap()
            .to_f64_lossy();
        let brighter =
            stellar_xray_luminosity_log10_erg_s(Fixed::from_int(10), Fixed::from_ratio(1, 1000))
                .unwrap()
                .to_f64_lossy();
        let more_active =
            stellar_xray_luminosity_log10_erg_s(Fixed::ONE, Fixed::from_ratio(1, 100))
                .unwrap()
                .to_f64_lossy();
        assert!(
            (brighter - base - 1.0).abs() < 0.01 && (more_active - base - 1.0).abs() < 0.01,
            "a decade in either ratio adds one to log10(L_X) (base {}, brighter {}, more_active {})",
            base,
            brighter,
            more_active
        );
    }

    #[test]
    fn the_absolute_xray_luminosity_closes_the_chain_into_the_wind_rate() {
        // The end-to-end L_X-chain-into-wind-rate composition: the derived log10(L_X) for a saturated young sun,
        // fed straight into the Owen wind rate, gives a positive finite rate. This is the interim log10(L_X)
        // retired into a derived quantity, the destination the coordinator's L_X-first ruling named.
        let log10_l_x =
            stellar_xray_luminosity_log10_erg_s(Fixed::ONE, Fixed::from_ratio(1, 1000)).unwrap();
        let fit = owen_appendix_b_fit();
        let wind = photoevaporative_wind_rate_msun_myr(log10_l_x, Fixed::ONE, &fit).unwrap();
        assert!(
            wind > Fixed::ZERO,
            "the derived L_X feeds the wind rate to a positive value (got {})",
            wind.to_f64_lossy()
        );
    }

    #[test]
    fn the_absolute_xray_luminosity_refuses_nonphysical_ratios() {
        // Fail-loud on a non-positive bolometric ratio or activity fraction: neither has a logarithm.
        assert!(
            stellar_xray_luminosity_log10_erg_s(Fixed::ZERO, Fixed::from_ratio(1, 1000)).is_none()
        );
        assert!(stellar_xray_luminosity_log10_erg_s(Fixed::ONE, Fixed::ZERO).is_none());
        assert!(stellar_xray_luminosity_log10_erg_s(
            Fixed::from_int(-1),
            Fixed::from_ratio(1, 1000)
        )
        .is_none());
    }

    #[test]
    fn the_rotation_period_follows_skumanich_spindown() {
        // Twin-independent oracle: P_rot = P_ref * (age/age_ref)^n. With n = 1/2, P_ref = 10 days, age_ref = 100
        // Myr, a four-times-older star (age 400) has P = 10 * sqrt(4) = 20 days, and a four-times-younger star
        // (age 25) has P = 10 * sqrt(0.25) = 5 days, both computed outside the code. At the reference age the
        // period is the reference itself.
        let half = Fixed::from_ratio(1, 2);
        let p_ref = Fixed::from_int(10);
        let age_ref = Fixed::from_int(100);
        let older =
            stellar_rotation_period_days(Fixed::from_int(400), p_ref, age_ref, half).unwrap();
        let younger =
            stellar_rotation_period_days(Fixed::from_int(25), p_ref, age_ref, half).unwrap();
        let at_ref = stellar_rotation_period_days(age_ref, p_ref, age_ref, half).unwrap();
        assert!(
            (older.to_f64_lossy() - 20.0).abs() < 0.05,
            "a four-times-older star spins to ~20 days (got {})",
            older.to_f64_lossy()
        );
        assert!(
            (younger.to_f64_lossy() - 5.0).abs() < 0.02,
            "a four-times-younger star spins at ~5 days (got {})",
            younger.to_f64_lossy()
        );
        assert!(
            (at_ref.to_f64_lossy() - 10.0).abs() < 0.02,
            "at the reference age the period is the reference (got {})",
            at_ref.to_f64_lossy()
        );
    }

    #[test]
    fn the_rotation_period_refuses_nonphysical_inputs() {
        // Fail-loud on a non-positive age, reference period, reference age, or spin-down exponent.
        let half = Fixed::from_ratio(1, 2);
        assert!(stellar_rotation_period_days(
            Fixed::ZERO,
            Fixed::from_int(10),
            Fixed::from_int(100),
            half
        )
        .is_none());
        assert!(stellar_rotation_period_days(
            Fixed::from_int(100),
            Fixed::ZERO,
            Fixed::from_int(100),
            half
        )
        .is_none());
        assert!(stellar_rotation_period_days(
            Fixed::from_int(100),
            Fixed::from_int(10),
            Fixed::ZERO,
            half
        )
        .is_none());
        assert!(stellar_rotation_period_days(
            Fixed::from_int(100),
            Fixed::from_int(10),
            Fixed::from_int(100),
            Fixed::ZERO
        )
        .is_none());
    }

    #[test]
    fn the_spindown_gives_a_younger_star_a_lower_rossby() {
        // The end-to-end chain property the spin-down exists to produce: a younger star spins faster (shorter
        // P_rot), so at a fixed convective turnover it sits at a LOWER Rossby number, deeper in the saturated
        // regime. Composing the spin-down with the Rossby number (an independent function) must show it.
        let half = Fixed::from_ratio(1, 2);
        let p_ref = Fixed::from_int(25); // ~solar rotation days
        let age_ref = Fixed::from_int(4570); // ~solar age Myr
        let tau_conv = Fixed::from_int(14); // ~solar convective turnover days (a fixed denominator here)
        let young =
            stellar_rotation_period_days(Fixed::from_int(100), p_ref, age_ref, half).unwrap();
        let old =
            stellar_rotation_period_days(Fixed::from_int(4570), p_ref, age_ref, half).unwrap();
        let ro_young = stellar_rossby_number(young, tau_conv).unwrap();
        let ro_old = stellar_rossby_number(old, tau_conv).unwrap();
        assert!(
            ro_young < ro_old,
            "the younger star sits at a lower Rossby number (young {}, old {})",
            ro_young.to_f64_lossy(),
            ro_old.to_f64_lossy()
        );
    }

    #[test]
    fn the_pre_main_sequence_luminosity_matches_the_hayashi_contraction_oracle() {
        // Twin-independent oracle, computed OUTSIDE the code from the closed form R^3 = G M^2/(28 pi sigma T_H^4 t)
        // and L = 4 pi sigma T_H^4 R^2: a solar-mass star at the Hayashi wall (T_H = 4000 K) at age 1 Myr sits at
        // R ~ 2.69 R_sun and L ~ 1.669 L_sun, brighter than the main-sequence Sun, exactly as a pre-main-sequence
        // contracting star should be.
        // The T_H = 4000 K here is an ORACLE EVALUATION POINT, arbitrary and FROZEN with its hand-computed pair (the
        // 1.6686 exists FOR it); NOT the retired production constant, which reads the per-star BHAC15 grid
        // (civsim_physics::hayashi_wall) as of the retirement commit. Moved only if the mapping itself changes, at
        // which point the whole pair recomputes by the independent closed-form route stated above.
        let l = pre_main_sequence_luminosity_lsun(
            Fixed::ONE,
            Fixed::from_int(4000),
            Fixed::ONE, // 1 Myr
        )
        .unwrap();
        assert!(
            (l.to_f64_lossy() - 1.6686).abs() / 1.6686 < 0.02,
            "the 1 Myr solar pre-MS luminosity is ~1.669 L_sun (got {})",
            l.to_f64_lossy()
        );
    }

    #[test]
    fn the_pre_main_sequence_luminosity_declines_as_age_to_the_minus_two_thirds() {
        // The contraction signature the race's wind time-dependence runs through: L ~ t^(-2/3), so across the
        // 1-to-8 Myr window the luminosity falls by 8^(2/3) = 4 (and ~10^(2/3) ~ 4.64 across 1-to-10 Myr, the
        // "factor of five" the race band must now carry). Checked against the base rather than a second oracle.
        let young =
            pre_main_sequence_luminosity_lsun(Fixed::ONE, Fixed::from_int(4000), Fixed::ONE)
                .unwrap()
                .to_f64_lossy();
        let older = pre_main_sequence_luminosity_lsun(
            Fixed::ONE,
            Fixed::from_int(4000),
            Fixed::from_int(8),
        )
        .unwrap()
        .to_f64_lossy();
        assert!(
            (young / older - 4.0).abs() < 0.1,
            "the luminosity falls by 8^(2/3) = 4 across 1 to 8 Myr (young {}, older {}, ratio {})",
            young,
            older,
            young / older
        );
    }

    #[test]
    fn the_pre_main_sequence_luminosity_scales_mass_to_the_four_thirds() {
        // L ~ R^2 ~ (M^2)^(2/3) = M^(4/3), so a half-solar star sits at 0.5^(4/3) ~ 0.397 of the solar value,
        // a second independent scaling law over the same closed form.
        let solar =
            pre_main_sequence_luminosity_lsun(Fixed::ONE, Fixed::from_int(4000), Fixed::ONE)
                .unwrap()
                .to_f64_lossy();
        let half = pre_main_sequence_luminosity_lsun(
            Fixed::from_ratio(1, 2),
            Fixed::from_int(4000),
            Fixed::ONE,
        )
        .unwrap()
        .to_f64_lossy();
        assert!(
            (half / solar - 0.39685).abs() < 0.01,
            "half the mass gives 0.5^(4/3) ~ 0.397 of the luminosity (ratio {})",
            half / solar
        );
    }

    #[test]
    fn the_pre_main_sequence_luminosity_refuses_nonphysical_inputs() {
        // Fail-loud on each non-positive axis: no mass, no wall temperature, no age, no contraction luminosity.
        assert!(
            pre_main_sequence_luminosity_lsun(Fixed::ZERO, Fixed::from_int(4000), Fixed::ONE)
                .is_none()
        );
        assert!(pre_main_sequence_luminosity_lsun(Fixed::ONE, Fixed::ZERO, Fixed::ONE).is_none());
        assert!(
            pre_main_sequence_luminosity_lsun(Fixed::ONE, Fixed::from_int(4000), Fixed::ZERO)
                .is_none()
        );
    }

    #[test]
    fn the_pre_ms_turnover_matches_the_hayashi_mlt_oracle() {
        // Twin-independent oracle from tau = C (M/(4 pi sigma T_H^4))^(1/3): a solar-mass star at the Hayashi wall
        // (T_H = 4000 K) with C = 1.5 gives ~385 days, roughly a decade longer than the main-sequence ~14.5 days,
        // the fully-convective turnover a disk-era star has. Computed outside the code.
        // The T_H = 4000 K here is an ORACLE EVALUATION POINT, arbitrary and FROZEN with its hand-computed pair (the
        // ~385 days exists FOR it); NOT the retired production constant, which reads the per-star BHAC15 grid
        // (civsim_physics::hayashi_wall) as of the retirement commit. Moved only if the mapping itself changes, at
        // which point the whole pair recomputes by the independent closed-form route stated above.
        let c = Fixed::from_ratio(3, 2); // C = 1.5
        let tau = pre_main_sequence_convective_turnover_days(Fixed::ONE, Fixed::from_int(4000), c)
            .unwrap();
        assert!(
            (tau.to_f64_lossy() - 385.0).abs() / 385.0 < 0.02,
            "the solar pre-MS turnover is ~385 days (got {})",
            tau.to_f64_lossy()
        );
    }

    #[test]
    fn the_pre_ms_turnover_exceeds_the_main_sequence_polynomial() {
        // The systematic the finding named: the pre-MS (fully convective) turnover is ~an order of magnitude
        // longer than the main-sequence polynomial's value at the same mass. That gap, not the fitting error, is
        // what decides saturation.
        let fit = tau_poly();
        let ms = convective_turnover_time_days(Fixed::ONE, &fit)
            .unwrap()
            .to_f64_lossy();
        let pre = pre_main_sequence_convective_turnover_days(
            Fixed::ONE,
            Fixed::from_int(4000),
            Fixed::from_ratio(3, 2),
        )
        .unwrap()
        .to_f64_lossy();
        assert!(
            pre > ms * 10.0,
            "the pre-MS turnover exceeds the MS value by more than a decade (MS {}, pre-MS {})",
            ms,
            pre
        );
    }

    #[test]
    fn the_pre_ms_turnover_saturates_the_disk_era_rossby() {
        // THE WRONG-RO TRACE CLEARANCE (the saturation assertion, RIDER 1), now the CI fact that closes the
        // coordinator's slice-3 gate rather than a clearance in anyone's head. THE TRACE: a MAIN-SEQUENCE turnover
        // on a pre-main-sequence star pushes Ro onto the DECAY branch instead of the saturated plateau, so L_X
        // evaluates wrong, so the wind rate is wrong, so tau_disk is wrong, so the #73 giant gate decides
        // giant-hood on a corrupt clock. THE CLOSURE (the standing saturation-margin ruling, evaluated with the
        // pre-main-sequence tau_conv in place): disk-era Ro sits BELOW the knee (ro_sat = 0.13) by MORE THAN the
        // turnover's own error band, across the disk-era mass and rotation range. When this is green the trace is
        // cleared BY MACHINERY. With the MAIN-SEQUENCE turnover it FAILS for the solar-and-above masses (the
        // original finding), which is why the fix was load-bearing.
        let ro_sat = Fixed::from_ratio(13, 100);
        let c = Fixed::from_ratio(3, 2);
        // The turnover's own error band: the pre-MS coefficient C is anchorable only to a factor ~2 (the reserved
        // basis, tau ~ 250 to 400 d), and the Wright MS fit RMS is 0.028 dex (~7 percent). The factor-TWO margin
        // asserted below is a 0.30 dex band, so it exceeds BOTH: a wrong turnover cannot push Ro to the decay
        // branch within any plausible turnover uncertainty. Ro scales as 1/tau, so a factor-2 Ro margin tolerates a
        // factor-2 turnover error, the coefficient band itself.
        let fit_error_band_factor = Fixed::from_int(2);
        for mass in [
            Fixed::from_ratio(3, 10),
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            Fixed::from_ratio(136, 100),
        ] {
            let tau =
                pre_main_sequence_convective_turnover_days(mass, Fixed::from_int(4000), c).unwrap();
            // Cover the disk-era ROTATION range, not one value: the representative disk-locked ~8 days sits below
            // the knee by more than the error-band factor, and even the SLOW-rotation end (~15 days, the worst case
            // for saturation) stays below the knee, so ANY disk-era rotation saturates.
            let ro_locked = stellar_rossby_number(Fixed::from_int(8), tau).unwrap();
            assert!(
                ro_locked.checked_mul(fit_error_band_factor).unwrap() < ro_sat,
                "disk-locked (8 d) Ro is below the knee by more than the turnover error band at M = {} (Ro {}, knee {})",
                mass.to_f64_lossy(),
                ro_locked.to_f64_lossy(),
                ro_sat.to_f64_lossy()
            );
            let ro_slow = stellar_rossby_number(Fixed::from_int(15), tau).unwrap();
            assert!(
                ro_slow < ro_sat,
                "even a slow (15 d) disk-era rotator stays saturated at M = {} (Ro {}, knee {})",
                mass.to_f64_lossy(),
                ro_slow.to_f64_lossy(),
                ro_sat.to_f64_lossy()
            );
        }
    }

    #[test]
    fn the_pre_ms_turnover_refuses_nonphysical_inputs() {
        // Fail-loud on each non-positive axis: no mass, no wall temperature, no mixing-length coefficient.
        let c = Fixed::from_ratio(3, 2);
        assert!(
            pre_main_sequence_convective_turnover_days(Fixed::ZERO, Fixed::from_int(4000), c)
                .is_none()
        );
        assert!(pre_main_sequence_convective_turnover_days(Fixed::ONE, Fixed::ZERO, c).is_none());
        assert!(pre_main_sequence_convective_turnover_days(
            Fixed::ONE,
            Fixed::from_int(4000),
            Fixed::ZERO
        )
        .is_none());
    }

    // De-camouflaged fixture wall (post-#198 retirement of the viewer's HAYASHI_WALL_T_EFF_K): an ARBITRARY
    // in-range pre-main-sequence wall T_eff for the disk_era_xray unit-math tests, deliberately NOT the retired
    // 4000 K digits, so a future grep (human or agent) cannot read it as a surviving constant or restore it. A math
    // fixture, not a physical claim: these tests exercise the composed clock's ARITHMETIC, and any in-range wall
    // serves the byte-equality, monotonicity, and determinism they check. The LIVE path reads the per-star wall
    // from the BHAC15 grid (`civsim_physics::hayashi_wall::HayashiWallGrid`). UPGRADE TRIGGER: when the slice-2
    // run-path wire makes disk_era_xray live, that path's integration test consumes the real grid read (fixtures
    // for arithmetic, grid for integration).
    const DISK_ERA_XRAY_TEST_WALL_K: Fixed = Fixed::from_int(4200);

    #[test]
    fn the_composed_disk_clock_byte_equals_the_hand_chained_pieces() {
        // TWIN-INDEPENDENCE for the composed clock: the end-to-end `disk_era_xray_disk_lifetime_myr` must return
        // exactly what a hand-chain of the seven links returns, byte for byte, with no hidden transform between the
        // links. A solar-analogue disk-hosting star at 1 Myr: convective (T Tauri) branch, disk-locked rotation.
        let hayashi = DISK_ERA_XRAY_TEST_WALL_K;
        let mlt_c = Fixed::from_ratio(3, 2);
        let age = Fixed::ONE;
        let p_rot = Fixed::from_int(8);
        let ro_sat = Fixed::from_ratio(13, 100);
        let sat = Fixed::from_ratio(-313, 100);
        let beta = Fixed::from_ratio(-27, 10);
        let gamma = Fixed::ONE;
        let t_visc = Fixed::ONE;
        let fit = owen_appendix_b_fit();
        // Hand-chain the pieces, each an independent function, to the wind rate.
        let tau_conv =
            pre_main_sequence_convective_turnover_days(Fixed::ONE, hayashi, mlt_c).unwrap();
        let rossby = stellar_rossby_number(p_rot, tau_conv).unwrap();
        let activity = activity_luminosity_fraction(rossby, ro_sat, sat, beta).unwrap();
        let l_bol = pre_main_sequence_luminosity_lsun(Fixed::ONE, hayashi, age).unwrap();
        let log10_l_x = stellar_xray_luminosity_log10_erg_s(l_bol, activity).unwrap();
        let wind = photoevaporative_wind_rate_msun_myr(log10_l_x, Fixed::ONE, &fit).unwrap();
        // Peak accretion set a hundredfold above the derived wind so the race tips to a positive lifetime; the same
        // triple feeds both the hand-chain and the composed clock, so any wind magnitude reproduces this.
        let mdot_0 = wind.checked_mul(Fixed::from_int(100)).unwrap();
        let expected = derive_disk_lifetime_myr(mdot_0, t_visc, gamma, wind).unwrap();
        let composed = disk_era_xray_disk_lifetime_myr(
            Fixed::ONE,
            hayashi,
            age,
            p_rot,
            mlt_c,
            ro_sat,
            sat,
            beta,
            &fit,
            mdot_0,
            t_visc,
            gamma,
        )
        .unwrap();
        assert_eq!(
            composed, expected,
            "the composed clock byte-equals the hand-chained pieces (composed {}, hand {})",
            composed, expected
        );
        // MECHANISM (the race crosses): a solar-analogue disk-hosting star yields a positive lifetime, not zero (the
        // wind-beats-birth case) or a refusal. This asserts the race tips, nothing about where.
        assert!(
            composed > Fixed::ZERO,
            "the race crosses to a positive lifetime (got {})",
            composed.to_f64_lossy()
        );
    }

    #[test]
    fn the_composed_disk_clock_is_mechanistic_not_calibrated() {
        // THE ORACLE, REDESIGNED to the replacement-circularity ruling: this arc RETIRES the observed disk lifetime,
        // so no CI assert may key off the Haisch-Lada few-Myr range. A test that the derived value lands in the
        // retiree's band would encode the calibration where it can never be argued with, the I_Fitting lens firing
        // on a constant. Instead: falsifiable mechanics, a units-catastrophe bracket orders of magnitude wider than
        // any observational claim, and a determinism pin. The few-Myr comparison lives in the ensemble validator,
        // band-aware and out of CI's reach, where the ruling homed it: it validates the ENSEMBLE output, never an
        // input, never a median. If the derived solar value comes out at half a Myr or thirty, that is a Residual
        // finding to surface, not a red X to make green.
        let hayashi = DISK_ERA_XRAY_TEST_WALL_K;
        let mlt_c = Fixed::from_ratio(3, 2);
        let p_rot = Fixed::from_int(8);
        let ro_sat = Fixed::from_ratio(13, 100);
        let sat = Fixed::from_ratio(-313, 100);
        let beta = Fixed::from_ratio(-27, 10);
        let gamma = Fixed::ONE;
        let t_visc = Fixed::ONE;
        let fit = owen_appendix_b_fit();
        let clock = |age: Fixed, mdot_0: Fixed| {
            disk_era_xray_disk_lifetime_myr(
                Fixed::ONE,
                hayashi,
                age,
                p_rot,
                mlt_c,
                ro_sat,
                sat,
                beta,
                &fit,
                mdot_0,
                t_visc,
                gamma,
            )
        };
        // A clock whose peak accretion is a hundredfold over the young star's wind, held fixed across the age sweep.
        let young_wind = {
            let tau_conv =
                pre_main_sequence_convective_turnover_days(Fixed::ONE, hayashi, mlt_c).unwrap();
            let rossby = stellar_rossby_number(p_rot, tau_conv).unwrap();
            let activity = activity_luminosity_fraction(rossby, ro_sat, sat, beta).unwrap();
            let l_bol = pre_main_sequence_luminosity_lsun(Fixed::ONE, hayashi, Fixed::ONE).unwrap();
            let log10_l_x = stellar_xray_luminosity_log10_erg_s(l_bol, activity).unwrap();
            photoevaporative_wind_rate_msun_myr(log10_l_x, Fixed::ONE, &fit).unwrap()
        };
        let mdot_0 = young_wind.checked_mul(Fixed::from_int(100)).unwrap();
        let solar = clock(Fixed::ONE, mdot_0).unwrap();
        // MECHANISM, monotone in the wind at fixed clock: an OLDER pre-MS star is dimmer (L ~ t^-2/3), so lower L_X,
        // so a weaker wind, so the race tips LATER and the disk lives LONGER. Same clock, same activity (turnover is
        // age-independent), only the bolometric luminosity moves, so this isolates the wind monotonicity end to end.
        let older = clock(Fixed::from_int(8), mdot_0).unwrap();
        assert!(
            older > solar,
            "a dimmer (older) star drives a weaker wind and a longer disk life (solar {}, older {})",
            solar.to_f64_lossy(),
            older.to_f64_lossy()
        );
        // UNITS-CATASTROPHE BRACKET: bounds orders of magnitude wider than any observational claim (the retiree
        // spans ~0.5 to 10 Myr), so this asserts nothing the retiree owns yet still catches a kilobar-class units
        // slip cold. A derived value outside [1e-6, 1e6] Myr is a units break, not a physics result.
        assert!(
            solar > Fixed::from_ratio(1, 1_000_000) && solar < Fixed::from_int(1_000_000),
            "the derived lifetime is finite and units-sane, not a scale catastrophe (got {} Myr)",
            solar.to_f64_lossy()
        );
        // DETERMINISM PIN: once twin-checked (the byte-equal test above), pin the exact computed value so the code
        // keeps producing what it produced. This asserts the pipeline is reproducible, NOT that nature agrees. The
        // value, ~20.5 Myr, runs on the tagged solar INTERIMS (the disk-locked P_rot, the hundredfold-over-wind peak
        // accretion, the unit t_visc), so it is NOT the disk lifetime of the real Sun and is not meant to match the
        // retired few-Myr band: a Residual finding surfaced, not a miss to tune away. It moves, deliberately, when
        // the Omega_star_0, Mdot_0, and R_1 draws land, and the pin moves with it under a recorded re-pin.
        assert_eq!(
            solar.to_bits(),
            88_237_297_984_i64,
            "the solar-analogue lifetime is reproducible (got bits {}, ~{} Myr)",
            solar.to_bits(),
            solar.to_f64_lossy()
        );
    }

    #[test]
    fn the_composed_disk_clock_propagates_a_link_refusal() {
        // FAIL-LOUD propagation: a refusal at any link (here the turnover, on a zero mass) surfaces as a `None` from
        // the whole composition rather than a swallowed error or a plausible-looking number.
        let fit = owen_appendix_b_fit();
        let call = |mass: Fixed, hayashi: Fixed| {
            disk_era_xray_disk_lifetime_myr(
                mass,
                hayashi,
                Fixed::ONE,
                Fixed::from_int(8),
                Fixed::from_ratio(3, 2),
                Fixed::from_ratio(13, 100),
                Fixed::from_ratio(-313, 100),
                Fixed::from_ratio(-27, 10),
                &fit,
                Fixed::ONE,
                Fixed::ONE,
                Fixed::ONE,
            )
        };
        // A zero mass refuses at the turnover (the first link); a zero wall temperature refuses at the turnover and
        // the luminosity both. Each must propagate to a whole-chain None.
        assert!(call(Fixed::ZERO, DISK_ERA_XRAY_TEST_WALL_K).is_none());
        assert!(call(Fixed::ONE, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_live_disk_clock_composes_on_the_real_hayashi_wall_grid() {
        // STAGE 3, the rider-2 upgrade from #200 landing: the disk clock's LIVE path reads its wall from the BHAC15
        // grid (`civsim_physics::hayashi_wall::HayashiWallGrid`), not a fixture, so this integration test consumes
        // the real grid read where the unit-math tests above use the de-camouflaged arithmetic fixture (fixtures for
        // arithmetic, grid for integration, the split the fixture comment names). It proves the composed clock runs
        // end to end on the per-star wall the wire will feed it, and that the grid's OWN drift band (its wall chord)
        // propagates through the clock to a lifetime band, the chord carried, never collapsed.
        use civsim_physics::hayashi_wall::HayashiWallGrid;
        let grid = HayashiWallGrid::standard().expect("the standard BHAC15 wall grid loads");
        let reading = grid
            .wall_teff(Fixed::ONE)
            .expect("a solar-mass star is inside the grid's convective-track domain");

        // The tagged solar interims, the same the unit-math tests run (disk-locked rotation, unit t_visc, the
        // hundredfold-over-wind peak accretion), so this isolates the ONE change: the wall now comes from the grid.
        let mlt_c = Fixed::from_ratio(3, 2);
        let age = Fixed::ONE;
        let p_rot = Fixed::from_int(8);
        let ro_sat = Fixed::from_ratio(13, 100);
        let sat = Fixed::from_ratio(-313, 100);
        let beta = Fixed::from_ratio(-27, 10);
        let gamma = Fixed::ONE;
        let t_visc = Fixed::ONE;
        let fit = XrayWindFit::owen_appendix_b();
        // The peak accretion is sized a hundredfold over the CENTRAL wall's wind and then HELD FIXED across the
        // drift band, so the wall moves the wind (through luminosity and turnover) against a fixed birth accretion,
        // the isolation the mechanistic test uses for the age sweep. Sizing mdot_0 per-wall instead would pin the
        // race ratio and cancel the wall, which is the wrong probe.
        let central_wind = {
            let tau_conv =
                pre_main_sequence_convective_turnover_days(Fixed::ONE, reading.wall_teff_k, mlt_c)
                    .unwrap();
            let rossby = stellar_rossby_number(p_rot, tau_conv).unwrap();
            let activity = activity_luminosity_fraction(rossby, ro_sat, sat, beta).unwrap();
            let l_bol =
                pre_main_sequence_luminosity_lsun(Fixed::ONE, reading.wall_teff_k, age).unwrap();
            let log10_l_x = stellar_xray_luminosity_log10_erg_s(l_bol, activity).unwrap();
            photoevaporative_wind_rate_msun_myr(log10_l_x, Fixed::ONE, &fit).unwrap()
        };
        let mdot_0 = central_wind.checked_mul(Fixed::from_int(100)).unwrap();
        let clock = |wall: Fixed| {
            disk_era_xray_disk_lifetime_myr(
                Fixed::ONE,
                wall,
                age,
                p_rot,
                mlt_c,
                ro_sat,
                sat,
                beta,
                &fit,
                mdot_0,
                t_visc,
                gamma,
            )
        };

        // The clock composes on the real grid wall and the race crosses to a finite, units-sane lifetime. The value
        // is the tagged-interim solar analogue, NOT the real Sun's disk life (the same Residual-not-miss framing as
        // the fixture tests), so the assert is a units bracket, not a physical-band claim.
        let on_wall = clock(reading.wall_teff_k).expect("the clock composes on the grid wall");
        assert!(
            on_wall > Fixed::from_ratio(1, 1_000_000) && on_wall < Fixed::from_int(1_000_000),
            "the grid-wall lifetime is finite and units-sane (got {} Myr)",
            on_wall.to_f64_lossy()
        );

        // THE GRID CHORD PROPAGATES: the wall's own drift band (drift_lo_k, drift_hi_k) maps through the clock to a
        // lifetime band. The clock is monotone in the wall (a hotter wall is brighter, a stronger wind, a shorter
        // life), so the low-edge wall gives the longer life and the high-edge wall the shorter, and the band has
        // non-zero width whenever the grid's drift band does. The chord is carried end to end, not averaged away.
        assert!(
            reading.drift_lo_k < reading.drift_hi_k,
            "the grid row carries a drift band"
        );
        let life_at_lo =
            clock(reading.drift_lo_k).expect("the clock composes on the drift low edge");
        let life_at_hi =
            clock(reading.drift_hi_k).expect("the clock composes on the drift high edge");
        assert!(
            life_at_lo > life_at_hi,
            "a cooler wall (drift low) drives a longer disk life than a hotter one (lo {} Myr, hi {} Myr)",
            life_at_lo.to_f64_lossy(),
            life_at_hi.to_f64_lossy()
        );
    }
}
