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

//! THE PARAMETERIZED-CONVECTION SCALING CONSTANTS: the prefactor `a` and the critical Rayleigh numbers of the
//! boundary-layer heat-flow law `Nu = a * (Ra / Ra_crit)^(1/3)`, read from the vendored cited-data column
//! (`data/convection_scaling.toml`, witnessed by `data/convection_scaling/`).
//!
//! # WHAT IT SUPPLIES, AND THE ONE MODELING SEAM IT CARRIES
//!
//! The deep-time interior lift reads two constants from here: the critical Rayleigh number `Ra_crit` (the
//! marginal-stability eigenvalue, which the ONE shared thermal boundary layer `delta = d (Ra_crit/Ra)^(1/3)`
//! normalizes against, so a layer at onset conducts the whole depth) and the Nusselt PREFACTOR `a` (which
//! multiplies the convective heat-loss flux, never the lid `delta`).
//!
//! `Ra_crit` is CONDITIONED ON THE MECHANICAL BOUNDARY CONDITION: free-free `657.511 = 27 pi^4 / 4` (Rayleigh
//! 1916, analytic), rigid-rigid `1707.762` (Pellew and Southwell 1940), and RIGID-FREE `1100.65` (a free surface
//! over a near-rigid base), which is the planetary-mantle case and the one the single-lid prefactor keys off.
//!
//! `a` IS CONVENTION-DEPENDENT, an O(1) number, not universal: `a = 1.0` is the single-boundary-layer (peel-away)
//! BASAL-heated form (Stevenson et al. 1983, Komacek and Abbot 2016, Vazan et al. 2018), `a = 2^(-4/3) ~ 0.397` the
//! symmetric two-boundary-layer INTERNAL-heated form. Which applies is NOT a taste but the WORLD'S OWN HEATING
//! CONFIGURATION (owner ruling 2026-07-18, the residue rule: a convention selects on somebody's state, so make it
//! the world's state, never the literature's default): [`ConvectionScaling::nusselt_prefactor_at_internal_fraction`]
//! DERIVES `a` from the internal-heating fraction (the Urey-class ratio of radiogenic production to surface loss),
//! the two cited endpoints as the band. The deep-time model is `heat_production`-only with NO basal core-flux term,
//! so its fraction is 1 and its prefactor is the internal `0.397` (picking `1.0` would import the literature's
//! basal-heated favourite and run every interior 2.5x too cold); the day a core-flux term lands, the dispatch is
//! already shaped. The bare coefficient `C = 0.294` of the un-normalized `Nu = C Ra^(1/3)` is a DIFFERENT
//! normalization and is carried as its own row so it can never be read into the `a` slot.
//!
//! # THE STAGNANT-LID FAMILY, A SECOND FORM RATHER THAN A SECOND VALUE
//!
//! Everything above is the MOBILE-LID instance, and most modelled bodies are not that. Where a
//! temperature-dependent viscosity locks a cold lid (Mars-class, Venus-class, one-plate worlds), only a thin warm
//! sublayer convects, and the heat loss is suppressed by a power of the Frank-Kamenetskii parameter `theta`, the
//! ratio of the layer's temperature drop to the drop over which the viscosity changes by about `e`. The form is
//! `Nu = alpha theta^gamma Ra^beta`, and its three numbers are ONE convention: `alpha` means nothing apart from
//! the `gamma` and `beta` it was fitted with, or apart from the `theta` and Rayleigh-number DEFINITIONS it was
//! fitted against. So a stagnant-lid row is read as a whole [`StagnantLidConvention`] or not at all, and a row
//! missing either exponent is not one (which is what keeps the mobile-lid rows above, whose `beta` field is
//! documentation, out of this reader).
//!
//! FOUR ROWS ARE BANKED AND THEY ARE NOT FOUR ANSWERS. Batra and Foley 2021 fit the linearized Frank-Kamenetskii
//! family on their own models and state it as TWO BRANCHES OF ONE PRESCRIPTION: `nu_stag_steady_C2` on the steady
//! branch, `nu_stag_time_dependent_C1` on the time-dependent one, and their eq. (11) evaluates both and takes the
//! LARGER, the transition falling where the two cross. Schulz et al. 2020 ran a full Arrhenius viscosity WITH an
//! activation volume on this engine's own creep bank and got a visibly shallower `theta` exponent, banked as
//! `nu_stag_arrhenius_internal_ra` and `nu_stag_arrhenius_harmonic_ra`, which differ in the viscosity average
//! their Rayleigh number is formed on, so reading one against the other's `Ra` is the same class of normalization
//! error the bare `C` guards against. At an evaluated state the column therefore reduces to at most THREE
//! cross-model determinations, and [`ConvectionScaling::stagnant_lid_determinations`] is where that happens: it
//! applies each family's own data-declared selection rule, reports which branch stood, and preflights every row
//! against the scope its source declares. The kernel it consumes is [`crate::laws::ln_stagnant_lid_nusselt`], and
//! the `theta` that kernel takes is [`crate::laws::stagnant_lid_rheological_theta`].
//!
//! THIS COMMENT PREVIOUSLY READ AS THOUGH A CALLER PICKED THE BATRA BRANCH BY DECLARING ITS CONVECTION PATTERN,
//! retired 2026-07-19 rather than edited around. Eq. (11) takes the larger branch, so the pattern is an OUTCOME of
//! the state, and a caller declaring it would be authoring the transition the source derives.
//!
//! NO ROW HERE IS FITTED AT THIS ENGINE'S OWN STRESS EXPONENT, so for the production world (pressure-bearing
//! Arrhenius dry-olivine dislocation creep at `n = 3.5`) the strict in-scope subset is EMPTY and the evaluation
//! returns the typed [`NoInScopeDetermination`] refusal, carrying the test that convicted each family and, where
//! the numbers were formable at all, an out-of-scope sensitivity envelope INSIDE the refusal. That gap is real, it
//! is recorded at the kernel, and it is why nothing here is promoted to a default.

use std::path::Path;

use civsim_core::Fixed;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ScalingFile {
    #[serde(default)]
    constant: Vec<ConstantRaw>,
}

#[derive(Debug, Deserialize)]
struct ConstantRaw {
    name: String,
    value: String,
    #[serde(default)]
    band_lo: Option<String>,
    #[serde(default)]
    band_hi: Option<String>,
    #[serde(default)]
    citation: Option<String>,
    /// The `gamma` of a stagnant-lid `Nu = alpha theta^gamma Ra^beta` row. Deliberately NOT named `gamma` or
    /// reusing the mobile-lid rows' documentation-only `beta` field: a row is readable as a stagnant-lid
    /// convention exactly when it carries BOTH of these, so the naming is the guard.
    #[serde(default)]
    theta_exponent: Option<String>,
    /// The `beta` of a stagnant-lid row, under a name the mobile-lid rows do not use.
    #[serde(default)]
    rayleigh_exponent: Option<String>,
    /// The family whose branches this row is one of: the rows a single source prescription evaluates TOGETHER.
    #[serde(default)]
    family: Option<String>,
    /// How that family's own prescription picks between its branches (`larger_branch`, `sole_determination`).
    #[serde(default)]
    family_selection: Option<String>,
    /// The held receipt the row was read from, so a consumer can see when two rows lean on ONE study.
    #[serde(default)]
    witness: Option<String>,
    /// The SYMBOL for the Rayleigh number this row's fit was performed against, beside the row's own prose
    /// `rayleigh_definition`. Two rows carry the same key only where their prose says they are the same quantity.
    #[serde(default)]
    rayleigh_definition_key: Option<String>,
    /// The symbol for the Frank-Kamenetskii parameter this row's fit was performed against.
    #[serde(default)]
    theta_definition_key: Option<String>,
    /// The stress exponent the FIT was performed at (`1` for a Newtonian fitting set).
    #[serde(default)]
    fitted_stress_exponent: Option<String>,
    /// The `theta` intervals the fit was MEASURED over, as the source states them rather than as one hull across
    /// them. A single sampled value is carried as a degenerate interval.
    #[serde(default)]
    fitted_theta_ranges: Option<Vec<(String, String)>>,
}

/// The mechanical boundary condition a critical Rayleigh number is conditioned on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryCondition {
    /// Both boundaries stress-free (the analytic `27 pi^4 / 4`).
    FreeFree,
    /// Both boundaries no-slip.
    RigidRigid,
    /// A free surface over a near-rigid base: the PLANETARY-MANTLE case.
    RigidFree,
}

impl BoundaryCondition {
    fn row_name(self) -> &'static str {
        match self {
            BoundaryCondition::FreeFree => "ra_crit_free_free",
            BoundaryCondition::RigidRigid => "ra_crit_rigid_rigid",
            BoundaryCondition::RigidFree => "ra_crit_rigid_free",
        }
    }
}

/// One scaling constant: the cited value and, where the literature disagrees, its band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalingConstant {
    /// The primary cited value.
    pub value: Fixed,
    /// The low edge of the band (the honest uncertainty), where one exists.
    pub band_lo: Option<Fixed>,
    /// The high edge of the band.
    pub band_hi: Option<Fixed>,
}

/// One cited STAGNANT-LID scaling convention, read as a whole: `Nu = coefficient * theta^theta_exponent *
/// Ra^rayleigh_exponent`.
///
/// THE THREE NUMBERS TRAVEL TOGETHER because none of them means anything alone. The coefficient was fitted
/// against those exponents, and both were fitted against a particular `theta` definition and a particular
/// Rayleigh-number definition (an internal viscosity, a harmonic mean, a reference state), which the row's own
/// `theta_definition` and `rayleigh_definition` fields record in prose. Handing a consumer a bare coefficient
/// would let it be multiplied by the wrong Rayleigh number, which is the error the sibling bare `C` row exists
/// to prevent in the mobile-lid family.
///
/// Consumed by [`crate::laws::ln_stagnant_lid_nusselt`]. The band, where a row carries one, is the honest
/// spread across the studies the row's citation names, never a tolerance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StagnantLidConvention {
    /// `alpha`, the fitted prefactor. Positive.
    pub coefficient: Fixed,
    /// `gamma`, the Frank-Kamenetskii exponent. Negative in every banked row: a stiffer lid suppresses heat loss.
    pub theta_exponent: Fixed,
    /// `beta`, the Rayleigh exponent. Positive: a more vigorous interior loses more heat.
    pub rayleigh_exponent: Fixed,
    /// The low edge of the coefficient's band, where the row carries one.
    pub band_lo: Option<Fixed>,
    /// The high edge of the coefficient's band.
    pub band_hi: Option<Fixed>,
}

/// A Rayleigh number a caller can TRUTHFULLY form, tagged with the definition it was formed under.
///
/// # WHY THE DEFINITION IS A TYPE AND NOT A COMMENT
///
/// A stagnant-lid coefficient was fitted against ONE Rayleigh number, and the banked definitions are different
/// quantities: the internal `Ra_i` on the viscosity at the interior temperature (Batra and Foley 2021, section 5),
/// the internal `Ra_i` on the system's MINIMUM viscosity, located at the interior temperature and pressure in the
/// middle of the convecting cell (Schulz et al. 2020, eq. 34), and `Ra_har` on the HARMONIC mean of the viscosity
/// beneath the lid (their eq. 35). Feeding one row's coefficient with another row's Rayleigh number is the
/// normalization error the bare coefficient `C` guards against in the mobile-lid family, one level deeper and
/// harder to see, because both numbers are called `Ra` and both are dimensionless.
///
/// So a caller states which Rayleigh number it formed by asking the column for that definition
/// ([`ConvectionScaling::rayleigh_projection`]), and a row is evaluated only against a projection carrying the
/// definition the row itself declares. A key no banked row declares does not construct at all, so an invented or
/// mistyped definition fails at the call site rather than matching nothing in silence.
///
/// THE MEMBERSHIP IS DATA (Principle 11): the definitions are the `rayleigh_definition_key` values the column's
/// own rows carry, so a new source with a new convention is a new row and never a code change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RayleighProjection<'a> {
    definition: &'a str,
    ln_rayleigh: Fixed,
}

