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

/// The Clausius-Mossotti denominator floor: `1 - y` must stay above this or the polarizability catastrophe is
/// near. Reserved calibration (basis: the packing at which the estimator-grade `alpha` can no longer be trusted to
/// keep `eps_inf` finite; a `~0.1` guard band below the `y = 1` divergence).
fn clausius_mossotti_floor() -> Fixed {
    Fixed::from_ratio(1, 10)
}

/// Whether a Clausius-Mossotti packing `y = (4 pi/3) N alpha` is in the polarizability CATASTROPHE (its denominator
/// `1 - y` at or below the guard floor). This is PHYSICS, not a numerics annoyance: `y -> 1` is the polarizability
/// catastrophe, the ferroelectric lattice instability in Clausius-Mossotti clothing, so a trip here is a Gap Law
/// datum (the phase wants to distort), flagged for the caller to escalate rather than emit a garbage `eps_inf`.
pub fn is_polarizability_catastrophe(polarizability_packing: Fixed) -> bool {
    Fixed::ONE
        .checked_sub(polarizability_packing)
        .map(|d| d <= clausius_mossotti_floor())
        .unwrap_or(true)
}

/// The high-frequency permittivity `eps_inf = (1 + 2y)/(1 - y)` from the Clausius-Mossotti packing `y = (4 pi/3) N
/// alpha` (the electronic polarizability column summed over the formula unit, times the number density). The
/// catastrophe guard is a PREFLIGHT: `None` if `1 - y` is at or below the floor (the polarizability catastrophe, a
/// Gap Law datum), never a silently divergent `eps_inf` that would corrupt the oscillator strength and `n,k`
/// downstream. `None` also on an overflow.
pub fn high_frequency_permittivity(polarizability_packing: Fixed) -> Option<Fixed> {
    let one_minus_y = Fixed::ONE.checked_sub(polarizability_packing)?;
    if one_minus_y <= clausius_mossotti_floor() {
        return None; // the polarizability catastrophe (escalate), not a finite eps_inf
    }
    Fixed::ONE
        .checked_add(Fixed::from_int(2).checked_mul(polarizability_packing)?)?
        .checked_div(one_minus_y)
}

/// The infrared oscillator strength `S = eps_0 - eps_inf` from the Szigeti model, `S = S_bare * ((eps_inf + 2)/3)^2`
/// with `S_bare = (Z_s e)^2 N / (eps_0 V mu omega_TO^2)` the bare mode strength (the caller assembles it from the
/// Szigeti effective charge, the ion-pair number density, the reduced mass, and the TO frequency). The
/// `((eps_inf + 2)/3)^2` LOCAL-FIELD factor is the Lorentz correction the classic derivation drops: at a typical
/// `eps_inf ~ 2` to `3` it is a factor `~2`, so omitting it would masquerade as a Szigeti-charge error one tier
/// down. `None` on an overflow.
pub fn oscillator_strength(
    bare_strength: Fixed,
    high_frequency_permittivity: Fixed,
) -> Option<Fixed> {
    let local_field = high_frequency_permittivity
        .checked_add(Fixed::from_int(2))?
        .checked_div(Fixed::from_int(3))?;
    bare_strength.checked_mul(local_field.checked_mul(local_field)?)
}

/// The crystalline damping `gamma = 0.05 omega_TO` (cm^-1), a declared residue [C] (memory-grade `~0.05` of the
/// mode frequency for an ordered lattice). An amorphous phase broadens it: passing a larger `fractional_width`
/// smears the band, which is how a glassy grain's fictive temperature reaches the sky (written state in the
/// spectrum). The `0.05` is reserved calibration (basis: the crystalline linewidth class; broadened per phase).
pub fn damping_cm_inverse(omega_to_cm: Fixed, fractional_width: Fixed) -> Option<Fixed> {
    omega_to_cm.checked_mul(fractional_width)
}

