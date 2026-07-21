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

//! Post-main-sequence stellar tracks (design #77): a star ages OFF the main sequence into a giant or a
//! supergiant, so a Betelgeuse-mass star is RED by construction, DERIVED rather than tabulated. The
//! resolution (`docs/working/CAPSTONE_RESEARCH_RESOLUTIONS.md` section 3, "attractors plus clocks, NO
//! isochrone table") replaces the interpolated isochrone grid with four derived pieces, each keyed off the
//! star's own data so the sun-relative discipline of [`crate::astro`] carries over and the alien star is a
//! data row rather than a rewrite:
//!
//! 1. THE TRIGGER, [`schonberg_chandrasekhar_limit`]: the maximum isothermal-core mass fraction
//!    `q_max = 0.37*(mu_env/mu_core)^2` (Schonberg & Chandrasekhar 1942). When the growing helium core's
//!    fraction crosses `q_max`, the isothermal core can no longer support the envelope and the star leaves
//!    the main sequence.
//! 2. THE TEMPERATURE ATTRACTOR, [`hayashi_effective_temperature`]: the Hayashi boundary pins the giant or
//!    supergiant photosphere into a narrow band (~2000 to 4000 K), because the H-minus opacity the engine
//!    already carries ([`civsim_physics::opacity::h_minus_opacity`]) has a steep temperature sensitivity that
//!    makes the fully-convective envelope forget its history and settle onto the boundary. The attractor is
//!    almost independent of luminosity, so the photosphere is cool whatever the core produces.
//! 3. THE LUMINOSITY, [`shell_burning_luminosity`]: shell-burning homology (Kippenhahn, extending
//!    Refsdal-Weigert), the calibrated instance `L = 238000*mu^3*Z_CNO^0.04*(M_c^2 - 0.0305*M_c - 0.1802)`
//!    in solar luminosities for helium-core masses 0.5 to 0.66 solar, carrying two loud domain flags.
//! 4. THE CLOCK, [`core_growth_rate_msun_per_myr`]: `dM_c/dt = L(M_c)/(X_env*E_H)` with the hydrogen energy
//!    yield `E_H` a measured constant, so the giant-branch track `L(t)`, `T_eff(t)` (Hayashi), and `R(t)`
//!    (Stefan-Boltzmann, [`stellar_radius_rsun`]) is analytic.
//!
//! The one HONEST KNOB is MASS LOSS, a named banded closure (Reimers 1975 or de Jager et al. 1988 class),
//! whose coefficient is a FETCH not yet done ([`MASS_LOSS_FETCH`]). The track is built mass-loss-free (the
//! rate is a caller input defaulting to zero); the coefficient is surfaced, never invented.
//!
//! Fixed-point discipline: a supergiant's luminosity (~1e5 L_sun) and radius (~1e11 m) overflow Q32.32,
//! so this module keeps luminosity in SOLAR units (`L/L_sun`, order 1e5, representable) and radius in SOLAR
//! units (`R/R_sun`, order 1e3, representable), and the core-growth clock runs its wide kg/s divide in exact
//! `BigRat` and reports the rate in solar masses per megayear (order 0.1). This mirrors the sun-relative
//! Stefan-Boltzmann inversion [`crate::astro::stellar_effective_temperature`] uses so a Betelgeuse-mass star
//! never forms an unrepresentable intermediate.
//!
//! This is a DORMANT module: no run-path consumer reads it, so the world pins hold byte-for-byte. It is the
//! derivation the system generator (#72 onward) reads when a star ages past the main sequence.

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;

// =====================================================================================================
// Cited constants. Each is a CITED RESIDENT (the resolution, Kippenhahn stellar-structure homology, or a
// measured atomic constant), never an authored world-content value: the per-world quantities (the mean
// molecular weights, the CNO abundance, the envelope hydrogen fraction, the core mass) all arrive as
// arguments so an alien star is a data row. The cited decimal strings construct through `from_decimal_str`
// (the constructor-gate `deserialization` class).
// =====================================================================================================

/// The Schonberg-Chandrasekhar coefficient in `q_max = 0.37*(mu_env/mu_core)^2`, the dimensionless number
/// from the isothermal-core pressure-maximum derivation. Cited: Schonberg & Chandrasekhar 1942 (ApJ 96,
/// 161); Kippenhahn, Weigert & Weiss, "Stellar Structure and Evolution", the isothermal-core chapter; the
/// resolution (CAPSTONE_RESEARCH_RESOLUTIONS.md section 3).
pub const SCHONBERG_CHANDRASEKHAR_COEFF: &str = "0.37";

/// The shell-burning homology prefactor (solar luminosities) in the calibrated core-mass-luminosity
/// instance. Cited: the resolution section 3, calibrating the Kippenhahn (extending Refsdal-Weigert)
/// shell-burning homology so that at solar `mu` and CNO abundance the relation reproduces the
/// Paczynski-class core-mass-luminosity law of the giant branch.
pub const HOMOLOGY_PREFACTOR_LSUN: &str = "238000";

/// The CNO-abundance exponent in `Z_CNO^0.04`, the weak metallicity dependence of the shell luminosity.
/// Cited: the resolution section 3 (the calibrated homology instance).
pub const Z_CNO_EXPONENT: &str = "0.04";

