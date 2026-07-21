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

//! Stage 6, the magnetism sub-arc (`docs/working/STAGE6_ELECTRONIC_STRUCTURE_DESIGN.md` section 11, gate ruled):
//! slice (a), the Hund's-rule spin-only LOCAL moment, dispatched on the banked `U/W` correlation classifier's
//! Localized class; and slice (b), the octahedral crystal-field high-spin/low-spin correction over that moment (the
//! Griffith spin-pairing comparison against the `10Dq` column).
//!
//! The free-ion Hund spin-only moment DERIVES from the banked floor, reserving no value (the derive-first result
//! the opener surfaced). For a 3d transition-metal ion the d-electron count is `d = (Z - 18) - q`, the valence
//! electrons above the argon core (`Z - 18`) less the ion's charge `q` (the ionization empties the 4s pool first,
//! so the count of the remaining valence electrons is the d-count). Hund's first rule (maximum multiplicity) gives
//! the unpaired count `n = 5 - |d - 5|` (`d` for `d <= 5`, `10 - d` for `d >= 5`), and the spin-only moment is
//! `mu = sqrt(n (n + 2))` Bohr magnetons. So iron(II) is `d^6`, four unpaired, `mu = sqrt(24) = 4.90 mu_B`, the
//! standard spin-only value. The Bohr magneton is a fundamental constant (the physics floor); the moment is
//! reported in units of it, so no dimensional value is authored.
//!
//! THE DISPATCH (consume, not rebuild). [`hund_local_moment`] runs the banked [`CorrelationClassifier`]: a
//! Localized (Mott) centre carries a Hund local moment, and an Itinerant, Window, or out-of-scope material
//! escalates (the itinerant Stoner branch is the deep piece, section 11.3, a later slice). The correlated cation
//! and its charge are read from the classifier's own pair identification, never re-derived.
//!
//! THE CRYSTAL-FIELD CORRECTION (slice (b), the `10Dq` fork). The free-ion moment above assumes high-spin filling; a
//! strong octahedral field pairs electrons in the `t2g` level before the `eg`, lowering the unpaired count. The
//! choice is the Griffith spin-pairing comparison (Griffith 1961): the low-spin energy shift is
//! `E_LS - E_HS = |Delta_D| * D - m * Delta_o`, with the cited per-configuration coefficients (`d4`-`d7` the only
//! configurations with a choice), `Delta_o` from the crystal-field column, and `D` the pairing scale. Low-spin wins
//! when the field gain `m * Delta_o` exceeds the pairing cost `|Delta_D| * D`; within a resolution of the boundary
//! the comparison escalates (a spin-crossover margin) rather than forcing a binary. The low-spin threshold
//! `Delta_o / D` rises `d4` (1.33) < `d5` (1.5) < `d6` (2.0) < `d7` (2.67), so monoxide fields
//! (`Delta_o ~ 8000 cm^-1`) sit far below every threshold and stay high-spin, matching the observed 3d monoxide
//! moments. The one wire-time reserved input is the pairing scale `D` in cm^-1 (Griffith's spin-pairing `D` as a
//! function of the Racah `B` and `C`, sibling to the `B` already in the crystal-field column), surfaced not
//! fabricated; the mechanism and the cited Griffith coefficients are byte-neutral and settled.
//!
//! HONEST LIMITS (stated at the site): the spin-only moment omits the orbital contribution (which lifts the
//! observed moment above spin-only for the early and late 3d ions). [`hund_local_moment`] assumes HIGH-SPIN free-ion
//! filling; the octahedral low-spin correction is slice (b) above ([`octahedral_spin_moment`]), its pairing scale
//! `D` the one wire-time reserved input. The d-count derivation is scoped to the 3d series (`Z` in `21..=30`), the
//! classifier's own scope; a 4d/5d/4f centre needs its own principal-shell range, a flagged follow-on. Byte-neutral:
//! `civsim-materials` is a leaf.

use civsim_core::Fixed;
use civsim_physics::periodic::PeriodicTable;

use crate::correlation::{CorrelationClass, CorrelationClassifier};

/// The lowest atomic number of the 3d transition series (scandium) and the argon core size, the scope of the
/// d-count derivation (the classifier's own 3d scope).
const THREE_D_Z_MIN: u8 = 21;
const THREE_D_Z_MAX: u8 = 30;
const ARGON_CORE: i32 = 18;

