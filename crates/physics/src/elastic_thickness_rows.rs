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

//! THE CITED ELASTIC-THICKNESS HINDCAST ROWS: what the field MEASURED for oceanic (Earth), Mars, and Venus, brought
//! from the primary-source fetch records into code with every conditioning field a comparison would need.
//!
//! # THIS IS THE DATA, NOT THE COMPARISON
//!
//! These rows are DORMANT. Nothing reads them on any run path, no derived `T_e` is scored against them, no overlap
//! is computed, and no verdict is emitted here. The comparison, the preregistered holdout, and the miss report are
//! a SEPARATE step held for the owner, because deciding what a miss means is constitutional judgment (RUNBOOK
//! section 12). This module builds the rows that step will consume, and it STOPS at the row. A method that
//! computed whether a derived `T_e` overlaps a row would have crossed that line; there is none here, by design.
//!
//! # THE ROW IS AN OBSERVATION, NEVER A TARGET
//!
//! The type is [`ObservedElasticThickness`], and the name is load-bearing. RUNBOOK section 12 is absolute that the
//! output is fit to observation NEVER: a derived `T_e` that misses these rows is the residual law firing, and
//! absorbing the miss deletes the signal and injects Terran bias at the deepest level. So there is deliberately no
//! `TargetTe` in this module and no field a derivation should hit. A row records what a field campaign measured
//! and the conditions it measured under; the engine's `T_e` is compared against it and is never nudged toward it.
//!
//! # THE REFERENCE PARTITION IS DECLARED IN THE DATA
//!
//! Each row carries a [`SampleRole`]. Earth's oceanic seamounts are a reference diagnostic because their loading
//! context is independently described. Mars and Venus are independent validation rows. The distinction is DATA
//! on the row, not a runtime choice, and neither role authorizes an inversion or supplies a canonical value. A
//! mismatch remains Residual Law evidence and cannot steer the physical realization.
//!
//! # THE ROWS ARE `T_e(elastic)`, WHICH IS WHY THE COMPARISON IS IN RIGIDITY SPACE
//!
//! Every row here is a best-fitting elastic-plate thickness ([`ElasticThicknessKind::TeElastic`]), never a
//! yield-strength-envelope moment equivalence. Calmant's oceanic `T_e` is a uniform elastic rigidity `D` scanned
//! over real bathymetry and re-expressed through an assumed `(E, nu)`, with no yield envelope, no fibre stress and
//! no moment integral (SEAMOUNT_TREATMENT_FETCH section 4.3). The Mars and Venus rows are admittance or flexural
//! effective elastic thicknesses, which are elastic-plate parameters by construction. Watts and Burov distinguish
//! `T_e(elastic)` from `T_e(YSE)` and give the ratio, so the comparison the rows license is in rigidity space,
//! each side owning its own moduli, which is what [`crate::moment_equivalence`] already emits. A row's own `(E,
//! nu)` is therefore a mandatory field: `T_e ~ (1/E)^(1/3)`, so converting a published `T_e` back to a rigidity
//! demands the pair the source assumed. Where the source does not state it, the pair is [`ModulusPair::Absent`],
//! recorded absent and never filled.
//!
//! # A ROW STATES ITS METHOD SO A CONSUMER NEVER AVERAGES ACROSS METHODS
//!
//! Earth is 3-D island flexure; Mars and Venus are admittance. These are different instruments, and where they
//! disagree the disagreement is a band, never a silent average. Each row carries its [`HindcastMethod`] so the
//! split is queryable. The method here classifies the human measurement technique, not any world content, so it
//! sits outside the emergence gate: these are external validation data, definitionally not emergent, and no row
//! ever enters a world's own `T_e` derivation.
//!
//! # THE ERRATUM RIDES A ROW (RUNBOOK section 11)
//!
//! Calmant's Table 1 prints `E = 10^12 N/m^2` (1000 GPa), refuted by the paper's own `D` range against its own
//! Table 2: that pair caps `T_e` at 20.8 km, below the paper's Mayotte at 40 km. Read as `10^12 dyn/cm^2` (10^11
//! N/m^2, 100 GPa), the same `D` range brackets Table 2 (3.56 to 44.8 km). The corrected 100 GPa SHIPS, the
//! published 1000 GPa is carried beside it, and the erratum is noted ([`ModulusPair::ReDerivedYoungsModulus`]).
//! The back-solve was reproduced independently of the fetch doc while loading this row. This row does NOT inherit
//! McNutt and Menard's `K(x_0)`/`C_2` curvature erratum: Calmant is a separate 3-D elastic fit, not a
//! moment-curvature construction, so no McNutt-derived quantity enters here.
//!
//! # THE BLINDNESS SET (what a row can still hide with every field recorded)
//!
//! 1. Mars and Venus `(E, nu)` are ABSENT, so those rows cannot be converted to a rigidity for the like-against-
//!    like comparison without sourcing the assumed pair from the underlying admittance primaries (McGovern et al.;
//!    Anderson and Smrekar), which were not read (HTTP 403, or abstract-only). The out-of-sample comparison is
//!    blocked on this until the pair is sourced.
//! 2. Earth's age convention (isochron against thermal or bathymetric) is unstated, so the isotherm cross-check
//!    carries a roughly 200 C ambiguity. The rigidity comparison does not depend on it; an isotherm-space reading
//!    would.
//! 3. Earth's `nu = 0.5` is printed but LOW-confidence (worth 7.7 percent in `T_e`), and its `E`-correction is
//!    MEDIUM (a back-solve and plausibility argument, not a printed correction).
//! 4. Mars absolute values are MEDIUM: a live Ding et al. 2019 disagreement and a 2002-to-2004 factor-of-two
//!    revision at Olympus Mons. Seven of the thirteen rows are one-sided bounds with no midpoint (a corrected
//!    count: GEOTHERM_FETCHES 3.1 prose says six, but its own Table 1 lists seven, the extra being Valles
//!    Marineris at ">= 60"; verified against the table on pull).
//! 5. Venus rows are distributional modes, not per-region values. The below-20-km mode is a NON-DETECTION (loading
//!    indistinguishable from isostasy) for 47 percent of the planet, the map has an 11 percent non-random hole,
//!    and `T_e` depends on the loading model.
//! 6. The load-class penalty (isolated-circular against chain-line-load, about a factor of 3.6) rests on Watts et
//!    al. 1988 figure 19, second-hand: that primary was not fetched.
//! 7. Whether the uniaxial-fibre yield assumption is adequate for an axisymmetric load is unmeasured in the fetch
//!    corpus. It is moot for these elastic rows (they carry no yield envelope) and open for the arc's forward
//!    solve, which does.
//!
//! # DEFAULTS TAKEN, carried forward from the fetch docs' own flags
//!
//! GEOTHERM_FETCHES.md flags: the Ding et al. 2019 Mars figures are SUMMARY-ONLY (#6); Watts and Zhong 2000 and
//! Watts 2001 were not read, so the classical `450 +/- 150` C isotherm reaches us through a secondary (#9); the
//! Venus primaries were not read (abstract-only) and Barnett et al. 2002 is SUMMARY-ONLY, and no per-region Venus
//! table was assembled (#10); the arc scope's "~600 K" against the literature's "~600 C" is surfaced and left for
//! the owner (#11). SEAMOUNT_TREATMENT_FETCH.md flags: the 11-to-40 km aspect-ratio figure and its factor of 3.6
//! are SECOND-HAND to Watts et al. 1988; Calmant's `nu = 0.5` is unresolved; and the `E = 10^12 dyn/cm^2` reading
//! is a back-solve (graded MEDIUM), not a printed statement.

