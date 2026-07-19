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

//! THE THERMAL-CONDUCTIVITY LADDER: one quantity, a measured rung and an estimated rung, one entry point.
//!
//! WHY A LADDER AND NOT A CHOICE. Two models of `k` exist and they are NOT competitors, they are RUNGS of the
//! lookup order this engine already runs for every other quantity: measured before estimator, dispatched per
//! material on ANCHOR AVAILABILITY. Nobody at a call site ever picks a physical model.
//!
//! - TOP RUNG, [`hofmeister_lattice_conductivity`]: a MEASURED `kappa_298` anchor carrying derived temperature
//!   and pressure dependence off banked Grueneisen, bulk modulus, and expansivity. Highest accuracy, available
//!   only where a mineral HAS a measured anchor.
//! - ESTIMATOR RUNG, [`crate::properties::lattice_thermal_conductivity_w_per_m_k`] (Slack): no anchor needed,
//!   evaluable for anything with banked columns, carrying the band its own docstring declares (roughly 3x
//!   symmetric on simple cells, ONE-SIDED on complex cells, where it is an intrinsic UPPER BOUND that can sit
//!   several-fold above truth; rutile is its own convicting exhibit at ~43 against a measured ~9).
//!
//! WHERE NO MEASUREMENT EXISTS, Slack's magnitude serves as the `[E]`-grade anchor with its one-sided
//! upper-bound band declared, and Hofmeister's class-keyed exponent governs the temperature shape ON BOTH RUNGS,
//! because the exponent split IS the same physics as the validity split: Slack's `a = 1` matches ice at `612/T`
//! and nearly matches MgO at `0.9`, while complex silicates take `0.33`.
//!
//! THE DOCTRINE THIS INSTANTIATES (standing, and it will recur): SAME-RUNG DUPLICATES are the
//! redundant-parameter defect at MODEL level and stay forbidden. DIFFERENT-RUNG models with a DECLARED ORDER are
//! the ladder. And the ladder carries a free integrity mechanism: WHEREVER BOTH RUNGS CAN EVALUATE, THE
//! DISAGREEMENT IS COMPUTED AND LOGGED AS A DIAGNOSTIC, NEVER SILENTLY RESOLVED
//! ([`rung_disagreement_ratio`]). MgO-class minerals are permanent OVERLAP SENTINELS, two models compared BY
//! CONSTRUCTION on every run, which turns "never compared" from a risk into an impossibility. That is the whole
//! point: the defect this module was built to avoid was two models answering one question in different call
//! sites, never compared, disagreeing several-fold, which is the k/kappa finding in a bigger coat.
//!
//! WHY THIS PAYS FOR THE ALIEN. The geotherm's minerals have measured anchors, so the top rung serves the front
//! lane. But exotic condensates (the carbide slice) will have NO `kappa_298` rows at all, and SLACK'S RUNG IS
//! THE ONLY LEGAL CONDUCTIVITY PATH AN ALIEN PHASE WILL EVER HAVE: banked columns in, banded estimate out,
//! upper-bound honesty attached. Hofmeister bolted BESIDE Slack would have served Earth minerals and stranded
//! every alien one.
//!
//! HOW THIS MODULE CAME TO EXIST, recorded because the rule it completed binds every future ruling: a ruling
//! ordered Hofmeister built as new machinery. A check for an existing conductivity found Slack already banked,
//! from the same estimator roster the ruling channel had itself written down. So the premise line COMPLETES
//! SYMMETRICALLY: existence claims and ABSENCE claims are one class. A ruling that says "wire X" verifies
//! PRESENCE; a ruling that says "BUILD X" verifies ABSENCE. One line either way. Checking for the thing before
//! building a second one is the named standing practice, and the first time it ran in the build direction it
//! prevented a ~5x silent disagreement from shipping.

use crate::properties::lattice_thermal_conductivity_w_per_m_k;
use civsim_core::Fixed;

const ZERO: Fixed = Fixed::ZERO;

/// The reference temperature Hofmeister's lattice form is anchored at: 298 K, the standard state the measured
/// `kappa_298` rows are reported against. It is the SOURCE'S OWN reference, not a chosen scale.
pub fn hofmeister_reference_temperature_k() -> Fixed {
    Fixed::from_int(298)
}