/// The linear coefficient of the shell-luminosity core-mass polynomial `M_c^2 - 0.0305*M_c - 0.1802`.
/// Cited: the resolution section 3 (the calibrated homology fit, core masses 0.5 to 0.66 solar).
pub const POLY_LINEAR_COEFF: &str = "0.0305";

/// The constant term of the shell-luminosity core-mass polynomial `M_c^2 - 0.0305*M_c - 0.1802`. Cited:
/// the resolution section 3 (the calibrated homology fit, core masses 0.5 to 0.66 solar).
pub const POLY_CONSTANT_COEFF: &str = "0.1802";

/// The hydrogen-fusion mass-defect fraction: the fraction of rest mass released when four hydrogen atoms
/// fuse to one helium-4, `(4*m_1H - m_4He)/(4*m_1H) ~ 0.00712`. Cited: the CODATA/AME atomic masses
/// (`m_1H = 1.0078250319 u`, `m_4He = 4.0026032542 u`). The hydrogen energy yield is this fraction times
/// the fundamental `c^2` ([`default_hydrogen_energy_yield_1e14`]); neutrino losses (a few percent for the
/// pp chain, larger for the CNO cycle) lower the EFFECTIVE yield slightly, a documented correction the
/// caller may fold into its own `e_h_1e14` argument.
pub const HYDROGEN_MASSDEFECT_FRACTION: &str = "0.00712";

/// The lower helium-core mass (solar masses) of the calibrated shell-luminosity fit. Below this the
/// luminosity is NOT a function of core mass alone (a loud domain flag). Cited: the resolution section 3.
pub const CORE_MASS_FIT_LOW: &str = "0.5";

/// The upper helium-core mass (solar masses) of the calibrated shell-luminosity fit. Cited: the resolution
/// section 3 (the fit domain is 0.5 to 0.66 solar).
pub const CORE_MASS_FIT_HIGH: &str = "0.66";

/// The helium-core mass (solar masses) above which the envelope convection penetrates the burning shell and
/// the linear core-mass-luminosity relation FAILS (a loud domain flag); above it the luminosity enters the
/// radiation-pressure regime where `L` becomes proportional to core mass alone. Cited: the resolution
/// section 3.
pub const CORE_MASS_CONVECTION_LIMIT: &str = "0.8";

/// The Hayashi-boundary photospheric temperature band (kelvin) for a giant or supergiant envelope, the cool
/// wall the fully-convective star cannot cross. Cited: Hayashi 1961 (PASJ 13, 450); Hayashi & Hoshi 1961;
/// the standard giant/supergiant photospheric range (~2000 to 4000 K) the H-minus opacity sets. The band is
/// a caller argument to [`hayashi_effective_temperature`] so an alien envelope chemistry ships its own band;
/// these are the cited reference edges.
pub const HAYASHI_BAND_LOW_K: i32 = 2000;
/// The upper edge of the Hayashi band (kelvin). See [`HAYASHI_BAND_LOW_K`].
pub const HAYASHI_BAND_HIGH_K: i32 = 4000;

/// A representative red-supergiant Hayashi-line photospheric temperature (kelvin) inside the band, the cited
/// reference anchor a solar-composition M-supergiant envelope settles onto (the observed Betelgeuse effective
/// temperature is ~3600 K). Cited: the standard M-supergiant effective-temperature scale; Hayashi-boundary
/// stellar structure. A caller argument to [`hayashi_effective_temperature`], not a kernel literal.
pub const HAYASHI_ANCHOR_SUPERGIANT_K: i32 = 3500;

/// The one honest knob, surfaced not invented. Post-main-sequence MASS LOSS is a named banded closure whose
/// coefficient is a FETCH not yet done. Two literature forms:
///
/// - Reimers 1975 (Mem. Soc. R. Sci. Liege 8, 369): `dM/dt = -eta_R * 4e-13 * (L*R/M)` in solar units per
///   year, with the dimensionless efficiency `eta_R ~ 0.3 to 0.5` the coefficient that MUST be fetched.
/// - de Jager, Nieuwenhuijzen & van der Hucht 1988 (A&AS 72, 259): the Chebyshev-polynomial fit in
///   `(log T_eff, log L)` for luminous stars and supergiants, whose coefficient table MUST be fetched.
///
/// The helium-ignition core-mass anchor (the core mass at which the track terminates on the giant branch) is
/// the second stellar-track fetch on the consolidated list. Neither coefficient is written here: the mass-loss
/// rate enters [`core_growth_rate_msun_per_myr`] and the track assembly as a caller input defaulting to zero,
/// so the track is derived mass-loss-free until the owner supplies the fetched closure.
pub const MASS_LOSS_FETCH: &str =
    "FETCH REQUIRED: Reimers 1975 eta_R (~0.3 to 0.5) or de Jager et al. 1988 supergiant coefficient table, \
     plus the helium-ignition core-mass anchor. Coefficient not invented; mass-loss rate is a caller input, \
     default zero.";

// =====================================================================================================
// Piece 1: the trigger (Schonberg-Chandrasekhar).
// =====================================================================================================