use civsim_core::Fixed;

/// Which non-authoritative diagnostic partition a row belongs to.
///
/// Both partitions are observer-only evidence. Neither can provide a canonical
/// magnitude, alter a derived band, or choose a realization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleRole {
    /// A contextual reference used only to diagnose a forward-model result.
    ReferenceDiagnostic,
    /// A row reserved for independent observer-side validation.
    IndependentValidation,
}

/// WHETHER THE ROW'S `T_e` IS A PURE ELASTIC-RIGIDITY FIT OR A YIELD-STRENGTH-ENVELOPE MOMENT EQUIVALENCE.
///
/// Watts and Burov distinguish `T_e(elastic)` from `T_e(YSE)` and give the ratio (about 0.5 at `K = 10^-6 m^-1`,
/// rising to 1 for `K < 10^-8 m^-1`). Every row this arc read is the elastic kind, which is exactly why the
/// comparison is done in rigidity space rather than against a yield envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElasticThicknessKind {
    /// A best-fitting uniform elastic-plate rigidity `D`, re-expressed as `T_e` through an assumed `(E, nu)`. No
    /// yield envelope, no fibre stress, no moment integral, no curvature evaluation point. Admittance and flexural
    /// effective elastic thicknesses are this kind too.
    TeElastic,
    /// A moment-equivalence `T_e` from a yield-strength envelope evaluated at a curvature (McNutt and Menard;
    /// Watts and Burov). NO row in this arc is this kind; the variant exists so the distinction cannot be lost, and
    /// so a future row through a YSE construction has a place to declare itself.
    TeYse,
}

/// THE MEASUREMENT METHOD, so a consumer never averages across methods.
///
/// This classifies the human instrument that produced the number, never world content, so it is outside the
/// emergence gate. The membership is the set of techniques read for this arc and grows by adding a variant as new
/// hindcast sources are read, the same status as [`crate::creep_rows::Modality`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HindcastMethod {
    /// A 3-D numerical thin-elastic-plate fit: the plate equation integrated over the real bathymetry grid, the
    /// best-fitting rigidity `D` scanned and re-expressed as `T_e` (Calmant et al. 1990). "3-D" names the LOAD
    /// GEOMETRY, not the yield surface: the model is purely elastic.
    ThreeDNumericalPlateFit,
    /// Localized gravity/topography admittance and correlation spectra (McGovern et al. for Mars, compiled by
    /// Ruiz; the global admittance mapping of Anderson and Smrekar for Venus). An effective-elastic-thickness
    /// estimator.
    GravityTopographyAdmittance,
    /// Flexural modelling of a specific load (Ruiz's polar caps; Barnett et al. for Venus).
    FlexuralModelling,
}

/// AN OBSERVED ELASTIC THICKNESS, never a target, in the SHAPE the source reports it.
///
/// The variants are the mathematical forms a bounded-uncertainty measurement takes in the literature read for this
/// arc: a two-sided interval, a one-sided bound in either direction, and a rate relation. Endpoints are in
/// kilometres. A consumer reads the endpoints to build its comparison; this type performs NO comparison and NO
/// rigidity conversion (the module's dormancy line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedElasticThickness {
    /// A two-sided interval `[low, high]` km, the form Ruiz reports for most Mars regions ("43 to 65").
    Interval { low_km: Fixed, high_km: Fixed },
    /// A one-sided LOWER bound: `T_e >= min`, the upper side open. A consumer scores against the BOUND; there is
    /// no midpoint (Ruiz's "> 300" is a load the lithosphere barely deflects under, not 300 km measured).
    LowerBound { min_km: Fixed },
    /// A one-sided UPPER bound: `T_e <= max`, the lower side open (Ruiz's "< 12"; Venus's below-20-km mode).
    UpperBound { max_km: Fixed },
    /// A rate relation `T_e(dt) = coefficient * sqrt(dt)`, with `dt` the plate age at loading in Ma (Calmant eq.
    /// 6). The rate standard deviation is the SD OF THE FITTED RATE, NOT a predictive band for a single locality,
    /// so this variant stores the rate and its SD and deliberately exposes NO `T_e` interval: turning it into a
    /// comparable `T_e` at a chosen `dt`, and deciding how to treat the rate SD against the raw scatter, is the
    /// consumer's constitutional call, not this row's.
    AgeRateRelation {
        coefficient_km_per_sqrt_ma: Fixed,
        rate_sd: Fixed,
    },
}

