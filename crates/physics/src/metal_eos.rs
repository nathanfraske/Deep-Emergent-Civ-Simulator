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

//! The elemental equation-of-state anchors floor (`crates/physics/data/metal_eos_anchors.toml`), the metallic
//! route's cohesive-energy EOS inputs.
//!
//! Per metal two MEASURED `[M]` values: the equilibrium molar volume (cm^3/mol) and the bulk modulus `B_0` (GPa),
//! cited to WebElements (the gate spot-checks against CRC/WebElements). The elemental Rose/Vinet cohesive-energy
//! EOS (the materials metallic route, D3-c) reads these two alongside the banked cohesive energy
//! (`periodic_table.toml` `atomization_enthalpy`, kJ/mol). No consumer is wired to them in any pinned run path yet
//! (a pure addition, byte-neutral).
//!
//! WHY PER-MOLE, NO AVOGADRO. The stored volume is the molar volume exactly as the source prints it (source-
//! verifiable), not a per-atom volume. The Rose length scale combines `B_0 * V_m`, which carries units
//! `GPa * cm^3/mol = kJ/mol`, the same energy-per-mole scale as the banked `E_coh`, so the EOS assembles
//! per-mole-consistently and no Avogadro conversion enters the path. The loader stores the two raw cited columns
//! and their positivity is guarded; the EOS derives its length scale from them in code (D3-c).
//!
//! WHAT IS NOT HERE: the Miedema `n_ws`. The alloy-model electron density `n_ws = 0.82 (B_0 / V_m)^(1/2)` (the
//! form cited across the Miedema literature, the 0.82 a cited model coefficient) is deliberately absent. It is an
//! ALLOY parameter (the elemental route does not read it), its only consumer (the Miedema formation-enthalpy term)
//! escalates until its book-fitted partner `phi*` is sourced, and its ABSOLUTE scale is book-pinned: the cited
//! form applied to these WebElements `B_0/V_m` reproduces the tabulated Miedema `n_ws^(1/3)` only to within roughly
//! six to ten percent (systematically low, versus de Boer/Boom/Miedema, Cohesion in Metals 1988). So a derived
//! `n_ws` would plant a value known to disagree with the accepted parameter; it rides with the escalating alloy
//! term, the same book-source seam as `phi*`.

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the metal EOS anchors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetalEosError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// A metal appears twice.
    Duplicate(String),
    /// The molar volume or bulk modulus is non-positive (a non-physical anchor).
    NonPositive(String),
}

impl fmt::Display for MetalEosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetalEosError::Parse(m) => write!(f, "metal-eos parse error: {m}"),
            MetalEosError::BadValue(m) => write!(f, "metal-eos value error: {m}"),
            MetalEosError::MissingSource(m) => write!(f, "metal-eos row without citation: {m}"),
            MetalEosError::Duplicate(m) => write!(f, "duplicate metal-eos entry: {m}"),
            MetalEosError::NonPositive(m) => write!(f, "metal-eos non-positive anchor: {m}"),
        }
    }
}

impl std::error::Error for MetalEosError {}

#[derive(Debug, Default, Deserialize, Serialize)]
struct MetalEosFile {
    #[serde(default)]
    metal: Vec<MetalDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct MetalDef {
    symbol: String,
    #[serde(default)]
    molar_volume: String,
    #[serde(default)]
    bulk_modulus_gpa: String,
    #[serde(default)]
    source: String,
}

/// One metal's EOS anchors: the measured equilibrium molar volume and bulk modulus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetalEosAnchor {
    /// The equilibrium molar volume in cm^3/mol (MEASURED `[M]`).
    pub molar_volume: Fixed,
    /// The bulk modulus `B_0` in GPa (MEASURED `[M]`).
    pub bulk_modulus_gpa: Fixed,
}

/// The elemental EOS anchors floor: per metal symbol, the two measured cohesive-energy EOS inputs.
#[derive(Debug, Clone, Default)]
pub struct MetalEosAnchors {
    by_symbol: BTreeMap<String, MetalEosAnchor>,
}

impl MetalEosAnchors {
    /// Load the metal EOS anchors from a TOML string. Every row must carry a citation and positive anchors.
    pub fn from_toml_str(s: &str) -> Result<Self, MetalEosError> {
        let file: MetalEosFile =
            toml::from_str(s).map_err(|e| MetalEosError::Parse(e.to_string()))?;
        let mut by_symbol = BTreeMap::new();
        for entry in file.metal {
            if entry.source.trim().is_empty() {
                return Err(MetalEosError::MissingSource(entry.symbol.clone()));
            }
            let molar_volume = Fixed::from_decimal_str(entry.molar_volume.trim()).map_err(|d| {
                MetalEosError::BadValue(format!("{} molar_volume: {d}", entry.symbol))
            })?;
            let bulk_modulus_gpa =
                Fixed::from_decimal_str(entry.bulk_modulus_gpa.trim()).map_err(|d| {
                    MetalEosError::BadValue(format!("{} bulk_modulus_gpa: {d}", entry.symbol))
                })?;
            if molar_volume <= Fixed::ZERO || bulk_modulus_gpa <= Fixed::ZERO {
                return Err(MetalEosError::NonPositive(entry.symbol.clone()));
            }
            let anchor = MetalEosAnchor {
                molar_volume,
                bulk_modulus_gpa,
            };
            if by_symbol.insert(entry.symbol.clone(), anchor).is_some() {
                return Err(MetalEosError::Duplicate(entry.symbol));
            }
        }
        Ok(MetalEosAnchors { by_symbol })
    }

