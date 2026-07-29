//! The electronic polarizability column, DERIVED from the ionization energy by the single-oscillator (Unsold)
//! estimate, the first of the two input columns the phonon-mode generator needs (it feeds `eps_inf` through
//! Clausius-Mossotti, which sets the LO-TO splitting and the oscillator strength of a grain's infrared bands).
//!
//! An atom polarizes because its electrons displace against the nucleus under a field; modelled as a single
//! Lorentz oscillator whose resonance sits at the ionization energy (the natural electronic energy scale), the
//! static polarizability is `alpha = N e^2/(m_e omega_0^2)` with `hbar omega_0 = IE` and `N` the count of
//! polarizable (valence) electrons. In Bohr-volume units this reduces to `alpha = N (E_h/IE)^2` (a_0^3), with the
//! Hartree energy `E_h` the only combination of fundamentals it needs. The estimate is deliberately simple and
//! runs ~11% low on hydrogen (4.0 a_0^3 against the exact 4.5), the known bound the unit test pins, so the phonon
//! generator treats it as a factor-grade input, never a fitted constant.
//!
//! The input is DATA: the cited ionization energy and the main-group valence count read from the periodic table,
//! so an alien element is its own row (admit-the-alien), never a rewrite.

use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use civsim_units::constants::SiExecutionMagnitudes;

/// One sealed SI execution value as an exact `BigRat`.
fn fundamental_bigrat(execution: &SiExecutionMagnitudes, symbol: &str) -> Option<BigRat> {
    Some(execution.get(symbol)?.exact_rational())
}

/// The Hartree energy in eV, `E_h = m_e e^3/(4 eps_0^2 h^2)`, DERIVED from the register. The Gaussian form
/// `m_e e^4/((4 pi eps_0)^2 hbar^2)` divided by `e` (J to eV) folds the `pi` and the `hbar = h/2pi` away exactly,
/// leaving this compact combination of `m_e, e, eps_0, h`. Lands `~27.211 eV`, never fetched. `None` if a
/// fundamental fails to resolve or the result leaves the representable range.
pub fn hartree_energy_ev(execution: &SiExecutionMagnitudes) -> Option<Fixed> {
    let m_e = fundamental_bigrat(execution, "m_e")?;
    let e = fundamental_bigrat(execution, "e")?;
    let eps0 = fundamental_bigrat(execution, "eps_0")?;
    let h = fundamental_bigrat(execution, "h")?;
    let e3 = e.mul(&e).mul(&e);
    let numer = m_e.mul(&e3);
    let denom = BigRat::from_i64(4).mul(&eps0).mul(&eps0).mul(&h).mul(&h);
    let e_h = numer.div(&denom);
    Fixed::from_bits_i128(e_h.round_to_scale(Fixed::FRAC_BITS)?)
}

/// The static electronic polarizability `alpha` in Bohr-volume units (`a_0^3`) of an atom with
/// `response_electron_count` polarizable electrons and first ionization energy `ionization_energy_ev`, by the
/// single-oscillator estimate `alpha = N (E_h/IE)^2`. The result is a polarizability VOLUME (the Gaussian
/// convention), directly the quantity Clausius-Mossotti sums to `eps_inf`. `None` on a non-positive ionization
/// energy, a zero electron count, or an overflow.
pub fn electronic_polarizability_a0_cubed(
    execution: &SiExecutionMagnitudes,
    ionization_energy_ev: Fixed,
    response_electron_count: u32,
) -> Option<Fixed> {
    if ionization_energy_ev <= Fixed::ZERO || response_electron_count == 0 {
        return None;
    }
    let ratio = hartree_energy_ev(execution)?.checked_div(ionization_energy_ev)?;
    ratio
        .checked_mul(ratio)?
        .checked_mul(Fixed::from_int(response_electron_count as i32))
}