/// The class-keyed temperature exponent `a` in Hofmeister's lattice form `kappa ~ (298/T)^a`, keyed on
/// ATOMS PER PRIMITIVE CELL, the class variable already banked and already in Slack's own signature.
///
/// THE PHYSICS OF THE SPLIT: a simple lattice's phonons scatter through few channels and the conductivity falls
/// as roughly `1/T` (the Umklapp limit). A complex cell has many optical branches that carry little heat but
/// scatter plenty, so the decline is far shallower. That is the SAME physics as Slack's validity split: Slack's
/// single-scattering form is built for the simple case, which is why it lands within its band on simple cells
/// and OVERSTATES complex ones.
///
/// THE CALIBRATION SET, and its honest limit. The exponents are pinned by three independent measurements: ice at
/// `612/T` (`a ~ 1`), MgO at `a = 0.9`, and complex silicates at `a = 0.33` (Hofmeister). The CELL-COUNT
/// boundary is calibrated on the cited set Slack's own docstring convicts itself with: diamond, NaCl, and MgO
/// (all `n = 2`) land inside its band, while rutile (`n = 6`) is overstated ~5x.
///
/// SO THE BOUNDARY IS UNDERDETERMINED IN `2 < n < 6`, AND THIS FUNCTION REFUSES THERE rather than picking a
/// number the cited set does not support. `None` is the honest answer for a cell the calibration cannot place,
/// and a caller that gets `None` must supply a measured exponent or escalate. Picking a boundary inside that gap
/// would be authoring the very scalar the shape-first method exists to avoid, and it would author it invisibly,
/// inside a classifier, which is the silent-parameter class exactly.
pub fn lattice_exponent_for_cell(atoms_per_primitive_cell: i32) -> Option<Fixed> {
    if atoms_per_primitive_cell < 1 {
        return None;
    }
    if atoms_per_primitive_cell <= 2 {
        // The simple-lattice limit: the Umklapp `1/T`. Ice (612/T) and MgO (0.9) both sit here.
        return Some(Fixed::from_ratio(95, 100));
    }
    if atoms_per_primitive_cell >= 6 {
        // The complex-cell class: Hofmeister's silicate exponent.
        return Some(Fixed::from_ratio(33, 100));
    }
    // 2 < n < 6: the cited set places nothing here. Refuse rather than author a boundary.
    None
}

/// HOFMEISTER'S LATTICE CONDUCTIVITY at a temperature (W/(m*K)), the TOP RUNG:
///
/// `kappa_lat(T) = kappa_298 * (298/T)^a * exp[-(4*gamma + 1/3) * integral(alpha d theta)]`
///
/// The measured `kappa_298` sets the MAGNITUDE; everything that moves it with temperature is DERIVED. The
/// power-law factor is the phonon-scattering decline with its class-keyed exponent
/// ([`lattice_exponent_for_cell`]); the exponential is the thermal-expansion correction, where `gamma` is the
/// banked Grueneisen parameter and the integral is expansivity accumulated from the reference temperature to
/// `T`. The `4` and the `1/3` are the form's own coefficients, not knobs.
///
/// THE EXPANSIVITY INTEGRAL is the caller's, because only the caller knows whether its expansivity is constant
/// over the range: `integral(alpha d theta)` from 298 to `T`. For a constant `alpha` that is `alpha * (T - 298)`,
/// which is the common case for a lid-temperature span; a caller with a temperature-dependent `alpha` integrates
/// its own and passes the result. Passing a bare `alpha` here would author the constancy assumption invisibly.
///
/// `None` on a non-positive temperature or anchor, or a fixed-point overflow. Deterministic fixed-point.
// @derives: lattice thermal conductivity k(T,P) <- a measured kappa_298 anchor + banked Grueneisen, bulk modulus and expansivity (measured rung)
pub fn hofmeister_lattice_conductivity(
    kappa_298: Fixed,
    exponent_a: Fixed,
    gruneisen: Fixed,
    expansivity_integral: Fixed,
    temperature: Fixed,
) -> Option<Fixed> {
    if temperature <= ZERO || kappa_298 <= ZERO {
        return None;
    }
    // The power-law decline (298/T)^a.
    let ratio = hofmeister_reference_temperature_k().checked_div(temperature)?;
    if ratio <= ZERO {
        return None;
    }
    let decline = ratio.powf(exponent_a);
    // The expansion correction exp[-(4 gamma + 1/3) * integral(alpha d theta)].
    let four_gamma = gruneisen.checked_mul(Fixed::from_int(4))?;
    let coefficient = four_gamma.checked_add(Fixed::ONE.checked_div(Fixed::from_int(3))?)?;
    let exponent = ZERO.checked_sub(coefficient.checked_mul(expansivity_integral)?)?;
    let correction = exponent.exp();
    kappa_298.checked_mul(decline)?.checked_mul(correction)
}

