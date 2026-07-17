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
/// RESERVED-with-basis, surfaced rather than fabricated: `alpha_viscosity`, the Shakura-Sunyaev turbulent-viscosity
/// parameter (basis ~0.001 to 0.01, Shakura & Sunyaev 1973; a per-disk datum, so a quiescent dead-zone disk and an
/// MRI-active disk are data rows), and `mean_molecular_weight` `mu`, the disk-gas mean molecular weight (basis
/// ~2.34 for a solar H2+He mix; a per-composition datum, so a carbon-rich or a metal-poor disk is a data row).
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
/// (`l_x_exponent = 1`, `mass_exponent = 0`), the paper's own order-of-magnitude form; (3) the Sellek et al. 2024
/// PLUTO+PRIZMO radiation-hydro revision, which finds integrated rates roughly an order of magnitude LOWER from
/// enhanced molecular cooling (a live rival, a lower coefficient on the same shape). The mechanism below applies
/// whichever row is passed.
///
/// RULED (owner, the batch audit): all three rows ship as the DECLARED ENSEMBLE, not a single picked row, because
/// they are distinct physics claims (a population-synthesis fit, an analytic estimate, a radiation-hydro rival),
/// the radiative-conductivity dispute pattern. Their roles: the appendix-B fit is the CENTRAL instance (pending
/// verbatim confirmation at the primary), equation 9 the same paper's order-of-magnitude cross-check, and Sellek
/// 2024 the LOW EDGE. THE COST, stated so no consumer is surprised: an order-of-magnitude wind band propagates
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
    let absorbed_irradiation_flux = reprocessing_factor.checked_mul(stellar_flux(
        mass_ratio,
        luminosity_exponent,
        distance_au,
    )?)?;
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
        // The Owen, Clarke and Ercolano 2012 appendix-B population-synthesis fit, as reserved-with-basis data.
        XrayWindFit {
            log10_coefficient_msun_yr: Fixed::from_ratio(-820412, 100_000), // log10(6.25e-9)
            log10_l_x_reference_erg_s: Fixed::from_int(30),                 // L_X_ref = 1e30 erg/s
            l_x_exponent: Fixed::from_ratio(114, 100),                      // 1.14
            mass_exponent: Fixed::from_ratio(-68, 1000),                    // -0.068
            mass_min_msun: Fixed::from_ratio(1, 10), // 0.1 M_sun (sample low-mass edge)
            mass_max_msun: Fixed::from_ratio(15, 10), // 1.5 M_sun (low-mass sample edge)
        }
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
        // THE SATURATION ASSERTION (RIDER 1), the claim the gate required proven rather than assumed and wants
        // mutation-tested. With the CORRECT pre-MS turnover and a disk-locked rotation (P_rot ~ 8 days), the Rossby
        // number sits BELOW the saturation knee (ro_sat = 0.13) by a margin exceeding the coefficient's own
        // uncertainty, across the disk-era mass range. This is what makes the blindness true on the right
        // substrate; with the MAIN-SEQUENCE turnover it FAILS for the solar-and-above masses (the finding).
        let ro_sat = Fixed::from_ratio(13, 100);
        let p_rot = Fixed::from_int(8); // disk-locked rotation, days
        let c = Fixed::from_ratio(3, 2);
        for mass in [
            Fixed::from_ratio(3, 10),
            Fixed::from_ratio(1, 2),
            Fixed::ONE,
            Fixed::from_ratio(136, 100),
        ] {
            let tau =
                pre_main_sequence_convective_turnover_days(mass, Fixed::from_int(4000), c).unwrap();
            let ro = stellar_rossby_number(p_rot, tau).unwrap();
            // Margin: Ro must sit at least a factor of two below the knee, exceeding the coefficient uncertainty.
            assert!(
                ro.checked_mul(Fixed::from_int(2)).unwrap() < ro_sat,
                "the disk-era star is saturated with margin at M = {} (Ro {}, knee {})",
                mass.to_f64_lossy(),
                ro.to_f64_lossy(),
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
}
