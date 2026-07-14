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

//! The multi-species SAHA free-electron-density solve (`n_e`), the single plasma-ionization state the disk-opacity
//! gas terms join onto: electron scattering (linear in `n_e`), free-free (the two-body `n_e * n_i` product), and
//! H- (through its electron pressure `P_e = n_e k T`) all read ONE `n_e` from ONE solve, so three implicit electron
//! densities cannot silently disagree (the join law). The same `n_e` is the future ionosphere / MRI-dead-zone lever,
//! a second banked consumer that reads THIS solver, not a sibling.
//!
//! CALCULABILITY (why this is log-space, the census pattern). The Saha equilibrium
//! `n_ion n_e / n_neutral = 2 (U_ion/U_neutral) (2 pi m_e k T / h^2)^(3/2) exp(-chi / k T) = S(T)`
//! carries three separate Q32.32 range violations: the quantum concentration `(2 pi m_e k T/h^2)^(3/2) ~ 1e21/m^3`
//! and `n_e ~ 1e19/m^3` overflow the ~2e9 ceiling by decades; the Boltzmann factor `exp(-chi/kT)` underflows to
//! zero for the large `chi/kT` of a cool gas; and even the CONSTANTS underflow (`m_e ~ 9e-31`, `h ~ 6.6e-34` sit
//! far below the ~2.3e-10 fixed-point floor), so `Fixed::ln` cannot be taken of them directly. The whole solve is
//! therefore in the LOG domain: `ln S(T)` is assembled from log-constants (this module's [`ln_of_decimal`], which
//! recovers `ln C = ln(mantissa) + exp * ln 10` from a constant's decimal string), the charge-neutrality root is
//! found by a BOUNDED bisection in `ln n_e` (charge neutrality is monotone in `n_e`, so the root is unique and the
//! bisection is fold-free and iteration-capped), and only the representable `P_e = n_e k T` is exported. The pinned
//! transcendentals ([`Fixed::exp`], [`Fixed::ln`]) and the fixed iteration count keep it deterministic.

use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use civsim_units::fundamentals;

/// The natural log of a decimal-string magnitude, for constants that overflow or underflow `Fixed` and so cannot
/// be `Fixed::ln`-ed directly (`m_e ~ 9e-31`, `N_A ~ 6e23`). It splits the mantissa from the base-ten exponent
/// (`"9.1093837015e-31" -> mantissa 9.109..., exponent -31`) and returns `ln(mantissa) + exponent * ln 10`: the
/// mantissa sits in `[1, 10)` (or is the whole plain integer, still representable) so its `Fixed::ln` is well
/// defined, and the exponent term carries the decades the raw value cannot hold. `None` if the string does not
/// parse or the mantissa is non-positive.
pub fn ln_of_decimal(s: &str) -> Option<Fixed> {
    let (mantissa_str, exp) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.trim().parse::<i32>().ok()?),
        None => (s, 0),
    };
    let mantissa = Fixed::from_decimal_str(mantissa_str.trim()).ok()?;
    if mantissa <= Fixed::ZERO {
        return None;
    }
    let ln_ten = Fixed::from_int(10).ln();
    Some(mantissa.ln() + Fixed::from_int(exp).mul(ln_ten))
}

/// The natural log of a registered fundamental constant's value (its underflow-safe log, via [`ln_of_decimal`]).
/// `None` if the symbol is not a registered fundamental.
pub fn ln_fundamental(symbol: &str) -> Option<Fixed> {
    ln_of_decimal(fundamentals::fundamental(symbol)?.value)
}

/// A deterministic, canonical `log(exp(a) + exp(b))` for the log-domain sums the Saha charge-neutrality root needs
/// (each ionization term and their sum live in the log domain). Written `hi + ln(1 + exp(lo - hi))` with `hi` the
/// larger operand, so `lo - hi <= 0` and the inner `exp` is in `(0, 1]` (never overflows, underflows harmlessly to
/// zero when one term dominates, giving `hi`). The fixed hi-then-lo ordering makes it associative-stable and
/// order-independent (the canonical-logsumexp determinism rule).
pub fn log_sum_exp(a: Fixed, b: Fixed) -> Fixed {
    let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
    // lo - hi <= 0; a saturating subtract only matters at the representable rails, which the log domain avoids.
    let d = lo - hi;
    hi + (Fixed::ONE + d.exp()).ln()
}

