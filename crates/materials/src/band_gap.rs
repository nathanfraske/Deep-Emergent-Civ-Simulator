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

//! Stage 6, the band-gap tier (`docs/working/STAGE6_ELECTRONIC_STRUCTURE_DESIGN.md`, section 10): the
//! fabrication-free core the tier's design surface ruled buildable now (independent of the Harrison-rung fork held
//! for the gate). Slice 1 is the log-space thermal carrier activation; slice 2 is the emergent conduction
//! classification with the `U/W` preflight over the banked correlation classifier; slice 3b reads the banked
//! band-gap column (the physics `[M]`/compute-once floor data) and routes a substance by its provenance.
//!
//! A semiconductor's intrinsic carrier density is `n_i = N_eff * exp(-E_gap / 2kT)`, thermally activated across
//! the gap. The activation factor `exp(-E_gap / 2kT)` is the RANGE-CENSUS flag of the electronic sub-arc: for a
//! wide-gap insulator (diamond's `5.47 eV`) at world temperature the factor is `exp(-106) ~ 1e-46`, far below the
//! Q32.32 floor (`~2.3e-10`), so the bare factor underflows to zero and loses all ordering. So the quantity is
//! carried in LOG SPACE, on the same discipline the creep deformation-mechanism rates use: this slice returns the
//! natural-log activation exponent `-E_gap / 2kT` (non-positive, always representable), and a consumer exponentiates
//! only when the value is in range, comparing insulators' activations by their logs otherwise.
//!
//! WHAT IS RESERVED HERE: nothing. The gap `E_gap` is caller-supplied (a measured `[M]` datum at the top rung, a
//! Harrison estimate at the middle rung once the gate rules that fork, a compute-once eigenvalue at the bottom),
//! and the temperature is the world's. The one constant, the Boltzmann constant in the working units (eV per
//! kelvin), reassembles as `k_B[J/K] / e[C]`, a ratio of two exact SI fundamental constants rather than a folded
//! dimensional decimal (the dimensionless-constant law), so the eV and the kelvin cancel and the activation
//! exponent is dimensionless by construction.
//!
//! SLICE 2 adds the EMERGENT conduction classification ([`ConductionClass`]) keyed on the gap, with the `U/W`
//! PREFLIGHT (section 10.2, redirect 2) run over the banked [`crate::correlation::CorrelationClassifier`] on the
//! reduced-order route so a Mott insulator is never called a metal. The metal / non-metal boundary is the gap SIGN
//! (a physical line, no threshold); the semiconductor-versus-insulator distinction is the CONTINUOUS carrier
//! activation rather than an authored eV boundary; and the classification is never shipped in the preflight-free
//! form redirect 2 warned reintroduces the Mott failure.
//!
//! THE EXPONENT RIDER (owner ruling): the carrier-density exponent `exp(-E_gap / 2kT)` is exponentially sensitive
//! to the gap, and the banked escalation law forbids estimator grade in an exponent (a factor-grade `+/-0.4 eV`
//! miss on a `1 eV` gap is `~2e3` in carrier density). So [`ln_thermal_carrier_activation`] guards on a
//! [`GapGrade`]: an estimator gap is BARRED from the exponent (escalates), admitted only to classification,
//! ranking, and optical cast (the sign and linear consumers). A measured or compute-once gap is authoritative and
//! admitted. So an estimator-grade non-metal classifies but carries no carrier density ([`ConductionClass::NonMetal`]
//! with `ln_activation: None`).
//!
//! Byte-neutral: `civsim-materials` is a leaf, not linked into the run_world binary.

use civsim_core::Fixed;
use civsim_physics::band_gap::{BandGapColumn, GapProvenance};

use crate::correlation::{CorrelationClass, CorrelationClassifier};

const ZERO: Fixed = Fixed::ZERO;