    /// The embedded standard anchors (`data/metal_eos_anchors.toml`).
    pub fn standard() -> Result<Self, MetalEosError> {
        Self::from_toml_str(include_str!("../data/metal_eos_anchors.toml"))
    }

    /// The two EOS anchors for a metal, or `None` when absent.
    pub fn anchor(&self, symbol: &str) -> Option<MetalEosAnchor> {
        self.by_symbol.get(symbol).copied()
    }

    /// The equilibrium molar volume (cm^3/mol) for a metal, or `None` when absent.
    pub fn molar_volume(&self, symbol: &str) -> Option<Fixed> {
        self.by_symbol.get(symbol).map(|a| a.molar_volume)
    }

    /// The bulk modulus `B_0` (GPa) for a metal, or `None` when absent.
    pub fn bulk_modulus_gpa(&self, symbol: &str) -> Option<Fixed> {
        self.by_symbol.get(symbol).map(|a| a.bulk_modulus_gpa)
    }

    /// The number of metals with seeded EOS anchors.
    pub fn len(&self) -> usize {
        self.by_symbol.len()
    }

    /// Whether the anchors floor is empty.
    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchors() -> MetalEosAnchors {
        MetalEosAnchors::standard().expect("the metal EOS anchors load")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_seed_is_the_seven_cohesive_energy_metals() {
        let a = anchors();
        assert_eq!(
            a.len(),
            7,
            "the seed is the 7 metals with a banked cohesive energy (Na, Mg, Al, K, Ca, Ti, Fe)"
        );
        for symbol in ["Na", "Mg", "Al", "K", "Ca", "Ti", "Fe"] {
            assert!(a.anchor(symbol).is_some(), "{symbol} has EOS anchors");
        }
    }

    #[test]
    fn the_anchors_match_the_cited_source() {
        // Spot-check the two ends of the bulk-modulus range against the WebElements values.
        let a = anchors();
        let fe = a.anchor("Fe").expect("Fe anchors");
        assert!(
            close(fe.bulk_modulus_gpa, 170.0, 0.001),
            "Fe B_0 = 170 GPa, got {}",
            fe.bulk_modulus_gpa.to_f64_lossy()
        );
        assert!(
            close(fe.molar_volume, 7.09, 0.001),
            "Fe molar volume = 7.09 cm^3/mol, got {}",
            fe.molar_volume.to_f64_lossy()
        );
        let k = a.anchor("K").expect("K anchors");
        assert!(
            close(k.bulk_modulus_gpa, 3.1, 0.001),
            "K B_0 = 3.1 GPa, got {}",
            k.bulk_modulus_gpa.to_f64_lossy()
        );
        assert!(
            close(k.molar_volume, 45.94, 0.001),
            "K molar volume = 45.94 cm^3/mol, got {}",
            k.molar_volume.to_f64_lossy()
        );
    }

    #[test]
    fn the_bulk_modulus_times_molar_volume_is_the_e_coh_scale() {
        // The per-mole consistency the metallic route rides: B_0 [GPa] * V_m [cm^3/mol] = kJ/mol, the same energy
        // scale as the banked cohesive energy (a few hundred kJ/mol). For Fe: 170 * 7.09 / 1000 ~ 1.21, in units
        // where 1 GPa*cm^3/mol = 1 kJ/mol, so B_0 * V_m ~ 1205 kJ/mol, the correct order for a cohesive scale.
        let a = anchors();
        let fe = a.anchor("Fe").expect("Fe anchors");
        let product = fe
            .bulk_modulus_gpa
            .checked_mul(fe.molar_volume)
            .expect("B_0 * V_m does not overflow");
        // 170 * 7.09 = 1205.3 (kJ/mol), a physical cohesive-energy order of magnitude.
        assert!(
            close(product, 1205.3, 0.1),
            "Fe B_0 * V_m = 1205.3 kJ/mol, got {}",
            product.to_f64_lossy()
        );
    }

    #[test]
    fn a_missing_citation_is_rejected() {
        let bad = r#"
[[metal]]
symbol = "Zz"
molar_volume = "10.0"
bulk_modulus_gpa = "50"
source = ""
"#;
        assert!(matches!(
            MetalEosAnchors::from_toml_str(bad),
            Err(MetalEosError::MissingSource(_))
        ));
    }

    #[test]
    fn a_non_positive_anchor_is_rejected() {
        let bad = r#"
[[metal]]
symbol = "Zz"
molar_volume = "0"
bulk_modulus_gpa = "50"
source = "test"
"#;
        assert!(matches!(
            MetalEosAnchors::from_toml_str(bad),
            Err(MetalEosError::NonPositive(_))
        ));
    }

    #[test]
    fn a_duplicate_metal_is_rejected() {
        let bad = r#"
[[metal]]
symbol = "Zz"
molar_volume = "10.0"
bulk_modulus_gpa = "50"
source = "test"

[[metal]]
symbol = "Zz"
molar_volume = "11.0"
bulk_modulus_gpa = "55"
source = "test"
"#;
        assert!(matches!(
            MetalEosAnchors::from_toml_str(bad),
            Err(MetalEosError::Duplicate(_))
        ));
    }
}
