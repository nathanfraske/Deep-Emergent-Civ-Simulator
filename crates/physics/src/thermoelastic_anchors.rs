// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.

//! THE MIE-GRUENEISEN-DEBYE ANCHOR COLUMN: the effective Debye temperature and the volume exponent.
//!
//! # What this holds and why it is a separate file
//!
//! The MGD thermal equation of state needs six per-phase anchors. Four were already banked and scattered
//! across two files by the physics they belong to: `V_0` in the phase registry, `K_0` in
//! `mineral_moduli.toml`, `K_0'` and `gamma_0` in `gruneisen.toml`. This file carries the two that no
//! other file had a reason to hold, the effective Debye temperature `theta_0` and the volume exponent `q`,
//! both transcribed from the same Stixrude & Lithgow-Bertelloni Table 1 rows that supplied the banked
//! `gamma_0`.
//!
//! # The type separation, and why it is a type rather than a comment
//!
//! There are TWO Debye temperatures in this repository and they are different quantities:
//!
//! - The **effective** one, loaded here, fit by its source to reproduce the vibrational entropy near
//!   1000 K. It stands for an integral over the whole phonon density of states, acoustic and optic.
//! - The **elastic** one, derived in `civsim_materials::thermoelastic` from the banked bulk and shear
//!   moduli, density and atomic volume. It is set by the three acoustic branches near `k = 0` and is
//!   asymptotically correct as `T -> 0`.
//!
//! Measured against each other on the banked inputs they run from 0.83 to 1.22, a spread that does not
//! cancel, and forsterite sits near the crossover where they coincide. A single forsterite spot-check
//! therefore certifies nothing about the pairing, and an earlier pass in this repository made exactly that
//! mistake: it derived the elastic value, checked it against forsterite, and concluded no `theta_0` column
//! was needed because a fetched one "would put one fact in two places". They are not one fact. They are
//! two averages of one spectrum, and calling them duplicates is the inverse of the diamond defect.
//!
//! So [`EffectiveDebyeTemperature`] is a newtype with a PRIVATE field and no public constructor from a
//! bare `Fixed`. The only way to obtain one is to load it from this column. The elastic value has its own
//! type on the materials side, there is no conversion in either direction, and the MGD assembly accepts
//! only this one. A defence carried in a comment is one that gets dropped; this one cannot be written
//! wrongly because the wrong version does not compile.
//!
//! The elastic value is NOT demoted by that separation. It is correct for the `T -> 0` acoustic regime,
//! the low-temperature `C_V = beta T^3` coefficient and the acoustic density of states, provided it is
//! paired with its own acoustic `gamma_el` rather than with the fitted `gamma_0` banked here.
//!
//! # One fit, one provenance key
//!
//! `theta_0`, `gamma_0`, `q` and `K'` are not four independent measurements. They are parameters of ONE
//! least-squares global inversion, iterated to self-consistency against a shared corpus, so they carry a
//! joint covariance that independent provenance leaves would falsely discard. [`JOINT_FIT_PROVENANCE_KEY`]
//! is the single key all four share, and [`ThermoelasticAnchorRow::pairs_with_banked_gamma`] reports
//! whether the row's own channel reproduces the banked `gamma_0`, so mixing two inversions is visible
//! rather than silent.

use civsim_core::Fixed;
use std::collections::BTreeMap;
use std::fmt;

/// The provenance key shared by every parameter of the Stixrude & Lithgow-Bertelloni joint inversion.
///
/// `theta_0`, `gamma_0`, `q` and `K'` were fit TOGETHER. Registering them under independent keys would let
/// an uncertainty combination treat four correlated values as four independent ones and shrink the
/// combined band by a factor that does not exist.
pub const JOINT_FIT_PROVENANCE_KEY: &str = "slb_mantle_thermodynamics_joint_inversion";

/// A phase's EFFECTIVE Debye temperature (K), the one an MGD equation of state consumes.
///
/// Private field, no public constructor from `Fixed`, no conversion from the elastic Debye temperature.
/// See the module documentation: the two are different averages of one phonon spectrum, and this type
/// exists so the substitution cannot be expressed rather than merely being warned against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EffectiveDebyeTemperature(Fixed);

impl EffectiveDebyeTemperature {
    /// The value in kelvin.
    ///
    /// Deliberately one-way: a `Fixed` comes OUT for arithmetic, and no `Fixed` goes back IN, so this
    /// cannot become a laundering route for the elastic value.
    pub fn kelvin(self) -> Fixed {
        self.0
    }
}

impl fmt::Display for EffectiveDebyeTemperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} K (effective, entropy-fit)", self.0.to_f64_lossy())
    }
}

/// How a cell was obtained, read from the SOURCE'S OWN TYPOGRAPHY rather than assumed.
///
/// Both source tables print "Italicized entries are from systematics", and `pdftotext` discards italics,
/// so the transcription read every cell a second time through the PDF font metadata. One cell in this
/// column came back italic (quartz's `q`), and it is the exact assumed `q = 1` that an unlabelled
/// transcription would have shipped wearing a measured grade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellGrade {
    /// Fit to data by the source's inversion (Roman in both tables).
    Fit,
    /// Estimated by the source from systematics (italic), never a measurement.
    Systematics,
}

impl CellGrade {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "fit" => Some(CellGrade::Fit),
            "unverified_measurement_candidate" | "systematics" => Some(CellGrade::Systematics),
            _ => None,
        }
    }

    /// Whether a consumer may read this cell as an anchor. A systematics cell may not: it is the source's
    /// own estimate for a species it could not constrain, and reading it as measured is the defect the
    /// font-metadata channel exists to catch.
    pub fn usable_as_anchor(self) -> bool {
        matches!(self, CellGrade::Fit)
    }
}

