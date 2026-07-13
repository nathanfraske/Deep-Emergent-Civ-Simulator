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

//! The 3d-state radius floor (`crates/physics/data/d_state_radii.toml`), the correlation classifier's
//! bandwidth-axis input.
//!
//! Per element the 3d-STATE radius in Angstrom, DERIVED `[D]`-from-`[M]`: the stored datum is the MEASURED `[M]`
//! effective nuclear charge `Zeff(3d)` (Clementi and Raimondi 1963, SCF screening constants), and the radius is
//! derived in code as `r_3d = (n*)^2 * a_0 / Zeff` with `n* = 3` (Slater, for the 3d shell) and `a_0` the Bohr
//! radius (the hydrogenic mean-radius relation). Harrison's d-band width reads it (`W ~ r_d^3 / d^5`), where `d`
//! is the banked Shannon-sum interionic distance. No consumer is wired to it in any pinned run path yet
//! (byte-neutral); the materials correlation classifier (D2b) reads it.
//!
//! THE 4s-VERSUS-3d CATCH, made a build guard. The readily-tabulated "orbital radius" for a 3d metal is the
//! OUTERMOST (4s) orbital, roughly 1.0 to 1.7 Angstrom, NOT the compact 3d-state radius, roughly 0.3 to 0.7.
//! Deriving from `Zeff(3d)` lands the d-state scale by construction, and the loader's SCALE GUARD rejects any
//! derived radius outside the d-state window (0.2 to 0.8 Angstrom), so a future entry of the wrong orbital fails
//! the build the way the ionization ladder's strictly-increasing guard catches a bad ladder.
//!
//! NORMALIZATION ROBUSTNESS (the D2b self-consistency condition). The absolute `r_d` scale is absorbed by D2b's
//! MIT-calibrated screening (a uniform scaling of `r_d` by `k` scales `W` by `k^3`, which the screening re-fits
//! to the same `U/W = 1` boundary), so only the RELATIVE contraction across the series is load-bearing, and
//! `Zeff` captures it (it rises across the series, so `r_3d` contracts). D2b MUST calibrate its screening
//! against this same `r_d` source.

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// The Slater effective principal quantum number for the 3d shell (`n* = 3` for `n = 3`).
const N_STAR_3D: i32 = 3;

/// The Bohr radius `a_0 = 0.52917721 Angstrom` (CODATA), a fundamental length constant (the physics floor),
/// built by exact ratio. The hydrogenic mean-radius scale in `r = (n*)^2 a_0 / Zeff`.
fn bohr_radius_angstrom() -> Fixed {
    Fixed::from_ratio(52_917_721, 100_000_000)
}

/// The d-state scale window (Angstrom): the derived radius must be the compact 3d-state scale, not the outermost
/// (4s) orbital (~1.0 to 1.7). A validity guard, not a world-content value: it only REJECTS a wrong-orbital
/// entry at load, the way the ladder's strictly-increasing guard rejects a bad ionization ladder.
fn d_state_radius_min() -> Fixed {
    Fixed::from_ratio(2, 10)
}
fn d_state_radius_max() -> Fixed {
    Fixed::from_ratio(8, 10)
}

/// What can go wrong loading the d-state-radius data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DStateRadiusError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point, or the derivation overflowed.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// An element appears twice.
    Duplicate(String),
    /// The `Zeff` is non-positive (a non-physical screening) so the radius does not derive.
    BadZeff(String),
    /// The derived radius is outside the 3d-state scale window (the 4s-orbital trap guard).
    OffScale(String),
}

impl fmt::Display for DStateRadiusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DStateRadiusError::Parse(m) => write!(f, "d-state-radius parse error: {m}"),
            DStateRadiusError::BadValue(m) => write!(f, "d-state-radius value error: {m}"),
            DStateRadiusError::MissingSource(m) => {
                write!(f, "d-state-radius row without citation: {m}")
            }
            DStateRadiusError::Duplicate(m) => write!(f, "duplicate d-state-radius element: {m}"),
            DStateRadiusError::BadZeff(m) => write!(f, "d-state-radius non-positive Zeff: {m}"),
            DStateRadiusError::OffScale(m) => {
                write!(
                    f,
                    "d-state-radius outside the 3d-state scale (the 4s trap): {m}"
                )
            }
        }
    }
}