/// The d-electron count `d = (Z - 18) - q` of a 3d transition-metal ion at charge `q`, or `None` when the element
/// is outside the 3d series (`Z` in `21..=30`) or the count falls outside `0..=10` (not a valid d-shell filling).
/// A derivation over the banked `Z` and the ion charge, no reserved value.
pub fn d_electron_count_3d(z: u8, charge: u32) -> Option<u32> {
    if !(THREE_D_Z_MIN..=THREE_D_Z_MAX).contains(&z) {
        return None;
    }
    let d = (z as i32 - ARGON_CORE) - charge as i32;
    if (0..=10).contains(&d) {
        Some(d as u32)
    } else {
        None
    }
}

/// The number of unpaired electrons of a `d^d` shell under Hund's first rule (maximum multiplicity):
/// `n = 5 - |d - 5|`, so `d` unpaired for `d <= 5` (filling singly) and `10 - d` for `d >= 5` (pairing). `None`
/// for a `d` above ten (not a d-shell).
pub fn hund_unpaired_count(d_count: u32) -> Option<u32> {
    if d_count > 10 {
        return None;
    }
    Some((5 - (d_count as i32 - 5).abs()) as u32)
}

/// The spin-only magnetic moment `mu = sqrt(n (n + 2))` in Bohr magnetons for `n` unpaired electrons (the
/// spin-only formula, `g = 2`, `S = n/2`). Reserves no value: `n` is derived and the Bohr magneton is the reporting
/// unit (a fundamental constant). Zero for a filled or empty shell (no unpaired electrons, diamagnetic).
pub fn spin_only_moment_bohr(n_unpaired: u32) -> Fixed {
    let n = Fixed::from_int(n_unpaired as i32);
    let n_plus_2 = Fixed::from_int(n_unpaired as i32 + 2);
    match n.checked_mul(n_plus_2) {
        Some(v) => v.sqrt(),
        None => Fixed::ZERO,
    }
}

/// The Hund's-rule spin-only LOCAL moment (Bohr magnetons) of a composition, dispatched on the banked `U/W`
/// classifier: a Localized (Mott) centre carries the moment on its correlated d-block cation, and an Itinerant,
/// Window, or out-of-scope material escalates (`None`, the itinerant Stoner branch being a later slice). The
/// correlated cation and its charge are read from the classifier (consume, not rebuild), the atomic number from the
/// banked periodic table. Reserves no value.
pub fn hund_local_moment(
    composition: &[(String, u32)],
    classifier: &CorrelationClassifier,
    table: &PeriodicTable,
) -> Option<Fixed> {
    // The dispatch: only a Localized (Mott) centre carries a local moment here; everything else escalates.
    if classifier.classify(composition) != CorrelationClass::Localized {
        return None;
    }
    let (cation, charge) = classifier.correlated_cation(composition)?;
    let z = table.element(&cation)?.z;
    let d_count = d_electron_count_3d(z, charge)?;
    let n_unpaired = hund_unpaired_count(d_count)?;
    Some(spin_only_moment_bohr(n_unpaired))
}

/// The octahedral high-spin / low-spin choice for a `d^n` ion, resolved by the Griffith spin-pairing comparison
/// (slice (b), the `10Dq` correction to the free-ion high-spin moment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OctahedralSpin {
    /// No high-spin/low-spin choice for this d-count (`d0`-`d3`, `d8`-`d10`): one ground configuration, the free-ion
    /// count stands.
    NoChoice,
    /// High-spin: the crystal field is too weak to overcome the pairing cost (`m * Delta_o < |Delta_D| * D`).
    HighSpin,
    /// Low-spin: the crystal field overcomes the pairing cost (`m * Delta_o > |Delta_D| * D`).
    LowSpin,
    /// The high-spin and low-spin energies lie within the caller's resolution of each other: the uniform Griffith
    /// comparison cannot cleanly call it (a spin-crossover margin), so it escalates rather than forcing a binary.
    Escalate,
}

/// The Griffith octahedral spin-pairing coefficients for a d-count with a high-spin/low-spin choice (`d4`-`d7`,
/// Griffith 1961): the pairing-cost coefficient `|Delta_D|` (in units of the pairing scale `D`) and the field-gain
/// coefficient `m` (the number of `eg` electrons that drop to `t2g` on the high-spin -> low-spin switch, so the
/// low-spin state gains `m * Delta_o` of crystal-field stabilization). `None` for a d-count with no choice
/// (`d0`-`d3`, `d8`-`d10`). Cited data (the delivered Griffith table), NOT fabricated: the coefficients are exact
/// rationals, internally consistent (`|Delta_D|` is `D_HS - D_LS`, `m` is the `eg`-population change), and reproduce
/// the low-spin threshold ordering `d4 < d5 < d6 < d7`.
fn griffith_octahedral_coeffs(d_count: u32) -> Option<(Fixed, u32)> {
    match d_count {
        4 => Some((Fixed::from_ratio(4, 3), 1)),
        5 => Some((Fixed::from_int(3), 2)),
        6 => Some((Fixed::from_int(4), 2)),
        7 => Some((Fixed::from_ratio(8, 3), 1)),
        _ => None,
    }
}