/// Whether the two source inversions agree on a value. THREE-VALUED, not a boolean.
///
/// This enum exists because a boolean got it wrong. The column's `channels_agree` field carries
/// `"false"`, `"within-band"` and `"single-channel"`, and a first loader read it with a helper that
/// mapped anything other than `"true"` to false. `"within-band"` means the channels DO agree, so the
/// misread inverted the meaning on every row that used it, silently, in the safe-looking direction of
/// reporting more disagreement than exists.
///
/// The lesson is the one this file already applies to [`CellGrade`]: an unrecognised member of a
/// vocabulary must be FATAL, never a fall-through to whichever variant the parser happened to default to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelAgreement {
    /// The successor inversion moved the value outside the bands: a real supersession.
    Disagree,
    /// The two inversions differ by less than their stated uncertainty.
    WithinBand,
    /// Only one inversion carries this phase, so there is nothing to compare.
    SingleChannel,
}

impl ChannelAgreement {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "false" => Some(ChannelAgreement::Disagree),
            "true" | "within-band" => Some(ChannelAgreement::WithinBand),
            "single-channel" => Some(ChannelAgreement::SingleChannel),
            _ => None,
        }
    }

    /// Whether the channels are consistent. A single-channel row is NOT consistent-by-default: it is
    /// unchecked, and the distinction is kept because "nothing contradicted it" is not "it was checked".
    pub fn agrees(self) -> bool {
        matches!(self, ChannelAgreement::WithinBand)
    }
}

/// Whether a row's own fit reproduces the `gamma_0` banked in `gruneisen.toml`. THREE-VALUED.
///
/// `"no-banked-gamma_0"` is a distinct state from a mismatch: quartz has no banked `gamma_0` at all, so
/// there is nothing to pair with, which is a different reason to refuse than a pair that was checked and
/// disagreed. Collapsing them would report quartz as a covariance violation when it is an absence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GammaPairing {
    /// This row's fit reproduces the banked `gamma_0`, so the parameters are one jointly-constrained set.
    Matches,
    /// This row's fit gives a different `gamma_0` than the bank: two inversions would be mixed.
    Mismatch,
    /// No `gamma_0` is banked for this phase, so no pairing can be established either way.
    NoBankedGamma,
}

impl GammaPairing {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "true" => Some(GammaPairing::Matches),
            "false" => Some(GammaPairing::Mismatch),
            "no-banked-gamma_0" => Some(GammaPairing::NoBankedGamma),
            _ => None,
        }
    }
}

/// WHERE A ROW'S FIT APPLIES, as the row itself states it.
///
/// Every `[[anchor]]` block carries a `scope` sentence and the loader discarded it, so a source's own
/// statement of where its model holds never reached the consumer. Carrying it is the first half of the fix;
/// the second half is making the testable part testable, and that split is deliberate:
///
/// - The `stated` sentence is carried VERBATIM, so nothing the source said is lost even where no machine
///   can act on it.
/// - `reference_temperature_k` and `reference_pressure_bar` are transcribed numerically on every row and
///   are checkable today. The MGD solver takes its thermal pressure as a DIFFERENCE from a reference
///   temperature it holds as its own constant, so a row declaring a different reference is a mismatch the
///   consumer must refuse rather than silently re-anchor.
/// - The fitted P,T SPAN is NOT numerically transcribed on any shipped row. The scope sentences say
///   "upper-mantle to lower-mantle P,T as fit by the inversion", which is a regime rather than an interval,
///   and inventing endpoints for it would be fabricating the very thing this type exists to carry. The
///   fields are here, the loader reads them when a row supplies them, and where a row does not the absence
///   rides every answer as [`ScopeCaveat::FittedSpanNotTranscribed`] instead of passing for validity.
/// - `riders` carry assumptions the fit inherits from outside the quasi-harmonic model. Fayalite's is live:
///   its scope discloses that the 2005 entropy assumes `R ln 5` magnetic entropy per Fe, a magnetic
///   contribution the Debye model does not contain and cannot reproduce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelScope {
    /// The row's own scope sentence, verbatim and unparsed.
    pub stated: String,
    /// The reference temperature the row's anchors are stated at (K).
    pub reference_temperature_k: Option<Fixed>,
    /// The reference pressure the row's anchors are stated at (bar).
    pub reference_pressure_bar: Option<Fixed>,
    /// The temperature span the fit was made over (K), where the row transcribes one.
    pub fitted_temperature_span_k: Option<(Fixed, Fixed)>,
    /// The pressure span the fit was made over (GPa), where the row transcribes one.
    pub fitted_pressure_span_gpa: Option<(Fixed, Fixed)>,
    /// Assumptions the fit carries from outside the model the consumer evaluates.
    pub riders: Vec<String>,
}

/// A test that CONVICTED a row at the evaluated state. A failure blocks; the row may not be evaluated there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeFailure {
    /// The row declares a reference state the consumer does not anchor at. Refused rather than re-anchored:
    /// the anchors are meaningful at the reference they were fit against and nowhere else, and quietly
    /// evaluating them against a different one would move every thermal pressure by an unstated amount.
    ReferenceStateMismatch {
        /// The reference temperature the row declares (K).
        declared_k: Fixed,
        /// The reference temperature the consumer anchors at (K).
        assumed_k: Fixed,
    },
    /// The evaluated temperature lies outside the span the row transcribes for its fit.
    TemperatureOutsideFittedSpan {
        /// The temperature asked about (K).
        t_k: Fixed,
        /// The low edge of the transcribed span (K).
        span_lo_k: Fixed,
        /// The high edge of the transcribed span (K).
        span_hi_k: Fixed,
    },
    /// The evaluated pressure lies outside the span the row transcribes for its fit.
    PressureOutsideFittedSpan {
        /// The pressure asked about (GPa).
        p_gpa: Fixed,
        /// The low edge of the transcribed span (GPa).
        span_lo_gpa: Fixed,
        /// The high edge of the transcribed span (GPa).
        span_hi_gpa: Fixed,
    },
}

