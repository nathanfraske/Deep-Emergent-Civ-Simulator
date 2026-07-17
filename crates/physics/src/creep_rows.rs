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
//!   material with banked columns, and its activation energy derives from the world's own material
//!   through Form B (`E* = f * E_coh`, ruling #187), never the retired composite `g * R * T_m`.
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
//! 4. THE STRAIN RATE IS A DAY-ONE INPUT ([`ductile_strength_mpa`] takes it as
//!    [`CreepConditions::ln_strain_rate_per_s`], required, with no default and no fallback), never retrofitted.
//!    `T_e` is a CHORD OVER LOAD TIMESCALE, so the rate is the load's own, and the row cannot be evaluated
//!    without one. THIS LINE WAS A FALSE IMPLEMENTATION-STATUS CLAIM until the inversion slice landed: it named
//!    `ductile_strength_mpa` as the realization of the condition while the function existed nowhere but this
//!    sentence, a rustdoc link to nothing. It is retired by shipping the evaluator, and it now cites the tests
//!    that hold it up rather than asserting itself: `the_worked_examples_referee_the_law_itself` and
//!    `the_worked_examples_referee_the_inversion_against_the_sources_own_stress` referee the arithmetic
//!    against H&K's OWN two worked examples, and `the_strain_rate_has_no_default_and_the_composite_needs_it`
//!    holds the required-input half.
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
use civsim_units::bignum::BigRat;
use civsim_units::fundamentals;
use std::sync::OnceLock;

/// `ln(mantissa * 10^power)`, assembled from the mantissa and the POWER OF TEN separately.
///
/// PUBLIC BECAUSE THE EVALUATOR'S OWN SIGNATURE CREATES THE NEED. [`CreepConditions::ln_strain_rate_per_s`]
/// takes the strain rate in LOG SPACE because a lid's rate (1e-15 per second) is not representable in Q32.32 at
/// all, so a caller CANNOT reach the log by writing `rate.ln()`: the rate rounds to zero before `ln` ever sees
/// it, and `ln(0)` is the [`Fixed::MIN`] sentinel. This is the only constructor that carries a sub-resolution
/// rate to its logarithm without passing through an unrepresentable intermediate, so it is the caller's route
/// rather than an internal convenience.
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
pub fn ln_scientific(mantissa_num: i64, mantissa_den: i64, power_of_ten: i32) -> Fixed {
    let mantissa = Fixed::from_ratio(mantissa_num, mantissa_den).ln();
    let decade = Fixed::from_int(10).ln();
    mantissa + decade * Fixed::from_int(power_of_ten)
}

/// THE MOLAR GAS CONSTANT `R = N_A * k_B` (joules per mole per kelvin), DERIVED once from the two CODATA
/// fundamentals and projected to `Fixed`, never an authored decimal. Memoized; a pure load constant.
///
/// THE DUPLICATION IS DELIBERATE AND NAMED. `melting.rs` derives `R` by this identical route from these
/// identical registered fundamentals, and this crate keeps its constants per-module rather than in a shared
/// home. The two sites cannot disagree: same registered inputs, same identity, same bits. So this is a
/// duplicated DERIVATION rather than a second MODEL of one quantity, which is the thing the ladder doctrine
/// forbids. Hoisting both to one crate-level constant is a cross-file change, so it is surfaced rather than
/// reached for.
///
/// H&K's own worked examples enter `R = 8.314`. This enters the CODATA value (8.3144626...), which differs by
/// 5.6e-5 relative and moves the worked-example twin's recovered stress by 0.06 percent, far inside that twin's
/// tolerance. The floor's derived value is the legal one; rounding it to match the source's printed constant
/// would be authoring a decimal to make a test prettier.
fn molar_gas_constant() -> Fixed {
    static R: OnceLock<Fixed> = OnceLock::new();
    *R.get_or_init(|| {
        let n_a = BigRat::from_decimal_str(
            fundamentals::fundamental("N_A")
                .expect("Avogadro is a registered fundamental")
                .value,
        )
        .expect("Avogadro parses");
        let k_b = BigRat::from_decimal_str(
            fundamentals::fundamental("k_B")
                .expect("Boltzmann is a registered fundamental")
                .value,
        )
        .expect("Boltzmann parses");
        Fixed::from_bits_i128(
            n_a.mul(&k_b)
                .round_to_scale(Fixed::FRAC_BITS)
                .expect("R = N_A k_B ~ 8.314 fits Q32.32"),
        )
        .expect("R projects to Fixed")
    })
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
    /// ESTIMATOR rung's activation energy is Form B's `E* = f * E_coh` (ruling #187): `f` is the per-class
    /// vacancy fraction (reserved-with-basis, cited, keyed on bonding class) and `E_coh` is derived, so the
    /// product is CLASS-GRADE IN THE EXPONENT, the same legal status the freezer already relies on, band propagated. A gate admitting only `Measured | Fitted` is correct for H&K ROW INGESTION and
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

impl Modality {
    /// Whether a value of this modality may cross into the ARRHENIUS EXPONENT without escalation.
    ///
    /// THE EXPONENT IS NOT AN ORDINARY CONSUMER. `exp(-(E* + P V*)/(R T))` multiplies a grade error through an
    /// exponential: a 10 percent error in `E*` is a factor at lid temperatures, not a 10 percent shift. So the
    /// bar here is higher than elsewhere in the engine, deliberately, and it is checked at INGESTION rather than
    /// trusted at the call site.
    ///
    /// The admitted set is `Measured | Fitted | ClassDerived`. The first two are the measured rung's; the third
    /// is the ESTIMATOR rung's, pre-admitted per the standing exponent rider (Form B's `f * E_coh` is a
    /// per-class reserved fraction times a derived cohesive energy, class-grade in the exponent, the freezer's
    /// own precedent, band propagated).
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

/// THE ESTIMATOR RUNG'S ACTIVATION ENERGY, and the only source of [`Modality::ClassDerived`].
///
/// TAKES THE FORM-B BARRIER, `E* = f * E_coh`: the per-class vacancy fraction `f` times the derived Rose
/// cohesive energy, computed by `civsim_materials::freezer::diffusion_barrier`.
///
/// # THE COMPOSITE `g * R * T_m` WAS RETIRED HERE, AND WHY MATTERS MORE THAN THAT IT WAS
///
/// This function first shipped computing `E* = g * R * T_m` from a caller-supplied `g`. That form was ALREADY
/// SUPERSEDED by the project's own ruling #187 before it was written, which the freezer's header records: Form B
/// "reuses the derived `E_coh` directly rather than routing through the composite `g * R * T_m`", because
/// `g = k * f` with `k = E_coh/(R * T_m)`, so pulling `k` out is ONE DERIVATION HOP SHORTER AND ONE CORRELATION
/// FEWER.
///
/// THE CORRELATION IS THE POINT, and it names a defect class: SAME-FACT-TWO-DOORS. `E_coh` and `T_m` are ONE
/// PHYSICAL FACT, cohesion, wearing two variables. The composite `g` carries that fact inside its own
/// calibration while `T_m` re-delivers it beside, so `g * R * T_m` consumes the same provenance THROUGH TWO
/// DOORS. It is the derivation-level sibling of the diamond: two CARRIERS of one fact inside a single FORMULA,
/// rather than two PROVIDERS of one fact across a codebase. Keying `g` per bonding class, which was the
/// alternative on the table, would have HARDENED that hidden correlation under a per-class band worn as
/// reassurance.
///
/// # THE SEAL'S HONEST REACH
///
/// [`ClassDerivedWitness`] still gates the grade: only this function mints it, so a convenient constant cannot
/// dress itself in `ClassDerived` to walk past the exponent gate. But the seal CANNOT VERIFY ITS INPUT'S
/// PROVENANCE, and the reason is structural: `civsim_materials` depends on `civsim_physics`, so this crate
/// cannot call the freezer, and the barrier must arrive as a value. The witness therefore certifies HOW THE
/// GRADE WAS OBTAINED (through this constructor) and NOT WHERE THE BARRIER CAME FROM. A caller passing a number
/// that did not come from `diffusion_barrier` is lying to the type, and the type cannot catch it. That limit is
/// named rather than papered over; closing it means moving this rung beside the freezer, which is a later
/// question than this retirement.
///
/// `None` on a non-positive barrier (no barrier, no route) rather than a fabricated energy.
pub fn class_derived_activation_energy(form_b_barrier: Fixed) -> Option<ExponentInput> {
    if form_b_barrier <= Fixed::ZERO {
        return None;
    }
    Some(ExponentInput {
        value: form_b_barrier,
        modality: Modality::ClassDerived(ClassDerivedWitness(())),
    })
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
    /// meaningful. A consumer outside every determination's interval is EXTRAPOLATING a chord, and what
    /// [`select_activation_volume`] reports there is the TABLE'S OWN EXTREMES rather than the nearest number.
    pub fn covers(&self, pressure_gpa: Fixed) -> bool {
        pressure_gpa >= self.interval_min_gpa && pressure_gpa <= self.interval_max_gpa
    }
}

/// WHETHER THE SOURCE'S OWN CHORDS REACH the pressure a `V*` bracket was drawn at. Carried with every bracket,
/// so a consumer can never read a span without learning whether the table had anything to say there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VolumeConstraint {
    /// At least one determination's chord COVERS the pressure, and the bracket spans exactly the covering ones.
    CoveredBySource,
    /// NO determination's chord covers the pressure. The bracket is the TABLE'S OWN EXTREMES, and the source
    /// constrains nothing at this pressure: the span is what the table supports, never a measurement of it.
    UnconstrainedBySource,
}

/// WHICH END of a `V*` bracket an evaluation is taken at. There is no default and no midpoint: a caller naming
/// neither end would be collapsing a span the primary declines to collapse, which is the whole defect the
/// bracket exists to prevent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VolumeEnd {
    /// The smallest `V*` the bracket spans. At a positive pressure this is the WEAKEST the row can be, since
    /// `P V*` raises the Arrhenius numerator.
    Low,
    /// The largest `V*` the bracket spans, and hence the STRONGEST the row can be at a positive pressure.
    High,
}

