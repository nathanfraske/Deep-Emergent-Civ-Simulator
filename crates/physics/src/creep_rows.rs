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

//! THE MEASURED CREEP ROWS: Hirth and Kohlstedt 2003 Table 1, as the source states it, under five conditions.
//!
//! THE DUCTILE BRANCH of the yield-strength envelope. It meets the brittle branch ([`crate::yield_envelope`])
//! at a depth the world's own physics sets, and `T_e` falls out of where they cross.
//!
//! # THIS IS A LADDER RUNG, NOT THE LAW
//!
//! Two parameterizations of creep exist in this engine and THEIR PREFACTORS ARE NOT INTERCHANGEABLE:
//!
//! - `civsim_materials::creep` (Mukherjee-Bird-Dorn): `eps_dot = A (sigma/G)^n (b/d)^p [D G b / (kT)]`, with `A`
//!   DIMENSIONLESS, stress normalized by the shear modulus, grain size by the Burgers vector, and the Arrhenius
//!   carried inside `D` (the freezer's jump rate). This is the DERIVED/ESTIMATOR rung: it evaluates for any
//!   material with banked columns, and the owner's route makes its activation energy derive from the world's own
//!   solidus (`E*` through the 3b class constant `g R T_m`).
//! - THIS MODULE (Hirth and Kohlstedt): `eps_dot = A sigma^n d^-p f_H2O^r exp(-(E* + P V*)/(R T))`, with `A` in
//!   the TABLE'S OWN UNITS, raw stress, an explicit Arrhenius, and a WATER-FUGACITY TERM the MBD form does not
//!   have at all. This is the MEASURED/ANCHOR rung.
//!
//! FEEDING H&K's `A = 1.1e5` INTO THE MBD PREFACTOR WOULD BE A SILENT UNIT CATASTROPHE. They are not one law
//! with two constant sets; they are two normalizations of one quantity, which makes creep a LADDER (measured
//! before estimator, dispatched on anchor availability) and NOT a choice anyone makes at a call site. The
//! prefactor hazard is sharper than it looks: the verification agent found H&K's Table 1 header prints NO UNIT
//! FOR `A` at all, so any "MPa^-3.5 s^-1" string attached to it downstream is a CONSUMER'S DERIVATION rather
//! than the source's claim.
//!
//! # THE FIVE CONDITIONS, each realized here
//!
//! 1. THE EXPONENT GATE RUNS AT INGESTION ([`ExponentInput`], [`CreepRow::exponent_admits`]). `E* + P V*` over
//!    `R T` sits inside an Arrhenius EXPONENTIAL, where a grade error does not add, it multiplies through an
//!    exp. So every quantity entering the exponent is grade-checked, and NOTHING ESTIMATOR-GRADE CROSSES without
//!    an explicit escalation.
//! 2. THE WATER VARIABLE'S KILOBAR DEFENSE ([`WaterContent`]). H&K's wet rows are FUGACITY-referenced; the fetch
//!    named feeding H-per-10^6-Si into them as the exact silent error, and the prior fetch INVERTED this very
//!    fact. So water content is a newtype ENCODING ITS REFERENCE FRAME, and the wrong reference FAILS TO
//!    TYPECHECK rather than fails to be noticed.
//! 3. THE ROW SCHEMA KEYS ON MECHANISM AND WATER STATE JOINTLY ([`CreepRow`] is atomic). A Dixon-style composite
//!    (his `E* = 400` from the DRY GBS row, welded to an `r = 1.2` from a WET dislocation row) cannot be
//!    assembled: a caller takes a whole row or nothing.
//! 4. THE STRAIN RATE IS A DAY-ONE INPUT ([`ductile_strength_mpa`] takes it), never retrofitted. `T_e` is a CHORD
//!    OVER LOAD TIMESCALE, so the rate is the load's own, and the row cannot be evaluated without one.
//! 5. The A-source retirement checklist is a separate concern (the fixture cluster lives at two call sites) and
//!    is tracked on the board, not here.
//!
//! # WHAT THIS MODULE DOES NOT CLAIM
//!
//! The YSE is an UPPER ENVELOPE with a live dispute band, never ground truth: laboratory friction integrated
//! through a yield-strength envelope is known to OVERPREDICT the strength of natural, faulted lithosphere (the
//! stress paradox and the weak-fault debate). Asserting the envelope as truth would be modality laundering at
//! framework scale. This module supplies one branch of a construction that ships hindcast-calibrated with its
//! lab-to-field dispute declared.

use civsim_core::Fixed;