impl fmt::Display for ScopeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeFailure::ReferenceStateMismatch {
                declared_k,
                assumed_k,
            } => write!(
                f,
                "the row's anchors are stated at {} K and the consumer anchors at {} K",
                declared_k.to_f64_lossy(),
                assumed_k.to_f64_lossy()
            ),
            ScopeFailure::TemperatureOutsideFittedSpan {
                t_k,
                span_lo_k,
                span_hi_k,
            } => write!(
                f,
                "{} K lies outside the transcribed fit span [{}, {}] K",
                t_k.to_f64_lossy(),
                span_lo_k.to_f64_lossy(),
                span_hi_k.to_f64_lossy()
            ),
            ScopeFailure::PressureOutsideFittedSpan {
                p_gpa,
                span_lo_gpa,
                span_hi_gpa,
            } => write!(
                f,
                "{} GPa lies outside the transcribed fit span [{}, {}] GPa",
                p_gpa.to_f64_lossy(),
                span_lo_gpa.to_f64_lossy(),
                span_hi_gpa.to_f64_lossy()
            ),
        }
    }
}

/// Something the scope check could not test, or tested and wants seen. A caveat NEVER blocks: it rides the
/// answer so a reader sees what the number is standing on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeCaveat {
    /// The row states its fitted regime in prose and transcribes no interval for this axis, so the span
    /// test could not run. Reported rather than passed over: "nothing convicted it" is not "it was
    /// checked", the same distinction [`ChannelAgreement::SingleChannel`] keeps one type over.
    FittedSpanNotTranscribed {
        /// The axis whose span is missing.
        axis: &'static str,
    },
    /// The fit carries an assumption from outside the model the consumer evaluates. Fayalite's `R ln 5`
    /// magnetic entropy per Fe is the live one: a magnetic contribution a Debye model does not contain, so
    /// the anchors reproduce their source's entropy while the consumer's quasi-harmonic form cannot.
    AssumptionOutsideTheModel {
        /// The assumption, in the row's own words.
        text: String,
    },
}

impl fmt::Display for ScopeCaveat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeCaveat::FittedSpanNotTranscribed { axis } => write!(
                f,
                "the row transcribes no fitted {axis} interval, so that scope test could not run"
            ),
            ScopeCaveat::AssumptionOutsideTheModel { text } => {
                write!(f, "the fit assumes {text}")
            }
        }
    }
}

/// Whether a row may be evaluated at a state, with everything the check found either way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeVerdict {
    /// No test convicted the row. The caveats ride the answer.
    InScope {
        /// What could not be tested, or was tested and wants seen.
        caveats: Vec<ScopeCaveat>,
    },
    /// At least one test convicted the row. EVERY failing test is reported, never the first one only: a row
    /// can be out of scope on several axes at once, and naming one would understate what it would take to
    /// bring it in.
    OutOfScope {
        /// Every test that convicted the row.
        failures: Vec<ScopeFailure>,
        /// What could not be tested, carried so a refusal still shows what it did not reach.
        caveats: Vec<ScopeCaveat>,
    },
}

impl ScopeVerdict {
    /// Whether the row may be evaluated at the state this verdict was taken for.
    pub fn in_scope(&self) -> bool {
        matches!(self, ScopeVerdict::InScope { .. })
    }

    /// The caveats, either way. A refusal carries them too.
    pub fn caveats(&self) -> &[ScopeCaveat] {
        match self {
            ScopeVerdict::InScope { caveats } | ScopeVerdict::OutOfScope { caveats, .. } => caveats,
        }
    }
}

impl ModelScope {
    /// Test this scope at a state, returning every conviction and every rider.
    ///
    /// `assumed_reference_k` is the reference temperature the CONSUMER anchors at, passed in rather than
    /// assumed here, so the mismatch check compares two stated things instead of one stated and one
    /// remembered.
    // @derives: whether a phase's fit applies at a state <- the row's own transcribed scope and reference state
    pub fn verdict_at(&self, t_k: Fixed, p_gpa: Fixed, assumed_reference_k: Fixed) -> ScopeVerdict {
        let mut failures = Vec::new();
        let mut caveats = Vec::new();

        if let Some(declared_k) = self.reference_temperature_k {
            if declared_k != assumed_reference_k {
                failures.push(ScopeFailure::ReferenceStateMismatch {
                    declared_k,
                    assumed_k: assumed_reference_k,
                });
            }
        }
        match self.fitted_temperature_span_k {
            Some((lo, hi)) if t_k < lo || t_k > hi => {
                failures.push(ScopeFailure::TemperatureOutsideFittedSpan {
                    t_k,
                    span_lo_k: lo,
                    span_hi_k: hi,
                })
            }
            Some(_) => {}
            None => caveats.push(ScopeCaveat::FittedSpanNotTranscribed {
                axis: "temperature",
            }),
        }
        match self.fitted_pressure_span_gpa {
            Some((lo, hi)) if p_gpa < lo || p_gpa > hi => {
                failures.push(ScopeFailure::PressureOutsideFittedSpan {
                    p_gpa,
                    span_lo_gpa: lo,
                    span_hi_gpa: hi,
                })
            }
            Some(_) => {}
            None => caveats.push(ScopeCaveat::FittedSpanNotTranscribed { axis: "pressure" }),
        }
        for rider in &self.riders {
            caveats.push(ScopeCaveat::AssumptionOutsideTheModel {
                text: rider.clone(),
            });
        }

        if failures.is_empty() {
            ScopeVerdict::InScope { caveats }
        } else {
            ScopeVerdict::OutOfScope { failures, caveats }
        }
    }
}

