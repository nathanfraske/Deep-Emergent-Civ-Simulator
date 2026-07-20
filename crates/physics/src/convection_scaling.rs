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
//! FOUR ARE BANKED AND NONE IS THE DEFAULT, because the literature disagrees along two real axes and the choice
//! belongs to the world's own state. Batra and Foley 2021 fit the linearized Frank-Kamenetskii family on their own
//! models and split it by convection pattern: `nu_stag_steady_C2` when the pattern is steady, and
//! `nu_stag_time_dependent_C1` when it is time-dependent, which is the branch a purely internally heated interior
//! takes. Schulz et al. 2020 ran a full Arrhenius viscosity WITH an activation volume on this engine's own creep
//! bank and got a visibly shallower `theta` exponent, banked as `nu_stag_arrhenius_internal_ra` and
//! `nu_stag_arrhenius_harmonic_ra` (which differ in the viscosity average their Rayleigh number is formed on, so
//! reading one against the other's `Ra` is the same class of normalization error the bare `C` guards against).
//! The kernel that consumes a convention is [`crate::laws::ln_stagnant_lid_nusselt`], and the `theta` it takes is
//! [`crate::laws::stagnant_lid_rheological_theta`].
//!
//! NO ROW HERE IS FITTED AT THIS ENGINE'S OWN STRESS EXPONENT. That gap is real, it is recorded at the kernel, and
//! it is why nothing here is promoted to a default.

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

/// One parsed row: the constant every consumer reads, plus the two exponents only a stagnant-lid row carries.
#[derive(Clone, Debug)]
struct Row {
    name: String,
    constant: ScalingConstant,
    theta_exponent: Option<Fixed>,
    rayleigh_exponent: Option<Fixed>,
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
            if c.citation
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                return Err(ScalingError::MissingProvenance);
            }
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
            constants.push(Row {
                name: c.name.trim().to_string(),
                constant: ScalingConstant {
                    value,
                    band_lo,
                    band_hi,
                },
                theta_exponent,
                rayleigh_exponent,
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

    /// The critical Rayleigh number (marginal-stability eigenvalue) for a mechanical boundary condition.
    pub fn critical_rayleigh(&self, bc: BoundaryCondition) -> Option<Fixed> {
        self.constant(bc.row_name()).map(|c| c.value)
    }
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
}