/// `ln(mantissa * 10^power)`, assembled from the mantissa and the POWER OF TEN separately.
///
/// THIS HELPER EXISTS BECAUSE ITS ABSENCE COST A 10x ERROR, caught by this module's own test. Written by hand,
/// `ln(1.1e5)` became `ln(1.1) + 4*ln(10)`: the power of ten was miscounted by one, and the result was 9.31
/// where the truth is 11.61. A prefactor multiplies the creep rate directly, so a silently mis-transcribed
/// power of ten is a factor-of-ten error in a flow law, arriving as a plausible number with no symptom.
///
/// H&K's prefactors span 90 to 4.7e10, which does not fit Q32.32's ~2.1e9 ceiling at all, so the log form is
/// the only representable one AND the transcription is where the hazard lives. Taking the mantissa and the
/// exponent as separate arguments makes the exponent something the caller states rather than something the
/// caller counts in their head.
fn ln_scientific(mantissa_num: i64, mantissa_den: i64, power_of_ten: i32) -> Fixed {
    let mantissa = Fixed::from_ratio(mantissa_num, mantissa_den).ln();
    let decade = Fixed::from_int(10).ln();
    mantissa + decade * Fixed::from_int(power_of_ten)
}

/// THE MODALITY of a value, verbatim from its source. The card field that exists because the laundered
/// `+/- 0.5` taught the lesson: a HYPOTHETICAL sensitivity sentence in H&K's own prose circulated downstream as
/// a MEASUREMENT, and the warning it carried (about exactly the strain-rate extrapolation this arc's spec
/// dropped) arrived as a number with its meaning thrown away.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modality {
    /// Measured directly, with the source's own uncertainty.
    Measured,
    /// Fitted from measurements, carrying the fit's band.
    Fitted,
    /// ASSUMED by the source, or TRANSFERRED from a different experiment. H&K's GBS activation energies are
    /// this: transferred from easy-slip data (their footnote f), never fitted to GBS at all.
    Assumed,
    /// A HYPOTHETICAL: a sensitivity statement, an illustration, a "suppose". The `+/- 0.5` was one of these.
    Hypothetical,
    /// ESTIMATED by a model rather than a measurement (a bare estimator output).
    Estimated,
    /// CLASS-DERIVED: a class-grade constant times a derived quantity, which the standing exponent rider ADMITS.
    ///
    /// THIS VARIANT IS A PRE-EMPTION, added before the gate could convict its own ladder's other leg. The
    /// ESTIMATOR rung's activation energy is `E* = g * R * T_m`: `g` is measured-class and `T_m` is derived, so
    /// the product is CLASS-GRADE IN THE EXPONENT, which is the same legal status the freezer already relies on,
    /// with the band propagated. A gate admitting only `Measured | Fitted` is correct for H&K ROW INGESTION and
    /// would be WRONG the moment the gate's scope reaches the estimator rung, failing that leg against a test
    /// written for its neighbour.
    ///
    /// It weakens nothing: NO H&K row carries this grade, so the measured rung's gate is untouched. This is the
    /// diamond-gate lesson applied in advance, that a gate must not convict the design it exists to protect,
    /// rather than after a green suite has to be explained.
    ///
    /// CONSTRUCTOR-GATED, which is this grade's graduation from DETECTION to IMPOSSIBILITY. The variant carries a
    /// [`ClassDerivedWitness`] whose only field is private, so it cannot be written as a literal annotation from
    /// outside this module: the ONLY way to obtain it is [`class_derived_activation_energy`], which COMPUTES
    /// `g * R * T_m`. A future convenient constant therefore cannot dress itself in the grade to walk past the
    /// exponent gate. The test that used to guard this by proving no row wore the tag is now guarding something
    /// the type system already forbids, which is where a defense belongs once its shape permits it.
    ClassDerived(ClassDerivedWitness),
}

/// The private witness that a value came from the `g * R * T_m` route. Its field is private, so only this module
/// can mint one, and [`class_derived_activation_energy`] is the only place that does.
///
/// This is the seal that turns [`Modality::ClassDerived`] from a tag anyone can apply into a receipt only the
/// computation can issue. Without it, the grade is an honour system guarded by a test, and an honour system in
/// front of an Arrhenius exponent is exactly the shape this project keeps convicting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClassDerivedWitness(());