/// ONE SOURCE INVERSION'S CELLS FOR A PHASE.
///
/// The column transcribes two global inversions per mantle row and the loader read only one of them,
/// keeping the successor's `q` solely as a fallback for the single-channel case and never reading its
/// `theta_0` at all. What survived was [`ChannelAgreement`], a three-valued flag recording THAT the
/// channels disagree while dropping BY HOW MUCH and TOWARD WHAT. For enstatite that discarded a factor of
/// 2.3 in `q` (7.8 against 3.4) with its own citation calling it the largest disagreement in the column,
/// and `q` enters two exponentials.
///
/// # One fit at a time, both fits kept
///
/// A channel is a COHERENT SET: its `V_0`, `K_0`, `K_0'`, `theta_0`, `gamma_0` and `q` were fit together
/// against one corpus and are meaningful together. Mixing cells across channels is what
/// [`ThermoelasticAnchorRow::pairs_with_banked_gamma`] exists to prevent and that rule is untouched. "One
/// joint fit or nothing" bars a MIXED set; it does not bar carrying the competing coherent set beside it,
/// and erasing the competitor was never what the rule asked for.
///
/// `gamma_0` is `None` on the primary channel by design: the bank (`gruneisen.toml`'s `gamma_eos_debye`)
/// holds the 2005 value, carried by pointer so there is no second copy to drift. A successor channel
/// carries its own, because the bank has no 2011 value to point at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorChannel {
    /// The source inversion this set came from, as the row names it.
    pub family: String,
    /// The volume exponent `q`.
    pub q: Option<Fixed>,
    /// `q`'s uncertainty as this channel's source states it.
    pub q_band: Option<Fixed>,
    /// Whether this channel's `q` was fit or estimated from systematics.
    pub q_grade: Option<CellGrade>,
    /// The EFFECTIVE Debye temperature from this channel.
    pub theta_0: Option<EffectiveDebyeTemperature>,
    /// `theta_0`'s uncertainty as this channel's source states it (K).
    pub theta_0_band_k: Option<Fixed>,
    /// Whether this channel's `theta_0` was fit or estimated from systematics.
    pub theta_0_grade: Option<CellGrade>,
    /// This channel's own `gamma_0`. `None` on the primary, which points at the bank instead.
    pub gamma_0: Option<Fixed>,
    /// `gamma_0`'s uncertainty as this channel's source states it.
    pub gamma_0_band: Option<Fixed>,
    /// Reference molar volume from this fit (cm^3/mol).
    pub v0_cm3: Option<Fixed>,
    /// Reference isothermal bulk modulus from this fit (GPa).
    pub k0_gpa: Option<Fixed>,
    /// Pressure derivative of the bulk modulus from this fit.
    pub k0_prime: Option<Fixed>,
    /// Atoms per formula unit matching THIS CHANNEL'S `v0_cm3`. The channels differ: enstatite is
    /// `Mg4Si4O12` (20 atoms) in 2005 and `MgMgSi2O6` (10) in 2011, a factor of two in both.
    pub atoms_per_formula_unit: Option<u32>,
}

/// One phase's MGD anchors, with the grade and channel agreement each value carries.
#[derive(Clone, Debug)]
pub struct ThermoelasticAnchorRow {
    /// The phase name, as the registry spells it.
    pub name: String,
    /// The volume exponent `q` in `gamma = gamma_0 (V/V_0)^q`.
    pub q: Option<Fixed>,
    /// `q`'s uncertainty as the source states it.
    pub q_band: Option<Fixed>,
    /// Whether `q` was fit or estimated from systematics.
    pub q_grade: Option<CellGrade>,
    /// The EFFECTIVE Debye temperature. See [`EffectiveDebyeTemperature`].
    pub theta_0: Option<EffectiveDebyeTemperature>,
    /// `theta_0`'s uncertainty as the source states it (K).
    pub theta_0_band_k: Option<Fixed>,
    /// Whether `theta_0` was fit or estimated from systematics.
    pub theta_0_grade: Option<CellGrade>,
    /// Whether the 2005 and 2011 inversions agree on this row's `theta_0`.
    pub theta_0_channels: ChannelAgreement,
    /// Whether the 2005 and 2011 inversions agree on this row's `q`.
    pub q_channels: ChannelAgreement,
    /// Whether this row's own fit reproduces the `gamma_0` banked in `gruneisen.toml`.
    pub gamma_pairing: GammaPairing,
    /// The column's own explicit refusal flag, honoured when present regardless of the cell grades.
    pub usable_as_anchor: Option<bool>,
    /// Reference molar volume from the SAME fit (cm^3/mol).
    pub v0_cm3: Option<Fixed>,
    /// Reference isothermal bulk modulus from the SAME fit (GPa).
    pub k0_gpa: Option<Fixed>,
    /// Pressure derivative of the bulk modulus from the SAME fit.
    pub k0_prime: Option<Fixed>,
    /// Atoms per formula unit matching THIS ROW'S `v0_cm3` basis.
    ///
    /// Carried per row rather than derived from the phase registry because the two source channels use
    /// DIFFERENT formula units for the same phase: enstatite is `Mg4Si4O12` in 2005 and `MgMgSi2O6` in
    /// 2011, a factor of two in both `V_0` and the atom count. Taking `V_0` from here and the atom count
    /// from elsewhere would halve the molar basis silently, and the thermal energy is per formula unit.
    pub atoms_per_formula_unit: Option<u32>,
    /// Where this row's fit applies, as the row itself states it. See [`ModelScope`].
    pub scope: ModelScope,
    /// Every source inversion this row transcribes, PRIMARY FIRST, in the order the row declares them.
    ///
    /// The flat fields above are the primary channel's cells, kept as a view onto `channels[0]` rather than
    /// a second parse, so there is nothing to drift.
    pub channels: Vec<AnchorChannel>,
}

