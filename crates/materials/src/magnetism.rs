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
//! Localized class.
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
//! HONEST LIMITS (stated at the site): the spin-only moment omits the orbital contribution (which lifts the
//! observed moment above spin-only for the early and late 3d ions) and assumes HIGH-SPIN free-ion filling; a strong
//! crystal field forces low-spin and changes the unpaired count, the `10Dq` correction of section 11.5 (b), a named
//! follow-on. The d-count derivation is scoped to the 3d series (`Z` in `21..=30`), the classifier's own scope; a
//! 4d/5d/4f centre needs its own principal-shell range, a flagged follow-on. Byte-neutral: `civsim-materials` is a
//! leaf.

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
            DStateRadii::standard().expect("d-state radii"),
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
}
