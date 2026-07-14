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

use civsim_core::Fixed;
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
}