/// THE TRIGGER: the Schonberg-Chandrasekhar limit `q_max = 0.37*(mu_env/mu_core)^2`, the maximum fraction of
/// the stellar mass an isothermal (spent, non-burning) core can hold before it can no longer support the
/// overlying envelope. `mu_env` is the mean molecular weight of the hydrogen-rich envelope (~0.6 for an
/// ionized solar mix) and `mu_core` the mean molecular weight of the isothermal helium core (~1.34 for fully
/// ionized helium); both are per-composition ARGUMENTS so an alien envelope or a different core ash is a data
/// row. Returns `q_max`, a dimensionless fraction (~0.07 to 0.10 for a helium core under a hydrogen envelope).
///
/// Fully derived from the cited coefficient and the two molecular weights, all order-one, so no wide
/// intermediate forms. `None` on a non-positive core molecular weight.
pub fn schonberg_chandrasekhar_limit(mu_env: Fixed, mu_core: Fixed) -> Option<Fixed> {
    if mu_core <= Fixed::ZERO {
        return None;
    }
    let coeff = Fixed::from_decimal_str(SCHONBERG_CHANDRASEKHAR_COEFF).ok()?;
    let ratio = mu_env.checked_div(mu_core)?;
    coeff.checked_mul(ratio.checked_mul(ratio)?)
}

/// Whether the star LEAVES THE MAIN SEQUENCE: the growing isothermal-core mass fraction `core_mass_fraction`
/// (`M_core/M_star`) has crossed the Schonberg-Chandrasekhar limit for the envelope and core molecular
/// weights. Once true, the core contracts and the envelope expands: the star heads for the giant branch,
/// where [`shell_burning_luminosity`] and [`hayashi_effective_temperature`] take over. `None` on a
/// non-positive core molecular weight (the limit is undefined).
pub fn leaves_main_sequence(
    core_mass_fraction: Fixed,
    mu_env: Fixed,
    mu_core: Fixed,
) -> Option<bool> {
    let q_max = schonberg_chandrasekhar_limit(mu_env, mu_core)?;
    Some(core_mass_fraction > q_max)
}

// =====================================================================================================
// Piece 3: the luminosity (shell-burning homology).
// =====================================================================================================

/// The shell-burning luminosity of a giant, with the two loud domain flags attached. `luminosity_lsun` is
/// `L/L_sun`; the flags say when the calibrated core-mass-luminosity relation is out of its fitted domain and
/// the value must not be trusted as a function of core mass alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellLuminosity {
    /// The luminosity in solar units (`L/L_sun`), from the calibrated homology fit.
    pub luminosity_lsun: Fixed,
    /// LOUD FLAG: the core mass is below the fitted range (0.5 solar). Below it the luminosity is NOT a
    /// function of core mass alone, so this value is an extrapolation to distrust.
    pub below_fitted_range: bool,
    /// LOUD FLAG: the core mass is above ~0.8 solar, where the envelope convection penetrates the burning
    /// shell and the linear relation FAILS. Above it the star enters the radiation-pressure regime where the
    /// luminosity becomes proportional to core mass alone (a different closure the fit does not carry).
    pub convection_penetration: bool,
    /// SOFT FLAG: the core mass is inside the physical regime but above the tight fitted ceiling (0.66
    /// solar) and below the convection limit (0.8 solar), so the value is an extrapolation of the fit rather
    /// than an interpolation.
    pub beyond_fit_extrapolation: bool,
}

/// THE LUMINOSITY: shell-burning homology, the calibrated instance
/// `L = 238000 * mu^3 * Z_CNO^0.04 * (M_c^2 - 0.0305*M_c - 0.1802)` in solar luminosities, for helium-core
/// masses `core_mass_msun` of 0.5 to 0.66 solar. `mu` is the mean molecular weight the homology carries (~0.6
/// for an ionized solar envelope) and `Z_CNO` the CNO mass fraction (~0.01 solar); both are per-composition
/// ARGUMENTS so a metal-poor or an alien-abundance star is a data row. Cited: the resolution section 3,
/// calibrating the Kippenhahn (extending Refsdal-Weigert) shell-burning homology.
///
/// TWO LOUD DOMAIN FLAGS travel with the value ([`ShellLuminosity`]): above ~0.8 solar core mass the envelope
/// convection penetrates the burning shell and the linear relation fails (the radiation-pressure regime, where
/// `L` becomes proportional to core mass alone); below the fitted 0.5-solar floor `L` is not a function of core
/// mass alone. The kernel still returns the polynomial value so a caller can see it, but the flags say when to
/// distrust it.
///
/// Every factor is order-one to order-1e5 and representable, so the luminosity forms directly in `Fixed` with
/// no wide intermediate. `None` on a non-positive `mu` or `Z_CNO` (the powers are undefined) or a negative
/// polynomial (a core mass so far below the fit that the quadratic goes negative, an unphysical read).
pub fn shell_burning_luminosity(
    core_mass_msun: Fixed,
    mu: Fixed,
    z_cno: Fixed,
) -> Option<ShellLuminosity> {
    if mu <= Fixed::ZERO || z_cno <= Fixed::ZERO {
        return None;
    }
    let prefactor = Fixed::from_decimal_str(HOMOLOGY_PREFACTOR_LSUN).ok()?;
    let z_exp = Fixed::from_decimal_str(Z_CNO_EXPONENT).ok()?;
    let lin = Fixed::from_decimal_str(POLY_LINEAR_COEFF).ok()?;
    let konst = Fixed::from_decimal_str(POLY_CONSTANT_COEFF).ok()?;

    // The core-mass polynomial M_c^2 - 0.0305*M_c - 0.1802.
    let mc2 = core_mass_msun.checked_mul(core_mass_msun)?;
    let poly = mc2
        .checked_sub(lin.checked_mul(core_mass_msun)?)?
        .checked_sub(konst)?;
    if poly <= Fixed::ZERO {
        return None;
    }

    // mu^3 (a cube, exact) and Z_CNO^0.04 (the pinned real power, base below one supported through ln/exp).
    let mu3 = mu.powi(3);
    let z_factor = z_cno.powf(z_exp);
    let luminosity_lsun = prefactor
        .checked_mul(mu3)?
        .checked_mul(z_factor)?
        .checked_mul(poly)?;

    let fit_low = Fixed::from_decimal_str(CORE_MASS_FIT_LOW).ok()?;
    let fit_high = Fixed::from_decimal_str(CORE_MASS_FIT_HIGH).ok()?;
    let conv_limit = Fixed::from_decimal_str(CORE_MASS_CONVECTION_LIMIT).ok()?;

    Some(ShellLuminosity {
        luminosity_lsun,
        below_fitted_range: core_mass_msun < fit_low,
        convection_penetration: core_mass_msun > conv_limit,
        beyond_fit_extrapolation: core_mass_msun > fit_high && core_mass_msun <= conv_limit,
    })
}

