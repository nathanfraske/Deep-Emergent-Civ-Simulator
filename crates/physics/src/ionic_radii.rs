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

//! The Shannon (1976) crystal ionic radii floor (`crates/physics/data/ionic_radii.toml`).
//!
//! Per (ion, oxidation state, coordination, spin) the crystal radius in Angstrom, MEASURED `[M]` (Shannon
//! fitted them to over a thousand observed interatomic distances; a laboratory refutes one by diffraction). A
//! sibling of the periodic table and the phase registry, read by the materials substrate's coordination and
//! lattice-energy stages; no consumer is wired to it yet (a pure addition, byte-neutral).
//!
//! The CONVENTION is explicit and load-bearing. An ionic radius is not the observable (the observable is the
//! interatomic distance; the split into two radii is a convention), and Shannon published two sets that
//! reproduce the same distances: CRYSTAL radii (`r(O2-) = 1.26`) and EFFECTIVE radii (`r(O2-) = 1.40`). This
//! file carries the CRYSTAL set, a correctness choice: Pauling's radius-ratio window is pure geometry, so the
//! radii must be the convention whose ratios match it (crystal `Si4+/O2- = 0.40/1.26 = 0.317` is correctly in
//! the tetrahedral window; the effective `0.26/1.40 = 0.186` wrongly falls below). The convention and its
//! anchor are stored so a consumer reading a ratio against the wrong window is a detectable mismatch, and the
//! matched-pair invariant is checked at load. The radius RATIO computed downstream is a derived `[E]` estimator
//! (a geometric pre-filter for coordination), never the authority: the disposer's free-energy minimization
//! decides coordination and the ratio escalates at the boundary cases (the resolution ladder).

use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// One ion's crystal radius at a coordination (and spin state, where the radius splits by spin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonicRadius {
    /// The element symbol, for example `Si`.
    pub symbol: String,
    /// The oxidation state (the ion charge), for example `4` for `Si4+`, `-2` for `O2-`.
    pub oxidation_state: i8,
    /// The coordination number (the number of nearest-neighbour anions), for example `4` (tetrahedral).
    pub coordination: u8,
    /// The spin state (`high` or `low`) for a transition-metal ion whose radius splits by spin; `None` when the
    /// radius does not depend on spin.
    pub spin: Option<String>,
    /// The crystal radius in Angstrom (fixed-point, parsed from a decimal string by integer arithmetic).
    pub crystal_radius: Fixed,
    /// The citation (every radius is real-with-source).
    pub source: String,
}

/// The Shannon crystal-radius floor: the convention and its anchor, and the per-ion radii.
#[derive(Debug, Clone)]
pub struct IonicRadii {
    convention: String,
    anchor_symbol: String,
    anchor_oxidation_state: i8,
    anchor_coordination: u8,
    anchor_radius: Fixed,
    radii: Vec<IonicRadius>,
}

/// What can go wrong loading or reading the ionic-radius floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadiiError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// The data file could not be read.
    Io(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue {
        /// The ion the value belongs to.
        ion: String,
        /// What went wrong.
        detail: String,
    },
    /// An entry (or the file) carries no citation (every radius is real-with-source).
    MissingProvenance(String),
    /// The convention is not the supported `crystal` set, or the anchor does not match the convention (the
    /// matched-pair invariant: `crystal` must anchor on `r(O2-) = 1.26`).
    BadConvention(String),
    /// The convention anchor ion is absent from the radius list (the anchor must be present and self-consistent).
    MissingAnchor(String),
    /// A radius names an element symbol that is not in the periodic table.
    UnknownElement(String),
    /// A duplicate `(symbol, oxidation_state, coordination, spin)` key.
    Duplicate(String),
}

impl fmt::Display for RadiiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RadiiError::Parse(m) => write!(f, "ionic-radii parse error: {m}"),
            RadiiError::Io(m) => write!(f, "ionic-radii read error: {m}"),
            RadiiError::BadValue { ion, detail } => {
                write!(f, "radius of ion '{ion}' could not be read: {detail}")
            }
            RadiiError::MissingProvenance(id) => {
                write!(f, "ionic radius '{id}' carries no citation")
            }
            RadiiError::BadConvention(m) => write!(f, "ionic-radii convention error: {m}"),
            RadiiError::MissingAnchor(m) => write!(f, "ionic-radii anchor missing: {m}"),
            RadiiError::UnknownElement(s) => write!(f, "ionic radius names unknown element '{s}'"),
            RadiiError::Duplicate(id) => write!(f, "duplicate ionic radius '{id}'"),
        }
    }
}

impl std::error::Error for RadiiError {}