/// The thermal-activation fold `2 * k_B` in eV/K (`~1.7234667e-4`), mapping `E_gap[eV] / (fold * T[K])` to the
/// dimensionless activation exponent. ASSEMBLED from the exact SI mantissas of the Boltzmann constant and the
/// elementary charge (the dimensionless-constant law, no folded dimensional decimal): `2 * k_B[eV/K] = 2 *
/// k_B[J/K] / e[C] = (2 * 1.380649 / 1.602176634) * 1e-4`, since `k_B` carries `10^-23` and `e` carries `10^-19`,
/// netting `10^-4`. The eV-per-kelvin convention IS `k_B / e`, so the fold reassembles from two fundamental
/// constants and the eV and the kelvin cancel: the activation exponent `E_gap / (fold * T)` is dimensionless.
fn two_kb_ev_per_k() -> Fixed {
    // 2 * k_B[J/K] mantissa (2 * 1.380649) and e[C] mantissa (1.602176634); the collapsed 10^-4 rides as the
    // /10000 below (k_B's 10^-23 over e's 10^-19).
    let two_kb_mantissa = Fixed::from_ratio(2 * 1_380_649, 1_000_000);
    let e_mantissa = Fixed::from_ratio(1_602_176_634, 1_000_000_000);
    let ratio = match two_kb_mantissa.checked_div(e_mantissa) {
        Some(v) => v,
        None => return ZERO,
    };
    ratio.checked_div(Fixed::from_int(10_000)).unwrap_or(ZERO)
}

/// The grade of a band gap on the provenance ladder, the key the EXPONENT RIDER guards on: a measured `[M]` value
/// or a compute-once hybrid/GW eigenvalue is AUTHORITATIVE (admitted to the carrier-density exponent), a
/// reduced-order Harrison estimate is ESTIMATOR grade (barred from the exponent, admitted only to classification,
/// ranking, and optical cast). The coarse routing grade, mapped from the column's provenance and the runtime
/// estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapGrade {
    /// A measured `[M]` gap (the top rung): authoritative, admitted to the exponent.
    Measured,
    /// A compute-once hybrid/GW eigenvalue (the bottom rung): authoritative, admitted to the exponent.
    ComputeOnce,
    /// A reduced-order estimate (the Harrison middle rung): barred from the carrier-density exponent, admitted to
    /// classification, ranking, and optical cast only.
    Estimator,
}

impl GapGrade {
    /// Whether this grade is admitted to the carrier-density exponent (the EXPONENT RIDER): only an authoritative
    /// gap (measured or compute-once). An estimator gap is barred.
    fn admits_exponent(self) -> bool {
        matches!(self, GapGrade::Measured | GapGrade::ComputeOnce)
    }
}

/// The natural log of the thermal carrier activation factor, `ln(exp(-E_gap / 2kT)) = -E_gap / (2 * k_B * T)`, the
/// LOG-SPACE form of the semiconductor intrinsic-carrier suppression (the range-census discipline: the bare factor
/// underflows Q32.32 for a wide gap). The returned exponent is non-positive and always representable; a consumer
/// exponentiates it only when the value is in the `exp` window, and orders insulators by the log otherwise.
///
/// THE EXPONENT RIDER (owner ruling): the `grade` guards the exponent. Estimator grade is BARRED and escalates
/// (`None`), because the exponent is exponentially sensitive to the gap and estimator grade is forbidden in
/// exponents: a factor-grade `+/-0.4 eV` miss on a `1 eV` gap at `300 K` is `exp(0.4 / 0.0517) ~ 2e3` in carrier
/// density, three orders of magnitude of fabrication wearing a derived pedigree. An estimator gap feeds
/// classification, ranking, and optical cast (the threshold and linear consumers), never the carrier-density
/// exponent, which escalates to `[M]` or compute-once.
///
/// Reserves no value: `E_gap` (eV) and the temperature (K) are caller-supplied, and the `2 * k_B` fold reassembles
/// from `k_B` and `e`. `None` (escalate) for an estimator-grade gap (barred from the exponent), a negative gap (a
/// band overlap is a metal, classified upstream), or a non-positive temperature (no thermal population).
pub fn ln_thermal_carrier_activation(
    e_gap_ev: Fixed,
    temperature_k: Fixed,
    grade: GapGrade,
) -> Option<Fixed> {
    // THE EXPONENT RIDER: an estimator-grade gap is barred from the exponent and escalates to [M]/compute-once.
    if !grade.admits_exponent() {
        return None;
    }
    if e_gap_ev < ZERO || temperature_k <= ZERO {
        return None;
    }
    // 2 * k_B * T in eV (the thermal energy scale the gap is measured against).
    let thermal_scale = two_kb_ev_per_k().checked_mul(temperature_k)?;
    if thermal_scale <= ZERO {
        return None;
    }
    // -E_gap / (2 k_B T): the log-space activation exponent, non-positive, representable even where exp underflows.
    let ratio = e_gap_ev.checked_div(thermal_scale)?;
    ZERO.checked_sub(ratio)
}