impl std::error::Error for DStateRadiusError {}

#[derive(Debug, Default, Deserialize, Serialize)]
struct DStateFile {
    #[serde(default)]
    d_state: Vec<DStateDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct DStateDef {
    symbol: String,
    #[serde(default)]
    z_eff_3d: String,
    #[serde(default)]
    source: String,
}

/// One element's 3d-state datum: the measured `Zeff(3d)` input and the radius derived from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DStateRadius {
    /// The measured `[M]` effective nuclear charge `Zeff(3d)` (Clementi-Raimondi).
    pub z_eff_3d: Fixed,
    /// The derived `[D]` 3d-state radius in Angstrom, `(n*)^2 a_0 / Zeff`.
    pub radius_angstrom: Fixed,
}

/// The 3d-state radius floor: per element symbol, the `Zeff`-derived radius.
#[derive(Debug, Clone, Default)]
pub struct DStateRadii {
    by_symbol: BTreeMap<String, DStateRadius>,
}

/// Derive the 3d-state radius from the effective nuclear charge: `r = (n*)^2 a_0 / Zeff`.
fn derive_radius(z_eff: Fixed) -> Option<Fixed> {
    let n_star_sq = Fixed::from_int(N_STAR_3D * N_STAR_3D);
    n_star_sq
        .checked_mul(bohr_radius_angstrom())?
        .checked_div(z_eff)
}

impl DStateRadii {
    /// Load the d-state radii from a TOML string. Every row must carry a citation and a positive `Zeff`, and its
    /// DERIVED radius must fall in the 3d-state scale window (the 4s-orbital guard).
    pub fn from_toml_str(s: &str) -> Result<Self, DStateRadiusError> {
        let file: DStateFile =
            toml::from_str(s).map_err(|e| DStateRadiusError::Parse(e.to_string()))?;
        let mut by_symbol = BTreeMap::new();
        for entry in file.d_state {
            if entry.source.trim().is_empty() {
                return Err(DStateRadiusError::MissingSource(entry.symbol.clone()));
            }
            let z_eff = Fixed::from_decimal_str(entry.z_eff_3d.trim())
                .map_err(|d| DStateRadiusError::BadValue(format!("{} Zeff: {d}", entry.symbol)))?;
            if z_eff <= Fixed::ZERO {
                return Err(DStateRadiusError::BadZeff(entry.symbol.clone()));
            }
            let radius = derive_radius(z_eff).ok_or_else(|| {
                DStateRadiusError::BadValue(format!(
                    "{} radius derivation overflowed",
                    entry.symbol
                ))
            })?;
            // The scale guard: the derived radius must be the compact 3d-state scale, never the 4s orbital.
            if radius < d_state_radius_min() || radius > d_state_radius_max() {
                return Err(DStateRadiusError::OffScale(format!(
                    "{}: derived {} Angstrom (Zeff {})",
                    entry.symbol,
                    radius.to_f64_lossy(),
                    z_eff.to_f64_lossy()
                )));
            }
            let datum = DStateRadius {
                z_eff_3d: z_eff,
                radius_angstrom: radius,
            };
            if by_symbol.insert(entry.symbol.clone(), datum).is_some() {
                return Err(DStateRadiusError::Duplicate(entry.symbol));
            }
        }
        Ok(DStateRadii { by_symbol })
    }

    /// The embedded standard d-state radii (`data/d_state_radii.toml`).
    pub fn standard() -> Result<Self, DStateRadiusError> {
        Self::from_toml_str(include_str!("../data/d_state_radii.toml"))
    }

    /// The derived 3d-state radius in Angstrom for an element, or `None` when absent.
    pub fn radius(&self, symbol: &str) -> Option<Fixed> {
        self.by_symbol.get(symbol).map(|d| d.radius_angstrom)
    }

