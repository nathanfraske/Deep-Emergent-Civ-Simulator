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

//! Stage 6, the band-gap tier's Harrison rung (`docs/working/STAGE6_ELECTRONIC_STRUCTURE_DESIGN.md` section 10,
//! owner ruling Part 4): the universal two-center tight-binding matrix elements, the foundation the sp bond-orbital
//! gap estimator builds on. The reduced-order MIDDLE rung of the provenance ladder (measured `[M]` top, this
//! estimator middle, compute-once bottom), ESTIMATOR grade, so its gaps feed classification, ranking, and optical
//! cast but NEVER the carrier-density exponent (the exponent rider, [`crate::band_gap::GapGrade::Estimator`]).
//!
//! THE PINNED CANONICAL QUARTET (owner ruling Part 4, rider 1). Harrison's universal two-center matrix element is
//! `V_{ll'm} = eta_{ll'm} * hbar^2 / (m_e * d^2)`, with `d` the interatomic distance and `eta_{ll'm}` a
//! dimensionless universal coefficient. The canonical Harrison 1980 quartet is pinned to a SINGLE provenance
//! (Harrison, Electronic Structure and the Properties of Solids, 1980), NOT the modified set that coexists in the
//! literature, so the reassembly and trend gates never chase mixed generations: `eta_ss_sigma = -1.40`,
//! `eta_sp_sigma = 1.84`, `eta_pp_sigma = 3.24`, `eta_pp_pi = -0.81`. These are pure numbers (the physics FLOOR of
//! authored law constants, no dimensional residue), and the sign convention is Harrison's (sigma bonds through
//! s-s negative, p-p positive; pi bond negative).
//!
//! THE DIMENSIONAL PREFACTOR, assembled (the dimensionless-constant law). `hbar^2 / m_e = 7.62 eV * Angstrom^2` is
//! a dimensional constant; it reassembles from the exact SI mantissas of `hbar` and `m_e` and the eV and Angstrom
//! unit folds, a single power of ten netting out, so no folded dimensional decimal is authored. With `d` in
//! Angstrom, `V_{ll'm}` comes out in eV.
//!
//! SCOPE. This is the two-center matrix-element PRIMITIVE only; the sp bond-orbital gap (the covalent energy `V_2`,
//! the polar energy `V_3` from the term-value column, and their combination into `E_gap`) is the next slice, its
//! term-value column gated on the one remaining book fetch (Herman-Skillman `eps_s`/`eps_p`). D-block substances
//! are excluded from the eventual estimator by tag (they route through the `U/W` preflight and `[M]` rows); the
//! primitive itself is universal for any two-center bond. Byte-neutral: `civsim-materials` is a leaf.

use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;

/// A two-center tight-binding bond type, the key of Harrison's universal quartet. The four canonical
/// sigma/pi combinations the sp bond-orbital model reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoCenterBond {
    /// `ss_sigma`: the s-s sigma overlap (`eta = -1.40`).
    SsSigma,
    /// `sp_sigma`: the s-p sigma overlap (`eta = 1.84`).
    SpSigma,
    /// `pp_sigma`: the p-p sigma overlap (`eta = 3.24`).
    PpSigma,
    /// `pp_pi`: the p-p pi overlap (`eta = -0.81`).
    PpPi,
}

/// The pinned canonical Harrison 1980 dimensionless coefficient `eta_{ll'm}` for a bond type (rider 1: single
/// provenance, Harrison 1980, never the modified set). A pure number (no dimensional residue).
pub fn eta(bond: TwoCenterBond) -> Fixed {
    match bond {
        TwoCenterBond::SsSigma => Fixed::from_ratio(-140, 100),
        TwoCenterBond::SpSigma => Fixed::from_ratio(184, 100),
        TwoCenterBond::PpSigma => Fixed::from_ratio(324, 100),
        TwoCenterBond::PpPi => Fixed::from_ratio(-81, 100),
    }
}

/// The Harrison dimensional prefactor `hbar^2 / m_e = 7.62 eV * Angstrom^2` (so `V = eta * prefactor / d^2` is in
/// eV with `d` in Angstrom). ASSEMBLED from the exact SI mantissas (the dimensionless-constant law, no folded
/// dimensional decimal): `hbar^2 / m_e` in `eV * Angstrom^2` is
/// `(hbar_mantissa^2 / m_e_mantissa / e_mantissa) * 10^2`, since `hbar^2` carries `10^-68`, `m_e` carries
/// `10^-31`, the joule-to-eV fold carries `10^19 / e`, and the `m^2`-to-`Angstrom^2` fold carries `10^20`, netting
/// `10^2`. The constituents underflow Q32.32 alone; only this collapsed form is representable.
fn harrison_prefactor_ev_angstrom2() -> Fixed {
    // hbar = 1.054571817e-34 J*s; m_e = 9.1093837015e-31 kg; e = 1.602176634e-19 C (exact SI mantissas).
    let hbar_mantissa = Fixed::from_ratio(1_054_571_817, 1_000_000_000);
    let me_mantissa = Fixed::from_ratio(91_093_837_015, 10_000_000_000);
    let e_mantissa = Fixed::from_ratio(1_602_176_634, 1_000_000_000);
    let hbar_sq = match hbar_mantissa.checked_mul(hbar_mantissa) {
        Some(v) => v,
        None => return ZERO,
    };
    // (hbar^2 / m_e / e) * 100: the collapsed 10^2 rides as the *100.
    hbar_sq
        .checked_div(me_mantissa)
        .and_then(|x| x.checked_div(e_mantissa))
        .and_then(|x| x.checked_mul(Fixed::from_int(100)))
        .unwrap_or(ZERO)
}

