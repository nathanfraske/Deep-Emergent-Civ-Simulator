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

//! The disk-opacity GENERATOR (capstone front-end 3c-i, the owner-redirected build): the Rosseland-mean opacity
//! `kappa_R(T, rho)` DERIVED from physics rather than read from a fitted ladder. The owner held the Bell-Lin 1994
//! and Semenov 2003 piecewise fits permanently (they are solar-composition compressions that bake one grain model
//! and fixed regime boundaries into dimensional coefficients, violating the dimensionless-constant law and
//! admit-the-alien); Bell-Lin/Semenov re-enter only as the validation battery (`OPACITY_VALIDATION_BATTERY.md`).
//! The generator is `kappa_R = Rosseland . Mie . (optical constants x size distribution x mixing rule x condensate
//! fractions) + gas terms`, built as a multi-slice arc.
//!
//! This module holds the gas/plasma terms, which derive to the digit from the fundamentals with no fetched
//! coefficient. The first is ELECTRON SCATTERING ([`electron_scattering_opacity`]): the Thomson value the whole
//! ladder tops out at, reassembled from the Thomson cross section and a hydrogen mass fraction. The Mie grain
//! terms (which read the measured optical constants, `OPACITY_OPTICAL_CONSTANTS_SOURCES.md`) and the Rosseland
//! assembly are the later slices.

use crate::optical_constants::OpticalSpecies;
use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use civsim_units::bignum::{BigRat, BigUint};
use civsim_units::compute;
use civsim_units::constants::{self, SiExecutionMagnitudes};

/// The decimal digits of pi the opacity computation carries, far above the ~10 significant figures a Q32.32
/// result holds, so the pi truncation never reaches the result's low bit. An engine-accuracy bound.
const OPACITY_PI_DIGITS: u32 = 40;

/// A non-negative `Fixed` as an exact rational (its bits over `2^FRAC_BITS`), so an order-one `Fixed` argument
/// enters the wide-magnitude `BigRat` computation without leaving exact arithmetic. The callers pass non-negative
/// values (a mass fraction in `[0, 1]`, a molar mass).
fn nonneg_fixed_to_bigrat(value: Fixed) -> BigRat {
    let bits = value.to_bits().max(0) as u64;
    BigRat::new(
        false,
        BigUint::from_u64(bits),
        BigUint::from_u64(1).shl_bits(Fixed::FRAC_BITS),
    )
}

/// One sealed SI execution value as an exact rational. This resolves both
/// representation definitions and derived values such as `eps_0`; no consumer
/// reads a drift-oracle decimal or binds a replacement magnitude.
fn execution_bigrat(execution: &SiExecutionMagnitudes, symbol: &str) -> Option<BigRat> {
    Some(execution.get(symbol)?.exact_rational())
}

/// One noncausal SI representation value as an exact rational.
fn representation_bigrat(symbol: &str) -> Option<BigRat> {
    Some(
        constants::si_representation_magnitudes()
            .ok()?
            .get(symbol)?
            .exact_rational(),
    )
}

