//! The phonon-mode generator, the estimator that turns a grain's lattice into its infrared optical constants
//! `n(lambda), k(lambda)` for the phases the measured library does not cover (Rule 1's admit-the-alien fallback).
//! It feeds the existing Lorentzian lineshape ([`crate::materials`]'s `lorentzian_response`, up-stack) with the
//! four vibrational parameters this module derives:
//!
//! - `omega_TO`, the transverse-optical mode wavenumber, from the force constant and the reduced mass:
//!   `omega_TO = (1/2 pi c) sqrt(k/mu)` (this module, [`omega_to_cm_inverse`]).
//! - `omega_LO`, by the Lyddane-Sachs-Teller relation `omega_LO = omega_TO sqrt(eps_0/eps_inf)`
//!   ([`omega_lo_cm_inverse`]), derived in form.
//! - the oscillator strength `S = eps_0 - eps_inf` (Szigeti, the later stage), with `eps_inf` from Clausius-Mossotti
//!   on the electronic polarizability column.
//! - the damping `gamma`, a declared residue (the later stage).
//!
//! UNIT PIN (the standing law): the generator works in wavenumbers (cm^-1), the lineshape in eV; the conversion
//! `1 eV = e/(h c) ~ 8065.5 cm^-1` is DERIVED from the register ([`wavenumber_per_ev`]) and round-tripped, never a
//! bare literal. This file builds the frequency core (omega_TO, the unit pin, LST); the dielectric assembly and the
//! battery rows are the following stage.

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use civsim_units::compute;
use civsim_units::fundamentals;

const PHONON_PI_DIGITS: u32 = 40;

/// One register fundamental as an exact `BigRat`.
fn fundamental_bigrat(symbol: &str) -> Option<BigRat> {
    BigRat::from_decimal_str(fundamentals::fundamental(symbol)?.value).ok()
}

/// The squared frequency constant `C^2 = 10^5 N_A / (2 pi c_cgs)^2`, so `omega_TO[cm^-1] = sqrt((k[mdyn/A]/mu[amu])
/// C^2)`. The `10^5` folds mdyn/A -> N/m (`x100`) and amu -> kg (`/ (1/(1000 N_A))`); `c_cgs = 100 c`. Derived from
/// `N_A` and `c`, lands `~1.696e6`. `None` if a fundamental fails to resolve or the value leaves the representable
/// range.
fn omega_to_squared_constant() -> Option<Fixed> {
    let n_a = fundamental_bigrat("N_A")?;
    let c_cgs = fundamental_bigrat("c")?.mul(&BigRat::from_i64(100)); // m/s -> cm/s
    let pi = compute::pi(PHONON_PI_DIGITS);
    let two_pi_c = BigRat::from_i64(2).mul(&pi).mul(&c_cgs);
    let c2 = BigRat::from_i64(100_000)
        .mul(&n_a)
        .div(&two_pi_c.mul(&two_pi_c));
    Fixed::from_bits_i128(c2.round_to_scale(Fixed::FRAC_BITS)?)
}

/// The transverse-optical mode wavenumber `omega_TO` (cm^-1) of a bond, from its stretching force constant `k`
/// (mdyn/Angstrom, the Badger column) and the reduced mass `mu` (amu): `omega_TO = (1/2 pi c) sqrt(k/mu)`. The SiO
/// bond (`k = 9.24`, `mu = 10.18`) lands `~1242 cm^-1`, the measured stretch. `None` on a non-positive reduced mass
/// or an overflow.
pub fn omega_to_cm_inverse(k_mdyn_per_angstrom: Fixed, reduced_mass_amu: Fixed) -> Option<Fixed> {
    if reduced_mass_amu <= Fixed::ZERO || k_mdyn_per_angstrom < Fixed::ZERO {
        return None;
    }
    let c2 = omega_to_squared_constant()?;
    Some(
        k_mdyn_per_angstrom
            .checked_div(reduced_mass_amu)?
            .checked_mul(c2)?
            .sqrt(),
    )
}