/// The universal two-center tight-binding matrix element `V_{ll'm} = eta_{ll'm} * hbar^2 / (m_e * d^2)` in eV, with
/// the interatomic distance `d` in Angstrom (Harrison 1980). Reserves no value: `eta` is the pinned canonical
/// quartet (physics-floor law constants), the prefactor reassembles from `hbar`, `m_e`, and `e`, and `d` is the
/// caller's structural datum. `None` for a non-positive distance (no bond) or on overflow.
pub fn two_center_matrix_element(bond: TwoCenterBond, d_angstrom: Fixed) -> Option<Fixed> {
    if d_angstrom <= ZERO {
        return None;
    }
    let d_sq = d_angstrom.checked_mul(d_angstrom)?;
    eta(bond)
        .checked_mul(harrison_prefactor_ev_angstrom2())?
        .checked_div(d_sq)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_prefactor_reassembles_from_hbar_and_the_electron_mass() {
        // THE DIMENSIONLESS-CONSTANT LAW: hbar^2/m_e reassembles to 7.62 eV*Angstrom^2 from the exact SI mantissas
        // of hbar, m_e, and e, with a single collapsed power of ten. The famous Harrison prefactor, not a folded
        // dimensional decimal.
        let p = harrison_prefactor_ev_angstrom2();
        assert!(
            close(p, 7.6199, 0.01),
            "hbar^2/m_e ~ 7.62 eV*Angstrom^2, got {}",
            p.to_f64_lossy()
        );
    }

    #[test]
    fn the_pinned_quartet_is_the_canonical_1980_set() {
        // Rider 1: the pinned canonical Harrison 1980 quartet, single provenance (never the modified set).
        assert!(close(eta(TwoCenterBond::SsSigma), -1.40, 1e-6));
        assert!(close(eta(TwoCenterBond::SpSigma), 1.84, 1e-6));
        assert!(close(eta(TwoCenterBond::PpSigma), 3.24, 1e-6));
        assert!(close(eta(TwoCenterBond::PpPi), -0.81, 1e-6));
    }

    #[test]
    fn the_matrix_elements_match_harrisons_tabulated_silicon_values() {
        // The primitive reproduces Harrison's own tabulated matrix elements for silicon (nearest-neighbour distance
        // d = 2.35 Angstrom, from a = 5.431 Angstrom, d = a*sqrt(3)/4): V_ppsigma = 3.24 * 7.62 / 2.35^2 ~ 4.47 eV,
        // V_sssigma ~ -1.93, V_spsigma ~ 2.54, V_pppi ~ -1.12. A cross-check of the primitive against Harrison's own
        // numbers (a consistency check on the pinned quartet and the prefactor, not an independent validation).
        let d = Fixed::from_ratio(235, 100);
        let pp_sigma = two_center_matrix_element(TwoCenterBond::PpSigma, d).expect("V_ppsigma");
        assert!(
            close(pp_sigma, 4.47, 0.03),
            "Si V_ppsigma ~ 4.47 eV, got {}",
            pp_sigma.to_f64_lossy()
        );
        let ss_sigma = two_center_matrix_element(TwoCenterBond::SsSigma, d).expect("V_sssigma");
        assert!(
            close(ss_sigma, -1.93, 0.03),
            "Si V_sssigma ~ -1.93 eV, got {}",
            ss_sigma.to_f64_lossy()
        );
        // The signs follow Harrison's convention: ss_sigma and pp_pi negative, sp_sigma and pp_sigma positive.
        assert!(two_center_matrix_element(TwoCenterBond::SpSigma, d).unwrap() > ZERO);
        assert!(two_center_matrix_element(TwoCenterBond::PpPi, d).unwrap() < ZERO);
    }

    #[test]
    fn the_matrix_element_scales_as_the_inverse_square_distance() {
        // V ~ 1/d^2: halving the distance quadruples the magnitude, the Harrison d^-2 law the homopolar gap trend
        // rides on (a wider-gap C has a shorter bond than Si).
        let short = two_center_matrix_element(TwoCenterBond::PpSigma, Fixed::from_int(2)).unwrap();
        let long = two_center_matrix_element(TwoCenterBond::PpSigma, Fixed::from_int(4)).unwrap();
        // (4/2)^2 = 4: the shorter bond's matrix element is 4x the longer.
        assert!(
            close(short.checked_div(long).unwrap(), 4.0, 0.01),
            "V(2A)/V(4A) = 4 (the inverse-square law)"
        );
    }

    #[test]
    fn a_non_positive_distance_has_no_bond() {
        assert!(two_center_matrix_element(TwoCenterBond::PpSigma, ZERO).is_none());
        assert!(two_center_matrix_element(TwoCenterBond::PpSigma, Fixed::from_int(-1)).is_none());
    }
}
