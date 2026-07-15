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

//! The stellar-structure FRONT-END (genesis-forward Stage 1): from a star's own data, its MASS as a fraction of
//! the Sun and its metallicity `Z` as a fraction of the Sun's, this derives the three quantities the
//! protoplanetary-disk thermal model downstream consumes: the luminosity `L`, the radius `R`, and the effective
//! temperature `T_eff`. The disk reads `L` for its irradiation term
//! (`T_irr(r) ~ (L/(16 pi sigma r^2))^(1/4)`, [`crate::astro::irradiated_disk_temperature`]) and `T_eff` as the
//! star's surface temperature. This slice keys composition on the single scalar `Z/Z_sun`; the hydrogen fraction
//! `X` (which the opacity substrate reads) and the detailed abundance pattern are the flagged sibling arc below,
//! not live arguments here.
//!
//! DORMANT by construction. Nothing here is called from the run path (the scenarios reach the star only through
//! [`crate::astro::stellar_flux`], which this module does not touch), so the byte-neutrality pins hold. Arming this
//! front-end into the run path, feeding its `luminosity_ratio` into the disk in place of the disk's internal
//! `mass_ratio^exponent`, is a later gated step, flagged in the module's record, not this slice.
//!
//! The one exact relation and the closure residues. The effective temperature is DERIVED, not fit: Stefan-Boltzmann
//! `L = 4*pi*R^2*sigma*T_eff^4` inverts to `T_eff = (L/(4*pi*R^2*sigma))^(1/4)`, with `sigma` the CODATA-derived
//! Stefan-Boltzmann constant ([`crate::physiology::derived_stefan_boltzmann`], `2*pi^5*k_B^4/(15*h^3*c^2)` over the
//! fundamentals, never authored). So `T_eff` demands no residue of its own: it follows from `L` and `R`. What the
//! main sequence's `L(M, Z)` and `R(M, Z)` DO need are the power-law SLOPES, and those are closure residues from
//! the stellar-structure integration (the opacity law, the energy-generation law, the boundary conditions), which
//! dimensional analysis alone cannot fix (see the Buckingham-Pi note below). The four slopes are the caller's
//! reserved values, surfaced with basis, never invented here.
//!
//! The Buckingham-Pi budget. The phenomenon's dimensional quantities are the three outputs (`L`, `R`, `T_eff`),
//! the mass `M`, and the constants that set a star's scale and transport (`G`, `sigma`, `c`, `k_B`, the proton
//! mass `m_H`, an opacity coefficient, an energy-generation coefficient), about eleven quantities over four base
//! dimensions (mass, length, time, temperature), leaving on the order of seven dimensionless groups, plus the
//! inherently dimensionless composition numbers (`X`, `Z`, the mean molecular weight, the opacity and burning
//! exponents). The count is large ON PURPOSE: it is why dimensional analysis does not reduce the star to a unique
//! `L(M)`, and why the exponents are honest closure residues rather than derivable numbers. This module's actual
//! demand is FOUR authored slopes (two in mass, two in metallicity), well inside that budget; the value line, not
//! the Pi budget, is the binding constraint.
//!
//! The value-authoring line and admit-the-alien. This kernel is fixed Rust. The authored things it holds are cited
//! REFERENCE ANCHORS, not world content: the solar luminosity, radius, and effective temperature (the Sun-anchored
//! scales, so at `M = M_sun` and `Z = Z_sun` it returns the Sun's `L`, `R`, and `T_eff`), reused from
//! [`crate::astro`]. Every PER-STAR input arrives as an ARGUMENT: the mass ratio, the metallicity ratio, and the
//! four slopes. A heavier star, a metal-poor halo star, a metal-rich disk star: each is a data row (different
//! arguments), never a rewrite. Nothing keys on the Sun as a hidden default; the Sun is the anchor the ratios are
//! measured against, exactly as the scenario supplies the mass ratio in [`crate::astro`].
//!
//! THE SOLAR-METALLICITY CONVENTION (owner ruling, a JOIN-LAW anchor). `Z/Z_sun` is dimensionless, so no `Z_sun`
//! value lives in this kernel: the scenario passes the ratio (Mirror = 1). But the adopted solar-abundance scale is
//! a project-wide CONVENTION that the stellar module's `Z/Z_sun` and the disk module's composition reference MUST
//! cite identically, or the ratio silently means different things across crates (a definition-mismatch across the
//! join). The project pins ASPLUND, GREVESSE, SAUVAL & SCOTT 2009 (AGSS09, `Z_sun ~ 0.0134`) as that anchor, the de
//! facto standard already: the AESOPUS opacity pulls and the Lodders-era condensation chain are AGSS09-referenced.
//! The scale has drifted across generations (Anders-Grevesse 1989 ~0.020, Grevesse-Sauval 1998 ~0.0170, Asplund-
//! Grevesse-Sauval 2005 (AGS05) ~0.0122, AGSS09 ~0.0134, Magg 2022 revising back toward ~0.016), so the anchor is
//! generation-pinned, not a bare number. HONEST TENSION (carried, not hidden): the low-`Z` Asplund scale conflicts
//! with helioseismic sound-speed inversions, the standing solar-modelling problem, partially eased by the Magg 2022
//! revision; the anchor carries this open tension rather than false settledness.
//!
//! The determinism and scale discipline. The dimensionless ratios (`L/L_sun`, `R/R_sun`) are order-one and stay
//! `Fixed`, formed by [`civsim_core::Fixed::powf`], the pinned transcendental. The effective temperature reuses the
//! proven wide path of [`crate::astro`]: the absolute surface flux `F = L/(4*pi*R^2)` (whose `L ~ 3.8e26 W` and
//! `R^2 ~ 4.8e17 m^2` overflow Q32.32 while the ~6.3e7 W/m^2 result fits) runs in exact rational arithmetic
//! (`civsim_units::bignum::BigRat`) with pi from Machin's formula, rounding once, and the fourth root reuses
//! [`civsim_physics::laws::radiative_equilibrium`] (two nested integer square roots, so the unrepresentable `T^4`
//! never forms). No floating point reaches canonical state.