impl ThermoelasticAnchorRow {
    /// Whether this row's parameters may be read against the banked `gamma_0`.
    ///
    /// Only [`GammaPairing::Matches`] passes. A mismatch would mix two inversions, and a missing banked
    /// `gamma_0` leaves the set incomplete: both are refusals, for different reasons, and the consumer
    /// gets neither a value nor a guess.
    pub fn pairs_with_banked_gamma(&self) -> bool {
        matches!(self.gamma_pairing, GammaPairing::Matches)
    }

    /// Whether every cell this row needs for an MGD evaluation is FIT rather than assumed.
    ///
    /// The column's own `usable_as_anchor = "false"` wins outright when present. A row may be refused for
    /// a reason the per-cell grades do not express, and the file gets the final word over an inference
    /// drawn from its parts.
    pub fn all_cells_fit(&self) -> bool {
        if self.usable_as_anchor == Some(false) {
            return false;
        }
        matches!(self.q_grade, Some(g) if g.usable_as_anchor())
            && matches!(self.theta_0_grade, Some(g) if g.usable_as_anchor())
    }
}

/// A parse failure in the anchor column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchorError {
    /// A block carried no `name`.
    UnnamedBlock,
    /// A value did not parse as fixed-point decimal.
    BadValue {
        /// The phase the bad value is on.
        phase: String,
        /// The field name.
        field: String,
        /// The text that failed to parse.
        text: String,
    },
    /// A grade string was not one this loader recognises. Deliberately fatal: an unrecognised grade
    /// silently defaulting to "fit" would launder a systematics cell into a measured one.
    UnknownGrade {
        /// The phase the grade is on.
        phase: String,
        /// The text that was not recognised.
        text: String,
    },
}

impl fmt::Display for AnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnchorError::UnnamedBlock => write!(f, "an [[anchor]] block carries no name"),
            AnchorError::BadValue {
                phase,
                field,
                text,
            } => write!(f, "phase {phase} field {field} does not parse as decimal: {text:?}"),
            AnchorError::UnknownGrade { phase, text } => write!(
                f,
                "phase {phase} carries grade {text:?}, which this loader does not recognise. Refused \
                 rather than defaulted: an unrecognised grade falling through to \"fit\" would launder a \
                 systematics estimate into a measured anchor."
            ),
        }
    }
}

impl std::error::Error for AnchorError {}

/// The cited per-phase MGD anchor table.
#[derive(Clone, Debug)]
pub struct ThermoelasticAnchors {
    rows: BTreeMap<String, ThermoelasticAnchorRow>,
}

impl ThermoelasticAnchors {
    /// Load the vendored column.
    pub fn standard() -> Result<Self, AnchorError> {
        Self::from_toml_str(include_str!("../data/thermoelastic_anchors.toml"))
    }