impl ObservedElasticThickness {
    /// The lower endpoint in km where the observation bounds `T_e` from below (an interval's low, a lower bound's
    /// minimum), else `None`. An upper bound is open below; a rate relation has no fixed `T_e`. This exposes what
    /// is available to a future consumer without converting anything.
    pub fn defined_low_km(self) -> Option<Fixed> {
        match self {
            ObservedElasticThickness::Interval { low_km, .. } => Some(low_km),
            ObservedElasticThickness::LowerBound { min_km } => Some(min_km),
            ObservedElasticThickness::UpperBound { .. }
            | ObservedElasticThickness::AgeRateRelation { .. } => None,
        }
    }

    /// The upper endpoint in km where the observation bounds `T_e` from above (an interval's high, an upper bound's
    /// maximum), else `None`. A lower bound is open above; a rate relation has no fixed `T_e`.
    pub fn defined_high_km(self) -> Option<Fixed> {
        match self {
            ObservedElasticThickness::Interval { high_km, .. } => Some(high_km),
            ObservedElasticThickness::UpperBound { max_km } => Some(max_km),
            ObservedElasticThickness::LowerBound { .. }
            | ObservedElasticThickness::AgeRateRelation { .. } => None,
        }
    }
}

/// THE `(E, nu)` A ROW'S `T_e` IS CONDITIONED ON, which a rigidity conversion needs and the source may not state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModulusPair {
    /// The source does not state `(E, nu)`. Recorded ABSENT, never filled (the arc's non-invention rule). A
    /// consumer cannot convert this row's `T_e` to a rigidity without sourcing the pair elsewhere, and that
    /// blockage is a finding, not a gap to paper over.
    Absent,
    /// A stated pair, both values the source's own.
    Stated {
        youngs_modulus_gpa: Fixed,
        poisson_ratio: Fixed,
    },
    /// A stated pair whose printed Young's modulus FAILED TO REPRODUCE from the source's own numbers and ships
    /// RE-DERIVED (RUNBOOK section 11). The corrected value is operative; the published value is carried beside it
    /// and never dropped silently, and the erratum states the slip.
    ReDerivedYoungsModulus {
        published_gpa: Fixed,
        corrected_gpa: Fixed,
        poisson_ratio: Fixed,
        erratum: &'static str,
    },
}

impl ModulusPair {
    /// The `(E, nu)` a rigidity conversion would use: the CORRECTED modulus where an erratum applies, the stated
    /// pair otherwise, and `None` where the source states no pair. This is the value that ships; the published
    /// figure is retrievable from the variant for anyone auditing the correction.
    pub fn operative(self) -> Option<(Fixed, Fixed)> {
        match self {
            ModulusPair::Absent => None,
            ModulusPair::Stated {
                youngs_modulus_gpa,
                poisson_ratio,
            } => Some((youngs_modulus_gpa, poisson_ratio)),
            ModulusPair::ReDerivedYoungsModulus {
                corrected_gpa,
                poisson_ratio,
                ..
            } => Some((corrected_gpa, poisson_ratio)),
        }
    }
}

/// ONE CITED HINDCAST ROW: an observed elastic thickness plus every conditioning field a comparison would need.
///
/// A row missing a field is a statistic with a hidden variable, so every field the sources supply is carried, and
/// every field they do not is recorded absent in [`ModulusPair::Absent`] or stated so in [`Self::conditioning`].
#[derive(Clone, Copy, Debug)]
pub struct ElasticThicknessRow {
    /// The body the row measures ("Earth", "Mars", "Venus"), a cited provenance label rather than an engine world.
    pub body: &'static str,
    /// A short identifier for the locality, region, or relation.
    pub region: &'static str,
    /// The observed elastic thickness, in the shape the source reports it.
    pub observed: ObservedElasticThickness,
    /// Elastic fit or yield-envelope moment equivalence.
    pub kind: ElasticThicknessKind,
    /// The measurement method.
    pub method: HindcastMethod,
    /// The `(E, nu)` the row is conditioned on, absent where the source does not state it.
    pub modulus_pair: ModulusPair,
    /// The observer-only evidence partition.
    pub sample_role: SampleRole,
    /// The load class or geometry, as the source characterizes it.
    pub load_class: &'static str,
    /// The epoch or loading-age conditioning, as the source states it.
    pub epoch: &'static str,
    /// The citation: author, year, journal or DOI, and the specific table, equation, or page.
    pub citation: &'static str,
    /// The conditioning notes: exclusions, weighting, hazards, verify-on-pull grades, and defaults carried forward.
    pub conditioning: &'static [&'static str],
}