use civsim_core::Fixed;
use civsim_units::bignum::{BigRat, BigUint};
use civsim_units::compute;

// The cited reference anchors and the pi precision are reused from the sibling stellar-source kernel, so this
// front-end and the disk it feeds share one Sun-anchored scale rather than re-authoring the solar values.
use crate::astro::{FLUX_PI_DIGITS, SOLAR_LUMINOSITY_W, SOLAR_RADIUS_M};

/// The main-sequence star the front-end derives: the two dimensionless structure ratios the disk and any later
/// consumer read, plus the effective temperature in kelvin. `luminosity_ratio` is `L/L_sun` and `radius_ratio` is
/// `R/R_sun` (both order-one on the main sequence, the anchor form the disk multiplies `L_sun` and `R_sun` by);
/// `effective_temperature_k` is `T_eff` in kelvin, derived from the two through Stefan-Boltzmann.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MainSequenceStar {
    /// `L / L_sun`, the luminosity as a fraction of the Sun's (dimensionless).
    pub luminosity_ratio: Fixed,
    /// `R / R_sun`, the radius as a fraction of the Sun's (dimensionless).
    pub radius_ratio: Fixed,
    /// `T_eff` in kelvin, the effective temperature derived from the luminosity and radius through Stefan-Boltzmann.
    pub effective_temperature_k: Fixed,
}

/// A non-negative `Fixed` (its bits over `2^FRAC_BITS`) as an exact rational, so an order-one `Fixed` ratio
/// multiplies into a wide-magnitude anchor without leaving exact arithmetic. The caller passes a non-negative
/// value (a luminosity or radius ratio is non-negative). Mirrors the private helper in [`crate::astro`].
fn nonneg_fixed_to_bigrat(value: Fixed) -> BigRat {
    let bits = value.to_bits();
    let num = BigUint::from_u64(bits.max(0) as u64);
    let den = BigUint::from_u64(1).shl_bits(Fixed::FRAC_BITS);
    BigRat::new(false, num, den)
}