/// THE ESTIMATOR RUNG'S ACTIVATION ENERGY, and the only source of [`Modality::ClassDerived`]:
/// `E* = g * R * T_m`, the 3b class constant against the world's OWN melting temperature.
///
/// This is what makes any material's creep derivable from its own solidus rather than from a table that only
/// exists for olivine, so it is the leg an alien phase stands on. `g` is measured-class and `T_m` is derived, so
/// the product is CLASS-GRADE IN THE EXPONENT, the same legal status the freezer relies on, band propagated.
///
/// `None` on a non-positive melting temperature or gas constant (no solidus, no route) or overflow. The returned
/// [`ExponentInput`] carries the witness, so it is admitted to the exponent BECAUSE it was computed here, never
/// because someone said so.
pub fn class_derived_activation_energy(
    class_constant_g: Fixed,
    gas_constant_r: Fixed,
    melting_point_k: Fixed,
) -> Option<ExponentInput> {
    if melting_point_k <= Fixed::ZERO || gas_constant_r <= Fixed::ZERO {
        return None;
    }
    let value = class_constant_g
        .checked_mul(gas_constant_r)?
        .checked_mul(melting_point_k)?;
    Some(ExponentInput {
        value,
        modality: Modality::ClassDerived(ClassDerivedWitness(())),
    })
}

impl Modality {
    /// Whether a value of this modality may cross into the ARRHENIUS EXPONENT without escalation.
    ///
    /// THE EXPONENT IS NOT AN ORDINARY CONSUMER. `exp(-(E* + P V*)/(R T))` multiplies a grade error through an
    /// exponential: a 10 percent error in `E*` is a factor at lid temperatures, not a 10 percent shift. So the
    /// bar here is higher than elsewhere in the engine, deliberately, and it is checked at INGESTION rather than
    /// trusted at the call site.
    /// The admitted set is `Measured | Fitted | ClassDerived`. The first two are the measured rung's; the third
    /// is the ESTIMATOR rung's, pre-admitted per the standing exponent rider (a class-grade constant times a
    /// derived quantity is class-grade in the exponent, the freezer's own precedent, band propagated).
    ///
    /// REFUSED: `Assumed`, `Hypothetical`, and bare `Estimated`. An assumed value inside an Arrhenius exponent
    /// is not a small sin, and a hypothetical one is a sentence someone turned into a number.
    pub fn admitted_to_exponent(self) -> bool {
        matches!(
            self,
            Modality::Measured | Modality::Fitted | Modality::ClassDerived(_)
        )
    }
}

/// WATER CONTENT, CARRYING ITS REFERENCE FRAME IN ITS TYPE.
///
/// THE DEFENCE THIS EXISTS FOR. H&K's wet rows come in two parameterizations that look identical in a table and
/// are not interchangeable: the FUGACITY-referenced rows (`A = 1600`, `E* = 520`) take water as the fugacity of
/// H2O in megapascals, while the CONSTANT-C_OH rows (`A = 90`, `E* = 480`) take it as hydroxyl concentration in
/// H per 10^6 Si. A prior fetch INVERTED exactly this, reporting the library's fugacity rows as C_OH-referenced,
/// which would have fed an H/10^6Si number into a law expecting megapascals of fugacity. The fetch's own hazard
/// section had NAMED that as the silent error before it happened.
///
/// So the frame is a TYPE, not a convention. A row keyed to one frame cannot be evaluated with the other: the
/// wrong reference fails to TYPECHECK rather than failing to be noticed. This is the kilobar defence, applied
/// before the error rather than after it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WaterContent {
    /// Water fugacity `f_H2O`, in MEGAPASCALS. The frame H&K's `A = 1600` / `E* = 520` rows are stated against.
    Fugacity { mpa: Fixed },
    /// Hydroxyl concentration `C_OH`, in H per 10^6 Si. The frame the constant-C_OH rows (`A = 90`,
    /// `E* = 480`) are stated against.
    Hydroxyl { h_per_10e6_si: Fixed },
}

/// The WATER STATE a row is calibrated for. Half of the row's joint identity (the other half is the mechanism),
/// which is what makes a cross-row composite unassemblable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WaterState {
    /// A DRY row: no water term at all. Its `r` is printed as "-" in the table, which is why welding a water
    /// exponent onto a dry row (as one secondary source's figure caption did) has no home in the physics.
    Dry,
    /// A WET row referenced to water FUGACITY.
    WetFugacity,
    /// A WET row referenced to constant hydroxyl concentration.
    WetHydroxyl,
}

