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

//! The Stoner column (`crates/physics/data/stoner.toml`), the itinerant magnetism criterion's input: per element the
//! Stoner exchange parameter `I` and the NONMAGNETIC band density of states at the Fermi level `N(E_F)`. The
//! materials Stoner classifier (`civsim_materials::stoner`) reads `I` and `N` to form the Stoner product `I * N`; a
//! value above 1 is the itinerant-ferromagnet threshold.
//!
//! UNITS (Janak's convention): `I` in rydberg, `N` in states per rydberg per atom. The product `I * N` is
//! DIMENSIONLESS and CONVENTION-INDEPENDENT, so the criterion compares it against 1 directly.
//!
//! DEFINITION TAG on `N`: this is the CALCULATED NONMAGNETIC BAND DOS (from the paramagnetic band structure), NOT a
//! calorimetric `(1 + lambda)`-dressed `gamma` DOS. The consuming newtype `NonmagneticDos` carries that promise so
//! the wrong DOS cannot be wired (the composition error this thread produced).
//!
//! PROVENANCE (owner-delivered primary, admissible): J. F. Janak, "Uniform susceptibilities of metallic elements,"
//! Physical Review B 16, 255 (1977), Table I. Audited for internal consistency (every `I * N` reproduces Janak's
//! tabulated Stoner product; the enhancement `1 / (1 - I * N)` reproduces the tabulated enhancement to rounding),
//! superseding two earlier rendered compilations the definition tag caught as mutually inconsistent. Tag:
//! `[Janak 1977 Phys Rev B 16 255 Table I, cited]`. No consumer is wired to it in any pinned run path (byte-neutral).

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the Stoner column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StonerError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// An element appears twice.
    Duplicate(String),
    /// A non-positive `I` or `N` (both are positive quantities; a non-positive value is not a Stoner input).
    NotPositive(String),
}

impl fmt::Display for StonerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StonerError::Parse(m) => write!(f, "stoner parse error: {m}"),
            StonerError::BadValue(m) => write!(f, "stoner value error: {m}"),
            StonerError::MissingSource(m) => write!(f, "stoner row without citation: {m}"),
            StonerError::Duplicate(m) => write!(f, "duplicate stoner element: {m}"),
            StonerError::NotPositive(m) => write!(f, "stoner value not positive: {m}"),
        }
    }
}

impl std::error::Error for StonerError {}

/// One element's Stoner inputs: the exchange parameter `I` (rydberg) and the nonmagnetic band DOS `N(E_F)`
/// (states/Ry/atom). The product `I * N` is the dimensionless Stoner discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StonerEntry {
    /// The Stoner exchange parameter `I` in rydberg (positive).
    pub i_ry: Fixed,
    /// The nonmagnetic band density of states `N(E_F)` in states/Ry/atom (positive).
    pub n_states_per_ry_atom: Fixed,
}

/// The Stoner column: per element symbol, the cited `I` and `N`.
#[derive(Debug, Clone, Default)]
pub struct StonerColumn {
    by_symbol: BTreeMap<String, StonerEntry>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct StonerFile {
    #[serde(default)]
    stoner: Vec<StonerDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct StonerDef {
    symbol: String,
    #[serde(default)]
    i_ry: String,
    #[serde(default)]
    n_states_per_ry_atom: String,
    #[serde(default)]
    source: String,
}

impl StonerColumn {
    /// Load the column from a TOML string. Every row must carry a citation and positive `I` and `N`.
    pub fn from_toml_str(s: &str) -> Result<Self, StonerError> {
        let file: StonerFile = toml::from_str(s).map_err(|e| StonerError::Parse(e.to_string()))?;
        let mut by_symbol = BTreeMap::new();
        for e in file.stoner {
            if e.source.trim().is_empty() {
                return Err(StonerError::MissingSource(e.symbol.clone()));
            }
            let i_ry = Fixed::from_decimal_str(e.i_ry.trim())
                .map_err(|d| StonerError::BadValue(format!("{} I: {d}", e.symbol)))?;
            let n = Fixed::from_decimal_str(e.n_states_per_ry_atom.trim())
                .map_err(|d| StonerError::BadValue(format!("{} N: {d}", e.symbol)))?;
            if i_ry <= Fixed::ZERO || n <= Fixed::ZERO {
                return Err(StonerError::NotPositive(e.symbol.clone()));
            }
            if by_symbol
                .insert(
                    e.symbol.clone(),
                    StonerEntry {
                        i_ry,
                        n_states_per_ry_atom: n,
                    },
                )
                .is_some()
            {
                return Err(StonerError::Duplicate(e.symbol));
            }
        }
        Ok(StonerColumn { by_symbol })
    }

    /// The embedded standard column (`data/stoner.toml`, the Janak 1977 Table I values).
    pub fn standard() -> Result<Self, StonerError> {
        Self::from_toml_str(include_str!("../data/stoner.toml"))
    }

    /// The Stoner inputs for an element, or `None` when the element is not in the column (the classifier then
    /// escalates: an unseeded element has no criterion input).
    pub fn entry(&self, symbol: &str) -> Option<StonerEntry> {
        self.by_symbol.get(symbol).copied()
    }

    /// The number of seeded elements.
    pub fn len(&self) -> usize {
        self.by_symbol.len()
    }

    /// Whether the column is empty.
    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_column_carries_the_janak_values_and_reproduces_the_stoner_products() {
        // The Janak 1977 Table I values, audited: I * N reproduces the tabulated Stoner product. Fe 0.0339*33.00 =
        // 1.119 (> 1, ferromagnetic); Ni 0.0345*59.57 = 2.055 (> 1); Cu 0.0343*3.93 = 0.135 (deep paramagnet).
        let c = StonerColumn::standard().expect("the column loads");
        assert!(!c.is_empty(), "the column carries the Janak values");
        let fe = c.entry("Fe").expect("Fe");
        let fe_product = fe
            .i_ry
            .checked_mul(fe.n_states_per_ry_atom)
            .expect("Fe I*N");
        assert!(
            close(fe_product, 1.119, 0.002),
            "Fe I*N ~ 1.119, got {}",
            fe_product.to_f64_lossy()
        );
        let cu = c.entry("Cu").expect("Cu");
        let cu_product = cu
            .i_ry
            .checked_mul(cu.n_states_per_ry_atom)
            .expect("Cu I*N");
        assert!(
            close(cu_product, 0.135, 0.002),
            "Cu I*N ~ 0.135, got {}",
            cu_product.to_f64_lossy()
        );
        // The ferromagnets, the marginal near-misses, and the deep controls are all seeded.
        for symbol in ["Fe", "Co", "Ni", "Pd", "Cu", "Ag", "Al"] {
            assert!(c.entry(symbol).is_some(), "{symbol} is seeded");
        }
    }

    #[test]
    fn a_missing_citation_and_a_non_positive_value_are_rejected() {
        let no_src = "[[stoner]]\nsymbol = \"Fe\"\ni_ry = \"0.0339\"\nn_states_per_ry_atom = \"33.0\"\nsource = \"\"\n";
        assert!(matches!(
            StonerColumn::from_toml_str(no_src),
            Err(StonerError::MissingSource(_))
        ));
        let neg = "[[stoner]]\nsymbol = \"Fe\"\ni_ry = \"0.0339\"\nn_states_per_ry_atom = \"-1.0\"\nsource = \"test\"\n";
        assert!(matches!(
            StonerColumn::from_toml_str(neg),
            Err(StonerError::NotPositive(_))
        ));
    }
}