// =====================================================================================================
// Piece 2: the temperature attractor (Hayashi boundary).
// =====================================================================================================

/// THE TEMPERATURE ATTRACTOR: the Hayashi-boundary effective temperature, the cool wall a fully-convective
/// giant or supergiant photosphere is pinned to. `hayashi_anchor_k` is the Hayashi-line photospheric
/// temperature for the world's envelope opacity regime (~3000 to 4000 K for a solar-composition red
/// supergiant, [`HAYASHI_ANCHOR_SUPERGIANT_K`]); `band_low_k` and `band_high_k` are the physical band edges
/// (~2000 to 4000 K, [`HAYASHI_BAND_LOW_K`]/[`HAYASHI_BAND_HIGH_K`]) the result is clamped into. All three are
/// cited-reference ARGUMENTS so an alien envelope ships its own band.
///
/// The attractor is almost independent of luminosity: the H-minus opacity the engine carries
/// ([`civsim_physics::opacity::h_minus_opacity`]) rises so steeply with temperature that a small photospheric
/// warming spikes the opacity, chokes the flux, and drives the surface back to the boundary, so the envelope
/// forgets its history and settles onto the line. `weak_luminosity_exponent` carries the residual Hayashi-line
/// luminosity slope (very small, of order a few hundredths; DEFAULT it to zero for the pure attractor, since
/// the exact slope is a cited residue of the analytic Hayashi derivation this module does not fabricate). The
/// result is `clamp(hayashi_anchor_k * (L/L_sun)^weak_luminosity_exponent, [band_low_k, band_high_k])`.
///
/// This is WHY a Betelgeuse-mass star is red by construction: whatever huge luminosity the massive core sets,
/// the photosphere is dragged to the cool Hayashi wall, so the effective temperature is low and (through
/// [`stellar_radius_rsun`]) the radius is forced enormous. `None` on a non-positive luminosity ratio when the
/// weak slope is used, or an inverted band.
pub fn hayashi_effective_temperature(
    hayashi_anchor_k: Fixed,
    luminosity_ratio: Fixed,
    weak_luminosity_exponent: Fixed,
    band_low_k: Fixed,
    band_high_k: Fixed,
) -> Option<Fixed> {
    if band_high_k < band_low_k {
        return None;
    }
    let scaled = if weak_luminosity_exponent == Fixed::ZERO {
        hayashi_anchor_k
    } else {
        if luminosity_ratio <= Fixed::ZERO {
            return None;
        }
        hayashi_anchor_k.checked_mul(luminosity_ratio.powf(weak_luminosity_exponent))?
    };
    Some(if scaled < band_low_k {
        band_low_k
    } else if scaled > band_high_k {
        band_high_k
    } else {
        scaled
    })
}

// =====================================================================================================
// The radius (Stefan-Boltzmann, sun-relative), the forced consequence of L and T_eff.
// =====================================================================================================