    /// Parse the `[[anchor]]` blocks. Values are QUOTED strings parsed to fixed-point through
    /// [`Fixed::from_decimal_str`], so no float ever enters.
    ///
    /// The block split is LINE-ANCHORED, never a bare string match. The file's own header names
    /// `[[anchor]]` while explaining the block-kind idiom, so a bare split manufactures a phantom block
    /// out of comment prose. That is not hypothetical: the transcription script that wrote this column hit
    /// exactly that bug, and the sibling Grueneisen loader carries the same defence for the same reason.
    pub fn from_toml_str(s: &str) -> Result<Self, AnchorError> {
        let mut rows: BTreeMap<String, ThermoelasticAnchorRow> = BTreeMap::new();
        let mut blocks: Vec<Vec<&str>> = Vec::new();
        let mut current: Option<Vec<&str>> = None;
        for line in s.lines() {
            if line.trim() == "[[anchor]]" {
                if let Some(prev) = current.take() {
                    blocks.push(prev);
                }
                current = Some(Vec::new());
            } else if let Some(buf) = current.as_mut() {
                buf.push(line);
            }
        }
        if let Some(last) = current.take() {
            blocks.push(last);
        }

        for block in blocks {
            let field = |key: &str| -> Option<String> {
                block.iter().find_map(|line| {
                    let rest = line.trim().strip_prefix(key)?.trim_start();
                    let rest = rest.strip_prefix('=')?.trim();
                    let rest = rest.strip_prefix('"')?;
                    rest.rfind('"').map(|end| rest[..end].to_string())
                })
            };
            let name = field("name").ok_or(AnchorError::UnnamedBlock)?;
            let num =
                |key: &str| -> Result<Option<Fixed>, AnchorError> {
                    match field(key) {
                        None => Ok(None),
                        Some(t) => Fixed::from_decimal_str(&t).map(Some).map_err(|_| {
                            AnchorError::BadValue {
                                phase: name.clone(),
                                field: key.to_string(),
                                text: t.clone(),
                            }
                        }),
                    }
                };
            // Grades are parsed PER CHANNEL below, through `cgrade`, because a row can carry a fit cell in
            // one inversion and an assumed one in the other and a single row-level read would lose that.
            // THE VOCABULARY FIELDS ARE FATAL ON AN UNRECOGNISED MEMBER, exactly as the grades are. A
            // helper that mapped "anything but true" to false read `channels_agree = "within-band"` as a
            // disagreement and inverted the meaning on every row that used it.
            let vocab = |key: &str,
                         f: fn(&str) -> Option<ChannelAgreement>|
             -> Result<ChannelAgreement, AnchorError> {
                match field(key) {
                    None => Ok(ChannelAgreement::SingleChannel),
                    Some(t) => f(&t).ok_or(AnchorError::UnknownGrade {
                        phase: name.clone(),
                        text: t.clone(),
                    }),
                }
            };
            let pairing = match field("gamma_0_matches_banked") {
                None => GammaPairing::NoBankedGamma,
                Some(t) => GammaPairing::parse(&t).ok_or(AnchorError::UnknownGrade {
                    phase: name.clone(),
                    text: t.clone(),
                })?,
            };

            // EVERY CHANNEL THE ROW DECLARES, primary first. The row names them in `channels`; a row that
            // declares none is a single primary channel named by its `q_source`, which is what every row
            // meant before the key existed.
            //
            // KEY RESOLUTION, and why it takes two spellings. A channel's cell lives under
            // `<base>_<family>`. The primary additionally reads the BARE key, and falls through to its own
            // suffixed form: that fall-through is the single-channel case generalized, and it is what lets
            // quartz (2011-only, so its `q` is `q_slb2011`) report an ASSUMED exponent rather than no
            // exponent at all, which is a different and more forgiving error than the truth. The second
            // spelling `<stem>_<family>_<tail>` is what this column grew before the convention settled;
            // two of those keys are pinned by the offline provenance script, so both are read rather than
            // renaming a receipt checker's anchors for tidiness.
            let families: Vec<String> = match field("channels") {
                Some(list) => list
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                None => vec![field("q_source").unwrap_or_else(|| "primary".to_string())],
            };
            let mut channels: Vec<AnchorChannel> = Vec::new();
            for (index, family) in families.iter().enumerate() {
                let primary = index == 0;
                let key = |base: &str| -> Option<String> {
                    let bare = if primary { field(base) } else { None };
                    bare.or_else(|| field(&format!("{base}_{family}")))
                        .or_else(|| {
                            base.rsplit_once('_')
                                .and_then(|(stem, tail)| field(&format!("{stem}_{family}_{tail}")))
                        })
                };
                let cnum = |base: &str| -> Result<Option<Fixed>, AnchorError> {
                    match key(base) {
                        None => Ok(None),
                        Some(t) => Fixed::from_decimal_str(&t).map(Some).map_err(|_| {
                            AnchorError::BadValue {
                                phase: name.clone(),
                                field: base.to_string(),
                                text: t.clone(),
                            }
                        }),
                    }
                };
                let cgrade = |base: &str| -> Result<Option<CellGrade>, AnchorError> {
                    match key(base) {
                        None => Ok(None),
                        Some(t) => {
                            CellGrade::parse(&t)
                                .map(Some)
                                .ok_or(AnchorError::UnknownGrade {
                                    phase: name.clone(),
                                    text: t.clone(),
                                })
                        }
                    }
                };
                channels.push(AnchorChannel {
                    family: family.clone(),
                    q: cnum("q")?,
                    q_band: cnum("q_band")?,
                    q_grade: cgrade("q_grade")?,
                    theta_0: cnum("theta_0_k")?.map(EffectiveDebyeTemperature),
                    theta_0_band_k: cnum("theta_0_band_k")?,
                    theta_0_grade: cgrade("theta_0_grade")?,
                    gamma_0: cnum("gamma_0")?,
                    gamma_0_band: cnum("gamma_0_band")?,
                    v0_cm3: cnum("v0_cm3_per_mol")?,
                    k0_gpa: cnum("k0_gpa")?,
                    k0_prime: cnum("k0_prime")?,
                    atoms_per_formula_unit: key("atoms_per_formula_unit")
                        .and_then(|v| v.parse::<u32>().ok()),
                });
            }
            // The flat fields are a VIEW onto the primary channel, never a second parse.
            let primary = channels.first().cloned().unwrap_or(AnchorChannel {
                family: "primary".to_string(),
                q: None,
                q_band: None,
                q_grade: None,
                theta_0: None,
                theta_0_band_k: None,
                theta_0_grade: None,
                gamma_0: None,
                gamma_0_band: None,
                v0_cm3: None,
                k0_gpa: None,
                k0_prime: None,
                atoms_per_formula_unit: None,
            });

            // THE SCOPE, WHICH THE LOADER USED TO DROP ON THE FLOOR. Every block carries a `scope`
            // sentence and none of it reached a consumer. The sentence is kept verbatim; the two numeric
            // clauses the file transcribes (the reference state) are parsed; the fitted P,T span is read
            // where a row supplies it and reported as untested where none does. No endpoint is invented
            // for a row whose source states its regime in prose.
            let span =
                |lo_key: &str, hi_key: &str| -> Result<Option<(Fixed, Fixed)>, AnchorError> {
                    Ok(match (num(lo_key)?, num(hi_key)?) {
                        (Some(lo), Some(hi)) => Some((lo, hi)),
                        _ => None,
                    })
                };
            let scope = ModelScope {
                stated: field("scope").unwrap_or_default(),
                reference_temperature_k: num("temperature_k")?,
                reference_pressure_bar: num("pressure_bar")?,
                fitted_temperature_span_k: span(
                    "scope_fitted_temperature_k_lo",
                    "scope_fitted_temperature_k_hi",
                )?,
                fitted_pressure_span_gpa: span(
                    "scope_fitted_pressure_gpa_lo",
                    "scope_fitted_pressure_gpa_hi",
                )?,
                // One key, several riders, split on " | " so a row that inherits two assumptions does not
                // need a second key spelling.
                riders: field("scope_rider")
                    .map(|t| {
                        t.split(" | ")
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default(),
            };

            let row = ThermoelasticAnchorRow {
                scope,
                q: primary.q,
                q_band: primary.q_band,
                q_grade: primary.q_grade,
                theta_0: primary.theta_0,
                theta_0_band_k: primary.theta_0_band_k,
                theta_0_grade: primary.theta_0_grade,
                theta_0_channels: vocab("theta_0_channels_agree", ChannelAgreement::parse)?,
                q_channels: vocab("channels_agree", ChannelAgreement::parse)?,
                gamma_pairing: pairing,
                usable_as_anchor: field("usable_as_anchor").map(|v| v == "true"),
                v0_cm3: primary.v0_cm3,
                k0_gpa: primary.k0_gpa,
                k0_prime: primary.k0_prime,
                atoms_per_formula_unit: primary.atoms_per_formula_unit,
                channels,
                name: name.clone(),
            };
            rows.insert(name, row);
        }
        Ok(Self { rows })
    }

    /// One phase's row, or `None`. A phase with no row is REFUSED by the consumer, never defaulted:
    /// hematite is the standing instance, absent from both source compilations because they are
    /// mantle-species databases and it is a ferric phase outside their systems.
    pub fn row(&self, phase: &str) -> Option<&ThermoelasticAnchorRow> {
        self.rows
            .get(crate::mineral_moduli::canonical_phase_key(phase))
            .or_else(|| self.rows.get(phase))
    }

    /// Every phase with a row, in deterministic order.
    pub fn phases(&self) -> impl Iterator<Item = &str> {
        self.rows.keys().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_column_loads_and_carries_both_anchors_for_the_fit_phases() {
        let t = ThermoelasticAnchors::standard().expect("anchors load");
        let fo = t.row("forsterite").expect("forsterite has a row");
        assert_eq!(
            fo.theta_0.expect("forsterite theta_0").kelvin().to_f64_lossy().round(),
            809.0,
            "the EFFECTIVE Debye temperature from Table 1, not the elastic 762 K derived from the moduli"
        );
        assert!(
            (2.0..=2.2).contains(&fo.q.expect("forsterite q").to_f64_lossy()),
            "forsterite q is 2.1 (2)"
        );
        assert!(
            fo.all_cells_fit(),
            "both forsterite cells are Roman, so fit"
        );
        assert!(fo.pairs_with_banked_gamma());
    }

    /// THE PER-CELL GRADE SPLIT: one row, two grades. Quartz's Debye temperature was fit and its volume
    /// exponent was assumed, and a row-level grade would have lost that distinction in whichever
    /// direction it rounded.
    #[test]
    fn quartz_carries_a_fit_theta_beside_an_assumed_q() {
        let t = ThermoelasticAnchors::standard().expect("anchors load");
        let qz = t.row("quartz").expect("quartz has a row");
        assert_eq!(
            qz.theta_0_grade,
            Some(CellGrade::Fit),
            "quartz's theta_0 cell is Roman in Table A1"
        );
        assert_eq!(
            qz.q_grade,
            Some(CellGrade::Systematics),
            "quartz's q cell is ITALIC, the assumed q = 1 that a text-only read would have shipped as measured"
        );
        assert!(
            !qz.all_cells_fit(),
            "so the row as a whole is NOT usable as an MGD anchor set, even though half of it is fit"
        );
    }

    /// THE TWO CHANNELS BREAK ON DIFFERENT PHASES, which is why agreement is tracked per quantity rather
    /// than per row. A reader who checked only one would carry the other's disagreement silently.
    #[test]
    fn the_supersession_breaks_on_different_phases_in_the_two_columns() {
        let t = ThermoelasticAnchors::standard().expect("anchors load");
        let spinel = t.row("spinel").expect("spinel");
        assert_eq!(
            spinel.theta_0_channels,
            ChannelAgreement::Disagree,
            "spinel is the theta_0 break: 900 (3) K in 2005 against 843 (33) K in 2011"
        );
        assert_eq!(
            spinel.q_channels,
            ChannelAgreement::WithinBand,
            "and its q agrees, 2.8 (6) against 2.7 (6)"
        );

        let enstatite = t.row("enstatite").expect("enstatite");
        assert_eq!(
            enstatite.q_channels,
            ChannelAgreement::Disagree,
            "enstatite is the q break: 7.8 (11) against 3.4 (4), a factor of 2.3"
        );
        assert_eq!(
            enstatite.theta_0_channels,
            ChannelAgreement::WithinBand,
            "and its theta_0 agrees, 810 (8) K against 812 (4) K"
        );

        // A SINGLE-CHANNEL ROW is not consistent-by-default. Quartz exists in one table only, so nothing
        // has ever contradicted it, and that is recorded as unchecked rather than as agreement.
        let quartz = t.row("quartz").expect("quartz");
        assert_eq!(quartz.q_channels, ChannelAgreement::SingleChannel);
        assert!(
            !quartz.q_channels.agrees(),
            "unchecked is not the same as checked and consistent"
        );
        assert_eq!(
            quartz.gamma_pairing,
            GammaPairing::NoBankedGamma,
            "quartz has NO banked gamma_0 to pair with, which is a different refusal from a mismatch"
        );
    }

    /// A phase outside the source compilations has NO row and gets no neighbour's numbers.
    #[test]
    fn an_absent_phase_refuses_rather_than_inheriting() {
        let t = ThermoelasticAnchors::standard().expect("anchors load");
        assert!(
            t.row("hematite").is_none(),
            "hematite is absent from both mantle-species compilations and is omitted, not estimated"
        );
        assert!(t.row("unobtainium").is_none());
    }

    /// A ROW'S OWN SCOPE REFUSES BY NAME, and it names EVERY test that convicted rather than the first.
    ///
    /// The shipped column transcribes no numeric fit span: its scopes state a REGIME in prose
    /// ("upper-mantle to lower-mantle P,T as fit by the inversion"), and inventing endpoints for that would
    /// fabricate the very thing the field exists to carry. So the span test is exercised on a row that does
    /// transcribe one. The mechanism is what is under test: a source that states where its fit reaches must
    /// be able to stop a consumer past that edge, and until now the loader discarded the sentence outright.
    #[test]
    fn a_phase_asked_outside_its_transcribed_fit_span_refuses_by_name() {
        let src = "[[anchor]]\nname = \"bounded\"\nq = \"2.0\"\nq_grade = \"fit\"\n\
                   temperature_k = \"300\"\npressure_bar = \"1\"\n\
                   scope = \"fit over a stated interval\"\n\
                   scope_fitted_temperature_k_lo = \"300\"\nscope_fitted_temperature_k_hi = \"1200\"\n\
                   scope_fitted_pressure_gpa_lo = \"0\"\nscope_fitted_pressure_gpa_hi = \"15\"\n";
        let t = ThermoelasticAnchors::from_toml_str(src).expect("the row loads");
        let row = t.row("bounded").expect("bounded has a row");
        let reference = Fixed::from_int(300);

        // Inside the transcribed span: no conviction, and no span caveat either, because the test RAN.
        let inside = row
            .scope
            .verdict_at(Fixed::from_int(1000), Fixed::from_int(10), reference);
        assert!(
            inside.in_scope(),
            "1000 K and 10 GPa lie inside [300, 1200] K and [0, 15] GPa: {inside:?}"
        );
        assert!(
            inside.caveats().is_empty(),
            "a row transcribing both spans has nothing untested to report: {inside:?}"
        );

        // Outside on BOTH axes: both convictions are reported, never the first only.
        let outside = row
            .scope
            .verdict_at(Fixed::from_int(1900), Fixed::from_int(40), reference);
        let ScopeVerdict::OutOfScope { failures, .. } = &outside else {
            panic!("1900 K and 40 GPa lie outside the transcribed spans: {outside:?}")
        };
        assert!(
            failures.contains(&ScopeFailure::TemperatureOutsideFittedSpan {
                t_k: Fixed::from_int(1900),
                span_lo_k: Fixed::from_int(300),
                span_hi_k: Fixed::from_int(1200),
            }),
            "the refusal must NAME the span it fell outside, with the edges: {failures:?}"
        );
        assert_eq!(
            failures.len(),
            2,
            "a row out of scope on two axes reports two convictions: naming one would understate what it \
             takes to bring it back in, which is the discipline RowExclusion already applies one column \
             over: {failures:?}"
        );

        // THE REFERENCE STATE IS A SCOPE TEST TOO, and this one bites on the shipped data. The anchors are
        // stated at the reference their fit was made against, and a consumer that anchors its thermal
        // pressure somewhere else is re-anchoring them silently.
        let rebased = row.scope.verdict_at(
            Fixed::from_int(1000),
            Fixed::from_int(10),
            Fixed::from_int(298),
        );
        assert!(
            matches!(&rebased, ScopeVerdict::OutOfScope { failures, .. }
                     if failures.iter().any(|f| matches!(f, ScopeFailure::ReferenceStateMismatch { .. }))),
            "the row declares 300 K; a consumer anchoring at 298 K must be refused: {rebased:?}"
        );
    }

    /// THE SHIPPED ROWS DO NOT TRANSCRIBE A SPAN, and every answer now says so rather than passing for
    /// checked. "Nothing convicted it" is not "it was checked", the same distinction
    /// [`ChannelAgreement::SingleChannel`] keeps for the channel comparison.
    #[test]
    fn the_shipped_rows_report_their_untranscribed_span_rather_than_claiming_validity() {
        let t = ThermoelasticAnchors::standard().expect("anchors load");
        let mut checked = 0;
        for phase in t.phases().map(str::to_string).collect::<Vec<_>>() {
            let row = t.row(&phase).expect("row");
            assert!(
                !row.scope.stated.is_empty(),
                "{phase}: the scope sentence must survive the loader, which used to drop it"
            );
            assert_eq!(
                row.scope.reference_temperature_k,
                Some(Fixed::from_int(300)),
                "{phase}: every row transcribes its reference state numerically, and that is the clause \
                 the scope check CAN test today"
            );
            let v = row.scope.verdict_at(
                Fixed::from_int(1600),
                Fixed::from_int(10),
                Fixed::from_int(300),
            );
            assert!(
                v.caveats().iter().any(|c| matches!(
                    c,
                    ScopeCaveat::FittedSpanNotTranscribed { axis } if *axis == "temperature"
                )),
                "{phase}: its scope states a regime in prose, so the span test could not run and the \
                 answer must carry that: {v:?}"
            );
            checked += 1;
        }
        assert_eq!(
            checked, 7,
            "all seven rows, not a subset that happened to load"
        );
    }

    /// An unrecognised grade is FATAL, not a silent fall-through to fit.
    #[test]
    fn an_unknown_grade_is_refused_rather_than_defaulted() {
        let src = "[[anchor]]\nname = \"x\"\nq = \"1.0\"\nq_grade = \"probably fine\"\n";
        let err = ThermoelasticAnchors::from_toml_str(src)
            .expect_err("an unrecognised grade must not load");
        assert!(matches!(err, AnchorError::UnknownGrade { .. }), "{err}");
    }

    /// The block split must be LINE-ANCHORED: the file's header prose names `[[anchor]]` while explaining
    /// the idiom, and a bare split would manufacture a phantom row out of a comment.
    #[test]
    fn header_prose_naming_the_block_marker_does_not_manufacture_a_row() {
        let t = ThermoelasticAnchors::standard().expect("anchors load");
        assert_eq!(
            t.phases().count(),
            7,
            "seven real phases, and no phantom block from the header's own mention of the marker"
        );
    }
}
