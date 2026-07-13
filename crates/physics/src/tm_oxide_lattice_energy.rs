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

//! The transition-metal monoxide Born-Haber lattice energies (`crates/physics/data/tm_oxide_lattice_energy.toml`),
//! the localized route's measured `[M]` fill.
//!
//! Per rock-salt divalent 3d monoxide the MEASURED Born-Haber lattice energy (kJ/mol, released/negative), cited.
//! The materials localized route (D3 follow-on #3) reads this to fill the correlation guard's Localized slot with
//! a real cohesive number for the seeded Mott insulators (NiO/CoO/FeO/MnO), escalating for any unseeded oxide. No
//! consumer is wired to it in any pinned run path yet (byte-neutral).
//!
//! WHY MEASURED, NOT DERIVED (the gate's ruling). A Mott insulator is ionically bonded, but D1's Born-Lande
//! estimator cannot score it: the Born exponent keys on the ion's isoelectronic noble-gas core, and a 3d cation
//! (Ni2+ = 26 electrons) is not isoelectronic with any noble gas, so it falls through, and the d-electron Born
//! exponent is not cleanly derivable (compressibility-fitting is circular; the Ar-core value is wrong for a
//! 3d-mediated repulsion). So the honest fill is the MEASURED Born-Haber value, the top rung of the provenance
//! ladder (measured, then estimator, then compute-once): escalating to the cited measurement when the substrate
//! cannot derive the value is the ladder working, not a lookup dodging derive-first. There is NO derived band; the
//! uncertainty is the measurement's. The analog of the D1 ionic validation set for the localized class.

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the TM-oxide lattice-energy set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmOxideError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// A composition appears twice.
    Duplicate(String),
    /// The lattice energy is non-negative (the released convention is negative, the bound solid below free ions).
    NotReleased(String),
}

impl fmt::Display for TmOxideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TmOxideError::Parse(m) => write!(f, "tm-oxide parse error: {m}"),
            TmOxideError::BadValue(m) => write!(f, "tm-oxide value error: {m}"),
            TmOxideError::MissingSource(m) => write!(f, "tm-oxide row without citation: {m}"),
            TmOxideError::Duplicate(m) => write!(f, "duplicate tm-oxide composition: {m}"),
            TmOxideError::NotReleased(m) => {
                write!(f, "tm-oxide lattice energy not released (>= 0): {m}")
            }
        }
    }
}

impl std::error::Error for TmOxideError {}

#[derive(Debug, Default, Deserialize, Serialize)]
struct OxideFile {
    #[serde(default)]
    oxide: Vec<OxideDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct OxideDef {
    name: String,
    #[serde(default)]
    composition: BTreeMap<String, u32>,
    #[serde(default)]
    born_haber_kj_per_mol: String,
    #[serde(default)]
    source: String,
}

/// The TM-oxide lattice-energy set: per canonical composition, the cited Born-Haber lattice energy (kJ/mol).
#[derive(Debug, Clone, Default)]
pub struct TmOxideLatticeEnergies {
    // Keyed by the canonical (BTreeMap-sorted) composition string, so a lookup is order-independent.
    by_composition: BTreeMap<String, Fixed>,
}

/// The canonical key of a composition: the elements in sorted order with their counts, so `{Ni:1, O:1}` and the
/// reverse both key the same row.
fn composition_key(composition: &BTreeMap<String, u32>) -> String {
    composition
        .iter()
        .map(|(el, n)| format!("{el}{n}"))
        .collect::<Vec<_>>()
        .join("")
}

impl TmOxideLatticeEnergies {
    /// Load the set from a TOML string. Every row must carry a citation and a released (negative) lattice energy.
    pub fn from_toml_str(s: &str) -> Result<Self, TmOxideError> {
        let file: OxideFile = toml::from_str(s).map_err(|e| TmOxideError::Parse(e.to_string()))?;
        let mut by_composition = BTreeMap::new();
        for o in file.oxide {
            if o.source.trim().is_empty() {
                return Err(TmOxideError::MissingSource(o.name.clone()));
            }
            let energy = Fixed::from_decimal_str(o.born_haber_kj_per_mol.trim())
                .map_err(|d| TmOxideError::BadValue(format!("{}: {d}", o.name)))?;
            if energy >= Fixed::ZERO {
                return Err(TmOxideError::NotReleased(o.name.clone()));
            }
            let key = composition_key(&o.composition);
            if by_composition.insert(key, energy).is_some() {
                return Err(TmOxideError::Duplicate(o.name));
            }
        }
        Ok(TmOxideLatticeEnergies { by_composition })
    }

