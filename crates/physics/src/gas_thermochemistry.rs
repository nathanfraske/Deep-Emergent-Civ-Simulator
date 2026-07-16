//! The ideal-gas RRHO (rigid-rotor harmonic-oscillator) thermochemistry estimator: the standard molar entropy and
//! free-energy function of a gas-phase species from first principles, the input the disk-condensation minimization
//! (#57) needs for its gas phase. It is the phonon chain's uncredited payoff: the same banked columns that priced
//! the sky (atomic masses, the Pyykko bond lengths, the Badger force constants) ARE the gas thermochemistry, and
//! `R = N_A k_B` and the second radiation constant `c_2 = h c / k_B` derive from the register, nothing fetched.
//!
//! THE mu-STANDARD SOURCE LADDER (owner ruling, the ÆSOPUS architecture rerun verbatim): JANAF `[M]` is the mu°(T)
//! TOTAL top rung, not merely a reference-energy anchor, because the JANAF Gibbs-energy functions were themselves
//! computed from spectroscopic constants, so for any species with a row the whole temperature-dependent mu° is
//! measured. This RRHO estimator is then exactly what the es/ff/H- closure turned out to be for opacity: the
//! CERTIFIER of the table where derivation reaches, and the ALIEN RUNG where no JANAF row exists. Ladder:
//! JANAF `[M]` total -> RRHO estimator -> compute-once. The load-bearing polyatomics (H2O, CO2, SO2, the silicate
//! vapors) carry JANAF rows, so the full inertia-tensor-plus-Hessian substrate is the POLYATOMIC alien rung, a
//! named follow-on (VSEPR angles, the force-constant column for stretches, the phonon bend-ratio class constant for
//! bends, a valence-force-field assembly at factor grade, compute-once behind it), NOT on #57's critical path.
//!
//! This module builds the LINEAR-molecule (and monatomic, and diatomic) estimator, which is complete off the banked
//! columns. The standard entropy `S°(T)` is the validatable quantity (it exercises all four partition functions),
//! certified against the SPECTROSCOPIC values of CO, N2, and O2. PRE-REGISTERED (owner): CO carries the residual-
//! entropy trap: its calorimetric third-law `S°` runs about `R ln 2 ~ 5 J/mol/K` BELOW the spectroscopic value
//! because solid CO freezes in orientational disorder (the Pauling machinery this ledger banked for ice and Pluto's
//! CO frosts), so a correct estimator matches the spectroscopic 197.7, not the calorimetric 192.9; a ~5 J/mol/K gap
//! against old calorimetry is the freezer writing state into a 1930s experiment, not the estimator failing.
//!
//! ELECTRONIC degeneracy (owner ruling, split derive-first): a molecular ground term is `g_0` from Huber-Herzberg
//! (CO 1-Sigma+ -> 1, O2 3-Sigma- -> 3), the caller's datum here. An ATOMIC gas species with low-lying fine
//! structure (carbon 3-P, iron 5-D, spaced at few-hundred-kelvin) needs the ground MULTIPLET with Boltzmann weights,
//! not a bare `g_0`, at condensation temperatures; those level energies are a small NIST ASD `[M]` column, the atomic
//! electronic follow-on. This module takes `g_0` as a caller argument so both cases are expressible.

use crate::saha::{ln_fundamental, ln_of_decimal};
use civsim_core::Fixed;

/// The molar gas constant `R = N_A k_B` (J/mol/K, about 8.314), DERIVED from the register (not a fetched constant):
/// `ln R = ln N_A + ln k_B`, then exponentiated back into the representable range. `None` if a fundamental fails to
/// resolve.
pub fn molar_gas_constant() -> Option<Fixed> {
    Some(
        ln_fundamental("N_A")?
            .checked_add(ln_fundamental("k_B")?)?
            .exp(),
    )
}

/// The second radiation constant `c_2 = h c / k_B` (in K per cm^-1, about 1.4388), DERIVED from the register: it
/// converts a vibrational wavenumber to a vibrational temperature `theta = c_2 * omega`. The factor of 100 lifts the
/// speed of light from m/s to cm/s so `omega` in cm^-1 gives an energy `h c omega`. `None` on a register miss.
fn second_radiation_constant_k_per_cm() -> Option<Fixed> {
    Some(
        ln_fundamental("h")?
            .checked_add(ln_fundamental("c")?)?
            .checked_add(Fixed::from_int(100).ln())?
            .checked_sub(ln_fundamental("k_B")?)?
            .exp(),
    )
}