/// The octahedral spin-state decision (Griffith 1961): the low-spin energy shift relative to high-spin is
/// `E_LS - E_HS = |Delta_D| * D - m * Delta_o`, where `D` is the pairing scale (cm^-1) and `Delta_o` the octahedral
/// crystal-field splitting (cm^-1). Low-spin when the shift falls below `-resolution` (the field wins), high-spin
/// above `+resolution` (pairing wins), escalate within the `resolution` band (the uniform comparison cannot resolve
/// it). A d-count with no choice (`d0`-`d3`, `d8`-`d10`) returns [`OctahedralSpin::NoChoice`].
///
/// Reserves no value here: `Delta_o` is read from the crystal-field column, `resolution` is the caller's escalate
/// margin (the Verdict resolution pattern), and the Griffith coefficients are cited data. The pairing scale `D` is
/// the one wire-time reserved input (Griffith's spin-pairing `D` as a function of the Racah `B` and `C`), supplied
/// by the caller; this function is the comparison, not the source of `D`. `Escalate` on any arithmetic overflow (the
/// safe escalation, never a fabricated verdict).
pub fn octahedral_spin_decision(
    d_count: u32,
    delta_o_cm: Fixed,
    pairing_d_cm: Fixed,
    resolution_cm: Fixed,
) -> OctahedralSpin {
    let Some((delta_d, m)) = griffith_octahedral_coeffs(d_count) else {
        return OctahedralSpin::NoChoice;
    };
    // E_LS - E_HS = |Delta_D| * D - m * Delta_o (Delta_D is stored as its magnitude).
    let pairing_cost = match delta_d.checked_mul(pairing_d_cm) {
        Some(v) => v,
        None => return OctahedralSpin::Escalate,
    };
    let field_gain = match Fixed::from_int(m as i32).checked_mul(delta_o_cm) {
        Some(v) => v,
        None => return OctahedralSpin::Escalate,
    };
    let shift = match pairing_cost.checked_sub(field_gain) {
        Some(v) => v,
        None => return OctahedralSpin::Escalate,
    };
    let neg_resolution = match Fixed::ZERO.checked_sub(resolution_cm) {
        Some(v) => v,
        None => return OctahedralSpin::Escalate,
    };
    if shift < neg_resolution {
        OctahedralSpin::LowSpin
    } else if shift > resolution_cm {
        OctahedralSpin::HighSpin
    } else {
        OctahedralSpin::Escalate
    }
}

/// The number of unpaired electrons of a `d^n` shell in an octahedral LOW-SPIN field: the `t2g` level (three
/// orbitals, up to six electrons) fills and pairs before the `eg` level (two orbitals, up to four) is occupied, each
/// level filling singly then pairing. Equals the high-spin count below `d4` (no pairing choice there): `d4` gives 2,
/// `d5` gives 1, `d6` gives 0, `d7` gives 1. `None` for `d > 10` (not a d-shell).
pub fn low_spin_unpaired_count(d_count: u32) -> Option<u32> {
    if d_count > 10 {
        return None;
    }
    let t2g = d_count.min(6);
    let eg = d_count - t2g;
    let t2g_unpaired = if t2g <= 3 { t2g } else { 6 - t2g };
    let eg_unpaired = if eg <= 2 { eg } else { 4 - eg };
    Some(t2g_unpaired + eg_unpaired)
}