/// The main-sequence LUMINOSITY ratio `L/L_sun`, DERIVED from the star's mass and metallicity through the
/// main-sequence relation `L/L_sun = (M/M_sun)^alpha * (Z/Z_sun)^lambda`. The mass power is the mass-luminosity
/// relation; the metallicity power is the ZAMS shift with composition.
///
/// The metallicity hook: HOW `Z` enters. Higher metallicity raises the envelope's Rosseland-mean opacity, the more
/// metal there is, the more bound-free and bound-bound absorption above the electron-scattering floor
/// ([`civsim_physics::opacity::electron_scattering_opacity`], the Thomson value the ladder tops out at, plus the
/// Kramers bound-free and free-free terms). A higher opacity impedes radiative transport, so at fixed mass the
/// zero-age main sequence sits at LOWER luminosity (metal-poor halo stars are the more luminous subdwarfs at fixed
/// mass). The DIRECTION is derived in form from the opacity physics; the MAGNITUDE, the exponent `lambda` linking
/// `d ln L` to `d ln Z`, is a stellar-structure closure residue (it needs the homology integration of the opacity
/// change through the envelope), so `metallicity_luminosity_exponent` is the caller's reserved value, expected
/// negative. At `Z = Z_sun` the metallicity factor is exactly one (anything to a power, with a unit base, is one),
/// so the solar anchor is preserved whatever the reserved exponent, mirroring the mass invariance at `M = M_sun`.
///
/// `mass_ratio` is `M/M_sun` and `metallicity_ratio` is `Z/Z_sun`, both scenario-set (the admit-the-alien test);
/// `mass_luminosity_exponent` (`alpha`) and `metallicity_luminosity_exponent` (`lambda`) are the reserved slopes.
/// A non-positive mass or metallicity ratio routes to `None`: a metal-free (`Z = 0`) population-III star is a
/// qualitatively different regime the single power law does not describe (no metal opacity, no CNO burning), a
/// flagged boundary rather than an extrapolation.
pub fn luminosity_ratio(
    mass_ratio: Fixed,
    metallicity_ratio: Fixed,
    mass_luminosity_exponent: Fixed,
    metallicity_luminosity_exponent: Fixed,
) -> Option<Fixed> {
    if mass_ratio <= Fixed::ZERO || metallicity_ratio <= Fixed::ZERO {
        return None;
    }
    let mass_factor = mass_ratio.powf(mass_luminosity_exponent);
    let metallicity_factor = metallicity_ratio.powf(metallicity_luminosity_exponent);
    mass_factor.checked_mul(metallicity_factor)
}

/// The main-sequence RADIUS ratio `R/R_sun`, DERIVED from the star's mass and metallicity through the mass-radius
/// relation `R/R_sun = (M/M_sun)^beta * (Z/Z_sun)^mu`. The mass power is the main-sequence mass-radius slope; the
/// metallicity power is the same opacity-driven ZAMS shift seen in the luminosity, in the OPPOSITE sense: the
/// higher opacity of a metal-rich envelope makes the star LARGER (puffier, cooler) at fixed mass, so
/// `metallicity_radius_exponent` is expected positive. Its magnitude is the same class of stellar-structure closure
/// residue as the luminosity's, the caller's reserved value.
///
/// `mass_ratio` (`M/M_sun`) and `metallicity_ratio` (`Z/Z_sun`) are scenario-set; `mass_radius_exponent` (`beta`)
/// and `metallicity_radius_exponent` (`mu`) are the reserved slopes. At `Z = Z_sun` the metallicity factor is
/// exactly one, preserving the solar anchor. A non-positive mass or metallicity ratio routes to `None` (the
/// population-III boundary, as in [`luminosity_ratio`]).
pub fn radius_ratio(
    mass_ratio: Fixed,
    metallicity_ratio: Fixed,
    mass_radius_exponent: Fixed,
    metallicity_radius_exponent: Fixed,
) -> Option<Fixed> {
    if mass_ratio <= Fixed::ZERO || metallicity_ratio <= Fixed::ZERO {
        return None;
    }
    let mass_factor = mass_ratio.powf(mass_radius_exponent);
    let metallicity_factor = metallicity_ratio.powf(metallicity_radius_exponent);
    mass_factor.checked_mul(metallicity_factor)
}

/// The main-sequence EFFECTIVE TEMPERATURE `T_eff` (K), DERIVED from the luminosity and radius through the
/// Stefan-Boltzmann inversion `T_eff = (L/(4*pi*R^2*sigma))^(1/4)`, with `L = L_sun*luminosity_ratio` and
/// `R = R_sun*radius_ratio` ([`luminosity_ratio`], [`radius_ratio`]) and `sigma` the CODATA-derived Stefan-Boltzmann
/// constant ([`crate::physiology::derived_stefan_boltzmann`]). `T_eff` carries no residue of its own: it follows
/// from the two structure ratios and the derived constant.
///
/// The metallicity cooling is EMERGENT, not authored. Higher `Z` lowers the luminosity (the negative `lambda`) and
/// raises the radius (the positive `mu`), and `T_eff ~ L^(1/4) * R^(-1/2)`, so BOTH effects push the effective
/// temperature down: a metal-rich star at fixed mass is cooler, a metal-poor star hotter (bluer), with no separate
/// `T_eff`-versus-`Z` slope entered anywhere. This is the derive-not-author check on the metallicity hook: the one
/// exact relation ties the surface temperature to the two structure ratios, so the ZAMS-cooling direction falls out
/// of `L` and `R` rather than being wired.
///
/// At `M = M_sun`, `Z = Z_sun` the ratios are one and this returns the Sun's effective temperature (~5772 K, the
/// IAU 2015 nominal value) from `L_sun`, `R_sun`, and the derived `sigma` alone, the derive-not-fit anchor, nothing
/// tuned to hit it. `t_max` is the representable ceiling the fourth-root read caps at (an engine bound the caller
/// sets, not a physical knob). `None` on a non-positive mass or metallicity ratio, or a surface flux past the
/// representable range.
pub fn effective_temperature(
    mass_ratio: Fixed,
    metallicity_ratio: Fixed,
    mass_luminosity_exponent: Fixed,
    mass_radius_exponent: Fixed,
    metallicity_luminosity_exponent: Fixed,
    metallicity_radius_exponent: Fixed,
    t_max: Fixed,
) -> Option<Fixed> {
    let l = luminosity_ratio(
        mass_ratio,
        metallicity_ratio,
        mass_luminosity_exponent,
        metallicity_luminosity_exponent,
    )?;
    let r = radius_ratio(
        mass_ratio,
        metallicity_ratio,
        mass_radius_exponent,
        metallicity_radius_exponent,
    )?;
    effective_temperature_from_ratios(l, r, t_max)
}