/// `ln` of the translational partition function per molecule at the standard pressure `P0 = 1 bar = 1e5 Pa` (the
/// Sackur-Tetrode momentum-space volume): `z_trans = (2 pi m k_B T / h^2)^(3/2) * (k_B T / P0)`, with `m` the
/// molecular mass. The intermediate `2 pi m k_B T / h^2 ~ 1e21 m^-2` overflows fixed-point, so `ln z_trans` is
/// ASSEMBLED from the `ln` of its factors (each register constant through [`ln_fundamental`], the order-one
/// quantities through `Fixed::ln`), never formed directly. `m = M_amu / (N_A * 1000)` kg. `None` on a non-positive
/// mass or temperature or a register miss.
pub fn ln_translational_partition(
    molecular_mass_amu: Fixed,
    temperature_k: Fixed,
) -> Option<Fixed> {
    if molecular_mass_amu <= Fixed::ZERO || temperature_k <= Fixed::ZERO {
        return None;
    }
    let ln_2pi = Fixed::PI.checked_mul(Fixed::from_int(2))?.ln();
    // ln m (kg) = ln M_amu - ln N_A - ln 1000.
    let ln_m = molecular_mass_amu
        .ln()
        .checked_sub(ln_fundamental("N_A")?)?
        .checked_sub(Fixed::from_int(1000).ln())?;
    let ln_kb = ln_fundamental("k_B")?;
    let ln_t = temperature_k.ln();
    let two_ln_h = ln_fundamental("h")?.checked_mul(Fixed::from_int(2))?;
    let ln_p0 = ln_of_decimal("1e5")?;
    // ln(2 pi m k_B T / h^2).
    let ln_arg = ln_2pi
        .checked_add(ln_m)?
        .checked_add(ln_kb)?
        .checked_add(ln_t)?
        .checked_sub(two_ln_h)?;
    // (3/2) ln_arg + ln(k_B T / P0).
    Fixed::from_ratio(3, 2)
        .checked_mul(ln_arg)?
        .checked_add(ln_kb)?
        .checked_add(ln_t)?
        .checked_sub(ln_p0)
}

/// `ln` of the rotational partition function of a LINEAR molecule (rigid rotor): `z_rot = 8 pi^2 I k_B T /
/// (sigma h^2)`, with the moment of inertia `I = mu r^2` (`mu` the reduced mass, `r` the bond length) and `sigma`
/// the symmetry number (1 heteronuclear, 2 homonuclear). `I ~ 1e-46 kg m^2` is tiny, so `ln I` is assembled from
/// the `ln` of its factors: `ln I = ln mu_amu - ln N_A - ln 1000 + 2 (ln r_pm + ln 1e-12)`. `None` on a
/// non-positive input or a register miss.
pub fn ln_rotational_partition_linear(
    reduced_mass_amu: Fixed,
    bond_length_pm: Fixed,
    symmetry_number: u32,
    temperature_k: Fixed,
) -> Option<Fixed> {
    if reduced_mass_amu <= Fixed::ZERO
        || bond_length_pm <= Fixed::ZERO
        || symmetry_number == 0
        || temperature_k <= Fixed::ZERO
    {
        return None;
    }
    // ln I = ln mu - ln N_A - ln 1000 + 2 (ln r_pm + ln 1e-12), I in kg m^2.
    let ln_i = reduced_mass_amu
        .ln()
        .checked_sub(ln_fundamental("N_A")?)?
        .checked_sub(Fixed::from_int(1000).ln())?
        .checked_add(
            Fixed::from_int(2)
                .checked_mul(bond_length_pm.ln().checked_add(ln_of_decimal("1e-12")?)?)?,
        )?;
    let ln_8pi2 = Fixed::from_int(8)
        .checked_mul(Fixed::PI)?
        .checked_mul(Fixed::PI)?
        .ln();
    ln_8pi2
        .checked_add(ln_i)?
        .checked_add(ln_fundamental("k_B")?)?
        .checked_add(temperature_k.ln())?
        .checked_sub(Fixed::from_int(symmetry_number as i32).ln())?
        .checked_sub(ln_fundamental("h")?.checked_mul(Fixed::from_int(2))?)
}

