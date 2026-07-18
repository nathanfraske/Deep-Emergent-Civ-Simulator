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
//! `a` IS CONVENTION-DEPENDENT, an O(1) number, not universal, and the column states the seam plainly: `a = 1.0`
//! is the single-boundary-layer (peel-away) form the planetary parameterized-convection literature uses (Stevenson
//! et al. 1983, Komacek and Abbot 2016, Vazan et al. 2018); `a = 2^(-4/3) ~ 0.397` is the symmetric
//! two-boundary-layer form. The band spans them, and which point applies is a HEATING-MODE modeling choice
//! (basal / single-lid toward 1.0, internal / symmetric toward 0.40) surfaced for the owner, not fabricated here.
//! The bare coefficient `C = 0.294` of the un-normalized `Nu = C Ra^(1/3)` is a DIFFERENT normalization and is
//! carried as its own row so it can never be read into the `a` slot.

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

/// The cited parameterized-convection scaling constants, keyed by name.
#[derive(Clone, Debug)]
pub struct ConvectionScaling {
    constants: Vec<(String, ScalingConstant)>,
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
            constants.push((
                c.name.trim().to_string(),
                ScalingConstant {
                    value,
                    band_lo,
                    band_hi,
                },
            ));
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
            .find(|(n, _)| n == name)
            .map(|(_, c)| *c)
    }

    /// The Nusselt PREFACTOR `a` for `Nu = a (Ra/Ra_crit)^(1/3)`: the single-boundary-layer planetary value with
    /// its two-convention band. Multiplies the heat-loss flux, never the lid boundary layer.
    pub fn nusselt_prefactor(&self) -> Option<ScalingConstant> {
        self.constant("nu_ra_prefactor_a")
    }

    /// The critical Rayleigh number (marginal-stability eigenvalue) for a mechanical boundary condition.
    pub fn critical_rayleigh(&self, bc: BoundaryCondition) -> Option<Fixed> {
        self.constant(bc.row_name()).map(|c| c.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn an_uncited_column_refuses_to_load() {
        let no_src = "[[constant]]\nname = \"x\"\nvalue = \"1.0\"\n";
        assert_eq!(
            ConvectionScaling::from_toml_str(no_src).unwrap_err(),
            ScalingError::MissingProvenance
        );
    }
}
