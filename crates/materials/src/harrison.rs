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
//! THE SP BOND-ORBITAL ESTIMATOR (this slice, unblocked by the owner's Table 4-1 / Table 2-3 / Table 2-2 delivery).
//! On the two-center primitive the tetrahedral-solid bond-orbital model builds the emergent bonding descriptors: the
//! covalent energy `V_2 = 2.16 * hbar^2 / (m_e * d^2)` (Harrison's universal tetrahedral coefficient, Table 4-1),
//! the polar energy `V_3` from the term-value column, the covalency `alpha_c = V_2 / sqrt(V_2^2 + V_3^2)`, the
//! polarity `alpha_p`, and the average gap `E_g = 2 * sqrt(V_2^2 + V_3^2)`. The average gap is a Penn/optical-scale
//! quantity (Si ~ 5.96 eV), NOT the fundamental band gap (Si 1.12 eV), so it is ESTIMATOR grade
//! ([`crate::band_gap::GapGrade::Estimator`]): it feeds bonding-character ranking and optical cast, never the
//! carrier-density exponent (which needs a measured or compute-once fundamental gap).
//!
//! THE POLAR-ENERGY PROVE-IT CATCH (verified against the owner's scans). Harrison's TABULATED polar energy `V_3` is
//! HALF THE P-TERM-VALUE DIFFERENCE, `V_3 = (eps_p_anion - eps_p_cation) / 2`, NOT half the sp3 HYBRID-energy
//! difference. The hybrid form (an earlier design draft) gives GaAs `V_3 = 1.87`; the tabulated value is 1.51. The
//! p-difference reproduces it (As `eps_p` -7.91, Ga -4.90 give 1.505 ~ 1.51) and ZnSe (Se -9.53, Zn -3.38 give
//! 3.075 ~ 3.08), it matches Table 2-3's own `(eps_p^(1) - eps_p^(2))/2` row, and it is the ONLY `V_3` that
//! reproduces Table 4-1's covalency column (GaAs `alpha_c` 0.88, ZnSe 0.66); the hybrid `V_3` fails all three. The
//! estimator reads the tabulated p-difference.
//!
//! SCOPE. D-block substances are excluded from the estimator by tag (they route through the `U/W` preflight and
//! `[M]` rows); the two-center primitive is universal for any two-center bond. Byte-neutral: `civsim-materials` is a
//! leaf.

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

/// Harrison's universal covalent-energy coefficient for tetrahedral solids, the `2.16` in
/// `V_2 = 2.16 * hbar^2 / (m_e * d^2)` (Harrison 1980 Chapter 4, tabulated in Table 4-1). A pure dimensionless fit
/// coefficient (a physics-floor law constant), cited never fabricated: it back-fits to `2.160` for every tetrahedral
/// entry in Table 4-1 (C `6.94 * 1.54^2 / 7.62`, Si `2.98 * 2.35^2 / 7.62`, Ge `2.76 * 2.44^2 / 7.62`, all `2.160`).
/// This is the sp3 BOND covalent energy, distinct from the individual two-center `eta` quartet (a single matrix
/// element); the `2.16` is Harrison's stated universal covalent energy for the bond-orbital model.
fn covalent_coefficient() -> Fixed {
    Fixed::from_ratio(216, 100)
}

/// The Harrison covalent energy `V_2 = 2.16 * hbar^2 / (m_e * d^2)` in eV, with the bond length `d` in Angstrom: the
/// homopolar bonding-antibonding half-splitting of the sp3 bond (Harrison 1980). Reads only the bond length; the
/// coefficient and the prefactor are cited constants (no reserved value). `None` for a non-positive distance (no
/// bond) or on overflow. The `V_2 ~ d^-2` law drives the homopolar gap trend (a shorter bond, a wider gap).
pub fn covalent_energy_v2(d_angstrom: Fixed) -> Option<Fixed> {
    if d_angstrom <= ZERO {
        return None;
    }
    let d_sq = d_angstrom.checked_mul(d_angstrom)?;
    covalent_coefficient()
        .checked_mul(harrison_prefactor_ev_angstrom2())?
        .checked_div(d_sq)
}