/// The stellar RADIUS in SOLAR units (`R/R_sun`), the forced consequence of the core-set luminosity and the
/// Hayashi-pinned effective temperature through the Stefan-Boltzmann law `L = 4*pi*R^2*sigma*T_eff^4`. In
/// SUN-RELATIVE form `R/R_sun = sqrt(L/L_sun) * (T_sun/T_eff)^2`, so a supergiant radius (~1e3 R_sun, ~1e11 m)
/// never forms an unrepresentable metre value: `L/L_sun` (order 1e5) and the temperature ratio (order one)
/// are both representable, exactly the sun-relative discipline [`crate::astro::stellar_effective_temperature`]
/// uses. `luminosity_lsun` is `L/L_sun`, `t_eff_k` the effective temperature (kelvin, from
/// [`hayashi_effective_temperature`] for a giant), and `t_max` the fourth-root ceiling the solar-anchor read
/// caps at (an engine bound the caller sets).
///
/// The solar effective temperature `T_sun` (~5772 K) is DERIVED, not authored: it is
/// [`crate::astro::stellar_effective_temperature`] at unit mass, where the mass-relation exponents drop out
/// and the value comes from the cited `L_sun`, `R_sun`, and the CODATA-derived Stefan-Boltzmann constant
/// alone. `None` on a non-positive luminosity or effective temperature, or if the solar anchor fails to
/// derive.
pub fn stellar_radius_rsun(luminosity_lsun: Fixed, t_eff_k: Fixed, t_max: Fixed) -> Option<Fixed> {
    if luminosity_lsun <= Fixed::ZERO || t_eff_k <= Fixed::ZERO {
        return None;
    }
    // T_sun from the astro sun-relative solve at unit mass (exponents drop out): the derived solar anchor.
    let t_sun =
        crate::astro::stellar_effective_temperature(Fixed::ONE, Fixed::ONE, Fixed::ONE, t_max)?;
    let temp_ratio = t_sun.checked_div(t_eff_k)?;
    let temp_ratio_sq = temp_ratio.checked_mul(temp_ratio)?;
    luminosity_lsun.sqrt().checked_mul(temp_ratio_sq)
}

// =====================================================================================================
// Piece 4: the clock (core growth from shell burning).
// =====================================================================================================