/// The eV-to-kelvin factor `e / k_B` (K/eV, ~11605): both `e` and `k_B` underflow `Fixed`, but their ratio is
/// representable, so the Boltzmann argument `chi/(k T) = chi_eV * (e/k_B) / T` is formed from this ratio rather
/// than from the underflowing individual constants. Computed once in exact `BigRat`.
fn ev_to_kelvin() -> Option<Fixed> {
    let e = BigRat::from_decimal_str(fundamentals::fundamental("e")?.value).ok()?;
    let k = BigRat::from_decimal_str(fundamentals::fundamental("k_B")?.value).ok()?;
    Fixed::from_bits_i128(e.div(&k).round_to_scale(Fixed::FRAC_BITS)?)
}

/// The GROUND-STATE statistical weights `(g_neutral, g_ion)` for the Saha partition-function ratio,
/// GROUND-STATE-ONLY: at disk temperatures the fine structure of the ground term is unresolved (its splitting is
/// far below `k T`) so the ground TERM's multiplicity is the partition function, and the excited terms (an eV up)
/// are unpopulated. Definition-tagged so a mixed-convention weight cannot silently join the shared `n_e`: `g` is
/// the ground-term multiplicity from the atomic term symbol, cited to the NIST Atomic Spectra Database ground
/// levels. Hydrogen and the alkalis are `2S` doublet neutrals ionizing to closed-shell singlet ions
/// (`g0 = 2, g+ = 1`); helium and the alkaline earths are the reverse (`g0 = 1, g+ = 2`). `None` for a species
/// with no pinned convention, so the solve EXCLUDES it (a rung the metal donors join once cited) rather than
/// guessing a weight.
fn ground_state_degeneracies(symbol: &str) -> Option<(Fixed, Fixed)> {
    let (g0, gp): (i32, i32) = match symbol {
        "H" | "Li" | "Na" | "K" | "Rb" | "Cs" => (2, 1),
        "He" | "Be" | "Mg" | "Ca" | "Sr" | "Ba" => (1, 2),
        _ => return None,
    };
    Some((Fixed::from_int(g0), Fixed::from_int(gp)))
}