/// The Stefan-Boltzmann inversion shared by [`effective_temperature`] and [`main_sequence_star`]: the surface flux
/// `F = L_sun*luminosity_ratio / (4*pi*(R_sun*radius_ratio)^2)` runs in exact rational arithmetic (its `L` and
/// `R^2` overflow Q32.32) and rounds once, then the fourth root reuses
/// [`civsim_physics::laws::radiative_equilibrium`] with emissivity one (a star radiates as a blackbody at its
/// effective temperature by the definition of `T_eff`).
///
/// A Sun-grade surface flux (~6.3e7 W/m^2) fits Q32.32, so that path rounds the absolute flux and is taken
/// unchanged. But a hot massive star's SURFACE flux itself crosses the ceiling (an 18 M_sun star radiates
/// ~1.5e10 W/m^2, above the ~2.1e9 Q32.32 max, because a hotter photosphere is genuinely brighter per unit area),
/// so the absolute-flux read returns `None`. When it does, the SUN-RELATIVE form takes over without ever forming
/// the wide flux: `T_eff = T_sun*(F/F_sun)^(1/4)`, where the flux RATIO `F/F_sun = luminosity_ratio/radius_ratio^2`
/// is representable (~240 for 18 M_sun) and `T_sun` derives from the Sun's OWN representable surface flux. This is
/// the same log-space-census discipline as [`stellar_effective_temperature`], and it is strictly additive: every
/// star whose absolute surface flux fits is byte-identical to before, and only the massive stars that used to fail
/// now resolve. `None` only if even the flux RATIO passes the representable range (far above any real stellar mass).
fn effective_temperature_from_ratios(
    luminosity_ratio: Fixed,
    radius_ratio: Fixed,
    t_max: Fixed,
) -> Option<Fixed> {
    let l_sun = BigRat::from_decimal_str(SOLAR_LUMINOSITY_W).ok()?;
    let r_sun = BigRat::from_decimal_str(SOLAR_RADIUS_M).ok()?;
    let four_pi = BigRat::from_i64(4).mul(&compute::pi(FLUX_PI_DIGITS));
    let sigma = crate::physiology::derived_stefan_boltzmann();
    let luminosity = l_sun.mul(&nonneg_fixed_to_bigrat(luminosity_ratio));
    let r_star = r_sun.mul(&nonneg_fixed_to_bigrat(radius_ratio));
    let r2 = r_star.mul(&r_star);
    let denom = four_pi.mul(&r2);
    let surface_flux_bits = luminosity.div(&denom).round_to_scale(Fixed::FRAC_BITS)?;
    // The Sun-grade path: when the absolute surface flux fits Q32.32, round it and invert directly (byte-identical
    // to the pre-fix form for every star that used to resolve).
    if let Some(surface_flux) = Fixed::from_bits_i128(surface_flux_bits) {
        return Some(civsim_physics::laws::radiative_equilibrium(
            surface_flux,
            Fixed::ONE,
            sigma,
            t_max,
        ));
    }
    // The massive-star path: the absolute surface flux overflowed, so scale the Sun's effective temperature by the
    // representable flux ratio's fourth root, never forming the wide flux. F_sun = L_sun/(4*pi*R_sun^2) IS
    // representable (~6.3e7 W/m^2), so T_sun derives; F/F_sun = luminosity_ratio/radius_ratio^2.
    let solar_flux_bits = l_sun
        .div(&four_pi.mul(&r_sun.mul(&r_sun)))
        .round_to_scale(Fixed::FRAC_BITS)?;
    let solar_flux = Fixed::from_bits_i128(solar_flux_bits)?;
    let t_sun = civsim_physics::laws::radiative_equilibrium(solar_flux, Fixed::ONE, sigma, t_max);
    let flux_ratio = luminosity_ratio.checked_div(radius_ratio.checked_mul(radius_ratio)?)?;
    let t_eff = t_sun.checked_mul(flux_ratio.powf(Fixed::from_ratio(1, 4)))?;
    Some(if t_eff > t_max { t_max } else { t_eff })
}