impl<'a> RayleighProjection<'a> {
    /// The definition key this Rayleigh number was formed under.
    pub fn definition(&self) -> &'a str {
        self.definition
    }

    /// The log-domain Rayleigh number itself, in the domain [`crate::laws::ln_rayleigh_number`] produces.
    pub fn ln_rayleigh(&self) -> Fixed {
        self.ln_rayleigh
    }
}

/// A Frank-Kamenetskii parameter a caller can truthfully form, tagged with the definition it was formed under.
///
/// The same discipline as [`RayleighProjection`], on the other axis a stagnant-lid fit is scoped to. The banked
/// definitions are two: the LINEARIZED, pressure-free `theta` of the Frank-Kamenetskii studies (Batra and Foley
/// 2021, whose models carry no activation volume and no pressure term at all), and the PRESSURE-CARRYING `theta`
/// of the full Arrhenius study (Schulz et al. 2020, eq. 29, which is the form
/// [`crate::laws::stagnant_lid_rheological_theta`] implements). Handing a pressure-carrying `theta` to a fit that
/// was performed on a pressure-free one is a laundering of the same kind as the Rayleigh error, so the type
/// refuses it rather than a comment warning against it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThetaProjection<'a> {
    definition: &'a str,
    theta: Fixed,
}

impl<'a> ThetaProjection<'a> {
    /// The definition key this Frank-Kamenetskii parameter was formed under.
    pub fn definition(&self) -> &'a str {
        self.definition
    }

    /// The parameter itself.
    pub fn theta(&self) -> Fixed {
        self.theta
    }
}

/// The world state a stagnant-lid determination is evaluated at.
///
/// THE CONVECTION PATTERN IS NOT AN INPUT, and its absence is the point. Batra and Foley's steady and
/// time-dependent branches are not a caller's choice between two laws: their eq. (11) evaluates BOTH and takes the
/// larger, so the pattern is an OUTCOME of the state and is reported back as
/// [`FamilyDetermination::selected_row`]. A dispatch on a declared pattern would author the transition the source
/// derives, and it would put a regime label in a causal position, which this engine's own tectonic-regime
/// contract forbids.
#[derive(Clone, Copy, Debug)]
pub struct StagnantLidState<'a> {
    /// The world's OWN stress exponent, from the creep rows its viscosity was solved on. `1` is Newtonian. The
    /// banked fits were all performed on Newtonian sets, so a world running dislocation creep at `n = 3.5` is out
    /// of every banked scope, which is a refusal rather than an extrapolation to be taken quietly.
    pub stress_exponent: Fixed,
    /// Every Rayleigh number the caller can truthfully form, each tagged. A caller supplies what it HAS: the
    /// engine forms no spatial harmonic mean beneath the lid today, so it supplies no `Ra_har`, and the row that
    /// was fitted against one is convicted rather than fed the nearest available number.
    pub rayleigh: &'a [RayleighProjection<'a>],
    /// Every Frank-Kamenetskii parameter the caller can truthfully form, each tagged.
    pub theta: &'a [ThetaProjection<'a>],
}

/// How one family's own source prescription picks between the rows that are its branches.
///
/// The membership is data (each row's `family_selection`); the rules are fixed Rust, and a rule this engine does
/// not implement is [`ScopeFailure::UnknownSelectionRule`], a typed stop rather than a quiet fallback to whichever
/// rule happens to be first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionRule {
    /// The source evaluates every branch and the LARGER one stands, so the transition between branches falls where
    /// they cross and nothing declares which branch applies (Batra and Foley 2021, eq. 11).
    LargerBranch,
    /// The source states ONE fit for this convention, so the family is that row.
    SoleDetermination,
}

impl SelectionRule {
    fn parse(s: &str) -> Option<SelectionRule> {
        match s {
            "larger_branch" => Some(SelectionRule::LargerBranch),
            "sole_determination" => Some(SelectionRule::SoleDetermination),
            _ => None,
        }
    }
}

/// The test that convicted a row, or a family's own declaration, in the scope preflight.
///
/// Every variant is a REFUSAL and none is a warning: a convicted row is not evaluated into the canonical world at
/// all. The two kinds are distinguished by [`ScopeFailure::permits_sensitivity_evaluation`], which asks whether
/// the law could still be evaluated (every input formable, only the fitted regime wrong) or whether there is no
/// number to form in the first place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeFailure {
    /// The caller formed no Rayleigh number under the definition this fit was performed against.
    RayleighDefinitionUnavailable { required: String },
    /// The caller formed no Frank-Kamenetskii parameter under the definition this fit was performed against.
    ThetaDefinitionUnavailable { required: String },
    /// The state carries MORE THAN ONE value under a definition this row reads. One state has one value for one
    /// quantity, so taking whichever came first would be a silent selection between two numbers the caller
    /// believes are both the state.
    AmbiguousProjection { definition: String },
    /// The fit was performed at one stress exponent and the world runs another. Matched EXACTLY, deliberately: a
    /// fit performed at one exponent states no validity at any other, and any width allowed here would be
    /// invented rather than read.
    StressExponent { fitted: Fixed, world: Fixed },
    /// The evaluated `theta` lies outside every interval the fit was measured over, so applying the row would be
    /// an extrapolation past every condition its source ran.
    ThetaOutsideFittedSpan {
        theta: Fixed,
        span_lo: Fixed,
        span_hi: Fixed,
    },
    /// The row does not transcribe the scope field this test needs, so the test could not run. An unrun check
    /// reads as a block and never as a pass.
    ScopeNotTranscribed { field: &'static str },
    /// The family declares a selection rule this engine does not implement.
    UnknownSelectionRule { rule: String },
    /// Two rows of one family disagree about their own selection rule, which is a defect in the column.
    InconsistentSelectionRule { declared: String, sibling: String },
    /// A family declaring itself a sole determination carries more than one row.
    OverfullSoleDetermination { rows: usize },
    /// The law kernel refused: a non-positive `theta` or coefficient, or an unrepresentable intermediate.
    KernelRefused,
}

impl ScopeFailure {
    /// Whether a row convicted by this test can still be EVALUATED, so its number may ride an out-of-scope
    /// sensitivity report.
    ///
    /// True exactly where every input the law needs was formable and only the fit's own regime failed. False where
    /// there is no number to form: an absent projection leaves the formula with an empty slot, and the one way to
    /// fill it would be to substitute a quantity the fit was not performed against, which is the error the typed
    /// projections exist to prevent.
    pub fn permits_sensitivity_evaluation(&self) -> bool {
        matches!(
            self,
            ScopeFailure::StressExponent { .. } | ScopeFailure::ThetaOutsideFittedSpan { .. }
        )
    }
}

impl std::fmt::Display for ScopeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeFailure::RayleighDefinitionUnavailable { required } => write!(
                f,
                "the caller formed no Rayleigh number under the definition '{required}' this fit was performed against"
            ),
            ScopeFailure::ThetaDefinitionUnavailable { required } => write!(
                f,
                "the caller formed no theta under the definition '{required}' this fit was performed against"
            ),
            ScopeFailure::AmbiguousProjection { definition } => write!(
                f,
                "the state carries more than one value under the definition '{definition}'"
            ),
            ScopeFailure::StressExponent { fitted, world } => write!(
                f,
                "the fit was performed at stress exponent {} and the world runs {}",
                fitted.to_f64_lossy(),
                world.to_f64_lossy()
            ),
            ScopeFailure::ThetaOutsideFittedSpan {
                theta,
                span_lo,
                span_hi,
            } => write!(
                f,
                "theta {} lies outside the fitted span [{}, {}]",
                theta.to_f64_lossy(),
                span_lo.to_f64_lossy(),
                span_hi.to_f64_lossy()
            ),
            ScopeFailure::ScopeNotTranscribed { field } => write!(
                f,
                "the row does not transcribe '{field}', so that scope test could not run"
            ),
            ScopeFailure::UnknownSelectionRule { rule } => {
                write!(f, "the family declares an unimplemented selection rule '{rule}'")
            }
            ScopeFailure::InconsistentSelectionRule { declared, sibling } => write!(
                f,
                "the family's rows disagree about their selection rule ('{declared}' against '{sibling}')"
            ),
            ScopeFailure::OverfullSoleDetermination { rows } => write!(
                f,
                "a sole-determination family carries {rows} rows"
            ),
            ScopeFailure::KernelRefused => {
                write!(f, "the law kernel refused this row's inputs")
            }
        }
    }
}

/// Something the preflight could not test, or tested and wants seen, on a row that is otherwise in scope.
///
/// A caveat never blocks: it rides the determination so a reader sees what the number is standing on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeCaveat {
    /// The evaluated `theta` is inside the span the fit was measured over but between two of its intervals. The
    /// fit is a functional family, so interpolating inside its own sampled span is what a fit is for; landing
    /// between two separate fitting sets is worth seeing, above all where the source ran OTHER experiments in
    /// that space and excluded them (Schulz et al. 2020, whose dislocation runs at theta 11.4 to 12.8 sit between
    /// the two Newtonian sets their eqs. 34 and 35 were fitted over).
    ThetaBetweenFittedIntervals { row: String, theta: Fixed },
    /// The row declares an empty fitted span, which is its source stating that the relation is not a fit over
    /// sampled conditions. The row's own prose carries why.
    SourceStatesNoFittedSpan { row: String },
}

impl std::fmt::Display for ScopeCaveat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeCaveat::ThetaBetweenFittedIntervals { row, theta } => write!(
                f,
                "{row}: theta {} falls between two of the fit's own intervals",
                theta.to_f64_lossy()
            ),
            ScopeCaveat::SourceStatesNoFittedSpan { row } => {
                write!(f, "{row}: the source states no fitted parameter span")
            }
        }
    }
}

/// One family's determination at the evaluated state: a single cross-model answer, with the branch its own source
/// prescription selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyDetermination {
    /// The family key the rows declare.
    pub family: String,
    /// The row the family's own selection rule picked at the point value. For a `larger_branch` family this is
    /// the branch that was larger at THIS state, which is the pattern the source's transition puts here. Its full
    /// citation is [`ConvectionScaling::citation`] at this name.
    ///
    /// ONE CORNER WHERE THE NAME CARRIES NOTHING: at the conductive floor every branch reads `Nu = 1`, so the
    /// branches tie and the earlier row is reported for determinism. A body that conducts has no convective
    /// pattern to name, so there is nothing there to lose.
    pub selected_row: String,
    /// The held receipt the selected row was read from, where the row names one. Two determinations sharing a
    /// witness are one study seen twice, never two independent agreements.
    pub witness: Option<String>,
    /// `ln Nu` at the rows' cited coefficients, clamped at the conductive limit (`Nu >= 1`) by the kernel.
    pub ln_nusselt: Fixed,
    /// `ln Nu` at the edges of the selected coefficients' OWN reported spread, where any row of the family carries
    /// one. The band is expressed here in `ln Nu` and nowhere in `alpha`: a band over coefficients fitted with
    /// different exponents against different Rayleigh definitions is not a quantity.
    pub ln_nusselt_band: Option<(Fixed, Fixed)>,
    /// What the preflight could not test, or tested and wants seen.
    pub caveats: Vec<ScopeCaveat>,
}

/// One row the preflight convicted, with every test that convicted it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowExclusion {
    /// The row name.
    pub row: String,
    /// EVERY failing test, never the first one only: a row can be out of scope on several axes at once, and
    /// reporting one would understate what it would take to bring it in.
    pub failures: Vec<ScopeFailure>,
}

/// One family the preflight excluded.
///
/// A `larger_branch` family is ALL OR NOTHING: its source prescription is the larger of its branches, so a family
/// with a convicted branch cannot apply it. Taking the maximum over the surviving branches would be a different
/// quantity, and one that can only be too small, since the missing branch is the one that might have been larger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyExclusion {
    /// The family key, or the row's own name where the row declares no family.
    pub family: String,
    /// Failures of the family's own declaration (an unknown, missing or inconsistent selection rule).
    pub family_failures: Vec<ScopeFailure>,
    /// Every convicted member, with every test that convicted it.
    pub convicted: Vec<RowExclusion>,
}