/// The vibrational entropy of one harmonic mode in units of `R`: `S_vib/R = x/(e^x - 1) - ln(1 - e^-x)`, with the
/// reduced temperature `x = theta/T = c_2 omega / T` (`omega` the mode wavenumber in cm^-1). A stiff, cold mode
/// (`x` large) is frozen and contributes about zero; the `x > 30` guard returns zero before `e^x` overflows
/// fixed-point (`e^30 ~ 1e13` is past the representable ceiling for the subtraction). `None` on a non-positive
/// input or a register miss.
pub fn vibrational_entropy_over_r(omega_cm: Fixed, temperature_k: Fixed) -> Option<Fixed> {
    if omega_cm <= Fixed::ZERO || temperature_k <= Fixed::ZERO {
        return None;
    }
    let x = second_radiation_constant_k_per_cm()?
        .checked_mul(omega_cm)?
        .checked_div(temperature_k)?;
    if x > Fixed::from_int(30) {
        return Some(Fixed::ZERO); // frozen mode
    }
    let ex = x.exp();
    let term1 = x.checked_div(ex.checked_sub(Fixed::ONE)?)?;
    let emx = Fixed::ZERO.checked_sub(x)?.exp(); // e^-x
    let term2 = Fixed::ONE.checked_sub(emx)?.ln(); // ln(1 - e^-x) <= 0
    term1.checked_sub(term2)
}

/// The standard molar entropy `S°(T)` (J/mol/K) of a LINEAR ideal-gas molecule, the RRHO sum
/// `S° = S_trans + S_rot + S_vib + S_elec`, with `S_trans/R = ln z_trans + 5/2`, `S_rot/R = ln z_rot + 1`, one
/// harmonic mode `S_vib/R`, and `S_elec/R = ln g_0`. The dimensionless sum is formed first (every term already
/// dimensionless), then multiplied by the derived `R`. The inputs are the banked columns: the molecular mass and
/// reduced mass from the atomic weights, the bond length from the Pyykko radii, the mode wavenumber from the Badger
/// force constant (or a measured spectroscopic value for the certification gate), the symmetry number, and the
/// electronic ground-term degeneracy `g_0`. A monatomic species passes `omega = 0`-flagged by omitting the mode:
/// use [`monatomic_standard_entropy`] for that. `None` on a bad input or a register miss.
#[allow(clippy::too_many_arguments)]
pub fn linear_molecule_standard_entropy(
    molecular_mass_amu: Fixed,
    reduced_mass_amu: Fixed,
    bond_length_pm: Fixed,
    omega_cm: Fixed,
    symmetry_number: u32,
    electronic_degeneracy: u32,
    temperature_k: Fixed,
) -> Option<Fixed> {
    if electronic_degeneracy == 0 {
        return None;
    }
    let s_over_r = ln_translational_partition(molecular_mass_amu, temperature_k)?
        .checked_add(Fixed::from_ratio(5, 2))?
        .checked_add(ln_rotational_partition_linear(
            reduced_mass_amu,
            bond_length_pm,
            symmetry_number,
            temperature_k,
        )?)?
        .checked_add(Fixed::ONE)?
        .checked_add(vibrational_entropy_over_r(omega_cm, temperature_k)?)?
        .checked_add(Fixed::from_int(electronic_degeneracy as i32).ln())?;
    molar_gas_constant()?.checked_mul(s_over_r)
}