/// The DEFORMATION MECHANISM a row is calibrated for. The other half of the row's joint identity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mechanism {
    /// Diffusion creep: `n = 1` (linear response), `p = 3`.
    Diffusion,
    /// Dislocation creep: `n = 3.5`, `p = 0` (grain-size independent).
    Dislocation,
    /// Grain-boundary sliding. H&K parameterize it DRY ONLY, which is the fact that convicts the composite: a
    /// consumer needing a water exponent for GBS cannot get one from this table, so one secondary source
    /// borrowed it from a different row and produced a triple that describes no experiment.
    GrainBoundarySliding,
}

/// An input to the Arrhenius exponent, carrying its modality so the gate can check it AT INGESTION.
#[derive(Clone, Copy, Debug)]
pub struct ExponentInput {
    pub value: Fixed,
    pub modality: Modality,
}

/// AN ACTIVATION-VOLUME DETERMINATION, WITH ITS PRESSURE INTERVAL ATTACHED.
///
/// `V*` IS A CHORD QUANTITY, which is the finding that dissolved an apparent conflict. Two sources reported
/// non-overlapping `V*` for dry olivine dislocation creep (6 versus 13 to 27 cubic centimetres per mole) and it
/// looked like a disagreement to arbitrate. It is not. H&K's Table 1 prints NO dry-dislocation `V*` at all;
/// their Table 2 gives NINE determinations spanning -2 to 27, and they fail to overlap because **`V*` DECREASES
/// WITH PRESSURE** (H&K state this themselves) and each source drew its chord over a DIFFERENT PRESSURE
/// INTERVAL. A bare `V*` is a chord with its endpoints dropped.
///
/// So a determination carries the interval it was drawn over, and a consumer takes the one matching ITS OWN
/// pressure regime: the lid reads the low-pressure determinations, the deep interior the high-pressure ones. The
/// sweep that scopes this band's blast radius sweeps the WITHIN-REGIME scatter, never the fake full span.
#[derive(Clone, Copy, Debug)]
pub struct ActivationVolume {
    /// The determination, in cubic centimetres per mole.
    pub cm3_per_mol: Fixed,
    /// The LOW end of the pressure interval this chord was drawn over (gigapascals).
    pub interval_min_gpa: Fixed,
    /// The HIGH end of the pressure interval this chord was drawn over (gigapascals).
    pub interval_max_gpa: Fixed,
    pub modality: Modality,
}

impl ActivationVolume {
    /// Whether this determination's chord covers a pressure, which is the only question that makes a bare `V*`
    /// meaningful. A consumer outside every determination's interval is EXTRAPOLATING a chord, and the honest
    /// answer there is `None` from [`select_activation_volume`] rather than the nearest number.
    pub fn covers(&self, pressure_gpa: Fixed) -> bool {
        pressure_gpa >= self.interval_min_gpa && pressure_gpa <= self.interval_max_gpa
    }
}

/// Select the activation-volume determination whose CHORD COVERS the consumer's pressure regime.
///
/// `None` when no determination covers it: the consumer is outside every measured interval, and reaching for
/// the nearest value would be extrapolating a chord past its endpoints, which is the exact defect the interval
/// tagging exists to prevent. Escalate rather than extrapolate.
pub fn select_activation_volume(
    determinations: &[ActivationVolume],
    pressure_gpa: Fixed,
) -> Option<ActivationVolume> {
    determinations
        .iter()
        .copied()
        .find(|d| d.covers(pressure_gpa))
}