/// The full main-sequence star the front-end hands the disk: the luminosity ratio, the radius ratio, and the
/// effective temperature, all DERIVED from the star's mass and metallicity ([`luminosity_ratio`], [`radius_ratio`],
/// [`effective_temperature`]). This is the Stage-1 deliverable: given `(M/M_sun, Z/Z_sun)` and the four reserved
/// slopes, it produces the `L`, `R`, `T_eff` the disk thermal skeleton reads. Every per-star input is an argument
/// (the admit-the-alien test); `t_max` is the engine ceiling. `None` on a non-positive mass or metallicity ratio or
/// a flux past the representable range.
///
/// The DETAILED abundance pattern is a sibling arc, not this front-end. This collapses composition to two scalars,
/// the metallicity ratio `Z/Z_sun` (the opacity and structure hook) and, where the opacity floor reads it, the
/// hydrogen fraction. WHICH elements make up `Z`, the abundance pattern set by the star's birth metallicity and
/// epoch, and its second-order effect on the opacity and the mean molecular weight, is the front-2 disk-research
/// hand-off, flagged for the owner, not derived here.
#[allow(clippy::too_many_arguments)]
pub fn main_sequence_star(
    mass_ratio: Fixed,
    metallicity_ratio: Fixed,
    mass_luminosity_exponent: Fixed,
    mass_radius_exponent: Fixed,
    metallicity_luminosity_exponent: Fixed,
    metallicity_radius_exponent: Fixed,
    t_max: Fixed,
) -> Option<MainSequenceStar> {
    let luminosity_ratio = luminosity_ratio(
        mass_ratio,
        metallicity_ratio,
        mass_luminosity_exponent,
        metallicity_luminosity_exponent,
    )?;
    let radius_ratio = radius_ratio(
        mass_ratio,
        metallicity_ratio,
        mass_radius_exponent,
        metallicity_radius_exponent,
    )?;
    let effective_temperature_k =
        effective_temperature_from_ratios(luminosity_ratio, radius_ratio, t_max)?;
    Some(MainSequenceStar {
        luminosity_ratio,
        radius_ratio,
        effective_temperature_k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The four slopes the tests pass are ILLUSTRATIVE, chosen only to exercise the mechanism's direction and rough
    // magnitude, never the reserved values. The mass slopes match the well-known main-sequence figures the sibling
    // kernel's tests use (alpha ~3.5, beta ~0.8); the metallicity slopes are round illustrative sensitivities of
    // the physically-expected sign (lambda negative, mu positive), not a claim about the true magnitude, which is
    // the owner's reserved closure residue.
    const ALPHA: (i64, i64) = (35, 10); // mass-luminosity slope, illustrative
    const BETA: (i64, i64) = (8, 10); // mass-radius slope, illustrative
    const LAMBDA: (i64, i64) = (-1, 2); // metallicity-luminosity slope, illustrative negative
    const MU: (i64, i64) = (1, 4); // metallicity-radius slope, illustrative positive

    fn alpha() -> Fixed {
        Fixed::from_ratio(ALPHA.0, ALPHA.1)
    }
    fn beta() -> Fixed {
        Fixed::from_ratio(BETA.0, BETA.1)
    }
    fn lambda() -> Fixed {
        Fixed::from_ratio(LAMBDA.0, LAMBDA.1)
    }
    fn mu() -> Fixed {
        Fixed::from_ratio(MU.0, MU.1)
    }
    fn t_max() -> Fixed {
        Fixed::from_int(100_000) // an engine ceiling above any main-sequence T_eff
    }

    #[test]
    fn the_sun_derives_unit_ratios_and_its_effective_temperature() {
        // At M = M_sun and Z = Z_sun the ratios are one (a unit base to any power) and T_eff derives from L_sun,
        // R_sun, and the CODATA-derived sigma alone: ~5772 K, the IAU 2015 nominal solar effective temperature,
        // never fit. The ~few-kelvin offset is the coarse Q32.32 sigma and the integer-root discretization, the
        // same as the sibling kernel's, not a knob.
        let star = main_sequence_star(
            Fixed::ONE,
            Fixed::ONE,
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .expect("the sun derives");
        assert!(
            (star.luminosity_ratio.to_f64_lossy() - 1.0).abs() < 1e-3,
            "L/L_sun is one at the Sun, got {}",
            star.luminosity_ratio.to_f64_lossy()
        );
        assert!(
            (star.radius_ratio.to_f64_lossy() - 1.0).abs() < 1e-3,
            "R/R_sun is one at the Sun, got {}",
            star.radius_ratio.to_f64_lossy()
        );
        assert!(
            (star.effective_temperature_k.to_f64_lossy() - 5772.0).abs() < 20.0,
            "the Sun derives T_eff ~5772 K, got {}",
            star.effective_temperature_k.to_f64_lossy()
        );
    }

    #[test]
    fn the_ratios_are_metallicity_independent_at_solar_metallicity() {
        // At Z = Z_sun (metallicity ratio one) the metallicity factor is one whatever the reserved slope, so a
        // solar-metallicity star's structure is the mass relation alone: the anchor stays put whatever lambda, mu.
        let a =
            luminosity_ratio(Fixed::ONE, Fixed::ONE, alpha(), Fixed::from_ratio(-3, 1)).unwrap();
        let b = luminosity_ratio(Fixed::ONE, Fixed::ONE, alpha(), Fixed::from_ratio(2, 1)).unwrap();
        assert_eq!(a, b, "at Z = Z_sun the metallicity slope does not move L");
        let c = radius_ratio(Fixed::ONE, Fixed::ONE, beta(), Fixed::from_ratio(5, 1)).unwrap();
        let d = radius_ratio(Fixed::ONE, Fixed::ONE, beta(), Fixed::ZERO).unwrap();
        assert_eq!(c, d, "at Z = Z_sun the metallicity slope does not move R");
    }

    #[test]
    fn a_more_massive_star_is_brighter_and_larger_and_hotter() {
        // The mass relations at solar metallicity: a two-solar-mass star has L/L_sun ~2^3.5 = ~11.3, R/R_sun ~2^0.8
        // = ~1.74, and (L outpacing the emitting area) a higher T_eff ~2^((alpha-2beta)/4) = ~2^0.475 = ~1.39 the
        // Sun's, the ordering and rough magnitudes the mass-luminosity and mass-radius relations assert.
        let sun = main_sequence_star(
            Fixed::ONE,
            Fixed::ONE,
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        let heavy = main_sequence_star(
            Fixed::from_int(2),
            Fixed::ONE,
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        assert!(heavy.luminosity_ratio > sun.luminosity_ratio, "brighter");
        assert!(heavy.radius_ratio > sun.radius_ratio, "larger");
        assert!(
            heavy.effective_temperature_k > sun.effective_temperature_k,
            "hotter"
        );
        let l_ratio = heavy.luminosity_ratio.to_f64_lossy() / sun.luminosity_ratio.to_f64_lossy();
        assert!(
            (l_ratio - 2.0_f64.powf(3.5)).abs() < 0.2,
            "L tracks 2^alpha (~11.3), got {l_ratio}"
        );
        let t_ratio = heavy.effective_temperature_k.to_f64_lossy()
            / sun.effective_temperature_k.to_f64_lossy();
        assert!(
            (t_ratio - 2.0_f64.powf(0.475)).abs() < 0.03,
            "T_eff tracks 2^((alpha-2beta)/4) (~1.39), got {t_ratio}"
        );
    }

    #[test]
    fn a_massive_star_resolves_its_hot_effective_temperature() {
        // The massive-star T_eff path (the regression guard for the Betelgeuse-mass hole). An 18 M_sun star's
        // ABSOLUTE surface flux (~1.5e10 W/m^2, a hotter photosphere radiating more per unit area) crosses the
        // Q32.32 ceiling, so the absolute-flux read returns None and the star used to fail to resolve entirely.
        // The sun-relative fallback (T_eff = T_sun*(L_ratio/R_ratio^2)^(1/4)) resolves it without forming the wide
        // flux. The expected T_eff is T_sun*18^((alpha-2beta)/4) = ~5769*18^0.475 = ~22800 K, a hot blue star.
        let star = main_sequence_star(
            Fixed::from_int(18),
            Fixed::ONE,
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .expect("an 18 M_sun (Betelgeuse-mass) star resolves its T_eff");
        let t_eff = star.effective_temperature_k.to_f64_lossy();
        assert!(
            (t_eff - 5769.0 * 18.0_f64.powf(0.475)).abs() < 400.0,
            "the 18 M_sun star reads a hot ~22800 K blue T_eff, got {t_eff}"
        );
        assert!(
            t_eff > 15000.0,
            "a Betelgeuse-mass star is far hotter than the Sun, got {t_eff}"
        );
    }

    #[test]
    fn the_massive_and_sun_grade_paths_agree_across_the_flux_ceiling() {
        // The sun-relative fallback and the absolute-flux read agree where both are valid (no seam at the crossover).
        // Just below the ceiling (a ~5 M_sun star, absolute surface flux still representable) the absolute path is
        // taken; the sun-relative form must land within a few kelvin of it, proving the two branches are the same
        // physics and the fix is strictly additive rather than a second, divergent model.
        let five = main_sequence_star(
            Fixed::from_int(5),
            Fixed::ONE,
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        // The sun-relative prediction T_sun*5^((alpha-2beta)/4) for the same star.
        let sun = main_sequence_star(
            Fixed::ONE,
            Fixed::ONE,
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        let predicted = sun.effective_temperature_k.to_f64_lossy() * 5.0_f64.powf(0.475);
        assert!(
            (five.effective_temperature_k.to_f64_lossy() - predicted).abs() < 20.0,
            "the absolute-flux and sun-relative T_eff agree at 5 M_sun: got {}, predicted {}",
            five.effective_temperature_k.to_f64_lossy(),
            predicted
        );
    }

    #[test]
    fn a_metal_rich_star_is_dimmer_and_larger_and_cooler_at_fixed_mass() {
        // The metallicity hook, direction. At fixed mass a metal-rich star (Z = 2 Z_sun) has higher opacity, so a
        // LOWER luminosity (negative lambda), a LARGER radius (positive mu), and, since T_eff ~ L^(1/4) R^(-1/2)
        // with both pushing down, a COOLER effective temperature. The cooling is emergent from L and R, not a
        // separate slope. Metal-poor is the mirror: hotter and bluer.
        let solar = main_sequence_star(
            Fixed::ONE,
            Fixed::ONE,
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        let rich = main_sequence_star(
            Fixed::from_int(2),
            Fixed::from_int(2),
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        // Hold mass fixed to isolate the metallicity effect: same mass, twice the metallicity.
        let poor = main_sequence_star(
            Fixed::ONE,
            Fixed::from_ratio(1, 2),
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        let rich_same_mass = main_sequence_star(
            Fixed::ONE,
            Fixed::from_int(2),
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        assert!(
            rich_same_mass.luminosity_ratio < solar.luminosity_ratio,
            "a metal-rich star is dimmer at fixed mass"
        );
        assert!(
            rich_same_mass.radius_ratio > solar.radius_ratio,
            "a metal-rich star is larger at fixed mass"
        );
        assert!(
            rich_same_mass.effective_temperature_k < solar.effective_temperature_k,
            "a metal-rich star is cooler at fixed mass"
        );
        assert!(
            poor.effective_temperature_k > solar.effective_temperature_k,
            "a metal-poor star is hotter at fixed mass"
        );
        // The rich two-solar-mass star is still hotter than the Sun (mass wins over the metallicity cooling here),
        // a sanity check that the two effects compose rather than one masking the other.
        assert!(rich.effective_temperature_k > solar.effective_temperature_k);
    }

    #[test]
    fn the_metallicity_cooling_matches_the_composed_power_law() {
        // T_eff / T_eff_sun = m^((alpha-2beta)/4) * z^((lambda-2mu)/4). At fixed solar mass, z = 2, the factor is
        // 2^((lambda-2mu)/4) = 2^((-0.5 - 0.5)/4) = 2^(-0.25) = ~0.841, a check that the emergent cooling is the
        // exact composition of the L and R metallicity powers, not an ad hoc shift.
        let solar = main_sequence_star(
            Fixed::ONE,
            Fixed::ONE,
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        let rich = main_sequence_star(
            Fixed::ONE,
            Fixed::from_int(2),
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        )
        .unwrap();
        let ratio = rich.effective_temperature_k.to_f64_lossy()
            / solar.effective_temperature_k.to_f64_lossy();
        let expected = 2.0_f64.powf((-0.5 - 2.0 * 0.25) / 4.0);
        assert!(
            (ratio - expected).abs() < 0.01,
            "the metallicity cooling is 2^((lambda-2mu)/4) ~{expected:.4}, got {ratio:.4}"
        );
    }

    #[test]
    fn the_subdwarf_sign_and_stefan_boltzmann_slope_are_the_pre_registered_receipt() {
        // The Population II subdwarf receipt, CORRECTED to the grid-extracted exponents (owner ruling, anomaly 2).
        // The naive "lambda < 0, mu > 0" assumption was FALSIFIED: MIST and PARSEC agree mu is small and slightly
        // NEGATIVE (~-0.018), because the grids couple helium to metals (Y rises with Z along the Galactic-
        // enrichment trajectory), and the mean-molecular-weight radius SHRINK outweighs the opacity SWELL. So at
        // fixed mass a metal-poor halo star is MORE LUMINOUS and HOTTER (robust, |lambda/4| dominates the T_eff
        // slope) and its radius is NEARLY FLAT, very slightly LARGER, not smaller. Both-clause receipt so the
        // corrected row is not misread against the name: subdwarfs are SUB-LUMINOUS at fixed COLOUR (the naming),
        // while BRIGHTER at fixed MASS (this row); both hold because the hotter metal-poor star slides blueward
        // along the colour axis. The exact Stefan-Boltzmann tie fixes d ln T_eff/d ln Z = lambda/4 - mu/2 = -0.101,
        // the joint cross-check the two independent grids closed to 0.001.
        let lam = Fixed::from_ratio(-44, 100); // lambda = -0.44, MIST + PARSEC banded, inside (-1, 0)
        let m = Fixed::from_ratio(-18, 1000); // mu = -0.018 along Y(Z), the Galactic-composition definition (b)
        let solar =
            main_sequence_star(Fixed::ONE, Fixed::ONE, alpha(), beta(), lam, m, t_max()).unwrap();
        let poor = main_sequence_star(
            Fixed::ONE,
            Fixed::from_ratio(1, 10),
            alpha(),
            beta(),
            lam,
            m,
            t_max(),
        )
        .unwrap();
        assert!(
            poor.luminosity_ratio > solar.luminosity_ratio,
            "metal-poor is more luminous at fixed mass (brighter clause)"
        );
        assert!(
            poor.effective_temperature_k > solar.effective_temperature_k,
            "metal-poor is hotter"
        );
        // Radius nearly flat, very slightly larger with mu < 0 (0.1^-0.018 ~ 1.04), never the old "smaller".
        assert!(
            poor.radius_ratio > solar.radius_ratio,
            "metal-poor radius is slightly larger, not smaller (the falsified half)"
        );
        assert!(
            poor.radius_ratio.to_f64_lossy() / solar.radius_ratio.to_f64_lossy() < 1.1,
            "the radius is nearly metallicity-independent (|mu| ~ 0.02)"
        );
        // The Stefan-Boltzmann slope identity by finite difference of ln T_eff over ln Z at fixed mass.
        let s1 = main_sequence_star(
            Fixed::ONE,
            Fixed::from_ratio(9, 10),
            alpha(),
            beta(),
            lam,
            m,
            t_max(),
        )
        .unwrap();
        let s2 = main_sequence_star(
            Fixed::ONE,
            Fixed::from_ratio(11, 10),
            alpha(),
            beta(),
            lam,
            m,
            t_max(),
        )
        .unwrap();
        let slope = (s2.effective_temperature_k.to_f64_lossy().ln()
            - s1.effective_temperature_k.to_f64_lossy().ln())
            / (1.1_f64.ln() - 0.9_f64.ln());
        let expected = -0.44 / 4.0 - (-0.018) / 2.0; // lambda/4 - mu/2 = -0.101
        assert!(
            (slope - expected).abs() < 0.01,
            "d ln T_eff/d ln Z = lambda/4 - mu/2 = {expected}, got {slope}"
        );
    }

    #[test]
    fn a_non_positive_mass_or_metallicity_routes_to_none() {
        // The population-III boundary and the guard: a non-positive mass or a metal-free metallicity is not an
        // extrapolation of the single power law, it is a different regime, routed to None.
        assert_eq!(
            main_sequence_star(
                Fixed::ZERO,
                Fixed::ONE,
                alpha(),
                beta(),
                lambda(),
                mu(),
                t_max()
            ),
            None
        );
        assert_eq!(
            main_sequence_star(
                Fixed::ONE,
                Fixed::ZERO,
                alpha(),
                beta(),
                lambda(),
                mu(),
                t_max()
            ),
            None
        );
        assert_eq!(
            luminosity_ratio(Fixed::ONE, Fixed::ZERO, alpha(), lambda()),
            None
        );
        assert_eq!(radius_ratio(Fixed::ZERO, Fixed::ONE, beta(), mu()), None);
    }

    #[test]
    fn the_front_end_is_deterministic() {
        // A pure derivation replays bit-for-bit, the determinism the canon requires.
        let a = main_sequence_star(
            Fixed::from_ratio(3, 2),
            Fixed::from_ratio(3, 2),
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        );
        let b = main_sequence_star(
            Fixed::from_ratio(3, 2),
            Fixed::from_ratio(3, 2),
            alpha(),
            beta(),
            lambda(),
            mu(),
            t_max(),
        );
        assert_eq!(a, b, "the front-end derivation replays deterministically");
    }
}