/// The in-scope stagnant-lid determinations at one state, kept DISCRETE.
///
/// THE MEMBERS ARE RETAINED RATHER THAN COLLAPSED. Each determination is one source prescription's answer, and the
/// space between two of them holds no model at all: nobody fitted the interior of the spread, so a hull over it
/// carries no support. [`Self::ln_nusselt_hull`] reports the envelope for a sensitivity statement and is named to
/// say what it is; the members are what a consumer should read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagnantLidEnsemble {
    /// The discrete in-scope determinations, one per family, in the column's own row order.
    pub determinations: Vec<FamilyDetermination>,
    /// Every family the preflight excluded, carried so an in-scope answer still shows what it left out.
    pub excluded: Vec<FamilyExclusion>,
}

impl StagnantLidEnsemble {
    /// The envelope of the determinations in `ln Nu`, over the members AND their bands.
    ///
    /// A REPORTING ENVELOPE, never a distribution: the interior of the hull is not supported by anything, because
    /// the determinations inside it are discrete answers from separate prescriptions rather than samples of one.
    /// `None` when there are no determinations.
    pub fn ln_nusselt_hull(&self) -> Option<(Fixed, Fixed)> {
        let mut lo: Option<Fixed> = None;
        let mut hi: Option<Fixed> = None;
        for d in &self.determinations {
            let (d_lo, d_hi) = match d.ln_nusselt_band {
                Some((band_lo, band_hi)) => (band_lo.min(d.ln_nusselt), band_hi.max(d.ln_nusselt)),
                None => (d.ln_nusselt, d.ln_nusselt),
            };
            lo = Some(lo.map_or(d_lo, |v: Fixed| v.min(d_lo)));
            hi = Some(hi.map_or(d_hi, |v: Fixed| v.max(d_hi)));
        }
        Some((lo?, hi?))
    }
}

/// THE TYPED REFUSAL: at this state, no banked stagnant-lid determination is in scope.
///
/// This is what the production world gets today, and it is the honest answer rather than a failure of the reader.
/// The engine's admitted rheology is pressure-bearing Arrhenius dry-olivine DISLOCATION creep at `n = 3.5`, and
/// every banked coefficient was fitted on a Newtonian set: Batra and Foley's models carry no stress dependence at
/// all, and Schulz et al. fitted their eqs. (34) and (35) over diffusion-creep and reduced-enthalpy runs while
/// warning in their own words against applying them to dislocation creep. So the strict in-scope subset is EMPTY,
/// and the refusal says which test convicted each family rather than advancing the nearest number.
///
/// The [`Self::sensitivity`] envelope rides INSIDE the refusal deliberately. A caller cannot reach it without
/// handling the refusal, so an out-of-scope extrapolation can be reported, compared and argued about, and cannot
/// silently become the world's heat loss.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoInScopeDetermination {
    /// Every family, with the tests that convicted it. Empty only where the column banks no stagnant-lid rows.
    pub excluded: Vec<FamilyExclusion>,
    /// The out-of-REGIME evaluations, for a sensitivity report ONLY. A family appears here when every input it
    /// needed was formable and only its fitted regime failed, so a number exists and is an extrapolation.
    pub sensitivity: Vec<FamilyDetermination>,
}

impl std::fmt::Display for NoInScopeDetermination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "no banked stagnant-lid determination is in scope at this state ({} famil{} convicted)",
            self.excluded.len(),
            if self.excluded.len() == 1 { "y" } else { "ies" }
        )?;
        for family in &self.excluded {
            write!(f, "; {}", family.family)?;
            for failure in &family.family_failures {
                write!(f, ": {failure}")?;
            }
            for row in &family.convicted {
                for failure in &row.failures {
                    write!(f, ": {} {failure}", row.row)?;
                }
            }
        }
        if !self.sensitivity.is_empty() {
            write!(
                f,
                "; {} out-of-scope sensitivity evaluation(s) available",
                self.sensitivity.len()
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for NoInScopeDetermination {}

/// Why loading the column failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalingError {
    /// The TOML did not parse.
    Parse(String),
    /// A value string was not a legal decimal.
    BadValue { name: String, detail: String },
    /// The column carries no source citation.
    MissingProvenance,
    /// The column holds no rows.
    Empty,
}

impl std::fmt::Display for ScalingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalingError::Parse(m) => write!(f, "convection-scaling parse error: {m}"),
            ScalingError::BadValue { name, detail } => {
                write!(
                    f,
                    "convection-scaling value '{name}' could not be read: {detail}"
                )
            }
            ScalingError::MissingProvenance => {
                write!(f, "convection-scaling column carries no source citation")
            }
            ScalingError::Empty => write!(f, "convection-scaling column holds no rows"),
        }
    }
}

impl std::error::Error for ScalingError {}

/// One parsed row: the constant every consumer reads, plus the two exponents and the declared scope only a
/// stagnant-lid row carries.
///
/// EVERY SCOPE FIELD IS AN `Option` AND AN ABSENT ONE IS NEVER A PASS. A row that does not transcribe its family,
/// its definitions, its fitted rheology or its fitted span cannot be scope-checked, so the preflight convicts it
/// with [`ScopeFailure::ScopeNotTranscribed`] rather than selecting it against a test that never ran.
#[derive(Clone, Debug)]
struct Row {
    name: String,
    constant: ScalingConstant,
    theta_exponent: Option<Fixed>,
    rayleigh_exponent: Option<Fixed>,
    family: Option<String>,
    selection: Option<String>,
    witness: Option<String>,
    rayleigh_definition: Option<String>,
    theta_definition: Option<String>,
    fitted_stress_exponent: Option<Fixed>,
    fitted_theta_ranges: Option<Vec<(Fixed, Fixed)>>,
    citation: String,
}

impl Row {
    /// Whether the row is readable as a stagnant-lid convention at all: a suppression law carries BOTH exponents,
    /// and a mobile-lid row carries neither under these names.
    fn is_stagnant_lid(&self) -> bool {
        self.theta_exponent.is_some() && self.rayleigh_exponent.is_some()
    }
}

/// The cited parameterized-convection scaling constants, keyed by name.
#[derive(Clone, Debug)]
pub struct ConvectionScaling {
    constants: Vec<Row>,
}

fn parse_decimal(name: &str, s: &str) -> Result<Fixed, ScalingError> {
    Fixed::from_decimal_str(s.trim()).map_err(|detail| ScalingError::BadValue {
        name: name.to_string(),
        detail,
    })
}

impl ConvectionScaling {
    /// Parse the column, requiring a source citation.
    pub fn from_toml_str(s: &str) -> Result<Self, ScalingError> {
        let file: ScalingFile =
            toml::from_str(s).map_err(|e| ScalingError::Parse(e.to_string()))?;
        if file.constant.is_empty() {
            return Err(ScalingError::Empty);
        }
        let mut constants = Vec::with_capacity(file.constant.len());
        for c in &file.constant {
            // Provenance is PER ROW (this column's idiom): every constant carries its own citation, and an
            // uncited row is a fabricated value that must fail to load rather than enter silently.
            let citation = c.citation.as_deref().map(str::trim).unwrap_or("");
            if citation.is_empty() {
                return Err(ScalingError::MissingProvenance);
            }
            let citation = citation.to_string();
            let value = parse_decimal(&c.name, &c.value)?;
            let band_lo = match &c.band_lo {
                Some(s) => Some(parse_decimal(&c.name, s)?),
                None => None,
            };
            let band_hi = match &c.band_hi {
                Some(s) => Some(parse_decimal(&c.name, s)?),
                None => None,
            };
            let theta_exponent = match &c.theta_exponent {
                Some(s) => Some(parse_decimal(&c.name, s)?),
                None => None,
            };
            let rayleigh_exponent = match &c.rayleigh_exponent {
                Some(s) => Some(parse_decimal(&c.name, s)?),
                None => None,
            };
            let fitted_stress_exponent = match &c.fitted_stress_exponent {
                Some(s) => Some(parse_decimal(&c.name, s)?),
                None => None,
            };
            let fitted_theta_ranges = match &c.fitted_theta_ranges {
                Some(pairs) => {
                    let mut out = Vec::with_capacity(pairs.len());
                    for (lo, hi) in pairs {
                        let lo = parse_decimal(&c.name, lo)?;
                        let hi = parse_decimal(&c.name, hi)?;
                        // An inverted interval is a transcription defect, and a silent swap would hide it. The
                        // degenerate case (a single sampled value) is legal and is how a point sample is carried.
                        if hi < lo {
                            return Err(ScalingError::BadValue {
                                name: c.name.trim().to_string(),
                                detail: format!(
                                    "fitted_theta_ranges holds an inverted interval [{}, {}]",
                                    lo.to_f64_lossy(),
                                    hi.to_f64_lossy()
                                ),
                            });
                        }
                        out.push((lo, hi));
                    }
                    Some(out)
                }
                None => None,
            };
            let trimmed = |s: &Option<String>| -> Option<String> {
                s.as_deref()
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
            };
            constants.push(Row {
                name: c.name.trim().to_string(),
                constant: ScalingConstant {
                    value,
                    band_lo,
                    band_hi,
                },
                theta_exponent,
                rayleigh_exponent,
                family: trimmed(&c.family),
                selection: trimmed(&c.family_selection),
                witness: trimmed(&c.witness),
                rayleigh_definition: trimmed(&c.rayleigh_definition_key),
                theta_definition: trimmed(&c.theta_definition_key),
                fitted_stress_exponent,
                fitted_theta_ranges,
                citation,
            });
        }
        Ok(ConvectionScaling { constants })
    }