/// EARTH, oceanic seamounts: Calmant, Francheteau and Cazenave 1990, equation (6).
///
/// `T_e (km) = 2.70 sqrt(dt)`, `dt` the plate age at loading in Ma, rate standard deviation 0.15. The row is the
/// relation, not a single `T_e`: the fit is a weighted regression over their Table 2 seamount set, conditioned on
/// dropping the anomalously low south-central Pacific subpopulation. Its `(E, nu)` carries the `E`-unit erratum.
pub fn calmant_oceanic_seamounts() -> ElasticThicknessRow {
    ElasticThicknessRow {
        body: "Earth",
        region: "oceanic_seamounts_te_vs_sqrt_loading_age",
        observed: ObservedElasticThickness::AgeRateRelation {
            // Calmant eq. (6): the coefficient 2.70 km per sqrt(Ma), and the rate SD 0.15, both the paper's own
            // printed digits from its summary and "Analysis of results".
            coefficient_km_per_sqrt_ma: Fixed::from_ratio(270, 100),
            rate_sd: Fixed::from_ratio(15, 100),
        },
        kind: ElasticThicknessKind::TeElastic,
        method: HindcastMethod::ThreeDNumericalPlateFit,
        modulus_pair: ModulusPair::ReDerivedYoungsModulus {
            // Table 1 prints E = 10^12 N/m^2 = 1000 GPa (refuted by the paper's own D range against its Table 2).
            published_gpa: Fixed::from_int(1000),
            // Read as 10^12 dyn/cm^2 = 10^11 N/m^2 = 100 GPa, the reading its own D range and Table 2 support.
            corrected_gpa: Fixed::from_int(100),
            // nu = 0.5 printed, LOW-confidence, uncorrected: no basis in the sources to change it.
            poisson_ratio: Fixed::from_ratio(1, 2),
            erratum: "Calmant Table 1 prints E = 10^12 N/m^2 (1000 GPa), refuted by the paper's own D range \
                      (5e20 to 1e24 N m) against its own Table 2 (that pair caps Te at 20.8 km, below Mayotte's \
                      40 km). Read as 10^12 dyn/cm^2 = 100 GPa the range brackets Table 2 (3.56 to 44.8 km); the \
                      corrected value ships (RUNBOOK section 11). Cause: a lost CGS unit, the classic value in the \
                      older flexure literature. Grade MEDIUM: a back-solve, not a printed correction. nu = 0.5 is \
                      printed but unconfirmed (LOW), worth 7.7 percent in Te.",
        },
        sample_role: SampleRole::ReferenceDiagnostic,
        load_class: "oceanic intraplate volcanic (seamounts and oceanic islands), modelled as an axisymmetric 3-D \
                     load over real bathymetry; isolated-circular against chain-line-load is a per-load choice \
                     priced at about a factor of 3.6 (Watts et al. 1988, second-hand)",
        epoch: "dt = plate age at loading, in Ma; the relation's free variable, not a fixed epoch",
        citation: "Calmant, S., Francheteau, J. and Cazenave, A., 1990, Geophys. J. Int. 100(1), 59-67, DOI \
                   10.1111/j.1365-246X.1990.tb04568.x: eq. (6); Table 1 (E, nu, D scan); Table 2 (the fitted \
                   seamount set)",
        conditioning: &[
            "EXCLUSION: the 2.70 rate holds only 'Excluding the anomalously low estimates from the south-central \
             Pacific', a real subpopulation dropped for a stated physical reason (a change in thermal structure). \
             A hindcast that includes that region is checked against a curve that excluded it.",
            "WEIGHTED regression: Te values were weighted by their error bars.",
            "RATE SD 0.15 is the standard deviation of the FITTED RATE, NOT the predictive band for a single \
             locality: the raw data carry 'a very large scatter'. (2.70 +/- 0.15) sqrt(dt) must not be read as a \
             locality band.",
            "AGE CONVENTION: ABSENT. The sources read do not state whether dt is isochron-derived or thermal or \
             bathymetric. It conditions the isotherm cross-check: the same Te data imply about 550 to 600 C \
             against thermal or bathymetric age and about 350 to 450 C against isochron age (McNutt 1984 via \
             Calmant); the 2.70 rate itself corresponds to the 350 to 450 C band. Carried, not resolved.",
            "ELASTIC vs YSE: Te_elastic. Calmant's Te is a best-fitting uniform elastic-plate rigidity D (scanned \
             5e20 to 1e24 N m, integrated over the real SEASAT bathymetry), re-expressed as Te through the assumed \
             pair. No yield envelope, no fibre stress, no moment integral (SEAMOUNT_TREATMENT_FETCH 4.3). This is \
             why the arc's comparison is in rigidity space.",
            "This row does NOT inherit McNutt and Menard's K(x_0)/C_2 curvature erratum: Calmant is a separate 3-D \
             elastic-D fit, not a moment-curvature construction, so no McNutt-derived quantity enters here.",
            "VERIFY-ON-PULL: 2.70, 0.15, the exclusion, and the weighted regression are cross-checked against both \
             fetch docs and quoted from Calmant's eq. (6) and summary; the E-erratum back-solve was reproduced \
             here, independently of the fetch, through eq. (2) on the printed D range against Table 2.",
            "DEFAULT TAKEN (GEOTHERM_FETCHES #9): Watts and Zhong 2000 and Watts 2001 were not read; the classical \
             450 +/- 150 C isotherm statement reaches us through a secondary (the Emperor Seamount study).",
            "DEFAULT TAKEN (GEOTHERM_FETCHES #11): the arc scope's '~600 K' matches no isotherm the literature \
             states (all in C); surfaced for the owner, not resolved here.",
        ],
    }
}

/// The oceanic (Earth) hindcast rows: the single Calmant relation, the arc's only Mirror-class instance.
pub fn earth_oceanic_rows() -> [ElasticThicknessRow; 1] {
    [calmant_oceanic_seamounts()]
}

