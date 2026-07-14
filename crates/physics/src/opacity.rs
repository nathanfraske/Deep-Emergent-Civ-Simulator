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

use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use civsim_units::bignum::{BigRat, BigUint};
use civsim_units::compute;
use civsim_units::fundamentals;

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

/// A fundamental constant's CODATA value as an exact rational, read from the single fundamentals register (never
/// re-authored here). `None` if the symbol is not a registered fundamental.
fn fundamental_bigrat(symbol: &str) -> Option<BigRat> {
    BigRat::from_decimal_str(fundamentals::fundamental(symbol)?.value).ok()
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
    hydrogen_mass_fraction: Fixed,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let e = fundamental_bigrat("e")?;
    let eps_0 = fundamental_bigrat("eps_0")?;
    let m_e = fundamental_bigrat("m_e")?;
    let c = fundamental_bigrat("c")?;
    let n_a = fundamental_bigrat("N_A")?;
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

/// The KRAMERS FREE-FREE (bremsstrahlung) Rosseland opacity `kappa_ff` (cm^2/g), DERIVED from the fundamentals
/// through the shared Rosseland kernel, never read from the Bell-Lin fit. A free electron accelerating past an ion
/// radiates a continuum; the bound-free-corrected absorption coefficient carries the spectral shape
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
    density_g_per_cm3: Fixed,
    temperature_k: Fixed,
    hydrogen_mass_fraction: Fixed,
    charge_weighted_abundance: Fixed,
    gaunt_factor: Fixed,
) -> Option<Fixed> {
    let e = fundamental_bigrat("e")?;
    let eps_0 = fundamental_bigrat("eps_0")?;
    let m_e = fundamental_bigrat("m_e")?;
    let h = fundamental_bigrat("h")?;
    let c = fundamental_bigrat("c")?;
    let k_b = fundamental_bigrat("k_B")?;
    let n_a = fundamental_bigrat("N_A")?;
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
    // T^(-1/2), leaving a clean rational), then a single Fixed::sqrt. T^7 = T^4 * T^2 * T.
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
/// derivable at this level; this is the measured [M] tier, like the grain optical constants).
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