/// The conduction class of a substance, an EMERGENT readout of its band gap (never an authored material lookup).
/// The metal / non-metal boundary is the gap SIGN, a physical line; the semiconductor-versus-insulator distinction
/// is the CONTINUOUS carrier activation carried in [`ConductionClass::NonMetal`], never a discrete authored eV
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductionClass {
    /// A metal: the gap is at or below zero (the bands overlap), so carriers are degenerate and there is no
    /// thermal-activation suppression.
    Metal,
    /// A non-metal (a gap above zero): its intrinsic carriers are thermally activated across the gap. The
    /// semiconductor-versus-insulator distinction is the CONTINUOUS readout of `ln_activation` against the world
    /// temperature (a more negative exponent is more insulating), never a discrete authored boundary.
    NonMetal {
        /// The natural-log thermal carrier activation `-E_gap / 2kT` (non-positive), the census-safe form, or
        /// `None` when the gap is ESTIMATOR grade (the exponent rider bars an estimator gap from the exponent):
        /// the non-metal CLASSIFICATION stands, but the carrier density escalates to `[M]` or compute-once.
        ln_activation: Option<Fixed>,
    },
    /// A correlated (Mott) insulator, sited by the `U/W` preflight BEFORE any reduced-order band model: it is an
    /// insulator, and a reduced-order model would wrongly call it a metal, so its gap must come from a measured
    /// value or a compute-once eigenvalue. The Mott guard, kept closed.
    CorrelatedInsulator,
    /// Escalate: the `U/W` window (estimators forbidden), a substance the correlation classifier cannot validate on
    /// a reduced-order route, or a gap that could not be scored. Route to a measured value or compute-once.
    Escalate,
}

/// The gap-SIGN sort (the emergent metal / non-metal boundary, no authored threshold): a gap at or below zero is a
/// metal, a positive gap is a non-metal whose carriers are thermally activated. The `grade` guards the exponent
/// (the rider): a non-metal's `ln_activation` is `Some` for an authoritative gap and `None` for an estimator gap
/// (the classification stands, the exponent escalates). The metal / non-metal SIGN itself is grade-blind, since the
/// classification, ranking, and optical cast are the linear consumers an estimator gap is admitted to.
fn conduction_class_from_gap(
    gap_ev: Fixed,
    temperature_k: Fixed,
    grade: GapGrade,
) -> ConductionClass {
    if gap_ev <= ZERO {
        return ConductionClass::Metal;
    }
    ConductionClass::NonMetal {
        ln_activation: ln_thermal_carrier_activation(gap_ev, temperature_k, grade),
    }
}

/// The conduction class from a MEASURED `[M]` band gap: the gap-sign sort directly, no `U/W` preflight. A measured
/// value is authoritative (it already encodes the correlation gap, a Mott insulator's charge-transfer gap
/// included), so it outranks the reduced-order `U/W` classifier on the provenance ladder, the preflight is skipped,
/// and its gap is admitted to the carrier-density exponent. Reserves no value: the gap is caller-supplied `[M]`
/// data, the temperature the world's.
pub fn conduction_class_measured(gap_ev: Fixed, temperature_k: Fixed) -> ConductionClass {
    conduction_class_from_gap(gap_ev, temperature_k, GapGrade::Measured)
}