/// The standard molar entropy `S°(T)` (J/mol/K) of a MONATOMIC ideal gas: the Sackur-Tetrode translational term plus
/// the electronic degeneracy, no rotation or vibration. `S° = R (ln z_trans + 5/2 + ln g_0)`. For an atom with
/// low-lying fine structure the caller must pass the multiplet's Boltzmann-weighted effective `g` rather than the
/// bare ground `g_0` (the atomic electronic follow-on). `None` on a bad input or a register miss.
pub fn monatomic_standard_entropy(
    atomic_mass_amu: Fixed,
    electronic_degeneracy: Fixed,
    temperature_k: Fixed,
) -> Option<Fixed> {
    if electronic_degeneracy <= Fixed::ZERO {
        return None;
    }
    let s_over_r = ln_translational_partition(atomic_mass_amu, temperature_k)?
        .checked_add(Fixed::from_ratio(5, 2))?
        .checked_add(electronic_degeneracy.ln())?;
    molar_gas_constant()?.checked_mul(s_over_r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amu(v: f64) -> Fixed {
        Fixed::from_ratio((v * 1000.0).round() as i64, 1000)
    }

    #[test]
    fn the_gas_constant_and_radiation_constant_derive_from_the_register() {
        let r = molar_gas_constant().unwrap();
        assert!(
            (r.to_f64_lossy() - 8.314).abs() < 0.02,
            "R = N_A k_B ~ 8.314 J/mol/K, got {}",
            r.to_f64_lossy()
        );
        let c2 = second_radiation_constant_k_per_cm().unwrap();
        assert!(
            (c2.to_f64_lossy() - 1.4388).abs() < 0.01,
            "c_2 = h c / k_B ~ 1.4388 K/cm^-1, got {}",
            c2.to_f64_lossy()
        );
    }

    #[test]
    fn the_co_standard_entropy_matches_the_spectroscopic_value() {
        // CO at 298.15 K from measured spectroscopic constants (Huber-Herzberg): m = 28.01, mu = 6.858,
        // r_e = 112.8 pm, omega_e = 2169.8 cm^-1, sigma = 1 (heteronuclear), g_0 = 1 (1-Sigma+). The RRHO S°
        // reproduces the SPECTROSCOPIC 197.66 J/mol/K. PRE-REGISTERED (owner): this is NOT the calorimetric third-
        // law 192.9 (which sits ~R ln 2 lower from solid-CO orientational disorder, the freezer's residual entropy);
        // a match here to ~197.7 is the estimator working, a match to ~192.9 would be wrong.
        let s = linear_molecule_standard_entropy(
            amu(28.01),
            amu(6.858),
            amu(112.8),
            amu(2169.8),
            1,
            1,
            amu(298.15),
        )
        .unwrap();
        assert!(
            (s.to_f64_lossy() - 197.66).abs() < 2.0,
            "CO spectroscopic S°(298) ~ 197.66 J/mol/K, got {} (must NOT be the ~192.9 calorimetric value)",
            s.to_f64_lossy()
        );
    }

    #[test]
    fn the_n2_standard_entropy_matches_with_the_homonuclear_symmetry_number() {
        // N2 at 298.15 K: m = 28.01, mu = 7.001, r_e = 109.77 pm, omega_e = 2358.6 cm^-1, sigma = 2 (homonuclear,
        // the symmetry number that CO lacks), g_0 = 1. Literature S°(298) = 191.6 J/mol/K.
        let s = linear_molecule_standard_entropy(
            amu(28.014),
            amu(7.001),
            amu(109.77),
            amu(2358.6),
            2,
            1,
            amu(298.15),
        )
        .unwrap();
        assert!(
            (s.to_f64_lossy() - 191.6).abs() < 2.0,
            "N2 S°(298) ~ 191.6 J/mol/K, got {}",
            s.to_f64_lossy()
        );
    }

    #[test]
    fn the_o2_standard_entropy_uses_the_triplet_ground_degeneracy() {
        // O2 at 298.15 K: m = 32.00, mu = 8.00, r_e = 120.75 pm, omega_e = 1580.2 cm^-1, sigma = 2, g_0 = 3 (the
        // 3-Sigma- triplet ground term, the electronic degeneracy CO and N2 lack). Literature S°(298) = 205.2
        // J/mol/K; the R ln 3 ~ 9.1 J/mol/K from the triplet is load-bearing (g_0 = 1 would give ~196, wrong).
        let s = linear_molecule_standard_entropy(
            amu(31.998),
            amu(7.9997),
            amu(120.75),
            amu(1580.2),
            2,
            3,
            amu(298.15),
        )
        .unwrap();
        assert!(
            (s.to_f64_lossy() - 205.2).abs() < 2.0,
            "O2 S°(298) ~ 205.2 J/mol/K (triplet g_0 = 3), got {}",
            s.to_f64_lossy()
        );
    }

    #[test]
    fn a_frozen_stiff_mode_contributes_no_vibrational_entropy() {
        // CO's 2170 cm^-1 mode at 298 K has x = theta/T ~ 10.5, so it is nearly frozen: S_vib/R is small and
        // positive, well under 0.05. At high T (2000 K, x ~ 1.56) it is thawed and contributes appreciably.
        let cold = vibrational_entropy_over_r(amu(2169.8), amu(298.15)).unwrap();
        assert!(
            cold.to_f64_lossy() < 0.05,
            "cold stiff mode frozen, got {}",
            cold.to_f64_lossy()
        );
        let hot = vibrational_entropy_over_r(amu(2169.8), amu(2000.0)).unwrap();
        assert!(
            hot.to_f64_lossy() > cold.to_f64_lossy(),
            "the mode thaws with temperature"
        );
    }
}
