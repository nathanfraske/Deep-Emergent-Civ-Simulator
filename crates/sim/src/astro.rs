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
}