/// The wavenumber-per-eV conversion `1 eV = e/(h c) ~ 8065.5 cm^-1`, DERIVED from the register (`e`, `h`, `c`), the
/// unit pin between the generator (cm^-1) and the Lorentzian lineshape (eV). `None` if a fundamental fails to
/// resolve.
pub fn wavenumber_per_ev() -> Option<Fixed> {
    let e = fundamental_bigrat("e")?;
    let h = fundamental_bigrat("h")?;
    let c_cgs = fundamental_bigrat("c")?.mul(&BigRat::from_i64(100));
    let v = e.div(&h.mul(&c_cgs));
    Fixed::from_bits_i128(v.round_to_scale(Fixed::FRAC_BITS)?)
}

/// A mode energy in eV from its wavenumber (cm^-1), through the derived unit pin.
pub fn ev_from_wavenumber(wavenumber_cm: Fixed) -> Option<Fixed> {
    wavenumber_cm.checked_div(wavenumber_per_ev()?)
}

/// A mode wavenumber (cm^-1) from its energy in eV, the inverse of [`ev_from_wavenumber`].
pub fn wavenumber_from_ev(energy_ev: Fixed) -> Option<Fixed> {
    energy_ev.checked_mul(wavenumber_per_ev()?)
}

/// The longitudinal-optical mode wavenumber `omega_LO` (cm^-1) by the Lyddane-Sachs-Teller relation
/// `omega_LO = omega_TO sqrt(eps_0/eps_inf)`, the LO-TO splitting the static and high-frequency permittivities set.
/// `None` on a non-positive `eps_inf` or an overflow.
pub fn omega_lo_cm_inverse(
    omega_to_cm: Fixed,
    static_permittivity: Fixed,
    high_frequency_permittivity: Fixed,
) -> Option<Fixed> {
    if high_frequency_permittivity <= Fixed::ZERO {
        return None;
    }
    let ratio = static_permittivity.checked_div(high_frequency_permittivity)?;
    omega_to_cm.checked_mul(ratio.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn the_unit_pin_derives_and_round_trips() {
        // 1 eV = e/(h c) ~ 8065.5 cm^-1 from the register; a mode at 1000 cm^-1 is 0.124 eV, and the conversion
        // round-trips.
        let per_ev = wavenumber_per_ev().unwrap();
        assert!(
            close(per_ev.to_f64_lossy(), 8065.54, 1.0),
            "1 eV ~ 8065.5 cm^-1, got {}",
            per_ev.to_f64_lossy()
        );
        let back = wavenumber_from_ev(ev_from_wavenumber(Fixed::from_int(1000)).unwrap()).unwrap();
        assert!(
            close(back.to_f64_lossy(), 1000.0, 0.5),
            "the wavenumber<->eV conversion round-trips, got {}",
            back.to_f64_lossy()
        );
    }

    #[test]
    fn the_omega_to_lands_the_sio_stretch() {
        // omega_TO = (1/2 pi c) sqrt(k/mu): SiO with k = 9.24 mdyn/A and mu = 10.18 amu is the measured ~1242 cm^-1
        // stretch, the check that the force constant and the reduced mass assemble to the right mode frequency.
        let w =
            omega_to_cm_inverse(Fixed::from_ratio(924, 100), Fixed::from_ratio(1018, 100)).unwrap();
        assert!(
            close(w.to_f64_lossy(), 1242.0, 15.0),
            "the SiO omega_TO is ~1242 cm^-1, got {}",
            w.to_f64_lossy()
        );
        // A heavier reduced mass or a softer bond lowers the frequency.
        let softer =
            omega_to_cm_inverse(Fixed::from_ratio(300, 100), Fixed::from_ratio(2000, 100)).unwrap();
        assert!(softer.to_f64_lossy() < w.to_f64_lossy());
    }

    #[test]
    fn the_lyddane_sachs_teller_gate_lands_nacl() {
        // The NaCl LST identity (the pre-registered gate): omega_TO ~ 164 cm^-1, eps_0 ~ 5.9, eps_inf = n^2 ~ 2.34
        // predict omega_LO ~ 260 cm^-1 against the measured ~264, ~2%.
        let omega_lo = omega_lo_cm_inverse(
            Fixed::from_int(164),
            Fixed::from_ratio(59, 10),
            Fixed::from_ratio(234, 100),
        )
        .unwrap();
        assert!(
            close(omega_lo.to_f64_lossy(), 262.0, 6.0),
            "NaCl omega_LO ~ 260-264 cm^-1, got {}",
            omega_lo.to_f64_lossy()
        );
    }
}