/// The DEFAULT hydrogen energy yield `E_H` in units of 1e14 J/kg (~6.4), DERIVED from the cited hydrogen
/// mass-defect fraction ([`HYDROGEN_MASSDEFECT_FRACTION`], ~0.00712) times the fundamental `c^2`, divided by
/// 1e14 to land an order-one value. The scaled unit keeps `E_H` representable in `Fixed` (the raw ~6.4e14
/// J/kg overflows Q32.32), and the clock rescales by 1e14 internally. Neutrino losses lower the EFFECTIVE
/// yield a few percent; a caller with an alien fusion route or a neutrino correction passes its own scaled
/// `e_h_1e14`. `None` if the wide `fraction*c^2` divide fails to resolve.
pub fn default_hydrogen_energy_yield_1e14() -> Option<Fixed> {
    let frac = BigRat::from_decimal_str(HYDROGEN_MASSDEFECT_FRACTION).ok()?;
    let c = BigRat::from_decimal_str(civsim_units::fundamentals::SPEED_OF_LIGHT.value).ok()?;
    let c2 = c.mul(&c);
    // E_H / 1e14 = fraction * c^2 / 1e14, an order-one value (~6.4). 1e14 fits i64 (max ~9.2e18).
    let scale = BigRat::from_i64(100_000_000_000_000);
    let e_h_scaled = frac.mul(&c2).div(&scale);
    let bits = e_h_scaled.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

/// THE CLOCK: the rate the helium core grows as the hydrogen shell burns, `dM_c/dt = L/(X_env*E_H)`, in solar
/// masses per MEGAYEAR. The shell burns envelope hydrogen (fraction `X_env`) into helium that settles onto the
/// core; the energy released is the luminosity `L`, so the core-growth rate is the luminosity over the energy
/// per unit core mass added. `luminosity_lsun` is `L/L_sun`, `x_env` the envelope hydrogen mass fraction (~0.70
/// solar), and `e_h_1e14` the hydrogen energy yield in units of 1e14 J/kg
/// ([`default_hydrogen_energy_yield_1e14`], ~6.4). Both `x_env` and `e_h_1e14` are ARGUMENTS so an alien
/// envelope or an alien energy route is a data row.
///
/// Fixed-point discipline: the raw rate is ~1e15 kg/s and the constants (`L_sun` ~3.828e26 W, `M_sun` ~1.989e30
/// kg, `E_H` ~6.4e14 J/kg) all overflow Q32.32, so the wide divide runs in exact `BigRat` and rounds once to
/// solar masses per megayear (order 0.1, representable). Integrating this rate gives the giant-branch track
/// `M_c(t)`, and through it `L(t)`, `T_eff(t)`, and `R(t)`; the integration over long times is gated on the
/// mass-loss closure ([`MASS_LOSS_FETCH`]), which strips the envelope and eventually ends the branch. `None` on
/// a non-positive `x_env` or `e_h_1e14`, or if the wide divide fails to resolve.
pub fn core_growth_rate_msun_per_myr(
    luminosity_lsun: Fixed,
    x_env: Fixed,
    e_h_1e14: Fixed,
) -> Option<Fixed> {
    if x_env <= Fixed::ZERO || e_h_1e14 <= Fixed::ZERO || luminosity_lsun <= Fixed::ZERO {
        return None;
    }
    // dM_c/dt [M_sun/Myr] = luminosity_lsun * L_sun[W] * seconds_per_Myr / (x_env * e_h_1e14 * 1e14 * M_sun[kg]).
    let l_sun_w = BigRat::from_decimal_str(crate::astro::SOLAR_LUMINOSITY_W).ok()?;
    let m_sun_kg = BigRat::from_decimal_str(crate::astro::SOLAR_MASS_KG).ok()?;
    let julian_year_s = BigRat::from_i64(31_557_600); // seconds per Julian year (365.25 * 86400), a unit bridge
    let seconds_per_myr = BigRat::from_i64(1_000_000).mul(&julian_year_s);
    let e_h_scale = BigRat::from_i64(100_000_000_000_000); // 1e14, fits i64

    let l_ratio = fixed_to_bigrat(luminosity_lsun);
    let x = fixed_to_bigrat(x_env);
    let e_h = fixed_to_bigrat(e_h_1e14).mul(&e_h_scale);

    let numerator = l_ratio.mul(&l_sun_w).mul(&seconds_per_myr);
    let denominator = x.mul(&e_h).mul(&m_sun_kg);
    let rate = numerator.div(&denominator);
    let bits = rate.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

/// A non-negative `Fixed` as an exact rational (its bits over `2^FRAC_BITS`), so an order-one `Fixed` argument
/// enters the wide-magnitude `BigRat` divide without leaving exact arithmetic. Mirrors the `astro` helper.
fn fixed_to_bigrat(value: Fixed) -> BigRat {
    let bits = value.to_bits().max(0);
    BigRat::new(
        false,
        civsim_units::bignum::BigUint::from_u64(bits as u64),
        civsim_units::bignum::BigUint::from_u64(1).shl_bits(Fixed::FRAC_BITS),
    )
}

// =====================================================================================================
// The assembled analytic track point (the four pieces at a given core mass).
// =====================================================================================================

/// A single analytic GIANT-BRANCH TRACK POINT: the star's state at a given helium-core mass, assembled from
/// the four derived pieces. Luminosity in solar units, effective temperature in kelvin (Hayashi-pinned),
/// radius in solar units, and the core-growth rate in solar masses per megayear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GiantBranchPoint {
    /// `L/L_sun` from [`shell_burning_luminosity`], with the domain flags.
    pub luminosity: ShellLuminosity,
    /// The Hayashi-pinned effective temperature (kelvin).
    pub t_eff_k: Fixed,
    /// `R/R_sun`, the forced Stefan-Boltzmann consequence of the luminosity and the effective temperature.
    pub radius_rsun: Fixed,
    /// The core-growth rate `dM_c/dt` in solar masses per megayear (the clock).
    pub core_growth_msun_per_myr: Fixed,
}

/// Assemble the analytic giant-branch track point at a helium-core mass, mass-loss-free (the mass-loss rate
/// enters as a caller input to the time integration, not this instantaneous state). The luminosity carries
/// its domain flags; the effective temperature is the Hayashi attractor; the radius follows from both; the
/// clock gives the core-growth rate. The arguments are the star's own composition data (`mu`, `z_cno`,
/// `x_env`) plus the Hayashi band, so an alien star is a data row. `None` if any piece fails to derive.
#[allow(clippy::too_many_arguments)]
pub fn giant_branch_point(
    core_mass_msun: Fixed,
    mu: Fixed,
    z_cno: Fixed,
    x_env: Fixed,
    e_h_1e14: Fixed,
    hayashi_anchor_k: Fixed,
    weak_luminosity_exponent: Fixed,
    band_low_k: Fixed,
    band_high_k: Fixed,
    t_max: Fixed,
) -> Option<GiantBranchPoint> {
    let luminosity = shell_burning_luminosity(core_mass_msun, mu, z_cno)?;
    let t_eff_k = hayashi_effective_temperature(
        hayashi_anchor_k,
        luminosity.luminosity_lsun,
        weak_luminosity_exponent,
        band_low_k,
        band_high_k,
    )?;
    let radius_rsun = stellar_radius_rsun(luminosity.luminosity_lsun, t_eff_k, t_max)?;
    let core_growth_msun_per_myr =
        core_growth_rate_msun_per_myr(luminosity.luminosity_lsun, x_env, e_h_1e14)?;
    Some(GiantBranchPoint {
        luminosity,
        t_eff_k,
        radius_rsun,
        core_growth_msun_per_myr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cited-default constructors for the tests, built from exact ratios so the constructor gate's hard
    // `from_decimal_str` ratchet stays confined to the kernel's cited constants.
    fn t_max() -> Fixed {
        Fixed::from_int(100_000)
    }
    fn solar_mu() -> Fixed {
        Fixed::from_ratio(60, 100) // ionized solar envelope mean molecular weight ~0.6
    }
    fn solar_mu_core() -> Fixed {
        Fixed::from_ratio(134, 100) // fully ionized helium core ~1.34
    }
    fn solar_z_cno() -> Fixed {
        Fixed::from_ratio(1, 100) // solar CNO mass fraction ~0.01
    }
    fn solar_x_env() -> Fixed {
        Fixed::from_ratio(70, 100) // solar envelope hydrogen mass fraction ~0.70
    }

    #[test]
    fn a_schonberg_chandrasekhar_limit_is_a_small_core_fraction() {
        // q_max = 0.37*(mu_env/mu_core)^2 for a helium core (~1.34) under a hydrogen envelope (~0.6):
        // 0.37*(0.6/1.34)^2 ~ 0.074, in the cited ~0.07 to 0.10 band. The resolution's ~0.10 is the
        // upper-envelope-mu end; the exact fraction rides the caller's molecular weights.
        let q =
            schonberg_chandrasekhar_limit(solar_mu(), solar_mu_core()).expect("the limit derives");
        let v = q.to_f64_lossy();
        assert!(
            (0.05..=0.12).contains(&v),
            "the SC limit lands a small core fraction, got {v}"
        );
    }

    #[test]
    fn a_growing_core_trips_the_main_sequence_trigger() {
        // Below the limit the star is on the main sequence; a core grown past ~0.074 of the mass leaves it.
        let below = leaves_main_sequence(Fixed::from_ratio(5, 100), solar_mu(), solar_mu_core())
            .expect("defined");
        let above = leaves_main_sequence(Fixed::from_ratio(15, 100), solar_mu(), solar_mu_core())
            .expect("defined");
        assert!(!below, "a 0.05 core fraction is still on the main sequence");
        assert!(above, "a 0.15 core fraction has left the main sequence");
    }

    #[test]
    fn a_shell_luminosity_reproduces_the_giant_branch_scale() {
        // At the fitted ceiling M_c = 0.66 solar with solar mu and CNO, the calibrated homology reproduces the
        // Paczynski-class core-mass-luminosity scale, ~1e4 L_sun (a bright red giant), inside the fitted range
        // (no flags). This is the derive-not-fit anchor for the luminosity piece.
        let sl = shell_burning_luminosity(Fixed::from_ratio(66, 100), solar_mu(), solar_z_cno())
            .expect("the luminosity derives");
        let l = sl.luminosity_lsun.to_f64_lossy();
        assert!(
            (5.0e3..=2.0e4).contains(&l),
            "M_c = 0.66 solar derives ~1e4 L_sun, got {l}"
        );
        assert!(!sl.below_fitted_range, "0.66 is not below the fitted floor");
        assert!(
            !sl.convection_penetration,
            "0.66 is below the convection limit"
        );
        assert!(
            !sl.beyond_fit_extrapolation,
            "0.66 is the fitted ceiling, not beyond it"
        );
    }

    #[test]
    fn a_low_core_mass_raises_the_below_range_flag() {
        // Below the fitted 0.5-solar floor the luminosity is not a function of core mass alone: the loud flag
        // fires. (0.48 keeps the polynomial positive so the value still forms; the flag is the point.)
        let sl = shell_burning_luminosity(Fixed::from_ratio(48, 100), solar_mu(), solar_z_cno())
            .expect("still forms");
        assert!(
            sl.below_fitted_range,
            "a 0.48-solar core is below the fitted range"
        );
    }

    #[test]
    fn a_massive_core_raises_the_convection_penetration_flag() {
        // An 18 M_sun star's helium core (~5.5 solar) is far above the 0.8-solar convection limit: the loud
        // flag fires, so the polynomial value must not be trusted (the star is in the radiation-pressure
        // regime where L is proportional to core mass alone, a closure this fit does not carry).
        let sl = shell_burning_luminosity(Fixed::from_ratio(55, 10), solar_mu(), solar_z_cno())
            .expect("still forms");
        assert!(
            sl.convection_penetration,
            "a 5.5-solar core trips the convection-penetration flag"
        );
    }

    #[test]
    fn a_hayashi_attractor_pins_a_cool_photosphere() {
        // The attractor pins the photosphere to the cited band whatever the luminosity. With the pure
        // attractor (zero weak slope), a 1e5 L_sun supergiant and a 1e3 L_sun giant land the SAME cool anchor,
        // ~3500 K, deep in the red. This is the "red by construction" mechanism.
        let anchor = Fixed::from_int(HAYASHI_ANCHOR_SUPERGIANT_K);
        let low = Fixed::from_int(HAYASHI_BAND_LOW_K);
        let high = Fixed::from_int(HAYASHI_BAND_HIGH_K);
        let hot_lum = Fixed::from_int(100_000);
        let cool_lum = Fixed::from_int(1_000);
        let t_hot = hayashi_effective_temperature(anchor, hot_lum, Fixed::ZERO, low, high)
            .expect("derives");
        let t_cool = hayashi_effective_temperature(anchor, cool_lum, Fixed::ZERO, low, high)
            .expect("derives");
        assert_eq!(
            t_hot, t_cool,
            "the pure attractor is luminosity-independent"
        );
        let k = t_hot.to_f64_lossy();
        assert!(
            (3000.0..=4000.0).contains(&k),
            "the Hayashi photosphere is red, ~3500 K, got {k}"
        );
    }

    #[test]
    fn a_hayashi_attractor_clamps_into_the_band() {
        // A hot anchor (say 6000 K) is clamped down to the 4000 K band ceiling: the star cannot cross the
        // Hayashi wall to the blue.
        let low = Fixed::from_int(HAYASHI_BAND_LOW_K);
        let high = Fixed::from_int(HAYASHI_BAND_HIGH_K);
        let t = hayashi_effective_temperature(
            Fixed::from_int(6000),
            Fixed::ONE,
            Fixed::ZERO,
            low,
            high,
        )
        .expect("derives");
        assert_eq!(
            t.to_int(),
            HAYASHI_BAND_HIGH_K,
            "clamped to the band ceiling"
        );
    }

    #[test]
    fn a_betelgeuse_mass_star_is_a_red_supergiant_by_construction() {
        // THE DEMONSTRATION. An 18 M_sun star, post-main-sequence:
        //  - its helium core (~5.5 solar) is far past the Schonberg-Chandrasekhar trigger, so it has left the
        //    main sequence, and past the 0.8-solar convection limit (the radiation-pressure regime where L is
        //    set by the core mass alone: huge, ~1e5 L_sun).
        //  - the Hayashi attractor pins its photosphere to ~3500 K: RED, whatever the luminosity.
        //  - Stefan-Boltzmann then FORCES a huge radius from the core-set L and the cool T.
        // The core-set L in the radiation-pressure regime is above the fitted polynomial's domain (correctly
        // flagged), so it enters here as the observed-class supergiant luminosity (a demonstration input, not a
        // fabricated fit value). The point proven is the FORCING: given that huge L and the Hayashi T, the
        // radius is enormous, so the star is a red supergiant by construction.
        let anchor = Fixed::from_int(HAYASHI_ANCHOR_SUPERGIANT_K);
        let low = Fixed::from_int(HAYASHI_BAND_LOW_K);
        let high = Fixed::from_int(HAYASHI_BAND_HIGH_K);

        // The trigger fired long ago: a 5.5-solar core in an 18-solar star is a fraction ~0.31, far past q_max.
        let left_ms = leaves_main_sequence(Fixed::from_ratio(31, 100), solar_mu(), solar_mu_core())
            .expect("defined");
        assert!(left_ms, "the massive core has left the main sequence");

        // The Hayashi-pinned effective temperature: RED.
        let t_eff =
            hayashi_effective_temperature(anchor, Fixed::from_int(126_000), Fixed::ZERO, low, high)
                .expect("derives");
        let t_k = t_eff.to_f64_lossy();
        assert!(
            (3000.0..=4000.0).contains(&t_k),
            "the supergiant photosphere is red, ~3500 K, got {t_k}"
        );

        // The core-set luminosity (~1.26e5 L_sun, the observed Betelgeuse class) and the cool T force a huge
        // radius: R/R_sun = sqrt(L/L_sun)*(T_sun/T_eff)^2 ~ sqrt(1.26e5)*(5772/3500)^2 ~ 355*2.72 ~ 970.
        let r = stellar_radius_rsun(Fixed::from_int(126_000), t_eff, t_max())
            .expect("the radius derives");
        let r_rsun = r.to_f64_lossy();
        assert!(
            r_rsun > 500.0,
            "a red supergiant has a huge radius, >500 R_sun, got {r_rsun}"
        );
        // Betelgeuse's measured radius is ~760 R_sun; the derived value is order-correct for a red supergiant.
        assert!(
            (500.0..=1500.0).contains(&r_rsun),
            "the forced radius is red-supergiant scale, got {r_rsun} R_sun"
        );
    }

    #[test]
    fn a_red_giant_track_point_assembles() {
        // The full analytic track point at the fitted ceiling M_c = 0.66 solar: a bright red giant, cool and
        // large, with a positive core-growth clock and no domain flags.
        let e_h = default_hydrogen_energy_yield_1e14().expect("E_H derives");
        let pt = giant_branch_point(
            Fixed::from_ratio(66, 100),
            solar_mu(),
            solar_z_cno(),
            solar_x_env(),
            e_h,
            Fixed::from_int(HAYASHI_ANCHOR_SUPERGIANT_K),
            Fixed::ZERO,
            Fixed::from_int(HAYASHI_BAND_LOW_K),
            Fixed::from_int(HAYASHI_BAND_HIGH_K),
            t_max(),
        )
        .expect("the track point assembles");
        assert!(!pt.luminosity.convection_penetration);
        assert!(!pt.luminosity.below_fitted_range);
        let l = pt.luminosity.luminosity_lsun.to_f64_lossy();
        let t = pt.t_eff_k.to_f64_lossy();
        let r = pt.radius_rsun.to_f64_lossy();
        let dmdt = pt.core_growth_msun_per_myr.to_f64_lossy();
        assert!(
            (5.0e3..=2.0e4).contains(&l),
            "a bright red giant, got {l} L_sun"
        );
        assert!((3000.0..=4000.0).contains(&t), "red, got {t} K");
        assert!(r > 100.0, "a large red-giant radius, got {r} R_sun");
        assert!(dmdt > 0.0, "the core grows, got {dmdt} M_sun/Myr");
    }

    #[test]
    fn a_hydrogen_energy_yield_lands_its_measured_scale() {
        // E_H = fraction * c^2 / 1e14 ~ 0.00712 * (2.998e8)^2 / 1e14 ~ 6.4, the ~6.4e14 J/kg hydrogen-burning
        // specific yield, DERIVED from the cited mass-defect fraction and the fundamental c.
        let e_h = default_hydrogen_energy_yield_1e14().expect("derives");
        let v = e_h.to_f64_lossy();
        assert!(
            (6.0..=6.8).contains(&v),
            "the hydrogen energy yield is ~6.4e14 J/kg, got {v}e14"
        );
    }

    #[test]
    fn a_core_growth_clock_is_a_slow_solar_mass_per_megayear() {
        // dM_c/dt = L/(X_env*E_H): a ~1e4 L_sun giant grows its core at ~0.1 M_sun/Myr, a slow, positive,
        // representable rate.
        let e_h = default_hydrogen_energy_yield_1e14().expect("derives");
        let rate = core_growth_rate_msun_per_myr(Fixed::from_int(10_000), solar_x_env(), e_h)
            .expect("the clock derives");
        let v = rate.to_f64_lossy();
        assert!(
            (0.01..=1.0).contains(&v),
            "the core grows ~0.1 M_sun/Myr, got {v}"
        );
    }
}