/// THE RADIATIVE conductivity (W/(m*K)) of an Fe2+-bearing phase, Hofmeister's polynomial:
///
/// `kappa_rad(T) = 0.0175 - 1.037e-4 T + 2.245e-7 T^2 - 3.407e-11 T^3`
///
/// Photons carry heat through a semi-transparent solid, and the Fe2+ absorption bands set how far they travel.
/// It matters only at the HOT end: the term is small and rises steeply with temperature, so it is a deep-mantle
/// quantity, and a caller adds it to the lattice term only for a phase that carries Fe2+.
///
/// THE DECLARED DISPUTE (type-II, and it ships with the row): modern high-pressure experiments find radiative
/// transport contributing around 40 percent of olivine's conductivity at depth, against this small,
/// pressure-independent polynomial, and the field itself calls the radiative role controversial. THE BLAST
/// RADIUS IS SCOPED PLAINLY rather than waved away: radiative transport GROWS WITH TEMPERATURE and the elastic
/// lid is the COLD END of the column, so `T_e` is only weakly exposed to this band, and the disagreement lands
/// mostly on deep-mantle and slab-thermal consumers. A consumer at depth inherits the band; a lid consumer does
/// not, and neither should pretend the other's exposure.
///
/// Returns zero below the temperature where the polynomial goes non-positive (its fit does not extend to the
/// cold end, where the physical answer is that radiative transport is negligible anyway), so a cold caller reads
/// the honest zero rather than a negative conductivity.
// @derives: the radiative conductivity silicates gain at high T <- temperature
pub fn radiative_conductivity_w_per_m_k(temperature: Fixed) -> Fixed {
    if temperature <= ZERO {
        return ZERO;
    }
    let t = temperature;
    let c0 = Fixed::from_ratio(175, 10_000);
    let c1 = Fixed::from_ratio(1_037, 10_000_000);
    let c2 = Fixed::from_ratio(2_245, 10_000_000_000);
    // The cubic coefficient 3.407e-11 is below the Q32.32 resolution (~2.3e-10), so it is applied to the
    // SCALED temperature (T/1000)^3 with the paired 1e9 folded in: 3.407e-11 * T^3 = 3.407e-2 * (T/1000)^3.
    let c3_scaled = Fixed::from_ratio(3_407, 100_000);
    let t2 = match t.checked_mul(t) {
        Some(v) => v,
        None => return ZERO,
    };
    let t_k = match t.checked_div(Fixed::from_int(1000)) {
        Some(v) => v,
        None => return ZERO,
    };
    let t_k3 = match t_k.checked_mul(t_k).and_then(|v| v.checked_mul(t_k)) {
        Some(v) => v,
        None => return ZERO,
    };
    let linear = match c1.checked_mul(t) {
        Some(v) => v,
        None => return ZERO,
    };
    let quad = match c2.checked_mul(t2) {
        Some(v) => v,
        None => return ZERO,
    };
    let cubic = match c3_scaled.checked_mul(t_k3) {
        Some(v) => v,
        None => return ZERO,
    };
    let total = c0 - linear + quad - cubic;
    if total <= ZERO {
        ZERO
    } else {
        total
    }
}