/// The log of the single-ionization SAHA function `ln S(T)` for one species, where `S = n_ion n_e / n_neutral`,
/// assembled in the log domain from the underflow-safe log-constants:
/// `ln S = ln(2 g+/g0) + (3/2) [ln 2pi + ln m_e + ln k_B + ln T - 2 ln h] - chi_eV (e/k_B) / T`,
/// with `S` in SI number density (per cubic metre, matching the number densities the charge-neutrality solve
/// sums). The `2 g+/g0` is the electron-spin-times-partition-ratio, `(2 pi m_e k_B T/h^2)^(3/2)` is the quantum
/// concentration, and the last term is the Boltzmann factor formed from the representable `e/k_B` ratio and the
/// FIRST ionization energy read from the measured [`PeriodicTable`] (the periodic table carries the first IE per
/// element; the successive-IE ladder is a separate transition-metal column). `None` if the species lacks a pinned
/// degeneracy convention or a first ionization energy, or on a non-positive temperature.
pub fn ln_saha_factor(symbol: &str, temperature_k: Fixed, table: &PeriodicTable) -> Option<Fixed> {
    if temperature_k <= Fixed::ZERO {
        return None;
    }
    let (g0, gp) = ground_state_degeneracies(symbol)?;
    let chi_ev = table.element(symbol)?.ionization_energy?;
    let g_factor = Fixed::from_int(2).mul(gp).checked_div(g0)?.ln();
    // (3/2) ln(2 pi m_e k_B T / h^2), each log underflow-safe.
    let ln_2pi = ln_of_decimal("6.283185307")?;
    let quantum = ln_2pi + ln_fundamental("m_e")? + ln_fundamental("k_B")? + temperature_k.ln()
        - Fixed::from_int(2).mul(ln_fundamental("h")?);
    let ln_quantum_concentration = Fixed::from_ratio(3, 2).mul(quantum);
    // chi/(k T) = chi_eV * (e/k_B) / T, from the representable ratio.
    let boltzmann = chi_ev.mul(ev_to_kelvin()?).checked_div(temperature_k)?;
    Some(g_factor + ln_quantum_concentration - boltzmann)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn ln_of_decimal_recovers_the_underflowing_constant_logs() {
        // The constants overflow/underflow Q32.32, so their logs come from the mantissa-exponent split, not
        // Fixed::ln of the raw value. ln(m_e = 9.109e-31) = ln(9.109) + (-31) ln 10 ~ -69.17; ln(k_B = 1.38e-23)
        // ~ -52.64; ln(N_A = 6.022e23) ~ +54.75. All far outside the representable value range yet representable as
        // logs.
        assert!(
            close(ln_of_decimal("9.1093837015e-31").unwrap(), -69.17, 0.02),
            "ln(m_e) ~ -69.17, got {}",
            ln_of_decimal("9.1093837015e-31").unwrap().to_f64_lossy()
        );
        assert!(
            close(ln_of_decimal("1.380649e-23").unwrap(), -52.637, 0.02),
            "ln(k_B) ~ -52.64, got {}",
            ln_of_decimal("1.380649e-23").unwrap().to_f64_lossy()
        );
        assert!(
            close(ln_of_decimal("6.02214076e23").unwrap(), 54.749, 0.02),
            "ln(N_A) ~ 54.75, got {}",
            ln_of_decimal("6.02214076e23").unwrap().to_f64_lossy()
        );
    }

    #[test]
    fn ln_of_decimal_handles_a_plain_integer_and_reads_the_register() {
        // A plain integer (no exponent) with a value that fits: ln(299792458) ~ 19.52. And the register read.
        assert!(
            close(ln_of_decimal("299792458").unwrap(), 19.518, 0.01),
            "ln(c) ~ 19.52, got {}",
            ln_of_decimal("299792458").unwrap().to_f64_lossy()
        );
        assert_eq!(ln_fundamental("m_e"), ln_of_decimal("9.1093837015e-31"));
        assert_eq!(ln_fundamental("not_a_constant"), None);
    }

    #[test]
    fn log_sum_exp_is_correct_canonical_and_order_independent() {
        // log(exp(a) + exp(b)): for a = b = 0, log(1 + 1) = ln 2 ~ 0.693; and it is symmetric in its arguments
        // (the canonical hi-then-lo ordering), and reduces to the dominant term when one is far larger.
        assert!(
            close(
                log_sum_exp(Fixed::ZERO, Fixed::ZERO),
                std::f64::consts::LN_2,
                1e-3
            ),
            "logsumexp(0,0) = ln 2, got {}",
            log_sum_exp(Fixed::ZERO, Fixed::ZERO).to_f64_lossy()
        );
        let a = Fixed::from_int(5);
        let b = Fixed::from_int(2);
        assert_eq!(
            log_sum_exp(a, b),
            log_sum_exp(b, a),
            "logsumexp is order-independent (canonical)"
        );
        // log(exp(5) + exp(2)) = 5 + ln(1 + exp(-3)) ~ 5.0486.
        assert!(
            close(log_sum_exp(a, b), 5.0486, 1e-3),
            "logsumexp(5,2) ~ 5.049, got {}",
            log_sum_exp(a, b).to_f64_lossy()
        );
        // A dominant term: log(exp(30) + exp(0)) ~ 30 (the small term underflows harmlessly).
        assert!(
            close(log_sum_exp(Fixed::from_int(30), Fixed::ZERO), 30.0, 1e-6),
            "a dominant term returns itself"
        );
    }

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the periodic table loads")
    }

    #[test]
    fn the_saha_factor_lands_hydrogen_and_makes_potassium_the_readier_donor() {
        // ln S(H, 6000 K) = ln(2 g+/g0) + (3/2) ln(2pi m_e k_B T/h^2) - chi/(kT), hand-checked: the g factor is 0
        // (2*1/2 = 1), the quantum-concentration log ~62.29, the Boltzmann term 13.6 * 11605 / 6000 ~26.30, so
        // ln S ~35.99.
        let t = table();
        let ln_s = ln_saha_factor("H", Fixed::from_int(6000), &t).unwrap();
        assert!(
            close(ln_s, 35.99, 0.3),
            "ln S(H, 6000K) ~ 35.99, got {}",
            ln_s.to_f64_lossy()
        );
        // The inner-disk character: potassium (IE 4.34 eV) ionizes far more readily than hydrogen (13.6 eV) at the
        // same cool temperature, so it feeds the electron budget below ~3000 K. ln S(K) >> ln S(H).
        let ln_k = ln_saha_factor("K", Fixed::from_int(3000), &t).unwrap();
        let ln_h = ln_saha_factor("H", Fixed::from_int(3000), &t).unwrap();
        assert!(
            ln_k > ln_h,
            "K ionizes more readily than H at 3000 K: ln S(K) {} > ln S(H) {}",
            ln_k.to_f64_lossy(),
            ln_h.to_f64_lossy()
        );
        // A species with no pinned degeneracy convention is excluded, not guessed.
        assert_eq!(ln_saha_factor("Xx", Fixed::from_int(6000), &t), None);
    }
}