/// Conditioning shared by every Mars row: the hazards and defaults that ride Ruiz's whole Table 1.
const MARS_SHARED_CONDITIONING: &[&str] = &[
    "ONE-SIDED BOUNDS: SEVEN of the thirteen rows are inequalities, not intervals (four lower bounds: north pole \
     > 300, south pole > 110, Valles Marineris >= 60, Olympus Mons > 70; three upper bounds: Noachis Terra < 12, \
     Terra Cimmeria < 12, Hellas Basin < 13). A consumer scores against the bound; there is no midpoint (Ruiz's \
     '> 300' is a load the lithosphere barely deflects under, not 300 km measured). CORRECTED COUNT: \
     GEOTHERM_FETCHES 3.1 prose says 'six', but its own Table 1 lists seven; the extra is Valles Marineris at \
     '>= 60'. Verified against the table on pull; the fetch prose miscounted by one.",
    "COMPILATION CAVEAT: Ruiz records that many of these Te estimates 'are not adequate for performing \
     well-constrained heat flow calculations' because the load curvatures were not always derived.",
    "MOVING TARGET: the McGovern et al. 2004 correction revised several 2002 estimates downward by up to a factor \
     of two (Olympus Mons lower bound 70 km, not 140 km). Any older citation is stale.",
    "DISAGREEMENT (SUMMARY-ONLY, GEOTHERM_FETCHES #6): Ding et al. 2019 report figures that do not sit on Ruiz's \
     (Olympus > 105 against > 70; Noachian southern highlands 20 to 60 against < 12). Reported, not adjudicated.",
    "(E, nu) ABSENT: Ruiz Table 1 and the fetch do not state the (E, nu) the admittance studies assumed, so the \
     row's rigidity conversion is blocked until that pair is sourced from the underlying primaries.",
    "PRIMARIES NOT READ DIRECTLY: the Mars admittance primaries (McGovern et al. 2002 and the 2004 correction) \
     returned HTTP 403; the row comes from Ruiz's compilation of them. Confidence MEDIUM on absolute values.",
    "VERIFY-ON-PULL: each Te value, epoch, and surface age is transcribed from Ruiz Table 1 as the fetch records \
     it; cross-checked against the fetch, not re-read from the paywalled primary.",
];

/// MARS: Ruiz 2014 Table 1, thirteen regions each with its epoch. A compilation of localized gravity/topography
/// admittance (McGovern et al., corrected 2004) for most regions and flexural modelling for the polar caps. Every
/// row is out-of-sample, and every row's `(E, nu)` is absent.
pub fn mars_ruiz_rows() -> [ElasticThicknessRow; 13] {
    // The shared spine of a Mars row; only the region, observed value, method, load class, and epoch differ.
    fn mars_row(
        region: &'static str,
        observed: ObservedElasticThickness,
        method: HindcastMethod,
        load_class: &'static str,
        epoch: &'static str,
    ) -> ElasticThicknessRow {
        ElasticThicknessRow {
            body: "Mars",
            region,
            observed,
            kind: ElasticThicknessKind::TeElastic,
            method,
            modulus_pair: ModulusPair::Absent,
            sample_role: SampleRole::IndependentValidation,
            load_class,
            epoch,
            citation: "Ruiz, J., 2014, Sci. Rep. 4, 4338, DOI 10.1038/srep04338, Table 1 (compiling McGovern et \
                       al. 2004; Zuber et al. 2000; Ruiz et al. 2011; Grott, Kiefer and Wieczorek)",
            conditioning: MARS_SHARED_CONDITIONING,
        }
    }
    let admittance = HindcastMethod::GravityTopographyAdmittance;
    let flexural = HindcastMethod::FlexuralModelling;
    let regional = "regional load (gravity/topography admittance); Ruiz Table 1 does not separately state the load \
                    geometry";
    [
        mars_row(
            "north_pole",
            ObservedElasticThickness::LowerBound {
                min_km: Fixed::from_int(300),
            },
            flexural,
            "north polar cap load (flexural modelling of a specific load)",
            "current",
        ),
        mars_row(
            "south_pole",
            ObservedElasticThickness::LowerBound {
                min_km: Fixed::from_int(110),
            },
            flexural,
            "south polar cap load (flexural modelling of a specific load)",
            "current",
        ),
        mars_row(
            "valles_marineris",
            ObservedElasticThickness::LowerBound {
                min_km: Fixed::from_int(60),
            },
            admittance,
            regional,
            "Hesperian-Amazonian, surface age 3.6 to 1.8 Ga",
        ),
        mars_row(
            "alba_patera",
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(43),
                high_km: Fixed::from_int(65),
            },
            admittance,
            regional,
            "Hesperian-Amazonian, surface age < 3.5 Ga",
        ),
        mars_row(
            "arsia_mons",
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(20),
                high_km: Fixed::from_int(35),
            },
            admittance,
            regional,
            "Hesperian or Amazonian, surface age ~3.5 Ga or lower",
        ),
        mars_row(
            "pavonis_mons",
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(50),
                high_km: Fixed::from_int(100),
            },
            admittance,
            regional,
            "Hesperian or Amazonian, surface age ~3.6 Ga or lower",
        ),
        mars_row(
            "ascraeus_mons",
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(50),
                high_km: Fixed::from_int(80),
            },
            admittance,
            regional,
            "Hesperian or Amazonian, surface age ~3.6 Ga or lower",
        ),
        mars_row(
            "olympus_mons",
            ObservedElasticThickness::LowerBound {
                min_km: Fixed::from_int(70),
            },
            admittance,
            regional,
            "Hesperian-Amazonian, surface age 3.7 to 2.5 Ga",
        ),
        mars_row(
            "elysium_rise",
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(36),
                high_km: Fixed::from_int(45),
            },
            admittance,
            regional,
            "Hesperian, surface age 3.7 to 3.0 Ga",
        ),
        mars_row(
            "isidis_planitia",
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(100),
                high_km: Fixed::from_int(180),
            },
            admittance,
            regional,
            "Hesperian, surface age 3.7 to 3.0 Ga",
        ),
        mars_row(
            "noachis_terra",
            ObservedElasticThickness::UpperBound {
                max_km: Fixed::from_int(12),
            },
            admittance,
            regional,
            "Noachian, surface age > 3.8 Ga",
        ),
        mars_row(
            "terra_cimmeria",
            ObservedElasticThickness::UpperBound {
                max_km: Fixed::from_int(12),
            },
            admittance,
            regional,
            "Noachian, surface age > 3.8 Ga",
        ),
        mars_row(
            "hellas_basin",
            ObservedElasticThickness::UpperBound {
                max_km: Fixed::from_int(13),
            },
            admittance,
            regional,
            "Noachian, surface age 4.1 to 3.9 Ga",
        ),
    ]
}