/// The octahedral spin-only moment (Bohr magnetons) for a `d^n` ion, resolving the high-spin/low-spin choice by the
/// Griffith comparison (slice (b)). High-spin (and a no-choice d-count) uses the free-ion Hund count, low-spin the
/// `t2g`-first count. `None` when the decision escalates (the marginal band) or the count is out of range. The
/// `10Dq` correction to [`spin_only_moment_bohr`]: a strong field lowers the moment (a `d6` FeO-like high-spin
/// `4.90` collapses to low-spin `0`).
pub fn octahedral_spin_moment(
    d_count: u32,
    delta_o_cm: Fixed,
    pairing_d_cm: Fixed,
    resolution_cm: Fixed,
) -> Option<Fixed> {
    let n_unpaired =
        match octahedral_spin_decision(d_count, delta_o_cm, pairing_d_cm, resolution_cm) {
            OctahedralSpin::NoChoice | OctahedralSpin::HighSpin => hund_unpaired_count(d_count)?,
            OctahedralSpin::LowSpin => low_spin_unpaired_count(d_count)?,
            OctahedralSpin::Escalate => return None,
        };
    Some(spin_only_moment_bohr(n_unpaired))
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_physics::d_state_radius::DStateRadii;
    use civsim_physics::ionic_radii::IonicRadii;
    use civsim_physics::ionization_ladder::IonizationLadder;
    use civsim_physics::mit_reference::MitReference;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    fn comp(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
        pairs.iter().map(|(s, c)| ((*s).to_string(), *c)).collect()
    }

    fn floors() -> (
        PeriodicTable,
        IonizationLadder,
        DStateRadii,
        IonicRadii,
        MitReference,
    ) {
        (
            PeriodicTable::standard().expect("periodic table"),
            IonizationLadder::standard().expect("ionization ladder"),
            DStateRadii::standard(
                &civsim_units::constants::canonical_si_execution_magnitudes()
                    .expect("the sealed physical floor projects"),
            )
            .expect("d-state radii"),
            IonicRadii::standard().expect("ionic radii"),
            MitReference::standard().expect("MIT reference set"),
        )
    }

    #[test]
    fn the_d_count_derives_from_z_and_charge() {
        // d = (Z - 18) - q: Fe2+ (Z=26) is d6, Ni2+ (Z=28) is d8, Mn2+ (Z=25) is d5, Fe3+ is d5.
        assert_eq!(d_electron_count_3d(26, 2), Some(6), "Fe2+ is d6");
        assert_eq!(d_electron_count_3d(28, 2), Some(8), "Ni2+ is d8");
        assert_eq!(d_electron_count_3d(25, 2), Some(5), "Mn2+ is d5");
        assert_eq!(d_electron_count_3d(26, 3), Some(5), "Fe3+ is d5");
        // Out of the 3d series (or an impossible count) escalates.
        assert_eq!(
            d_electron_count_3d(13, 3),
            None,
            "aluminium is not a 3d centre"
        );
        assert_eq!(d_electron_count_3d(21, 5), None, "Sc5+ would be d-negative");
    }

    #[test]
    fn hund_gives_the_maximum_multiplicity_unpaired_count() {
        // n = 5 - |d - 5|: d5 (half-filled) is 5 unpaired (the peak), d0 and d10 are 0, d6 is 4, d8 is 2.
        assert_eq!(
            hund_unpaired_count(5),
            Some(5),
            "d5 half-filled, 5 unpaired"
        );
        assert_eq!(hund_unpaired_count(0), Some(0), "d0 empty, 0 unpaired");
        assert_eq!(hund_unpaired_count(10), Some(0), "d10 filled, 0 unpaired");
        assert_eq!(hund_unpaired_count(6), Some(4), "d6, 4 unpaired");
        assert_eq!(hund_unpaired_count(8), Some(2), "d8, 2 unpaired");
    }

    #[test]
    fn the_spin_only_moment_matches_the_standard_values() {
        // mu = sqrt(n(n+2)) mu_B: 5 unpaired -> sqrt(35) = 5.92, 4 -> 4.90, 2 -> 2.83, 0 -> 0.
        assert!(close(spin_only_moment_bohr(5), 5.92, 0.01), "d5, 5.92 mu_B");
        assert!(
            close(spin_only_moment_bohr(4), 4.90, 0.01),
            "4 unpaired, 4.90 mu_B"
        );
        assert!(
            close(spin_only_moment_bohr(2), 2.83, 0.01),
            "2 unpaired, 2.83 mu_B"
        );
        assert_eq!(
            spin_only_moment_bohr(0),
            Fixed::ZERO,
            "0 unpaired, diamagnetic"
        );
    }

    #[test]
    fn the_dispatch_gives_a_mott_insulators_moment_and_escalates_the_rest() {
        // THE DISPATCH: the Localized (Mott) monoxides carry the standard spin-only moments (NiO 2.83, CoO 3.87,
        // FeO 4.90, MnO 5.92), all derived from Z and the charge-balance charge, no reserved value. The itinerant
        // TiO escalates (the Stoner branch is a later slice), as does a non-correlated substance.
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        // NiO: Ni2+ d8, 2 unpaired, 2.83 mu_B.
        let nio = hund_local_moment(&comp(&[("Ni", 1), ("O", 1)]), &c, &t).expect("NiO moment");
        assert!(
            close(nio, 2.83, 0.01),
            "NiO (Ni2+ d8) ~ 2.83 mu_B, got {}",
            nio.to_f64_lossy()
        );
        // MnO: Mn2+ d5, 5 unpaired, 5.92 mu_B (the half-filled peak).
        let mno = hund_local_moment(&comp(&[("Mn", 1), ("O", 1)]), &c, &t).expect("MnO moment");
        assert!(
            close(mno, 5.92, 0.01),
            "MnO (Mn2+ d5) ~ 5.92 mu_B, got {}",
            mno.to_f64_lossy()
        );
        // FeO: Fe2+ d6, 4 unpaired, 4.90 mu_B.
        let feo = hund_local_moment(&comp(&[("Fe", 1), ("O", 1)]), &c, &t).expect("FeO moment");
        assert!(
            close(feo, 4.90, 0.01),
            "FeO (Fe2+ d6) ~ 4.90 mu_B, got {}",
            feo.to_f64_lossy()
        );
        // TiO is Itinerant: no local moment here, it escalates to the Stoner branch (a later slice).
        assert!(
            hund_local_moment(&comp(&[("Ti", 1), ("O", 1)]), &c, &t).is_none(),
            "an itinerant TiO escalates (the Stoner branch is a later slice)"
        );
        // A non-correlated substance (elemental silicon) escalates.
        assert!(
            hund_local_moment(&comp(&[("Si", 1)]), &c, &t).is_none(),
            "a non-correlated substance carries no Hund local moment here"
        );
    }

    // A test-only synthetic pairing scale (cm^-1). The real D = f(Racah B, C) is the one wire-time reserved input;
    // these tests exercise the comparison mechanism and the threshold ordering, not a live D.
    fn test_pairing_scale() -> Fixed {
        Fixed::from_int(15000)
    }

    fn resolution() -> Fixed {
        Fixed::from_int(500)
    }

    #[test]
    fn the_griffith_coefficients_are_the_delivered_octahedral_set() {
        // The cited Griffith octahedral spin-pairing coefficients (|Delta_D|, m): d4 (4/3, 1), d5 (3, 2), d6 (4, 2),
        // d7 (8/3, 1); d0-d3 and d8-d10 have no high-spin/low-spin choice.
        assert!(close(
            griffith_octahedral_coeffs(4).unwrap().0,
            4.0 / 3.0,
            1e-6
        ));
        assert_eq!(griffith_octahedral_coeffs(4).unwrap().1, 1);
        assert!(close(griffith_octahedral_coeffs(5).unwrap().0, 3.0, 1e-6));
        assert_eq!(griffith_octahedral_coeffs(5).unwrap().1, 2);
        assert!(close(griffith_octahedral_coeffs(6).unwrap().0, 4.0, 1e-6));
        assert_eq!(griffith_octahedral_coeffs(6).unwrap().1, 2);
        assert!(close(
            griffith_octahedral_coeffs(7).unwrap().0,
            8.0 / 3.0,
            1e-6
        ));
        assert_eq!(griffith_octahedral_coeffs(7).unwrap().1, 1);
        assert!(griffith_octahedral_coeffs(3).is_none(), "d3 has no choice");
        assert!(griffith_octahedral_coeffs(8).is_none(), "d8 has no choice");
    }

    #[test]
    fn low_spin_pairs_in_t2g_before_eg() {
        // Octahedral low-spin filling (t2g fills and pairs first): d4 -> 2 unpaired, d5 -> 1, d6 -> 0, d7 -> 1. Below
        // d4 it equals the high-spin count (d3 -> 3), and d8 -> 2 (t2g6 eg2), d9 -> 1.
        assert_eq!(low_spin_unpaired_count(4), Some(2));
        assert_eq!(low_spin_unpaired_count(5), Some(1));
        assert_eq!(low_spin_unpaired_count(6), Some(0));
        assert_eq!(low_spin_unpaired_count(7), Some(1));
        assert_eq!(
            low_spin_unpaired_count(3),
            Some(3),
            "d3 low-spin == high-spin"
        );
        assert_eq!(low_spin_unpaired_count(8), Some(2));
        assert_eq!(low_spin_unpaired_count(9), Some(1));
    }

    #[test]
    fn a_weak_monoxide_field_stays_high_spin_and_a_strong_field_goes_low_spin() {
        // The Griffith comparison E_LS - E_HS = |Delta_D|*D - m*Delta_o. For d6 with D = 15000 cm^-1: a monoxide-scale
        // field (Delta_o ~ 8000, cited from the crystal-field arc) gives shift = 4*15000 - 2*8000 = +44000 > 0, so
        // high-spin (matching the robustly high-spin 3d monoxides); a strong field (Delta_o = 35000) gives
        // 60000 - 70000 = -10000 < 0, so low-spin.
        let d = test_pairing_scale();
        let r = resolution();
        let monoxide = octahedral_spin_decision(6, Fixed::from_int(8000), d, r);
        assert_eq!(
            monoxide,
            OctahedralSpin::HighSpin,
            "d6 monoxide stays high-spin"
        );
        let strong = octahedral_spin_decision(6, Fixed::from_int(35000), d, r);
        assert_eq!(
            strong,
            OctahedralSpin::LowSpin,
            "a strong field forces low-spin"
        );
        // At the boundary (Delta_o = 30000, shift = 0) the comparison escalates rather than forcing a binary.
        let marginal = octahedral_spin_decision(6, Fixed::from_int(30000), d, r);
        assert_eq!(marginal, OctahedralSpin::Escalate, "the boundary escalates");
        // A no-choice d-count (d3) never forks.
        assert_eq!(
            octahedral_spin_decision(3, Fixed::from_int(35000), d, r),
            OctahedralSpin::NoChoice
        );
    }

    #[test]
    fn the_low_spin_threshold_rises_across_d4_to_d7() {
        // THE TREND GATE: the low-spin threshold Delta_o/D rises d4 (1.33) < d5 (1.5) < d6 (2.0) < d7 (2.67). At a
        // fixed field Delta_o = 25000 with D = 15000 (ratio 1.667), d4 and d5 are already low-spin (thresholds 1.33,
        // 1.5) while d6 and d7 remain high-spin (thresholds 2.0, 2.67). So d7 is the hardest to make low-spin.
        let d = test_pairing_scale();
        let r = resolution();
        let field = Fixed::from_int(25000);
        assert_eq!(
            octahedral_spin_decision(4, field, d, r),
            OctahedralSpin::LowSpin
        );
        assert_eq!(
            octahedral_spin_decision(5, field, d, r),
            OctahedralSpin::LowSpin
        );
        assert_eq!(
            octahedral_spin_decision(6, field, d, r),
            OctahedralSpin::HighSpin
        );
        assert_eq!(
            octahedral_spin_decision(7, field, d, r),
            OctahedralSpin::HighSpin
        );
    }

    #[test]
    fn the_corrected_moment_collapses_under_a_strong_field() {
        // The 10Dq correction to the moment: d6 (FeO-like) is 4.90 mu_B high-spin (weak monoxide field) and collapses
        // to 0 low-spin (strong field); d5 is 5.92 high-spin and 1.73 (one unpaired) low-spin; the marginal band
        // returns None (escalate); a no-choice d8 keeps its 2.83 regardless of field.
        let d = test_pairing_scale();
        let r = resolution();
        let d6_hs = octahedral_spin_moment(6, Fixed::from_int(8000), d, r).expect("d6 high-spin");
        assert!(
            close(d6_hs, 4.90, 0.01),
            "d6 weak field 4.90 mu_B, got {}",
            d6_hs.to_f64_lossy()
        );
        let d6_ls = octahedral_spin_moment(6, Fixed::from_int(35000), d, r).expect("d6 low-spin");
        assert_eq!(
            d6_ls,
            Fixed::ZERO,
            "d6 strong field collapses to 0 (diamagnetic low-spin)"
        );
        let d5_ls = octahedral_spin_moment(5, Fixed::from_int(35000), d, r).expect("d5 low-spin");
        assert!(
            close(d5_ls, 1.73, 0.01),
            "d5 low-spin 1.73 mu_B (one unpaired), got {}",
            d5_ls.to_f64_lossy()
        );
        assert!(
            octahedral_spin_moment(6, Fixed::from_int(30000), d, r).is_none(),
            "the marginal band escalates (None)"
        );
        let d8 = octahedral_spin_moment(8, Fixed::from_int(35000), d, r).expect("d8 no-choice");
        assert!(
            close(d8, 2.83, 0.01),
            "d8 keeps 2.83 mu_B regardless of field"
        );
    }
}