/// The Harrison polar energy `V_3 = |eps_p_a - eps_p_b| / 2` in eV, HALF THE P-TERM-VALUE DIFFERENCE of the two
/// bonded atoms. This is the prove-it-corrected form: Harrison's TABULATED `V_3` (Table 2-3's
/// `(eps_p^(1) - eps_p^(2))/2` row, Table 4-1's `V_3` column) is the p-term-value half-difference, NOT the sp3
/// hybrid-energy half-difference (which would give GaAs `1.87` against the tabulated `1.51`). Verified against Table
/// 4-1: GaAs (As `eps_p` -7.91, Ga -4.90) gives `1.505 ~ 1.51`; ZnSe (Se -9.53, Zn -3.38) gives `3.075 ~ 3.08`. The
/// magnitude is returned (the polar energy is unsigned in the covalency and gap forms), so the caller need not know
/// which atom is the cation. `None` on overflow.
pub fn polar_energy_v3(eps_p_a: Fixed, eps_p_b: Fixed) -> Option<Fixed> {
    let diff = eps_p_a.checked_sub(eps_p_b)?;
    diff.abs().checked_div(Fixed::from_int(2))
}

/// The bond magnitude `sqrt(V_2^2 + V_3^2)`, the radius of the covalent-polar bonding energy. A sum of squares, so
/// the `sqrt` is always real (never the negative-input branch). `None` on overflow of a square or the sum.
fn bond_magnitude(v2: Fixed, v3: Fixed) -> Option<Fixed> {
    let v2_sq = v2.checked_mul(v2)?;
    let v3_sq = v3.checked_mul(v3)?;
    Some(v2_sq.checked_add(v3_sq)?.sqrt())
}

/// The bond covalency `alpha_c = V_2 / sqrt(V_2^2 + V_3^2)` (Harrison 1980): `1` for a pure homopolar covalent bond
/// (`V_3 = 0`), falling toward `0` as the bond grows ionic. It emerges from the bond length (through `V_2`) and the
/// term-value polarity (through `V_3`), and reproduces Table 4-1's covalency column across the isoelectronic row
/// (Ge `1.0` -> GaAs `0.88` -> ZnSe `0.66`). `None` on a degenerate zero-magnitude bond or overflow.
pub fn bond_covalency(v2: Fixed, v3: Fixed) -> Option<Fixed> {
    let mag = bond_magnitude(v2, v3)?;
    if mag <= ZERO {
        return None;
    }
    v2.checked_div(mag)
}

/// The bond polarity `alpha_p = V_3 / sqrt(V_2^2 + V_3^2)` (Harrison 1980): `0` for a pure homopolar covalent bond,
/// rising toward `1` as the bond grows ionic. The covalency's complement (`alpha_c^2 + alpha_p^2 = 1`). `None` on a
/// degenerate zero-magnitude bond or overflow.
pub fn bond_polarity(v2: Fixed, v3: Fixed) -> Option<Fixed> {
    let mag = bond_magnitude(v2, v3)?;
    if mag <= ZERO {
        return None;
    }
    v3.checked_div(mag)
}