    /// The measured `Zeff(3d)` for an element (the `[M]` input the radius derives from), or `None` when absent.
    pub fn z_eff(&self, symbol: &str) -> Option<Fixed> {
        self.by_symbol.get(symbol).map(|d| d.z_eff_3d)
    }

    /// The number of elements with a seeded d-state radius.
    pub fn len(&self) -> usize {
        self.by_symbol.len()
    }

    /// Whether the d-state radius floor is empty.
    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radii() -> DStateRadii {
        DStateRadii::standard().expect("the d-state radii load")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_seed_is_the_3d_series() {
        let r = radii();
        assert_eq!(r.len(), 10, "the seed is the 3d transition series Sc-Zn");
    }

    #[test]
    fn the_radius_is_the_compact_d_state_scale_derived_from_zeff() {
        // Every derived radius is the compact 3d-state scale (0.3 to 0.7 Angstrom), NOT the 4s (1.0 to 1.7). Cr
        // and Cu spot-check the derivation r = 9 a_0 / Zeff: Cr 9 * 0.52918 / 9.757 = 0.488, Cu / 13.201 = 0.361.
        let r = radii();
        let cr = r.radius("Cr").expect("Cr radius");
        assert!(
            close(cr, 0.488, 0.01),
            "Cr r_3d ~ 0.488 A, got {}",
            cr.to_f64_lossy()
        );
        let cu = r.radius("Cu").expect("Cu radius");
        assert!(
            close(cu, 0.361, 0.01),
            "Cu r_3d ~ 0.361 A, got {}",
            cu.to_f64_lossy()
        );
        for symbol in ["Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn"] {
            let radius = r.radius(symbol).expect("radius present");
            assert!(
                radius > d_state_radius_min() && radius < d_state_radius_max(),
                "{symbol} r_3d {} is the compact d-state scale",
                radius.to_f64_lossy()
            );
            // And it is far below the 4s scale, so the 4s trap is genuinely excluded.
            assert!(
                radius < Fixed::from_ratio(9, 10),
                "{symbol} r_3d is well below the 4s scale"
            );
        }
    }

    #[test]
    fn the_radius_contracts_monotonically_across_the_series() {
        // The physical trend: r_3d contracts as Z (and Zeff) rise across the row. In atomic-number order the
        // derived radius strictly decreases, the relative pattern D2b's siting rides.
        let r = radii();
        let series = ["Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn"];
        let mut prev = f64::INFINITY;
        for symbol in series {
            let radius = r.radius(symbol).expect("radius present").to_f64_lossy();
            assert!(
                radius < prev,
                "{symbol} r_3d {radius} should be smaller than the prior element {prev} (contraction)"
            );
            prev = radius;
        }
    }

    #[test]
    fn the_scale_guard_rejects_the_4s_orbital_trap() {
        // THE CATCH AS A BUILD GUARD. A Zeff that would derive a 4s-scale radius (the outermost-orbital trap) is
        // rejected at load. Fe's true Zeff(3d) is ~11.18 (r ~ 0.426); a value near 3 would derive r ~ 1.6 (the
        // 4s scale), which the scale guard rejects rather than silently accepting the wrong orbital.
        let trap = r#"
[[d_state]]
symbol = "Zz"
z_eff_3d = "3.0"
source = "test (a 4s-scale Zeff)"
"#;
        assert!(matches!(
            DStateRadii::from_toml_str(trap),
            Err(DStateRadiusError::OffScale(_))
        ));
    }

    #[test]
    fn a_non_positive_zeff_is_rejected() {
        let bad = r#"
[[d_state]]
symbol = "Zz"
z_eff_3d = "0"
source = "test"
"#;
        assert!(matches!(
            DStateRadii::from_toml_str(bad),
            Err(DStateRadiusError::BadZeff(_))
        ));
    }

    #[test]
    fn a_missing_citation_is_rejected() {
        let bad = r#"
[[d_state]]
symbol = "Zz"
z_eff_3d = "10.0"
source = ""
"#;
        assert!(matches!(
            DStateRadii::from_toml_str(bad),
            Err(DStateRadiusError::MissingSource(_))
        ));
    }
}