/// The conduction class from a REDUCED-ORDER or COMPUTED (non-measured) band gap, with the `U/W` PREFLIGHT
/// (section 10.2, redirect 2) run FIRST over the banked correlation classifier, so a reduced-order band model can
/// never call a Mott insulator a metal. The preflight sites the correlation regime before the gap sort:
/// - Localized: a Mott insulator ([`ConductionClass::CorrelatedInsulator`]), the guard kept closed, regardless of
///   what the reduced gap says.
/// - Itinerant: a validated itinerant material, proceed to the gap-sign sort with the supplied gap.
/// - Window or OutOfScope: escalate (estimators forbidden in the window; a substance the classifier cannot
///   validate on a reduced-order route is not scored). See the HONEST LIMIT below.
///
/// Reserves no value. HONEST LIMIT (a flagged seam, surfaced for the Harrison-rung ruling): the classifier's
/// OutOfScope collapses two cases the preflight would treat differently once a reduced-order estimator exists, a
/// non-correlated material (an sp semiconductor, which SHOULD proceed to the gap sort) and a correlated centre
/// beyond the classifier's seeded 3d rock-salt scope (a 4d/5d/4f Mott insulator, which SHOULD escalate). This
/// slice takes the conservative-safe choice (escalate both), correct now because the reduced-order estimator (the
/// held Harrison rung) has no live consumer; distinguishing them needs a per-substance "is this a correlated-oxide
/// candidate" check, a follow-on tied to the Harrison-rung ruling.
pub fn conduction_class_estimated(
    composition: &[(String, u32)],
    gap_ev: Fixed,
    classifier: &CorrelationClassifier,
    temperature_k: Fixed,
) -> ConductionClass {
    match classifier.classify(composition) {
        CorrelationClass::Localized => ConductionClass::CorrelatedInsulator,
        CorrelationClass::Window | CorrelationClass::OutOfScope => ConductionClass::Escalate,
        // Itinerant: the gap-sign sort at ESTIMATOR grade. The metal / non-metal classification stands, but the
        // exponent rider bars the estimator gap from the carrier density, so a non-metal here carries
        // `ln_activation: None` (escalate to [M]/compute-once for the carrier density).
        CorrelationClass::Itinerant => {
            conduction_class_from_gap(gap_ev, temperature_k, GapGrade::Estimator)
        }
    }
}