/// The Harrison sp bond-orbital average gap `E_g = 2 * sqrt(V_2^2 + V_3^2)` in eV: the bonding-antibonding splitting
/// of the sp3 bond, a Penn/optical-scale quantity (Si ~ 5.96 eV), NOT the fundamental band gap (Si 1.12 eV).
/// ESTIMATOR grade ([`crate::band_gap::GapGrade::Estimator`]): it feeds bonding-character ranking and optical cast,
/// never the carrier-density exponent (which needs a measured or compute-once fundamental gap). Reproduces both
/// pre-registered trends: homopolar C>Si>Ge>alpha-Sn (through `V_2 ~ d^-2`) and isoelectronic Ge->GaAs->ZnSe
/// (through rising `V_3`). `None` on overflow.
pub fn bond_orbital_average_gap_ev(v2: Fixed, v3: Fixed) -> Option<Fixed> {
    bond_magnitude(v2, v3)?.checked_mul(Fixed::from_int(2))
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

    // The estimator's cited test fixtures are the owner-delivered Harrison Table 4-1 bond lengths (Angstrom) and
    // Table 2-2 p-term values (eV), used as clearly-labeled cited inputs, never seeded from memory.
    fn average_gap(d: Fixed, eps_p_a: Fixed, eps_p_b: Fixed) -> Fixed {
        let v2 = covalent_energy_v2(d).expect("V_2");
        let v3 = polar_energy_v3(eps_p_a, eps_p_b).expect("V_3");
        bond_orbital_average_gap_ev(v2, v3).expect("E_g")
    }

    #[test]
    fn the_covalent_energy_reproduces_harrisons_table_4_1() {
        // V_2 = 2.16 * hbar^2/(m_e d^2). Silicon d = 2.35 A -> 2.98 eV; GaAs d = 2.45 A -> 2.74 eV (Table 4-1). A
        // consistency check of the covalent coefficient and prefactor against Harrison's own tabulated V_2.
        let si = covalent_energy_v2(Fixed::from_ratio(235, 100)).expect("Si V_2");
        assert!(
            close(si, 2.98, 0.02),
            "Si V_2 ~ 2.98 eV, got {}",
            si.to_f64_lossy()
        );
        let gaas = covalent_energy_v2(Fixed::from_ratio(245, 100)).expect("GaAs V_2");
        assert!(
            close(gaas, 2.74, 0.02),
            "GaAs V_2 ~ 2.74 eV, got {}",
            gaas.to_f64_lossy()
        );
    }

    #[test]
    fn the_polar_energy_is_the_p_term_value_difference_not_the_hybrid() {
        // THE PROVE-IT CATCH: Harrison's tabulated V_3 is half the P-term-value difference, not half the sp3 hybrid
        // difference. GaAs from p-values (As eps_p -7.91, Ga -4.90): |(-7.91)-(-4.90)|/2 = 1.505, matching Table
        // 4-1's 1.51. The hybrid difference (eps_h As -10.265, Ga -6.5175) would give 1.874, which Table 4-1 does NOT
        // list. ZnSe (Se -9.53, Zn -3.38): 3.075 ~ Table 4-1's 3.08.
        let gaas = polar_energy_v3(Fixed::from_ratio(-791, 100), Fixed::from_ratio(-490, 100))
            .expect("GaAs V_3");
        assert!(
            close(gaas, 1.505, 0.01),
            "GaAs V_3 ~ 1.51 (the p-difference, not the hybrid 1.87), got {}",
            gaas.to_f64_lossy()
        );
        let znse = polar_energy_v3(Fixed::from_ratio(-953, 100), Fixed::from_ratio(-338, 100))
            .expect("ZnSe V_3");
        assert!(
            close(znse, 3.075, 0.01),
            "ZnSe V_3 ~ 3.08, got {}",
            znse.to_f64_lossy()
        );
        // A homopolar bond (same eps_p on both atoms) has zero polar energy.
        let homopolar = polar_energy_v3(Fixed::from_ratio(-652, 100), Fixed::from_ratio(-652, 100))
            .expect("Si V_3");
        assert_eq!(homopolar, ZERO, "a homopolar bond is unpolarized");
    }

    #[test]
    fn the_homopolar_gap_trend_gate_holds_c_gt_si_gt_ge_gt_sn() {
        // PRE-REGISTERED TREND GATE 1: for the homopolar group-IV row (V_3 = 0, so E_g = 2*V_2 ~ d^-2), the average
        // gap falls strictly with the bond length: C (d 1.54) > Si (2.35) > Ge (2.44) > alpha-Sn (2.80). The gap
        // ORDER is the emergent result; the absolutes (C 13.88, Si 5.96, alpha-Sn 4.20 eV) are the Penn/optical
        // scale, not the fundamental gap.
        let c = average_gap(
            Fixed::from_ratio(154, 100),
            Fixed::from_ratio(-897, 100),
            Fixed::from_ratio(-897, 100),
        );
        let si = average_gap(
            Fixed::from_ratio(235, 100),
            Fixed::from_ratio(-652, 100),
            Fixed::from_ratio(-652, 100),
        );
        let ge = average_gap(
            Fixed::from_ratio(244, 100),
            Fixed::from_ratio(-636, 100),
            Fixed::from_ratio(-636, 100),
        );
        let sn = average_gap(
            Fixed::from_ratio(280, 100),
            Fixed::from_ratio(-594, 100),
            Fixed::from_ratio(-594, 100),
        );
        assert!(
            c > si && si > ge && ge > sn,
            "C>Si>Ge>alpha-Sn: {} {} {} {}",
            c.to_f64_lossy(),
            si.to_f64_lossy(),
            ge.to_f64_lossy(),
            sn.to_f64_lossy()
        );
        assert!(close(c, 13.88, 0.05) && close(si, 5.96, 0.05) && close(sn, 4.20, 0.05));
    }

    #[test]
    fn the_isoelectronic_gap_trend_gate_holds_ge_lt_gaas_lt_znse() {
        // PRE-REGISTERED TREND GATE 2: across the isoelectronic row at ~constant bond length (Ge/GaAs/ZnSe, d ~ 2.44-
        // 2.45 A), the average gap RISES with polarity as V_3 grows (Ge 0 -> GaAs 1.51 -> ZnSe 3.08): E_g 5.52 ->
        // 6.26 -> 8.24 eV. Same-row bond length near-constant, so the trend is polarity, not bond length.
        let ge = average_gap(
            Fixed::from_ratio(244, 100),
            Fixed::from_ratio(-636, 100),
            Fixed::from_ratio(-636, 100),
        );
        let gaas = average_gap(
            Fixed::from_ratio(245, 100),
            Fixed::from_ratio(-791, 100),
            Fixed::from_ratio(-490, 100),
        );
        let znse = average_gap(
            Fixed::from_ratio(245, 100),
            Fixed::from_ratio(-953, 100),
            Fixed::from_ratio(-338, 100),
        );
        assert!(
            ge < gaas && gaas < znse,
            "Ge<GaAs<ZnSe: {} {} {}",
            ge.to_f64_lossy(),
            gaas.to_f64_lossy(),
            znse.to_f64_lossy()
        );
        assert!(close(gaas, 6.26, 0.05) && close(znse, 8.24, 0.05));
    }

    #[test]
    fn the_covalency_falls_across_the_isoelectronic_row() {
        // alpha_c = V_2/sqrt(V_2^2+V_3^2) reproduces Table 4-1's covalency column: Ge 1.0 (pure covalent) -> GaAs
        // 0.88 -> ZnSe 0.66 (increasingly ionic). The emergent bonding-character trend, and a fourth cross-check that
        // the p-difference V_3 (not the hybrid) is the right one: only the p-difference reproduces this column.
        let ge_ac = bond_covalency(
            covalent_energy_v2(Fixed::from_ratio(244, 100)).unwrap(),
            ZERO,
        )
        .expect("Ge alpha_c");
        let gaas_v2 = covalent_energy_v2(Fixed::from_ratio(245, 100)).unwrap();
        let gaas_v3 =
            polar_energy_v3(Fixed::from_ratio(-791, 100), Fixed::from_ratio(-490, 100)).unwrap();
        let gaas_ac = bond_covalency(gaas_v2, gaas_v3).expect("GaAs alpha_c");
        let znse_v2 = covalent_energy_v2(Fixed::from_ratio(245, 100)).unwrap();
        let znse_v3 =
            polar_energy_v3(Fixed::from_ratio(-953, 100), Fixed::from_ratio(-338, 100)).unwrap();
        let znse_ac = bond_covalency(znse_v2, znse_v3).expect("ZnSe alpha_c");
        assert!(
            close(ge_ac, 1.0, 0.001) && close(gaas_ac, 0.88, 0.01) && close(znse_ac, 0.66, 0.01),
            "alpha_c: Ge {} GaAs {} ZnSe {}",
            ge_ac.to_f64_lossy(),
            gaas_ac.to_f64_lossy(),
            znse_ac.to_f64_lossy()
        );
        assert!(
            ge_ac > gaas_ac && gaas_ac > znse_ac,
            "covalency falls with polarity"
        );
    }

    #[test]
    fn the_average_gap_is_estimator_grade_and_barred_from_the_exponent() {
        // The average gap is Penn/optical scale (Si 5.96 eV), not the fundamental gap, and is ESTIMATOR grade: fed to
        // the carrier-density activation with GapGrade::Estimator the exponent is barred (None), the exponent rider
        // (owner Part 3). The estimator gap reaches classification, ranking, and optical cast, never the exponent.
        let si_v2 = covalent_energy_v2(Fixed::from_ratio(235, 100)).unwrap();
        let e_g = bond_orbital_average_gap_ev(si_v2, ZERO).unwrap();
        let barred = crate::band_gap::ln_thermal_carrier_activation(
            e_g,
            Fixed::from_int(300),
            crate::band_gap::GapGrade::Estimator,
        );
        assert!(
            barred.is_none(),
            "an estimator-grade gap is barred from the carrier-density exponent"
        );
    }
}