/// THE `V*` SELECTION'S ANSWER: A BRACKET over the determinations, never the point the primary declines to pick.
///
/// # THE FIRST-MATCH DEFECT THIS RETIRES
///
/// This selection used to be a `.find()`. With one determination banked that was invisible; with H&K's Table 2
/// banked (nine determinations, several of which cover any lid pressure) it would have become an
/// ORDER-DEPENDENT AUTHORED SELECTION of the very number the primary declines to choose, made silently, by
/// slice position. The bracket answers that: it reports the span and picks no point inside it, and a min and a
/// max over a SET cannot read the order the set was listed in (Principle 3).
///
/// # THE GAP CASE IS THE SAME SHAPE, AND IT IS WHY THE SPAN REACHES OUTSIDE THE CHORDS
///
/// The banked chords start at 0.3 GPa, about nine kilometres on Earth, so a lid sampled FROM THE SURFACE sits
/// outside every interval through its whole brittle top. Refusing there blocks the full-column solve over a
/// quantity that cannot move the answer; reaching for the nearest determination authors one. So outside every
/// chord the bracket is the TABLE'S OWN EXTREMES, tagged [`VolumeConstraint::UnconstrainedBySource`]: it does
/// not invent and it does not refuse, it reports what the table supports and leaves the consumer to prove the
/// width cannot reach its answer.
///
/// THAT PROOF IS THE CONSUMER'S, AND IT IS ASSERTED RATHER THAN ASSUMED. `P V*` tops out near 8 kJ/mol at
/// 0.3 GPa against `E*`'s 530, so in the shallow column the span cannot change WHICH BRANCH of the yield
/// envelope wins, and the envelope's minimum is identical at both ends
/// ([`crate::moment_equivalence::LithosphereEnvelope`], whose `yield_in_sense` evaluates both ends and reports
/// no single number where they disagree).
#[derive(Clone, Copy, Debug)]
pub struct ActivationVolumeBracket {
    /// The smallest `V*` the bracketed set reports (cubic centimetres per mole).
    low_cm3_per_mol: Fixed,
    /// The largest. Equal to `low_cm3_per_mol` where one determination stands alone, which is the DEGENERATE
    /// bracket: a span of zero width is a determination rather than a band, and it evaluates identically at
    /// both ends.
    high_cm3_per_mol: Fixed,
    constraint: VolumeConstraint,
    /// Whether EVERY determination the bracket was drawn from may cross into the Arrhenius exponent.
    ///
    /// ASKED OVER THE SET RATHER THAN THE TWO ENDS, which is what keeps the gate order-free. Two determinations
    /// can report the same `V*` under different modalities, and then which one lands at an end is a fact about
    /// the slice's order rather than about the table. The span is ONE claim, so every member of it is graded and
    /// the answer is a pure function of the SET (Principle 3).
    admitted_to_exponent: bool,
}

impl ActivationVolumeBracket {
    /// The `V*` at one end of the span (cubic centimetres per mole).
    pub fn at(&self, end: VolumeEnd) -> Fixed {
        match end {
            VolumeEnd::Low => self.low_cm3_per_mol,
            VolumeEnd::High => self.high_cm3_per_mol,
        }
    }

    /// Whether the source's own chords reach the pressure this bracket was drawn at.
    pub fn constraint(&self) -> VolumeConstraint {
        self.constraint
    }

    /// Whether the span has zero width, which is one determination standing alone and is a determination rather
    /// than a band.
    pub fn is_degenerate(&self) -> bool {
        self.low_cm3_per_mol == self.high_cm3_per_mol
    }

    /// Whether every determination the span was drawn from may cross into the Arrhenius exponent.
    pub fn admitted_to_exponent(&self) -> bool {
        self.admitted_to_exponent
    }
}