/// The conduction class from the banked band-gap column (the tier's top and bottom rungs), reading a substance's
/// gap and its provenance and routing accordingly. Every row in the column is AUTHORITATIVE, a measured `[M]` gap
/// or a compute-once hybrid/GW eigenvalue, and each encodes the correlation gap (a Mott insulator's charge-transfer
/// gap included), so the gap-sign sort runs directly with NO `U/W` preflight; the preflight is only for the
/// reduced-order Harrison estimator (the held middle rung, which is not a column). A substance absent from the
/// column has no banked gap: the reduced-order route would sit there, and until it lands the tier escalates rather
/// than guess. Reserves no value: the gap is the column's cited data, the temperature the world's.
pub fn conduction_class_from_column(
    column: &BandGapColumn,
    composition: &[(String, u32)],
    temperature_k: Fixed,
) -> ConductionClass {
    match column.gap(composition) {
        // Both column provenances are authoritative and admitted to the exponent; map to the routing grade so the
        // carrier density is computed (never barred). A measured and a compute-once gap route identically here.
        Some(band_gap) => {
            let grade = match band_gap.provenance {
                GapProvenance::Measured => GapGrade::Measured,
                GapProvenance::ComputeOnce { .. } => GapGrade::ComputeOnce,
            };
            conduction_class_from_gap(band_gap.gap_ev, temperature_k, grade)
        }
        None => ConductionClass::Escalate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_physics::d_state_radius::DStateRadii;
    use civsim_physics::ionic_radii::IonicRadii;
    use civsim_physics::ionization_ladder::IonizationLadder;
    use civsim_physics::mit_reference::MitReference;
    use civsim_physics::periodic::PeriodicTable;

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
    fn the_fold_reassembles_from_the_boltzmann_and_charge_constants() {
        // THE DIMENSIONLESS-CONSTANT LAW: the 2*k_B[eV/K] fold reassembles as 2*k_B[J/K]/e[C] from the exact SI
        // mantissas, so the eV-per-kelvin convention is a ratio of two fundamental constants, not a folded
        // dimensional decimal. 2*k_B/e = 2 * 1.380649e-23 / 1.602176634e-19 = 1.7234667e-4 eV/K.
        let fold = two_kb_ev_per_k();
        assert!(
            close(fold, 1.7234667e-4, 1e-8),
            "2 k_B ~ 1.7234667e-4 eV/K, got {}",
            fold.to_f64_lossy()
        );
    }

    #[test]
    fn the_activation_exponent_is_minus_the_gap_over_2kt() {
        // The log-space activation exponent -E_gap/(2 k_B T) at world temperature (300 K, 2 k_B T = 0.05170 eV).
        let t300 = Fixed::from_int(300);
        // Germanium E_gap = 0.67 eV: -0.67 / 0.05170 = -12.96.
        let ge =
            ln_thermal_carrier_activation(Fixed::from_ratio(67, 100), t300, GapGrade::Measured)
                .expect("Ge activation");
        assert!(
            close(ge, -12.96, 0.05),
            "Ge ln-activation ~ -12.96, got {}",
            ge.to_f64_lossy()
        );
        // Silicon E_gap = 1.12 eV: -1.12 / 0.05170 = -21.66.
        let si =
            ln_thermal_carrier_activation(Fixed::from_ratio(112, 100), t300, GapGrade::Measured)
                .expect("Si activation");
        assert!(
            close(si, -21.66, 0.05),
            "Si ln-activation ~ -21.66, got {}",
            si.to_f64_lossy()
        );
    }

    #[test]
    fn a_metal_gap_has_zero_suppression_and_bad_inputs_escalate() {
        let t300 = Fixed::from_int(300);
        // A zero gap (the metal/semimetal boundary): activation exponent 0 (factor 1, no thermal suppression).
        let metal = ln_thermal_carrier_activation(ZERO, t300, GapGrade::Measured)
            .expect("a zero gap has a defined activation");
        assert_eq!(metal, ZERO, "a zero gap has no thermal suppression");
        // A negative gap (a band overlap, a metal) is classified upstream and escalates here rather than modelling
        // a gap that is not there.
        assert!(
            ln_thermal_carrier_activation(Fixed::from_int(-1), t300, GapGrade::Measured).is_none(),
            "a negative gap (overlap) escalates: it is not a thermally-activated semiconductor"
        );
        // A non-positive temperature has no thermal population defined and escalates.
        assert!(
            ln_thermal_carrier_activation(Fixed::from_int(1), ZERO, GapGrade::Measured).is_none(),
            "a non-positive temperature escalates"
        );
    }

    #[test]
    fn the_log_space_form_survives_an_insulator_where_the_bare_factor_underflows() {
        // THE CENSUS PAYOFF. Diamond's 5.47 eV gap gives an activation factor exp(-105.8) ~ 1e-46, far below the
        // Q32.32 floor (~2.3e-10): the bare factor underflows to zero and loses all ordering. The LOG-SPACE
        // exponent -105.8 is representable, so insulators' activations stay ordered without underflow. This is why
        // the carrier density is carried in log space (the range-census verdict of the electronic sub-arc).
        let t300 = Fixed::from_int(300);
        let diamond =
            ln_thermal_carrier_activation(Fixed::from_ratio(547, 100), t300, GapGrade::Measured)
                .expect("diamond activation");
        assert!(
            close(diamond, -105.8, 0.2),
            "diamond ln-activation ~ -105.8, got {}",
            diamond.to_f64_lossy()
        );
        // And a wider gap is strictly more suppressed (more negative), so the log-space values order correctly: the
        // insulator sits below the semiconductor.
        let si =
            ln_thermal_carrier_activation(Fixed::from_ratio(112, 100), t300, GapGrade::Measured)
                .expect("Si");
        assert!(
            diamond < si,
            "the wider gap is more suppressed (a more negative log-activation)"
        );
    }

    #[test]
    fn the_activation_factor_round_trips_through_exp_in_window() {
        // For an in-window semiconductor exponent, exp rebuilds the activation factor: germanium's -12.96
        // exponentiates to ~2.35e-6, the thermal carrier fraction. (The insulator case above stays in log space by
        // design, which is the point of returning the log.)
        let t300 = Fixed::from_int(300);
        let ge_ln =
            ln_thermal_carrier_activation(Fixed::from_ratio(67, 100), t300, GapGrade::Measured)
                .expect("Ge");
        let factor = ge_ln.exp();
        assert!(
            close(factor, 2.35e-6, 5e-7),
            "Ge activation factor ~ 2.35e-6, got {}",
            factor.to_f64_lossy()
        );
    }

    #[test]
    fn the_measured_route_sorts_by_gap_sign_with_no_semiconductor_insulator_threshold() {
        // The emergent metal / non-metal boundary is the gap SIGN, and the semiconductor-versus-insulator split is
        // the CONTINUOUS activation, never a threshold. (Cited test fixtures: silicon 1.12 eV, diamond 5.47 eV.)
        let t300 = Fixed::from_int(300);
        // A metal: a zero (band-overlap) gap.
        assert_eq!(
            conduction_class_measured(ZERO, t300),
            ConductionClass::Metal
        );
        let si = conduction_class_measured(Fixed::from_ratio(112, 100), t300);
        let diamond = conduction_class_measured(Fixed::from_ratio(547, 100), t300);
        // The measured route is authoritative, so the exponent is present (Some) on both.
        match (si, diamond) {
            (
                ConductionClass::NonMetal {
                    ln_activation: Some(si_ln),
                },
                ConductionClass::NonMetal {
                    ln_activation: Some(dia_ln),
                },
            ) => {
                // No eV threshold separates them: both are non-metals, and the wider-gap diamond is simply more
                // suppressed (more negative), so the semiconductor/insulator distinction emerges continuously.
                assert!(
                    dia_ln < si_ln,
                    "the wider-gap diamond is more suppressed than silicon (the thresholdless continuous split)"
                );
            }
            _ => panic!(
                "silicon and diamond are non-metals with a present exponent on the measured route"
            ),
        }
    }

    #[test]
    fn the_uw_preflight_keeps_a_mott_insulator_from_being_called_a_metal() {
        // THE REDIRECT-2 PAYOFF. On the reduced-order route, a naive band model would hand NiO a metallic (<= 0)
        // gap. The U/W preflight sites NiO as Localized FIRST and returns CorrelatedInsulator, so the Mott
        // insulator is never called a metal, the exact failure the correlation turn closed.
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        let t300 = Fixed::from_int(300);
        // A bogus metallic gap (0) a reduced-order model might return for NiO; the preflight overrides it.
        let nio = conduction_class_estimated(&comp(&[("Ni", 1), ("O", 1)]), ZERO, &c, t300);
        assert_eq!(
            nio,
            ConductionClass::CorrelatedInsulator,
            "the preflight keeps NiO a Mott insulator, never a metal"
        );
    }

    #[test]
    fn the_uw_preflight_lets_an_itinerant_material_through_to_the_gap_sort() {
        // TiO sites Itinerant, so the preflight lets it through to the gap-sign sort; with a metallic (<= 0) gap it
        // is a metal, which TiO is (an itinerant early-3d monoxide). The preflight guards the correlated-insulator
        // case without blocking a validated itinerant.
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        let t300 = Fixed::from_int(300);
        let tio = conduction_class_estimated(&comp(&[("Ti", 1), ("O", 1)]), ZERO, &c, t300);
        assert_eq!(
            tio,
            ConductionClass::Metal,
            "an itinerant TiO passes the preflight and sorts as a metal"
        );
    }

    #[test]
    fn the_measured_route_is_authoritative_and_skips_the_preflight() {
        // A measured NiO gap (~4.0 eV, a cited charge-transfer-gap test fixture) outranks the reduced-order U/W
        // classifier: the measured route runs the gap sort directly and calls NiO a non-metal insulator (a huge
        // suppression, activation ~ -77), with no CorrelatedInsulator interference. The measurement encodes the
        // Mott gap.
        let t300 = Fixed::from_int(300);
        let nio = conduction_class_measured(Fixed::from_int(4), t300);
        match nio {
            ConductionClass::NonMetal {
                ln_activation: Some(ln),
            } => {
                assert!(
                    ln.to_f64_lossy() < -70.0,
                    "a 4 eV gap is deeply insulating (activation ~ -77), got {}",
                    ln.to_f64_lossy()
                );
            }
            _ => panic!("measured NiO is a non-metal insulator via its measured gap"),
        }
    }

    #[test]
    fn a_non_correlatable_substance_escalates_on_the_reduced_order_route_for_now() {
        // THE FLAGGED SEAM (surfaced for the Harrison-rung ruling): silicon is OutOfScope for the correlation
        // classifier (not a rock-salt d-block oxide), so on the reduced-order route the preflight escalates it (the
        // conservative-safe choice). Correct NOW (the reduced-order estimator is held); when the Harrison rung
        // lands, an sp semiconductor like silicon must proceed to the gap sort while an out-of-3d correlated centre
        // must still escalate, the follow-on. On the MEASURED route silicon is a normal semiconductor, unaffected.
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        let t300 = Fixed::from_int(300);
        let si_estimated =
            conduction_class_estimated(&comp(&[("Si", 1)]), Fixed::from_ratio(112, 100), &c, t300);
        assert_eq!(
            si_estimated,
            ConductionClass::Escalate,
            "silicon (OutOfScope) escalates on the reduced-order route for now"
        );
        // But the measured route sorts it as the semiconductor it is.
        let si_measured = conduction_class_measured(Fixed::from_ratio(112, 100), t300);
        assert!(
            matches!(si_measured, ConductionClass::NonMetal { .. }),
            "measured silicon is a semiconductor"
        );
    }

    #[test]
    fn the_column_consumer_sorts_a_seeded_semiconductor_and_a_mott_insulator() {
        // Slice 3b: the banked column drives the classification. Silicon's cited 1.12 eV measured gap sorts it a
        // non-metal semiconductor; NiO's cited 4.3 eV measured gap sorts it a non-metal insulator (a Mott insulator
        // whose MEASURED gap is authoritative, so it sorts correctly with no preflight, and is far more suppressed
        // than the semiconductor). The metal/insulator split is the continuous activation from the banked gaps.
        let col = BandGapColumn::standard().expect("the band-gap column loads");
        let t300 = Fixed::from_int(300);
        let si = conduction_class_from_column(&col, &comp(&[("Si", 1)]), t300);
        let nio = conduction_class_from_column(&col, &comp(&[("Ni", 1), ("O", 1)]), t300);
        // The column rows are measured (authoritative), so the exponent is present (Some) on both.
        match (si, nio) {
            (
                ConductionClass::NonMetal {
                    ln_activation: Some(si_ln),
                },
                ConductionClass::NonMetal {
                    ln_activation: Some(nio_ln),
                },
            ) => {
                assert!(
                    nio_ln < si_ln,
                    "the 4.3 eV Mott insulator is far more suppressed than the 1.12 eV semiconductor"
                );
            }
            _ => panic!("Si and NiO sort as non-metals with a present exponent from their banked measured gaps"),
        }
    }

    #[test]
    fn the_column_consumer_escalates_a_substance_with_no_banked_gap() {
        // A substance absent from the column has no banked [M] or compute-once gap, so the tier escalates (the
        // reduced-order Harrison route, held, would sit there). Aluminium is a metal, not a gapped substance, and
        // is not in the seeded gap column.
        let col = BandGapColumn::standard().expect("the band-gap column loads");
        let t300 = Fixed::from_int(300);
        assert_eq!(
            conduction_class_from_column(&col, &comp(&[("Al", 1)]), t300),
            ConductionClass::Escalate,
            "a substance with no banked gap escalates (the reduced-order route is held)"
        );
    }

    #[test]
    fn a_compute_once_gw_row_routes_authoritatively_like_a_measurement() {
        // A COMPUTE-ONCE hybrid/GW gap is authoritative (it encodes the correlation gap), so the column consumer
        // routes it through the gap-sign sort exactly as a measurement, with no U/W preflight. (A test-only GW
        // fixture; the standard column is all measured until a cited GW value is banked.)
        let fixture = r#"
[[gap]]
name = "test GW semiconductor"
composition = { Xx = 1 }
gap_ev = "2.0"
provenance = "computed"
functional = "GW"
source = "test-only fixture"
"#;
        let col = BandGapColumn::from_toml_str(fixture).expect("the fixture column loads");
        let t300 = Fixed::from_int(300);
        let xx = conduction_class_from_column(&col, &comp(&[("Xx", 1)]), t300);
        // Compute-once is authoritative, so the exponent is admitted (Some), like a measurement.
        assert!(
            matches!(
                xx,
                ConductionClass::NonMetal {
                    ln_activation: Some(_)
                }
            ),
            "a compute-once GW gap routes authoritatively (a non-metal with a present exponent), no preflight"
        );
    }

    #[test]
    fn the_exponent_rider_bars_an_estimator_gap_from_the_carrier_density() {
        // THE EXPONENT RIDER (owner ruling): an estimator-grade gap is barred from the carrier-density exponent (a
        // factor-grade gap miss is orders of magnitude in carrier density), while an authoritative gap is admitted.
        let t300 = Fixed::from_int(300);
        let gap = Fixed::from_ratio(112, 100); // 1.12 eV
        assert!(
            ln_thermal_carrier_activation(gap, t300, GapGrade::Estimator).is_none(),
            "an estimator gap is barred from the carrier-density exponent"
        );
        assert!(
            ln_thermal_carrier_activation(gap, t300, GapGrade::Measured).is_some(),
            "a measured gap is admitted to the exponent"
        );
        assert!(
            ln_thermal_carrier_activation(gap, t300, GapGrade::ComputeOnce).is_some(),
            "a compute-once gap is admitted to the exponent"
        );
    }

    #[test]
    fn an_estimator_route_classifies_but_escalates_the_exponent() {
        // On the reduced-order route, an itinerant substance with a positive estimator gap is CLASSIFIED a non-metal
        // (classification and ranking are admitted to the estimator), but its carrier-density exponent escalates
        // (ln_activation None), so no estimator gap is ever exponentiated into a fabricated carrier density. (A
        // synthetic positive gap on the itinerant TiO exercises the non-metal-estimator branch; TiO is truly a
        // metal, but the mechanism under test is the estimator exponent guard.)
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        let t300 = Fixed::from_int(300);
        let tio_gapped =
            conduction_class_estimated(&comp(&[("Ti", 1), ("O", 1)]), Fixed::from_int(1), &c, t300);
        assert_eq!(
            tio_gapped,
            ConductionClass::NonMetal { ln_activation: None },
            "an estimator-grade non-metal classifies, but the carrier-density exponent escalates (the rider)"
        );
    }
}