/// THE OVERLAP SENTINEL: the ratio between the two rungs where BOTH can evaluate, `estimator / measured`.
///
/// This is the ladder's integrity mechanism, and it is the reason the ladder is safer than either model alone.
/// Wherever a mineral has BOTH a measured anchor and banked columns, the two rungs are computed and their
/// DISAGREEMENT IS REPORTED, never silently resolved. A ratio near one says the estimator is honest for that
/// class; a ratio far from one is a finding, and on a complex cell it should be ABOVE one (Slack's overstatement
/// is one-sided, the intrinsic upper bound its docstring declares).
///
/// MgO-class minerals are PERMANENT SENTINELS: they sit in both rungs' domains by construction, so every run
/// compares the two models against each other. That converts "two models, never compared" from a standing risk
/// into an impossibility, which is exactly the failure this module was built to foreclose.
///
/// `None` when either rung fails to evaluate (no comparison exists, so none is reported). Diagnostic only: no
/// caller resolves a conductivity from this, it exists to be watched.
pub fn rung_disagreement_ratio(measured_rung: Fixed, estimator_rung: Fixed) -> Option<Fixed> {
    if measured_rung <= ZERO || estimator_rung <= ZERO {
        return None;
    }
    estimator_rung.checked_div(measured_rung)
}