/// Conditioning shared by every Venus row: the distribution, coverage, and model dependence Smrekar and Anderson
/// report, plus the fetch's Venus defaults.
const VENUS_SHARED_CONDITIONING: &[&str] = &[
    "TRIMODAL DISTRIBUTION: Smrekar and Anderson report three common ranges of Te (< 20 km, 40 to 70 km, and a few \
     locations > 90 km). These three rows are the modes of one distribution, not per-region values.",
    "47 PERCENT below 20 km: for that near-half of the planet 'we cannot distinguish loading from isostasy', so \
     the below-20-km row is a NON-DETECTION (tectonically inactive), not a measured thickness.",
    "GLOBAL SPAN: the global admittance mapping gives an overall Te range of 0 to 100 km; previous local \
     admittance studies gave 5 to 55 km. Recorded as context for the modes.",
    "COVERAGE: best-fit models fit 26 of 35 spectral classes (89 percent of the surface); the remaining 11 percent \
     is a NON-RANDOM hole (large-amplitude top-loading signatures the model class could not fit).",
    "MODEL DEPENDENCE: 'estimates of Te varied significantly between bottom loading and top or hot spot models'. A \
     Venus Te without its loading model attached is incomplete.",
    "(E, nu) ABSENT: the abstract read does not state the (E, nu) the admittance inversion assumed, so the rigidity \
     conversion is blocked until it is sourced.",
    "DEFAULT TAKEN (GEOTHERM_FETCHES #10): the Venus primaries were not read (this is the 2005 LPSC abstract of \
     Anderson and Smrekar 2006); Barnett et al. 2002 (flexural, Te about 10 to 40 km, and 20 to 60 km for seven \
     volcano-like structures) is SUMMARY-ONLY; no per-region Venus table was assembled.",
    "VERIFY-ON-PULL: the ranges, the trimodality, the 47 percent, and the coverage are transcribed from the \
     abstract as the fetch records it; cross-checked against the fetch, not re-read from the primary.",
];

/// VENUS: Smrekar and Anderson 2005, the global admittance mapping, reported as a trimodal distribution. Three
/// mode rows, all out-of-sample, all with absent `(E, nu)`.
pub fn venus_smrekar_anderson_rows() -> [ElasticThicknessRow; 3] {
    fn venus_row(
        region: &'static str,
        observed: ObservedElasticThickness,
        load_class: &'static str,
    ) -> ElasticThicknessRow {
        ElasticThicknessRow {
            body: "Venus",
            region,
            observed,
            kind: ElasticThicknessKind::TeElastic,
            method: HindcastMethod::GravityTopographyAdmittance,
            modulus_pair: ModulusPair::Absent,
            sample_role: SampleRole::IndependentValidation,
            load_class,
            epoch: "global admittance map (Magellan gravity and topography); no per-region epoch stated",
            citation: "Smrekar, S. E. and Anderson, F. S., 2005, LPSC XXXVI abstract 1804 (the conference version \
                       of Anderson and Smrekar 2006, J. Geophys. Res. Planets 111, E08006, DOI \
                       10.1029/2004JE002395)",
            conditioning: VENUS_SHARED_CONDITIONING,
        }
    }
    [
        venus_row(
            "mode_te_below_20",
            ObservedElasticThickness::UpperBound {
                max_km: Fixed::from_int(20),
            },
            "47 percent of the planet, where loading cannot be distinguished from isostasy (tectonically \
             inactive): a NON-DETECTION, not a measured 20 km",
        ),
        venus_row(
            "mode_te_40_to_70",
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(40),
                high_km: Fixed::from_int(70),
            },
            "the intermediate mode of the trimodal distribution",
        ),
        venus_row(
            "mode_te_above_90",
            ObservedElasticThickness::LowerBound {
                min_km: Fixed::from_int(90),
            },
            "a few locations, the strong-plate tail of the distribution",
        ),
    ]
}