/// Bracket the activation-volume determinations at the consumer's pressure: the COVERING ones where the
/// source's chords reach it, the WHOLE TABLE where they do not. See [`ActivationVolumeBracket`].
///
/// `None` only where there is no determination at all to bracket. An empty table is the one case with no span
/// to report, and it is distinct from a table whose chords miss: the latter still supports its own extremes.
pub fn select_activation_volume(
    determinations: &[ActivationVolume],
    pressure_gpa: Fixed,
) -> Option<ActivationVolumeBracket> {
    let covers_any = determinations.iter().any(|d| d.covers(pressure_gpa));
    let constraint = if covers_any {
        VolumeConstraint::CoveredBySource
    } else {
        VolumeConstraint::UnconstrainedBySource
    };
    // THE BRACKETED SET: the covering determinations where the chords reach the pressure, the whole table where
    // none does. Both ends and the grade are taken over that set with a min, a max, and an `all`, none of which
    // can read the slice's order.
    let mut low: Option<Fixed> = None;
    let mut high: Option<Fixed> = None;
    let mut admitted = true;
    for d in determinations
        .iter()
        .filter(|d| !covers_any || d.covers(pressure_gpa))
    {
        low = Some(low.map_or(d.cm3_per_mol, |l: Fixed| l.min(d.cm3_per_mol)));
        high = Some(high.map_or(d.cm3_per_mol, |h: Fixed| h.max(d.cm3_per_mol)));
        admitted = admitted && d.modality.admitted_to_exponent();
    }
    Some(ActivationVolumeBracket {
        low_cm3_per_mol: low?,
        high_cm3_per_mol: high?,
        constraint,
        admitted_to_exponent: admitted,
    })
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
    /// THE EXPONENT GATE, RUN AT INGESTION (condition 1). Whether this row's activation energy and the bracketed
    /// activation volume may both cross into `exp(-(E* + P V*)/(R T))` without escalation.
    ///
    /// Returns `false` for a row whose `E*` is Assumed or Hypothetical, or where ANY determination the `V*`
    /// bracket spans is. H&K's two GBS rows fail this by construction, which is correct and is the gate working:
    /// their energies were never fitted to GBS.
    ///
    /// THE VOLUME'S HALF IS ASKED OVER THE WHOLE SPAN, never the two ends alone, which is what stops the grade
    /// from depending on the order a caller listed its determinations in. See
    /// [`ActivationVolumeBracket::admitted_to_exponent`].
    pub fn exponent_admits(&self, volume: &ActivationVolumeBracket) -> bool {
        self.activation_energy.modality.admitted_to_exponent() && volume.admitted_to_exponent()
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

/// H&K 2003 Table 1, WET DISLOCATION creep, CONSTANT-C_OH referenced: `A = 90`, `n = 3.5 +/- 0.3`, `p = 0`,
/// `r = 1.2`, `E* = 480 +/- 40`, `V* = 11`.
///
/// THE ROW THE FETCH NAMED AS MISSING, AND THE ONE THE SOURCE ITSELF WORKS. The verification found that the
/// widely-used machine-readable reproduction carries only the FUGACITY rows while labelling their water field
/// `1000`, and that `1000` is plausible in BOTH frames: it is 1000 MPa of fugacity (inside the primary's own
/// figure range) and it is also the 1000 H/10^6 Si the primary calls the asthenospheric water content. A reader
/// who sees `1000`, reads it as a content, and feeds the fugacity row has silently switched parameterization
/// with every number still looking reasonable. The primary settles which set belongs with a content of 1000
/// H/10^6 Si by doing that exact calculation in its own footnote e: it uses THIS row. The two parameterizations
/// differ by a factor of ~18 in `A` and 40 kJ/mol in `E*`, so the switch is not a rounding.
///
/// It is banked here because it was ABSENT from every reproduction while being the row the source's own worked
/// example uses, which makes it both the closed hazard and the referee: `the_worked_examples_referee_the_law_itself`
/// evaluates it against H&K's printed answer. `WetHydroxyl` had no constructor at all until now, so the frame
/// the type system was built to protect had nothing to protect.
///
/// NOT IN THE ENGINE'S ADMITTED SET (see [`admit_candidate`]): it needs a water content this engine derives
/// nowhere. It is a cited row waiting on a water substrate, never a row to feed a fabricated number to.
pub fn hk_wet_dislocation_hydroxyl() -> CreepRow {
    CreepRow {
        mechanism: Mechanism::Dislocation,
        water_state: WaterState::WetHydroxyl,
        ln_prefactor: ln_scientific(9, 1, 1), // A = 90 = 9e1
        stress_exponent: Fixed::from_ratio(35, 10),
        grain_size_exponent: 0,
        water_exponent: Some(Fixed::from_ratio(12, 10)),
        activation_energy: ExponentInput {
            value: Fixed::from_int(480),
            // 480 +/- 40 carries a fitted band, and it is the constant-water-content REPARAMETERIZATION of the
            // same wet dislocation creep the 520 +/- 40 row states in the fugacity frame, never a competing
            // estimate of a different quantity.
            modality: Modality::Fitted,
        },
    }
}

/// H&K 2003 Table 1, WET DIFFUSION creep, CONSTANT-C_OH referenced: `A = 1.0e6`, `n = 1`, `p = 3`, `r = 1`,
/// `E* = 335 +/- 75`, `V* = 4`.
///
/// The second row the source works in its own footnote (footnote c), and the DIFFUSION-mechanism referee: it
/// exercises the linear branch (`n = 1`, so the whole exponent passes into the stress undivided) and the
/// GRAIN-SIZE term (`p = 3`, at `d = 10 mm`), neither of which the dislocation rows can test. Note what the
/// source does with its own law there: it evaluates `p = 3` at a grain size of 10 mm, THREE ORDERS OF MAGNITUDE
/// coarser than the fine-grained synthetic aggregates the diffusion-creep experiments ran on. The extrapolation
/// is the authors' own, and it rides with the row.
///
/// NOT IN THE ENGINE'S ADMITTED SET, twice over: it needs a water content AND a grain size, and this engine
/// derives neither.
pub fn hk_wet_diffusion_hydroxyl() -> CreepRow {
    CreepRow {
        mechanism: Mechanism::Diffusion,
        water_state: WaterState::WetHydroxyl,
        ln_prefactor: ln_scientific(1, 1, 6), // A = 1.0e6
        stress_exponent: Fixed::ONE,
        grain_size_exponent: 3,
        water_exponent: Some(Fixed::ONE),
        activation_energy: ExponentInput {
            value: Fixed::from_int(335),
            modality: Modality::Fitted,
        },
    }
}

/// THE CONDITIONS a load presents to the creep rows. Every field is an input the caller must supply; there is
/// no default anywhere in this struct, which is condition 4 wearing a type.
#[derive(Clone, Copy, Debug)]
pub struct CreepConditions {
    /// `ln(eps_dot)`, the strain rate in reciprocal seconds, IN LOG SPACE.
    ///
    /// THE LOG IS NOT A CONVENIENCE, IT IS THE ONLY REPRESENTABLE FORM. Geological strain rates run 1e-15 to
    /// 1e-10 per second and Q32.32 resolves 2^-32 (about 2.3e-10), so a lid's own strain rate ROUNDS TO ZERO as
    /// a bare `Fixed`, and a law fed that zero returns an infinite strength with no symptom. Build it with
    /// [`ln_scientific`], which reaches the logarithm without passing through the unrepresentable value.
    ///
    /// THE RATE IS THE LOAD'S OWN (condition 4). `T_e` is a chord over the load's timescale, so the caller
    /// supplies THE RATE ITS LOAD IMPOSES, and it is REQUIRED. No default exists to fall back to, and inventing
    /// one would author the very quantity the elastic thickness is a chord over.
    ///
    /// NOT THE CONVECTIVE RATE, and this sentence used to say otherwise IN THE SAME BREATH AS FORBIDDING IT.
    /// It read "the rate is the load's own ... so this is derived from THE WORLD'S CONVECTIVE TIMESCALE by the
    /// caller", which names the right rule and then points at the wrong rate, in one sentence. The convective
    /// rate (`laws::convective_strain_rate`, which exposes the `|v|/L` that `convective_stress` had always
    /// formed and discarded) is the MANTLE-AND-THERMAL chord: it serves mantle viscosity and the thermal side.
    /// A LOAD IS NOT THE MANTLE. A seamount's flexure and an interior convection cell impose different rates on
    /// different timescales, and evaluating a lid's STRENGTH against the mantle's rate while its DRIVING STRESS
    /// answers to the load would compare two chords and call the difference physics. Found by the slice that
    /// consumes this function, which is the reader most able to be misled by it.
    pub ln_strain_rate_per_s: Fixed,
    /// Temperature (kelvin).
    pub temperature_k: Fixed,
    /// Pressure (GIGAPASCALS).
    ///
    /// GPa IS THIS MODULE'S OWN PRESSURE CURRENCY, already: [`ActivationVolume::interval_min_gpa`] and
    /// [`ActivationVolume::covers`] speak it, so the domain gate and the law's arithmetic read ONE number in
    /// ONE unit and cannot drift apart. The source's own examples enter pressure in PASCALS paired with `V*` in
    /// m^3/mol; the pairing is what the physics requires (their product must land in J/mol), and this module
    /// banks the equivalent pairing (GPa with cm^3/mol) that its existing fields already state. Pascals would
    /// also be the wrong choice on the type: `Fixed` tops out near 2.1e9, so a pressure in Pa would overflow
    /// above ~2.1 GPa, and the primary's own Table 2 carries determinations to 15 GPa.
    pub pressure_gpa: Fixed,
    /// Grain size (MICROMETRES), for the rows whose `p != 0`. `None` where the caller has no grain size.
    ///
    /// The engine derives no grain size today, so every admitted row is a `p = 0` row and this is `None` on the
    /// production path. The field exists because the LAW has the term, and the rows that carry it are cited and
    /// banked; what refuses them is [`admit_candidate`], which is an ENGINE-CAPABILITY gate rather than a
    /// physics one.
    pub grain_size_um: Option<Fixed>,
    /// Water, IN THE FRAME IT WAS MEASURED IN. `None` for a dry evaluation.
    ///
    /// The engine derives no water fugacity and no water content today, so this is `None` on the production
    /// path and the wet rows are refused at admission rather than fed a fabricated number.
    pub water: Option<WaterContent>,
}

/// A CANDIDATE MECHANISM for the parallel composite: a row plus the activation-volume determinations available
/// for it.
///
/// The determinations arrive as a SET rather than a value because `V*` is a chord (see [`ActivationVolume`]):
/// H&K's Table 1 prints no dry-dislocation `V*` at all and defers to a Table 2 of nine determinations over
/// nine different pressure intervals. [`select_activation_volume`] BRACKETS them at the caller's own pressure,
/// spanning the covering chords where they reach it and the table's own extremes where none does, so no point
/// inside the span is ever chosen and no slice order can move which one is read.
#[derive(Clone, Copy, Debug)]
pub struct CreepCandidate<'a> {
    pub row: CreepRow,
    /// The `V*` determinations for this row, each carrying the pressure chord it was drawn over.
    pub volumes: &'a [ActivationVolume],
}

/// WHY A ROW OR A COMPOSITE REFUSED. Every variant is a refusal to evaluate, never a degraded answer.
///
/// THE REASON IS CARRIED RATHER THAN COLLAPSED TO `None`, and the difference is testable: a test that asserts
/// `is_none()` passes when the WRONG gate fires, so a suite built on `Option` can go green while proving
/// something other than what it claims. Naming the gate makes each test convict its own gate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CreepRefusal {
    /// THE ENGINE CANNOT FEED THIS ROW: it is wet, and no water fugacity or content is derived anywhere in this
    /// engine. An ENGINE-CAPABILITY refusal, not a physics one: the row is cited and correct, and it retires
    /// the day a water substrate lands.
    WaterNotDerived,
    /// THE ENGINE CANNOT FEED THIS ROW: its `p != 0` and no grain size is derived anywhere in this engine.
    /// Also an engine-capability refusal. `p = 0` is dislocation creep, which is the whole admitted set today.
    GrainSizeNotDerived,
    /// The row's own water frame was violated: a fugacity row handed a hydroxyl number, a hydroxyl row handed a
    /// fugacity, or a dry row handed water. The row's OWN contract, from the source, distinct from the two
    /// engine-capability refusals above.
    WaterFrameMismatch,
    /// A `p != 0` row was handed no grain size, or a non-positive one.
    GrainSizeMissing,
    /// The exponent gate refused: an `Assumed`, `Hypothetical` or bare `Estimated` value tried to cross into
    /// `exp(-(E* + P V*)/(R T))` without escalation. H&K's two GBS rows fail here by construction.
    ExponentGrade,
    /// NO `V*` DETERMINATION EXISTS AT ALL for this row, so there is no span to bracket and no exponent to
    /// evaluate.
    ///
    /// THIS IS NOT THE OUT-OF-CHORD CASE, and the difference is the third application of the gap precedent. A
    /// caller whose pressure sits outside every banked chord is served the TABLE'S OWN EXTREMES, tagged
    /// [`VolumeConstraint::UnconstrainedBySource`], because the table still supports its own span there. Only an
    /// EMPTY table has nothing to report, and that is this variant.
    NoActivationVolumeBanked,
    /// No candidate survived admission, so there is no composite to solve.
    NoAdmittedRow,
    /// A condition is outside the law's domain (a non-positive temperature, water, or stress).
    ConditionOutOfDomain,
    /// THE STRESS IS REAL BUT NOT REPRESENTABLE in Q32.32 megapascals, and the log-space answer rides along so
    /// nothing is lost. Reachable in both directions from a lid's own range: a cold lid at a geological rate
    /// drives the creep strength past `Fixed::MAX` (the flow law saying creep is irrelevant there, which the
    /// envelope's brittle branch then floors), and a hot one drives it below `Fixed::EPSILON`. Returning a
    /// saturated maximum or a silent zero would hand the envelope a fabricated number in the one place the
    /// envelope cannot tell it from a real one.
    StressNotRepresentable { ln_stress_mpa: Fixed },
    /// The fixed-point arithmetic overflowed.
    NotRepresentable,
}

/// THE ADMISSION GATE, and the honest name for what it is: a statement about THIS ENGINE, not about the physics.
///
/// A row is admitted when the engine can supply every input the row needs, and refused when it cannot. Today
/// that means DRY rows with `p = 0`, which is dislocation creep, because the engine derives no water fugacity,
/// no water content, and no grain size. The refusal is the value line doing its job at the widest point: the
/// alternative to refusing a wet row is picking a fugacity, and there is no fugacity to pick, only one to
/// invent. So the wet rows stay banked, cited, and unfed until a water substrate derives their input, and this
/// gate loosens by deleting a branch rather than by anyone rewriting a law.
///
/// It also runs the gate the ROW itself states, which is a different thing and is the source's rather than
/// ours: every quantity entering the Arrhenius exponent must be graded for it ([`CreepRow::exponent_admits`]).
///
/// THE CHORD-COVERAGE GATE IS GONE FROM HERE, and its absence is the ruling rather than an omission. It used to
/// refuse a caller whose pressure sat outside every banked interval, which blocked a lid from being sampled at
/// its own surface: the banked chords start at 0.3 GPa, about nine kilometres down. Coverage is now REPORTED on
/// the bracket ([`VolumeConstraint`]) instead of being a refusal, so the consumer learns what the source
/// constrains and proves for itself whether the span can reach its answer.
///
/// Returns the bracketed activation volume, which is what an admitted row needs to be evaluated at either end.
pub fn admit_candidate(
    candidate: &CreepCandidate<'_>,
    pressure_gpa: Fixed,
) -> Result<ActivationVolumeBracket, CreepRefusal> {
    // THE ENGINE-CAPABILITY GATES, first, because they are about our inputs rather than the row's validity.
    if candidate.row.water_state != WaterState::Dry {
        return Err(CreepRefusal::WaterNotDerived);
    }
    if candidate.row.grain_size_exponent != 0 {
        return Err(CreepRefusal::GrainSizeNotDerived);
    }
    // THE ROW'S OWN GATE, which is the source's.
    let volume = select_activation_volume(candidate.volumes, pressure_gpa)
        .ok_or(CreepRefusal::NoActivationVolumeBanked)?;
    if !candidate.row.exponent_admits(&volume) {
        return Err(CreepRefusal::ExponentGrade);
    }
    Ok(volume)
}