/// The ESTIMATOR RUNG's anchor: Slack's derived magnitude at the reference temperature, for a phase with NO
/// measured `kappa_298`. This is where option "Slack anchors Hofmeister" survives, and ONLY here: when no
/// measurement exists, Slack supplies the magnitude at 298 K and Hofmeister's class-keyed exponent carries it
/// through temperature.
///
/// THE BAND IS ONE-SIDED AND IT RIDES ALONG: on a complex cell this anchor is an INTRINSIC UPPER BOUND that can
/// sit several-fold above truth (rutile, ~43 against a measured ~9). A consumer of this rung inherits that
/// one-sidedness and must not report it as a symmetric uncertainty. On a simple cell the band is Slack's
/// declared ~3x, roughly symmetric.
///
/// `None` when Slack cannot evaluate. Deterministic fixed-point.
pub fn estimator_anchor_298(
    gruneisen: Fixed,
    mean_atomic_mass_amu: Fixed,
    debye_temperature_k: Fixed,
    atomic_volume_angstrom3: Fixed,
    atoms_per_primitive_cell: i32,
) -> Option<Fixed> {
    let k = lattice_thermal_conductivity_w_per_m_k(
        gruneisen,
        mean_atomic_mass_amu,
        debye_temperature_k,
        atomic_volume_angstrom3,
        atoms_per_primitive_cell,
        hofmeister_reference_temperature_k(),
    );
    if k <= ZERO {
        None
    } else {
        Some(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exponent_refuses_the_boundary_the_cited_set_does_not_place() {
        // The calibration set pins n <= 2 (diamond, NaCl, MgO all land inside Slack's band) and n = 6 (rutile,
        // overstated ~5x). It says NOTHING about 2 < n < 6, so the classifier REFUSES there. Picking a boundary
        // in that gap would author a scalar invisibly, inside a classifier, which is the silent-parameter class.
        assert!(
            lattice_exponent_for_cell(2).is_some(),
            "n = 2 is the calibrated simple class"
        );
        assert!(
            lattice_exponent_for_cell(6).is_some(),
            "n = 6 is the calibrated complex class"
        );
        for n in 3..=5 {
            assert!(
                lattice_exponent_for_cell(n).is_none(),
                "n = {n} sits in the gap the cited set does not place; the classifier must refuse, not guess"
            );
        }
        assert!(
            lattice_exponent_for_cell(0).is_none(),
            "a cell with no atoms is not a lattice"
        );
    }

    #[test]
    fn the_simple_class_declines_far_more_steeply_than_the_complex_one() {
        // The whole point of the class-keyed exponent: at the same temperature rise, a simple lattice (~1/T)
        // loses far more of its conductivity than a complex silicate (0.33). A single exponent for both, which
        // is what "1/T for everything" would have shipped, gets one of these two badly wrong.
        let simple = lattice_exponent_for_cell(2).unwrap();
        let complex = lattice_exponent_for_cell(6).unwrap();
        assert!(
            simple > complex,
            "the simple lattice's decline is the steeper one"
        );
        let hot = Fixed::from_int(1200);
        let k_simple =
            hofmeister_lattice_conductivity(Fixed::from_int(10), simple, ZERO, ZERO, hot).unwrap();
        let k_complex =
            hofmeister_lattice_conductivity(Fixed::from_int(10), complex, ZERO, ZERO, hot).unwrap();
        assert!(
            k_simple < k_complex,
            "at 1200 K the same anchor retains more conductivity under the shallow silicate exponent: simple={k_simple:?} complex={k_complex:?}"
        );
    }

    #[test]
    fn the_anchor_is_read_exactly_at_the_reference_temperature() {
        // At T = 298 the power law is unity and the expansion integral is zero, so the form must return the
        // MEASURED anchor untouched. If it does not, the top rung is not measured-magnitude at all.
        let anchor = Fixed::from_int(5);
        let k = hofmeister_lattice_conductivity(
            anchor,
            Fixed::from_ratio(33, 100),
            Fixed::from_ratio(15, 10),
            ZERO,
            hofmeister_reference_temperature_k(),
        )
        .unwrap();
        let err = (k - anchor).abs();
        assert!(
            err < Fixed::from_ratio(1, 100),
            "the anchor reads through at 298 K, got {k:?}"
        );
    }

    #[test]
    fn the_expansion_correction_only_ever_reduces_conductivity() {
        // exp[-(4 gamma + 1/3) * integral] with a positive Grueneisen and a positive expansivity integral is
        // strictly below one: thermal expansion softens the lattice and impedes phonon transport. A positive
        // correction would be the sign error this test exists to catch.
        let hot = Fixed::from_int(1000);
        let a = Fixed::from_ratio(33, 100);
        let no_expansion = hofmeister_lattice_conductivity(
            Fixed::from_int(5),
            a,
            Fixed::from_ratio(15, 10),
            ZERO,
            hot,
        )
        .unwrap();
        let expanded = hofmeister_lattice_conductivity(
            Fixed::from_int(5),
            a,
            Fixed::from_ratio(15, 10),
            Fixed::from_ratio(3, 100),
            hot,
        )
        .unwrap();
        assert!(
            expanded < no_expansion,
            "expansion impedes transport: {expanded:?} !< {no_expansion:?}"
        );
    }

    #[test]
    fn the_radiative_term_is_a_hot_end_quantity_and_stays_non_negative() {
        // It is small and rises steeply with temperature, which is exactly why T_e (the COLD end of the column)
        // is only weakly exposed to its declared dispute band while deep-mantle consumers are not.
        let cold = radiative_conductivity_w_per_m_k(Fixed::from_int(300));
        let hot = radiative_conductivity_w_per_m_k(Fixed::from_int(1800));
        assert!(
            hot > cold,
            "radiative transport grows with temperature: {hot:?} !> {cold:?}"
        );
        assert!(
            cold >= ZERO && hot >= ZERO,
            "a conductivity is never negative"
        );
        assert_eq!(
            radiative_conductivity_w_per_m_k(ZERO),
            ZERO,
            "no photons at zero temperature"
        );
    }

    #[test]
    fn the_overlap_sentinel_reports_the_disagreement_rather_than_resolving_it() {
        // The ladder's integrity mechanism. Where both rungs evaluate, the ratio is REPORTED. It resolves
        // nothing: no caller reads a conductivity from this, it exists to be watched, so that "two models
        // answering one question, never compared" is impossible rather than merely discouraged.
        let measured = Fixed::from_int(9);
        let estimated = Fixed::from_int(43); // rutile: Slack's own convicting exhibit
        let ratio = rung_disagreement_ratio(measured, estimated).unwrap();
        assert!(
            ratio > Fixed::from_int(4),
            "the sentinel surfaces Slack's complex-cell overstatement as the several-fold ratio it is, got {ratio:?}"
        );
        // One-sided by construction on a complex cell: the estimator sits ABOVE, never below.
        assert!(
            ratio > Fixed::ONE,
            "Slack's complex-cell error is an upper bound, so the ratio exceeds one"
        );
        assert!(
            rung_disagreement_ratio(ZERO, estimated).is_none(),
            "no comparison exists without both rungs"
        );
    }
}