impl IonicRadii {
    /// Parse the floor from TOML text, validating the matched-pair invariant (the convention and its anchor)
    /// and that every entry is cited.
    pub fn from_toml_str(s: &str) -> Result<Self, RadiiError> {
        let file: RadiiFile = toml::from_str(s).map_err(|e| RadiiError::Parse(e.to_string()))?;
        if file.source.trim().is_empty() {
            return Err(RadiiError::MissingProvenance(
                "the convention header".to_string(),
            ));
        }
        // Only the crystal convention is supported, and it must anchor on r(O2-) = 1.26 (the matched-pair
        // invariant): the radius-ratio window is calibrated to this convention.
        if file.convention.trim() != "crystal" {
            return Err(RadiiError::BadConvention(format!(
                "unsupported convention '{}', only 'crystal' (the Pauling-window-matched set) is supported",
                file.convention.trim()
            )));
        }
        let anchor_radius =
            Fixed::from_decimal_str(file.anchor_radius.trim()).map_err(|detail| {
                RadiiError::BadValue {
                    ion: "anchor".to_string(),
                    detail,
                }
            })?;
        let expected_anchor = Fixed::from_ratio(126, 100); // r(O2-) = 1.26 for crystal radii
        if anchor_radius != expected_anchor {
            return Err(RadiiError::BadConvention(format!(
                "crystal convention must anchor on r(O2-) = 1.26, found {anchor_radius:?}"
            )));
        }

        let mut radii = Vec::with_capacity(file.radii.len());
        let mut seen = std::collections::BTreeSet::new();
        for def in file.radii {
            let ion = format!(
                "{}{:+} [{}]{}",
                def.symbol,
                def.oxidation_state,
                def.coordination,
                def.spin
                    .as_deref()
                    .map(|s| format!(" {s}-spin"))
                    .unwrap_or_default()
            );
            if def.source.trim().is_empty() {
                return Err(RadiiError::MissingProvenance(ion));
            }
            let key = (
                def.symbol.clone(),
                def.oxidation_state,
                def.coordination,
                def.spin.clone(),
            );
            if !seen.insert(key) {
                return Err(RadiiError::Duplicate(ion));
            }
            let crystal_radius = Fixed::from_decimal_str(def.crystal_radius.trim())
                .map_err(|detail| RadiiError::BadValue { ion, detail })?;
            radii.push(IonicRadius {
                symbol: def.symbol,
                oxidation_state: def.oxidation_state,
                coordination: def.coordination,
                spin: def.spin,
                crystal_radius,
                source: def.source,
            });
        }

        let table = IonicRadii {
            convention: file.convention.trim().to_string(),
            anchor_symbol: file.anchor_symbol.trim().to_string(),
            anchor_oxidation_state: file.anchor_oxidation_state,
            anchor_coordination: file.anchor_coordination,
            anchor_radius,
            radii,
        };
        // The anchor ion must be present in the list and self-consistent with the header anchor_radius.
        match table.radius(
            &table.anchor_symbol,
            table.anchor_oxidation_state,
            table.anchor_coordination,
        ) {
            Some(a) if a.crystal_radius == anchor_radius => {}
            Some(a) => {
                return Err(RadiiError::BadConvention(format!(
                    "the anchor ion's radius {:?} disagrees with the header anchor_radius {anchor_radius:?}",
                    a.crystal_radius
                )))
            }
            None => {
                return Err(RadiiError::MissingAnchor(format!(
                    "{}{:+} [{}] is not in the radius list",
                    table.anchor_symbol, table.anchor_oxidation_state, table.anchor_coordination
                )))
            }
        }
        Ok(table)
    }