/// The ELECTRON-SCATTERING (Thomson) opacity `kappa_es` (cm^2/g), DERIVED from the fundamentals and the periodic
/// table, never fetched from an opacity fit. Free electrons scatter photons with the Thomson cross section, so the
/// opacity per unit mass is `kappa_es = sigma_T * n_e / rho = sigma_T * (1 + X) / (2 * m_H)`, where `X` is the
/// hydrogen mass fraction and the `(1 + X)/2` factor is the electrons per nucleon of an ionized hydrogen-helium
/// plasma (pure hydrogen contributes one electron per proton, helium two per four nucleons). It is constant in
/// density and temperature (the top regime of the opacity ladder).
///
/// Every constant is DERIVED, nothing fetched: the Thomson cross section is
/// `sigma_T = (8*pi/3) * r_e^2` with the classical electron radius `r_e = e^2 / (4*pi*eps_0*m_e*c^2)`, so it reads
/// only the fundamentals `e`, `eps_0`, `m_e`, `c` (the electron mass `m_e` is the eighth register fundamental,
/// reached by exactly this term); the mass per hydrogen `m_H = M_H / N_A` reads the periodic-table molar mass of
/// hydrogen and the Avogadro fundamental. So the Bell-Lin ladder's top value `0.348` (their `0.2(1 + X)` rounded)
/// is reassembled here from atomic principles as `0.1989(1 + X)`, the more precise coefficient. `X` is the only
/// per-world input (the admit-the-alien seam): a hydrogen-poor plasma is a data row, a lower `kappa_es`, never a
/// rewrite.
///
/// The wide-magnitude computation (`sigma_T ~ 6.65e-29 m^2` and `m_H ~ 1.67e-27 kg` underflow Q32.32 while the
/// ~0.35 cm^2/g result fits) runs in exact `BigRat` and rounds once; the `m^2/kg -> cm^2/g` conversion is the
/// exact factor 10. `None` if a fundamental or the hydrogen molar mass fails to resolve, or the result leaves the
/// representable range.
pub fn electron_scattering_opacity(
    execution: &SiExecutionMagnitudes,
    hydrogen_mass_fraction: Fixed,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let e = execution_bigrat(execution, "e")?;
    let eps_0 = execution_bigrat(execution, "eps_0")?;
    let m_e = execution_bigrat(execution, "m_e")?;
    let c = execution_bigrat(execution, "c")?;
    let n_a = execution_bigrat(execution, "N_A")?;
    let pi = compute::pi(OPACITY_PI_DIGITS);

    // The classical electron radius r_e = e^2 / (4*pi*eps_0*m_e*c^2), then the Thomson cross section
    // sigma_T = (8*pi/3) * r_e^2 (SI, m^2).
    let e2 = e.mul(&e);
    let four_pi_eps0 = BigRat::from_i64(4).mul(&pi).mul(&eps_0);
    let m_e_c2 = m_e.mul(&c).mul(&c);
    let r_e = e2.div(&four_pi_eps0.mul(&m_e_c2));
    let r_e2 = r_e.mul(&r_e);
    let sigma_t = BigRat::from_i64(8)
        .mul(&pi)
        .div(&BigRat::from_i64(3))
        .mul(&r_e2); // m^2

    // The mass per hydrogen m_H = M_H / N_A (kg): the periodic-table hydrogen molar mass (g/mol) to kg/mol, over
    // Avogadro. This reads the same molar-mass kernel the materials substrate uses, never an authored m_H.
    let m_h_g_per_mol = table.molar_mass_of(&[("H", 1)]).ok()?;
    let m_h_kg = nonneg_fixed_to_bigrat(m_h_g_per_mol)
        .div(&BigRat::from_i64(1000))
        .div(&n_a);

    // kappa_es = sigma_T * (1 + X) / (2 * m_H) [m^2/kg], then * 10 to cm^2/g (1 m^2/kg = 10 cm^2/g).
    let one_plus_x = BigRat::from_i64(1).add(&nonneg_fixed_to_bigrat(hydrogen_mass_fraction));
    let kappa_m2_per_kg = sigma_t
        .mul(&one_plus_x)
        .div(&BigRat::from_i64(2).mul(&m_h_kg));
    let kappa_cm2_per_g = kappa_m2_per_kg.mul(&BigRat::from_i64(10));

    let bits = kappa_cm2_per_g.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

/// `ln` of the Thomson cross section in cm^2, from the fundamentals in the log domain: `sigma_T ~6.65e-25 cm^2`
/// underflows `Fixed`, so the `n_e`-linear electron-scattering opacity carries it as its log. `sigma_T = (8 pi/3)
/// r_e^2`, `r_e = e^2/(4 pi eps_0 m_e c^2)`; `ln r_e` in cm folds the metre-to-cm scaling (`r_e * 100`) as `+2 ln 10`.
fn ln_thomson_cross_section_cm2(execution: &SiExecutionMagnitudes) -> Option<Fixed> {
    let ln_e = crate::saha::ln_fundamental(execution, "e")?;
    let ln_eps0 = crate::saha::ln_fundamental(execution, "eps_0")?;
    let ln_me = crate::saha::ln_fundamental(execution, "m_e")?;
    let ln_c = crate::saha::ln_fundamental(execution, "c")?;
    let ln_4pi = crate::saha::ln_of_decimal("12.56637061")?;
    let ln_8pi_3 = crate::saha::ln_of_decimal("8.37758041")?;
    let ln10 = Fixed::from_int(10).ln();
    // ln r_e in cm = [2 ln e - ln(4 pi) - ln eps_0 - ln m_e - 2 ln c] (SI, m) + 2 ln 10 (m -> cm).
    let ln_re_cm =
        Fixed::from_int(2).mul(ln_e) - ln_4pi - ln_eps0 - ln_me - Fixed::from_int(2).mul(ln_c)
            + Fixed::from_int(2).mul(ln10);
    Some(ln_8pi_3 + Fixed::from_int(2).mul(ln_re_cm))
}

/// The electron-scattering opacity `kappa_es = sigma_T * n_e / rho` (cm^2/g), LINEAR in the free-electron density
/// (electron scattering IS `sigma_T n_e / rho`; the fully-ionized `0.348(1+X)` was only that evaluated at full
/// ionization, so this computes from `n_e`, never patches the constant with a fraction). Consumes the SHARED Saha
/// `n_e` (as its log, cm^-3) and `ln rho` (g/cm^3), so es, ff, and H- read ONE electron density (the join law).
/// Computed in the log domain because `sigma_T * n_e` underflows `Fixed`: `ln kappa_es = ln sigma_T + ln n_e -
/// ln rho`, then one `exp`. At full ionization `n_e = (1+X) rho / (2 m_H)` it reproduces the restored electron-
/// scattering constant `0.348(1+X)` exactly, the general form provably containing its old limit (the reassembly
/// identity). `None` if a constant fails to load.
pub fn electron_scattering_opacity_from_electron_density(
    execution: &SiExecutionMagnitudes,
    ln_electron_density_cm3: Fixed,
    ln_density_g_cm3: Fixed,
) -> Option<Fixed> {
    let ln_kappa =
        ln_thomson_cross_section_cm2(execution)? + ln_electron_density_cm3 - ln_density_g_cm3;
    Some(ln_kappa.exp())
}

/// The number of quadrature intervals the Rosseland-mean integral takes: a FIXED count (the determinism bound, the
/// `SURFACE_BALANCE_ITERS` model), integer-only, no until-converged spin. Set so the Planck weighting is well
/// resolved across its peak and its tails past the bounds are negligible.
const ROSSELAND_INTERVALS: u32 = 512;

/// The dimensionless-frequency `x = h*nu/(k_B*T)` lower bound of the Rosseland integral. Below it the weighting
/// `w(x) ~ x^2` is negligible; `1/20` keeps the near-zero curvature resolved.
fn rosseland_x_min() -> Fixed {
    Fixed::from_ratio(1, 20)
}

/// The dimensionless-frequency upper bound. The weighting `w(x) ~ x^4 e^-x` is negligible past it, and it stays
/// well inside the `Fixed::exp` window (`e^-20` is representable) so no overflow-prone `e^x` ever forms.
fn rosseland_x_max() -> Fixed {
    Fixed::from_int(20)
}

/// The ROSSELAND weighting `w(x)` (proportional to `dB_nu/dT`, the temperature derivative of the Planck function)
/// in the dimensionless frequency `x = h*nu/(k_B*T)`: `w(x) = x^4 * e^-x / (1 - e^-x)^2`. Written with `e^-x`
/// (always in `(0, 1]` for `x >= 0`) rather than the algebraically-equal `x^4 e^x/(e^x-1)^2`, because `e^x`
/// overflows `Fixed` past `x ~ 21.5` while `e^-x` never does. Zero at `x <= 0`. `None` only on an arithmetic
/// overflow (the `x^4` stays small over the integration range).
fn rosseland_weight(x: Fixed) -> Option<Fixed> {
    if x <= Fixed::ZERO {
        return Some(Fixed::ZERO);
    }
    let e_neg_x = Fixed::ZERO.checked_sub(x)?.exp();
    let one_minus = Fixed::ONE.checked_sub(e_neg_x)?;
    if one_minus <= Fixed::ZERO {
        return Some(Fixed::ZERO);
    }
    let x2 = x.checked_mul(x)?;
    let x4 = x2.checked_mul(x2)?;
    x4.checked_mul(e_neg_x)?
        .checked_div(one_minus.checked_mul(one_minus)?)
}

/// The ROSSELAND-MEAN opacity `kappa_R` of a monochromatic opacity `kappa_nu` (the gate's determinism-critical
/// kernel, reused by every opacity term). The Rosseland mean is the harmonic mean of `kappa_nu` weighted by the
/// Planck temperature-derivative: `1/kappa_R = integral[(1/kappa_nu) w dx] / integral[w dx]`, so
/// `kappa_R = (sum w) / (sum w/kappa_nu)` over a BOUNDED fixed-count midpoint quadrature (no until-converged spin,
/// integer-only `Fixed`, so determinism holds by construction). `kappa_nu` is a function of the dimensionless
/// frequency `x = h*nu/(k_B*T)`, so this kernel is temperature-scale-free (the temperature dependence lives in the
/// caller's `kappa_nu(x)`).
///
/// STRICT-POSITIVITY PRECONDITION (fail-loud): `kappa_nu` must be positive across the whole range. The assembled
/// opacity always carries the electron-scattering floor (`kappa_nu >= kappa_es > 0`), so it holds by construction.
/// A `None` or non-positive `kappa_nu` at any quadrature point is an ERROR, not a transparent window (a real
/// transparent window is `kappa_nu -> 0+`, a small positive value the harmonic sum `w/kappa_nu` handles on its own
/// by blowing up so `kappa_R -> 0`), so this returns `None` rather than silently dropping the point. Dropping it
/// would leave `sum w` (over every `x`) and `sum w/kappa_nu` (over the kept `x`) summing over different point sets,
/// biasing `kappa_R` HIGH by the weight of the dropped point (worst near the `x ~ 3.83` peak). `None` if the
/// precondition is violated, the accumulation overflows, or no frequency contributes.
pub fn rosseland_mean(kappa_nu: impl Fn(Fixed) -> Option<Fixed>) -> Option<Fixed> {
    let x_min = rosseland_x_min();
    let dx = rosseland_x_max()
        .checked_sub(x_min)?
        .checked_div(Fixed::from_int(ROSSELAND_INTERVALS as i32))?;
    let half_dx = dx.checked_div(Fixed::from_int(2))?;
    let mut weight_sum = Fixed::ZERO;
    let mut harmonic_sum = Fixed::ZERO; // sum of w / kappa_nu
    for i in 0..ROSSELAND_INTERVALS {
        let x = x_min.checked_add(
            dx.checked_mul(Fixed::from_int(i as i32))?
                .checked_add(half_dx)?,
        )?;
        let w = rosseland_weight(x)?;
        weight_sum = weight_sum.checked_add(w)?;
        // Fail loud on the strict-positivity precondition: a None or non-positive kappa_nu is an error, not a
        // transparent window, so propagate None rather than drop the point and leave the numerator and
        // denominator summing over different point sets (which would bias kappa_R high).
        let k = kappa_nu(x)?;
        if k <= Fixed::ZERO {
            return None;
        }
        harmonic_sum = harmonic_sum.checked_add(w.checked_div(k)?)?;
    }
    if harmonic_sum <= Fixed::ZERO {
        return None;
    }
    weight_sum.checked_div(harmonic_sum)
}

/// The FREE-FREE spectral shape `f(x) = x^-3 * (1 - e^-x)` (dimensionless), the frequency dependence of the
/// bremsstrahlung absorption coefficient in the dimensionless frequency `x = h*nu/(k_B*T)`, with the `(1 - e^-x)`
/// factor its stimulated-emission correction. It is strictly positive on `(0, x_max]` (`x^-3 > 0` and
/// `1 - e^-x > 0`), so it meets the Rosseland kernel's strict-positivity precondition. `None` only on an arithmetic
/// overflow (the values stay near `x^-3 ~ 8000` at the low bound, well inside range). Zero-or-below `x` returns
/// `None` (out of the integration domain).
fn free_free_shape(x: Fixed) -> Option<Fixed> {
    if x <= Fixed::ZERO {
        return None;
    }
    let e_neg_x = Fixed::ZERO.checked_sub(x)?.exp();
    let one_minus = Fixed::ONE.checked_sub(e_neg_x)?;
    let x3 = x.checked_mul(x)?.checked_mul(x)?;
    one_minus.checked_div(x3)
}

/// The KRAMERS FREE-FREE (bremsstrahlung) Rosseland opacity `kappa_ff_ion` (cm^2/g), DERIVED from the fundamentals
/// through the shared Rosseland kernel, never read from the Bell-Lin fit. DEFINITION TAG: this is `kappa_ff_ion`,
/// an electron scattering off a POSITIVE ION, scaling as `n_e sum(Z_i^2 n_i)`; it is a DIFFERENT channel from the
/// H- free-free [`h_minus_free_free_opacity`] (`kappa_ff_Hminus`, an electron off a NEUTRAL hydrogen, scaling as
/// `(X/m_H) P_e`). Both are additive in the monochromatic sum but key off different densities and must never be
/// merged. A free electron accelerating past an ion radiates a continuum; the bound-free-corrected absorption
/// coefficient carries the spectral shape
/// [`free_free_shape`] `x^-3 (1 - e^-x)`, and its Rosseland mean is the classic
/// `kappa_ff = C_ff * (1+X) * <Z^2/A> * g_ff * rho * T^(-7/2)` (cgs). The whole point of the generator is that
/// `C_ff` is NOT the fitted `~3.68e22`: it reassembles from the bremsstrahlung prefactor and the kernel's Rosseland
/// average of the free-free shape, and LANDS inside the cited `[3.68e22, 3.8e22]` textbook envelope as a
/// consequence, never as an input.
///
/// Every constant is read from the register, nothing fetched:
/// - The monochromatic prefactor is the SI bremsstrahlung coefficient
///   `A = (4/(3 m_e h c)) * (e^2/(4*pi*eps_0))^3 * sqrt(2*pi/(3 k_B m_e))`, reading `e, eps_0, m_e, h, c, k_B` (the
///   Gaussian `e^6` becomes `(e^2/(4*pi*eps_0))^3` in SI). After substituting `nu = x k_B T/h` the temperature
///   power becomes `T^(-7/2)` and the dimensional prefactor is `pref = A * (h^3/k_B^3)/(2 m_u^2)`, with the atomic
///   mass unit `m_u = 1/(1000 N_A)` kg (the free-free composition reduction counts nucleons in `m_u`).
/// - `Phi = rosseland_mean(free_free_shape)` is DERIVED by the shared kernel (Rosseland-averaging the actual
///   free-free spectral shape), landing `~5.09e-3` (the closed form `(4*pi^4/15)/(2520*(zeta(6)+zeta(7)))`), so no
///   fitted `C_ff` is ever cited; `C_ff = 10^4 * pref * Phi` falls out inside the envelope.
/// - `kappa_ff = 10^4 * pref * Phi * (1+X) * charge_weighted_abundance * g_ff * rho * T^(-7/2)` (the `10^4` is `10`
///   for `m^2/kg -> cm^2/g` times `10^3` for `rho g/cm^3 -> kg/m^3`).
///
/// The wide-magnitude compute runs in exact `BigRat` and the single square root is taken LAST (the squaring trick:
/// `kappa_ff^2` is a clean rational because `sqrt(2*pi/(3 k_B m_e))` squares away and `T^(-7/2)` becomes `T^-7`,
/// and unlike the `~10^5` dimensional prefactor the result `kappa_ff^2 ~ 10^5` fits `Fixed`), then one
/// `Fixed::sqrt`. Every per-world quantity is a caller argument (the admit-the-alien seam): `hydrogen_mass_fraction`
/// X (the `1+X` electrons per nucleon of an ionized H-He plasma), `charge_weighted_abundance` `sum(Z_i^2 X_i/A_i)`
/// (the ion factor, `X+Y` for hydrogen-helium), and `gaunt_factor` g_ff (basis: the thermally-averaged free-free
/// Gaunt factor `~1.0 to 1.2` over the disk's temperature and frequency range, Rybicki and Lightman 1979). A
/// hydrogen-poor or metal-rich plasma is a data row, never a rewrite. `None` if a fundamental fails to resolve, the
/// kernel returns no `Phi`, or the result leaves the representable range (an extreme density or temperature whose
/// `kappa_ff^2` overflows `Fixed`).
pub fn kramers_free_free_opacity(
    execution: &SiExecutionMagnitudes,
    density_g_per_cm3: Fixed,
    temperature_k: Fixed,
    hydrogen_mass_fraction: Fixed,
    charge_weighted_abundance: Fixed,
    gaunt_factor: Fixed,
) -> Option<Fixed> {
    let e = execution_bigrat(execution, "e")?;
    let eps_0 = execution_bigrat(execution, "eps_0")?;
    let m_e = execution_bigrat(execution, "m_e")?;
    let h = execution_bigrat(execution, "h")?;
    let c = execution_bigrat(execution, "c")?;
    let k_b = execution_bigrat(execution, "k_B")?;
    let n_a = execution_bigrat(execution, "N_A")?;
    let pi = compute::pi(OPACITY_PI_DIGITS);

    // The Coulomb-squared charge alpha_c = e^2/(4*pi*eps_0) (the SI stand-in for the Gaussian e^2), and the atomic
    // mass unit m_u = 1/(1000 N_A) kg (1 g/mol over Avogadro).
    let alpha_c = e.mul(&e).div(&BigRat::from_i64(4).mul(&pi).mul(&eps_0));
    let m_u = BigRat::from_i64(1).div(&BigRat::from_i64(1000).mul(&n_a));

    // RAT: the pure-rational part of the cgs prefactor, 10^4 * (4 alpha_c^3/(3 m_e h c)) * (h^3/k_B^3)/(2 m_u^2).
    // SQ: the part under the single square root, 2*pi/(3 k_B m_e). So pref_cgs = RAT * sqrt(SQ).
    let alpha_c3 = alpha_c.mul(&alpha_c).mul(&alpha_c);
    let brems = BigRat::from_i64(4)
        .mul(&alpha_c3)
        .div(&BigRat::from_i64(3).mul(&m_e).mul(&h).mul(&c));
    let h3 = h.mul(&h).mul(&h);
    let kb3 = k_b.mul(&k_b).mul(&k_b);
    let temp_prefactor = h3.div(&kb3).div(&BigRat::from_i64(2).mul(&m_u).mul(&m_u));
    let rat = BigRat::from_i64(10000).mul(&brems).mul(&temp_prefactor);
    let sq = BigRat::from_i64(2)
        .mul(&pi)
        .div(&BigRat::from_i64(3).mul(&k_b).mul(&m_e));

    // Phi, the Rosseland mean of the free-free spectral shape, DERIVED by the shared kernel (never a cited C_ff).
    let phi = nonneg_fixed_to_bigrat(rosseland_mean(free_free_shape)?);

    // The composition and state factors, all caller-supplied (admit-the-alien): comp = (1+X) * <Z^2/A>.
    let comp = BigRat::from_i64(1)
        .add(&nonneg_fixed_to_bigrat(hydrogen_mass_fraction))
        .mul(&nonneg_fixed_to_bigrat(charge_weighted_abundance));
    let rho = nonneg_fixed_to_bigrat(density_g_per_cm3);
    let g = nonneg_fixed_to_bigrat(gaunt_factor);
    let t = nonneg_fixed_to_bigrat(temperature_k);

    // kappa_ff^2 = RAT^2 * SQ * Phi^2 * comp^2 * rho^2 * g^2 * T^-7 (the squaring removes both sqrt(SQ) and the
    // T^(-1/2), leaving a clean rational), then a single Fixed::sqrt. T^7 = T^4 * T^2 * T. The whole product is
    // formed together so the small ~kappa_ff^2 never forms the overflowing A_ff^2 alone.
    let t2 = t.mul(&t);
    let t7 = t2.mul(&t2).mul(&t2).mul(&t);
    let kappa_squared = rat
        .mul(&rat)
        .mul(&sq)
        .mul(&phi)
        .mul(&phi)
        .mul(&comp)
        .mul(&comp)
        .mul(&rho)
        .mul(&rho)
        .mul(&g)
        .mul(&g)
        .div(&t7);

    let bits = kappa_squared.round_to_scale(Fixed::FRAC_BITS)?;
    Some(Fixed::from_bits_i128(bits)?.sqrt())
}

/// The frequency-INDEPENDENT free-free (bremsstrahlung) opacity prefactor `A_ff` (cgs), so the monochromatic
/// H-plasma free-free opacity is `kappa_ff_nu(x) = A_ff * free_free_shape(x)`, which the total-opacity assembly
/// sums with the other monochromatic terms BEFORE Rosseland-averaging (the Rosseland mean is not additive, so the
/// per-term means cannot be summed). It is recovered as `A_ff = kappa_ff / Phi` from the Rosseland free-free
/// ([`kramers_free_free_opacity`]) and `Phi = rosseland_mean(free_free_shape)`, because the Rosseland mean is
/// homogeneous of degree one (`rosseland_mean(A_ff * shape) = A_ff * Phi`). Recovered this way rather than from
/// `sqrt(A_ff^2)` because `A_ff ~ 1e5` overflows Q32.32 when squared, while `A_ff` and `kappa_ff` are both
/// representable: `kramers` forms the small `kappa_ff^2 = A_ff^2 * Phi^2` together and never `A_ff^2` alone.
/// `None` if the free-free or the shape mean fails to resolve.
fn free_free_prefactor(
    execution: &SiExecutionMagnitudes,
    density_g_per_cm3: Fixed,
    temperature_k: Fixed,
    hydrogen_mass_fraction: Fixed,
    charge_weighted_abundance: Fixed,
    gaunt_factor: Fixed,
) -> Option<Fixed> {
    let phi = rosseland_mean(free_free_shape)?;
    let kappa_ff = kramers_free_free_opacity(
        execution,
        density_g_per_cm3,
        temperature_k,
        hydrogen_mass_fraction,
        charge_weighted_abundance,
        gaunt_factor,
    )?;
    kappa_ff.checked_div(phi)
}

/// The free-free monochromatic prefactor `A_ff` for a PARTIALLY IONIZED plasma: free-free is a two-body
/// electron-ion process, so it carries the PRODUCT `n_e * sum(Z_i^2 n_i)` (QUADRATIC in ionization fraction, since
/// for hydrogen both the electron and the ion side track the ionized fraction `x`), where [`free_free_prefactor`]'s
/// fully-ionized coefficient assumed every nucleon donated its charge. Both densities are the SHARED Saha output
/// (the join law: es, ff, and H- read one `n_e`). Under SINGLE-STAGE ionization `sum(Z_i^2 n_i) = n_e` (every ion
/// has `Z = 1`), so the ion side collapses to `n_e` and `A_ff ~ n_e^2`; carrying the general `sum(Z_i^2 n_i)`
/// argument keeps a multi-stage plasma (a doubly-ionized species, `Z = 2`) from silently regressing to the
/// single-charge form. The Gaunt factor `g_ff` is the caller's plasma datum, physical band `~1.1 to 1.5` over disk
/// temperatures and frequencies (van Hoof et al. 2014), never fabricated here.
///
/// Computed as `A_ff_full * (n_e * sum Z^2 n_i)_actual / (n_e * sum Z^2 n_i)_full`, a DIMENSIONLESS ratio (every
/// unit cancels) formed in the log domain because the number densities (`~1e17 cm^-3`) overflow Q32.32. The
/// fully-ionized reference densities are `n_e_full = (1+X)/2 * rho/m_u` and `sum Z^2 n_i_full = <Z^2/A> * rho/m_u`
/// with `m_u = 1/N_A` g (so `-ln m_u = +ln N_A`). At full ionization the ratio is one and `A_ff` reproduces
/// [`free_free_prefactor`] exactly, the general `n_e^2` form provably containing the Kramers limit (the reassembly
/// identity). `None` if the full-ionization prefactor fails to resolve or a constant fails to load.
#[allow(clippy::too_many_arguments)]
fn free_free_prefactor_ionized(
    execution: &SiExecutionMagnitudes,
    ln_electron_density_cm3: Fixed,
    ln_sum_z2_ni_cm3: Fixed,
    ln_density_g_cm3: Fixed,
    density_g_per_cm3: Fixed,
    temperature_k: Fixed,
    hydrogen_mass_fraction: Fixed,
    charge_weighted_abundance: Fixed,
    gaunt_factor: Fixed,
) -> Option<Fixed> {
    let a_ff_full = free_free_prefactor(
        execution,
        density_g_per_cm3,
        temperature_k,
        hydrogen_mass_fraction,
        charge_weighted_abundance,
        gaunt_factor,
    )?;
    let ln_na = crate::saha::ln_fundamental(execution, "N_A")?;
    // ln of the fully-ionized number densities (cm^-3): n_e_full = (1+X)/2 * rho/m_u, sum Z^2 n_i_full =
    // <Z^2/A> * rho/m_u, m_u = 1/N_A g so the -ln m_u is a +ln N_A.
    let ln_half_1px = Fixed::ONE
        .checked_add(hydrogen_mass_fraction)?
        .checked_div(Fixed::from_int(2))?
        .ln();
    let ln_cwa = charge_weighted_abundance.ln();
    let ln_ne_full = ln_half_1px
        .checked_add(ln_density_g_cm3)?
        .checked_add(ln_na)?;
    let ln_sum_full = ln_cwa.checked_add(ln_density_g_cm3)?.checked_add(ln_na)?;
    let ln_ratio = ln_electron_density_cm3
        .checked_add(ln_sum_z2_ni_cm3)?
        .checked_sub(ln_ne_full)?
        .checked_sub(ln_sum_full)?;
    a_ff_full.checked_mul(ln_ratio.exp())
}

/// The TOTAL ionized-gas Rosseland-mean opacity `kappa_R` (cm^2/g): the assembly of the three gas terms into the
/// single opacity the disk reads, `kappa_R = rosseland_mean(kappa_es + kappa_H-(x) + kappa_ff(x))`. The
/// MONOCHROMATIC terms sum at each dimensionless frequency `x`, then the whole is Rosseland-averaged: the Rosseland
/// (harmonic) mean is NOT additive, so the per-term Rosseland means cannot be summed, the monochromatic opacities
/// must be.
///
/// THE JOIN LAW: all three terms read ONE electron density from ONE Saha solve. Electron scattering is
/// [`electron_scattering_opacity_from_electron_density`] (`sigma_T n_e/rho`, linear in `n_e`); free-free is
/// `A_ff * free_free_shape(x)` with the partial-ionization prefactor [`free_free_prefactor_ionized`] (the
/// `n_e sum Z^2 n_i` product); H- ([`h_minus_opacity`]) reads the same solve's electron pressure `P_e`. Passing
/// three implicit electron densities to three terms is the definition-mismatch class this assembly legislates
/// against.
///
/// Because electron scattering is now `n_e`-linear (not the grey `0.348(1+X)` constant), the grey POSITIVE FLOOR
/// is gone: in cold weakly-ionized gas `n_e -> 0`, es and ff vanish and H- sleeps, so the summed monochromatic
/// opacity reaches zero and the strict-positivity Rosseland kernel returns `None`. That is the physical singularity
/// (grains sublimated ~1500 K, H- not yet risen ~2500 K) whose occupant is MOLECULAR opacity, supplied by the
/// Ferguson regime handoff, NOT by this ionized-gas assembly. This function is therefore the HOT-REGIME closure;
/// its `None` in the cold gap is the handoff signal, not a failure. `ln n_e`, `ln sum Z^2 n_i`, `ln rho`, and
/// `P_e` are the Saha state; `T`, `rho`, `X`, `<Z^2/A>`, and the Gaunt factor are the plasma data (admit-the-alien).
/// `None` if a term or the mean fails to resolve (including the cold molecular gap).
#[allow(clippy::too_many_arguments)]
pub fn total_gas_rosseland_opacity(
    execution: &SiExecutionMagnitudes,
    temperature_k: Fixed,
    density_g_per_cm3: Fixed,
    ln_density_g_cm3: Fixed,
    hydrogen_mass_fraction: Fixed,
    charge_weighted_abundance: Fixed,
    gaunt_factor: Fixed,
    ln_electron_density_cm3: Fixed,
    ln_sum_z2_ni_cm3: Fixed,
    electron_pressure_dyn_cm2: Fixed,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let kappa_es = electron_scattering_opacity_from_electron_density(
        execution,
        ln_electron_density_cm3,
        ln_density_g_cm3,
    )?;
    let a_ff = free_free_prefactor_ionized(
        execution,
        ln_electron_density_cm3,
        ln_sum_z2_ni_cm3,
        ln_density_g_cm3,
        density_g_per_cm3,
        temperature_k,
        hydrogen_mass_fraction,
        charge_weighted_abundance,
        gaunt_factor,
    )?;
    rosseland_mean(|x| {
        let ff = a_ff.checked_mul(free_free_shape(x)?)?;
        let hm = h_minus_opacity(
            execution,
            x,
            temperature_k,
            hydrogen_mass_fraction,
            electron_pressure_dyn_cm2,
            table,
        )?;
        kappa_es.checked_add(ff)?.checked_add(hm)
    })
}

/// The TOTAL gas-plus-grain Rosseland-mean opacity `kappa_R` (cm^2/g): [`total_gas_rosseland_opacity`] with the
/// monochromatic GRAIN term joined into the same sum, `kappa_R = rosseland_mean(kappa_es + kappa_H-(x) +
/// kappa_ff(x) + kappa_grain(x))`. The grain opacity is supplied as a closure `grain_kappa_at(lambda_um)` (the
/// materials-crate wire builds it from the realized condensate assemblage: Rule-1 optical dispatch, Rule-2
/// effective-medium topology, Rule-3 shared size distribution, then [`grain_size_averaged_opacity`] at each
/// wavelength). Keeping the grain term a closure keeps this physics primitive composition-agnostic and off the
/// dependency cycle (physics does not depend on materials).
///
/// The sum is MONOCHROMATIC, then Rosseland-averaged: the harmonic mean is not additive, so a caller MUST NOT
/// Rosseland-average gas and grains separately and add the two means. Adding the grain term also repairs the cold
/// molecular gap for a condensed disk: where es and ff are Saha-killed and H- sleeps, [`total_gas_rosseland_opacity`]
/// returns `None` (its handoff signal), but grains at the ice line carry the opacity, so the summed monochromatic
/// opacity is positive and the mean resolves. The ice-line opacity CLIFF is therefore an emergent output of this
/// sum (grains present below the front dominate the budget, absent above it), never a coded regime boundary. The
/// gas arguments are exactly [`total_gas_rosseland_opacity`]'s; `None` if a term or the mean fails to resolve.
#[allow(clippy::too_many_arguments)]
pub fn total_gas_and_grain_rosseland_opacity(
    execution: &SiExecutionMagnitudes,
    temperature_k: Fixed,
    density_g_per_cm3: Fixed,
    ln_density_g_cm3: Fixed,
    hydrogen_mass_fraction: Fixed,
    charge_weighted_abundance: Fixed,
    gaunt_factor: Fixed,
    ln_electron_density_cm3: Fixed,
    ln_sum_z2_ni_cm3: Fixed,
    electron_pressure_dyn_cm2: Fixed,
    table: &PeriodicTable,
    grain_kappa_at: impl Fn(Fixed) -> Option<Fixed>,
) -> Option<Fixed> {
    let kappa_es = electron_scattering_opacity_from_electron_density(
        execution,
        ln_electron_density_cm3,
        ln_density_g_cm3,
    )?;
    let a_ff = free_free_prefactor_ionized(
        execution,
        ln_electron_density_cm3,
        ln_sum_z2_ni_cm3,
        ln_density_g_cm3,
        density_g_per_cm3,
        temperature_k,
        hydrogen_mass_fraction,
        charge_weighted_abundance,
        gaunt_factor,
    )?;
    // lambda(x, T) = (h c / k_B) / (x T), in microns: the wavelength the grain closure is priced at.
    let h = representation_bigrat("h")?;
    let c = representation_bigrat("c")?;
    let k_b = representation_bigrat("k_B")?;
    let alpha_um_k = h.mul(&c).div(&k_b).mul(&BigRat::from_i64(1_000_000));
    let t_br = nonneg_fixed_to_bigrat(temperature_k);

    // Precompute the monochromatic grain opacity on the coarse Rosseland-window grid (the same grid the grain-only
    // spectral mean uses), so the possibly-expensive grain closure is evaluated GRAIN_ROSS_GRID+1 times, not once
    // per Rosseland quadrature point (the disk fixed-point solve calls this repeatedly, so the closure cost must
    // not multiply by ROSSELAND_INTERVALS). The grain term is then interpolated LINEARLY in ln(x) at each
    // quadrature frequency: linear rather than log-log because the additive grain term is zero where no grains are
    // present (ln 0 is undefined), and the gas terms stay exact per point.
    let ln_x_min = rosseland_x_min().ln();
    let ln_x_max = rosseland_x_max().ln();
    let span = ln_x_max.checked_sub(ln_x_min)?;
    let mut grain_grid = Vec::with_capacity(GRAIN_ROSS_GRID + 1);
    for j in 0..=GRAIN_ROSS_GRID {
        let ln_x = ln_x_min
            .checked_add(span.checked_mul(Fixed::from_ratio(j as i64, GRAIN_ROSS_GRID as i64))?)?;
        let x = ln_x.exp();
        let lam_br = alpha_um_k.div(&nonneg_fixed_to_bigrat(x).mul(&t_br));
        let lam = Fixed::from_bits_i128(lam_br.round_to_scale(Fixed::FRAC_BITS)?)?;
        grain_grid.push(grain_kappa_at(lam)?);
    }

    rosseland_mean(|x| {
        let ff = a_ff.checked_mul(free_free_shape(x)?)?;
        let hm = h_minus_opacity(
            execution,
            x,
            temperature_k,
            hydrogen_mass_fraction,
            electron_pressure_dyn_cm2,
            table,
        )?;
        // Interpolate the precomputed grain term linearly in ln(x), clamped to the grid endpoints.
        let ln_x = x.ln();
        let pos = ln_x
            .checked_sub(ln_x_min)?
            .checked_div(span)?
            .checked_mul(Fixed::from_int(GRAIN_ROSS_GRID as i32))?;
        let pos = if pos < Fixed::ZERO {
            Fixed::ZERO
        } else if pos > Fixed::from_int(GRAIN_ROSS_GRID as i32) {
            Fixed::from_int(GRAIN_ROSS_GRID as i32)
        } else {
            pos
        };
        let idx = (pos.to_int() as usize).min(GRAIN_ROSS_GRID - 1);
        let local = pos.checked_sub(Fixed::from_int(idx as i32))?;
        let lo = grain_grid[idx];
        let hi = grain_grid[idx + 1];
        let grain = lo.checked_add(hi.checked_sub(lo)?.checked_mul(local)?)?;
        kappa_es
            .checked_add(ff)?
            .checked_add(hm)?
            .checked_add(grain)
    })
}

/// The John 1988 photodetachment threshold wavelength `lambda_0` (micron) of the H- bound-free cross-section fit:
/// the fit's internal threshold, implying a binding `hc/lambda_0 = 0.7551 eV`, ~1 meV above the measured H- electron
/// affinity (`0.754 eV`, read from the periodic table for the physical Saha binding). It stays here only inside the
/// cross-section polynomial, per the fit's construction.
fn h_minus_bf_lambda0_um() -> Fixed {
    Fixed::from_ratio(16419, 10000)
}

/// The lower wavelength bound of the John 1988 H- bound-free fit domain (micron): below `0.125` the fit is not
/// valid (the far-UV), so the cross-section reads zero there.
fn h_minus_bf_lambda_lo_um() -> Fixed {
    Fixed::from_ratio(125, 1000)
}

/// The John 1988 (A&A 193, 189, eq. 5) polynomial coefficients `C_1..C_6` of the H- bound-free (photodetachment)
/// cross section, the compact representation of the primary Wishart 1979 (MNRAS 187, 59P) close-coupling computed
/// cross section (H- is bound only by electron correlation, so the two-electron calculation IS the physics, not
/// derivable at this level; this is the measured `[M]` tier, like the grain optical constants).
///
/// PROVENANCE (tier-honest, the standard met not bent): [Wishart 1979 primary computed cross-section; John 1988 A&A
/// 193 189 eq.5 fit; cross-validated 5-code open-source transcription; peak-validated 3.99e-17 at 8513A]. The John
/// PDF and the Wishart table are paywalled and did not parse; these coefficients are transcribed byte-identical from
/// five independent open codes (pyratbay, BeAR, Transparency.jl, and two more) each citing John 1988, and are
/// re-validated by the peak-reproduction gate in the tests (`the_h_minus_cross_section_reproduces_the_wishart_peak`),
/// the standing physics check that any corruption of these numbers fails the build. OWNER UPGRADE PATH: if the
/// primary Wishart 1979 / John 1988 PDF becomes reachable it swaps in verbatim and the tier rises; the peak gate
/// predicts the coefficients do not move.
fn h_minus_bf_coefficients() -> [Fixed; 6] {
    [
        Fixed::from_ratio(152519, 1000),
        Fixed::from_ratio(49534, 1000),
        Fixed::from_ratio(-118858, 1000),
        Fixed::from_ratio(92536, 1000),
        Fixed::from_ratio(-34194, 1000),
        Fixed::from_ratio(4982, 1000),
    ]
}

/// The H- BOUND-FREE (photodetachment) cross section at wavelength `lambda_um` (micron), in REDUCED units of
/// `1e-18 cm^2` (the bare `~4e-17 cm^2` value underflows Q32.32, so the `1e-18` is applied downstream in `BigRat`).
/// John 1988 eq. 5, `sigma_bf = 1e-18 * lambda^3 * (1/lambda - 1/lambda_0)^(3/2) * sum_{n=1..6} C_n (1/lambda -
/// 1/lambda_0)^((n-1)/2)`, reformulated as a plain polynomial in `g = sqrt(1/lambda - 1/lambda_0)` so a single
/// `Fixed::sqrt` serves all the half-integer powers: `sigma_bf/1e-18 = lambda^3 * sum_{n=0..5} C_n g^(n+3)`,
/// evaluated by Horner (which keeps the intermediate magnitudes near the coefficient scale rather than the
/// cancellation-prone `x1800` term scale). Zero outside the fit domain `[0.125, lambda_0]` micron (no
/// photodetachment below the binding threshold `lambda_0`, and the fit is undefined in the far UV).
///
/// PEAK GATE: reproduces the primary Wishart peak `3.994e-17 cm^2` (reduced `39.94`) at `8513 A`, the standing
/// physics check that the cited coefficients are faithful (see the module provenance note). `None` on overflow.
pub fn h_minus_bound_free_reduced_cross_section(lambda_um: Fixed) -> Option<Fixed> {
    let lambda0 = h_minus_bf_lambda0_um();
    if lambda_um < h_minus_bf_lambda_lo_um() || lambda_um >= lambda0 {
        return Some(Fixed::ZERO); // no photodetachment outside the fit domain
    }
    let inv_lambda = Fixed::ONE.checked_div(lambda_um)?;
    let inv_lambda0 = Fixed::ONE.checked_div(lambda0)?;
    let f = inv_lambda.checked_sub(inv_lambda0)?;
    if f <= Fixed::ZERO {
        return Some(Fixed::ZERO);
    }
    let g = f.sqrt();
    // Horner of sum_{n=0..5} C_n g^n = C_0 + g(C_1 + g(C_2 + ... + g*C_5)), then * g^3 * lambda^3.
    let c = h_minus_bf_coefficients();
    let mut poly = c[5];
    for coefficient in c.iter().take(5).rev() {
        poly = poly.checked_mul(g)?.checked_add(*coefficient)?;
    }
    let g3 = g.checked_mul(g)?.checked_mul(g)?;
    let lambda3 = lambda_um.checked_mul(lambda_um)?.checked_mul(lambda_um)?;
    lambda3.checked_mul(g3)?.checked_mul(poly)
}

/// The monochromatic H- BOUND-FREE opacity `kappa_bf` (cm^2/g) at dimensionless frequency `x = h*nu/(k_B*T)`,
/// DERIVED from the fundamentals, the periodic table, and the cited Wishart/John cross section, never fetched as a
/// whole. A neutral hydrogen atom binds a second electron (H-) only through electron correlation, and a photon
/// above the binding threshold photodetaches it, absorbing a continuum. The opacity is the Saha population of H-
/// per unit electron pressure, times the cross section, times the stimulated-emission correction:
///   `kappa_bf = (1/4)(h^2/2pi m_e)^(3/2) k^(-5/2) * T^(-5/2) * exp(chi/kT) * (1 - e^-x) * sigma_bf(lambda) *
///              (X / m_H) * P_e`   [cgs, per John 1988 eq. 3]
/// with `lambda = (h c/k)/(x T)` the wavelength at `x`. Everything derivable DERIVES: the Saha prefactor is the John
/// 1988 `0.750` reassembled from the fundamentals as the more precise `0.74989`, the binding `chi` reads the
/// periodic-table H electron affinity (`0.754 eV`), and `h c/k` is the wavelength fold. The one fetched piece is
/// `sigma_bf` ([`h_minus_bound_free_reduced_cross_section`], the cited `[M]` tier). The electron pressure `P_e`
/// (dyn/cm^2) is the caller's plasma variable rather than the electron density `n_e`, because `P_e = n_e k T` stays
/// in the representable range (`~1` to `1e5`) where `n_e ~ 1e13` overflows `Fixed`; this is exactly why the stellar-
/// atmosphere form is per-`P_e`.
///
/// The wide-magnitude compute runs in exact `BigRat` with the SQUARING trick (`kappa^2` is a clean rational because
/// both the `(h^2/2pi m_e)^(3/2)` and the `T^(-5/2)` half-integer powers square away to `T^-5`), then one
/// `Fixed::sqrt`; the two `exp` factors and the cross section are `Fixed` (order one) folded in as `BigRat`. Every
/// per-world quantity is a caller argument (admit-the-alien): `x`, `T`, `hydrogen_mass_fraction` X, and
/// `electron_pressure_dyn_cm2` `P_e` (a cool disk has `P_e -> 0` so H- vanishes, correct). Returns zero below the
/// photodetachment threshold (where `sigma_bf = 0`, the transparent window the free-free term fills at assembly).
/// HONEST LIMIT: valid for `T > ~410 K` (below it `chi/kT` leaves the `Fixed::exp` window and `exp(chi/kT)`
/// saturates, but H- needs free electrons a `<410 K` gas does not supply, so `P_e -> 0` makes the term vanish there
/// anyway). `None` if a fundamental or the hydrogen data fails to resolve, or the result leaves the representable
/// range.
pub fn h_minus_bound_free_opacity(
    execution: &SiExecutionMagnitudes,
    x: Fixed,
    temperature_k: Fixed,
    hydrogen_mass_fraction: Fixed,
    electron_pressure_dyn_cm2: Fixed,
    table: &PeriodicTable,
) -> Option<Fixed> {
    if x <= Fixed::ZERO || temperature_k <= Fixed::ZERO {
        return None;
    }
    let h = execution_bigrat(execution, "h")?;
    let c = execution_bigrat(execution, "c")?;
    let k_b = execution_bigrat(execution, "k_B")?;
    let m_e = execution_bigrat(execution, "m_e")?;
    let e = execution_bigrat(execution, "e")?;
    let n_a = execution_bigrat(execution, "N_A")?;
    let pi = compute::pi(OPACITY_PI_DIGITS);

    // The wavelength fold lambda = (h c/k)/(x T) [micron]: h c/k in SI is m*K, *1e6 -> micron*K.
    let alpha_um_k = h.mul(&c).div(&k_b).mul(&BigRat::from_i64(1_000_000));
    let t_br = nonneg_fixed_to_bigrat(temperature_k);
    let lambda_br = alpha_um_k.div(&nonneg_fixed_to_bigrat(x).mul(&t_br));
    let lambda = Fixed::from_bits_i128(lambda_br.round_to_scale(Fixed::FRAC_BITS)?)?;
    let sigma = h_minus_bound_free_reduced_cross_section(lambda)?;
    if sigma <= Fixed::ZERO {
        return Some(Fixed::ZERO); // below the photodetachment threshold, no bound-free opacity
    }

    // The stimulated-emission bracket (1 - e^-x) and the Saha Boltzmann factor exp(chi/kT), both via Fixed::exp:
    // chi/kT = affinity_eV * (e/k_B) / T, with e/k_B = e_SI/k_SI (K/eV) reading the affinity from the periodic table.
    let one_minus_e = Fixed::ONE.checked_sub(Fixed::ZERO.checked_sub(x)?.exp())?;
    let affinity = table.element("H")?.electron_affinity?;
    let e_over_k = Fixed::from_bits_i128(e.div(&k_b).round_to_scale(Fixed::FRAC_BITS)?)?;
    let exp_chi = affinity
        .checked_mul(e_over_k)?
        .checked_div(temperature_k)?
        .exp();

    // kappa^2 = C^2 * T^-5 * exp_chi^2 * (1-e^-x)^2 * (sigma*1e-18)^2 * (X/m_H)^2 * P_e^2 (cgs), then Fixed::sqrt.
    // C^2 = (1/16)(h_cgs^2/(2pi m_e_cgs))^3 k_cgs^-5 (the John 0.74989 squared); cgs = SI * (1e7 for h,k erg;
    // 1e3 for m_e g). The two half-integer powers square away.
    let h_cgs = h.mul(&BigRat::from_i64(10_000_000));
    let m_e_cgs = m_e.mul(&BigRat::from_i64(1000));
    let k_cgs = k_b.mul(&BigRat::from_i64(10_000_000));
    let base = h_cgs
        .mul(&h_cgs)
        .div(&BigRat::from_i64(2).mul(&pi).mul(&m_e_cgs));
    let base3 = base.mul(&base).mul(&base);
    let k_cgs5 = k_cgs.mul(&k_cgs).mul(&k_cgs).mul(&k_cgs).mul(&k_cgs);
    let c_squared = base3.div(&BigRat::from_i64(16).mul(&k_cgs5));

    let m_h_cgs = nonneg_fixed_to_bigrat(table.molar_mass_of(&[("H", 1)]).ok()?).div(&n_a); // g
    let x_over_mh = nonneg_fixed_to_bigrat(hydrogen_mass_fraction).div(&m_h_cgs);
    let p_e = nonneg_fixed_to_bigrat(electron_pressure_dyn_cm2);
    let sigma_br = nonneg_fixed_to_bigrat(sigma);
    let exp_chi_br = nonneg_fixed_to_bigrat(exp_chi);
    let one_minus_br = nonneg_fixed_to_bigrat(one_minus_e);
    let inv_1e18 = BigRat::from_i64(1).div(&BigRat::from_i64(1_000_000_000_000_000_000));
    let t5 = t_br.mul(&t_br).mul(&t_br).mul(&t_br).mul(&t_br);

    let kappa_squared = c_squared
        .mul(&exp_chi_br)
        .mul(&exp_chi_br)
        .mul(&one_minus_br)
        .mul(&one_minus_br)
        .mul(&sigma_br)
        .mul(&sigma_br)
        .mul(&inv_1e18)
        .mul(&inv_1e18) // (1e-18)^2 = 1e-36
        .mul(&x_over_mh)
        .mul(&x_over_mh)
        .mul(&p_e)
        .mul(&p_e)
        .div(&t5);
    Some(Fixed::from_bits_i128(kappa_squared.round_to_scale(Fixed::FRAC_BITS)?)?.sqrt())
}

/// The John 1988 (A&A 193, 189, eq. 6) H- FREE-FREE absorption-coefficient fit, REGION 1 (`lambda >= 0.3645
/// micron`). Each entry is the temperature index `n` and the six polynomial coefficients `[A, B, C, D, E, F]` of
/// `A*lambda^2 + B + C/lambda + D/lambda^2 + E/lambda^3 + F/lambda^4` (lambda in micron), summed as
/// `kappa_ff = 1e-29 * sum_n (5040/T)^((n+1)/2) * poly_n(lambda)` (cm^4/dyn, per neutral H per electron pressure).
/// The fit represents the primary Bell & Berrington 1987 (J.Phys.B 20, 801) R-matrix free-free calculation.
///
/// PROVENANCE (tier-honest, the same `[M]` secondary-transcription class ruled load-able as the bound-free): [Bell &
/// Berrington 1987 primary R-matrix free-free; John 1988 A&A 193 189 eq.6 fit; cross-validated pyratbay+BeAR
/// open-source transcription; validated via the assembled H- opacity in the free-free-dominated regime]. The
/// Bell-Berrington table and the John PDF are paywalled, so these coefficients are byte-identical across the two
/// independent open codes; the standing validation is the assembled bf+ff H- opacity against a primary-citable
/// benchmark in the pure-free-free regime `lambda > 1.6419 micron`, where the bound-free is exactly zero.
fn h_minus_ff_region1() -> [(i32, [&'static str; 6]); 5] {
    [
        (
            2,
            [
                "2483.346",
                "285.827",
                "-2054.291",
                "2827.776",
                "-1341.537",
                "208.952",
            ],
        ),
        (
            3,
            [
                "-3449.889",
                "-1158.382",
                "8746.523",
                "-11485.632",
                "5303.609",
                "-812.939",
            ],
        ),
        (
            4,
            [
                "2200.040",
                "2427.719",
                "-13651.105",
                "16755.524",
                "-7510.494",
                "1132.738",
            ],
        ),
        (
            5,
            [
                "-696.271",
                "-1841.400",
                "8624.970",
                "-10051.530",
                "4400.067",
                "-655.020",
            ],
        ),
        (
            6,
            [
                "88.283",
                "444.517",
                "-1863.864",
                "2095.288",
                "-901.788",
                "132.985",
            ],
        ),
    ]
}

/// The John 1988 eq. 6 H- free-free fit, REGION 2 (`0.1823 <= lambda < 0.3645 micron`), `n = 1..4`. Same form,
/// units, and provenance as [`h_minus_ff_region1`].
fn h_minus_ff_region2() -> [(i32, [&'static str; 6]); 4] {
    [
        (
            1,
            [
                "518.1021",
                "-734.8666",
                "1021.1775",
                "-479.0721",
                "93.1373",
                "-6.4285",
            ],
        ),
        (
            2,
            [
                "473.2636",
                "1443.4137",
                "-1977.3395",
                "922.3575",
                "-178.9275",
                "12.3600",
            ],
        ),
        (
            3,
            [
                "-482.2089",
                "-737.1616",
                "1096.8827",
                "-521.1341",
                "101.7963",
                "-7.0571",
            ],
        ),
        (
            4,
            [
                "115.5291",
                "169.6374",
                "-245.6490",
                "114.2430",
                "-21.9972",
                "1.5097",
            ],
        ),
    ]
}

/// The `(5040/T)^((n+1)/2)` temperature factor times the `[A, B, C, D, E, F]` polynomial, accumulated in `BigRat`
/// (exact, so the large-coefficient cancellation in the polynomial sum loses no precision). `sqrt_theta` is
/// `sqrt(5040/T)` (one `Fixed::sqrt`, since the fit's half-integer powers of `5040/T` are integer powers of its
/// root), and `inv_lambda` is `1/lambda`. Returns the running sum contribution `sum_n (sqrt_theta)^(n+1) * poly_n`.
fn h_minus_ff_sum(
    rows: &[(i32, [&'static str; 6])],
    lambda: &BigRat,
    inv_lambda: &BigRat,
    sqrt_theta: &BigRat,
) -> Option<BigRat> {
    let lambda2 = lambda.mul(lambda);
    let inv2 = inv_lambda.mul(inv_lambda);
    let inv3 = inv2.mul(inv_lambda);
    let inv4 = inv3.mul(inv_lambda);
    let mut sum = BigRat::from_i64(0);
    for (n, coeffs) in rows {
        let a = BigRat::from_decimal_str(coeffs[0]).ok()?;
        let b = BigRat::from_decimal_str(coeffs[1]).ok()?;
        let cc = BigRat::from_decimal_str(coeffs[2]).ok()?;
        let d = BigRat::from_decimal_str(coeffs[3]).ok()?;
        let e = BigRat::from_decimal_str(coeffs[4]).ok()?;
        let f = BigRat::from_decimal_str(coeffs[5]).ok()?;
        let poly = a
            .mul(&lambda2)
            .add(&b)
            .add(&cc.mul(inv_lambda))
            .add(&d.mul(&inv2))
            .add(&e.mul(&inv3))
            .add(&f.mul(&inv4));
        // (sqrt_theta)^(n+1) by repeated multiply.
        let mut theta_pow = BigRat::from_i64(1);
        for _ in 0..(n + 1) {
            theta_pow = theta_pow.mul(sqrt_theta);
        }
        sum = sum.add(&theta_pow.mul(&poly));
    }
    Some(sum)
}

/// The monochromatic H- FREE-FREE opacity `kappa_ff_Hminus` (cm^2/g) at dimensionless frequency `x = h*nu/(k_B*T)`,
/// the H- gas term that fills the below-photodetachment-threshold window the bound-free leaves empty (so the
/// assembled bf+ff H- opacity is positive at every frequency and can be Rosseland-averaged). A neutral hydrogen, a
/// free electron, and a photon interact (`H0 + e- + photon`) with no threshold, so this term is continuous across
/// all wavelengths and rises to the infrared. The absorption coefficient is the cited John 1988 eq. 6 fit
/// (`kappa_ff_coeff = 1e-29 * sum_n (5040/T)^((n+1)/2) * poly_n(lambda)`, cm^4/dyn per neutral H per electron
/// pressure, [`h_minus_ff_region1`]/[`h_minus_ff_region2`]) times `(X / m_H) * P_e`, so
/// `kappa_ff_Hminus = kappa_ff_coeff * (X / m_H) * P_e`.
///
/// DEFINITION TAG (the two free-free channels are DISTINCT physics and must never be merged): this is
/// `kappa_ff_Hminus`, an electron scattering off a NEUTRAL hydrogen atom (the transient H- during the encounter),
/// so it scales as the neutral-H density times the electron pressure, `(X/m_H) P_e`. It is NOT the electron-ION
/// bremsstrahlung [`kramers_free_free_opacity`] (`kappa_ff_ion`), which is an electron scattering off a POSITIVE
/// ion and scales as `n_e sum(Z_i^2 n_i)`. Both are real and ADDITIVE in the monochromatic sum, but they key off
/// different densities (neutral H versus ions) and different ionization powers; joining them, or feeding one's
/// density to the other, is the definition-mismatch class this codebase legislates against.
///
/// DECLARED REDUCTION: this per-`P_e` form is exact BECAUSE the actual H- number density is a negligible fraction
/// of the electrons, `n(H-)/n_e ~ 1e-8` (the Saha population of a 0.754 eV binding at stellar temperatures). H- is
/// never a bulk species to track; the opacity is a per-encounter cross section carried by the neutral-H population
/// and the electron pressure, which is exactly why the stellar-atmosphere form is written per neutral H per `P_e`
/// rather than over an explicit H- density.
///
/// The only fetched piece is the fit's coefficients (the cited `[M]` tier); the `5040/T` temperature scaling, the
/// `hc/k` wavelength fold, and the `m_H` from the periodic table all derive. The polynomial sum runs in exact
/// `BigRat` (its large-coefficient cancellation loses no precision), with a single `Fixed::sqrt` for the
/// `sqrt(5040/T)` root that serves the fit's half-integer temperature powers. Every per-world quantity is a caller
/// argument (admit-the-alien): `x`, `T`, `hydrogen_mass_fraction` X, and `electron_pressure_dyn_cm2` `P_e` (the
/// same per-electron-pressure form as the bound-free). Returns zero for `lambda < 0.1823 micron` (below the fit's
/// short-wavelength bound). HONEST LIMIT: the fit is stated for `lambda` up to 14 micron; far-infrared `lambda`
/// (very small `x`) extrapolates the region-1 polynomial, but the Rosseland weight there (`w ~ x^2`) is negligible,
/// so the extrapolation does not reach the mean. `None` if a fundamental or the hydrogen data fails to resolve, or
/// the result leaves the representable range.
pub fn h_minus_free_free_opacity(
    x: Fixed,
    temperature_k: Fixed,
    hydrogen_mass_fraction: Fixed,
    electron_pressure_dyn_cm2: Fixed,
    table: &PeriodicTable,
) -> Option<Fixed> {
    if x <= Fixed::ZERO || temperature_k <= Fixed::ZERO {
        return None;
    }
    let h = representation_bigrat("h")?;
    let c = representation_bigrat("c")?;
    let k_b = representation_bigrat("k_B")?;
    let n_a = representation_bigrat("N_A")?;

    // lambda = (h c/k)/(x T) [micron] (h c/k in SI is m*K, *1e6 -> micron*K), as Fixed for the region select and as
    // BigRat for the polynomial.
    let t_br = nonneg_fixed_to_bigrat(temperature_k);
    let alpha_um_k = h.mul(&c).div(&k_b).mul(&BigRat::from_i64(1_000_000));
    let lambda_br_raw = alpha_um_k.div(&nonneg_fixed_to_bigrat(x).mul(&t_br));
    let lambda = Fixed::from_bits_i128(lambda_br_raw.round_to_scale(Fixed::FRAC_BITS)?)?;
    if lambda < Fixed::from_ratio(1823, 10000) {
        return Some(Fixed::ZERO); // below the fit's short-wavelength bound
    }
    let region1 = h_minus_ff_region1();
    let region2 = h_minus_ff_region2();
    let rows: &[(i32, [&'static str; 6])] = if lambda >= Fixed::from_ratio(3645, 10000) {
        &region1
    } else {
        &region2
    };

    let lambda_bigrat = nonneg_fixed_to_bigrat(lambda);
    let inv_lambda = BigRat::from_i64(1).div(&lambda_bigrat);
    // sqrt(5040/T) via one Fixed::sqrt.
    let theta = BigRat::from_i64(5040).div(&t_br);
    let sqrt_theta = nonneg_fixed_to_bigrat(
        Fixed::from_bits_i128(theta.round_to_scale(Fixed::FRAC_BITS)?)?.sqrt(),
    );
    let sum = h_minus_ff_sum(rows, &lambda_bigrat, &inv_lambda, &sqrt_theta)?;

    // kappa_ff = 1e-29 * sum * (X/m_H) * P_e (cm^2/g). 1e-29 = 1/(1e15 * 1e14).
    let inv_1e29 = BigRat::from_i64(1)
        .div(&BigRat::from_i64(1_000_000_000_000_000))
        .div(&BigRat::from_i64(100_000_000_000_000));
    let m_h_cgs = nonneg_fixed_to_bigrat(table.molar_mass_of(&[("H", 1)]).ok()?).div(&n_a); // g
    let x_over_mh = nonneg_fixed_to_bigrat(hydrogen_mass_fraction).div(&m_h_cgs);
    let p_e = nonneg_fixed_to_bigrat(electron_pressure_dyn_cm2);
    let kappa = sum.mul(&inv_1e29).mul(&x_over_mh).mul(&p_e);
    let bits = kappa.round_to_scale(Fixed::FRAC_BITS)?;
    if bits < 0 {
        return Some(Fixed::ZERO); // a rare far-infrared extrapolation dip below zero reads as no opacity
    }
    Fixed::from_bits_i128(bits)
}

/// The assembled MONOCHROMATIC H- opacity `kappa(H-)` (cm^2/g) at dimensionless frequency `x`: the sum of the
/// bound-free ([`h_minus_bound_free_opacity`]) and free-free ([`h_minus_free_free_opacity`]) terms, the H- gas
/// term's total spectral contribution. The free-free fills the below-photodetachment-threshold window the
/// bound-free leaves empty, so within the H- fit domains this is positive and carries the famous H- opacity
/// MINIMUM near the `1.6419 micron` threshold (the bound-free cutting off while the free-free rises).
///
/// IMPORTANT (the fail-loud seam): this is a SPECTRAL PROVIDER for the full Rosseland assembly, NOT a standalone
/// Rosseland-averageable opacity. Beyond the fit domains (the far ultraviolet `lambda < 0.1823 micron`, reached at
/// the high-`x` end for `T` above `~3946 K`) BOTH H- fits read zero, so `kappa(H-) = 0` there, and Rosseland-
/// averaging H- alone would trip the kernel's strict-positivity precondition. That is correct physics: H- is not
/// the opacity there, the electron-scattering floor is, so the Rosseland mean is taken over the ASSEMBLED total
/// (H- + electron scattering + the rest), where the floor keeps the sum positive. The assembly is the later slice
/// (the bounded midplane fixed point); this term supplies its H- spectral piece. `None` on the same conditions as
/// the two component terms.
pub fn h_minus_opacity(
    execution: &SiExecutionMagnitudes,
    x: Fixed,
    temperature_k: Fixed,
    hydrogen_mass_fraction: Fixed,
    electron_pressure_dyn_cm2: Fixed,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let bf = h_minus_bound_free_opacity(
        execution,
        x,
        temperature_k,
        hydrogen_mass_fraction,
        electron_pressure_dyn_cm2,
        table,
    )?;
    let ff = h_minus_free_free_opacity(
        x,
        temperature_k,
        hydrogen_mass_fraction,
        electron_pressure_dyn_cm2,
        table,
    )?;
    bf.checked_add(ff)
}

/// The monochromatic small-grain RAYLEIGH absorption opacity `kappa_grain` (cm^2/g of grain material) at
/// dimensionless frequency `x = h*nu/(k_B*T)`, for one dust species read from the optical-constants library. In the
/// Rayleigh limit (grain radius << wavelength) a small sphere's mass absorption coefficient is
/// `kappa_nu = (6*pi/(rho_grain*lambda)) * Im[(m^2-1)/(m^2+2)]` with `m = n + i k` (from the polarizability
/// `alpha = 4*pi*a^3 (m^2-1)/(m^2+2)`, `C_abs = (2*pi/lambda) Im(alpha)`, over the grain mass), which is GRAIN-SIZE-
/// INDEPENDENT (the size distribution matters only in the Mie regime). The imaginary part is the analytic real
/// expression `Im[(m^2-1)/(m^2+2)] = 6nk/((n^2-k^2+2)^2 + 4n^2k^2)`, so no complex arithmetic is needed here (it
/// enters only at the full Mie kernel, the later sub-slice).
///
/// The optical constants `n(lambda),k(lambda)` are the cited `[M]` library ([`OpticalSpecies`]), composition-keyed
/// (a carbon-rich or metal-poor grain is a different species membership, admit-the-alien); the bulk density
/// `rho_grain` is a caller argument, DERIVED upstream from the grain's composition via the materials density kernel,
/// never authored here (Pollack's per-species densities are the validation target, not a floor value). A metal
/// (iron) has a small Rayleigh absorption (it scatters more than it absorbs, the `(m^2+2)` denominator large), a
/// silicate a large one, keyed on the species data.
///
/// When the far-infrared optical constants follow the Lorentz-oscillator wing (`k ~ 1/lambda`), `kappa_nu ~
/// lambda^-2 ~ x^2`, and the Rosseland mean of `x^2` is `(4/5)pi^2`, so `kappa_R ~ T^2` (the standard `beta=2`
/// small-grain opacity law), which holds in the cold far-infrared and gives way to the full spectral opacity near
/// the resonance bands. This is a SPECTRAL PROVIDER (monochromatic); the Rosseland mean is taken over the assembled
/// multi-species total with the gas floor (the 3c-iii assembly), never one species in isolation, whose coverage
/// gaps and transparent windows would trip the fail-loud kernel. Computed in exact `BigRat` (a metal's `4n^2k^2`
/// overflows Q32.32) and rounded once. `None` if a fundamental fails to resolve, the wavelength is outside the
/// species' sampled coverage, or the result leaves the representable range.
pub fn rayleigh_grain_opacity(
    x: Fixed,
    temperature_k: Fixed,
    bulk_density_g_cm3: Fixed,
    species: &OpticalSpecies,
) -> Option<Fixed> {
    if x <= Fixed::ZERO || temperature_k <= Fixed::ZERO || bulk_density_g_cm3 <= Fixed::ZERO {
        return None;
    }
    let h = representation_bigrat("h")?;
    let c = representation_bigrat("c")?;
    let k_b = representation_bigrat("k_B")?;
    let pi = compute::pi(OPACITY_PI_DIGITS);

    // lambda = (h c/k)/(x T) [micron], as Fixed for the table interpolation and BigRat for the opacity.
    let alpha_um_k = h.mul(&c).div(&k_b).mul(&BigRat::from_i64(1_000_000));
    let lambda_br =
        alpha_um_k.div(&nonneg_fixed_to_bigrat(x).mul(&nonneg_fixed_to_bigrat(temperature_k)));
    let lambda_fixed = Fixed::from_bits_i128(lambda_br.round_to_scale(Fixed::FRAC_BITS)?)?;
    let (n, k) = species.interpolate(lambda_fixed)?;

    // Im[(m^2-1)/(m^2+2)] = 6nk/((n^2-k^2+2)^2 + 4n^2k^2), in BigRat (a metal's 4n^2k^2 overflows Fixed; the
    // n^2-k^2+2 term goes negative for a metal but squares positive).
    let n_br = nonneg_fixed_to_bigrat(n);
    let k_br = nonneg_fixed_to_bigrat(k);
    let n2 = n_br.mul(&n_br);
    let k2 = k_br.mul(&k_br);
    let num = BigRat::from_i64(6).mul(&n_br).mul(&k_br);
    let a = n2.sub(&k2).add(&BigRat::from_i64(2));
    let denom = a.mul(&a).add(&BigRat::from_i64(4).mul(&n2).mul(&k2));
    let im = num.div(&denom);

    // kappa_nu = (6*pi/(rho * lambda_cm)) * Im, lambda_cm = lambda_um * 1e-4.
    let lambda_cm = lambda_br.div(&BigRat::from_i64(10000));
    let rho = nonneg_fixed_to_bigrat(bulk_density_g_cm3);
    let kappa = BigRat::from_i64(6)
        .mul(&pi)
        .div(&rho.mul(&lambda_cm))
        .mul(&im);
    Fixed::from_bits_i128(kappa.round_to_scale(Fixed::FRAC_BITS)?)
}

/// The fractional-bit scale of the Mie kernel's internal complex arithmetic. A working value is a plain
/// `i128` reading `w / 2^MIE_SCALE_BITS`, so it carries ~19 significant decimal figures (well above the
/// Q32.32 result's ~9.6, which absorbs the recurrence's rounding), and the largest magnitude the field
/// recurrence reaches over the kernel's valid range (`|chi_n|^2 ~ 2e10` near the small-x end) fits the
/// `i128` backing with room to spare. Only a product or quotient, which can exceed `i128`, borrows the
/// exact `BigRat` as its wide intermediate and rounds straight back; every other step is `i128` arithmetic.
/// An engine-accuracy bound, chosen from the representable-range and precision budget, not world content.
const MIE_SCALE_BITS: u32 = 64;

/// The `i128` reading of `Fixed::ONE` at the Mie scale.
const MIE_ONE_W: i128 = 1i128 << MIE_SCALE_BITS;

/// The largest size parameter the Mie kernel evaluates. Above it the geometric-optics limit (`Q_ext -> 2`,
/// `Q_abs` set by the surface reflectance) is the accurate and cheap description, so the caller uses that
/// rather than paying the `O(x)` field recurrence. A performance-and-range bound, not a physics claim.
const MIE_X_SWITCH: i32 = 50;

/// The smallest size parameter the Mie kernel evaluates, as the ratio `MIE_X_MIN_NUM / MIE_X_MIN_DEN`.
/// Below it the closed dipole (Rayleigh) form [`rayleigh_grain_opacity`] is exact to a fraction of a
/// percent, and the upward field recurrence's `1/x` amplification would erode the `i128` margin (the
/// recurrence sits deeper in the unstable small-x regime). The caller uses the Rayleigh form there. A
/// range bound matched to where the cheaper closed form is accurate, not world content.
const MIE_X_MIN_NUM: i64 = 1;
const MIE_X_MIN_DEN: i64 = 10;

/// A `u128` magnitude as a `BigUint` (the register carries `from_u64` only; a Mie intermediate magnitude
/// reaches ~`2^112`, beyond a single `u64`).
fn biguint_from_u128(v: u128) -> BigUint {
    BigUint::from_u64((v >> 64) as u64)
        .shl_bits(64)
        .add(&BigUint::from_u64(v as u64))
}

/// A wide-fixed value (`w / 2^MIE_SCALE_BITS`) as an exact rational, sign preserved.
fn wfixed_to_bigrat(w: i128) -> BigRat {
    BigRat::new(
        w < 0,
        biguint_from_u128(w.unsigned_abs()),
        BigUint::from_u64(1).shl_bits(MIE_SCALE_BITS),
    )
}

/// A `Fixed` (Q32.32) as a wide-fixed value: an exact left shift by the scale difference, sign preserved.
fn fixed_to_wfixed(value: Fixed) -> i128 {
    (value.to_bits() as i128) << (MIE_SCALE_BITS - Fixed::FRAC_BITS)
}

/// The wide-fixed product `a * b`, formed exactly in `BigRat` and rounded once to the scale. `None` if the
/// result leaves the `i128` backing (fail-loud: no wrap ever reaches the caller).
fn wmul(a: i128, b: i128) -> Option<i128> {
    wfixed_to_bigrat(a)
        .mul(&wfixed_to_bigrat(b))
        .round_to_scale(MIE_SCALE_BITS)
}

/// The wide-fixed quotient `a / b`, formed exactly and rounded once. `None` on a zero divisor or an
/// out-of-range result (fail-loud).
fn wdiv(a: i128, b: i128) -> Option<i128> {
    if b == 0 {
        return None;
    }
    wfixed_to_bigrat(a)
        .div(&wfixed_to_bigrat(b))
        .round_to_scale(MIE_SCALE_BITS)
}

/// A complex number in wide-fixed components (`re + i*im`, each `w / 2^MIE_SCALE_BITS`). The whole
/// arithmetic is fail-loud: every operation returns `None` the instant a component leaves the `i128`
/// backing, so the Mie kernel never propagates a wrapped value.
#[derive(Clone, Copy)]
struct WCplx {
    re: i128,
    im: i128,
}

impl WCplx {
    /// A real value (`im = 0`).
    fn real(re: i128) -> Self {
        WCplx { re, im: 0 }
    }

    /// Sum.
    fn add(self, o: WCplx) -> Option<WCplx> {
        Some(WCplx {
            re: self.re.checked_add(o.re)?,
            im: self.im.checked_add(o.im)?,
        })
    }

    /// Difference.
    fn sub(self, o: WCplx) -> Option<WCplx> {
        Some(WCplx {
            re: self.re.checked_sub(o.re)?,
            im: self.im.checked_sub(o.im)?,
        })
    }

    /// Product `(a+bi)(c+di) = (ac-bd) + (ad+bc)i`.
    fn mul(self, o: WCplx) -> Option<WCplx> {
        Some(WCplx {
            re: wmul(self.re, o.re)?.checked_sub(wmul(self.im, o.im)?)?,
            im: wmul(self.re, o.im)?.checked_add(wmul(self.im, o.re)?)?,
        })
    }

    /// Product with a real scalar.
    fn scale(self, s: i128) -> Option<WCplx> {
        Some(WCplx {
            re: wmul(self.re, s)?,
            im: wmul(self.im, s)?,
        })
    }

    /// The squared magnitude `re^2 + im^2` (a real value).
    fn norm_sq(self) -> Option<i128> {
        wmul(self.re, self.re)?.checked_add(wmul(self.im, self.im)?)
    }

    /// Quotient `self / o = self * conj(o) / |o|^2`.
    fn div(self, o: WCplx) -> Option<WCplx> {
        let d = o.norm_sq()?;
        Some(WCplx {
            re: wdiv(wmul(self.re, o.re)?.checked_add(wmul(self.im, o.im)?)?, d)?,
            im: wdiv(wmul(self.im, o.re)?.checked_sub(wmul(self.re, o.im)?)?, d)?,
        })
    }
}

/// The Mie absorption efficiency `Q_abs = Q_ext - Q_sca` of a homogeneous sphere of size parameter
/// `x = 2*pi*a/lambda` and complex refractive index `m = n + i*k`, DERIVED from Mie theory by the
/// Bohren and Huffman BHMIE recurrence rather than read from a fitted table. `Q_abs` is the dimensionless
/// absorption cross section per geometric cross section, the quantity the grain opacity integral weights by
/// the size distribution.
///
/// The algorithm is BHMIE (Bohren and Huffman 1983, Appendix A): the logarithmic derivative `D_n(mx)` by
/// DOWNWARD recurrence seeded at `nmx = max(nstop, |mx|) + 15` with `D_nmx = 0` (downward is the stable
/// direction for `D_n`), the Riccati-Bessel `psi_n` and `chi_n` by upward recurrence from `psi_0 = sin x`,
/// `chi_0 = cos x`, then the coefficients `a_n`, `b_n` and the sums `Q_ext = (2/x^2) sum (2n+1) Re(a_n+b_n)`
/// and `Q_sca = (2/x^2) sum (2n+1) (|a_n|^2 + |b_n|^2)`, truncated at Wiscombe's `nstop = x + 4 x^(1/3) + 2`.
/// The whole computation runs in deterministic wide-fixed complex arithmetic (see [`MIE_SCALE_BITS`]), so it
/// is bit-identical on every machine; a component that would leave the `i128` backing returns `None` rather
/// than wrapping.
///
/// Every input is per-particle DATA: `n` and `k` are read from the measured optical constants (the alien
/// seam, a metal or an exotic condensate is a different `(n, k)` row, never a rewrite), and `x` carries the
/// grain radius and wavelength. `None` outside the validated range `[MIE_X_MIN, MIE_X_SWITCH]` (below it the
/// closed Rayleigh form is exact and the caller uses [`rayleigh_grain_opacity`]; above it the geometric
/// limit applies), on a non-physical index (`n <= 0` or `k < 0`), or on any wide-fixed overflow.
pub fn mie_q_abs(size_parameter: Fixed, n: Fixed, k: Fixed) -> Option<Fixed> {
    if n <= Fixed::ZERO || k < Fixed::ZERO {
        return None;
    }
    let x_min = Fixed::from_ratio(MIE_X_MIN_NUM, MIE_X_MIN_DEN);
    if size_parameter < x_min || size_parameter > Fixed::from_int(MIE_X_SWITCH) {
        return None;
    }
    let x = size_parameter;

    // Wiscombe's truncation nstop = floor(x + 4 x^(1/3) + 2) and the downward-recurrence seed
    // nmx = max(nstop, |mx|) + 15, |mx| = |m| x = sqrt(n^2 + k^2) x.
    let ns_fixed = x
        .checked_add(Fixed::from_int(4).checked_mul(x.cbrt())?)?
        .checked_add(Fixed::from_int(2))?;
    let nstop = (ns_fixed.to_bits() >> Fixed::FRAC_BITS) as u32;
    let abs_m = n.checked_mul(n)?.checked_add(k.checked_mul(k)?)?.sqrt();
    let abs_mx_int = (x.checked_mul(abs_m)?.to_bits() >> Fixed::FRAC_BITS) as u32;
    let nmx = nstop.max(abs_mx_int) + 15;

    let x_w = fixed_to_wfixed(x);
    let n_w = fixed_to_wfixed(n);
    let k_w = fixed_to_wfixed(k);
    let m = WCplx { re: n_w, im: k_w };
    let mx = WCplx {
        re: wmul(n_w, x_w)?,
        im: wmul(k_w, x_w)?,
    };

    // D_n(mx) by downward recurrence: D_{n-1} = n/mx - 1/(D_n + n/mx), from D_nmx = 0.
    let mut d = vec![WCplx::real(0); (nmx as usize) + 1];
    for nn in (1..=nmx).rev() {
        let n_over_mx = WCplx::real((nn as i128) << MIE_SCALE_BITS).div(mx)?;
        let denom = d[nn as usize].add(n_over_mx)?;
        d[(nn - 1) as usize] = n_over_mx.sub(WCplx::real(MIE_ONE_W).div(denom)?)?;
    }

    // psi and chi seeds: psi_{-1} = cos x, psi_0 = sin x; chi_{-1} = -sin x, chi_0 = cos x;
    // xi_0 = psi_0 - i chi_0.
    let (sin_x, cos_x) = x.sin_cos();
    let sin_w = fixed_to_wfixed(sin_x);
    let cos_w = fixed_to_wfixed(cos_x);
    let mut psi_prev = cos_w;
    let mut psi_curr = sin_w;
    let mut chi_prev = sin_w.checked_neg()?;
    let mut chi_curr = cos_w;
    let mut xi_prev = WCplx {
        re: sin_w,
        im: cos_w.checked_neg()?,
    };
    let mut qsca: i128 = 0;
    let mut qext: i128 = 0;

    for nn in 1..=nstop {
        // psi_n and chi_n at the top of the loop, so a_n uses psi_n (new) with psi_{n-1} (prev): the
        // BHMIE indexing that a naive port gets wrong.
        let c1 = (2 * nn - 1) as i128;
        let psi_n = wdiv(c1.checked_mul(psi_curr)?, x_w)?.checked_sub(psi_prev)?;
        let chi_n = wdiv(c1.checked_mul(chi_curr)?, x_w)?.checked_sub(chi_prev)?;
        let xi_n = WCplx {
            re: psi_n,
            im: chi_n.checked_neg()?,
        };

        let n_over_x = wdiv((nn as i128) << MIE_SCALE_BITS, x_w)?;
        let da = d[nn as usize].div(m)?.add(WCplx::real(n_over_x))?;
        let db = m.mul(d[nn as usize])?.add(WCplx::real(n_over_x))?;
        let an = da
            .scale(psi_n)?
            .sub(WCplx::real(psi_curr))?
            .div(da.mul(xi_n)?.sub(xi_prev)?)?;
        let bn = db
            .scale(psi_n)?
            .sub(WCplx::real(psi_curr))?
            .div(db.mul(xi_n)?.sub(xi_prev)?)?;

        let c2 = (2 * nn + 1) as i128;
        let sca_term = an.norm_sq()?.checked_add(bn.norm_sq()?)?;
        qsca = qsca.checked_add(c2.checked_mul(sca_term)?)?;
        let ext_term = an.re.checked_add(bn.re)?;
        qext = qext.checked_add(c2.checked_mul(ext_term)?)?;

        psi_prev = psi_curr;
        psi_curr = psi_n;
        chi_prev = chi_curr;
        chi_curr = chi_n;
        xi_prev = xi_n;
    }

    // Q = (2/x^2) * sum; Q_abs = Q_ext - Q_sca, then rounded once to the Q32.32 result.
    let two_over_x2 = wdiv(2 * MIE_ONE_W, wmul(x_w, x_w)?)?;
    let q_sca = wmul(two_over_x2, qsca)?;
    let q_ext = wmul(two_over_x2, qext)?;
    let q_abs = q_ext.checked_sub(q_sca)?;
    Fixed::from_bits_i128(wfixed_to_bigrat(q_abs).round_to_scale(Fixed::FRAC_BITS)?)
}

/// The number of log-spaced grain-size samples in the size-distribution quadrature. The integrand is
/// smooth in `ln a` and the trapezoid sum converges to a few parts in 1e5 by this count (checked against a
/// four-times-finer grid), so the discretization error sits far below the Q32.32 result's low bit. An
/// engine-accuracy bound, a quadrature resolution, not world content.
const GRAIN_SIZE_INTERVALS: i32 = 128;

/// The absorption efficiency `Q_abs` of a single sphere of size parameter `x` and index `m = n + i*k`,
/// dispatched across the three regimes so a caller integrating over a size distribution needs one call:
/// the closed dipole (Rayleigh) form `4x Im[(m^2-1)/(m^2+2)]` below `MIE_X_MIN` (exact for `x << 1`, and
/// the `Im` runs in `BigRat` because a metal's `4 n^2 k^2` overflows Q32.32), the full [`mie_q_abs`] on
/// `[MIE_X_MIN, MIE_X_SWITCH]`, and the geometric opaque-sphere limit `Q_abs -> 1` above it. `None` on a
/// non-physical index or a wide-fixed overflow inside the Mie branch.
///
/// The geometric branch carries an honest limit: a large opaque grain absorbs about its geometric cross
/// section, so `Q_abs -> 1`, which drops the Fresnel surface-reflectance correction (a few percent for a
/// dielectric). The `a^(2-p)` size weighting suppresses this regime's contribution for the standard grain
/// bounds (a grain reaches `x > 50` only well above the micron sizes the distribution holds), so the
/// simplification changes the size-averaged opacity by far less than the reflectance error itself.
fn grain_qabs(x: Fixed, n: Fixed, k: Fixed) -> Option<Fixed> {
    if n <= Fixed::ZERO || k < Fixed::ZERO {
        return None;
    }
    let x_min = Fixed::from_ratio(MIE_X_MIN_NUM, MIE_X_MIN_DEN);
    if x < x_min {
        // Rayleigh dipole: Q_abs = 4x * Im[(m^2-1)/(m^2+2)], Im = 6nk/((n^2-k^2+2)^2 + 4n^2k^2).
        let n_br = nonneg_fixed_to_bigrat(n);
        let k_br = nonneg_fixed_to_bigrat(k);
        let n2 = n_br.mul(&n_br);
        let k2 = k_br.mul(&k_br);
        let num = BigRat::from_i64(6).mul(&n_br).mul(&k_br);
        let a = n2.sub(&k2).add(&BigRat::from_i64(2));
        let denom = a.mul(&a).add(&BigRat::from_i64(4).mul(&n2).mul(&k2));
        let im = num.div(&denom);
        let four_x = nonneg_fixed_to_bigrat(x).mul(&BigRat::from_i64(4));
        Fixed::from_bits_i128(four_x.mul(&im).round_to_scale(Fixed::FRAC_BITS)?)
    } else if x <= Fixed::from_int(MIE_X_SWITCH) {
        mie_q_abs(x, n, k)
    } else {
        Some(Fixed::ONE)
    }
}

/// The size-distribution-averaged grain mass absorption coefficient `kappa_abs(lambda)` (cm^2/g), DERIVED
/// by integrating the single-sphere [`grain_qabs`] over a power-law grain-size distribution rather than
/// read from a fitted opacity. For a number density `n(a) da ~ a^(-slope) da` between `a_min` and `a_max`,
/// the mass absorption coefficient is
/// `kappa_abs = (3 / (4 rho)) * <a^2 Q_abs> / <a^3>`, `<f> = integral a^(-slope) f(a) da`,
/// the ratio of the absorption cross section per grain to the mass per grain, averaged over the
/// distribution (the number-scale cancels between numerator and denominator). The integral runs in
/// `ln a` (the integrand is smooth there) by the trapezoid rule over [`GRAIN_SIZE_INTERVALS`] samples.
///
/// The distribution is DATA, not authored: `slope`, `a_min`, and `a_max` are arguments the caller supplies
/// from the upstream condensation and coagulation physics. The Dohnanyi collisional-cascade steady state
/// (`slope = 3.5`) is the reserved validation anchor for that upstream, surfaced with its basis, never
/// inlined here. The index `(n, k)` and bulk density `rho` are per-composition data (the alien seam: a
/// metal, an ice, or an exotic condensate is a different set of arguments, never a rewrite).
///
/// In the small-grain limit (every grain `x << 1`) this reduces exactly to the size-independent Rayleigh
/// opacity [`rayleigh_grain_opacity`], the closed dipole form. `None` on a non-positive wavelength, density,
/// or size bound, on `a_max <= a_min`, or on any overflow in the quadrature.
pub fn grain_size_averaged_opacity(
    lambda_um: Fixed,
    n: Fixed,
    k: Fixed,
    bulk_density_g_cm3: Fixed,
    slope: Fixed,
    a_min_um: Fixed,
    a_max_um: Fixed,
) -> Option<Fixed> {
    if lambda_um <= Fixed::ZERO
        || bulk_density_g_cm3 <= Fixed::ZERO
        || a_min_um <= Fixed::ZERO
        || a_max_um <= a_min_um
    {
        return None;
    }
    let u_min = a_min_um.ln();
    let u_max = a_max_um.ln();
    let du = u_max
        .checked_sub(u_min)?
        .checked_div(Fixed::from_int(GRAIN_SIZE_INTERVALS))?;
    let two_pi = Fixed::PI.checked_mul(Fixed::from_int(2))?;
    // int a^(-slope) a^2 Q da = int a^(3-slope) Q du and int a^(-slope) a^3 da = int a^(4-slope) du, so the
    // numerator weight is a^(3-slope) and the denominator weight is a^(4-slope) = a * a^(3-slope).
    let three_minus_slope = Fixed::from_int(3).checked_sub(slope)?;

    let mut num = Fixed::ZERO;
    let mut den = Fixed::ZERO;
    for i in 0..=GRAIN_SIZE_INTERVALS {
        let u = u_min.checked_add(du.checked_mul(Fixed::from_int(i))?)?;
        let a = u.exp();
        let x = two_pi.checked_mul(a)?.checked_div(lambda_um)?;
        let q = grain_qabs(x, n, k)?;
        // Trapezoid: half weight at the two endpoints. Halving a^(3-slope) halves both accumulations, so
        // the du scale cancels in the num/den ratio and is never formed.
        let a3 = three_minus_slope.checked_mul(u)?.exp();
        let a3 = if i == 0 || i == GRAIN_SIZE_INTERVALS {
            a3.checked_div(Fixed::from_int(2))?
        } else {
            a3
        };
        num = num.checked_add(a3.checked_mul(q)?)?;
        den = den.checked_add(a3.checked_mul(a)?)?;
    }

    // kappa_abs = (3/(4 rho)) * (num/den) * 1e4, the last factor converting the 1/micron of a^2/a^3 to
    // 1/cm (the lengths otherwise cancel in the ratio).
    let ratio = num.checked_div(den)?;
    Fixed::from_ratio(3, 4)
        .checked_div(bulk_density_g_cm3)?
        .checked_mul(ratio)?
        .checked_mul(Fixed::from_int(10000))
}

/// The Newton iteration count for the Bruggeman effective-medium solve. The iteration converges
/// quadratically from the volume-weighted seed, so a converged root is reached in well under this count;
/// the surplus holds the root fixed and keeps the count data-independent (determinism needs a fixed number
/// of steps, not a convergence branch). An engine-convergence bound, not world content.
const BRUGGEMAN_ITERS: u32 = 40;

/// The effective complex refractive index `(n_eff, k_eff)` of a grain built from several condensate species
/// mixed at the given volume fractions, DERIVED by the Bruggeman effective-medium rule rather than by
/// averaging the indices. Bruggeman treats every component symmetrically (no host), solving
/// `sum_i f_i (eps_i - eps_eff) / (eps_i + 2 eps_eff) = 0` for the effective permittivity `eps_eff`, with
/// each `eps_i = (n_i + i k_i)^2`. The solve is Newton's method (`f'(eps) = -3 sum_i f_i eps_i /
/// (eps_i + 2 eps_eff)^2`) from the volume-weighted seed, run in the deterministic wide-fixed complex
/// arithmetic, then `eps_eff` is turned back into `(n, k)`.
///
/// The composition is DATA: the fractions come from the upstream condensate assemblage (the disposer
/// output), and each `(n_i, k_i)` is a measured optical-constants row (the alien seam: a world whose grains
/// carry a condensate not in the Terran set is a different set of rows and fractions, never a rewrite). The
/// rule is scale-free in the fractions (the root is unchanged if they are all scaled), so it does not
/// require them pre-normalized. `None` if the lists differ in length or are empty, on a non-physical index
/// or a negative fraction, or on any overflow or singular denominator in the solve.
pub fn bruggeman_effective_index(
    fractions: &[Fixed],
    indices: &[(Fixed, Fixed)],
) -> Option<(Fixed, Fixed)> {
    if fractions.is_empty() || fractions.len() != indices.len() {
        return None;
    }
    for (frac, (n, k)) in fractions.iter().zip(indices.iter()) {
        if *frac < Fixed::ZERO || *n <= Fixed::ZERO || *k < Fixed::ZERO {
            return None;
        }
    }

    // eps_i = (n_i + i k_i)^2 = (n^2 - k^2) + i (2 n k), in wide-fixed.
    let mut eps_list = Vec::with_capacity(indices.len());
    for (n, k) in indices {
        let n_w = fixed_to_wfixed(*n);
        let k_w = fixed_to_wfixed(*k);
        let re = wmul(n_w, n_w)?.checked_sub(wmul(k_w, k_w)?)?;
        let im = wmul(n_w, k_w)?.checked_mul(2)?;
        eps_list.push(WCplx { re, im });
    }

    // Seed: the volume-weighted mean permittivity (sum f_i eps_i) / (sum f_i).
    let mut num_seed = WCplx::real(0);
    let mut den_seed: i128 = 0;
    for (frac, eps) in fractions.iter().zip(eps_list.iter()) {
        let f_w = fixed_to_wfixed(*frac);
        num_seed = num_seed.add(eps.scale(f_w)?)?;
        den_seed = den_seed.checked_add(f_w)?;
    }
    if den_seed <= 0 {
        return None;
    }
    let mut eff = num_seed.div(WCplx::real(den_seed))?;

    let two_w = 2 * MIE_ONE_W;
    let neg_three_w = -3 * MIE_ONE_W;
    for _ in 0..BRUGGEMAN_ITERS {
        let mut f_val = WCplx::real(0);
        let mut f_prime = WCplx::real(0);
        for (frac, eps) in fractions.iter().zip(eps_list.iter()) {
            let f_w = fixed_to_wfixed(*frac);
            let denom = eps.add(eff.scale(two_w)?)?; // eps_i + 2 eps_eff
            let term = eps.sub(eff)?.div(denom)?.scale(f_w)?; // f_i (eps_i - eps_eff)/(eps_i + 2 eps_eff)
            f_val = f_val.add(term)?;
            let denom_sq = denom.mul(denom)?;
            let term_p = eps.div(denom_sq)?.scale(f_w)?; // f_i eps_i / (eps_i + 2 eps_eff)^2
            f_prime = f_prime.add(term_p)?;
        }
        let f_prime = f_prime.scale(neg_three_w)?; // f'(eps) = -3 sum ...
        eff = eff.sub(f_val.div(f_prime)?)?; // Newton step
    }

    // eps_eff = re + i im -> (n, k): n = sqrt((|eps| + re)/2), k = sqrt((|eps| - re)/2).
    let re_f = Fixed::from_bits_i128(wfixed_to_bigrat(eff.re).round_to_scale(Fixed::FRAC_BITS)?)?;
    let im_f = Fixed::from_bits_i128(wfixed_to_bigrat(eff.im).round_to_scale(Fixed::FRAC_BITS)?)?;
    let modulus = re_f
        .checked_mul(re_f)?
        .checked_add(im_f.checked_mul(im_f)?)?
        .sqrt();
    let two = Fixed::from_int(2);
    let n_eff = modulus.checked_add(re_f)?.checked_div(two)?.sqrt();
    // The k branch can round a hair below zero for a pure dielectric (|eps| == re); clamp before the root.
    let k_arg = modulus.checked_sub(re_f)?;
    let k_arg = if k_arg < Fixed::ZERO {
        Fixed::ZERO
    } else {
        k_arg
    };
    let k_eff = k_arg.checked_div(two)?.sqrt();
    Some((n_eff, k_eff))
}

/// The effective complex refractive index `(n_eff, k_eff)` of a grain built as a HOST MATRIX with embedded
/// INCLUSIONS, DERIVED by the Maxwell-Garnett effective-medium rule. Unlike [`bruggeman_effective_index`]
/// (symmetric, no host), Maxwell-Garnett is ASYMMETRIC: it models inclusions dispersed in a continuous matrix, the
/// topology the condensation history writes below the ice line (Rule 2: refractories arrive as cores inside ice
/// mantles as the disposer's condensation sequence deposits them, so ICE is the matrix and the refractories are the
/// inclusions). The two rules give DIFFERENT effective indices for the same components (ice-mantled iron absorbs
/// differently from a symmetric iron-plus-ice mixture), which is the factor-level distinction Rule 2 keys on: below
/// the ice line the mantle topology is Maxwell-Garnett, above it the bare mixture is Bruggeman.
///
/// Closed form (no iteration): with `eps_m` the matrix and `eps_i` the inclusion permittivities (`eps = (n+i k)^2`),
///   `beta = sum_i f_i (eps_i - eps_m)/(eps_i + 2 eps_m)`,  `eps_eff = eps_m (1 + 2 beta)/(1 - beta)`,
/// run in the deterministic wide-fixed complex arithmetic, then `eps_eff` is turned back into `(n, k)`. The
/// inclusion fractions are the VOLUME fractions from the disposer condensate assemblage (data, admit-the-alien: an
/// alien mantle carrying a condensate not in the Terran set is a different set of rows and fractions, never a
/// rewrite); an empty inclusion list or all-zero fractions returns the bare matrix. `None` if the lists differ in
/// length, on a non-physical index or a negative fraction, or on any overflow or singular denominator (an
/// over-packed inclusion set driving `1 - beta` through zero is non-physical and fails loud).
pub fn maxwell_garnett_effective_index(
    matrix_index: (Fixed, Fixed),
    inclusion_fractions: &[Fixed],
    inclusion_indices: &[(Fixed, Fixed)],
) -> Option<(Fixed, Fixed)> {
    if inclusion_fractions.len() != inclusion_indices.len() {
        return None;
    }
    let (n_m, k_m) = matrix_index;
    if n_m <= Fixed::ZERO || k_m < Fixed::ZERO {
        return None;
    }
    let to_eps = |n: Fixed, k: Fixed| -> Option<WCplx> {
        let n_w = fixed_to_wfixed(n);
        let k_w = fixed_to_wfixed(k);
        let re = wmul(n_w, n_w)?.checked_sub(wmul(k_w, k_w)?)?;
        let im = wmul(n_w, k_w)?.checked_mul(2)?;
        Some(WCplx { re, im })
    };

    let eps_m = to_eps(n_m, k_m)?;
    let two_w = 2 * MIE_ONE_W;
    let eps_m_two = eps_m.scale(two_w)?; // 2 eps_m

    // beta = sum_i f_i (eps_i - eps_m)/(eps_i + 2 eps_m).
    let mut beta = WCplx::real(0);
    for (frac, (n, k)) in inclusion_fractions.iter().zip(inclusion_indices.iter()) {
        if *frac < Fixed::ZERO || *n <= Fixed::ZERO || *k < Fixed::ZERO {
            return None;
        }
        let f_w = fixed_to_wfixed(*frac);
        let eps_i = to_eps(*n, *k)?;
        let denom = eps_i.add(eps_m_two)?; // eps_i + 2 eps_m
        let term = eps_i.sub(eps_m)?.div(denom)?.scale(f_w)?;
        beta = beta.add(term)?;
    }

    // eps_eff = eps_m (1 + 2 beta)/(1 - beta).
    let one = WCplx::real(MIE_ONE_W);
    let numer = one.add(beta.scale(two_w)?)?; // 1 + 2 beta
    let denom = one.sub(beta)?; // 1 - beta
    let eff = eps_m.mul(numer)?.div(denom)?;

    // eps_eff = re + i im -> (n, k): n = sqrt((|eps| + re)/2), k = sqrt((|eps| - re)/2).
    let re_f = Fixed::from_bits_i128(wfixed_to_bigrat(eff.re).round_to_scale(Fixed::FRAC_BITS)?)?;
    let im_f = Fixed::from_bits_i128(wfixed_to_bigrat(eff.im).round_to_scale(Fixed::FRAC_BITS)?)?;
    let modulus = re_f
        .checked_mul(re_f)?
        .checked_add(im_f.checked_mul(im_f)?)?
        .sqrt();
    let two = Fixed::from_int(2);
    let n_eff = modulus.checked_add(re_f)?.checked_div(two)?.sqrt();
    let k_arg = modulus.checked_sub(re_f)?;
    let k_arg = if k_arg < Fixed::ZERO {
        Fixed::ZERO
    } else {
        k_arg
    };
    let k_eff = k_arg.checked_div(two)?.sqrt();
    Some((n_eff, k_eff))
}

/// The number of log-spaced wavelength samples the grain Rosseland mean precomputes. The Rosseland weight
/// is smooth in `ln x`, so the spectral opacity is sampled on this coarse grid once and interpolated
/// (log-log) at each of the [`ROSSELAND_INTERVALS`] quadrature points, rather than paying a full
/// size-distribution integral per quadrature point (which would be `ROSSELAND_INTERVALS` times as many Mie
/// evaluations). An engine-performance bound: the grid is fine enough that its interpolation error sits
/// below the quadrature's own, not world content.
const GRAIN_ROSS_GRID: usize = 32;

/// The ROSSELAND-MEAN grain opacity `kappa_R(T)` (cm^2/g), the single temperature-dependent number the disk
/// midplane solve reads, DERIVED by Rosseland-averaging the size-distribution-averaged spectral opacity
/// [`grain_size_averaged_opacity`] over the Planck frequency window. Each dimensionless frequency
/// `x = h*nu/(k_B*T)` maps to a wavelength `lambda = (h c / k_B) / (x T)`, so the spectral opacity is
/// sampled on a coarse log grid across the window and interpolated (in `ln lambda`, `ln kappa`) at each
/// quadrature point, then [`rosseland_mean`] takes the harmonic (transparency-weighted) average.
///
/// In the Rayleigh regime (a wavelength-independent index, so `kappa_abs ~ 1/lambda ~ x`) this scales
/// linearly with temperature, the derived `kappa_R ~ T` that a steeper `k(lambda)` would push toward the
/// `T^2` of the classic dust laws. All inputs are per-composition and per-distribution DATA (the alien and
/// distribution seams); `None` on a non-positive temperature, a bad distribution, or any overflow.
pub fn grain_rosseland_opacity(
    temperature_k: Fixed,
    n: Fixed,
    k: Fixed,
    bulk_density_g_cm3: Fixed,
    slope: Fixed,
    a_min_um: Fixed,
    a_max_um: Fixed,
) -> Option<Fixed> {
    // A wavelength-independent index is the constant closure; the spectral form is the general one below.
    grain_rosseland_opacity_spectral(
        temperature_k,
        |_lambda_um| Some((n, k)),
        bulk_density_g_cm3,
        slope,
        a_min_um,
        a_max_um,
    )
}

/// The ROSSELAND-MEAN grain opacity `kappa_R(T)` (cm^2/g) for a grain whose complex index VARIES with wavelength,
/// the generalization of [`grain_rosseland_opacity`] that the disposer-condensate wire needs: the effective
/// `(n, k)` of a mixed grain is a function of wavelength (each condensate's optical constants are wavelength
/// tables, and the effective-medium mix of them is evaluated at each wavelength), so the index is supplied as a
/// closure `index_at(lambda_um)` rather than a pair of constants. At each Rosseland-window wavelength the closure
/// returns the effective `(n, k)` there, the size-distribution average [`grain_size_averaged_opacity`] turns it
/// into a monochromatic opacity, and [`rosseland_mean`] takes the transparency-weighted average.
///
/// The wire (up-stack, in the materials crate, since it consumes the realized assemblage) supplies `index_at` as
/// the Rule-1 optical dispatch (measured constants where the species is in the library, the phonon estimator for
/// an alien phase) composed with the Rule-2 effective-medium topology (Maxwell-Garnett below the ice line,
/// Bruggeman above). This physics primitive stays composition-agnostic: it is handed an index-versus-wavelength
/// function and a shared size distribution and knows nothing of which condensates produced them (admit-the-alien
/// by construction). `None` on a non-positive temperature, a wavelength the closure cannot price, a bad
/// distribution, or any overflow.
pub fn grain_rosseland_opacity_spectral(
    temperature_k: Fixed,
    index_at: impl Fn(Fixed) -> Option<(Fixed, Fixed)>,
    bulk_density_g_cm3: Fixed,
    slope: Fixed,
    a_min_um: Fixed,
    a_max_um: Fixed,
) -> Option<Fixed> {
    if temperature_k <= Fixed::ZERO {
        return None;
    }
    // lambda(x, T) = (h c / k_B) / (x T), in microns (the same constant the Rayleigh term forms).
    let h = representation_bigrat("h")?;
    let c = representation_bigrat("c")?;
    let k_b = representation_bigrat("k_B")?;
    let alpha_um_k = h.mul(&c).div(&k_b).mul(&BigRat::from_i64(1_000_000));
    let t_br = nonneg_fixed_to_bigrat(temperature_k);

    // Precompute ln(kappa_abs) on a log-spaced x grid across the Rosseland window.
    let ln_x_min = rosseland_x_min().ln();
    let ln_x_max = rosseland_x_max().ln();
    let span = ln_x_max.checked_sub(ln_x_min)?;
    let mut ln_kappa = Vec::with_capacity(GRAIN_ROSS_GRID + 1);
    for j in 0..=GRAIN_ROSS_GRID {
        let ln_x = ln_x_min
            .checked_add(span.checked_mul(Fixed::from_ratio(j as i64, GRAIN_ROSS_GRID as i64))?)?;
        let x = ln_x.exp();
        let lam_br = alpha_um_k.div(&nonneg_fixed_to_bigrat(x).mul(&t_br));
        let lam = Fixed::from_bits_i128(lam_br.round_to_scale(Fixed::FRAC_BITS)?)?;
        let (n, k) = index_at(lam)?;
        let kappa =
            grain_size_averaged_opacity(lam, n, k, bulk_density_g_cm3, slope, a_min_um, a_max_um)?;
        ln_kappa.push(kappa.ln());
    }

    // Rosseland-average, interpolating ln(kappa) linearly in ln(x) at each quadrature frequency.
    rosseland_mean(|x| {
        let ln_x = x.ln();
        // Position on the uniform ln-x grid, clamped to the endpoints.
        let pos = ln_x
            .checked_sub(ln_x_min)?
            .checked_div(span)?
            .checked_mul(Fixed::from_int(GRAIN_ROSS_GRID as i32))?;
        let pos = if pos < Fixed::ZERO {
            Fixed::ZERO
        } else if pos > Fixed::from_int(GRAIN_ROSS_GRID as i32) {
            Fixed::from_int(GRAIN_ROSS_GRID as i32)
        } else {
            pos
        };
        let idx = (pos.to_int() as usize).min(GRAIN_ROSS_GRID - 1);
        let local = pos.checked_sub(Fixed::from_int(idx as i32))?;
        let lo = ln_kappa[idx];
        let hi = ln_kappa[idx + 1];
        Some(
            lo.checked_add(hi.checked_sub(lo)?.checked_mul(local)?)?
                .exp(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execution() -> SiExecutionMagnitudes {
        constants::canonical_si_execution_magnitudes().expect("the sealed floor projects")
    }

    fn electron_scattering_opacity(
        hydrogen_mass_fraction: Fixed,
        table: &PeriodicTable,
    ) -> Option<Fixed> {
        super::electron_scattering_opacity(&execution(), hydrogen_mass_fraction, table)
    }

    fn electron_scattering_opacity_from_electron_density(
        ln_electron_density_cm3: Fixed,
        ln_density_g_cm3: Fixed,
    ) -> Option<Fixed> {
        super::electron_scattering_opacity_from_electron_density(
            &execution(),
            ln_electron_density_cm3,
            ln_density_g_cm3,
        )
    }

    fn kramers_free_free_opacity(
        density_g_per_cm3: Fixed,
        temperature_k: Fixed,
        hydrogen_mass_fraction: Fixed,
        charge_weighted_abundance: Fixed,
        gaunt_factor: Fixed,
    ) -> Option<Fixed> {
        super::kramers_free_free_opacity(
            &execution(),
            density_g_per_cm3,
            temperature_k,
            hydrogen_mass_fraction,
            charge_weighted_abundance,
            gaunt_factor,
        )
    }

    fn free_free_prefactor(
        density_g_per_cm3: Fixed,
        temperature_k: Fixed,
        hydrogen_mass_fraction: Fixed,
        charge_weighted_abundance: Fixed,
        gaunt_factor: Fixed,
    ) -> Option<Fixed> {
        super::free_free_prefactor(
            &execution(),
            density_g_per_cm3,
            temperature_k,
            hydrogen_mass_fraction,
            charge_weighted_abundance,
            gaunt_factor,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn free_free_prefactor_ionized(
        ln_electron_density_cm3: Fixed,
        ln_sum_z2_ni_cm3: Fixed,
        ln_density_g_cm3: Fixed,
        density_g_per_cm3: Fixed,
        temperature_k: Fixed,
        hydrogen_mass_fraction: Fixed,
        charge_weighted_abundance: Fixed,
        gaunt_factor: Fixed,
    ) -> Option<Fixed> {
        super::free_free_prefactor_ionized(
            &execution(),
            ln_electron_density_cm3,
            ln_sum_z2_ni_cm3,
            ln_density_g_cm3,
            density_g_per_cm3,
            temperature_k,
            hydrogen_mass_fraction,
            charge_weighted_abundance,
            gaunt_factor,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn total_gas_rosseland_opacity(
        temperature_k: Fixed,
        density_g_per_cm3: Fixed,
        ln_density_g_cm3: Fixed,
        hydrogen_mass_fraction: Fixed,
        charge_weighted_abundance: Fixed,
        gaunt_factor: Fixed,
        ln_electron_density_cm3: Fixed,
        ln_sum_z2_ni_cm3: Fixed,
        electron_pressure_dyn_cm2: Fixed,
        table: &PeriodicTable,
    ) -> Option<Fixed> {
        super::total_gas_rosseland_opacity(
            &execution(),
            temperature_k,
            density_g_per_cm3,
            ln_density_g_cm3,
            hydrogen_mass_fraction,
            charge_weighted_abundance,
            gaunt_factor,
            ln_electron_density_cm3,
            ln_sum_z2_ni_cm3,
            electron_pressure_dyn_cm2,
            table,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn total_gas_and_grain_rosseland_opacity(
        temperature_k: Fixed,
        density_g_per_cm3: Fixed,
        ln_density_g_cm3: Fixed,
        hydrogen_mass_fraction: Fixed,
        charge_weighted_abundance: Fixed,
        gaunt_factor: Fixed,
        ln_electron_density_cm3: Fixed,
        ln_sum_z2_ni_cm3: Fixed,
        electron_pressure_dyn_cm2: Fixed,
        table: &PeriodicTable,
        grain_kappa_at: impl Fn(Fixed) -> Option<Fixed>,
    ) -> Option<Fixed> {
        super::total_gas_and_grain_rosseland_opacity(
            &execution(),
            temperature_k,
            density_g_per_cm3,
            ln_density_g_cm3,
            hydrogen_mass_fraction,
            charge_weighted_abundance,
            gaunt_factor,
            ln_electron_density_cm3,
            ln_sum_z2_ni_cm3,
            electron_pressure_dyn_cm2,
            table,
            grain_kappa_at,
        )
    }

    fn h_minus_bound_free_opacity(
        x: Fixed,
        temperature_k: Fixed,
        hydrogen_mass_fraction: Fixed,
        electron_pressure_dyn_cm2: Fixed,
        table: &PeriodicTable,
    ) -> Option<Fixed> {
        super::h_minus_bound_free_opacity(
            &execution(),
            x,
            temperature_k,
            hydrogen_mass_fraction,
            electron_pressure_dyn_cm2,
            table,
        )
    }

    fn h_minus_opacity(
        x: Fixed,
        temperature_k: Fixed,
        hydrogen_mass_fraction: Fixed,
        electron_pressure_dyn_cm2: Fixed,
        table: &PeriodicTable,
    ) -> Option<Fixed> {
        super::h_minus_opacity(
            &execution(),
            x,
            temperature_k,
            hydrogen_mass_fraction,
            electron_pressure_dyn_cm2,
            table,
        )
    }

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the periodic table loads")
    }

    #[test]
    fn the_solar_electron_scattering_opacity_reassembles_the_bell_lin_digit() {
        // The pre-registered 0.348 gate: at the solar hydrogen mass fraction X = 0.75 the derived electron-
        // scattering opacity is kappa_es = 0.1989 * 1.75 = 0.348 cm^2/g, reassembling Bell & Lin 1994 Table 3's
        // top regime (their 0.2(1+X) rounded) from the Thomson cross section and m_H alone, nothing fetched.
        let k = electron_scattering_opacity(Fixed::from_ratio(75, 100), &table())
            .expect("the opacity derives");
        assert!(
            (k.to_f64_lossy() - 0.348).abs() < 1e-3,
            "solar electron scattering is ~0.348 cm^2/g, got {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_electron_scattering_from_ne_reproduces_the_full_ionization_constant() {
        // The reassembly identity (the 0.348 row, generalized): kappa_es = sigma_T n_e/rho evaluated at the
        // fully-ionized electron density n_e = (1+X) rho/(2 m_H) must reproduce the restored 0.348(1+X) constant,
        // so the general n_e-linear form provably contains its old limit. X = 0.75, rho = 1e-6 g/cm^3:
        // n_e = 1.75/(2 * 1.674e-24) * 1e-6 = 5.228e17 cm^-3.
        let es_constant =
            electron_scattering_opacity(Fixed::from_ratio(75, 100), &table()).unwrap();
        let ln_ne_full = crate::saha::ln_of_decimal("5.228e17").unwrap();
        let ln_rho = crate::saha::ln_of_decimal("1e-6").unwrap();
        let es_from_ne =
            electron_scattering_opacity_from_electron_density(ln_ne_full, ln_rho).unwrap();
        assert!(
            (es_from_ne.to_f64_lossy() - es_constant.to_f64_lossy()).abs() < 0.01,
            "es(n_e) at full ionization reproduces 0.348(1+X) = {}, got {}",
            es_constant.to_f64_lossy(),
            es_from_ne.to_f64_lossy()
        );
    }

    #[test]
    fn the_electron_scattering_coefficient_is_the_thomson_value() {
        // At X = 0 the opacity is the bare coefficient sigma_T/(2 m_H) = 0.1989 cm^2/g (the pure-helium electron-
        // scattering value), and at X = 1 (pure ionized hydrogen) it is 2 * 0.1989 = 0.398. The coefficient is
        // the Thomson cross section over twice the hydrogen mass, derived, not the fitted 0.2.
        let bare = electron_scattering_opacity(Fixed::ZERO, &table()).unwrap();
        let pure_h = electron_scattering_opacity(Fixed::ONE, &table()).unwrap();
        assert!(
            (bare.to_f64_lossy() - 0.1989).abs() < 1e-3,
            "the bare coefficient is ~0.1989 cm^2/g, got {}",
            bare.to_f64_lossy()
        );
        assert!(
            (pure_h.to_f64_lossy() - 0.3978).abs() < 1e-3,
            "pure ionized hydrogen is ~0.398 cm^2/g, got {}",
            pure_h.to_f64_lossy()
        );
    }

    #[test]
    fn a_hydrogen_poor_plasma_scatters_less_admit_the_alien() {
        // X is the only per-world input: a hydrogen-poor plasma (lower X) has fewer electrons per unit mass and a
        // lower electron-scattering opacity, a data row, never a rewrite. The opacity rises monotonically with X.
        let poor = electron_scattering_opacity(Fixed::from_ratio(1, 10), &table()).unwrap();
        let solar = electron_scattering_opacity(Fixed::from_ratio(75, 100), &table()).unwrap();
        assert!(
            poor < solar,
            "a hydrogen-poor plasma scatters less than a solar one"
        );
    }

    #[test]
    fn the_opacity_is_a_deterministic_pure_derivation() {
        let t = table();
        let x = Fixed::from_ratio(75, 100);
        assert_eq!(
            electron_scattering_opacity(x, &t),
            electron_scattering_opacity(x, &t),
            "a pure derivation from the fundamentals replays byte for byte"
        );
    }

    #[test]
    fn the_rosseland_weight_peaks_near_the_planck_derivative_maximum() {
        // The Rosseland weighting w(x) = x^4 e^-x/(1-e^-x)^2 (the temperature derivative of the Planck function)
        // peaks near x ~ 3.8: it is larger at x = 4 than at x = 1 or x = 10, the shape that makes the Rosseland
        // mean weight the frequencies near the Planck-derivative maximum.
        let w1 = rosseland_weight(Fixed::from_int(1)).unwrap();
        let w4 = rosseland_weight(Fixed::from_int(4)).unwrap();
        let w10 = rosseland_weight(Fixed::from_int(10)).unwrap();
        assert!(w4 > w1 && w4 > w10, "the weight peaks near x ~ 4");
    }

    #[test]
    fn a_grey_opacity_rosseland_means_to_itself() {
        // The harmonic mean of a constant is the constant: a frequency-independent (grey) kappa_nu = kappa_0
        // Rosseland-averages back to kappa_0. This is the exact-recovery test that validates the quadrature kernel
        // against a known analytic answer, independent of any opacity physics.
        let kappa_0 = Fixed::from_int(5);
        let k = rosseland_mean(|_x| Some(kappa_0)).expect("a grey opacity has a Rosseland mean");
        assert!(
            (k.to_f64_lossy() - 5.0).abs() < 0.01,
            "a grey opacity Rosseland-means to itself (~5), got {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_rosseland_mean_is_bounded_by_the_opacity_range_and_favors_the_low_side() {
        // For a kappa_nu rising from kappa_0 (at low x) to 3*kappa_0 (at high x), the Rosseland (harmonic) mean
        // sits inside [kappa_0, 3*kappa_0] and below the midpoint 2*kappa_0: the harmonic mean weights the low-
        // opacity (transparent) frequencies, the physical reason a spectral window dominates the mean.
        let kappa_0 = 2.0;
        let k = rosseland_mean(|x| {
            // kappa_nu = kappa_0 * (1 + x/10): kappa_0 at x=0 up to ~3*kappa_0 at x=20.
            let factor = Fixed::ONE.checked_add(x.checked_div(Fixed::from_int(10))?)?;
            Fixed::from_ratio(2, 1).checked_mul(factor)
        })
        .expect("the mean resolves");
        let kf = k.to_f64_lossy();
        assert!(
            kf > kappa_0 && kf < 3.0 * kappa_0,
            "the Rosseland mean is inside the opacity range, got {kf}"
        );
        assert!(
            kf < 2.0 * kappa_0,
            "the harmonic mean favors the low-opacity side (below the midpoint), got {kf}"
        );
    }

    #[test]
    fn the_rosseland_mean_is_deterministic() {
        let k1 = rosseland_mean(|_| Some(Fixed::from_int(3)));
        let k2 = rosseland_mean(|_| Some(Fixed::from_int(3)));
        assert_eq!(k1, k2, "the bounded quadrature replays byte for byte");
    }

    #[test]
    fn a_non_positive_or_missing_opacity_fails_loud_rather_than_biasing_the_mean() {
        // The strict-positivity precondition (the gate's fix): a kappa_nu that is None or non-positive at any
        // quadrature point is an ERROR, not a transparent window, so the mean returns None rather than silently
        // dropping the point (which would leave the numerator over every x and the denominator over the kept x,
        // biasing kappa_R high). A real transparent window is kappa_nu -> 0+ (small positive), which the harmonic
        // sum w/kappa_nu handles on its own.
        let all_zero = rosseland_mean(|_| Some(Fixed::ZERO));
        assert_eq!(all_zero, None, "a non-positive opacity fails loud");
        let gappy = rosseland_mean(|x| {
            if x > Fixed::from_int(3) {
                None // a missing point past the weight peak
            } else {
                Some(Fixed::from_int(2))
            }
        });
        assert_eq!(
            gappy, None,
            "a missing frequency fails loud, it does not drop from the mean"
        );
    }

    #[test]
    fn a_power_law_opacity_rosseland_means_to_the_analytic_moment_ratio() {
        // Beyond the grey recovery (which is resolution-independent, so it checks only the harmonic-mean algebra),
        // a power-law kappa_nu = kappa_0 * x has a closed-form Rosseland mean, kappa_R = kappa_0 * J(4)/J(3) =
        // kappa_0 * 4*zeta(4)/zeta(3) ~ 3.6016 * kappa_0, where J(s) = Gamma(s+1) zeta(s) is the s-th moment of the
        // Planck-derivative weight over (0, inf). The 512-interval sum landing this (the truncation to [1/20, 20]
        // costs ~0.02%) is what proves the quadrature RESOLVES the integral, not merely the harmonic-mean algebra.
        let kappa_0 = Fixed::from_int(2);
        let k = rosseland_mean(|x| kappa_0.checked_mul(x)).expect("the mean resolves");
        let ratio = k.to_f64_lossy() / 2.0;
        assert!(
            (ratio - 3.6016).abs() < 0.036, // within 1% of 4*zeta(4)/zeta(3)
            "a power-law kappa_nu Rosseland-means to 4*zeta(4)/zeta(3) ~ 3.6016 * kappa_0, got ratio {ratio}"
        );
    }

    /// A hydrogen-helium plasma reference: X = 0.7, `<Z^2/A> = X + Y = 1.0` (Y = 0.3), Gaunt factor 1.
    fn solar_ff(rho: Fixed, t: Fixed) -> Fixed {
        kramers_free_free_opacity(rho, t, Fixed::from_ratio(7, 10), Fixed::ONE, Fixed::ONE)
            .expect("the free-free opacity derives")
    }

    #[test]
    fn the_free_free_shape_rosseland_averages_to_the_analytic_phi() {
        // Phi = rosseland_mean(free_free_shape) is DERIVED by the kernel, never a cited C_ff. It lands the closed
        // form (4*pi^4/15)/(2520*(zeta(6)+zeta(7))) ~ 5.0886e-3, so the free-free spectral shape is Rosseland-
        // averaged through the same kernel every opacity term reuses.
        let phi =
            rosseland_mean(free_free_shape).expect("the free-free shape has a Rosseland mean");
        assert!(
            (phi.to_f64_lossy() - 5.0886e-3).abs() < 5e-5,
            "the free-free shape Rosseland-averages to Phi ~ 5.09e-3, got {}",
            phi.to_f64_lossy()
        );
    }

    #[test]
    fn the_solar_free_free_opacity_lands_inside_the_kramers_envelope() {
        // Pre-registered acceptance gate: the DERIVED C_ff lands inside the cited [3.68e22, 3.8e22] textbook
        // envelope (Bell-Lin ~3.68e22 to KWW ~3.8e22), a consequence of the derivation, never an input. At
        // rho = 1e-6 g/cm^3, T = 1e4 K, comp = (1+X)<Z^2/A> = 1.7, the opacity is kappa_ff = C_ff * comp * rho *
        // T^-3.5, so the implied C_ff = kappa_ff / (comp * rho * T^-3.5) is the number under test.
        let rho = Fixed::from_ratio(1, 1_000_000);
        let t = Fixed::from_int(10_000);
        let kappa = solar_ff(rho, t);
        let comp = 1.7_f64;
        let c_ff = kappa.to_f64_lossy() / (comp * 1e-6 * 1e4_f64.powf(-3.5));
        assert!(
            (3.68e22..=3.8e22).contains(&c_ff),
            "the derived C_ff lands in the cited [3.68e22, 3.8e22] envelope, got {c_ff:e}"
        );
    }

    #[test]
    fn the_total_gas_opacity_joins_the_three_terms_on_one_saha_solve() {
        // The JOIN LAW end to end: ONE Saha solve at T = 6000 K (photospheric H with trace metal donors) sets the
        // electron density and pressure, and es (sigma_T n_e/rho), free-free (n_e sum Z^2 n_i), and H- (P_e) all
        // read that single solve. The monochromatic terms sum at each frequency and the whole is Rosseland-averaged.
        // In this hot ionized state the sum is positive, so the strict-positivity kernel succeeds and the assembly
        // sits at or above the n_e-linear electron-scattering floor.
        let tbl = table();
        let temp = Fixed::from_int(6000);
        let species = [
            ("H", crate::saha::ln_of_decimal("1e17").unwrap()),
            ("Na", crate::saha::ln_of_decimal("2e11").unwrap()),
            ("K", crate::saha::ln_of_decimal("1e10").unwrap()),
            ("Mg", crate::saha::ln_of_decimal("3e12").unwrap()),
            ("Ca", crate::saha::ln_of_decimal("2e11").unwrap()),
        ];
        let state = crate::saha::electron_density_saha(&execution(), temp, &species, &tbl).unwrap();
        assert!(
            !state.no_free_electrons,
            "the 6000 K gas has free electrons"
        );
        // rho ~ n_H m_H / X for n_H = 1e17, X = 0.7: ~2.4e-7 g/cm^3.
        let rho = Fixed::from_ratio(24, 100_000_000);
        let ln_rho = crate::saha::ln_of_decimal("2.4e-7").unwrap();
        let (x, z2a, g) = (
            Fixed::from_ratio(7, 10),
            Fixed::ONE,
            Fixed::from_ratio(12, 10),
        );
        // Single-stage: sum Z^2 n_i = n_e, so ln n_e feeds both the electron and the ion side (the general
        // interface, collapsed for the single-charge regime).
        let ln_ne = state.ln_electron_density_cm3;
        let total = total_gas_rosseland_opacity(
            temp,
            rho,
            ln_rho,
            x,
            z2a,
            g,
            ln_ne,
            ln_ne,
            state.electron_pressure_dyn_cm2,
            &tbl,
        )
        .expect("the hot-regime gas closure assembles");
        let es_floor = electron_scattering_opacity_from_electron_density(ln_ne, ln_rho).unwrap();
        assert!(
            total > Fixed::ZERO,
            "the assembled gas opacity is positive in the hot regime"
        );
        assert!(
            total >= es_floor,
            "the assembly is at least the n_e-linear electron-scattering floor ({} vs {})",
            total.to_f64_lossy(),
            es_floor.to_f64_lossy()
        );
        // Deterministic replay.
        assert_eq!(
            total,
            total_gas_rosseland_opacity(
                temp,
                rho,
                ln_rho,
                x,
                z2a,
                g,
                ln_ne,
                ln_ne,
                state.electron_pressure_dyn_cm2,
                &tbl,
            )
            .unwrap(),
            "the assembly replays byte for byte"
        );
    }

    #[test]
    fn the_grain_term_joins_the_monochromatic_sum() {
        // The gas-plus-grain join at the same 6000 K Saha state. A zero grain closure must reproduce the gas-only
        // Rosseland mean EXACTLY (the grain term enters as an additive monochromatic contribution, so adding zero is
        // an identity, the consistency the byte pins would otherwise have to guard). A positive grain closure must
        // raise the total: the grain opacity joins the sum at each frequency before the Rosseland average, never as
        // a separate mean added after.
        let tbl = table();
        let temp = Fixed::from_int(6000);
        let species = [
            ("H", crate::saha::ln_of_decimal("1e17").unwrap()),
            ("Na", crate::saha::ln_of_decimal("2e11").unwrap()),
            ("K", crate::saha::ln_of_decimal("1e10").unwrap()),
            ("Mg", crate::saha::ln_of_decimal("3e12").unwrap()),
            ("Ca", crate::saha::ln_of_decimal("2e11").unwrap()),
        ];
        let state = crate::saha::electron_density_saha(&execution(), temp, &species, &tbl).unwrap();
        let rho = Fixed::from_ratio(24, 100_000_000);
        let ln_rho = crate::saha::ln_of_decimal("2.4e-7").unwrap();
        let (x, z2a, g) = (
            Fixed::from_ratio(7, 10),
            Fixed::ONE,
            Fixed::from_ratio(12, 10),
        );
        let ln_ne = state.ln_electron_density_cm3;
        let gas_only = total_gas_rosseland_opacity(
            temp,
            rho,
            ln_rho,
            x,
            z2a,
            g,
            ln_ne,
            ln_ne,
            state.electron_pressure_dyn_cm2,
            &tbl,
        )
        .expect("the gas-only closure assembles");
        let with_zero_grains = total_gas_and_grain_rosseland_opacity(
            temp,
            rho,
            ln_rho,
            x,
            z2a,
            g,
            ln_ne,
            ln_ne,
            state.electron_pressure_dyn_cm2,
            &tbl,
            |_lambda| Some(Fixed::ZERO),
        )
        .expect("the gas-plus-grain closure assembles");
        assert_eq!(
            gas_only, with_zero_grains,
            "a zero grain term reproduces the gas-only Rosseland mean exactly"
        );
        let with_grains = total_gas_and_grain_rosseland_opacity(
            temp,
            rho,
            ln_rho,
            x,
            z2a,
            g,
            ln_ne,
            ln_ne,
            state.electron_pressure_dyn_cm2,
            &tbl,
            |_lambda| Some(Fixed::from_ratio(1, 10)),
        )
        .expect("the gas-plus-grain closure assembles with grains");
        assert!(
            with_grains > gas_only,
            "a positive monochromatic grain term raises the total ({} vs {})",
            with_grains.to_f64_lossy(),
            gas_only.to_f64_lossy()
        );
    }

    #[test]
    fn the_gas_closure_returns_none_in_the_cold_molecular_gap() {
        // The singularity the n_e-linear restatement exposes: with the grey electron-scattering floor gone, a cold
        // weakly-ionized gas (T = 1200 K, grains sublimating, H- not yet risen) has n_e -> 0 from the Saha solve,
        // so es and ff vanish and H- sleeps, the summed monochromatic opacity reaches zero, and the strict-
        // positivity Rosseland kernel returns None. That None is the MOLECULAR handoff signal (the Ferguson term's
        // window), not a failure of the ionized-gas closure.
        let tbl = table();
        let temp = Fixed::from_int(1200);
        let species = [
            ("H", crate::saha::ln_of_decimal("1e17").unwrap()),
            ("K", crate::saha::ln_of_decimal("1e10").unwrap()),
        ];
        let state = crate::saha::electron_density_saha(&execution(), temp, &species, &tbl).unwrap();
        let rho = Fixed::from_ratio(24, 100_000_000);
        let ln_rho = crate::saha::ln_of_decimal("2.4e-7").unwrap();
        let ln_ne = state.ln_electron_density_cm3;
        let total = total_gas_rosseland_opacity(
            temp,
            rho,
            ln_rho,
            Fixed::from_ratio(7, 10),
            Fixed::ONE,
            Fixed::from_ratio(12, 10),
            ln_ne,
            ln_ne,
            state.electron_pressure_dyn_cm2,
            &tbl,
        );
        assert!(
            total.is_none(),
            "the cold molecular gap has no ionized-gas opacity: the closure hands off to the molecular term, got {:?}",
            total.map(|k| k.to_f64_lossy())
        );
    }

    #[test]
    fn the_free_free_opacity_scales_as_density_and_inverse_temperature() {
        // kappa_ff ~ rho * T^(-7/2): doubling the density doubles the opacity (linear), and raising the temperature
        // lowers it steeply. This is the Kramers signature the disk-thermal profile reads.
        let rho = Fixed::from_ratio(1, 1_000_000);
        let base = solar_ff(rho, Fixed::from_int(10_000));
        let double_rho = solar_ff(Fixed::from_ratio(2, 1_000_000), Fixed::from_int(10_000));
        let hotter = solar_ff(rho, Fixed::from_int(20_000));
        let ratio = double_rho.to_f64_lossy() / base.to_f64_lossy();
        assert!(
            (ratio - 2.0).abs() < 0.02,
            "doubling the density doubles the free-free opacity, got ratio {ratio}"
        );
        assert!(
            hotter.to_f64_lossy() < base.to_f64_lossy(),
            "a hotter plasma has a lower free-free opacity (T^-7/2)"
        );
    }

    #[test]
    fn a_hydrogen_poor_plasma_has_a_lower_free_free_opacity_admit_the_alien() {
        // Composition is caller data, not a rewrite: a hydrogen-poor plasma (lower X, a lower 1+X electron factor)
        // has a lower free-free opacity at the same <Z^2/A>. The alien is a data row.
        let rho = Fixed::from_ratio(1, 1_000_000);
        let t = Fixed::from_int(10_000);
        let solar = solar_ff(rho, t);
        let poor =
            kramers_free_free_opacity(rho, t, Fixed::from_ratio(1, 10), Fixed::ONE, Fixed::ONE)
                .expect("the free-free opacity derives");
        assert!(
            poor.to_f64_lossy() < solar.to_f64_lossy(),
            "a hydrogen-poor plasma has a lower free-free opacity"
        );
    }

    #[test]
    fn the_free_free_opacity_is_deterministic() {
        let rho = Fixed::from_ratio(1, 1_000_000);
        let t = Fixed::from_int(10_000);
        let a = kramers_free_free_opacity(rho, t, Fixed::from_ratio(7, 10), Fixed::ONE, Fixed::ONE);
        let b = kramers_free_free_opacity(rho, t, Fixed::from_ratio(7, 10), Fixed::ONE, Fixed::ONE);
        assert_eq!(a, b, "the free-free derivation replays byte for byte");
    }

    #[test]
    fn the_ionized_free_free_prefactor_reproduces_the_kramers_limit_at_full_ionization() {
        // The reassembly identity for free-free: fed the fully-ionized electron density (pure hydrogen X = 1, where
        // single-stage ionization IS full ionization and n_e = sum Z^2 n_i = rho/m_u = rho * N_A), the partial-
        // ionization prefactor reproduces free_free_prefactor exactly (ratio = 1), the general n_e * sum Z^2 n_i
        // form provably containing the Kramers limit.
        let rho = Fixed::from_ratio(1, 1_000_000); // 1e-6 g/cm^3
        let t = Fixed::from_int(100_000); // hot, fully ionized
        let x = Fixed::ONE; // pure hydrogen
        let cwa = Fixed::ONE; // <Z^2/A> = 1 for pure hydrogen
        let g = Fixed::ONE;
        let ln_rho = crate::saha::ln_of_decimal("1e-6").unwrap();
        let ln_na = crate::saha::ln_si_representation("N_A").unwrap();
        // (1+X)/2 = 1 at X = 1, so ln n_e_full = ln rho + ln N_A; sum Z^2 n_i = n_e (single-stage = full for H).
        let ln_ne_full = ln_rho + ln_na;
        let a_ff_full = free_free_prefactor(rho, t, x, cwa, g).unwrap();
        let a_ff_ionized =
            free_free_prefactor_ionized(ln_ne_full, ln_ne_full, ln_rho, rho, t, x, cwa, g).unwrap();
        let rel = (a_ff_ionized.to_f64_lossy() - a_ff_full.to_f64_lossy()).abs()
            / a_ff_full.to_f64_lossy();
        assert!(
            rel < 1e-2,
            "at full ionization the partial-ionization prefactor reproduces Kramers: full {}, ionized {}",
            a_ff_full.to_f64_lossy(),
            a_ff_ionized.to_f64_lossy()
        );
    }

    #[test]
    fn the_ionized_free_free_prefactor_is_quadratic_in_ionization_fraction() {
        // Free-free carries n_e * sum Z^2 n_i, so at ionization fraction x = 0.1 (n_e and the single-stage ion side
        // both 0.1 of full) the prefactor is 0.01 of the fully-ionized value: QUADRATIC in x, the two-body electron-
        // ion signature that keeps a cool weakly-ionized gas from radiating like a full plasma.
        let rho = Fixed::from_ratio(1, 1_000_000);
        let t = Fixed::from_int(100_000);
        let x = Fixed::ONE;
        let cwa = Fixed::ONE;
        let g = Fixed::ONE;
        let ln_rho = crate::saha::ln_of_decimal("1e-6").unwrap();
        let ln_na = crate::saha::ln_si_representation("N_A").unwrap();
        let ln_ne_full = ln_rho + ln_na;
        let ln_tenth = crate::saha::ln_of_decimal("0.1").unwrap();
        let ln_ne_tenth = ln_ne_full + ln_tenth; // n_e = 0.1 n_e_full, single-stage so sum Z^2 n_i = n_e
        let a_ff_full =
            free_free_prefactor_ionized(ln_ne_full, ln_ne_full, ln_rho, rho, t, x, cwa, g).unwrap();
        let a_ff_tenth =
            free_free_prefactor_ionized(ln_ne_tenth, ln_ne_tenth, ln_rho, rho, t, x, cwa, g)
                .unwrap();
        let ratio = a_ff_tenth.to_f64_lossy() / a_ff_full.to_f64_lossy();
        assert!(
            (ratio - 0.01).abs() < 1e-3,
            "free-free is quadratic in ionization fraction: 0.1^2 = 0.01, got {ratio}"
        );
    }

    #[test]
    fn the_h_minus_cross_section_reproduces_the_wishart_peak() {
        // The standing provenance gate (the gate's condition 2): the cited John 1988 coefficients MUST reproduce
        // the primary Wishart 1979 photodetachment peak, 3.994e-17 cm^2 (reduced 39.94) at 8513 A. Any corruption
        // of the transcribed coefficients fails this, so it is the build-time physics check that the secondary-
        // transcribed [M] column is faithful to the primary cross section.
        let peak =
            h_minus_bound_free_reduced_cross_section(Fixed::from_ratio(8513, 10000)).unwrap();
        assert!(
            (peak.to_f64_lossy() - 39.9355).abs() < 0.05,
            "the H- cross section reproduces the Wishart peak ~39.94 (x1e-18 cm^2) at 8513 A, got {}",
            peak.to_f64_lossy()
        );
        // and 8513 A IS the peak: above the values at 0.4 and 1.2 micron.
        let short = h_minus_bound_free_reduced_cross_section(Fixed::from_ratio(4, 10)).unwrap();
        let long = h_minus_bound_free_reduced_cross_section(Fixed::from_ratio(12, 10)).unwrap();
        assert!(
            peak > short && peak > long,
            "8513 A is the cross-section peak (above 0.4 and 1.2 micron)"
        );
    }

    #[test]
    fn the_h_minus_cross_section_is_zero_outside_the_photodetachment_domain() {
        // No photodetachment beyond the binding threshold lambda_0 = 1.6419 micron (a longer-wavelength photon
        // lacks the energy to detach the electron), and the fit is undefined below 0.125 micron; the cross section
        // reads zero in both, which is why the bound-free term cannot be Rosseland-averaged in isolation (its below-
        // threshold transparent window is filled by the free-free term at assembly).
        assert_eq!(
            h_minus_bound_free_reduced_cross_section(Fixed::from_ratio(17, 10)),
            Some(Fixed::ZERO),
            "no photodetachment beyond the 1.6419 micron threshold"
        );
        assert_eq!(
            h_minus_bound_free_reduced_cross_section(Fixed::from_ratio(1, 10)),
            Some(Fixed::ZERO),
            "the fit is undefined below 0.125 micron"
        );
        let inside =
            h_minus_bound_free_reduced_cross_section(Fixed::from_ratio(8513, 10000)).unwrap();
        assert!(
            inside > Fixed::ZERO,
            "the cross section is positive inside the photodetachment domain"
        );
    }

    #[test]
    fn the_h_minus_cross_section_is_deterministic() {
        let lam = Fixed::from_ratio(8513, 10000);
        assert_eq!(
            h_minus_bound_free_reduced_cross_section(lam),
            h_minus_bound_free_reduced_cross_section(lam),
            "the cross section replays byte for byte"
        );
    }

    #[test]
    fn the_h_minus_bound_free_opacity_lands_the_derived_saha_magnitude() {
        // At the 0.8513 micron cross-section peak (x = 2.817 for T = 6000 K), X = 0.7, P_e = 10 dyn/cm^2, the
        // derived bound-free opacity is ~0.182 cm^2/g: the John 1988 0.74989 Saha prefactor (reassembled from the
        // fundamentals) times the cited cross section times the stimulated-emission bracket, a magnitude fixed by
        // physics, nothing fit.
        let k = h_minus_bound_free_opacity(
            Fixed::from_ratio(2817, 1000),
            Fixed::from_int(6000),
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &table(),
        )
        .expect("the bound-free opacity derives");
        assert!(
            (k.to_f64_lossy() - 0.1815).abs() < 0.005,
            "the H- bound-free opacity at the peak is ~0.182 cm^2/g, got {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_h_minus_bound_free_opacity_scales_with_pressure_and_hydrogen() {
        // Linear in the electron pressure (the Saha H- population per P_e) and in the hydrogen mass fraction (the
        // neutral-H reservoir): doubling either doubles the opacity.
        let t = table();
        let x = Fixed::from_ratio(2817, 1000);
        let temp = Fixed::from_int(6000);
        let base = h_minus_bound_free_opacity(
            x,
            temp,
            Fixed::from_ratio(35, 100),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        let double_pe = h_minus_bound_free_opacity(
            x,
            temp,
            Fixed::from_ratio(35, 100),
            Fixed::from_int(20),
            &t,
        )
        .unwrap();
        let double_x = h_minus_bound_free_opacity(
            x,
            temp,
            Fixed::from_ratio(70, 100),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        assert!(
            (double_pe.to_f64_lossy() / base.to_f64_lossy() - 2.0).abs() < 0.02,
            "the bound-free opacity is linear in the electron pressure"
        );
        assert!(
            (double_x.to_f64_lossy() / base.to_f64_lossy() - 2.0).abs() < 0.02,
            "the bound-free opacity is linear in the hydrogen mass fraction"
        );
    }

    #[test]
    fn the_h_minus_bound_free_opacity_is_zero_below_the_photodetachment_threshold() {
        // At x = 1.0 for T = 6000 K the wavelength is 2.4 micron, beyond the 1.6419 micron binding threshold, so
        // the cross section is zero and the bound-free opacity is zero: the transparent window the free-free term
        // fills at assembly (which is why the bound-free cannot be Rosseland-averaged in isolation).
        let k = h_minus_bound_free_opacity(
            Fixed::from_int(1),
            Fixed::from_int(6000),
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &table(),
        );
        assert_eq!(
            k,
            Some(Fixed::ZERO),
            "no bound-free opacity below the photodetachment threshold"
        );
    }

    #[test]
    fn the_h_minus_bound_free_opacity_is_deterministic() {
        let t = table();
        let a = h_minus_bound_free_opacity(
            Fixed::from_ratio(2817, 1000),
            Fixed::from_int(6000),
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        );
        let b = h_minus_bound_free_opacity(
            Fixed::from_ratio(2817, 1000),
            Fixed::from_int(6000),
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        );
        assert_eq!(a, b, "the H- bound-free derivation replays byte for byte");
    }

    #[test]
    fn the_h_minus_cross_section_agrees_with_the_bhatia_pesnell_calculation() {
        // Cross-SOURCE check (the gate values these): the John/Wishart bound-free cross section vs the INDEPENDENT
        // Ohmura-Ohmura calculation tabulated by Bhatia & Pesnell 2020 (Atoms 8(3), 37, Table 1). In the well-
        // determined peak region the two agree to a few percent (John 39.2/39.6 vs BP 41.5/41.3 reduced, ~4-6%),
        // a faithfulness confirmation beyond the Wishart peak gate; the wings differ more (~14%, the cruder
        // Ohmura-Ohmura analytic vs Wishart's close-coupling), so this anchors the peak region.
        let at_7594 =
            h_minus_bound_free_reduced_cross_section(Fixed::from_ratio(7594, 10000)).unwrap();
        let at_9113 =
            h_minus_bound_free_reduced_cross_section(Fixed::from_ratio(9113, 10000)).unwrap();
        assert!(
            (at_7594.to_f64_lossy() - 41.5).abs() / 41.5 < 0.08,
            "John bf cross section agrees with Bhatia-Pesnell at 7594 A within 8%, got {}",
            at_7594.to_f64_lossy()
        );
        assert!(
            (at_9113.to_f64_lossy() - 41.3).abs() / 41.3 < 0.08,
            "John bf cross section agrees with Bhatia-Pesnell at 9113 A within 8%, got {}",
            at_9113.to_f64_lossy()
        );
    }

    #[test]
    fn the_h_minus_free_free_opacity_lands_the_reference_magnitude() {
        // At lambda = 3 micron (x = 0.799 for T = 6000 K, pure free-free beyond the bound-free threshold), X = 0.7,
        // P_e = 10, the derived free-free opacity is ~0.205 cm^2/g: the cited John eq.6 coefficient (order 1e-26
        // cm^4/dyn, the Bhatia-Pesnell Fig.1 magnitude anchor) times (X/m_H) P_e.
        let k = h_minus_free_free_opacity(
            Fixed::from_ratio(799, 1000),
            Fixed::from_int(6000),
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &table(),
        )
        .expect("the free-free opacity derives");
        assert!(
            (k.to_f64_lossy() - 0.2052).abs() < 0.01,
            "the H- free-free opacity at 3 micron is ~0.205 cm^2/g, got {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_h_minus_free_free_opacity_rises_to_long_wavelength_and_scales_with_pressure() {
        // The free-free is continuous and rises to the infrared (lower x = longer wavelength), the shape that fills
        // the below-threshold window; and it is linear in the electron pressure.
        let t = table();
        let temp = Fixed::from_int(6000);
        let x_hi = Fixed::from_ratio(4, 10); // ~6 micron
        let x_lo = Fixed::from_ratio(8, 10); // ~3 micron
        let long = h_minus_free_free_opacity(
            x_hi,
            temp,
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        let short = h_minus_free_free_opacity(
            x_lo,
            temp,
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        assert!(
            long > short,
            "the free-free opacity rises to long wavelength (the infrared window filler)"
        );
        let double_pe = h_minus_free_free_opacity(
            x_lo,
            temp,
            Fixed::from_ratio(7, 10),
            Fixed::from_int(20),
            &t,
        )
        .unwrap();
        assert!(
            (double_pe.to_f64_lossy() / short.to_f64_lossy() - 2.0).abs() < 0.02,
            "the free-free opacity is linear in the electron pressure"
        );
    }

    #[test]
    fn the_h_minus_free_free_opacity_is_zero_below_the_short_wavelength_bound() {
        // At x = 20 for T = 6000 K the wavelength is 0.12 micron, below the fit's 0.1823 micron short-wavelength
        // bound, so the free-free reads zero (and there the bound-free is also zero, the far-UV where H- is not the
        // opacity and the electron-scattering floor is; this is the fail-loud seam noted at h_minus_opacity).
        let k = h_minus_free_free_opacity(
            Fixed::from_int(20),
            Fixed::from_int(6000),
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &table(),
        );
        assert_eq!(
            k,
            Some(Fixed::ZERO),
            "no free-free below the fit's short-wavelength bound"
        );
    }

    #[test]
    fn the_h_minus_free_free_opacity_is_deterministic() {
        let t = table();
        let a = h_minus_free_free_opacity(
            Fixed::from_ratio(799, 1000),
            Fixed::from_int(6000),
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        );
        let b = h_minus_free_free_opacity(
            Fixed::from_ratio(799, 1000),
            Fixed::from_int(6000),
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        );
        assert_eq!(a, b, "the H- free-free derivation replays byte for byte");
    }

    #[test]
    fn the_h_minus_opacity_has_its_minimum_near_the_photodetachment_threshold() {
        // The pre-registered 1.64 micron battery row: the assembled bf+ff H- opacity has its MINIMUM near the
        // 1.6419 micron photodetachment threshold, where the bound-free (peaking near 0.85 micron) has cut off and
        // the free-free (rising to the infrared) has not yet climbed. This is the famous H- opacity window that lets
        // us see deepest into a star near 1.6 micron. At T = 6000 K, x = (hc/k)/(lambda T) = 2.398/lambda_um, so
        // lambda = 1.0, 1.6419, 3.0 micron are x = 2.398, 1.4605, 0.799.
        let t = table();
        let temp = Fixed::from_int(6000);
        let (x_val, p_e) = (Fixed::from_ratio(7, 10), Fixed::from_int(100));
        let short = h_minus_opacity(Fixed::from_ratio(2398, 1000), temp, x_val, p_e, &t).unwrap();
        let minimum =
            h_minus_opacity(Fixed::from_ratio(14605, 10000), temp, x_val, p_e, &t).unwrap();
        let long = h_minus_opacity(Fixed::from_ratio(799, 1000), temp, x_val, p_e, &t).unwrap();
        assert!(
            minimum < short,
            "H- opacity at the 1.64 micron threshold is below the bound-free peak side ({} vs {})",
            minimum.to_f64_lossy(),
            short.to_f64_lossy()
        );
        assert!(
            minimum < long,
            "H- opacity at the 1.64 micron threshold is below the infrared free-free side ({} vs {})",
            minimum.to_f64_lossy(),
            long.to_f64_lossy()
        );
    }

    fn optics() -> crate::optical_constants::OpticalConstants {
        crate::optical_constants::OpticalConstants::standard().expect("the optical library loads")
    }

    #[test]
    fn the_rayleigh_grain_opacity_lands_the_silicate_magnitude() {
        // At the 10 micron Si-O band (x = 2.398 for T = 600 K), silicate (bulk density 3.49 g/cm^3), the Rayleigh
        // mass absorption is ~2673 cm^2/g: the analytic Im[(m^2-1)/(m^2+2)] over the cited Draine n,k, times
        // 6*pi/(rho*lambda). A magnitude fixed by the optical constants and the density, nothing tuned.
        let lib = optics();
        let sil = lib.species("astronomical_silicate").unwrap();
        let k = rayleigh_grain_opacity(
            Fixed::from_ratio(2398, 1000),
            Fixed::from_int(600),
            Fixed::from_ratio(349, 100),
            sil,
        )
        .expect("the grain opacity derives");
        assert!(
            (k.to_f64_lossy() - 2673.0).abs() < 40.0,
            "silicate Rayleigh opacity at 10 micron is ~2673 cm^2/g, got {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_rayleigh_grain_opacity_falls_as_lambda_squared_in_the_far_infrared() {
        // The beta=2 small-grain law's basis: where the far-IR follows the Lorentz wing (k ~ 1/lambda), the Rayleigh
        // opacity falls as lambda^-2, so kappa(100um)/kappa(300um) ~ 3^2 = 9 (a lambda ratio of 3). This is the
        // spectral slope that Rosseland-averages to the kappa_R ~ T^2 law.
        let lib = optics();
        let sil = lib.species("astronomical_silicate").unwrap();
        let rho = Fixed::from_ratio(349, 100);
        // lambda = alpha/(x T); at T = 100, x = 1.4388 -> 100 micron, x = 0.4796 -> 300 micron.
        let at_100 = rayleigh_grain_opacity(
            Fixed::from_ratio(14388, 10000),
            Fixed::from_int(100),
            rho,
            sil,
        )
        .unwrap();
        let at_300 = rayleigh_grain_opacity(
            Fixed::from_ratio(4796, 10000),
            Fixed::from_int(100),
            rho,
            sil,
        )
        .unwrap();
        let ratio = at_100.to_f64_lossy() / at_300.to_f64_lossy();
        assert!(
            (7.0..=13.0).contains(&ratio),
            "the far-IR opacity falls ~lambda^-2 (ratio ~9-10 over a lambda ratio of 3), got {ratio}"
        );
    }

    #[test]
    fn a_metal_grain_absorbs_far_less_than_a_silicate_admit_the_alien() {
        // Composition is species data: a metal (iron) has a small Rayleigh ABSORPTION (it scatters more than it
        // absorbs, the large (m^2+2) denominator), far below a silicate at the same wavelength. The alien is a
        // species row, not a rewrite.
        let lib = optics();
        let sil = lib.species("astronomical_silicate").unwrap();
        let fe = lib.species("metallic_iron").unwrap();
        let x = Fixed::from_ratio(2398, 1000);
        let t = Fixed::from_int(600);
        let k_sil = rayleigh_grain_opacity(x, t, Fixed::from_ratio(349, 100), sil).unwrap();
        let k_fe = rayleigh_grain_opacity(x, t, Fixed::from_ratio(787, 100), fe).unwrap();
        assert!(
            k_fe < k_sil,
            "a metal grain absorbs far less than a silicate in the Rayleigh limit ({} vs {})",
            k_fe.to_f64_lossy(),
            k_sil.to_f64_lossy()
        );
    }

    #[test]
    fn the_rayleigh_grain_opacity_is_none_outside_the_species_coverage() {
        // A wavelength beyond the species' sampled coverage (here ~2877 micron, past silicate's 2000 micron ceiling)
        // returns None, an honest coverage gap the assembly handles rather than extrapolating the measured table.
        let lib = optics();
        let sil = lib.species("astronomical_silicate").unwrap();
        let k = rayleigh_grain_opacity(
            Fixed::from_ratio(1, 10),
            Fixed::from_int(50),
            Fixed::from_ratio(349, 100),
            sil,
        );
        assert_eq!(
            k, None,
            "outside the sampled coverage the grain opacity is None"
        );
    }

    #[test]
    fn the_rayleigh_grain_opacity_is_deterministic() {
        let lib = optics();
        let sil = lib.species("astronomical_silicate").unwrap();
        let x = Fixed::from_ratio(2398, 1000);
        let t = Fixed::from_int(600);
        let rho = Fixed::from_ratio(349, 100);
        assert_eq!(
            rayleigh_grain_opacity(x, t, rho, sil),
            rayleigh_grain_opacity(x, t, rho, sil),
            "the grain opacity replays byte for byte"
        );
    }

    #[test]
    fn the_assembled_h_minus_opacity_shows_the_opacity_minimum_at_the_threshold() {
        // The famous H- opacity MINIMUM near the 1.6419 micron photodetachment threshold, the qualitative feature
        // no single coefficient can fake (Bhatia-Pesnell 2020 place the total minimum just past threshold): the
        // assembled bf+ff opacity is lower just past the threshold (x = 1.454, ~1.65 micron, bound-free cut off,
        // free-free still small) than at the bound-free peak (x = 2.82, ~0.85 micron) or deep in the free-free
        // infrared (x = 0.799, ~3 micron). And beyond the threshold the opacity is PURE free-free.
        let t = table();
        let temp = Fixed::from_int(6000);
        let x_peak = Fixed::from_ratio(282, 100); // ~0.85 micron, bound-free peak
        let x_min = Fixed::from_ratio(1454, 1000); // ~1.65 micron, just past threshold
        let x_ff = Fixed::from_ratio(799, 1000); // ~3 micron, pure free-free
        let at_peak = h_minus_opacity(
            x_peak,
            temp,
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        let at_min = h_minus_opacity(
            x_min,
            temp,
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        let at_ff = h_minus_opacity(
            x_ff,
            temp,
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        assert!(
            at_min < at_peak && at_min < at_ff,
            "the assembled H- opacity has its minimum near the threshold (min {} < peak {}, ff {})",
            at_min.to_f64_lossy(),
            at_peak.to_f64_lossy(),
            at_ff.to_f64_lossy()
        );
        // Beyond the threshold the bound-free is exactly zero, so the H- opacity is the free-free alone.
        let bf = h_minus_bound_free_opacity(
            x_ff,
            temp,
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        let ff = h_minus_free_free_opacity(
            x_ff,
            temp,
            Fixed::from_ratio(7, 10),
            Fixed::from_int(10),
            &t,
        )
        .unwrap();
        assert_eq!(
            bf,
            Fixed::ZERO,
            "the bound-free is zero beyond the threshold"
        );
        assert_eq!(
            at_ff, ff,
            "beyond the threshold the assembled H- opacity is the free-free alone"
        );
    }

    // The Mie references below are the BHMIE (Bohren and Huffman 1983, Appendix A) values for Q_abs,
    // cross-checked against the standard f64 implementation. The kernel runs in wide-fixed arithmetic, so it
    // reproduces them to within the ~1e-9 seed difference propagated through the recurrence.

    #[test]
    fn the_mie_q_abs_matches_the_bhmie_reference_at_unit_size() {
        // x = 1, m = 1.5 + 0.01i: a weakly absorbing dielectric at the resonance-onset size. Q_abs = 0.02884.
        let q = mie_q_abs(
            Fixed::from_int(1),
            Fixed::from_ratio(150, 100),
            Fixed::from_ratio(1, 100),
        )
        .expect("a valid Mie size returns Q_abs");
        assert!(
            (q.to_f64_lossy() - 0.02884).abs() < 1e-3,
            "Q_abs(x=1, m=1.5+0.01i) ~ 0.02884, got {}",
            q.to_f64_lossy()
        );
    }

    #[test]
    fn the_mie_q_abs_matches_the_bhmie_reference_in_the_resonance_region() {
        // x = 3, m = 1.5 + 0.1i: the interference-and-absorption region, Q_abs = 0.89525.
        let q3 = mie_q_abs(
            Fixed::from_int(3),
            Fixed::from_ratio(150, 100),
            Fixed::from_ratio(10, 100),
        )
        .expect("a valid Mie size returns Q_abs");
        assert!(
            (q3.to_f64_lossy() - 0.89525).abs() < 2e-3,
            "Q_abs(x=3, m=1.5+0.1i) ~ 0.89525, got {}",
            q3.to_f64_lossy()
        );
        // x = 6, m = 1.68 + 0.03i (an olivine-like index), Q_abs = 0.80626.
        let q6 = mie_q_abs(
            Fixed::from_int(6),
            Fixed::from_ratio(168, 100),
            Fixed::from_ratio(3, 100),
        )
        .expect("a valid Mie size returns Q_abs");
        assert!(
            (q6.to_f64_lossy() - 0.80626).abs() < 2e-3,
            "Q_abs(x=6, m=1.68+0.03i) ~ 0.80626, got {}",
            q6.to_f64_lossy()
        );
    }

    #[test]
    fn the_mie_q_abs_scales_linearly_in_the_rayleigh_regime() {
        // For x << 1 the absorption efficiency is the dipole (Rayleigh) form Q_abs = 4x Im[(m^2-1)/(m^2+2)],
        // linear in x with an index-only prefactor. So doubling x from 0.1 to 0.2 doubles Q_abs. This is the
        // reduction that a wrong 1/x scaling (the classic BHMIE off-by-one) fails by orders of magnitude.
        let m = (Fixed::from_ratio(150, 100), Fixed::from_ratio(1, 100));
        let q1 = mie_q_abs(Fixed::from_ratio(1, 10), m.0, m.1).unwrap();
        let q2 = mie_q_abs(Fixed::from_ratio(2, 10), m.0, m.1).unwrap();
        let ratio = q2.to_f64_lossy() / q1.to_f64_lossy();
        assert!(
            (ratio - 2.0).abs() < 0.05,
            "Q_abs doubles from x=0.1 to x=0.2 in the Rayleigh regime, ratio {}",
            ratio
        );
        assert!(
            q1.to_f64_lossy() > 0.0 && q1.to_f64_lossy() < 0.01,
            "the Rayleigh-regime Q_abs is small and positive, got {}",
            q1.to_f64_lossy()
        );
    }

    #[test]
    fn the_mie_q_abs_is_deterministic() {
        let (x, n, k) = (
            Fixed::from_int(3),
            Fixed::from_ratio(150, 100),
            Fixed::from_ratio(10, 100),
        );
        assert_eq!(
            mie_q_abs(x, n, k),
            mie_q_abs(x, n, k),
            "the wide-fixed Mie recurrence replays byte for byte"
        );
    }

    #[test]
    fn the_mie_q_abs_is_none_outside_its_validated_range() {
        let n = Fixed::from_ratio(150, 100);
        let k = Fixed::from_ratio(1, 100);
        // Below MIE_X_MIN (x = 0.1) the Rayleigh form is used instead.
        assert_eq!(
            mie_q_abs(Fixed::from_ratio(5, 100), n, k),
            None,
            "below the range the kernel declines (Rayleigh handles it)"
        );
        // Above MIE_X_SWITCH (x = 50) the geometric limit is used instead.
        assert_eq!(
            mie_q_abs(Fixed::from_int(60), n, k),
            None,
            "above the range the kernel declines (the geometric limit handles it)"
        );
    }

    #[test]
    fn the_mie_q_abs_rejects_a_non_physical_index() {
        // n <= 0 or k < 0 is not a physical refractive index.
        assert_eq!(
            mie_q_abs(Fixed::from_int(3), Fixed::ZERO, Fixed::from_ratio(1, 100)),
            None,
            "a zero real index is rejected"
        );
        assert_eq!(
            mie_q_abs(
                Fixed::from_int(3),
                Fixed::from_ratio(150, 100),
                Fixed::from_ratio(-1, 100)
            ),
            None,
            "a negative absorption index is rejected"
        );
    }

    // The size-integral references below are the size-distribution-averaged silicate-like grain opacity
    // (m = 1.68 + 0.03i, rho = 3.3 g/cm^3, MRN slope 3.5, a in [0.005, 0.25] micron), computed by the same
    // integral composed over the f64 BHMIE. The Rust runs it over Fixed exp/ln and the wide-fixed Mie, so
    // it reproduces them to a few parts in 1e4.
    fn silicate_grain() -> (Fixed, Fixed, Fixed, Fixed, Fixed, Fixed) {
        (
            Fixed::from_ratio(168, 100), // n
            Fixed::from_ratio(3, 100),   // k
            Fixed::from_ratio(33, 10),   // rho g/cm^3
            Fixed::from_ratio(35, 10),   // MRN slope
            Fixed::from_ratio(5, 1000),  // a_min micron
            Fixed::from_ratio(25, 100),  // a_max micron
        )
    }

    #[test]
    fn the_grain_opacity_lands_the_silicate_magnitude_across_the_spectrum() {
        let (n, k, rho, p, amin, amax) = silicate_grain();
        // Optical (0.5 micron): the geometric-ish plateau of small grains, ~2832 cm^2/g.
        let k_opt = grain_size_averaged_opacity(Fixed::from_ratio(5, 10), n, k, rho, p, amin, amax)
            .unwrap();
        assert!(
            (k_opt.to_f64_lossy() - 2832.16).abs() < 2.0,
            "kappa_abs(0.5um) ~ 2832 cm^2/g, got {}",
            k_opt.to_f64_lossy()
        );
        // Mid-IR (10 micron), ~74.5 cm^2/g.
        let k_mir =
            grain_size_averaged_opacity(Fixed::from_int(10), n, k, rho, p, amin, amax).unwrap();
        assert!(
            (k_mir.to_f64_lossy() - 74.487).abs() < 0.2,
            "kappa_abs(10um) ~ 74.5 cm^2/g, got {}",
            k_mir.to_f64_lossy()
        );
    }

    #[test]
    fn the_grain_opacity_falls_as_one_over_lambda_at_constant_index() {
        // With a wavelength-independent index every grain is in the size-independent Rayleigh regime at
        // long wavelength, where kappa_abs = 6 pi Im/(rho lambda), so it falls as 1/lambda: a factor-ten
        // wavelength is a factor-ten drop. The steeper lambda^-2 of real dust comes from k(lambda) falling,
        // a wavelength-dependent index, not from this constant-index kernel.
        let (n, k, rho, p, amin, amax) = silicate_grain();
        let k100 = grain_size_averaged_opacity(Fixed::from_int(100), n, k, rho, p, amin, amax)
            .unwrap()
            .to_f64_lossy();
        let k1000 = grain_size_averaged_opacity(Fixed::from_int(1000), n, k, rho, p, amin, amax)
            .unwrap()
            .to_f64_lossy();
        let ratio = k100 / k1000;
        assert!(
            (ratio - 10.0).abs() < 0.1,
            "the far-IR opacity falls as 1/lambda (ratio ~ 10), got {}",
            ratio
        );
    }

    #[test]
    fn the_grain_opacity_is_lower_for_a_less_dense_or_less_absorbing_grain() {
        // kappa_abs ~ 1/rho, so a less dense grain of the same optics has a higher opacity; and a lower
        // absorption index k lowers Im and so the opacity. Both key on the per-composition data, the alien
        // seam: an ice, a metal, or an exotic condensate is a different set of arguments, never a rewrite.
        let (n, k, rho, p, amin, amax) = silicate_grain();
        let base = grain_size_averaged_opacity(Fixed::from_int(100), n, k, rho, p, amin, amax)
            .unwrap()
            .to_f64_lossy();
        let denser = grain_size_averaged_opacity(
            Fixed::from_int(100),
            n,
            k,
            Fixed::from_int(7),
            p,
            amin,
            amax,
        )
        .unwrap()
        .to_f64_lossy();
        let less_absorbing = grain_size_averaged_opacity(
            Fixed::from_int(100),
            n,
            Fixed::from_ratio(1, 100),
            rho,
            p,
            amin,
            amax,
        )
        .unwrap()
        .to_f64_lossy();
        assert!(denser < base, "a denser grain has a lower mass opacity");
        assert!(
            less_absorbing < base,
            "a less absorbing grain (lower k) has a lower opacity"
        );
    }

    #[test]
    fn the_grain_opacity_is_deterministic() {
        let (n, k, rho, p, amin, amax) = silicate_grain();
        let lam = Fixed::from_int(10);
        assert_eq!(
            grain_size_averaged_opacity(lam, n, k, rho, p, amin, amax),
            grain_size_averaged_opacity(lam, n, k, rho, p, amin, amax),
            "the size-distribution quadrature replays byte for byte"
        );
    }

    #[test]
    fn the_grain_opacity_rejects_bad_arguments() {
        let (n, k, rho, p, amin, amax) = silicate_grain();
        assert_eq!(
            grain_size_averaged_opacity(Fixed::ZERO, n, k, rho, p, amin, amax),
            None,
            "a non-positive wavelength is rejected"
        );
        assert_eq!(
            grain_size_averaged_opacity(Fixed::from_int(10), n, k, Fixed::ZERO, p, amin, amax),
            None,
            "a non-positive density is rejected"
        );
        assert_eq!(
            grain_size_averaged_opacity(Fixed::from_int(10), n, k, rho, p, amax, amin),
            None,
            "a_max <= a_min is rejected"
        );
    }

    // The Bruggeman references below are the effective (n, k) of a mixed grain, cross-checked against the
    // exact two-component quadratic and the N-component Newton solve in f64.
    fn sil_index() -> (Fixed, Fixed) {
        (Fixed::from_ratio(168, 100), Fixed::from_ratio(3, 100))
    }
    fn ice_index() -> (Fixed, Fixed) {
        (Fixed::from_ratio(131, 100), Fixed::from_ratio(1, 1_000_000))
    }
    fn iron_index() -> (Fixed, Fixed) {
        (Fixed::from_ratio(35, 10), Fixed::from_int(4))
    }

    #[test]
    fn the_bruggeman_mix_matches_the_two_component_quadratic() {
        // A 50/50 silicate-ice mix: the effective index sits between the two, n ~ 1.491, k ~ 0.0144. This
        // is the case with a closed-form quadratic root, so it pins the Newton solve.
        let half = Fixed::from_ratio(5, 10);
        let (n, k) = bruggeman_effective_index(&[half, half], &[sil_index(), ice_index()]).unwrap();
        assert!(
            (n.to_f64_lossy() - 1.4912).abs() < 1e-3,
            "effective n ~ 1.491, got {}",
            n.to_f64_lossy()
        );
        assert!(
            (k.to_f64_lossy() - 0.014426).abs() < 1e-3,
            "effective k ~ 0.0144, got {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_bruggeman_mix_of_one_component_is_that_component() {
        // A single component at any fraction returns its own index (the root of f_i(eps_i - eps)/... = 0 is
        // eps_i itself), the reduction that validates the solve against the trivial case.
        let (n, k) = bruggeman_effective_index(&[Fixed::from_int(1)], &[sil_index()]).unwrap();
        assert!(
            (n.to_f64_lossy() - 1.68).abs() < 1e-4 && (k.to_f64_lossy() - 0.03).abs() < 1e-4,
            "a pure component returns itself, got n={} k={}",
            n.to_f64_lossy(),
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_bruggeman_mix_grows_absorbing_with_the_metal_fraction() {
        // Mixing in a metal (high k iron) raises the effective absorption index monotonically: the alien
        // seam is a data change (a different fraction of a different measured row), never a rewrite.
        let mut last_k = -1.0;
        for pct in [0i64, 5, 20, 40] {
            let f = Fixed::from_ratio(pct, 100);
            let one_minus = Fixed::from_int(1).checked_sub(f).unwrap();
            let (_, k) =
                bruggeman_effective_index(&[one_minus, f], &[sil_index(), iron_index()]).unwrap();
            assert!(
                k.to_f64_lossy() > last_k,
                "the effective k rises with the metal fraction"
            );
            last_k = k.to_f64_lossy();
        }
    }

    #[test]
    fn the_bruggeman_mix_is_scale_free_in_the_fractions() {
        // The Bruggeman root is unchanged if every fraction is scaled (the equation is homogeneous in f),
        // so unnormalized fractions give the same effective index as their normalized form.
        let a = bruggeman_effective_index(
            &[Fixed::from_ratio(6, 10), Fixed::from_ratio(4, 10)],
            &[sil_index(), iron_index()],
        )
        .unwrap();
        let b = bruggeman_effective_index(
            &[Fixed::from_int(3), Fixed::from_int(2)],
            &[sil_index(), iron_index()],
        )
        .unwrap();
        assert_eq!(
            a, b,
            "scaling all fractions leaves the effective index unchanged"
        );
    }

    #[test]
    fn the_bruggeman_mix_is_deterministic() {
        let third = Fixed::from_ratio(1, 3);
        assert_eq!(
            bruggeman_effective_index(
                &[third, third, third],
                &[sil_index(), ice_index(), iron_index()]
            ),
            bruggeman_effective_index(
                &[third, third, third],
                &[sil_index(), ice_index(), iron_index()]
            ),
            "the Newton solve replays byte for byte"
        );
    }

    #[test]
    fn the_bruggeman_mix_rejects_bad_arguments() {
        assert_eq!(
            bruggeman_effective_index(&[], &[]),
            None,
            "an empty mix is rejected"
        );
        assert_eq!(
            bruggeman_effective_index(&[Fixed::from_int(1)], &[sil_index(), ice_index()]),
            None,
            "a length mismatch is rejected"
        );
        assert_eq!(
            bruggeman_effective_index(&[Fixed::from_ratio(-1, 10)], &[sil_index()]),
            None,
            "a negative fraction is rejected"
        );
    }

    #[test]
    fn the_maxwell_garnett_mix_matches_the_closed_form() {
        // A dielectric matrix n_m = 1.5 (eps_m = 2.25) with a single inclusion n_i = 3.0 (eps_i = 9.0) at volume
        // fraction f = 0.5: beta = 0.5 (9 - 2.25)/(9 + 4.5) = 0.25, eps_eff = 2.25 (1 + 0.5)/(1 - 0.25) = 4.5, so
        // n_eff = sqrt(4.5) = 2.1213. Pins the closed-form solve against the hand computation.
        let (n, k) = maxwell_garnett_effective_index(
            (Fixed::from_ratio(3, 2), Fixed::ZERO),
            &[Fixed::from_ratio(1, 2)],
            &[(Fixed::from_int(3), Fixed::ZERO)],
        )
        .unwrap();
        assert!(
            (n.to_f64_lossy() - 2.1213).abs() < 1e-3,
            "Maxwell-Garnett n_eff ~ 2.1213, got {}",
            n.to_f64_lossy()
        );
        assert!(
            k.to_f64_lossy() < 1e-3,
            "a pure-dielectric mix stays non-absorbing, got k {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_maxwell_garnett_mix_of_no_inclusions_is_the_bare_matrix() {
        // An empty inclusion list (or all-zero fractions) returns the matrix index unchanged: beta = 0, eps_eff =
        // eps_m.
        let empty =
            maxwell_garnett_effective_index(sil_index(), &[], &[]).expect("no inclusions is valid");
        assert!(
            (empty.0.to_f64_lossy() - sil_index().0.to_f64_lossy()).abs() < 1e-4
                && (empty.1.to_f64_lossy() - sil_index().1.to_f64_lossy()).abs() < 1e-4,
            "no inclusions returns the bare matrix, got {:?}",
            (empty.0.to_f64_lossy(), empty.1.to_f64_lossy())
        );
        let zero = maxwell_garnett_effective_index(sil_index(), &[Fixed::ZERO], &[iron_index()])
            .expect("a zero fraction is valid");
        assert!(
            (zero.0.to_f64_lossy() - sil_index().0.to_f64_lossy()).abs() < 1e-4,
            "an all-zero inclusion fraction returns the bare matrix, got n {}",
            zero.0.to_f64_lossy()
        );
    }

    #[test]
    fn the_maxwell_garnett_topology_differs_from_bruggeman() {
        // Rule 2's whole point: the aggregation topology matters. Iron inclusions in an ice matrix
        // (Maxwell-Garnett, the below-ice-line mantle structure) give a DIFFERENT effective index from the same
        // iron and ice mixed symmetrically (Bruggeman, the above-ice-line bare mixture), the factor-level
        // distinction the condensation history writes.
        let f = Fixed::from_ratio(3, 10);
        let one_minus = Fixed::from_ratio(7, 10);
        let mg = maxwell_garnett_effective_index(ice_index(), &[f], &[iron_index()]).unwrap();
        let brugg =
            bruggeman_effective_index(&[one_minus, f], &[ice_index(), iron_index()]).unwrap();
        assert!(
            (mg.0.to_f64_lossy() - brugg.0.to_f64_lossy()).abs() > 1e-2
                || (mg.1.to_f64_lossy() - brugg.1.to_f64_lossy()).abs() > 1e-2,
            "the two topologies give different effective indices: MG {:?} vs Bruggeman {:?}",
            (mg.0.to_f64_lossy(), mg.1.to_f64_lossy()),
            (brugg.0.to_f64_lossy(), brugg.1.to_f64_lossy())
        );
        // Iron inclusions raise the absorption above the bare ice matrix (k_eff > k_ice).
        assert!(
            mg.1.to_f64_lossy() > ice_index().1.to_f64_lossy(),
            "iron inclusions make the ice-matrix grain absorbing, k_eff {}",
            mg.1.to_f64_lossy()
        );
    }

    #[test]
    fn the_maxwell_garnett_mix_rejects_bad_arguments() {
        assert_eq!(
            maxwell_garnett_effective_index(sil_index(), &[Fixed::ONE], &[]),
            None,
            "a length mismatch is rejected"
        );
        assert_eq!(
            maxwell_garnett_effective_index(
                (Fixed::ZERO, Fixed::ZERO),
                &[Fixed::ONE],
                &[iron_index()]
            ),
            None,
            "a non-physical matrix index is rejected"
        );
        assert_eq!(
            maxwell_garnett_effective_index(
                sil_index(),
                &[Fixed::from_ratio(-1, 10)],
                &[ice_index()]
            ),
            None,
            "a negative inclusion fraction is rejected"
        );
    }

    #[test]
    fn the_maxwell_garnett_mix_is_deterministic() {
        let f = Fixed::from_ratio(2, 10);
        assert_eq!(
            maxwell_garnett_effective_index(ice_index(), &[f], &[iron_index()]),
            maxwell_garnett_effective_index(ice_index(), &[f], &[iron_index()]),
            "the closed-form solve replays byte for byte"
        );
    }

    #[test]
    fn the_grain_rosseland_opacity_scales_with_temperature() {
        // For a wavelength-independent index, kappa_abs ~ 1/lambda ~ x, so the Rosseland mean scales
        // linearly with temperature: doubling T doubles kappa_R. The reference (silicate-like grain) is
        // ~74.7 cm^2/g at 400 K, matched against the same average composed over the f64 BHMIE.
        let (n, k, rho, p, amin, amax) = silicate_grain();
        let k400 = grain_rosseland_opacity(Fixed::from_int(400), n, k, rho, p, amin, amax)
            .unwrap()
            .to_f64_lossy();
        let k800 = grain_rosseland_opacity(Fixed::from_int(800), n, k, rho, p, amin, amax)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (k400 - 74.7).abs() < 2.0,
            "kappa_R(400 K) ~ 74.7 cm^2/g, got {}",
            k400
        );
        assert!(
            (k800 / k400 - 2.0).abs() < 0.06,
            "doubling T doubles the Rosseland-mean opacity, ratio {}",
            k800 / k400
        );
    }

    #[test]
    fn the_grain_rosseland_opacity_is_deterministic_and_guards_temperature() {
        let (n, k, rho, p, amin, amax) = silicate_grain();
        let t = Fixed::from_int(300);
        assert_eq!(
            grain_rosseland_opacity(t, n, k, rho, p, amin, amax),
            grain_rosseland_opacity(t, n, k, rho, p, amin, amax),
            "the Rosseland-mean grain opacity replays byte for byte"
        );
        assert_eq!(
            grain_rosseland_opacity(Fixed::ZERO, n, k, rho, p, amin, amax),
            None,
            "a non-positive temperature is rejected"
        );
    }
}