/// THE STRESS-INDEPENDENT PART of `ln(eps_dot)`, which is where every unit conversion in this module lives.
///
/// The law is `eps_dot = A sigma^n d^-p f_H2O^r exp(-(E* + P V*)/(R T))`, so in log space it is LINEAR in
/// `ln sigma` with slope `n`:
///
/// `ln(eps_dot) = [ ln A - p ln d + r ln f - (E* + P V*)/(R T) ] + n * ln(sigma)`
///
/// The bracket is this function. Both directions follow from it in one line each (the forward rate adds
/// `n * ln sigma`, the single-row inverse subtracts and divides by `n`), so the units are converted in ONE
/// place and both directions are refereed by the same twin.
///
/// THE UNITS, READ FROM THE PRIMARY RATHER THAN INFERRED (H&K Table 1 footnotes a, c and e; the header prints
/// NO unit for `A` at all, and footnotes c and e are the source's own dimensional key, being the only place it
/// enters numbers into its own law):
///
/// - stress in MPa, water fugacity in MPa, water content in H/10^6 Si, grain size in MICROMETRES, T in K.
/// - `E*` ENTERS IN J/mol WHILE TABLE 1 HEADS IT IN kJ/mol. This is the trap, and it is the source's own: the
///   column reads `530` and the worked examples enter `335000` and `480000`. The row banks the header's units
///   verbatim (per the R-UNITS-PIN discipline: a measured row carries its SOURCE's units and converts once), so
///   the kJ-to-J conversion happens HERE, exactly once, at the one boundary where the value crosses into the
///   exponent. Applied twice or never, it is a silent 1000x inside an exponential.
/// - `P V*` MUST LAND IN J/mol, which is what fixes the pressure pairing. The source pairs Pa with m^3/mol; this
///   module banks GPa with cm^3/mol, and `1 MPa * 1 cm^3/mol = 1e6 Pa * 1e-6 m^3/mol = 1 J/mol` exactly, so
///   `P[GPa] * 1000 -> P[MPa]`, then `P[MPa] * V*[cm^3/mol] -> J/mol`. The multiply is ordered that way rather
///   than through the source's own `1e9 * 11e-6` because `1e9 * 11` overflows Q32.32 outright, so the source's
///   own arithmetic is not performable in this type in the source's own order.
fn ln_rate_intercept(
    row: &CreepRow,
    volume_cm3_per_mol: Fixed,
    conditions: &CreepConditions,
) -> Result<Fixed, CreepRefusal> {
    if conditions.temperature_k <= Fixed::ZERO {
        return Err(CreepRefusal::ConditionOutOfDomain);
    }
    if !row.accepts_water(conditions.water) {
        return Err(CreepRefusal::WaterFrameMismatch);
    }

    // ln A, banked in log space because H&K's prefactors span 90 to 4.7e10 and overflow Q32.32's ~2.1e9 ceiling.
    let mut acc = row.ln_prefactor;

    // - p * ln(d), with d in micrometres. A p = 0 row drops the term ENTIRELY rather than multiplying ln(d) by
    // zero: d^0 = 1 for any d, so a dislocation row must not care whether a grain size was supplied, and
    // computing ln(d) first would turn a missing or zero d into the Fixed::MIN sentinel times zero.
    if row.grain_size_exponent != 0 {
        let d = conditions
            .grain_size_um
            .ok_or(CreepRefusal::GrainSizeMissing)?;
        if d <= Fixed::ZERO {
            return Err(CreepRefusal::GrainSizeMissing);
        }
        let term = d
            .ln()
            .checked_mul(Fixed::from_int(row.grain_size_exponent))
            .ok_or(CreepRefusal::NotRepresentable)?;
        acc = acc
            .checked_sub(term)
            .ok_or(CreepRefusal::NotRepresentable)?;
    }

    // + r * ln(f_H2O). A dry row has no water exponent, and accepts_water above already proved the frame
    // matches, so the pair is consistent by construction here.
    if let Some(r) = row.water_exponent {
        let water = match conditions.water {
            Some(WaterContent::Fugacity { mpa }) => mpa,
            Some(WaterContent::Hydroxyl { h_per_10e6_si }) => h_per_10e6_si,
            None => return Err(CreepRefusal::WaterFrameMismatch),
        };
        if water <= Fixed::ZERO {
            return Err(CreepRefusal::ConditionOutOfDomain);
        }
        let term = water
            .ln()
            .checked_mul(r)
            .ok_or(CreepRefusal::NotRepresentable)?;
        acc = acc
            .checked_add(term)
            .ok_or(CreepRefusal::NotRepresentable)?;
    }

    // - (E* + P V*) / (R T). THE ONE CONVERSION BOUNDARY, per the doc above.
    let e_star_j_per_mol = row
        .activation_energy
        .value
        .checked_mul(Fixed::from_int(1000))
        .ok_or(CreepRefusal::NotRepresentable)?;
    let pressure_mpa = conditions
        .pressure_gpa
        .checked_mul(Fixed::from_int(1000))
        .ok_or(CreepRefusal::NotRepresentable)?;
    let pv_j_per_mol = pressure_mpa
        .checked_mul(volume_cm3_per_mol)
        .ok_or(CreepRefusal::NotRepresentable)?;
    let numerator = e_star_j_per_mol
        .checked_add(pv_j_per_mol)
        .ok_or(CreepRefusal::NotRepresentable)?;
    let rt = molar_gas_constant()
        .checked_mul(conditions.temperature_k)
        .ok_or(CreepRefusal::NotRepresentable)?;
    let arrhenius = numerator
        .checked_div(rt)
        .ok_or(CreepRefusal::NotRepresentable)?;
    acc.checked_sub(arrhenius)
        .ok_or(CreepRefusal::NotRepresentable)
}

/// THE FORWARD LAW for ONE row: `ln(eps_dot)` at a trial stress. The composite's inner loop, and the surface
/// the worked-example twin referees.
///
/// It evaluates whatever row it is handed, including the wet ones, because THE LAW IS GENERAL EVEN WHERE THE
/// ENGINE'S INPUTS ARE NOT. What the engine can feed is [`admit_candidate`]'s question, and it is asked at the
/// composite's boundary; what the law says is this function's, and narrowing it to today's admitted set would
/// bake an engine limitation into a citation.
fn ln_row_strain_rate(
    row: &CreepRow,
    volume_cm3_per_mol: Fixed,
    conditions: &CreepConditions,
    ln_stress_mpa: Fixed,
) -> Result<Fixed, CreepRefusal> {
    let intercept = ln_rate_intercept(row, volume_cm3_per_mol, conditions)?;
    let slope = ln_stress_mpa
        .checked_mul(row.stress_exponent)
        .ok_or(CreepRefusal::NotRepresentable)?;
    intercept
        .checked_add(slope)
        .ok_or(CreepRefusal::NotRepresentable)
}

/// THE SINGLE-ROW INVERSE in log space: the `ln(sigma)` at which this row alone delivers a strain rate.
///
/// Exact (the single row's log form is linear in `ln sigma`, so this is algebra rather than a solve). It is not
/// the composite's answer, and it is the composite's BRACKET: see [`ductile_strength_mpa`].
fn ln_row_stress_mpa(
    row: &CreepRow,
    volume_cm3_per_mol: Fixed,
    conditions: &CreepConditions,
    ln_strain_rate_per_s: Fixed,
) -> Result<Fixed, CreepRefusal> {
    let intercept = ln_rate_intercept(row, volume_cm3_per_mol, conditions)?;
    if row.stress_exponent <= Fixed::ZERO {
        return Err(CreepRefusal::ConditionOutOfDomain);
    }
    ln_strain_rate_per_s
        .checked_sub(intercept)
        .ok_or(CreepRefusal::NotRepresentable)?
        .checked_div(row.stress_exponent)
        .ok_or(CreepRefusal::NotRepresentable)
}

/// The number of halvings the composite's bisection takes.
///
/// AN ENGINE-CONVERGENCE BOUND, NOT WORLD CONTENT, and the same count and the same reasoning as `melting.rs`'s
/// eutectic bisection. The bracket is DERIVED (see [`ductile_strength_mpa`]) and its width is at most
/// `ln(row count) / min(n)`, which is of order one; Q32.32 resolves 2^-32, about 2.3e-10; so 52 halvings drive
/// the interval to about 2e-15 in log space, well below the floor the type can represent, and further steps
/// change no bit. The count therefore cannot move a result: it is chosen past the point where it can.
const COMPOSITE_BISECTION_STEPS: u32 = 52;

/// `ln(sum of exp(x_i))` over the admitted rows' log-space rates: the parallel sum, in the only representable
/// domain. It CONSUMES the banked pairwise primitive [`crate::saha::log_sum_exp`] rather than computing the
/// shift itself.
///
/// # A CENSUS FOUND A DIAMOND HERE, WHICH IS WHY THIS FOLDS INSTEAD OF COMPUTING
///
/// This workspace already carries TWO implementations of `ln(sum exp(x_i))`, under two names, in two crates:
/// [`crate::saha::log_sum_exp`] (PAIRWISE, `hi + ln(1 + exp(lo - hi))`, public, this crate) and
/// `civsim_materials::creep::logsumexp_canonical` (N-ARY, terms sorted ascending, private, and in a crate that
/// depends on this one so it cannot be called from here at all). They compute ONE quantity by different
/// constructions and cannot agree bit for bit: a pairwise fold rounds at every step where a sorted n-ary sum
/// rounds once. Both docs invoke the project's determinism discipline BY NAME, one as "the canonical-logsumexp
/// determinism rule" and the other as "rider 1c, the fixed-topology-reduction discipline": one named rule, two
/// doors. Writing a third here would have been the worst available outcome, so this consumes the one it can
/// reach. The unification is a lane-crossing refactor with a real byte risk (moving Saha from a fold to a sorted
/// n-ary reduction could move the pins) and is sequenced elsewhere.
///
/// # THE ORDER IS A PROPERTY OF THIS CODE, NEVER OF THE CALLER'S SLICE
///
/// A pairwise fold is not exactly associative in fixed point, so the terms are SORTED ASCENDING before folding.
/// That normalizes every permutation of the same multiset onto one sequence, which makes the result independent
/// of the order a caller happened to list its candidates in (Principle 3) rather than merely deterministic given
/// one, and it folds the smallest terms first, where a fold loses the least. The sort is total and on the values
/// themselves, so ties are indistinguishable by construction.
///
/// A lid's strain rate is 1e-15 per second and Q32.32's smallest positive value is about 2.3e-10, so THE RATES
/// CANNOT BE ADDED DIRECTLY: every term is zero in this type and their sum is zero. A mechanism more than about
/// 22 in log below the fastest underflows to zero inside the primitive and contributes nothing, which is the
/// type's floor rather than a modelling choice: a mechanism running 1e9 times slower than its neighbour cannot
/// move the sum at this resolution anyway.
fn ln_total_strain_rate(terms: &mut [Fixed]) -> Option<Fixed> {
    terms.sort();
    terms.iter().copied().reduce(crate::saha::log_sum_exp)
}