    /// Load from a path (the runtime read).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ScalingError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ScalingError::Parse(format!("read error: {e}")))?;
        Self::from_toml_str(&text)
    }

    /// The standard vendored column, embedded at compile time.
    pub fn standard() -> Result<Self, ScalingError> {
        Self::from_toml_str(include_str!("../data/convection_scaling.toml"))
    }

    /// A constant by name, or `None` if it is absent.
    pub fn constant(&self, name: &str) -> Option<ScalingConstant> {
        self.constants
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.constant)
    }

    /// A STAGNANT-LID convention by row name, or `None` when the row is absent or is not one.
    ///
    /// A row qualifies only when it carries BOTH exponents, so the mobile-lid rows can never be read here: the
    /// prefactor `a` row's `beta` field is documentation under a different key, and it has no `theta_exponent`
    /// at all, so it refuses rather than arriving as a convention with a silently missing suppression term.
    ///
    /// THE NAME IS THE CALLER'S, DELIBERATELY. There is no `stagnant_lid_default()`, because the four banked
    /// conventions differ by convection pattern (steady against time-dependent) and by rheology (a linearized
    /// Frank-Kamenetskii viscosity against a full Arrhenius one carrying pressure), and which applies is a
    /// property of the world being modelled. A default would make that choice invisible at the call site, which
    /// is the residue rule's failure mode. The four row names are `nu_stag_time_dependent_C1`,
    /// `nu_stag_steady_C2`, `nu_stag_arrhenius_internal_ra` and `nu_stag_arrhenius_harmonic_ra`; each row's own
    /// `regime`, `theta_definition` and `rayleigh_definition` fields in the column state what it is scoped to.
    ///
    /// THIS IS THE TRANSCRIPTION READ AND NOT THE SELECTION. It hands back one row's triple for a provenance test
    /// or a referee example, and it checks nothing about the state: a caller that feeds the triple to the kernel
    /// with whatever Rayleigh number it has to hand has performed no scope check at all, which is the very error
    /// the typed projections exist to prevent. Selecting a convention for a world goes through
    /// [`Self::stagnant_lid_determinations`], which applies each family's own prescription and refuses where the
    /// state is outside every banked scope.
    pub fn stagnant_lid_convention(&self, name: &str) -> Option<StagnantLidConvention> {
        let row = self.constants.iter().find(|r| r.name == name)?;
        Some(StagnantLidConvention {
            coefficient: row.constant.value,
            theta_exponent: row.theta_exponent?,
            rayleigh_exponent: row.rayleigh_exponent?,
            band_lo: row.constant.band_lo,
            band_hi: row.constant.band_hi,
        })
    }

    /// The Nusselt PREFACTOR `a` band for `Nu = a (Ra/Ra_crit)^(1/3)`, its value the single-boundary-layer basal
    /// endpoint and its band spanning the two conventions. Prefer [`Self::nusselt_prefactor_at_internal_fraction`],
    /// which selects within the band from the world's own heating configuration.
    pub fn nusselt_prefactor(&self) -> Option<ScalingConstant> {
        self.constant("nu_ra_prefactor_a")
    }

    /// The heating-conditioned Nusselt prefactor `a`, DERIVED from the world's own internal-heating fraction rather
    /// than an authored convention (owner ruling 2026-07-18, the residue rule: a convention selects on somebody's
    /// state, so it is made the WORLD's state). `internal_fraction` is the fraction of the interior's heat budget
    /// that is INTERNAL (radiogenic), the Urey-class ratio: `1` for a purely internally-heated mantle (the
    /// deep-time model today, `heat_production` with no basal core-flux term) and `0` for a purely basal-heated
    /// one. The prefactor runs continuously between the two cited endpoints, `a = 1.0` (single-boundary-layer,
    /// basal) at `f = 0` down to `a = 2^(-4/3) ~ 0.397` (symmetric two-boundary-layer, internal) at `f = 1`:
    /// `a = a_basal - f (a_basal - a_internal)`. `None` if the prefactor band is absent.
    pub fn nusselt_prefactor_at_internal_fraction(
        &self,
        internal_fraction: Fixed,
    ) -> Option<Fixed> {
        let c = self.nusselt_prefactor()?;
        let a_basal = c.band_hi?;
        let a_internal = c.band_lo?;
        let f = internal_fraction.clamp(Fixed::ZERO, Fixed::ONE);
        let span = a_basal.checked_sub(a_internal)?;
        a_basal.checked_sub(f.checked_mul(span)?)
    }

    /// One row's full citation, the human-readable half of its provenance. A determination reports the row it
    /// selected, and this is where that row says what it was read from and where.
    pub fn citation(&self, name: &str) -> Option<&str> {
        self.constants
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.citation.as_str())
    }

    /// The critical Rayleigh number (marginal-stability eigenvalue) for a mechanical boundary condition.
    pub fn critical_rayleigh(&self, bc: BoundaryCondition) -> Option<Fixed> {
        self.constant(bc.row_name()).map(|c| c.value)
    }

    /// A tagged Rayleigh number for a definition the COLUMN declares, or `None` for a key no banked row uses.
    ///
    /// This is the only way to build a [`RayleighProjection`], so a caller cannot invent a definition, and a
    /// mistyped one fails here rather than matching nothing later. The caller is asserting a fact about the number
    /// it formed, and the assertion is checkable against the row's own `rayleigh_definition` prose.
    pub fn rayleigh_projection(
        &self,
        definition_key: &str,
        ln_rayleigh: Fixed,
    ) -> Option<RayleighProjection<'_>> {
        let declared = self.constants.iter().find_map(|r| {
            r.rayleigh_definition
                .as_deref()
                .filter(|d| *d == definition_key)
        })?;
        Some(RayleighProjection {
            definition: declared,
            ln_rayleigh,
        })
    }

    /// A tagged Frank-Kamenetskii parameter for a definition the column declares, or `None` for a key no banked
    /// row uses. The sibling of [`Self::rayleigh_projection`] on the other scoped axis.
    pub fn theta_projection(
        &self,
        definition_key: &str,
        theta: Fixed,
    ) -> Option<ThetaProjection<'_>> {
        let declared = self.constants.iter().find_map(|r| {
            r.theta_definition
                .as_deref()
                .filter(|d| *d == definition_key)
        })?;
        Some(ThetaProjection {
            definition: declared,
            theta,
        })
    }

    /// EVERY BANKED STAGNANT-LID DETERMINATION, evaluated at a world state and preflighted against its own declared
    /// scope. The production entry point, and the one that may refuse.
    ///
    /// # WHAT AN EVALUATED STATE REDUCES THE COLUMN TO
    ///
    /// Four rows are banked and they are not four independent answers. The two Batra and Foley rows are ONE
    /// state-dispatched family: their eq. (11) evaluates the steady and the time-dependent branch and takes the
    /// LARGER, so at any state the pair yields a single determination and the branch that won is an outcome to be
    /// reported rather than a choice to be made. The two Schulz rows are separate determinations because they were
    /// fitted against different Rayleigh numbers. So a state sees at most THREE cross-model determinations, and the
    /// selection rule that collapses the pair is read from the data (`family_selection`), never wired here.
    ///
    /// # THE PREFLIGHT, AND WHY IT REFUSES RATHER THAN PICKS
    ///
    /// A row may be evaluated only where the state matches what the row was FITTED against. The tests, each of them
    /// a plain reading of the row's own transcribed scope: the caller must have formed the row's own Rayleigh
    /// number and the row's own `theta` (typed, so the nearest available number can never be substituted); the
    /// world's stress exponent must equal the exponent the fit was performed at, matched exactly because a fit at
    /// one exponent states no validity at another; the `theta` must lie inside the span the fit was measured over;
    /// and a row that does not transcribe one of those scopes is convicted rather than passed, because a check that
    /// did not run is not a check that passed. A family whose own selection rule is missing, unknown or
    /// inconsistent across its rows is convicted whole.
    ///
    /// Where every family is convicted the return is [`NoInScopeDetermination`], which carries the tests that
    /// convicted each one and, where the numbers were formable, an out-of-scope sensitivity envelope INSIDE the
    /// refusal. That is the production answer today: the engine runs pressure-bearing Arrhenius dislocation creep
    /// at `n = 3.5` and every banked coefficient was fitted on a Newtonian set.
    ///
    /// Where at least one family is in scope the return is a [`StagnantLidEnsemble`] whose determinations stay
    /// DISCRETE. Any band it carries is in `ln Nu`, never in `alpha`: the coefficients were fitted with different
    /// exponents against different Rayleigh definitions, so a spread over them is not a quantity, while a spread
    /// over the heat transport the rival prescriptions predict at ONE state is.
    // @derives: the in-scope stagnant-lid log-domain Nusselt determinations and their reporting envelope <- the world's stress exponent and its tagged Rayleigh and Frank-Kamenetskii projections, each cited row's coefficient and two exponents, and each row's own declared fitting scope
    pub fn stagnant_lid_determinations(
        &self,
        state: &StagnantLidState<'_>,
    ) -> Result<StagnantLidEnsemble, NoInScopeDetermination> {
        let mut determinations = Vec::new();
        let mut excluded = Vec::new();
        let mut sensitivity = Vec::new();
        for group in self.stagnant_lid_families() {
            match self.evaluate_family(&group, state) {
                FamilyOutcome::InScope(d) => determinations.push(d),
                FamilyOutcome::Excluded {
                    exclusion,
                    sensitivity: s,
                } => {
                    excluded.push(exclusion);
                    if let Some(s) = s {
                        sensitivity.push(s);
                    }
                }
            }
        }
        if determinations.is_empty() {
            return Err(NoInScopeDetermination {
                excluded,
                sensitivity,
            });
        }
        Ok(StagnantLidEnsemble {
            determinations,
            excluded,
        })
    }

    /// The stagnant-lid rows grouped into the families their data declares, in the column's own row order so the
    /// walk is deterministic. A row that declares no family is its own group, and the preflight convicts it for the
    /// missing declaration rather than skipping it: a silently skipped row is a determination nobody was told about.
    fn stagnant_lid_families(&self) -> Vec<FamilyGroup> {
        let mut groups: Vec<FamilyGroup> = Vec::new();
        for (i, row) in self.constants.iter().enumerate() {
            if !row.is_stagnant_lid() {
                continue;
            }
            let key = row.family.clone().unwrap_or_else(|| row.name.clone());
            match groups.iter_mut().find(|g| g.key == key) {
                Some(g) => g.members.push(i),
                None => groups.push(FamilyGroup {
                    key,
                    members: vec![i],
                }),
            }
        }
        groups
    }

    /// One family: its own declaration checked, its members preflighted, and its prescription applied where every
    /// branch survived.
    fn evaluate_family(&self, group: &FamilyGroup, state: &StagnantLidState<'_>) -> FamilyOutcome {
        let mut family_failures = Vec::new();

        // The selection rule is the family's, so every row of it must declare the same one.
        let mut declared: Option<&str> = None;
        let mut missing = false;
        for &i in &group.members {
            match self.constants[i].selection.as_deref() {
                None => missing = true,
                Some(s) => match declared {
                    None => declared = Some(s),
                    Some(d) if d != s => {
                        family_failures.push(ScopeFailure::InconsistentSelectionRule {
                            declared: d.to_string(),
                            sibling: s.to_string(),
                        });
                    }
                    Some(_) => {}
                },
            }
        }
        if missing {
            family_failures.push(ScopeFailure::ScopeNotTranscribed {
                field: "family_selection",
            });
        }
        let rule = match declared {
            Some(s) => match SelectionRule::parse(s) {
                Some(r) => Some(r),
                None => {
                    family_failures.push(ScopeFailure::UnknownSelectionRule {
                        rule: s.to_string(),
                    });
                    None
                }
            },
            None => None,
        };
        if rule == Some(SelectionRule::SoleDetermination) && group.members.len() > 1 {
            family_failures.push(ScopeFailure::OverfullSoleDetermination {
                rows: group.members.len(),
            });
        }

        let preflights: Vec<MemberPreflight> = group
            .members
            .iter()
            .map(|&i| self.preflight_row(i, state))
            .collect();
        let convicted: Vec<RowExclusion> = preflights
            .iter()
            .filter(|p| !p.failures.is_empty())
            .map(|p| RowExclusion {
                row: self.constants[p.row].name.clone(),
                failures: p.failures.clone(),
            })
            .collect();

        // IN SCOPE ONLY WHEN NOTHING WAS CONVICTED. A `larger_branch` family with one branch out of scope cannot
        // apply its own prescription: the maximum over the surviving branches is a different quantity, and one
        // that can only be too small, since the branch that was dropped is the one that might have been larger.
        if family_failures.is_empty() && convicted.is_empty() {
            if let Some(rule) = rule {
                if let Some(d) = self.combine(group, rule, &preflights) {
                    return FamilyOutcome::InScope(d);
                }
                // Defensive: a family with no convicted row has a value for every member, so this is unreachable
                // by construction. If it is ever reached, the family is excluded loudly rather than dropped.
                family_failures.push(ScopeFailure::KernelRefused);
            }
        }

        // A sensitivity number exists only where the family's own prescription is intact, every member was
        // evaluable, and every failure is one that leaves the law computable (the fitted regime, never a missing
        // input). Anything else has no number to report, and substituting one would be the laundering the typed
        // projections exist to stop.
        let all_evaluated = preflights.iter().all(|p| p.value.is_some());
        let regime_only = preflights.iter().all(|p| {
            p.failures
                .iter()
                .all(ScopeFailure::permits_sensitivity_evaluation)
        });
        let sensitivity = match rule {
            Some(rule) if family_failures.is_empty() && all_evaluated && regime_only => {
                self.combine(group, rule, &preflights)
            }
            _ => None,
        };
        FamilyOutcome::Excluded {
            exclusion: FamilyExclusion {
                family: group.key.clone(),
                family_failures,
                convicted,
            },
            sensitivity,
        }
    }

    /// One row against one state: every scope test, all of them run so the report names every axis the row is off
    /// on rather than the first, and the row's own log-domain evaluation where the inputs exist.
    fn preflight_row(&self, index: usize, state: &StagnantLidState<'_>) -> MemberPreflight {
        let row = &self.constants[index];
        let mut failures = Vec::new();
        let mut caveats = Vec::new();

        if row.family.is_none() {
            failures.push(ScopeFailure::ScopeNotTranscribed { field: "family" });
        }

        // THE RAYLEIGH AXIS: the row's own definition or nothing at all.
        let ln_rayleigh = match &row.rayleigh_definition {
            None => {
                failures.push(ScopeFailure::ScopeNotTranscribed {
                    field: "rayleigh_definition_key",
                });
                None
            }
            Some(key) => match one_of(state.rayleigh.iter().filter(|p| p.definition == key)) {
                Found::One(p) => Some(p.ln_rayleigh),
                Found::Several => {
                    failures.push(ScopeFailure::AmbiguousProjection {
                        definition: key.clone(),
                    });
                    None
                }
                Found::None => {
                    failures.push(ScopeFailure::RayleighDefinitionUnavailable {
                        required: key.clone(),
                    });
                    None
                }
            },
        };

        // THE THETA AXIS, the same way: a pressure-carrying theta cannot stand in for a linearized one.
        let theta = match &row.theta_definition {
            None => {
                failures.push(ScopeFailure::ScopeNotTranscribed {
                    field: "theta_definition_key",
                });
                None
            }
            Some(key) => match one_of(state.theta.iter().filter(|p| p.definition == key)) {
                Found::One(p) => Some(p.theta),
                Found::Several => {
                    failures.push(ScopeFailure::AmbiguousProjection {
                        definition: key.clone(),
                    });
                    None
                }
                Found::None => {
                    failures.push(ScopeFailure::ThetaDefinitionUnavailable {
                        required: key.clone(),
                    });
                    None
                }
            },
        };

        // THE RHEOLOGY THE FIT WAS PERFORMED AT, matched exactly.
        match row.fitted_stress_exponent {
            None => failures.push(ScopeFailure::ScopeNotTranscribed {
                field: "fitted_stress_exponent",
            }),
            Some(fitted) if fitted != state.stress_exponent => {
                failures.push(ScopeFailure::StressExponent {
                    fitted,
                    world: state.stress_exponent,
                });
            }
            Some(_) => {}
        }

        // THE SPAN THE FIT WAS MEASURED OVER, as the source's own intervals rather than one hull across them.
        match (&row.fitted_theta_ranges, theta) {
            (None, _) => failures.push(ScopeFailure::ScopeNotTranscribed {
                field: "fitted_theta_ranges",
            }),
            (Some(ranges), _) if ranges.is_empty() => {
                caveats.push(ScopeCaveat::SourceStatesNoFittedSpan {
                    row: row.name.clone(),
                });
            }
            (Some(ranges), Some(theta)) => {
                let mut span_lo = ranges[0].0;
                let mut span_hi = ranges[0].1;
                for (lo, hi) in ranges {
                    span_lo = span_lo.min(*lo);
                    span_hi = span_hi.max(*hi);
                }
                if theta < span_lo || theta > span_hi {
                    failures.push(ScopeFailure::ThetaOutsideFittedSpan {
                        theta,
                        span_lo,
                        span_hi,
                    });
                } else if !ranges.iter().any(|(lo, hi)| theta >= *lo && theta <= *hi) {
                    caveats.push(ScopeCaveat::ThetaBetweenFittedIntervals {
                        row: row.name.clone(),
                        theta,
                    });
                }
            }
            // No theta to test against: the theta axis above has already convicted this row.
            (Some(_), None) => {}
        }

        // THE EVALUATION, through the shared kernel so this reader holds no second copy of the law.
        let value = match (ln_rayleigh, theta) {
            (Some(ln_ra), Some(th)) => {
                let point = ln_nusselt_at(row, ln_ra, th, row.constant.value);
                let lo = row
                    .constant
                    .band_lo
                    .map(|c| ln_nusselt_at(row, ln_ra, th, c));
                let hi = row
                    .constant
                    .band_hi
                    .map(|c| ln_nusselt_at(row, ln_ra, th, c));
                let edges_formed = !matches!(lo, Some(None)) && !matches!(hi, Some(None));
                match (point, edges_formed) {
                    (Some(p), true) => {
                        let lo = lo.flatten();
                        let hi = hi.flatten();
                        let band = match (lo, hi) {
                            (None, None) => None,
                            _ => {
                                let l = lo.unwrap_or(p);
                                let h = hi.unwrap_or(p);
                                Some((l.min(h), l.max(h)))
                            }
                        };
                        Some(MemberValue {
                            ln_nusselt: p,
                            band,
                        })
                    }
                    _ => {
                        failures.push(ScopeFailure::KernelRefused);
                        None
                    }
                }
            }
            _ => None,
        };

        MemberPreflight {
            row: index,
            failures,
            caveats,
            value,
        }
    }

    /// The family's own prescription applied to its evaluated branches.
    ///
    /// `larger_branch` takes the greater `ln Nu` at this state, ties going to the earlier row so the walk is
    /// deterministic, and the band edges are the same maximum taken at each edge (the max is monotone in every
    /// branch, so the envelope of the maximum is the maximum of the envelopes). `sole_determination` has one row,
    /// for which the same expression is that row.
    fn combine(
        &self,
        group: &FamilyGroup,
        rule: SelectionRule,
        preflights: &[MemberPreflight],
    ) -> Option<FamilyDetermination> {
        let mut selected: Option<usize> = None;
        let mut ln_nusselt = Fixed::ZERO;
        let mut band_lo: Option<Fixed> = None;
        let mut band_hi: Option<Fixed> = None;
        let mut any_band = false;
        let mut caveats = Vec::new();
        for (n, p) in preflights.iter().enumerate() {
            let v = p.value.as_ref()?;
            if selected.is_none() || v.ln_nusselt > ln_nusselt {
                selected = Some(n);
                ln_nusselt = v.ln_nusselt;
            }
            let (lo, hi) = match v.band {
                Some((lo, hi)) => {
                    any_band = true;
                    (lo, hi)
                }
                None => (v.ln_nusselt, v.ln_nusselt),
            };
            band_lo = Some(band_lo.map_or(lo, |b: Fixed| b.max(lo)));
            band_hi = Some(band_hi.map_or(hi, |b: Fixed| b.max(hi)));
            caveats.extend(p.caveats.iter().cloned());
        }
        let selected = selected?;
        debug_assert!(
            rule != SelectionRule::SoleDetermination || preflights.len() == 1,
            "a sole-determination family reaching the combinator carries exactly one row; the arity is checked \
             before this point and an overfull family is excluded rather than silently maximised"
        );
        let row = &self.constants[preflights[selected].row];
        Some(FamilyDetermination {
            family: group.key.clone(),
            selected_row: row.name.clone(),
            witness: row.witness.clone(),
            ln_nusselt,
            ln_nusselt_band: if any_band {
                Some((band_lo?, band_hi?))
            } else {
                None
            },
            caveats,
        })
    }
}