    /// The embedded standard set (`data/tm_oxide_lattice_energy.toml`).
    pub fn standard() -> Result<Self, TmOxideError> {
        Self::from_toml_str(include_str!("../data/tm_oxide_lattice_energy.toml"))
    }

    /// The cited Born-Haber lattice energy (kJ/mol, released/negative) for a composition, or `None` when the
    /// oxide is not in the seeded set (the localized route then escalates). Order-independent in the composition.
    pub fn lattice_energy(&self, composition: &[(String, u32)]) -> Option<Fixed> {
        let map: BTreeMap<String, u32> = composition.iter().cloned().collect();
        self.by_composition.get(&composition_key(&map)).copied()
    }

    /// The number of seeded oxides.
    pub fn len(&self) -> usize {
        self.by_composition.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.by_composition.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> TmOxideLatticeEnergies {
        TmOxideLatticeEnergies::standard().expect("the TM-oxide set loads")
    }

    fn comp(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
        pairs.iter().map(|(s, c)| ((*s).to_string(), *c)).collect()
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_seed_is_the_four_mott_insulator_monoxides() {
        let s = set();
        assert_eq!(s.len(), 4, "the seed is NiO/CoO/FeO/MnO");
        for oxide in [("Ni", 1), ("Co", 1), ("Fe", 1), ("Mn", 1)] {
            assert!(
                s.lattice_energy(&comp(&[oxide, ("O", 1)])).is_some(),
                "{}O is seeded",
                oxide.0
            );
        }
    }

    #[test]
    fn the_lattice_energy_matches_the_cited_value_order_independently() {
        let s = set();
        // NiO, the deepest, cited -3908 kJ/mol; the lookup is order-independent in the composition.
        let nio = s
            .lattice_energy(&comp(&[("Ni", 1), ("O", 1)]))
            .expect("NiO");
        assert!(
            close(nio, -3908.0, 0.5),
            "NiO = -3908, got {}",
            nio.to_f64_lossy()
        );
        let nio_rev = s
            .lattice_energy(&comp(&[("O", 1), ("Ni", 1)]))
            .expect("NiO reversed");
        assert_eq!(nio, nio_rev, "the lookup is order-independent");
        // MnO, the shallowest of the four, cited -3724.
        let mno = s
            .lattice_energy(&comp(&[("Mn", 1), ("O", 1)]))
            .expect("MnO");
        assert!(
            close(mno, -3724.0, 0.5),
            "MnO = -3724, got {}",
            mno.to_f64_lossy()
        );
    }

    #[test]
    fn the_series_trend_is_physical() {
        // Across the late 3d series the lattice energy deepens (smaller radius, stronger binding): MnO shallower
        // than NiO. The physical ordering the localized fill preserves.
        let s = set();
        let mno = s.lattice_energy(&comp(&[("Mn", 1), ("O", 1)])).unwrap();
        let nio = s.lattice_energy(&comp(&[("Ni", 1), ("O", 1)])).unwrap();
        assert!(
            nio < mno,
            "NiO is more strongly bound (more negative) than MnO"
        );
    }

    #[test]
    fn an_unseeded_oxide_is_absent() {
        let s = set();
        assert!(
            s.lattice_energy(&comp(&[("Ti", 1), ("O", 1)])).is_none(),
            "TiO (itinerant, not a seeded insulator) is absent so the route escalates"
        );
    }

    #[test]
    fn a_missing_citation_is_rejected() {
        let bad = r#"
[[oxide]]
name = "test oxide"
composition = { Ni = 1, O = 1 }
born_haber_kj_per_mol = "-3900"
source = ""
"#;
        assert!(matches!(
            TmOxideLatticeEnergies::from_toml_str(bad),
            Err(TmOxideError::MissingSource(_))
        ));
    }

    #[test]
    fn a_non_released_energy_is_rejected() {
        let bad = r#"
[[oxide]]
name = "test oxide"
composition = { Ni = 1, O = 1 }
born_haber_kj_per_mol = "3900"
source = "test"
"#;
        assert!(matches!(
            TmOxideLatticeEnergies::from_toml_str(bad),
            Err(TmOxideError::NotReleased(_))
        ));
    }
}