/// The default crystalline damping fraction (`gamma/omega_TO ~ 0.05`), the ordered-lattice class; amorphous phases
/// pass a larger value.
pub fn crystalline_damping_fraction() -> Fixed {
    Fixed::from_ratio(5, 100)
}

/// The bend-to-stretch force-constant ratio class constant `k_bend/k_stretch ~ 0.1 to 0.15` [E, flagged,
/// memory-grade band]. Badger's rule is a STRETCH rule and cannot supply a bend constant; the O-Si-O bend mode
/// (the silicate 18 micron feature) reads its force constant as this fraction of the stretch, booked separately so
/// a bend miss convicts the bend estimator, never the validated stretch chain.
pub fn bend_to_stretch_ratio() -> Fixed {
    Fixed::from_ratio(125, 1000)
}

/// One Lorentz oscillator's contribution folded onto `eps_inf`: `eps(omega) = eps_inf + S omega_TO^2 / (omega_TO^2
/// - omega^2 - i gamma omega)`, returned as `(Re eps, Im eps)`. Between `omega_TO` and `omega_LO` the real part
/// goes negative, which IS the Reststrahlen band. `None` on an overflow or a zero denominator.
pub fn lorentz_permittivity(
    high_frequency_permittivity: Fixed,
    oscillator_strength: Fixed,
    omega_to_cm: Fixed,
    gamma_cm: Fixed,
    omega_cm: Fixed,
) -> Option<(Fixed, Fixed)> {
    let wto2 = omega_to_cm.checked_mul(omega_to_cm)?;
    let d = wto2.checked_sub(omega_cm.checked_mul(omega_cm)?)?;
    let gw = gamma_cm.checked_mul(omega_cm)?;
    let denom = d.checked_mul(d)?.checked_add(gw.checked_mul(gw)?)?;
    if denom <= Fixed::ZERO {
        return None;
    }
    let s_wto2 = oscillator_strength.checked_mul(wto2)?;
    let re = high_frequency_permittivity.checked_add(s_wto2.checked_mul(d)?.checked_div(denom)?)?;
    let im = s_wto2.checked_mul(gw)?.checked_div(denom)?;
    Some((re, im))
}

/// The complex refractive index `(n, k)` from a permittivity `(Re eps, Im eps)`: `n = sqrt((|eps| + Re eps)/2)`,
/// `k = sqrt((|eps| - Re eps)/2)`. The BRANCH is pinned so `k >= 0` (Im(n) >= 0, absorption), which is the branch
/// that survives the Reststrahlen band where `Re eps < 0` (there `n -> 0`, `k` large, reflectivity -> 1). `None`
/// on an overflow.
pub fn refractive_index_from_permittivity(eps_re: Fixed, eps_im: Fixed) -> Option<(Fixed, Fixed)> {
    let modulus = eps_re
        .checked_mul(eps_re)?
        .checked_add(eps_im.checked_mul(eps_im)?)?
        .sqrt();
    let two = Fixed::from_int(2);
    let n = modulus.checked_add(eps_re)?.checked_div(two)?.sqrt();
    let k_arg = modulus.checked_sub(eps_re)?;
    let k_arg = if k_arg < Fixed::ZERO {
        Fixed::ZERO
    } else {
        k_arg
    };
    let k = k_arg.checked_div(two)?.sqrt();
    Some((n, k))
}