/// One family as the column groups it: the rows one source prescription evaluates together, in file order.
struct FamilyGroup {
    key: String,
    members: Vec<usize>,
}

/// One row's evaluated log-domain answer, with the band its own coefficient spread maps to.
struct MemberValue {
    ln_nusselt: Fixed,
    band: Option<(Fixed, Fixed)>,
}

/// One row's preflight: every test it failed, everything the preflight wants seen, and its value where the inputs
/// to form one existed.
struct MemberPreflight {
    row: usize,
    failures: Vec<ScopeFailure>,
    caveats: Vec<ScopeCaveat>,
    value: Option<MemberValue>,
}

/// A family is either in scope with one determination or excluded, and an excluded one may still carry a number
/// for a sensitivity report.
enum FamilyOutcome {
    InScope(FamilyDetermination),
    Excluded {
        exclusion: FamilyExclusion,
        sensitivity: Option<FamilyDetermination>,
    },
}

/// What a search for one tagged projection found.
enum Found<T> {
    None,
    One(T),
    Several,
}

/// Exactly one, or the reason there is not exactly one.
///
/// A plain `find` would take the first of several, which is a silent selection between two numbers a caller holds
/// for one quantity. One state has one value per definition, so anything else is a refusal.
fn one_of<T, I: Iterator<Item = T>>(mut it: I) -> Found<T> {
    match (it.next(), it.next()) {
        (None, _) => Found::None,
        (Some(first), None) => Found::One(first),
        (Some(_), Some(_)) => Found::Several,
    }
}