/// ALL HINDCAST ROWS: Earth's one Mirror-class relation, thirteen Mars rows, and three Venus mode rows, in one
/// list for a consumer that iterates the whole set. The set is dormant: assembling it reads nothing on a run path.
pub fn all_hindcast_rows() -> Vec<ElasticThicknessRow> {
    let mut rows = Vec::new();
    rows.extend(earth_oceanic_rows());
    rows.extend(mars_ruiz_rows());
    rows.extend(venus_smrekar_anderson_rows());
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f64_of(x: Fixed) -> f64 {
        x.to_f64_lossy()
    }

    #[test]
    fn earth_row_carries_calmant_eq6_as_a_rate_relation_never_a_target() {
        let row = calmant_oceanic_seamounts();
        assert_eq!(row.body, "Earth");
        assert_eq!(row.sample_role, SampleRole::ReferenceDiagnostic);
        assert_eq!(row.kind, ElasticThicknessKind::TeElastic);
        assert_eq!(row.method, HindcastMethod::ThreeDNumericalPlateFit);
        match row.observed {
            ObservedElasticThickness::AgeRateRelation {
                coefficient_km_per_sqrt_ma,
                rate_sd,
            } => {
                // The paper's own printed digits: 2.70 and 0.15.
                assert_eq!(coefficient_km_per_sqrt_ma, Fixed::from_ratio(270, 100));
                assert_eq!(rate_sd, Fixed::from_ratio(15, 100));
            }
            other => panic!("Calmant's row is a rate relation, not {other:?}"),
        }
        // A rate relation exposes NO Te interval: converting it at a chosen dt is the consumer's call, not the
        // row's, so both endpoints are open here.
        assert_eq!(row.observed.defined_low_km(), None);
        assert_eq!(row.observed.defined_high_km(), None);
    }

    #[test]
    fn earth_modulus_erratum_ships_corrected_and_carries_published() {
        let row = calmant_oceanic_seamounts();
        match row.modulus_pair {
            ModulusPair::ReDerivedYoungsModulus {
                published_gpa,
                corrected_gpa,
                poisson_ratio,
                ..
            } => {
                assert_eq!(published_gpa, Fixed::from_int(1000)); // 10^12 N/m^2
                assert_eq!(corrected_gpa, Fixed::from_int(100)); // 10^12 dyn/cm^2
                assert_ne!(
                    published_gpa, corrected_gpa,
                    "both values are carried and they are not the same number"
                );
                assert_eq!(poisson_ratio, Fixed::from_ratio(1, 2));
            }
            other => panic!("the Earth row carries the E-unit erratum, not {other:?}"),
        }
        // The operative pair is the CORRECTED modulus, which is what a rigidity conversion would use.
        let (e, nu) = row.modulus_pair.operative().expect("the pair ships");
        assert_eq!(e, Fixed::from_int(100));
        assert_eq!(nu, Fixed::from_ratio(1, 2));
    }

    #[test]
    fn the_e_correction_reproduces_calmants_own_d_range_against_its_own_table2() {
        // TWIN, computed in f64 OUTSIDE the codebase's fixed-point path, from Calmant's OWN printed numbers: the D
        // scan 5e20 to 1e24 N m (their p. 60) and Table 2's Mayotte at 40 km. Te = (12 D (1 - nu^2) / E)^(1/3),
        // their eq. (2) inverted. This is the RUNBOOK section 11 conviction: the printed pair cannot make the
        // paper's own table, the CGS reading can.
        let te_km =
            |d: f64, e_pa: f64, nu: f64| (12.0 * d * (1.0 - nu * nu) / e_pa).cbrt() / 1000.0;
        let (d_lo, d_hi) = (5.0e20, 1.0e24);
        let mayotte_km = 40.0;

        // Printed pair E = 10^12 N/m^2, nu = 0.5: caps Te below Mayotte, so it is refuted by the paper's own table.
        let printed_hi = te_km(d_hi, 1.0e12, 0.5);
        assert!(
            printed_hi < mayotte_km,
            "printed pair caps Te at {printed_hi:.2} km, below Mayotte's 40 km: refuted"
        );
        assert!(
            (printed_hi - 20.8).abs() < 0.2,
            "printed-pair max is 20.8 km"
        );

        // Corrected pair E = 10^11 N/m^2 (10^12 dyn/cm^2), nu = 0.5: brackets Table 2's ~4 to 40 km span.
        let corrected_lo = te_km(d_lo, 1.0e11, 0.5);
        let corrected_hi = te_km(d_hi, 1.0e11, 0.5);
        assert!(
            corrected_lo < 4.0 && corrected_hi >= mayotte_km,
            "corrected pair spans {corrected_lo:.2} to {corrected_hi:.2} km, bracketing Table 2"
        );
        assert!((corrected_lo - 3.56).abs() < 0.05 && (corrected_hi - 44.8).abs() < 0.1);
    }

    #[test]
    fn mars_is_thirteen_out_of_sample_rows_with_absent_moduli() {
        let rows = mars_ruiz_rows();
        assert_eq!(rows.len(), 13);
        for r in &rows {
            assert_eq!(r.body, "Mars");
            assert_eq!(r.sample_role, SampleRole::IndependentValidation);
            assert_eq!(r.kind, ElasticThicknessKind::TeElastic);
            assert_eq!(
                r.modulus_pair,
                ModulusPair::Absent,
                "Ruiz Table 1 does not state (E, nu), so it is recorded absent, never filled"
            );
            assert_eq!(r.modulus_pair.operative(), None);
        }
        // The polar caps are flexural modelling; the rest are admittance.
        let by_region = |name: &str| rows.iter().find(|r| r.region == name).unwrap().method;
        assert_eq!(by_region("north_pole"), HindcastMethod::FlexuralModelling);
        assert_eq!(by_region("south_pole"), HindcastMethod::FlexuralModelling);
        assert_eq!(
            by_region("olympus_mons"),
            HindcastMethod::GravityTopographyAdmittance
        );
        // Spot-check the value shapes against Ruiz Table 1's own printed entries.
        let observed = |name: &str| rows.iter().find(|r| r.region == name).unwrap().observed;
        assert_eq!(
            observed("olympus_mons"),
            ObservedElasticThickness::LowerBound {
                min_km: Fixed::from_int(70)
            }
        );
        assert_eq!(
            observed("noachis_terra"),
            ObservedElasticThickness::UpperBound {
                max_km: Fixed::from_int(12)
            }
        );
        assert_eq!(
            observed("isidis_planitia"),
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(100),
                high_km: Fixed::from_int(180)
            }
        );
    }

    #[test]
    fn seven_of_the_thirteen_mars_rows_are_one_sided_bounds() {
        // VERIFY-ON-PULL CATCH: GEOTHERM_FETCHES 3.1 prose says "six of the thirteen rows are inequalities", but
        // its own Ruiz Table 1 lists SEVEN (Valles Marineris at ">= 60" is the seventh, undercounted by the
        // prose). The transcribed data is faithful to the table, so the verified count is seven: four lower bounds
        // and three upper bounds, six two-sided intervals, thirteen total. The bound is scored against directly;
        // there is no midpoint.
        let rows = mars_ruiz_rows();
        let lower = rows
            .iter()
            .filter(|r| matches!(r.observed, ObservedElasticThickness::LowerBound { .. }))
            .count();
        let upper = rows
            .iter()
            .filter(|r| matches!(r.observed, ObservedElasticThickness::UpperBound { .. }))
            .count();
        let interval = rows
            .iter()
            .filter(|r| matches!(r.observed, ObservedElasticThickness::Interval { .. }))
            .count();
        assert_eq!(
            lower, 4,
            "north pole, south pole, Valles Marineris, Olympus"
        );
        assert_eq!(upper, 3, "Noachis Terra, Terra Cimmeria, Hellas Basin");
        assert_eq!(
            lower + upper,
            7,
            "the fetch prose 'six' undercounts its own table"
        );
        assert_eq!(interval, 6);
        assert_eq!(lower + upper + interval, 13);
    }

    #[test]
    fn venus_is_three_out_of_sample_modes_with_absent_moduli() {
        let rows = venus_smrekar_anderson_rows();
        assert_eq!(rows.len(), 3);
        for r in &rows {
            assert_eq!(r.body, "Venus");
            assert_eq!(r.sample_role, SampleRole::IndependentValidation);
            assert_eq!(r.method, HindcastMethod::GravityTopographyAdmittance);
            assert_eq!(r.modulus_pair, ModulusPair::Absent);
        }
        let observed = |name: &str| rows.iter().find(|r| r.region == name).unwrap().observed;
        assert_eq!(
            observed("mode_te_below_20"),
            ObservedElasticThickness::UpperBound {
                max_km: Fixed::from_int(20)
            }
        );
        assert_eq!(
            observed("mode_te_40_to_70"),
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(40),
                high_km: Fixed::from_int(70)
            }
        );
        assert_eq!(
            observed("mode_te_above_90"),
            ObservedElasticThickness::LowerBound {
                min_km: Fixed::from_int(90)
            }
        );
    }

    #[test]
    fn defined_endpoints_expose_only_the_bounded_side() {
        let interval = ObservedElasticThickness::Interval {
            low_km: Fixed::from_int(40),
            high_km: Fixed::from_int(70),
        };
        assert_eq!(interval.defined_low_km(), Some(Fixed::from_int(40)));
        assert_eq!(interval.defined_high_km(), Some(Fixed::from_int(70)));

        let lower = ObservedElasticThickness::LowerBound {
            min_km: Fixed::from_int(90),
        };
        assert_eq!(lower.defined_low_km(), Some(Fixed::from_int(90)));
        assert_eq!(lower.defined_high_km(), None); // open above

        let upper = ObservedElasticThickness::UpperBound {
            max_km: Fixed::from_int(12),
        };
        assert_eq!(upper.defined_low_km(), None); // open below
        assert_eq!(upper.defined_high_km(), Some(Fixed::from_int(12)));
    }

    #[test]
    fn the_observer_only_reference_partitions_are_stable() {
        let rows = all_hindcast_rows();
        assert_eq!(rows.len(), 1 + 13 + 3);
        let reference = rows
            .iter()
            .filter(|r| r.sample_role == SampleRole::ReferenceDiagnostic)
            .count();
        let independent = rows
            .iter()
            .filter(|r| r.sample_role == SampleRole::IndependentValidation)
            .count();
        assert_eq!(
            reference, 1,
            "only Earth's oceanic row is the reference diagnostic"
        );
        assert_eq!(independent, 16);
        assert!(independent > reference);
        let earth = rows
            .iter()
            .find(|r| r.sample_role == SampleRole::ReferenceDiagnostic)
            .unwrap();
        assert_eq!(earth.body, "Earth");
    }

    #[test]
    fn every_row_carries_a_method_a_role_and_a_citation() {
        // A row missing a field is a statistic with a hidden variable: prove the load-bearing fields are populated.
        for r in &all_hindcast_rows() {
            assert!(!r.body.is_empty());
            assert!(!r.region.is_empty());
            assert!(!r.citation.is_empty());
            assert!(!r.load_class.is_empty());
            assert!(!r.epoch.is_empty());
            assert!(!r.conditioning.is_empty());
        }
    }

    #[test]
    fn earth_te_at_a_sample_age_matches_calmants_coefficient() {
        // NOT a comparison and NOT a target: this only confirms the stored coefficient reproduces the paper's own
        // relation arithmetic, Te = 2.70 sqrt(dt), at an illustrative loading age. No derived Te, no overlap.
        let row = calmant_oceanic_seamounts();
        if let ObservedElasticThickness::AgeRateRelation {
            coefficient_km_per_sqrt_ma,
            ..
        } = row.observed
        {
            let dt_ma = 100.0_f64; // a round loading age, purely to exercise the stored coefficient
            let te = f64_of(coefficient_km_per_sqrt_ma) * dt_ma.sqrt();
            assert!(
                (te - 27.0).abs() < 0.05,
                "2.70 sqrt(100) = 27 km, got {te:.3}"
            );
        } else {
            panic!("Calmant's row is a rate relation");
        }
    }
}