/// ONE ROW OF H&K TABLE 1, ATOMIC BY CONSTRUCTION.
///
/// THE ATOMICITY IS THE DEFENCE (condition 3). The row's identity is `(mechanism, water_state)` JOINTLY, and a
/// caller takes a whole row or nothing. That is what makes a DIXON-STYLE COMPOSITE unassemblable: his triple
/// welded `E* = 400` (which appears exactly once in Table 1, as DRY GBS below 1250 C) to `V* = 14` and `r = 1.2`
/// from a different row of different mechanism AND different water state. The dry GBS row's `r` is printed "-",
/// so his water exponent has no home in it. The composite describes no experiment, and it exists because H&K
/// parameterize GBS dry only while he needed a water exponent. Here the type system refuses the weld.
#[derive(Clone, Copy, Debug)]
pub struct CreepRow {
    pub mechanism: Mechanism,
    pub water_state: WaterState,
    /// `ln A`. Carried in LOG SPACE because H&K's prefactors span 90 to 4.7e10 and the raw values overflow
    /// Q32.32's ~2.1e9 ceiling outright. THE UNIT HAZARD, verbatim from the verification: the table's HEADER
    /// PRINTS NO UNIT FOR `A`, so any unit string attached to it downstream is a consumer's derivation rather
    /// than the source's claim, and it is NOT interchangeable with the MBD form's dimensionless prefactor.
    pub ln_prefactor: Fixed,
    /// The stress exponent `n`. NOTE: 3.5 +/- 0.3 for dislocation, from the table itself. The widely circulated
    /// +/- 0.5 is MODALITY LAUNDERING: it traces to a HYPOTHETICAL sensitivity sentence in H&K's prose ("an
    /// uncertainty in the stress exponent of +/- 0.5 results in +/- one order of magnitude uncertainty"), which
    /// was the source WARNING about extrapolation sensitivity, not reporting a measurement.
    pub stress_exponent: Fixed,
    /// The grain-size exponent `p`: 3 diffusion, 0 dislocation, 2 GBS. A mechanism label, never a tuneable.
    pub grain_size_exponent: i32,
    /// The water-fugacity exponent `r`. `None` for a DRY row, whose table entry is "-": a dry row has no water
    /// exponent, and that absence is load-bearing rather than a missing datum.
    pub water_exponent: Option<Fixed>,
    /// The activation energy `E*` (kilojoules per mole) WITH ITS MODALITY. H&K's GBS energies are `Assumed`
    /// (transferred from easy-slip data, their footnote f), which the exponent gate then refuses without
    /// escalation, because an assumed value inside an exponential is not a small sin.
    pub activation_energy: ExponentInput,
}

impl CreepRow {
    /// THE EXPONENT GATE, RUN AT INGESTION (condition 1). Whether this row's activation energy and the selected
    /// activation volume may both cross into `exp(-(E* + P V*)/(R T))` without escalation.
    ///
    /// Returns `false` for a row whose `E*` is Assumed or Hypothetical, or whose `V*` determination is. H&K's
    /// two GBS rows fail this by construction, which is correct and is the gate working: their energies were
    /// never fitted to GBS.
    pub fn exponent_admits(&self, volume: &ActivationVolume) -> bool {
        self.activation_energy.modality.admitted_to_exponent()
            && volume.modality.admitted_to_exponent()
    }

    /// Whether a water measurement is in the REFERENCE FRAME this row is stated against (condition 2).
    ///
    /// A dry row admits no water at all. A fugacity row admits only fugacity; a hydroxyl row only hydroxyl. The
    /// enum makes the mismatch representable so it can be REFUSED rather than silently evaluated: passing an
    /// H/10^6Si number to a fugacity row is the silent error the fetch predicted, and it dies here.
    pub fn accepts_water(&self, water: Option<WaterContent>) -> bool {
        matches!(
            (self.water_state, water),
            (WaterState::Dry, None)
                | (WaterState::WetFugacity, Some(WaterContent::Fugacity { .. }))
                | (WaterState::WetHydroxyl, Some(WaterContent::Hydroxyl { .. }))
        )
    }
}

/// H&K 2003 Table 1, DRY DISLOCATION creep: `A = 1.1e5`, `n = 3.5 +/- 0.3`, `p = 0`, `r = -`, `E* = 530 +/- 4`.
/// The row the lid's ductile branch reads on a dry silicate world.
pub fn hk_dry_dislocation() -> CreepRow {
    CreepRow {
        mechanism: Mechanism::Dislocation,
        water_state: WaterState::Dry,
        ln_prefactor: ln_scientific(11, 10, 5), // A = 1.1e5
        stress_exponent: Fixed::from_ratio(35, 10),
        grain_size_exponent: 0,
        water_exponent: None,
        activation_energy: ExponentInput {
            value: Fixed::from_int(530),
            modality: Modality::Fitted,
        },
    }
}

/// H&K 2003 Table 1, DRY GRAIN-BOUNDARY SLIDING below 1250 C: `A = 6500`, `n = 3.5`, `p = 2`, `r = -`,
/// `E* = 400`.
///
/// THE ROW THAT CONVICTS THE COMPOSITE, kept for exactly that reason. `E* = 400` appears EXACTLY ONCE in Table 1
/// and this is it: DRY, `r = "-"`. A widely read secondary source's FIGURE CAPTION labels it "wet dislocation
/// creep" while his own BODY TEXT calls it GBS, and his `(400, 14, 1.2)` triple welds this row's energy to a
/// water exponent from a different row of different mechanism and water state. Its `E*` is `Assumed` (transferred
/// from easy-slip data, footnote f), so [`CreepRow::exponent_admits`] REFUSES it without escalation.
pub fn hk_dry_gbs_below_1250c() -> CreepRow {
    CreepRow {
        mechanism: Mechanism::GrainBoundarySliding,
        water_state: WaterState::Dry,
        ln_prefactor: ln_scientific(65, 10, 3), // A = 6500 = 6.5e3
        stress_exponent: Fixed::from_ratio(35, 10),
        grain_size_exponent: 2,
        water_exponent: None,
        activation_energy: ExponentInput {
            value: Fixed::from_int(400),
            // ASSUMED, transferred from easy-slip data (H&K footnote f), never fitted to GBS.
            modality: Modality::Assumed,
        },
    }
}