/// One row at one coefficient, in the log domain, through the shared kernel.
///
/// The law lives in [`crate::laws::ln_stagnant_lid_nusselt`] and is called rather than restated here, so the
/// `Nu >= 1` conductive floor and the checked arithmetic are the same ones every other consumer gets.
fn ln_nusselt_at(row: &Row, ln_rayleigh: Fixed, theta: Fixed, coefficient: Fixed) -> Option<Fixed> {
    crate::laws::ln_stagnant_lid_nusselt(
        ln_rayleigh,
        theta,
        coefficient,
        row.theta_exponent?,
        row.rayleigh_exponent?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixture_eigenvalue_literal_is_pinned_to_the_cited_row() {
        // ONE UNCOMPARED INSTANCE (owner ruling 2026-07-18): the geodynamics test fixtures spell the rigid-rigid
        // critical Rayleigh as Fixed::from_ratio(1_707_762, 1000). It is pinned here to the cited convection_scaling
        // row so the fixture literal is a COMPARED copy, never a second uncompared instance of the eigenvalue; the
        // sibling sentinel in deeptime pins the RIGID_RIGID_RA_CRIT const the same way, leaving the cited row alone.
        let cited = ConvectionScaling::standard()
            .expect("convection_scaling.toml is vendored")
            .critical_rayleigh(BoundaryCondition::RigidRigid)
            .expect("the rigid-rigid row is present");
        let fixture = Fixed::from_ratio(1_707_762, 1000);
        assert!(
            (fixture - cited).abs() < Fixed::from_ratio(1, 100),
            "the fixture eigenvalue {} must equal the cited rigid-rigid row {}",
            fixture.to_f64_lossy(),
            cited.to_f64_lossy()
        );
    }

    fn scaling() -> ConvectionScaling {
        ConvectionScaling::standard().expect("the vendored column loads")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() <= tol
    }

    #[test]
    fn the_prefactor_is_the_single_lid_value_with_the_two_convention_band() {
        let a = scaling().nusselt_prefactor().expect("a is present");
        assert_eq!(
            a.value,
            Fixed::ONE,
            "the single-lid planetary prefactor a = 1.0"
        );
        // The band spans the symmetric two-boundary-layer 2^(-4/3) ~ 0.397 up to 1.0.
        assert!(close(a.band_lo.expect("band_lo"), 0.397, 0.002));
        assert!(close(a.band_hi.expect("band_hi"), 1.0, 1e-9));
    }

    #[test]
    fn the_prefactor_derives_from_the_internal_heating_fraction() {
        let s = scaling();
        // The deep-time model is heat_production-only (internal fraction 1) -> the internal endpoint 2^(-4/3),
        // NOT the literature's basal default of 1.0 (the residue rule: condition on the world's own state).
        let a_internal = s
            .nusselt_prefactor_at_internal_fraction(Fixed::ONE)
            .unwrap();
        assert!(
            close(a_internal, 0.397, 0.002),
            "fully internal heating derives a = 2^(-4/3)"
        );
        // Purely basal heating (fraction 0) recovers the single-boundary-layer endpoint 1.0.
        assert_eq!(
            s.nusselt_prefactor_at_internal_fraction(Fixed::ZERO)
                .unwrap(),
            Fixed::ONE
        );
        // A mixed budget interpolates strictly between the endpoints.
        let a_mid = s
            .nusselt_prefactor_at_internal_fraction(Fixed::from_ratio(1, 2))
            .unwrap();
        assert!(
            a_mid > a_internal && a_mid < Fixed::ONE,
            "mixed heating interpolates the band"
        );
    }

    #[test]
    fn the_critical_rayleigh_numbers_are_bc_conditioned() {
        let s = scaling();
        // Free-free is the analytic 27 pi^4 / 4 = 657.511.
        assert!(close(
            s.critical_rayleigh(BoundaryCondition::FreeFree).unwrap(),
            657.511,
            0.01
        ));
        // Rigid-rigid, the Pellew-Southwell eigenvalue.
        assert!(close(
            s.critical_rayleigh(BoundaryCondition::RigidRigid).unwrap(),
            1707.762,
            0.01
        ));
        // Rigid-free, the planetary-mantle case (free surface over a near-rigid base).
        assert!(close(
            s.critical_rayleigh(BoundaryCondition::RigidFree).unwrap(),
            1100.65,
            0.01
        ));
    }

    #[test]
    fn the_bare_coefficient_is_a_separate_row_never_the_prefactor() {
        // C = 0.294 belongs to the UN-normalized Nu = C Ra^(1/3); it must not be readable as the prefactor a.
        let c = scaling()
            .constant("nu_ra_bare_coefficient_C")
            .expect("C row present");
        assert!(close(c.value, 0.294, 0.001));
        assert_ne!(c.value, scaling().nusselt_prefactor().unwrap().value);
    }

    #[test]
    fn the_four_stagnant_lid_conventions_load_with_their_cited_triples() {
        let s = scaling();
        let c1 = s
            .stagnant_lid_convention("nu_stag_time_dependent_C1")
            .expect("the time-dependent row is present");
        // Batra & Foley 2021 eq. (10): Nu = 0.48 theta^(-4/3) Ra_i^(1/3).
        assert!(close(c1.coefficient, 0.48, 1e-9));
        assert!(close(c1.theta_exponent, -4.0 / 3.0, 1e-9));
        assert!(close(c1.rayleigh_exponent, 1.0 / 3.0, 1e-9));
        // The band runs from their own bottom-heated fit to the internally heated value they report.
        assert!(close(c1.band_lo.expect("band_lo"), 0.48, 1e-9));
        assert!(close(c1.band_hi.expect("band_hi"), 0.55, 1e-9));

        // Batra & Foley 2021 eq. (9): Nu = 2.95 theta^(-6/5) Ra_i^(1/5).
        let c2 = s
            .stagnant_lid_convention("nu_stag_steady_C2")
            .expect("the steady row is present");
        assert!(close(c2.coefficient, 2.95, 1e-9));
        assert!(close(c2.theta_exponent, -1.2, 1e-9));
        assert!(close(c2.rayleigh_exponent, 0.2, 1e-9));

        // Schulz et al. 2020 eq. (34) and eq. (35), the Arrhenius-with-pressure fits.
        let internal = s
            .stagnant_lid_convention("nu_stag_arrhenius_internal_ra")
            .expect("the Arrhenius internal-Ra row is present");
        assert!(close(internal.coefficient, 0.278, 1e-9));
        assert!(close(internal.theta_exponent, -0.4, 1e-9));
        assert!(close(internal.rayleigh_exponent, 0.203, 1e-9));
        let harmonic = s
            .stagnant_lid_convention("nu_stag_arrhenius_harmonic_ra")
            .expect("the Arrhenius harmonic-Ra row is present");
        assert!(close(harmonic.coefficient, 0.219, 1e-9));
        assert!(close(harmonic.theta_exponent, -0.581, 1e-9));
        assert!(close(harmonic.rayleigh_exponent, 0.262, 1e-9));
    }

    #[test]
    fn the_linearized_rows_satisfy_the_sources_own_one_parameter_family() {
        // Batra & Foley eq. (8) states the family as Nu = C* theta^-(1+beta) Ra_i^beta, so gamma and beta are
        // NOT independent. Checking gamma = -(1 + beta) on both rows is a transcription check that a slipped
        // digit in either exponent fails, and it is the source's own relation rather than an imposed one. It
        // is asserted only on the two rows that come from that family: the Arrhenius fits are free fits and
        // do not obey it (0.203 would demand -1.203, not -0.4), which is itself the point of banking them apart.
        let s = scaling();
        for name in ["nu_stag_time_dependent_C1", "nu_stag_steady_C2"] {
            let c = s.stagnant_lid_convention(name).expect("row present");
            let expected = Fixed::ZERO - (Fixed::ONE + c.rayleigh_exponent);
            assert!(
                (c.theta_exponent - expected).abs() < Fixed::from_ratio(1, 1_000_000),
                "{name}: gamma {} must equal -(1 + beta) = {}",
                c.theta_exponent.to_f64_lossy(),
                expected.to_f64_lossy()
            );
        }
    }

    #[test]
    fn a_mobile_lid_row_can_never_be_read_as_a_stagnant_lid_convention() {
        // The prefactor `a` row carries a `beta` field, but under a key the stagnant-lid reader does not read,
        // and it carries no theta exponent at all. If it could be read here it would arrive as a suppression
        // law with no suppression, which is the mobile-lid law wearing the stagnant-lid name.
        let s = scaling();
        assert!(s.stagnant_lid_convention("nu_ra_prefactor_a").is_none());
        assert!(s
            .stagnant_lid_convention("nu_ra_bare_coefficient_C")
            .is_none());
        assert!(s.stagnant_lid_convention("ra_crit_rigid_free").is_none());
        assert!(s.stagnant_lid_convention("no_such_row").is_none());
        // And the reverse direction still works: a stagnant row is still readable as a plain constant, but its
        // value is the coefficient of a DIFFERENT form, which is why it is named apart from `a`.
        let plain = s
            .constant("nu_stag_time_dependent_C1")
            .expect("readable as a constant too");
        assert_ne!(plain.value, s.nusselt_prefactor().unwrap().value);
    }

    #[test]
    fn the_two_linearized_branches_cross_inside_the_sources_own_rayleigh_range() {
        // Batra & Foley eq. (11) takes the LARGER of the two branches and says the steady-to-time-dependent
        // transition falls where they cross. Setting the two equal gives Ra_cross = (C2/C1)^(15/2) theta, so
        // the crossing is COMPUTED from the four banked numbers rather than asserted. Their models were run at
        // reference Rayleigh numbers 1e6 to 1e8 with theta of 13.82 and 16.12, so a correct transcription of
        // all four must put the crossing inside that box. A slipped digit in any one of them moves it out.
        //
        // The arithmetic is f64 DELIBERATELY, and this is a test rather than a kernel. Re-deriving the crossing
        // with the same fixed-point path the reader uses would check the reader against itself; an independent
        // evaluation checks the COLUMN. The canonical integer-only path is the kernel in laws.rs, which the
        // steering gate scans and which carries no float at all.
        let s = scaling();
        let c1 = s
            .stagnant_lid_convention("nu_stag_time_dependent_C1")
            .unwrap();
        let c2 = s.stagnant_lid_convention("nu_stag_steady_C2").unwrap();
        let ratio = c2.coefficient.to_f64_lossy() / c1.coefficient.to_f64_lossy();
        let exponent_gap =
            c1.rayleigh_exponent.to_f64_lossy() - c2.rayleigh_exponent.to_f64_lossy();
        for theta in [13.82_f64, 16.12_f64] {
            let ra_cross = ratio.powf(1.0 / exponent_gap) * theta;
            assert!(
                (1e6..1e8).contains(&ra_cross),
                "the branches cross at Ra = {ra_cross:.3e} for theta = {theta}, outside the source's own 1e6 to 1e8 range"
            );
        }
    }

    #[test]
    fn an_uncited_column_refuses_to_load() {
        let no_src = "[[constant]]\nname = \"x\"\nvalue = \"1.0\"\n";
        assert_eq!(
            ConvectionScaling::from_toml_str(no_src).unwrap_err(),
            ScalingError::MissingProvenance
        );
    }

    // ---------------------------------------------------------------------------------------------------------
    // THE STATE-DISPATCHED EVALUATION: the max(C1, C2) combinator, the typed projections, and the scope preflight.
    //
    // Every test below either proves a scope test CONVICTS or proves the combinator applies the source's own
    // prescription. The state values are test states rather than world values: the stress exponents are the ones
    // the sources and the engine's own creep bank declare, the thetas are read from the rows' own fitted spans,
    // and the Rayleigh numbers are representative magnitudes chosen to sit either side of a crossing the four
    // banked numbers compute for themselves.
    // ---------------------------------------------------------------------------------------------------------

    /// The keys the standard column declares, spelled once here so a test reads the same symbol a consumer would.
    const RA_BATRA: &str = "ra_i_sublid_viscosity_at_t_i";
    const RA_SCHULZ_INTERNAL: &str = "ra_i_min_viscosity_midcell";
    const RA_SCHULZ_HARMONIC: &str = "ra_har_sublid_harmonic_mean";
    const THETA_LINEARIZED: &str = "theta_fk_linearized_pressure_free";
    const THETA_ARRHENIUS: &str = "theta_arrhenius_pressure_carrying";

    /// The engine's own admitted dislocation-creep stress exponent (Hirth and Kohlstedt 2003 dry olivine).
    fn dislocation_n() -> Fixed {
        Fixed::from_ratio(7, 2)
    }

    fn failures_of<'a>(
        excluded: &'a [FamilyExclusion],
        row: &str,
    ) -> Option<&'a Vec<ScopeFailure>> {
        excluded
            .iter()
            .flat_map(|f| f.convicted.iter())
            .find(|r| r.row == row)
            .map(|r| &r.failures)
    }

    #[test]
    fn the_two_batra_branches_are_one_family_and_the_larger_one_stands() {
        // THE SOURCE'S OWN PRESCRIPTION IN PRODUCTION (Batra and Foley 2021 eq. 11): evaluate both branches, take
        // the larger, and the transition IS the crossing. Nothing declares which pattern applies. So on a
        // Newtonian, pressure-free world the same column returns the STEADY branch below the crossing and the
        // TIME-DEPENDENT branch above it, and the crossing is computed from the four banked numbers rather than
        // asserted (the arithmetic in f64 deliberately, as the sibling transcription test explains).
        let s = scaling();
        let theta = Fixed::from_int(15); // inside the sampled span 13.82 to 16.12
        let c1 = s
            .stagnant_lid_convention("nu_stag_time_dependent_C1")
            .unwrap();
        let c2 = s.stagnant_lid_convention("nu_stag_steady_C2").unwrap();
        let ln_ra_cross = ((c2.coefficient.to_f64_lossy() / c1.coefficient.to_f64_lossy()).powf(
            1.0 / (c1.rayleigh_exponent.to_f64_lossy() - c2.rayleigh_exponent.to_f64_lossy()),
        ) * theta.to_f64_lossy())
        .ln();
        assert!(
            (15.0..18.0).contains(&ln_ra_cross),
            "the test's two states must straddle the crossing at ln Ra = {ln_ra_cross}"
        );

        for (ln_ra, expected) in [(15, "nu_stag_steady_C2"), (18, "nu_stag_time_dependent_C1")] {
            let theta_projection = s.theta_projection(THETA_LINEARIZED, theta).unwrap();
            let ra_projection = s
                .rayleigh_projection(RA_BATRA, Fixed::from_int(ln_ra))
                .unwrap();
            let state = StagnantLidState {
                stress_exponent: Fixed::ONE,
                rayleigh: &[ra_projection],
                theta: &[theta_projection],
            };
            let ensemble = s
                .stagnant_lid_determinations(&state)
                .expect("a Newtonian pressure-free world is inside the linearized rows' scope");
            let batra = ensemble
                .determinations
                .iter()
                .find(|d| d.family == "batra_foley_2021_linearized")
                .expect("the Batra family is in scope");
            assert_eq!(
                batra.selected_row, expected,
                "at ln Ra = {ln_ra}, either side of the crossing at {ln_ra_cross}, the larger branch is {expected}"
            );
            // The two branches were both evaluated: the answer is the larger of them, so it is at least the value
            // the selected branch alone would give and never smaller than the branch that lost.
            let losing = ln_stagnant_lid_nusselt_for(&s, expected, ln_ra, theta);
            assert!(
                (batra.ln_nusselt - losing).abs() < Fixed::from_ratio(1, 1000),
                "the family's answer is the selected branch's own value"
            );
        }
    }

    /// One row evaluated alone, for a test that wants to compare the family's answer against a single branch.
    fn ln_stagnant_lid_nusselt_for(
        s: &ConvectionScaling,
        row: &str,
        ln_ra: i32,
        theta: Fixed,
    ) -> Fixed {
        let c = s.stagnant_lid_convention(row).expect("row present");
        crate::laws::ln_stagnant_lid_nusselt(
            Fixed::from_int(ln_ra),
            theta,
            c.coefficient,
            c.theta_exponent,
            c.rayleigh_exponent,
        )
        .expect("the row evaluates at this state")
    }

    #[test]
    fn the_production_world_gets_a_typed_refusal_rather_than_the_nearest_number() {
        // THE PRODUCTION STATE AS THE ENGINE STANDS: a pressure-carrying Frank-Kamenetskii theta (the eq. 29 form
        // laws::stagnant_lid_rheological_theta computes), dislocation creep at n = 3.5, and NO Rayleigh number
        // carrying a declared definition, because the column's ln_viscosity is a bare scalar and no row's average
        // has been formed. Every family is convicted, and nothing is evaluable, so there is no sensitivity number
        // either. This is the honest answer and it is a typed stop.
        let s = scaling();
        let theta = s
            .theta_projection(THETA_ARRHENIUS, Fixed::from_ratio(1273, 100))
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: dislocation_n(),
            rayleigh: &[],
            theta: &[theta],
        };
        let refusal = s.stagnant_lid_determinations(&state).expect_err(
            "no banked row is in scope for a non-Newtonian world with no tagged Rayleigh number",
        );
        assert_eq!(refusal.excluded.len(), 3, "three families, all convicted");
        assert!(
            refusal.sensitivity.is_empty(),
            "nothing is evaluable, so there is no envelope to report either"
        );
        // Each Schulz row is convicted on BOTH the missing projection and the rheology, and the report carries
        // both rather than the first.
        let internal = failures_of(&refusal.excluded, "nu_stag_arrhenius_internal_ra").unwrap();
        assert!(
            internal.contains(&ScopeFailure::RayleighDefinitionUnavailable {
                required: RA_SCHULZ_INTERNAL.to_string()
            })
        );
        assert!(internal.contains(&ScopeFailure::StressExponent {
            fitted: Fixed::ONE,
            world: dislocation_n()
        }));
        // The linearized rows are convicted on the theta axis too: their fits carry no pressure term at all, so a
        // pressure-carrying theta is not a theta they were fitted against.
        let c1 = failures_of(&refusal.excluded, "nu_stag_time_dependent_C1").unwrap();
        assert!(c1.contains(&ScopeFailure::ThetaDefinitionUnavailable {
            required: THETA_LINEARIZED.to_string()
        }));
    }

    #[test]
    fn the_rheology_alone_still_refuses_once_the_wiring_supplies_a_tagged_rayleigh_number() {
        // THE NEXT STATE THE WIRING LANE REACHES: the engine forms the minimum viscosity at the interior
        // temperature and pressure in the middle of the cell, tags its Rayleigh number as the Schulz internal
        // definition, and carries the pressure-carrying theta. The projections now match, and the answer is STILL
        // a refusal, because the fit was performed on Newtonian runs and the world runs dislocation creep at 3.5.
        // The number exists at that point, so it rides as a sensitivity evaluation INSIDE the refusal.
        let s = scaling();
        let theta_value = Fixed::from_ratio(1273, 100);
        let theta = s.theta_projection(THETA_ARRHENIUS, theta_value).unwrap();
        // A representative Mars-class interior vigour, the magnitude the source's own runs sit at.
        let ra = s
            .rayleigh_projection(RA_SCHULZ_INTERNAL, Fixed::from_int(5_000_000).ln())
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: dislocation_n(),
            rayleigh: &[ra],
            theta: &[theta],
        };
        let refusal = s
            .stagnant_lid_determinations(&state)
            .expect_err("a Newtonian-fitted relation is out of scope at n = 3.5");
        let internal = failures_of(&refusal.excluded, "nu_stag_arrhenius_internal_ra").unwrap();
        assert_eq!(
            internal,
            &vec![ScopeFailure::StressExponent {
                fitted: Fixed::ONE,
                world: dislocation_n()
            }],
            "the rheology is the ONLY thing left wrong with this row, and it is enough"
        );
        // The envelope is reachable only through the refusal, and it names the row it came from.
        assert_eq!(refusal.sensitivity.len(), 1);
        assert_eq!(
            refusal.sensitivity[0].selected_row,
            "nu_stag_arrhenius_internal_ra"
        );
        assert!(refusal.sensitivity[0].ln_nusselt > Fixed::ZERO);
        // The harmonic row is not rescued by its sibling's projection: a different average is a different quantity.
        let harmonic = failures_of(&refusal.excluded, "nu_stag_arrhenius_harmonic_ra").unwrap();
        assert!(
            harmonic.contains(&ScopeFailure::RayleighDefinitionUnavailable {
                required: RA_SCHULZ_HARMONIC.to_string()
            })
        );
    }

    #[test]
    fn the_stress_exponent_test_convicts_on_that_axis_alone() {
        // THE PIVOT. One state, one axis moved: at the fitted Newtonian exponent the Schulz internal row is in
        // scope, and at the engine's own dislocation exponent the same state is refused. Nothing else changes, so
        // the conviction is attributable to the rheology and to nothing else.
        let s = scaling();
        let theta = s
            .theta_projection(THETA_ARRHENIUS, Fixed::from_int(10))
            .unwrap();
        let ra = s
            .rayleigh_projection(RA_SCHULZ_INTERNAL, Fixed::from_int(5_000_000).ln())
            .unwrap();
        let newtonian = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let ensemble = s
            .stagnant_lid_determinations(&newtonian)
            .expect("the Newtonian world is inside the fit's own rheology");
        assert!(ensemble
            .determinations
            .iter()
            .any(|d| d.selected_row == "nu_stag_arrhenius_internal_ra"));

        let dislocation = StagnantLidState {
            stress_exponent: dislocation_n(),
            ..newtonian
        };
        let refusal = s
            .stagnant_lid_determinations(&dislocation)
            .expect_err("moving only the stress exponent must refuse");
        assert_eq!(
            failures_of(&refusal.excluded, "nu_stag_arrhenius_internal_ra").unwrap(),
            &vec![ScopeFailure::StressExponent {
                fitted: Fixed::ONE,
                world: dislocation_n()
            }]
        );
    }

    #[test]
    fn a_row_cannot_be_selected_against_another_rows_rayleigh_number() {
        // THE NORMALIZATION ERROR, BLOCKED BY TYPE. Supplying only the harmonic-mean Rayleigh number leaves the
        // internal-Ra row convicted rather than evaluated against an average it was not fitted against, and the
        // reverse holds too. Both are the bare-coefficient error one level deeper: two dimensionless numbers, both
        // called Ra, formed on different viscosities.
        let s = scaling();
        let theta = s
            .theta_projection(THETA_ARRHENIUS, Fixed::from_int(10))
            .unwrap();
        let harmonic_only = s
            .rayleigh_projection(RA_SCHULZ_HARMONIC, Fixed::from_int(2_890_000).ln())
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[harmonic_only],
            theta: &[theta],
        };
        let ensemble = s.stagnant_lid_determinations(&state).unwrap();
        assert!(
            ensemble
                .determinations
                .iter()
                .all(|d| d.selected_row != "nu_stag_arrhenius_internal_ra"),
            "the internal-Ra row must not be evaluated against a harmonic mean"
        );
        assert!(
            failures_of(&ensemble.excluded, "nu_stag_arrhenius_internal_ra")
                .unwrap()
                .contains(&ScopeFailure::RayleighDefinitionUnavailable {
                    required: RA_SCHULZ_INTERNAL.to_string()
                })
        );
    }

    #[test]
    fn a_projection_cannot_carry_a_definition_no_row_declares() {
        // The construction is the guard: a caller cannot invent a definition or mistype one into silence, because
        // the only way to build a projection is to name a key the column itself carries.
        let s = scaling();
        assert!(s
            .rayleigh_projection("ra_whatever_i_happen_to_have", Fixed::from_int(14))
            .is_none());
        assert!(s
            .theta_projection("theta_close_enough", Fixed::from_int(12))
            .is_none());
        // And a real key does construct, carrying the column's own spelling back.
        assert_eq!(
            s.rayleigh_projection(RA_BATRA, Fixed::from_int(14))
                .unwrap()
                .definition(),
            RA_BATRA
        );
    }

    #[test]
    fn two_values_under_one_definition_refuse_rather_than_the_first_one_winning() {
        // ONE STATE HAS ONE VALUE PER QUANTITY. A caller holding two Rayleigh numbers under one definition has two
        // numbers it believes are both the state; taking whichever came first would be a silent selection between
        // them, which is the same defect as selecting a row against the wrong average, one level up.
        let s = scaling();
        let theta = s
            .theta_projection(THETA_LINEARIZED, Fixed::from_int(15))
            .unwrap();
        let a = s
            .rayleigh_projection(RA_BATRA, Fixed::from_int(16))
            .unwrap();
        let b = s
            .rayleigh_projection(RA_BATRA, Fixed::from_int(18))
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[a, b],
            theta: &[theta],
        };
        let refusal = s
            .stagnant_lid_determinations(&state)
            .expect_err("two Rayleigh numbers under one definition is a refusal");
        assert!(failures_of(&refusal.excluded, "nu_stag_time_dependent_C1")
            .unwrap()
            .contains(&ScopeFailure::AmbiguousProjection {
                definition: RA_BATRA.to_string()
            }));
        assert!(
            refusal.sensitivity.is_empty(),
            "an ambiguous state forms no number, so it reports no envelope"
        );
    }

    #[test]
    fn a_theta_past_every_sampled_condition_convicts_as_an_extrapolation() {
        // Batra sampled theta at two values only, 13.82 and 16.12. A world at theta = 5 is past every condition
        // their models were run at, so the row is an extrapolation there and the preflight says so.
        let s = scaling();
        let theta = s
            .theta_projection(THETA_LINEARIZED, Fixed::from_int(5))
            .unwrap();
        let ra = s
            .rayleigh_projection(RA_BATRA, Fixed::from_int(16))
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let refusal = s
            .stagnant_lid_determinations(&state)
            .expect_err("theta = 5 is outside every banked fitted span, so no family is in scope");
        let c1 = failures_of(&refusal.excluded, "nu_stag_time_dependent_C1").unwrap();
        assert!(matches!(
            c1.as_slice(),
            [ScopeFailure::ThetaOutsideFittedSpan { .. }]
        ));
        // It is evaluable, so it rides the sensitivity envelope: the number exists and is an extrapolation.
        assert!(refusal
            .sensitivity
            .iter()
            .any(|d| d.family == "batra_foley_2021_linearized"));
    }

    #[test]
    fn a_theta_between_two_fitting_sets_rides_as_a_caveat_rather_than_passing_unseen() {
        // Schulz fitted eqs. (34) and (35) over two SEPARATE sets, theta 6.89 to 11.07 and 24.2 to 27.6, and their
        // own dislocation runs at 11.4 to 12.8 sit between them. A hull would call 12.73 supported. The row keeps
        // its intervals, so the determination carries the caveat instead of hiding it.
        let s = scaling();
        let theta_value = Fixed::from_ratio(1273, 100);
        let theta = s.theta_projection(THETA_ARRHENIUS, theta_value).unwrap();
        let ra = s
            .rayleigh_projection(RA_SCHULZ_INTERNAL, Fixed::from_int(5_000_000).ln())
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let ensemble = s.stagnant_lid_determinations(&state).unwrap();
        let internal = ensemble
            .determinations
            .iter()
            .find(|d| d.family == "schulz_2020_internal_ra")
            .unwrap();
        assert!(internal
            .caveats
            .contains(&ScopeCaveat::ThetaBetweenFittedIntervals {
                row: "nu_stag_arrhenius_internal_ra".to_string(),
                theta: theta_value
            }));
        // Inside one of the intervals the caveat does not fire, so it is reporting a real position rather than
        // decorating every answer.
        let inside = s
            .theta_projection(THETA_ARRHENIUS, Fixed::from_int(10))
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[inside],
        };
        let ensemble = s.stagnant_lid_determinations(&state).unwrap();
        let internal = ensemble
            .determinations
            .iter()
            .find(|d| d.family == "schulz_2020_internal_ra")
            .unwrap();
        assert!(internal.caveats.is_empty());
    }

    #[test]
    fn the_band_is_carried_in_ln_nu_and_the_members_stay_discrete() {
        // THE BAND IS OVER THE HEAT TRANSPORT, NEVER OVER alpha. C1 carries a coefficient spread of 0.48 to 0.55,
        // and above the crossing, where C1 is the branch that stands, that spread appears as an OFFSET in ln Nu of
        // exactly ln(0.55/0.48), because the exponents and the definitions are held fixed while the coefficient
        // moves. The members stay discrete beside it: the hull is an envelope over separate determinations and is
        // named as one.
        let s = scaling();
        let theta = s
            .theta_projection(THETA_LINEARIZED, Fixed::from_int(15))
            .unwrap();
        let ra = s
            .rayleigh_projection(RA_BATRA, Fixed::from_int(18))
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let ensemble = s.stagnant_lid_determinations(&state).unwrap();
        let batra = ensemble
            .determinations
            .iter()
            .find(|d| d.family == "batra_foley_2021_linearized")
            .unwrap();
        assert_eq!(batra.selected_row, "nu_stag_time_dependent_C1");
        let (lo, hi) = batra.ln_nusselt_band.expect("C1 carries a reported spread");
        let expected = (0.55_f64 / 0.48).ln();
        assert!(
            ((hi - lo).to_f64_lossy() - expected).abs() < 1e-3,
            "the band width in ln Nu is ln(0.55/0.48) = {expected}, got {}",
            (hi - lo).to_f64_lossy()
        );
        assert_eq!(
            lo, batra.ln_nusselt,
            "the point value is the band's low edge"
        );
        // The determinations are RETAINED. The hull spans them and is not one of them.
        let (hull_lo, hull_hi) = ensemble.ln_nusselt_hull().unwrap();
        assert!(hull_lo <= batra.ln_nusselt && hull_hi >= hi);
        assert!(!ensemble.determinations.is_empty());
    }

    #[test]
    fn the_selected_rows_provenance_is_reachable_from_the_determination() {
        // The determination names the row its family's prescription selected, and that name reaches both halves of
        // the provenance: the held receipt on the determination itself, and the full citation at the column.
        let s = scaling();
        let theta = s
            .theta_projection(THETA_LINEARIZED, Fixed::from_int(15))
            .unwrap();
        let ra = s
            .rayleigh_projection(RA_BATRA, Fixed::from_int(18))
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let ensemble = s.stagnant_lid_determinations(&state).unwrap();
        let batra = &ensemble.determinations[0];
        assert_eq!(batra.witness.as_deref(), Some("batra_foley_2021"));
        let citation = s
            .citation(&batra.selected_row)
            .expect("the selected row carries its citation");
        assert!(
            citation.contains("ggab366"),
            "the selected branch's citation names its primary: {citation}"
        );
    }

    #[test]
    fn the_witness_is_reported_so_two_rows_of_one_study_are_not_read_as_two_agreements() {
        let s = scaling();
        let theta = s
            .theta_projection(THETA_ARRHENIUS, Fixed::from_int(10))
            .unwrap();
        let internal = s
            .rayleigh_projection(RA_SCHULZ_INTERNAL, Fixed::from_int(5_000_000).ln())
            .unwrap();
        let harmonic = s
            .rayleigh_projection(RA_SCHULZ_HARMONIC, Fixed::from_int(2_890_000).ln())
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[internal, harmonic],
            theta: &[theta],
        };
        let ensemble = s.stagnant_lid_determinations(&state).unwrap();
        let witnesses: Vec<_> = ensemble
            .determinations
            .iter()
            .filter_map(|d| d.witness.clone())
            .collect();
        assert_eq!(
            witnesses,
            vec![
                "schulz_tosi_plesa_breuer_2020".to_string(),
                "schulz_tosi_plesa_breuer_2020".to_string()
            ],
            "two determinations, ONE held study: an envelope over them is one voice"
        );
    }

    // --- The synthetic-column tests: a defect in the data must convict rather than pass through. -------------

    fn synthetic_row(name: &str, family: &str, selection: &str, extra: &str) -> String {
        format!(
            "[[constant]]\nname = \"{name}\"\nvalue = \"0.48\"\n\
             theta_exponent = \"-1.333333333333\"\nrayleigh_exponent = \"0.333333333333\"\n\
             family = \"{family}\"\nfamily_selection = \"{selection}\"\n\
             theta_definition_key = \"{THETA_LINEARIZED}\"\n\
             fitted_stress_exponent = \"1.0\"\nfitted_theta_ranges = [[\"1.0\", \"100.0\"]]\n\
             citation = \"synthetic row, this test only\"\n{extra}\n"
        )
    }

    fn synthetic_state<'a>(
        column: &'a ConvectionScaling,
        ra_key: &str,
    ) -> (RayleighProjection<'a>, ThetaProjection<'a>) {
        (
            column
                .rayleigh_projection(ra_key, Fixed::from_int(16))
                .expect("the synthetic column declares this key"),
            column
                .theta_projection(THETA_LINEARIZED, Fixed::from_int(15))
                .expect("the synthetic column declares this key"),
        )
    }

    #[test]
    fn an_incomplete_larger_branch_family_refuses_rather_than_taking_the_surviving_branch() {
        // A `larger_branch` family is ALL OR NOTHING. If one branch cannot be evaluated, the maximum over what is
        // left is a different quantity, and one that can only be too small: the dropped branch is the one that
        // might have been the larger. The two branches here declare different Rayleigh definitions and the caller
        // can form only one.
        let column = ConvectionScaling::from_toml_str(&format!(
            "{}{}",
            synthetic_row(
                "row_a",
                "pair",
                "larger_branch",
                "rayleigh_definition_key = \"ra_have_this\""
            ),
            synthetic_row(
                "row_b",
                "pair",
                "larger_branch",
                "rayleigh_definition_key = \"ra_do_not_have_this\""
            ),
        ))
        .expect("the synthetic column loads");
        let (ra, theta) = synthetic_state(&column, "ra_have_this");
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let refusal = column
            .stagnant_lid_determinations(&state)
            .expect_err("one branch missing means the family's own prescription cannot be applied");
        assert_eq!(refusal.excluded.len(), 1);
        assert_eq!(refusal.excluded[0].convicted.len(), 1);
        assert_eq!(refusal.excluded[0].convicted[0].row, "row_b");
        assert!(
            refusal.sensitivity.is_empty(),
            "an unevaluable branch leaves no envelope either"
        );
    }

    #[test]
    fn an_unimplemented_selection_rule_is_a_typed_stop() {
        // The rules are fixed Rust and the membership is data, so a family declaring a rule this engine does not
        // implement is refused rather than quietly falling back to one that is implemented.
        let column = ConvectionScaling::from_toml_str(&synthetic_row(
            "row_a",
            "solo",
            "average_the_branches",
            "rayleigh_definition_key = \"ra_have_this\"",
        ))
        .expect("the synthetic column loads");
        let (ra, theta) = synthetic_state(&column, "ra_have_this");
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let refusal = column.stagnant_lid_determinations(&state).unwrap_err();
        assert_eq!(
            refusal.excluded[0].family_failures,
            vec![ScopeFailure::UnknownSelectionRule {
                rule: "average_the_branches".to_string()
            }]
        );
        assert!(refusal.sensitivity.is_empty());
    }

    #[test]
    fn rows_of_one_family_disagreeing_about_their_own_rule_convicts_the_family() {
        let column = ConvectionScaling::from_toml_str(&format!(
            "{}{}",
            synthetic_row(
                "row_a",
                "pair",
                "larger_branch",
                "rayleigh_definition_key = \"ra_have_this\""
            ),
            synthetic_row(
                "row_b",
                "pair",
                "sole_determination",
                "rayleigh_definition_key = \"ra_have_this\""
            ),
        ))
        .expect("the synthetic column loads");
        let (ra, theta) = synthetic_state(&column, "ra_have_this");
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let refusal = column.stagnant_lid_determinations(&state).unwrap_err();
        assert!(refusal.excluded[0]
            .family_failures
            .iter()
            .any(|f| matches!(f, ScopeFailure::InconsistentSelectionRule { .. })));
    }

    #[test]
    fn a_sole_determination_family_carrying_two_rows_convicts() {
        // A source that states ONE fit cannot have its family silently maximised over two rows, which is what a
        // combinator with no arity check would do.
        let column = ConvectionScaling::from_toml_str(&format!(
            "{}{}",
            synthetic_row(
                "row_a",
                "solo",
                "sole_determination",
                "rayleigh_definition_key = \"ra_have_this\""
            ),
            synthetic_row(
                "row_b",
                "solo",
                "sole_determination",
                "rayleigh_definition_key = \"ra_have_this\""
            ),
        ))
        .expect("the synthetic column loads");
        let (ra, theta) = synthetic_state(&column, "ra_have_this");
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let refusal = column.stagnant_lid_determinations(&state).unwrap_err();
        assert_eq!(
            refusal.excluded[0].family_failures,
            vec![ScopeFailure::OverfullSoleDetermination { rows: 2 }]
        );
    }

    #[test]
    fn an_untranscribed_scope_is_convicted_rather_than_passed() {
        // A CHECK THAT DID NOT RUN IS NOT A CHECK THAT PASSED. A row missing the rheology it was fitted at, or the
        // span it was measured over, or its family, cannot be scope-checked, so it is refused rather than selected
        // against tests that silently did not fire.
        let bare = "[[constant]]\nname = \"row_a\"\nvalue = \"0.48\"\n\
                    theta_exponent = \"-1.333333333333\"\nrayleigh_exponent = \"0.333333333333\"\n\
                    rayleigh_definition_key = \"ra_have_this\"\n\
                    theta_definition_key = \"theta_have_this\"\n\
                    citation = \"synthetic row, this test only\"\n";
        let column = ConvectionScaling::from_toml_str(bare).expect("the synthetic column loads");
        let ra = column
            .rayleigh_projection("ra_have_this", Fixed::from_int(16))
            .unwrap();
        let theta = column
            .theta_projection("theta_have_this", Fixed::from_int(15))
            .unwrap();
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let refusal = column.stagnant_lid_determinations(&state).unwrap_err();
        let failures = &refusal.excluded[0].convicted[0].failures;
        for field in ["family", "fitted_stress_exponent", "fitted_theta_ranges"] {
            assert!(
                failures.contains(&ScopeFailure::ScopeNotTranscribed { field }),
                "the missing '{field}' must convict, not pass: {failures:?}"
            );
        }
        assert!(refusal.excluded[0]
            .family_failures
            .contains(&ScopeFailure::ScopeNotTranscribed {
                field: "family_selection"
            }));
        // An untranscribed scope leaves NO sensitivity number either: the tests did not run, so nothing is known
        // about how far out of scope the row is.
        assert!(refusal.sensitivity.is_empty());
    }

    #[test]
    fn a_kernel_refusal_convicts_the_row_rather_than_dropping_it() {
        // A non-positive coefficient is not a scaling law, and the kernel says so. The row is reported as
        // convicted rather than quietly skipped, which is how a family could otherwise shrink without notice.
        let column = ConvectionScaling::from_toml_str(
            &synthetic_row(
                "row_a",
                "solo",
                "sole_determination",
                "rayleigh_definition_key = \"ra_have_this\"",
            )
            .replace("value = \"0.48\"", "value = \"-0.48\""),
        )
        .expect("the synthetic column loads");
        let (ra, theta) = synthetic_state(&column, "ra_have_this");
        let state = StagnantLidState {
            stress_exponent: Fixed::ONE,
            rayleigh: &[ra],
            theta: &[theta],
        };
        let refusal = column.stagnant_lid_determinations(&state).unwrap_err();
        assert_eq!(
            refusal.excluded[0].convicted[0].failures,
            vec![ScopeFailure::KernelRefused]
        );
        assert!(refusal.sensitivity.is_empty());
    }

    #[test]
    fn an_inverted_fitted_interval_fails_the_load_rather_than_being_swapped() {
        let bad = synthetic_row(
            "row_a",
            "solo",
            "sole_determination",
            "rayleigh_definition_key = \"ra_have_this\"",
        )
        .replace(
            "fitted_theta_ranges = [[\"1.0\", \"100.0\"]]",
            "fitted_theta_ranges = [[\"100.0\", \"1.0\"]]",
        );
        assert!(matches!(
            ConvectionScaling::from_toml_str(&bad),
            Err(ScalingError::BadValue { .. })
        ));
    }
}
