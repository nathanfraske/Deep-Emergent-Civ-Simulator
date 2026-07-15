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
/// `ln S = ln(2 g+/g0) + (3/2) [ln 2pi + ln m_e + ln k_B + ln T - 2 ln h] - 6 ln 10 - chi_eV (e/k_B) / T`.
/// UNIT PIN: `S` is CGS number density (per cubic CENTIMETRE), the single system the export `P_e` (dyn/cm^2) and
/// the H- / stellar-atmospheres literature (Chandrasekhar, Gray, Wishart) live in, so no mixed-unit `P_e` can
/// reach the H- consumer; the SI form (per cubic metre) is `6 ln 10 = 13.816` larger, so `ln S(H, 6000 K) = 22.17`
/// here versus `35.99` in SI. The `2 g+/g0` is the electron-spin-times-partition-ratio, `(2 pi m_e k_B T/h^2)^(3/2)`
/// the quantum concentration, and the last term the Boltzmann factor formed from the representable `e/k_B` ratio and
/// the
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
    // (3/2) ln(...) is per cubic metre; subtract 6 ln 10 to land per cubic centimetre (the cgs join system).
    let ln_quantum_concentration =
        Fixed::from_ratio(3, 2).mul(quantum) - Fixed::from_int(6).mul(Fixed::from_int(10).ln());
    // chi/(k T) = chi_eV * (e/k_B) / T, from the representable ratio.
    let boltzmann = chi_ev.mul(ev_to_kelvin()?).checked_div(temperature_k)?;
    Some(g_factor + ln_quantum_concentration - boltzmann)
}

/// The number of bisection steps the charge-neutrality root takes: a FIXED count (the determinism bound, the
/// `SURFACE_BALANCE_ITERS` model), over an 80-e-fold `ln n_e` bracket, so the bracket collapses far below the
/// `Fixed` resolution and any count at or above it gives the identical root. Not world content.
const SAHA_ITERS: u32 = 72;

/// The width in e-folds of the `ln n_e` bracket below full ionization: the root of the weakly-ionized branch,
/// `n_e ~ sqrt(sum N_i S_i)`, never falls more than this far below `ln(sum N_i)` for any state that has electrons
/// at resolution (the zero-electron short-circuit takes the states that would).
const SAHA_BRACKET_EFOLDS: i32 = 80;

/// The resolved outcome of the multi-species Saha solve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SahaState {
    /// The free-electron number density `ln n_e` (natural log of the per-cm^3 value; `n_e` itself overflows
    /// `Fixed` in a warm plasma, so it is carried as its log).
    pub ln_electron_density_cm3: Fixed,
    /// The electron PRESSURE `P_e = n_e k_B T` (dyn/cm^2, cgs), the representable export the H- opacity and the
    /// stellar-atmospheres literature parameterize by. Zero under the no-free-electrons verdict.
    pub electron_pressure_dyn_cm2: Fixed,
    /// The rider-1 SINGLE-IONIZATION validity flag: `false` when the ionization degree `n_e / sum N_i` exceeds one
    /// half, where the neglected next ionization stage (hydrogen's own, helium's first) starts moving the electron
    /// budget and the single-stage reduction leaves its declared domain. A loud edge, not a silent degrade.
    pub single_ionization_valid: bool,
    /// The rider-2 ZERO-ELECTRON verdict: `true` when even the fully-ionized electron budget is below the
    /// representable floor (the cold outer disk), a LEGAL state (grains and molecular opacity own it there), so the
    /// bisection is short-circuited rather than chasing a `ln n_e ~ -500` root no bracket should follow.
    pub no_free_electrons: bool,
}

/// `ln(k_B)` in CGS (erg/K): the SI `k_B` (J/K) times `1e7`, in the log domain `ln k_B + 7 ln 10`. For `P_e` in
/// dyn/cm^2 from `n_e` in cm^-3 and `T` in K.
fn ln_k_boltzmann_cgs() -> Option<Fixed> {
    Some(ln_fundamental("k_B")? + Fixed::from_int(7).mul(Fixed::from_int(10).ln()))
}