/// THE DUCTILE BRANCH OF THE YIELD-STRENGTH ENVELOPE: the differential stress (MEGAPASCALS) a lid sustains at
/// the load's own strain rate, over the creep mechanisms this engine can feed, acting IN PARALLEL.
///
/// # THE MECHANISMS ARE IN PARALLEL, SO THE RATES ADD AND THE STRENGTH IS A SOLVE
///
/// At one stress every mechanism runs at once and their strain rates SUM:
///
/// `eps_dot_total(sigma) = SUM over admitted rows of [ A_i sigma^n_i d^-p_i f^r_i exp(-(E*_i + P V*_i)/(R T)) ]`
///
/// and the strength is the `sigma` delivering the requested total. A sum of power laws with differing `n` has
/// NO closed form, so this is a monotone numeric solve rather than algebra. WHICH MECHANISM DOMINATES IS AN
/// OUTPUT of that solve and never an input to it: picking a dominant row to get a closed form would author the
/// answer to the question the composite exists to ask, and it would author it invisibly, since the picked row
/// is right over most of the domain and wrong exactly at the crossover the envelope cares about.
///
/// The composite may admit ONE row today. The form is still the composite: a composite of one is correct and
/// reduces to the exact closed form by construction (with one row the bracket below collapses to a point and
/// the bisection returns it without stepping), whereas a single-row special case would have to be torn out when
/// the second row arrives.
///
/// # THE BRACKET IS DERIVED FROM THE ROWS THEMSELVES, NOT AUTHORED
///
/// A bisection needs a bracket, and an authored one would be a scalar hiding in a solver. Both ends fall out of
/// the single-row inverses, which are exact:
///
/// - UPPER. Every term is positive, so `eps_dot_total(sigma) >= eps_dot_i(sigma)` for each row. At
///   `sigma_hi = min over i of sigma_i*` (each `sigma_i*` being row `i` alone delivering the target), the
///   argmin row alone already delivers the target and the rest add to it, so the total is at or above target
///   and the answer is at or below `sigma_hi`.
/// - LOWER. `sigma_lo = min over i of sigma_i**`, where `sigma_i**` is row `i` alone delivering `target / N`
///   for `N` admitted rows. At `sigma_lo` every row delivers at most `target / N`, so the total is at most
///   target, and the answer is at or above `sigma_lo`.
///
/// With `N = 1` the two ends coincide (`ln(1) = 0`), which is why the composite of one is exact rather than
/// approximate.
///
/// # WHAT IS REFEREED AND WHAT IS NOT
///
/// The LAW and its unit conversions are refereed against H&K's OWN two worked examples, which are the only
/// numbers the source ever put through its own equation (see `ln_rate_intercept` and the twin tests). The
/// COMPOSITE SUM IS NOT REFEREED BY ANY SOURCE: H&K work single rows only, so the parallel sum rests on the
/// derived bracket, the monotonicity above, and this module's own tests. That is a real gap and it is named
/// rather than covered.
///
/// # THE `V*` END IS AN EXPLICIT ARGUMENT, WHICH IS THE BRACKET REACHING THE CALL SITE
///
/// `V*` is a span rather than a value ([`ActivationVolumeBracket`]), so a strength is a span too, and this
/// evaluates ONE END of it per call. The end is named by the caller because there is no end to default to: the
/// answer at the low end is the weakest the source's own table permits and the answer at the high end the
/// strongest, and collapsing them here would author the number the primary declines to choose.
///
/// The composite is MONOTONE IN `V*` at a non-negative pressure (a larger `V*` raises `E* + P V*`, which lowers
/// every row's rate at a given stress and therefore raises the stress the target rate needs), so the two ends
/// bracket the truth rather than merely differing from it. At zero pressure the `P V*` term vanishes and the
/// two ends return the same bits, which is the span costing nothing exactly where the source constrains it
/// least.
pub fn ductile_strength_mpa(
    candidates: &[CreepCandidate<'_>],
    conditions: CreepConditions,
    end: VolumeEnd,
) -> Result<Fixed, CreepRefusal> {
    if conditions.temperature_k <= Fixed::ZERO {
        return Err(CreepRefusal::ConditionOutOfDomain);
    }

    // ADMIT, then take each admitted row's exact single-row inverse at the named end of its own `V*` bracket.
    // The bracket below is built from MINIMA and the sum from a SORTED fold ([`ln_total_strain_rate`]), so
    // neither reads the caller's slice order: the result is a pure function of the candidate SET (Principle 3),
    // never of how it was arranged.
    let mut admitted: Vec<(CreepRow, Fixed)> = Vec::new();
    for candidate in candidates {
        if let Ok(volume) = admit_candidate(candidate, conditions.pressure_gpa) {
            admitted.push((candidate.row, volume.at(end)));
        }
    }
    if admitted.is_empty() {
        return Err(CreepRefusal::NoAdmittedRow);
    }

    let ln_count = Fixed::from_int(i32::try_from(admitted.len()).unwrap_or(i32::MAX)).ln();
    let ln_target_shared = conditions
        .ln_strain_rate_per_s
        .checked_sub(ln_count)
        .ok_or(CreepRefusal::NotRepresentable)?;

    let mut ln_hi: Option<Fixed> = None;
    let mut ln_lo: Option<Fixed> = None;
    for (row, volume) in &admitted {
        let hi = ln_row_stress_mpa(row, *volume, &conditions, conditions.ln_strain_rate_per_s)?;
        let lo = ln_row_stress_mpa(row, *volume, &conditions, ln_target_shared)?;
        ln_hi = Some(match ln_hi {
            Some(current) if current <= hi => current,
            _ => hi,
        });
        ln_lo = Some(match ln_lo {
            Some(current) if current <= lo => current,
            _ => lo,
        });
    }
    let mut lo = ln_lo.ok_or(CreepRefusal::NoAdmittedRow)?;
    let mut hi = ln_hi.ok_or(CreepRefusal::NoAdmittedRow)?;

    // BISECT. The composite is strictly increasing in ln(sigma) (every term has slope n_i > 0), so the sign of
    // the residual says which half holds the root, and the interval halves every step.
    for _ in 0..COMPOSITE_BISECTION_STEPS {
        if hi <= lo {
            break;
        }
        let mid = lo
            .checked_add(hi)
            .ok_or(CreepRefusal::NotRepresentable)?
            .checked_div(Fixed::from_int(2))
            .ok_or(CreepRefusal::NotRepresentable)?;
        let mut terms: Vec<Fixed> = Vec::with_capacity(admitted.len());
        for (row, volume) in &admitted {
            terms.push(ln_row_strain_rate(row, *volume, &conditions, mid)?);
        }
        let total = ln_total_strain_rate(&mut terms).ok_or(CreepRefusal::NotRepresentable)?;
        if total < conditions.ln_strain_rate_per_s {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let ln_stress_mpa = lo;

    // THE REPRESENTABILITY GUARD, off the TYPE rather than an authored window: `Fixed::MAX.ln()` and
    // `Fixed::EPSILON.ln()` are the exact log-space edges of what Q32.32 can hold, so the bound is read from
    // the representation instead of chosen near it.
    if ln_stress_mpa > Fixed::MAX.ln() || ln_stress_mpa < Fixed::EPSILON.ln() {
        return Err(CreepRefusal::StressNotRepresentable { ln_stress_mpa });
    }
    Ok(ln_stress_mpa.exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// H&K's footnote c and footnote e are the ONLY numbers the source ever puts through its own equation, and
    /// they are this module's referee. Each fixture below is the footnote's own stated input, transcribed and
    /// nothing else.
    ///
    /// THE `V*` CHORD IS DEGENERATE ON PURPOSE. Table 1 prints these two `V*` (4 and 11) BARE: no band, no
    /// pressure interval, unlike the fugacity row's `22 +/- 11`. The one pressure the source states for them is
    /// the `P = 1 GPa` of its own worked example, so the chord here is `[1, 1]` GPa, the degenerate interval at
    /// that single point. It makes the fixture usable exactly where the source used it and refused everywhere
    /// else. Any wider interval would be invented, which is the defect the interval tagging exists to prevent.
    ///
    /// THE MODALITY TAG IS THE FIXTURE'S, NOT A READING OF THE TABLE. `Fitted` here is set so the exponent gate
    /// admits and the arithmetic under test is what the test exercises. What a bare `V*` column means in
    /// modality terms is a real open question (the module's own reasoning elsewhere treats a missing band as an
    /// `Assumed` signature, which would make these `Assumed`), and this test does not answer it. It does not
    /// have to: no shipping row reads these, because neither row is in the admitted set.
    fn footnote_c_wet_diffusion() -> (CreepRow, ActivationVolume, CreepConditions) {
        (
            hk_wet_diffusion_hydroxyl(),
            ActivationVolume {
                cm3_per_mol: Fixed::from_int(4),
                interval_min_gpa: Fixed::ONE,
                interval_max_gpa: Fixed::ONE,
                modality: Modality::Fitted,
            },
            CreepConditions {
                // "= 7.8x10^-15 /s", the source's own printed answer.
                ln_strain_rate_per_s: ln_scientific(78, 10, -15),
                temperature_k: Fixed::from_int(1673), // "T = 1400 C"
                pressure_gpa: Fixed::ONE,             // "P = 1 GPa"
                grain_size_um: Some(Fixed::from_int(10_000)), // "d = 10 mm", entered as (10,000)^-3
                water: Some(WaterContent::Hydroxyl {
                    h_per_10e6_si: Fixed::from_int(1000), // "C_OH = 1000 H/10^6 Si"
                }),
            },
        )
    }

    fn footnote_e_wet_dislocation() -> (CreepRow, ActivationVolume, CreepConditions) {
        (
            hk_wet_dislocation_hydroxyl(),
            ActivationVolume {
                cm3_per_mol: Fixed::from_int(11),
                interval_min_gpa: Fixed::ONE,
                interval_max_gpa: Fixed::ONE,
                modality: Modality::Fitted,
            },
            CreepConditions {
                // "= 2.5x10^-12 /s", the source's own printed answer.
                ln_strain_rate_per_s: ln_scientific(25, 10, -12),
                temperature_k: Fixed::from_int(1673),
                pressure_gpa: Fixed::ONE,
                grain_size_um: None, // p = 0: the row is grain-size independent and the footnote states no d.
                water: Some(WaterContent::Hydroxyl {
                    h_per_10e6_si: Fixed::from_int(1000),
                }),
            },
        )
    }

    /// One of H&K Table 2's nine determinations (Karato and Jung 2002, `V* = 14` over 0.3 to 2 GPa), used as a
    /// FIXTURE for the dry dislocation row, which Table 1 does not print a `V*` for at all.
    ///
    /// THIS SLICE BANKS NO `V*` DETERMINATION and this is not a selection it endorses. Table 2 offers nine
    /// values from -2 to 27, they fail to overlap because `V*` is a chord that decreases with pressure, and
    /// picking one is a decision the primary declines to make. The tests below assert RELATIONS (the composite
    /// of one is exact, the composite of two is weaker than either alone, the strength rises with rate), none
    /// of which this number can move.
    fn table2_dry_dislocation_volume_fixture() -> ActivationVolume {
        ActivationVolume {
            cm3_per_mol: Fixed::from_int(14),
            interval_min_gpa: Fixed::from_ratio(3, 10),
            interval_max_gpa: Fixed::from_int(2),
            modality: Modality::Fitted,
        }
    }

    #[test]
    fn the_worked_examples_referee_the_law_itself() {
        // THE REFEREE, and what makes it one: 7.8e-15 and 2.5e-12 are H&K's OWN numbers, computed by H&K,
        // printed in H&K's own footnotes from H&K's own stated inputs. They came from outside this codebase
        // entirely, so reproducing them is not this module agreeing with itself. Every unit decision in
        // `ln_rate_intercept` is on trial here: the kJ-to-J conversion on E* (the silent 1000x), the
        // GPa-to-MPa-times-cm^3 pairing that lands P*V* in J/mol, micrometres of grain size, the log-space
        // prefactor, and the sign of the Arrhenius term.
        //
        // THE TOLERANCE IS THE SOURCE'S OWN PRECISION. H&K print their rates to TWO significant figures, so
        // "7.8e-15" is the true value to within about half a unit in the last place, or 0.6 percent, which is
        // 0.006 in log space. The bound is 0.02, three times that, leaving room for the fixed-point ln and exp.
        // It still convicts what it is for: a unit-class error moves the log by 6.9 PER DECADE, and reading E*
        // in kJ/mol instead of J/mol moves footnote e's exponent from 35.3 to 0.035, that is by 34 in log space.
        let ln_tolerance = Fixed::from_ratio(2, 100);
        let ln_sigma = ln_scientific(3, 10, 0); // "sigma = 0.3 MPa", the footnotes' stated stress.

        let (row, volume, conditions) = footnote_c_wet_diffusion();
        let ln_rate = ln_row_strain_rate(&row, volume.cm3_per_mol, &conditions, ln_sigma)
            .expect("footnote c's own inputs must evaluate");
        assert!(
            (ln_rate - conditions.ln_strain_rate_per_s).abs() < ln_tolerance,
            "footnote c: H&K compute 7.8e-15 /s from their own inputs; we get ln = {:?} against ln(7.8e-15) = {:?}",
            ln_rate,
            conditions.ln_strain_rate_per_s
        );

        let (row, volume, conditions) = footnote_e_wet_dislocation();
        let ln_rate = ln_row_strain_rate(&row, volume.cm3_per_mol, &conditions, ln_sigma)
            .expect("footnote e's own inputs must evaluate");
        assert!(
            (ln_rate - conditions.ln_strain_rate_per_s).abs() < ln_tolerance,
            "footnote e: H&K compute 2.5e-12 /s from their own inputs; we get ln = {:?} against ln(2.5e-12) = {:?}",
            ln_rate,
            conditions.ln_strain_rate_per_s
        );
    }

    #[test]
    fn the_worked_examples_referee_the_inversion_against_the_sources_own_stress() {
        // THE REFEREE IN THE DIRECTION THIS MODULE SHIPS. The forward test above proves the law; this
        // one proves the INVERSE, which is what `ductile_strength_mpa` computes. Feed H&K's own printed rate
        // back into the row and their own stated stress must come out: they say sigma = 0.3 MPa, and nothing in
        // this codebase told them so.
        //
        // TOLERANCE 1 percent, from the same two-significant-figure printing, divided by n: the log-space
        // rounding in the rate enters the stress as rounding/n, which is 0.08 percent for footnote c (n = 1) and
        // 0.29 percent for footnote e (n = 3.5). One percent clears both with margin and stays far below the 25
        // to 33 percent a doubled P*V* would move the answer, so it still convicts.
        let target = Fixed::from_ratio(3, 10);
        let tolerance = Fixed::from_ratio(3, 1000);

        for (label, (row, volume, conditions)) in [
            ("footnote c", footnote_c_wet_diffusion()),
            ("footnote e", footnote_e_wet_dislocation()),
        ] {
            let ln_sigma = ln_row_stress_mpa(
                &row,
                volume.cm3_per_mol,
                &conditions,
                conditions.ln_strain_rate_per_s,
            )
            .expect("the source's own worked example must invert");
            let sigma = ln_sigma.exp();
            assert!(
                (sigma - target).abs() < tolerance,
                "{label}: H&K state sigma = 0.3 MPa for this rate; the inversion returns {}",
                sigma.to_f64_lossy()
            );
        }
    }

    #[test]
    fn the_admission_gate_names_which_input_the_engine_cannot_derive() {
        // THE VALUE LINE AT THE WIDEST POINT. The alternative to refusing a wet row is picking a fugacity, and
        // there is no fugacity to pick, only one to invent. Each refusal below names the input the engine lacks,
        // so a test cannot pass because the wrong gate fired.
        let volumes = [table2_dry_dislocation_volume_fixture()];
        let p = Fixed::ONE;

        // The wet rows are cited, correct, and unfeedable: no water substrate exists.
        for row in [hk_wet_dislocation_fugacity(), hk_wet_dislocation_hydroxyl()] {
            assert_eq!(
                admit_candidate(
                    &CreepCandidate {
                        row,
                        volumes: &volumes
                    },
                    p
                )
                .err(),
                Some(CreepRefusal::WaterNotDerived),
                "a wet row must be refused for the input the engine cannot derive, by name"
            );
        }
        // The wet diffusion row is unfeedable twice over; water is checked first, so water is what it reports.
        assert_eq!(
            admit_candidate(
                &CreepCandidate {
                    row: hk_wet_diffusion_hydroxyl(),
                    volumes: &volumes
                },
                p
            )
            .err(),
            Some(CreepRefusal::WaterNotDerived)
        );
        // THE GBS ROW IS REFUSED, AND THAT IS THE GATE WORKING, NOT A WORKAROUND. It is dry, so it clears the
        // water gate, and it dies on the grain-size one (p = 2, and no grain size is derived). Its activation
        // energy is ASSUMED besides (H&K's footnote f: transferred from easy-slip data, never fitted to GBS),
        // which the exponent gate refuses independently, proven below.
        assert_eq!(
            admit_candidate(
                &CreepCandidate {
                    row: hk_dry_gbs_below_1250c(),
                    volumes: &volumes
                },
                p
            )
            .err(),
            Some(CreepRefusal::GrainSizeNotDerived)
        );
        // The exponent gate convicts the GBS row on its own, with the grain-size question taken out of the way,
        // so the two refusals are independent rather than one hiding behind the other.
        let mut gbs_p0 = hk_dry_gbs_below_1250c();
        gbs_p0.grain_size_exponent = 0;
        assert_eq!(
            admit_candidate(&CreepCandidate { row: gbs_p0, volumes: &volumes }, p).err(),
            Some(CreepRefusal::ExponentGrade),
            "an ASSUMED activation energy must not cross into an Arrhenius exponent without escalation"
        );

        // THE ADMITTED SET TODAY IS EXACTLY ONE ROW: dry, p = 0, fitted. That is dislocation creep.
        assert!(admit_candidate(
            &CreepCandidate {
                row: hk_dry_dislocation(),
                volumes: &volumes
            },
            p
        )
        .is_ok());

        // OUTSIDE EVERY CHORD THE TABLE STILL SUPPORTS ITS OWN EXTREMES, tagged so the consumer knows the source
        // said nothing here. This is the ruling rather than a loosening: refusing instead blocked a lid from
        // being sampled at its own SURFACE, since the banked chords start at 0.3 GPa (about nine kilometres
        // down), and the consumer proves for itself that the span cannot reach its answer there. The fixture's
        // chord stops at 2 GPa, so 9 GPa is outside it.
        let far = admit_candidate(
            &CreepCandidate {
                row: hk_dry_dislocation(),
                volumes: &volumes,
            },
            Fixed::from_int(9),
        )
        .expect("the table supports its own extremes even where no chord reaches");
        assert_eq!(
            far.constraint(),
            VolumeConstraint::UnconstrainedBySource,
            "outside every chord the source constrains nothing, and the bracket must say so"
        );
        // AND INSIDE ONE IT IS TAGGED THE OTHER WAY, which is what makes the tag a claim rather than a constant.
        assert_eq!(
            admit_candidate(
                &CreepCandidate {
                    row: hk_dry_dislocation(),
                    volumes: &volumes
                },
                p
            )
            .expect("covered")
            .constraint(),
            VolumeConstraint::CoveredBySource
        );
        // AN EMPTY TABLE IS THE ONE CASE WITH NOTHING TO REPORT, and it is a different answer from a table whose
        // chords merely miss. No determination, no span, no exponent.
        assert_eq!(
            admit_candidate(
                &CreepCandidate {
                    row: hk_dry_dislocation(),
                    volumes: &[]
                },
                p
            )
            .err(),
            Some(CreepRefusal::NoActivationVolumeBanked)
        );
    }

    #[test]
    fn the_composite_of_one_is_exact_and_reduces_to_the_single_row_inverse() {
        // A COMPOSITE OF ONE IS CORRECT, not a special case waiting to be torn out. With one admitted row the
        // derived bracket collapses to a point (ln(1) = 0, so the upper and lower ends coincide) and the
        // bisection returns it without stepping, which makes the answer the EXACT closed-form inverse rather
        // than a converged approximation. Asserted to the bit.
        let volumes = [table2_dry_dislocation_volume_fixture()];
        let row = hk_dry_dislocation();
        let conditions = CreepConditions {
            ln_strain_rate_per_s: ln_scientific(1, 1, -12),
            temperature_k: Fixed::from_int(1673),
            pressure_gpa: Fixed::ONE,
            grain_size_um: None,
            water: None,
        };
        let composite = ductile_strength_mpa(
            &[CreepCandidate {
                row,
                volumes: &volumes,
            }],
            conditions,
            VolumeEnd::Low,
        )
        .expect("the one admitted row resolves");
        let single = ln_row_stress_mpa(
            &row,
            volumes[0].cm3_per_mol,
            &conditions,
            conditions.ln_strain_rate_per_s,
        )
        .expect("the single-row inverse resolves")
        .exp();
        assert_eq!(
            composite, single,
            "a composite of one must BE the single-row inverse, to the bit"
        );

        // And with no candidate admitted there is no composite, rather than a fabricated strength.
        assert_eq!(
            ductile_strength_mpa(
                &[CreepCandidate {
                    row: hk_wet_dislocation_fugacity(),
                    volumes: &volumes
                }],
                conditions,
                VolumeEnd::Low
            ),
            Err(CreepRefusal::NoAdmittedRow)
        );
    }

    /// The synthetic second mechanism the composite tests need, in one place.
    ///
    /// IT CARRIES NO CITATION, DESCRIBES NO EXPERIMENT, AND SHIPS NOWHERE. It exists because a parallel sum
    /// needs TWO admitted rows to be a sum at all, and today's admitted set has exactly one member: H&K's dry
    /// rows are one dislocation row plus two GBS rows the gates refuse. It is the dry dislocation row with its
    /// stress exponent moved to 1, so it is transparently a perturbation of a real row rather than a claim about
    /// any material, and the two rows then share an intercept exactly, which is what lets the composite's root
    /// be reasoned about in closed form.
    fn synthetic_linear_second_mechanism() -> CreepRow {
        let mut row = hk_dry_dislocation();
        row.stress_exponent = Fixed::ONE;
        row
    }

    #[test]
    fn the_composite_solves_its_own_equation_rather_than_returning_a_bracket() {
        // THE TEST THIS SUITE'S FIRST DRAFT DID NOT HAVE, AND THE REASON IT IS HERE, RECORDED RATHER THAN
        // QUIETLY FIXED. The parallel-property test below asserts the composite lands below both single-row
        // answers. That is TRUE OF THE DERIVED LOWER BRACKET BY CONSTRUCTION, so it passed with the bisection
        // deleted outright (COMPOSITE_BISECTION_STEPS = 0). It proved the bracket and never the solve, and a
        // mutation run is what said so rather than a review.
        //
        // THE DEFINING EQUATION IS THE PIN: at the returned stress, the admitted mechanisms' rates must SUM to
        // the rate that was asked for. Nothing weaker distinguishes a converged root from a bracket endpoint.
        //
        // NOT CIRCULAR: the forward law being summed here is refereed INDEPENDENTLY, against H&K's own two
        // worked examples. This checks only that the solver found the root of that refereed law, which is the
        // one thing a solver is for.
        let volumes = [table2_dry_dislocation_volume_fixture()];
        let conditions = CreepConditions {
            ln_strain_rate_per_s: ln_scientific(1, 1, -12),
            temperature_k: Fixed::from_int(1673),
            pressure_gpa: Fixed::ONE,
            grain_size_um: None,
            water: None,
        };
        let rows = [hk_dry_dislocation(), synthetic_linear_second_mechanism()];
        let candidates: Vec<CreepCandidate<'_>> = rows
            .iter()
            .map(|row| CreepCandidate {
                row: *row,
                volumes: &volumes,
            })
            .collect();

        let sigma = ductile_strength_mpa(&candidates, conditions, VolumeEnd::Low)
            .expect("the composite resolves");
        let ln_sigma = sigma.ln();

        // Re-sum the mechanisms at the answer and demand the target back.
        let terms: Vec<Fixed> = rows
            .iter()
            .map(|row| {
                ln_row_strain_rate(row, volumes[0].cm3_per_mol, &conditions, ln_sigma)
                    .expect("evaluates")
            })
            .collect();
        let mut terms = terms;
        let total = ln_total_strain_rate(&mut terms).expect("the rates sum");

        // The bisection converges to the Q32.32 floor, so the residual is the type's noise (about 1e-9 in log
        // space) rather than a modelling tolerance. The bound is 1e-4, five orders above that noise and four
        // orders below the ln(2) = 0.69 that returning the lower bracket would leave behind.
        assert!(
            (total - conditions.ln_strain_rate_per_s).abs() < Fixed::from_ratio(1, 10_000),
            "the composite must deliver the rate it was asked for: asked ln {:?}, the answer's rates sum to ln {:?}",
            conditions.ln_strain_rate_per_s,
            total
        );
    }

    #[test]
    fn the_sum_is_a_sum_pinned_by_a_root_this_code_cannot_compute() {
        // THE RESIDUAL TEST ABOVE HAS ONE BLIND SPOT, AND IT IS THE WHOLE SUM. It re-sums with `ln_sum_exp`, the
        // same function the solver sums with, so a broken sum satisfies it: mutate `ln_sum_exp` to return only
        // its LARGEST term (which is the "pick the dominant mechanism" defect, the exact shortcut the composite
        // exists to refuse) and the residual test goes green, because the solver and the check agree on the same
        // wrong sum. A check that consumes the routine it is checking referees nothing.
        //
        // SO THE ROOT IS PINNED FROM OUTSIDE. Both rows here share an intercept `I` exactly (they differ only in
        // `n`), so the composite is `exp(I) * (sigma^3.5 + sigma)` and its root is available in closed form. At
        // sigma = 2 the bracket is `2^3.5 + 2 = 13.313708...`, an arithmetic identity computed OUTSIDE this
        // codebase and typed here as a literal, the same twin discipline the prefactor pins already use. Ask the
        // composite for that rate and it must return 2.
        //
        // WHAT IT DISCRIMINATES: the max-only sum returns 2.095 (it solves sigma^3.5 = 13.31 instead), the
        // deleted bisection returns 1.719 (the lower bracket), and both are orders outside the bound below. The
        // manufactured target is back-solved from the root ON PURPOSE and this test therefore proves the SOLVE
        // and never the physics; the physics is refereed by H&K's worked examples, which are back-solved from
        // nothing.
        let volumes = [table2_dry_dislocation_volume_fixture()];
        let rows = [hk_dry_dislocation(), synthetic_linear_second_mechanism()];
        let mut conditions = CreepConditions {
            ln_strain_rate_per_s: Fixed::ZERO, // replaced below, once the intercept is in hand
            temperature_k: Fixed::from_int(1673),
            pressure_gpa: Fixed::ONE,
            grain_size_um: None,
            water: None,
        };
        let intercept =
            ln_rate_intercept(&rows[0], volumes[0].cm3_per_mol, &conditions).expect("resolves");
        assert_eq!(
            intercept,
            ln_rate_intercept(&rows[1], volumes[0].cm3_per_mol, &conditions).expect("resolves"),
            "the two rows must share an intercept exactly, or the closed-form root below is not the root"
        );
        // ln( exp(I) * (2^3.5 + 2) ) = I + ln(13.313708...)
        conditions.ln_strain_rate_per_s = intercept + Fixed::from_ratio(13_313_708, 1_000_000).ln();

        let candidates: Vec<CreepCandidate<'_>> = rows
            .iter()
            .map(|row| CreepCandidate {
                row: *row,
                volumes: &volumes,
            })
            .collect();
        let sigma = ductile_strength_mpa(&candidates, conditions, VolumeEnd::Low)
            .expect("the composite resolves");
        assert!(
            (sigma - Fixed::from_int(2)).abs() < Fixed::from_ratio(2, 1000),
            "the composite's root is 2 MPa by construction (2^3.5 + 2 = 13.3137); it returned {}",
            sigma.to_f64_lossy()
        );
    }

    #[test]
    fn the_parallel_composite_is_weaker_than_either_mechanism_alone() {
        // THE PROPERTY THAT MAKES THE SUM A SUM. Mechanisms act in PARALLEL: at one stress they each deliver a
        // strain rate and the rates ADD, so two mechanisms reach a given rate at a LOWER stress than either
        // needs alone. If the composite ever returned the min of the single-row answers, or the answer of a
        // hand-picked dominant row, this test would catch it, because both of those are strictly higher.
        //
        // THIS TEST STATES THE PHYSICS AND CANNOT POLICE THE SOLVE: landing below both singles is a property the
        // derived lower bracket already has, so a mutation that deletes the bisection passes here. That is not a
        // hole, because `the_composite_solves_its_own_equation_rather_than_returning_a_bracket` pins the root;
        // it is a division of labour, and it is written down because a reader would otherwise take this test for
        // a guard it is not.
        let synthetic_linear = synthetic_linear_second_mechanism();
        let volumes = [table2_dry_dislocation_volume_fixture()];
        let conditions = CreepConditions {
            ln_strain_rate_per_s: ln_scientific(1, 1, -12),
            temperature_k: Fixed::from_int(1673),
            pressure_gpa: Fixed::ONE,
            grain_size_um: None,
            water: None,
        };

        let real = CreepCandidate {
            row: hk_dry_dislocation(),
            volumes: &volumes,
        };
        let synthetic = CreepCandidate {
            row: synthetic_linear,
            volumes: &volumes,
        };

        let alone_real =
            ductile_strength_mpa(&[real], conditions, VolumeEnd::Low).expect("resolves");
        let alone_synthetic =
            ductile_strength_mpa(&[synthetic], conditions, VolumeEnd::Low).expect("resolves");
        let together =
            ductile_strength_mpa(&[real, synthetic], conditions, VolumeEnd::Low).expect("resolves");

        assert!(
            together < alone_real && together < alone_synthetic,
            "two mechanisms in parallel reach the rate at a LOWER stress than either alone: together {}, real {}, synthetic {}",
            together.to_f64_lossy(),
            alone_real.to_f64_lossy(),
            alone_synthetic.to_f64_lossy()
        );

        // THE ORDER OF THE CANDIDATES CANNOT MOVE THE ANSWER (Principle 3): a sum is a sum.
        assert_eq!(
            together,
            ductile_strength_mpa(&[synthetic, real], conditions, VolumeEnd::Low).expect("resolves"),
            "the composite must be a pure function of the candidate SET, not of the slice order"
        );
    }

    #[test]
    fn the_strain_rate_has_no_default_and_the_composite_needs_it() {
        // CONDITION 4, in the only form a test can hold: the rate is REQUIRED (there is no `Default` on
        // `CreepConditions` and no fallback in the evaluator, which the type checker enforces at every call
        // site), and it is LOAD-BEARING rather than decorative. A faster load needs a higher stress to sustain
        // it, monotonically, so a rate the evaluator quietly ignored would show up here as a flat answer.
        let volumes = [table2_dry_dislocation_volume_fixture()];
        let row = hk_dry_dislocation();
        let at = |ln_rate: Fixed| {
            ductile_strength_mpa(
                &[CreepCandidate {
                    row,
                    volumes: &volumes,
                }],
                CreepConditions {
                    ln_strain_rate_per_s: ln_rate,
                    temperature_k: Fixed::from_int(1673),
                    pressure_gpa: Fixed::ONE,
                    grain_size_um: None,
                    water: None,
                },
                VolumeEnd::Low,
            )
            .expect("resolves")
        };
        let slow = at(ln_scientific(1, 1, -15));
        let fast = at(ln_scientific(1, 1, -12));
        assert!(
            fast > slow,
            "the strength must rise with the load's own strain rate: 1e-15 gave {}, 1e-12 gave {}",
            slow.to_f64_lossy(),
            fast.to_f64_lossy()
        );
    }

    #[test]
    fn a_stress_outside_the_type_refuses_and_carries_its_log_rather_than_saturating() {
        // THE SATURATION IS REACHABLE FROM A LID'S OWN RANGE, which is why this is a refusal and not a comment.
        // A cold lid at a geological strain rate drives the creep strength past Fixed::MAX: the flow law saying
        // creep is irrelevant there, which is true and is the envelope's brittle branch's cue. Returning a
        // saturated Fixed::MAX would hand the envelope a fabricated number in the one place it cannot tell a
        // fabricated number from a real one, so the refusal carries the log-space answer instead and nothing is
        // lost.
        let volumes = [table2_dry_dislocation_volume_fixture()];
        let row = hk_dry_dislocation();
        let cold = ductile_strength_mpa(
            &[CreepCandidate {
                row,
                volumes: &volumes,
            }],
            CreepConditions {
                ln_strain_rate_per_s: ln_scientific(1, 1, -15),
                temperature_k: Fixed::from_int(500),
                pressure_gpa: Fixed::ONE,
                grain_size_um: None,
                water: None,
            },
            VolumeEnd::Low,
        );
        match cold {
            Err(CreepRefusal::StressNotRepresentable { ln_stress_mpa }) => assert!(
                ln_stress_mpa > Fixed::MAX.ln(),
                "the refusal must carry the real log-space answer, above the type's own ceiling"
            ),
            other => {
                panic!("a 500 K lid at 1e-15 /s must refuse rather than saturate, got {other:?}")
            }
        }
    }

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

    /// A bracket over ONE determination, read at a pressure its own chord covers, for the tests that are about
    /// the GRADE rather than the span. It routes through the real [`select_activation_volume`] rather than
    /// building a bracket by hand, so a test cannot drift from the selection it is meant to be testing against.
    fn bracket_of(v: ActivationVolume) -> ActivationVolumeBracket {
        select_activation_volume(&[v], v.interval_min_gpa)
            .expect("one determination still brackets")
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
        assert!(hk_dry_dislocation().exponent_admits(&bracket_of(measured_v)));
        // THE GBS ROW FAILS BY CONSTRUCTION, and that is the gate working: H&K's GBS energies are ASSUMED,
        // transferred from easy-slip data (their footnote f), never fitted to GBS at all.
        assert!(
            !hk_dry_gbs_below_1250c().exponent_admits(&bracket_of(measured_v)),
            "an ASSUMED activation energy must not cross into an Arrhenius exponent without escalation"
        );
        // An estimated V* also fails, even under a fitted E*: the gate checks EVERY quantity entering the
        // exponent, not just the headline one.
        let estimated_v = ActivationVolume {
            modality: Modality::Estimated,
            ..measured_v
        };
        assert!(!hk_dry_dislocation().exponent_admits(&bracket_of(estimated_v)));
        // AND THE GRADE IS ASKED OVER THE WHOLE SPAN, never the two ends alone. A table pairing a fitted
        // determination with an estimated one is refused wherever the span reaches both, IN EITHER ORDER, which
        // is what stops the gate's answer from being a fact about how a caller listed its rows.
        for pair in [[measured_v, estimated_v], [estimated_v, measured_v]] {
            let spanning = select_activation_volume(&pair, Fixed::ONE).expect("both cover 1 GPa");
            assert!(
                !hk_dry_dislocation().exponent_admits(&spanning),
                "an estimated determination anywhere in the span refuses it, whatever order it was listed in"
            );
        }
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
        // Form B's barrier, as freezer::diffusion_barrier(E_coh, f) would return it.
        let e_star = class_derived_activation_energy(Fixed::from_int(240))
            .expect("the Form B route resolves");
        assert!(
            e_star.modality.admitted_to_exponent(),
            "the estimator rung's g*R*T_m route must cross, or the ladder's other leg fails its neighbour's test"
        );
        // The route REFUSES rather than fabricating when there is no barrier to key on.
        assert!(class_derived_activation_energy(Fixed::ZERO).is_none());
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
            hk_wet_dislocation_hydroxyl(),
            hk_wet_diffusion_hydroxyl(),
        ] {
            assert!(
                !matches!(row.activation_energy.modality, Modality::ClassDerived(_)),
                "no measured row may wear the estimator's grade; that would launder the ladder's rungs together"
            );
        }
        assert!(
            !hk_dry_gbs_below_1250c().exponent_admits(&bracket_of(v)),
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
        // quantity, same table, different chords, and the difference is physics rather than disagreement. Each
        // is covered by exactly one chord, so each bracket is DEGENERATE: a span of zero width, which is a
        // determination rather than a band.
        let lid = select_activation_volume(&rows, Fixed::ONE).expect("the lid's regime is covered");
        assert!(lid.is_degenerate() && lid.at(VolumeEnd::Low) == Fixed::from_int(20));
        assert_eq!(lid.constraint(), VolumeConstraint::CoveredBySource);
        let deep = select_activation_volume(&rows, Fixed::from_int(5))
            .expect("the deep regime is covered");
        assert!(deep.is_degenerate() && deep.at(VolumeEnd::High) == Fixed::from_int(6));

        // WHERE TWO CHORDS OVERLAP, THE FIRST-MATCH DEFECT IS VISIBLE, and this is the case the ruling names.
        // Both chords are closed at 2 GPa, so both cover it, and a `.find()` would have returned whichever the
        // caller happened to list first: an ORDER-DEPENDENT AUTHORED SELECTION of the very number the primary
        // declines to choose. The bracket spans them both instead, and it is the same span in EITHER ORDER.
        for order in [[low, high], [high, low]] {
            let both =
                select_activation_volume(&order, Fixed::from_int(2)).expect("both cover 2 GPa");
            assert_eq!(both.constraint(), VolumeConstraint::CoveredBySource);
            assert_eq!(both.at(VolumeEnd::Low), Fixed::from_int(6));
            assert_eq!(both.at(VolumeEnd::High), Fixed::from_int(20));
            assert!(
                !both.is_degenerate(),
                "two determinations cover 2 GPa: that is a band"
            );
        }

        // OUTSIDE EVERY INTERVAL, the source constrains nothing and the bracket says so: it reports the TABLE'S
        // OWN EXTREMES tagged unconstrained, which is neither the nearest value (extrapolating a chord past its
        // endpoints) nor a refusal (which would block a lid from being sampled at its own surface).
        let far = select_activation_volume(&rows, Fixed::from_int(40))
            .expect("no chord covers 40 GPa, and the table still supports its own extremes");
        assert_eq!(far.constraint(), VolumeConstraint::UnconstrainedBySource);
        assert_eq!(far.at(VolumeEnd::Low), Fixed::from_int(6));
        assert_eq!(far.at(VolumeEnd::High), Fixed::from_int(20));

        // AN EMPTY TABLE HAS NO SPAN, which is the one case that reports nothing at all.
        assert!(
            select_activation_volume(&[], Fixed::ONE).is_none(),
            "no determination banked, no bracket to report"
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
            hk_wet_dislocation_hydroxyl(),
        ] {
            assert_eq!(
                row.stress_exponent,
                Fixed::from_ratio(35, 10),
                "every dislocation-class row carries the table's n = 3.5"
            );
        }
        // The DIFFUSION row is the control: n = 1 is a mechanism label rather than a laundered band, and its
        // presence proves the loop above tests a claim rather than restating a constant every row happens to
        // share.
        assert_eq!(hk_wet_diffusion_hydroxyl().stress_exponent, Fixed::ONE);
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
        // TWIN: ln(90) = 4.4998... by external decimal log, against the entry's ln_scientific(9, 1, 1). This
        // row's prefactor is the SMALLEST in the table and the fugacity row's is 1600, an 18x gap that is the
        // whole content of the constant-C_OH-versus-fugacity switch the verification found inverted downstream.
        assert!(
            (hk_wet_dislocation_hydroxyl().ln_prefactor - Fixed::from_ratio(450, 100)).abs()
                < Fixed::from_ratio(1, 10),
            "ln(90) ~ 4.50, got {:?}",
            hk_wet_dislocation_hydroxyl().ln_prefactor
        );
        // TWIN: ln(1.0e6) = 13.8155... by external decimal log, against the entry's ln_scientific(1, 1, 6).
        assert!(
            (hk_wet_diffusion_hydroxyl().ln_prefactor - Fixed::from_ratio(1382, 100)).abs()
                < Fixed::from_ratio(1, 10),
            "ln(1.0e6) ~ 13.82, got {:?}",
            hk_wet_diffusion_hydroxyl().ln_prefactor
        );
    }
}