/// H&K 2003 Table 1, WET DISLOCATION creep, FUGACITY-referenced: `A = 1600`, `n = 3.5 +/- 0.3`, `p = 0`,
/// `r = 1.2 +/- 0.4`, `E* = 520 +/- 40`, `V* = 22 +/- 11`.
///
/// ITS WATER IS FUGACITY IN MEGAPASCALS, not H per 10^6 Si. [`CreepRow::accepts_water`] enforces that, because
/// a prior fetch reported this row as C_OH-referenced and feeding it a hydroxyl number is the silent error its
/// own hazard section had already named.
pub fn hk_wet_dislocation_fugacity() -> CreepRow {
    CreepRow {
        mechanism: Mechanism::Dislocation,
        water_state: WaterState::WetFugacity,
        ln_prefactor: ln_scientific(16, 10, 3), // A = 1600 = 1.6e3
        stress_exponent: Fixed::from_ratio(35, 10),
        grain_size_exponent: 0,
        water_exponent: Some(Fixed::from_ratio(12, 10)),
        activation_energy: ExponentInput {
            value: Fixed::from_int(520),
            modality: Modality::Fitted,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_water_frame_is_a_type_so_the_wrong_reference_cannot_be_evaluated() {
        // CONDITION 2, the kilobar defence. The fugacity row takes MEGAPASCALS of f_H2O. A prior fetch reported
        // it as C_OH-referenced; feeding it H/10^6 Si is the silent error its own hazard section named. Here the
        // mismatch is REFUSED rather than silently computed.
        let wet = hk_wet_dislocation_fugacity();
        assert!(wet.accepts_water(Some(WaterContent::Fugacity {
            mpa: Fixed::from_int(300)
        })));
        assert!(
            !wet.accepts_water(Some(WaterContent::Hydroxyl {
                h_per_10e6_si: Fixed::from_int(1000)
            })),
            "a fugacity row must REFUSE a hydroxyl number: that is the exact silent error the fetch predicted"
        );
        assert!(!wet.accepts_water(None), "a wet row is not a dry row");

        // A DRY row admits no water at all, and that absence is load-bearing.
        let dry = hk_dry_dislocation();
        assert!(dry.accepts_water(None));
        assert!(
            !dry.accepts_water(Some(WaterContent::Fugacity {
                mpa: Fixed::from_int(300)
            })),
            "a dry row has no water term; handing it water is not a small approximation"
        );
        assert!(
            dry.water_exponent.is_none(),
            "the dry row's r is printed '-' in the table"
        );
    }

    #[test]
    fn the_exponent_gate_refuses_assumed_values_at_ingestion() {
        // CONDITION 1. exp(-(E* + P V*)/(R T)) multiplies a grade error through an exponential, so the bar is
        // higher here than elsewhere and it is checked at INGESTION.
        let measured_v = ActivationVolume {
            cm3_per_mol: Fixed::from_int(15),
            interval_min_gpa: Fixed::ZERO,
            interval_max_gpa: Fixed::from_int(2),
            modality: Modality::Fitted,
        };
        // A fitted row passes.
        assert!(hk_dry_dislocation().exponent_admits(&measured_v));
        // THE GBS ROW FAILS BY CONSTRUCTION, and that is the gate working: H&K's GBS energies are ASSUMED,
        // transferred from easy-slip data (their footnote f), never fitted to GBS at all.
        assert!(
            !hk_dry_gbs_below_1250c().exponent_admits(&measured_v),
            "an ASSUMED activation energy must not cross into an Arrhenius exponent without escalation"
        );
        // An estimated V* also fails, even under a fitted E*: the gate checks EVERY quantity entering the
        // exponent, not just the headline one.
        let estimated_v = ActivationVolume {
            modality: Modality::Estimated,
            ..measured_v
        };
        assert!(!hk_dry_dislocation().exponent_admits(&estimated_v));
        // The admitted set. Measured and Fitted are the measured rung's.
        assert!(Modality::Measured.admitted_to_exponent());
        assert!(Modality::Fitted.admitted_to_exponent());
        assert!(!Modality::Assumed.admitted_to_exponent());
        assert!(!Modality::Hypothetical.admitted_to_exponent());
        assert!(
            !Modality::Estimated.admitted_to_exponent(),
            "a bare estimator output does not cross"
        );
    }

    #[test]
    fn the_class_derived_preemption_admits_the_estimator_leg_without_weakening_this_one() {
        // THE PRE-EMPTION, and its own proof. The estimator rung's E* = g*R*T_m is a class-grade constant times
        // a derived melting point, which the standing exponent rider and the freezer precedent both ADMIT. A
        // gate accepting only Measured|Fitted is right for H&K ingestion and would fail the ladder's OTHER LEG
        // against a test written for its neighbour. This is the diamond-gate lesson applied BEFORE the fact: do
        // not build a gate that convicts your own ladder.
        // The grade is obtainable ONLY by computing it. There is no literal to write.
        let e_star = class_derived_activation_energy(
            Fixed::from_ratio(23, 10), // a class-grade g
            Fixed::from_ratio(8314, 1000),
            Fixed::from_int(1600), // the world's own T_m
        )
        .expect("the g*R*T_m route resolves");
        assert!(
            e_star.modality.admitted_to_exponent(),
            "the estimator rung's g*R*T_m route must cross, or the ladder's other leg fails its neighbour's test"
        );
        // The route REFUSES rather than fabricating when the world has no solidus to key on.
        assert!(class_derived_activation_energy(Fixed::ONE, Fixed::ONE, Fixed::ZERO).is_none());
        // AND IT WEAKENS NOTHING HERE, which is the half that makes the pre-emption safe rather than a loophole:
        // no H&K row carries the class-derived grade, so the measured rung's gate is untouched, and the row that
        // must fail still fails.
        let v = ActivationVolume {
            cm3_per_mol: Fixed::from_int(15),
            interval_min_gpa: Fixed::ZERO,
            interval_max_gpa: Fixed::from_int(2),
            modality: Modality::Fitted,
        };
        for row in [
            hk_dry_dislocation(),
            hk_wet_dislocation_fugacity(),
            hk_dry_gbs_below_1250c(),
        ] {
            assert!(
                !matches!(row.activation_energy.modality, Modality::ClassDerived(_)),
                "no measured row may wear the estimator's grade; that would launder the ladder's rungs together"
            );
        }
        assert!(
            !hk_dry_gbs_below_1250c().exponent_admits(&v),
            "the ASSUMED GBS energy is still refused after the pre-emption: nothing was loosened"
        );
    }

    #[test]
    fn v_star_is_a_chord_and_the_consumer_takes_the_one_covering_its_regime() {
        // THE CHORD RULING. The apparent 6-versus-13-to-27 conflict was never a disagreement: V* DECREASES WITH
        // PRESSURE and each source drew its chord over a different interval. So a determination carries its
        // interval and the consumer takes the one covering ITS regime.
        let low = ActivationVolume {
            cm3_per_mol: Fixed::from_int(20),
            interval_min_gpa: Fixed::ZERO,
            interval_max_gpa: Fixed::from_int(2),
            modality: Modality::Fitted,
        };
        let high = ActivationVolume {
            cm3_per_mol: Fixed::from_int(6),
            interval_min_gpa: Fixed::from_int(2),
            interval_max_gpa: Fixed::from_int(10),
            modality: Modality::Fitted,
        };
        let rows = [low, high];
        // THE LID reads the low-pressure determination; THE DEEP INTERIOR reads the high-pressure one. Same
        // quantity, same table, different chords, and the difference is physics rather than disagreement.
        let lid = select_activation_volume(&rows, Fixed::ONE).expect("the lid's regime is covered");
        assert_eq!(lid.cm3_per_mol, Fixed::from_int(20));
        let deep = select_activation_volume(&rows, Fixed::from_int(5))
            .expect("the deep regime is covered");
        assert_eq!(deep.cm3_per_mol, Fixed::from_int(6));
        // OUTSIDE every interval: escalate rather than extrapolate. Reaching for the nearest value would be
        // extrapolating a chord past its endpoints, which is the defect the tagging exists to prevent.
        assert!(
            select_activation_volume(&rows, Fixed::from_int(40)).is_none(),
            "no determination covers 40 GPa; the honest answer is None, never the nearest number"
        );
    }

    #[test]
    fn a_dixon_style_composite_cannot_be_assembled_from_the_rows() {
        // CONDITION 3, and the row that convicts it. E* = 400 appears EXACTLY ONCE in Table 1: dry GBS below
        // 1250 C. The composite welded that energy to a water exponent r = 1.2 taken from a WET DISLOCATION row.
        // The atomic row makes the weld unrepresentable: identity is (mechanism, water_state) JOINTLY.
        let gbs = hk_dry_gbs_below_1250c();
        let wet = hk_wet_dislocation_fugacity();
        assert_eq!(gbs.activation_energy.value, Fixed::from_int(400));
        // The 400 row is DRY and has NO water exponent for a 1.2 to live in.
        assert_eq!(gbs.water_state, WaterState::Dry);
        assert!(
            gbs.water_exponent.is_none(),
            "the row carrying E* = 400 has no water exponent: the composite's r = 1.2 has no home in it"
        );
        // The r = 1.2 belongs to a row of DIFFERENT mechanism AND DIFFERENT water state. Two axes apart.
        assert_eq!(wet.water_exponent, Some(Fixed::from_ratio(12, 10)));
        assert_ne!(gbs.mechanism, wet.mechanism, "different mechanism");
        assert_ne!(gbs.water_state, wet.water_state, "different water state");
        // And its energy is ASSUMED, so it cannot reach the exponent unescalated regardless.
        assert_eq!(gbs.activation_energy.modality, Modality::Assumed);
    }

    #[test]
    fn the_stress_exponent_is_the_tables_value_not_the_laundered_hypothetical() {
        // MODALITY LAUNDERING, pinned. The table prints 3.5 +/- 0.3. The circulating +/- 0.5 traces to a
        // HYPOTHETICAL sensitivity sentence in H&K's own prose, which was the source WARNING about extrapolation
        // sensitivity, and the warning arrived downstream as a number with its meaning thrown away.
        for row in [
            hk_dry_dislocation(),
            hk_wet_dislocation_fugacity(),
            hk_dry_gbs_below_1250c(),
        ] {
            assert_eq!(
                row.stress_exponent,
                Fixed::from_ratio(35, 10),
                "every dislocation-class row carries the table's n = 3.5"
            );
        }
    }

    #[test]
    fn the_prefactors_are_carried_in_log_space_because_they_overflow_the_type() {
        // H&K's prefactors span 90 to 4.7e10, and Q32.32 tops out near 2.1e9, so the raw values do not fit at
        // all. Carrying ln(A) is not an optimization, it is the only representable form. And the unit hazard
        // rides with it: the table's header prints NO UNIT for A, so any unit string downstream is a consumer's
        // derivation, and it is NOT interchangeable with the MBD form's dimensionless prefactor.
        // THE TWIN ROUTE, STATED, because a twin whose independence is accidental asserts nothing. Each pinned
        // value below comes from a DIFFERENT ROUTE than the entry: the entry builds `ln(m) + p*ln(10)` through
        // the fixed-point `ln`, while the pin is the decimal logarithm read off the printed value by an outside
        // computation and typed as a literal. Different method, different representation. If the pin were
        // re-derived from the same expression it would be the same hand typing twice, and the 10x power-of-ten
        // error this test caught would have passed.
        let dry = hk_dry_dislocation();
        // TWIN: ln(1.1e5) = 11.6083... by external decimal log, against the entry's ln_scientific(11, 10, 5).
        assert!(
            (dry.ln_prefactor - Fixed::from_ratio(1161, 100)).abs() < Fixed::from_ratio(1, 10),
            "ln(1.1e5) ~ 11.61, got {:?}",
            dry.ln_prefactor
        );
        // TWIN: ln(1600) = 7.3778... by external decimal log, against the entry's ln_scientific(16, 10, 3).
        let wet = hk_wet_dislocation_fugacity();
        assert!(
            (wet.ln_prefactor - Fixed::from_ratio(738, 100)).abs() < Fixed::from_ratio(1, 10),
            "ln(1600) ~ 7.38, got {:?}",
            wet.ln_prefactor
        );
        // TWIN: ln(6500) = 8.7794... by external decimal log, against the entry's ln_scientific(65, 10, 3).
        // PINNED because this row carried the SAME off-by-one power of ten as the dry dislocation row did, and
        // both were caught by this test rather than by review.
        let gbs = hk_dry_gbs_below_1250c();
        assert!(
            (gbs.ln_prefactor - Fixed::from_ratio(878, 100)).abs() < Fixed::from_ratio(1, 10),
            "ln(6500) ~ 8.78, got {:?}",
            gbs.ln_prefactor
        );
    }
}