    /// Load the floor from a file path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RadiiError> {
        let text = std::fs::read_to_string(path).map_err(|e| RadiiError::Io(e.to_string()))?;
        Self::from_toml_str(&text)
    }

    /// The embedded floor, built from the crate's data so a caller needs no filesystem path.
    pub fn standard() -> Result<Self, RadiiError> {
        Self::from_toml_str(include_str!("../data/ionic_radii.toml"))
    }

    /// The radius convention (`crystal`), stored so a consumer's ratio window is a matched pair, never silent.
    pub fn convention(&self) -> &str {
        &self.convention
    }

    /// The convention anchor radius (`r(O2-) = 1.26` for the crystal set).
    pub fn anchor_radius(&self) -> Fixed {
        self.anchor_radius
    }

    /// A crystal radius by ion, ignoring spin (the first matching entry). For a spin-split ion, prefer
    /// [`Self::radius_spin`].
    pub fn radius(
        &self,
        symbol: &str,
        oxidation_state: i8,
        coordination: u8,
    ) -> Option<&IonicRadius> {
        self.radii.iter().find(|r| {
            r.symbol == symbol
                && r.oxidation_state == oxidation_state
                && r.coordination == coordination
        })
    }

    /// A crystal radius by ion and spin state.
    pub fn radius_spin(
        &self,
        symbol: &str,
        oxidation_state: i8,
        coordination: u8,
        spin: Option<&str>,
    ) -> Option<&IonicRadius> {
        self.radii.iter().find(|r| {
            r.symbol == symbol
                && r.oxidation_state == oxidation_state
                && r.coordination == coordination
                && r.spin.as_deref() == spin
        })
    }

    /// The radii, in file order.
    pub fn radii(&self) -> &[IonicRadius] {
        &self.radii
    }

    /// Validate that every ion's symbol resolves to a periodic-table element (the floor is self-consistent
    /// against the periodic table it sits beside).
    pub fn validate_against(&self, table: &PeriodicTable) -> Result<(), RadiiError> {
        for r in &self.radii {
            if table.element(&r.symbol).is_none() {
                return Err(RadiiError::UnknownElement(r.symbol.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RadiiFile {
    #[serde(default)]
    convention: String,
    #[serde(default)]
    anchor_symbol: String,
    #[serde(default)]
    anchor_oxidation_state: i8,
    #[serde(default)]
    anchor_coordination: u8,
    #[serde(default)]
    anchor_radius: String,
    #[serde(default)]
    source: String,
    #[serde(default, rename = "radius")]
    radii: Vec<RadiusDef>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RadiusDef {
    symbol: String,
    oxidation_state: i8,
    coordination: u8,
    #[serde(default)]
    spin: Option<String>,
    crystal_radius: String,
    source: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radii() -> IonicRadii {
        IonicRadii::standard().expect("the standard ionic-radii floor loads")
    }

    /// The Pauling coordination predicted by a radius ratio `r_cation / r_anion`: pure geometry (the ratio at
    /// which a cation fits the interstitial hole of close-packed anions), convention-free. The `crystal` radii
    /// are the convention whose ratios reproduce these windows.
    fn predicted_coordination(ratio: Fixed) -> u8 {
        let triangular = Fixed::from_ratio(155, 1000);
        let tetrahedral = Fixed::from_ratio(225, 1000);
        let octahedral = Fixed::from_ratio(414, 1000);
        let cubic = Fixed::from_ratio(732, 1000);
        if ratio < triangular {
            2
        } else if ratio < tetrahedral {
            3
        } else if ratio < octahedral {
            4
        } else if ratio < cubic {
            6
        } else if ratio < Fixed::ONE {
            8
        } else {
            12
        }
    }

    #[test]
    fn the_floor_loads_with_the_crystal_convention_and_anchor() {
        let r = radii();
        assert_eq!(r.convention(), "crystal");
        assert_eq!(
            r.anchor_radius(),
            Fixed::from_ratio(126, 100),
            "the crystal anchor is r(O2-) = 1.26"
        );
        // The anchor ion is present and self-consistent (checked at load; assert it is readable).
        assert_eq!(
            r.radius("O", -2, 6).unwrap().crystal_radius,
            Fixed::from_ratio(126, 100)
        );
    }

    #[test]
    fn the_crystal_radii_reproduce_known_coordinations() {
        // THE COORDINATION-REPRODUCTION GUARD (the gate's mandate): a sample of ions must land their known
        // coordination through Pauling's geometric window. A wrong-convention or transcribed radius fails this
        // rather than shipping an authoritative-looking mistake. The clear cases only; borderline ions (Na, K)
        // are where the [E] ratio escalates to the disposer's energy, so they are not asserted here.
        let r = radii();
        let anchor = r.anchor_radius();
        let ratio = |ion: &IonicRadius| ion.crystal_radius / anchor;

        let si = r.radius("Si", 4, 4).expect("Si4+ [4]");
        assert_eq!(
            predicted_coordination(ratio(si)),
            4,
            "Si4+ is tetrahedral (0.40/1.26 = 0.317)"
        );
        let al = r.radius("Al", 3, 6).expect("Al3+ [6]");
        assert_eq!(
            predicted_coordination(ratio(al)),
            6,
            "Al3+ [6] is octahedral (0.675/1.26 = 0.536)"
        );
        let mg = r.radius("Mg", 2, 6).expect("Mg2+ [6]");
        assert_eq!(
            predicted_coordination(ratio(mg)),
            6,
            "Mg2+ [6] is octahedral (0.86/1.26 = 0.683)"
        );
    }

    #[test]
    fn every_radius_resolves_against_the_periodic_table() {
        let r = radii();
        let table = PeriodicTable::standard().expect("the periodic table loads");
        r.validate_against(&table)
            .expect("every ion symbol is a periodic element");
    }

    #[test]
    fn a_radius_without_its_citation_fails_to_load() {
        let no_src = r#"
convention = "crystal"
anchor_symbol = "O"
anchor_oxidation_state = -2
anchor_coordination = 6
anchor_radius = "1.26"
source = "test"
[[radius]]
symbol = "O"
oxidation_state = -2
coordination = 6
crystal_radius = "1.26"
source = "anchor"
[[radius]]
symbol = "Si"
oxidation_state = 4
coordination = 4
crystal_radius = "0.40"
source = ""
"#;
        assert!(matches!(
            IonicRadii::from_toml_str(no_src).unwrap_err(),
            RadiiError::MissingProvenance(_)
        ));
    }

    #[test]
    fn the_effective_convention_is_rejected_as_a_mismatched_pair() {
        // The matched-pair invariant: an effective-radii anchor (1.40) with the crystal window would mispredict
        // coordinations, so it fails loud rather than loading a silent mismatch.
        let effective = r#"
convention = "effective"
anchor_symbol = "O"
anchor_oxidation_state = -2
anchor_coordination = 6
anchor_radius = "1.40"
source = "test"
"#;
        assert!(matches!(
            IonicRadii::from_toml_str(effective).unwrap_err(),
            RadiiError::BadConvention(_)
        ));
    }
}
