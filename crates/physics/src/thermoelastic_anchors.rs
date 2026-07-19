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
            let grade = |key: &str| -> Result<Option<CellGrade>, AnchorError> {
                match field(key) {
                    None => Ok(None),
                    Some(t) => CellGrade::parse(&t)
                        .map(Some)
                        .ok_or(AnchorError::UnknownGrade {
                            phase: name.clone(),
                            text: t.clone(),
                        }),
                }
            };
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

            // A SINGLE-CHANNEL ROW carries its value under the channel's own key rather than a bare one.
            // Quartz exists only in the 2011 table, so its `q` is `q_slb2011`, and a loader reading only
            // the bare key would report the phase as having no exponent at all rather than having an
            // assumed one, which is a different and more forgiving error than the truth.
            let q = match num("q")? {
                Some(v) => Some(v),
                None => num("q_slb2011")?,
            };
            let q_grade = match grade("q_grade")? {
                Some(g) => Some(g),
                None => grade("q_slb2011_grade")?,
            };

            let row = ThermoelasticAnchorRow {
                q,
                q_band: num("q_band")?.or(num("q_slb2011_band")?),
                q_grade,
                theta_0: num("theta_0_k")?.map(EffectiveDebyeTemperature),
                theta_0_band_k: num("theta_0_band_k")?,
                theta_0_grade: grade("theta_0_grade")?,
                theta_0_channels: vocab("theta_0_channels_agree", ChannelAgreement::parse)?,
                q_channels: vocab("channels_agree", ChannelAgreement::parse)?,
                gamma_pairing: pairing,
                usable_as_anchor: field("usable_as_anchor").map(|v| v == "true"),
                v0_cm3: num("v0_cm3_per_mol")?,
                k0_gpa: num("k0_gpa")?,
                k0_prime: num("k0_prime")?,
                atoms_per_formula_unit: field("atoms_per_formula_unit")
                    .and_then(|v| v.parse::<u32>().ok()),
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