/// The normal-incidence reflectivity `R = ((n-1)^2 + k^2)/((n+1)^2 + k^2)` of a surface with index `(n, k)`, the
/// physical signature the branch choice must survive: in a Reststrahlen band `R -> 1`. `None` on an overflow.
pub fn reflectivity(n: Fixed, k: Fixed) -> Option<Fixed> {
    let k2 = k.checked_mul(k)?;
    let num = n
        .checked_sub(Fixed::ONE)?
        .checked_mul(n.checked_sub(Fixed::ONE)?)?
        .checked_add(k2)?;
    let den = n
        .checked_add(Fixed::ONE)?
        .checked_mul(n.checked_add(Fixed::ONE)?)?
        .checked_add(k2)?;
    num.checked_div(den)
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
    fn the_clausius_mossotti_permittivity_and_catastrophe_guard() {
        // eps_inf = (1 + 2y)/(1 - y): a packing y = 0.3 gives eps_inf = 1.6/0.7 = 2.286.
        let eps = high_frequency_permittivity(Fixed::from_ratio(3, 10)).unwrap();
        assert!(
            close(eps.to_f64_lossy(), 2.286, 0.01),
            "eps_inf(y=0.3) ~ 2.286, got {}",
            eps.to_f64_lossy()
        );
        // The catastrophe guard: y = 0.95 (1 - y = 0.05 <= the 0.1 floor) is the polarizability catastrophe -> the
        // preflight returns None (escalate, a Gap Law datum) rather than a huge finite eps_inf.
        assert!(is_polarizability_catastrophe(Fixed::from_ratio(95, 100)));
        assert_eq!(
            high_frequency_permittivity(Fixed::from_ratio(95, 100)),
            None,
            "the polarizability catastrophe escalates, never a garbage eps_inf"
        );
    }

    #[test]
    fn the_szigeti_oscillator_strength_keeps_the_local_field_factor() {
        // S = S_bare * ((eps_inf + 2)/3)^2: at eps_inf = 2.34 the local-field factor is ((4.34)/3)^2 = 2.09, so a
        // bare strength of 1.0 becomes 2.09 -- dropping the factor would halve S and masquerade as a charge error.
        let s = oscillator_strength(Fixed::ONE, Fixed::from_ratio(234, 100)).unwrap();
        assert!(
            close(s.to_f64_lossy(), 2.09, 0.02),
            "the Szigeti local-field factor ~2.09 is applied, got {}",
            s.to_f64_lossy()
        );
    }

    #[test]
    fn the_reststrahlen_band_is_reflective_with_the_pinned_branch() {
        // The Reststrahlen band (the feature the module exists to produce): with NaCl-like parameters (eps_inf 2.34,
        // S = eps_0 - eps_inf = 3.56, omega_TO 164, low damping), between omega_TO and omega_LO the real permittivity
        // is NEGATIVE, and the pinned branch (k >= 0) yields a near-zero n with large k, so the reflectivity
        // approaches one. A wrong branch cut would mangle exactly this.
        let (eps_re, eps_im) = lorentz_permittivity(
            Fixed::from_ratio(234, 100), // eps_inf
            Fixed::from_ratio(356, 100), // S
            Fixed::from_int(164),        // omega_TO
            Fixed::from_ratio(328, 100), // gamma = 0.02 * omega_TO
            Fixed::from_int(200),        // omega, in-band (between omega_TO 164 and omega_LO ~260)
        )
        .unwrap();
        assert!(
            eps_re.to_f64_lossy() < 0.0,
            "the in-band real permittivity is negative, got {}",
            eps_re.to_f64_lossy()
        );
        let (n, k) = refractive_index_from_permittivity(eps_re, eps_im).unwrap();
        assert!(
            k >= Fixed::ZERO,
            "the branch is pinned to k >= 0 (absorption)"
        );
        assert!(
            k.to_f64_lossy() > n.to_f64_lossy(),
            "in-band k exceeds n (n -> 0), got n={} k={}",
            n.to_f64_lossy(),
            k.to_f64_lossy()
        );
        let r = reflectivity(n, k).unwrap();
        assert!(
            r.to_f64_lossy() > 0.85,
            "the Reststrahlen reflectivity approaches one, got {}",
            r.to_f64_lossy()
        );
    }

    #[test]
    fn the_phonon_battery_pass_one_measured_lengths_isolates_the_generator() {
        // PASS 1 (calibration): the generator run through MEASURED lattice geometry, so a miss is generator error,
        // never length-estimator error. omega_TO = (1/2 pi c) sqrt(k/mu), k from the Badger column at the measured
        // r_e. Results and their honest reading:
        //  - silicate Si-O stretch, quartz r_e = 1.61 A -> ~994 cm^-1 vs the 9.7 um (1031) feature, -4%: VALIDATED.
        //  - free O-H, r_e = 0.957 A -> ~3818 cm^-1 vs gas-phase ~3700, +3%: VALIDATED (the ice 3.1 um band at 3226
        //    is H-bond-redshifted, a condensed-phase effect the single-oscillator model does not carry, ~+18%).
        //  - SiC stretch, r_e = 1.889 A -> ~597 cm^-1 vs the measured TO ~793, -25%: the strongly-covalent Si-C bond
        //    is stiffer than the polar-weighted Badger (1,2) average, a declared generator band for covalent bonds.
        let fc = crate::force_constant::ForceConstants::standard().unwrap();
        let t = crate::periodic::PeriodicTable::standard().unwrap();
        let wto = |a_z, b_z, a, b, re: Fixed| -> f64 {
            let k = fc.force_constant_mdyn_per_angstrom(a_z, b_z, re).unwrap();
            let mu = crate::force_constant::ForceConstants::reduced_mass_amu(&t, a, b).unwrap();
            omega_to_cm_inverse(k, mu).unwrap().to_f64_lossy()
        };
        // The clean validations (the generator where Badger + a single oscillator apply).
        let si_o = wto(14, 8, "Si", "O", Fixed::from_ratio(161, 100));
        assert!(
            (si_o - 1031.0).abs() / 1031.0 < 0.10,
            "silicate Si-O stretch validates (~1031), got {si_o}"
        );
        let o_h = wto(8, 1, "O", "H", Fixed::from_ratio(957, 1000));
        assert!(
            (o_h - 3700.0).abs() / 3700.0 < 0.10,
            "the generator gives the free O-H (~3700), got {o_h}"
        );
        // The declared covalent band: SiC underestimates by ~25% (locked so a regression is caught, flagged as the
        // generator's honest band for strongly-covalent bonds, not a pass).
        let si_c = wto(14, 6, "Si", "C", Fixed::from_ratio(1889, 1000));
        assert!(
            si_c < 700.0 && si_c > 500.0,
            "SiC sits in the declared covalent underestimate band (~597, -25% vs 793), got {si_c}"
        );
    }

    #[test]
    fn the_phonon_battery_pass_two_estimator_lengths_grades_the_length_rung() {
        // PASS 2 (alien rung): the SAME features through RADII-SUM lengths, grading the length estimator (a miss
        // here convicts the length estimator, never the validated frequency core). The tetrahedral Si-C length
        // (194.9 pm) is longer than the measured 188.9, so it softens omega_TO further below the measured-length
        // result, the honest length band the production chain carries when no crystal has been measured.
        let radii = crate::covalent_radii::CovalentRadii::standard().unwrap();
        let fc = crate::force_constant::ForceConstants::standard().unwrap();
        let t = crate::periodic::PeriodicTable::standard().unwrap();
        let re_tet = radii.tetrahedral_bond_length_pm("Si", "C").unwrap(); // pm
        let re_ang = re_tet.checked_div(Fixed::from_int(100)).unwrap();
        let k = fc.force_constant_mdyn_per_angstrom(14, 6, re_ang).unwrap();
        let mu = crate::force_constant::ForceConstants::reduced_mass_amu(&t, "Si", "C").unwrap();
        let w_tet = omega_to_cm_inverse(k, mu).unwrap().to_f64_lossy();
        // The measured-length pass-1 result for comparison.
        let k_meas = fc
            .force_constant_mdyn_per_angstrom(14, 6, Fixed::from_ratio(1889, 1000))
            .unwrap();
        let w_meas = omega_to_cm_inverse(k_meas, mu).unwrap().to_f64_lossy();
        assert!(
            w_tet < w_meas,
            "the tetrahedral length estimator softens omega_TO below the measured-length pass ({w_tet} < {w_meas}), the estimator's band"
        );
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