/// The multi-species SAHA free-electron density from single-stage ionization and charge neutrality. Each `species`
/// is `(symbol, ln_number_density_cm3)`: its total (neutral plus ionized) number density in cm^-3, carried as a log
/// because it overflows `Fixed`. The ionization fraction of species `i` is `x_i = S_i / (n_e + S_i)`
/// ([`ln_saha_factor`] gives `ln S_i`), and charge neutrality `n_e = sum_i N_i x_i` is a monotone (decreasing)
/// equation in `n_e`, so its root is unique and found by a BOUNDED bisection in `ln n_e` using the log-domain sum
/// ([`log_sum_exp`], `ln(n_e + S_i) = logsumexp(ln n_e, ln S_i)`). Species without a pinned degeneracy or a first
/// ionization energy are excluded. Returns the [`SahaState`] with the cgs `P_e` export, the single-ionization
/// validity flag (rider 1), and the zero-electron verdict (rider 2). `None` if no species resolves or a constant
/// fails to load.
pub fn electron_density_saha(
    temperature_k: Fixed,
    species: &[(&str, Fixed)],
    table: &PeriodicTable,
) -> Option<SahaState> {
    if temperature_k <= Fixed::ZERO {
        return None;
    }
    // (ln N_i, ln S_i) for each species that carries both a pinned convention and a first ionization energy.
    let mut terms: Vec<(Fixed, Fixed)> = Vec::with_capacity(species.len());
    for (symbol, ln_n_i) in species {
        if let Some(ln_s) = ln_saha_factor(symbol, temperature_k, table) {
            terms.push((*ln_n_i, ln_s));
        }
    }
    if terms.is_empty() {
        return None;
    }
    // The total nuclei ln(sum N_i) (the full-ionization ceiling on n_e), by a canonical log-domain fold.
    let ln_n_total = terms
        .iter()
        .map(|(ln_n, _)| *ln_n)
        .reduce(log_sum_exp)
        .unwrap();
    // ln RHS(ln n_e) = logsumexp_i [ln N_i + ln S_i - logsumexp(ln n_e, ln S_i)], the charge-neutrality right side.
    let ln_rhs = |ln_ne: Fixed| -> Fixed {
        terms
            .iter()
            .map(|(ln_n, ln_s)| *ln_n + *ln_s - log_sum_exp(ln_ne, *ln_s))
            .reduce(log_sum_exp)
            .unwrap()
    };

    let ln_k_cgs = ln_k_boltzmann_cgs()?;
    let ln_t = temperature_k.ln();
    // ln P_e for a given ln n_e (P_e = n_e k_B T, cgs). The representable floor: P_e must exceed the smallest
    // positive Fixed, else the electrons are below resolution.
    let ln_p_e = |ln_ne: Fixed| -> Fixed { ln_ne + ln_k_cgs + ln_t };
    let ln_p_floor = Fixed::from_bits(1).ln(); // ln of the smallest positive Fixed (~2.3e-10 -> ln ~ -22.2).

    // The max achievable n_e "through each donor's own S": the weakly-ionized estimate n_e ~ sqrt(sum N_i S_i),
    // i.e. (1/2) logsumexp(ln N_i + ln S_i). Accurate in the cold regime (where the verdict matters), and merely
    // conservative (over-estimating) in the hot regime (where the bisection solves anyway).
    let ln_ne_estimate = terms
        .iter()
        .map(|(ln_n, ln_s)| *ln_n + *ln_s)
        .reduce(log_sum_exp)
        .unwrap()
        .div(Fixed::from_int(2));
    // The zero-electron verdict (rider 2): if even that estimate's electron pressure is below the representable
    // floor, there are no free electrons at resolution (the cold outer disk).
    if ln_p_e(ln_ne_estimate) < ln_p_floor {
        return Some(SahaState {
            ln_electron_density_cm3: Fixed::MIN,
            electron_pressure_dyn_cm2: Fixed::ZERO,
            single_ionization_valid: true,
            no_free_electrons: true,
        });
    }

    // Bounded bisection in ln n_e over [ln_n_total - efolds, ln_n_total]: charge neutrality is monotone, so
    // ln RHS(ln n_e) - ln n_e is decreasing and its unique zero is the root.
    let mut lo = ln_n_total - Fixed::from_int(SAHA_BRACKET_EFOLDS);
    let mut hi = ln_n_total;
    for _ in 0..SAHA_ITERS {
        let mid = (lo + hi).div(Fixed::from_int(2));
        if ln_rhs(mid) > mid {
            lo = mid; // n_e too small, the neutrality right side wants more electrons
        } else {
            hi = mid;
        }
    }
    let ln_ne = (lo + hi).div(Fixed::from_int(2));

    // Rider 1: the single-ionization validity edge, ionization degree n_e / N_total > 1/2.
    let ionization_degree = (ln_ne - ln_n_total).exp();
    let single_ionization_valid = ionization_degree <= Fixed::from_ratio(1, 2);

    let p_e = ln_p_e(ln_ne).exp();
    Some(SahaState {
        ln_electron_density_cm3: ln_ne,
        electron_pressure_dyn_cm2: p_e,
        single_ionization_valid,
        no_free_electrons: false,
    })
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
        // ln S(H, 6000 K) in CGS (per cm^3): the SI value 35.99 (quantum-concentration log ~62.29 minus the
        // Boltzmann term 13.6 * 11605 / 6000 ~26.30) minus 6 ln 10 = 13.816, so ~22.17. The g factor is 0
        // (2*1/2 = 1); a ground-state slip to g0 = 1 would shift this by ln 2 ~ 0.69, so the pin is load-bearing here.
        let t = table();
        let ln_s = ln_saha_factor("H", Fixed::from_int(6000), &t).unwrap();
        assert!(
            close(ln_s, 22.17, 0.3),
            "ln S(H, 6000K) ~ 22.17 cgs, got {}",
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

    #[test]
    fn the_solar_photosphere_ionizes_hydrogen_to_about_1e_minus_4() {
        // The century-old Saha result (the battery's external anchor): at T = 5800 K and photospheric number
        // densities (H ~1e17 cm^-3 with trace metal donors), hydrogen is only ~1e-4 ionized despite 13.6 eV
        // against kT ~ 0.5 eV, because the low-IE metals (Na, K, Mg, Ca) and H's own tail set n_e ~1.4e13 cm^-3.
        // This exercises the huge-exponent regime the log-space design exists for.
        let t = table();
        let temp = Fixed::from_int(5800);
        let species = [
            ("H", ln_of_decimal("1e17").unwrap()),
            ("Na", ln_of_decimal("2e11").unwrap()),
            ("K", ln_of_decimal("1e10").unwrap()),
            ("Mg", ln_of_decimal("3e12").unwrap()),
            ("Ca", ln_of_decimal("2e11").unwrap()),
        ];
        let state = electron_density_saha(temp, &species, &t).unwrap();
        assert!(
            !state.no_free_electrons,
            "the photosphere has free electrons"
        );
        assert!(
            state.single_ionization_valid,
            "the photosphere is well below full ionization"
        );
        // x_H = S_H / (S_H + n_e), in the log domain.
        let ln_s_h = ln_saha_factor("H", temp, &t).unwrap();
        let x_h = (ln_s_h - log_sum_exp(state.ln_electron_density_cm3, ln_s_h)).exp();
        assert!(
            x_h.to_f64_lossy() > 1e-5 && x_h.to_f64_lossy() < 1e-3,
            "hydrogen is ~1e-4 ionized at the photosphere, got {}",
            x_h.to_f64_lossy()
        );
        // The export is representable and positive (dyn/cm^2), and the solve replays.
        assert!(state.electron_pressure_dyn_cm2 > Fixed::ZERO);
        assert_eq!(state, electron_density_saha(temp, &species, &t).unwrap());
    }

    #[test]
    fn the_cold_outer_disk_returns_the_no_free_electrons_verdict() {
        // At 100 K the alkali Boltzmann exponent is ~500, so the true n_e is an ~e^-200 non-entity; the solve
        // short-circuits to the LEGAL no-free-electrons verdict (grains and molecules own the opacity there) rather
        // than chasing a -500-class root no bracket should follow.
        let t = table();
        let species = [
            ("H", ln_of_decimal("1e15").unwrap()),
            ("Na", ln_of_decimal("2e9").unwrap()),
            ("K", ln_of_decimal("1e8").unwrap()),
        ];
        let state = electron_density_saha(Fixed::from_int(100), &species, &t).unwrap();
        assert!(state.no_free_electrons, "no free electrons at 100 K");
        assert_eq!(state.electron_pressure_dyn_cm2, Fixed::ZERO);
    }

    #[test]
    fn a_hot_ionized_gas_trips_the_single_ionization_validity_flag() {
        // Push the temperature up until hydrogen is more than half ionized: the single-stage reduction leaves its
        // declared domain (hydrogen's own ionization and helium's first stage start moving the budget), and the
        // flag trips LOUDLY rather than degrading silently.
        let t = table();
        let species = [("H", ln_of_decimal("1e14").unwrap())];
        let hot = electron_density_saha(Fixed::from_int(15000), &species, &t).unwrap();
        assert!(
            !hot.single_ionization_valid,
            "a >50%-ionized gas trips the validity flag"
        );
        assert!(!hot.no_free_electrons);
    }
}