/// The per-element static electronic polarizability (`a_0^3`) read from the periodic table: its cited ionization
/// energy and the main-group valence-electron count as the response-electron number `N`. `None` if the element,
/// its ionization energy, or its valence count is unavailable (a transition metal without a main-group count
/// escalates to the caller rather than guessing).
pub fn element_electronic_polarizability_a0_cubed(
    execution: &SiExecutionMagnitudes,
    symbol: &str,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let ie = table.element(symbol)?.ionization_energy?;
    let n = table.main_group_valence(symbol)?;
    electronic_polarizability_a0_cubed(execution, ie, n as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the periodic table loads")
    }

    fn execution() -> SiExecutionMagnitudes {
        civsim_units::constants::canonical_si_execution_magnitudes()
            .expect("the sealed floor projects")
    }

    #[test]
    fn the_hartree_energy_derives_from_the_register() {
        // E_h = m_e e^3/(4 eps_0^2 h^2) lands the 27.211 eV atomic energy scale from m_e, e, eps_0, h alone.
        let e_h = hartree_energy_ev(&execution()).unwrap().to_f64_lossy();
        assert!(
            (e_h - 27.211).abs() < 0.01,
            "the Hartree energy is ~27.211 eV, got {e_h}"
        );
    }

    #[test]
    fn the_hydrogen_polarizability_lands_the_known_eleven_percent_low_bound() {
        // The pinned unit test: the single-oscillator estimate gives hydrogen alpha = (E_h/IE)^2 = (27.211/13.598)^2
        // = 4.00 a_0^3, against the EXACT 4.5 a_0^3, ~11% low. This is the known accuracy bound of the estimate,
        // the honest factor-grade the phonon generator inherits, never tuned away.
        let execution = execution();
        let alpha =
            electronic_polarizability_a0_cubed(&execution, Fixed::from_ratio(13598, 1000), 1)
                .unwrap();
        assert!(
            (alpha.to_f64_lossy() - 4.00).abs() < 0.02,
            "hydrogen polarizability estimate is ~4.0 a_0^3, got {}",
            alpha.to_f64_lossy()
        );
        let low_fraction = alpha.to_f64_lossy() / 4.5;
        assert!(
            low_fraction > 0.87 && low_fraction < 0.91,
            "the estimate sits ~11% below the exact 4.5 a_0^3, ratio {low_fraction}"
        );
    }

    #[test]
    fn a_lower_ionization_energy_is_more_polarizable() {
        // Polarizability scales as (E_h/IE)^2, so a loosely-bound electron (low IE, like an alkali) polarizes far
        // more than a tightly-bound one (high IE, like a noble gas) at the same electron count.
        let execution = execution();
        let soft =
            electronic_polarizability_a0_cubed(&execution, Fixed::from_ratio(5, 1), 1).unwrap();
        let stiff = electronic_polarizability_a0_cubed(&execution, Fixed::from_int(20), 1).unwrap();
        assert!(
            soft.to_f64_lossy() > stiff.to_f64_lossy(),
            "a lower ionization energy is more polarizable: {} vs {}",
            soft.to_f64_lossy(),
            stiff.to_f64_lossy()
        );
    }

    #[test]
    fn the_element_column_reads_the_periodic_table() {
        // The per-element read: sodium (IE 5.14 eV, one main-group valence electron) is highly polarizable; the
        // column derives from the cited data, admit-the-alien.
        let tbl = table();
        let execution = execution();
        let na = element_electronic_polarizability_a0_cubed(&execution, "Na", &tbl).unwrap();
        assert!(
            na.to_f64_lossy() > 20.0,
            "sodium's low ionization energy makes it very polarizable, got {} a_0^3",
            na.to_f64_lossy()
        );
        // Deterministic replay.
        assert_eq!(
            na,
            element_electronic_polarizability_a0_cubed(&execution, "Na", &tbl).unwrap()
        );
    }

    #[test]
    fn it_rejects_non_physical_arguments() {
        let execution = execution();
        assert_eq!(
            electronic_polarizability_a0_cubed(&execution, Fixed::ZERO, 1),
            None,
            "a non-positive ionization energy is rejected"
        );
        assert_eq!(
            electronic_polarizability_a0_cubed(&execution, Fixed::from_int(10), 0),
            None,
            "a zero electron count is rejected"
        );
    }
}
