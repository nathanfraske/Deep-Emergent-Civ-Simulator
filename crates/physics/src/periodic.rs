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

//! The basal periodic-table floor: the per-element standard atomic weights (`data/periodic_table.toml`)
//! and the molar-mass-from-formula derivation over them. This is the primitive that RETIRES per-substance
//! authored molar masses: a molecule's molar mass is COMPUTED from its atomic composition times the table
//! ([`PeriodicTable::molar_mass`]), never authored as its own number.
//!
//! The three-way test (AGENTIC_ADDENDUM section 9) places a standard atomic weight as PER-WORLD DATA, not a
//! universal fundamental constant: it is the isotope-abundance-weighted average of an element's isotope
//! masses, and that abundance mix is a property of a particular world's material history. The embedded
//! table is Mirror-calibrated to Earth's terrestrial isotope mix (CIAAW/IUPAC Standard Atomic Weights
//! 2021). An alien world with a different isotope mix is a DATA ROW (a different weight), not a rewrite: a
//! world may load its own table or override rows. The molar mass constant that carries the g/mol unit
//! (`M_u = 1 g/mol` to within its CODATA uncertainty) derives from `N_A` and the atomic mass constant, so
//! it is not authored here; the derivation's numeric output in g/mol equals the abundance-weighted sum of
//! relative atomic masses.
//!
//! Every value is fixed-point ([`Fixed`]), parsed from a decimal string by integer arithmetic, so no
//! floating point reaches canonical state, the same discipline as the physics registry. The mechanism (the
//! loader and the molar-mass kernel) is fixed Rust; the element membership is data and grows with the world
//! (Principle 11): a heavier element, or an alien isotope mix, is a new or edited row in the data file. No
//! consumer is wired to this table yet; it is a pure addition.

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// One element's floor row: its symbol, name, atomic number, standard atomic weight (both as the parsed
/// [`Fixed`] the derivation reads and as the raw decimal string retained verbatim from the data), the
/// terrestrial interval bounds for an interval element, whether the stored weight is a true abundance-
/// averaged standard atomic weight (as against a single-isotope reference mass for a radioactive-only
/// element), and the citation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The chemical symbol, for example `Fe`.
    pub symbol: String,
    /// The element name, for example `iron`.
    pub name: String,
    /// The atomic number Z (the proton count).
    pub z: u8,
    /// The standard (conventional) atomic weight as fixed-point, the value the molar-mass kernel reads.
    pub standard_atomic_weight: Fixed,
    /// The raw decimal string of the standard atomic weight, retained verbatim (a bound below the Q32.32
    /// epsilon would lose magnitude in the `Fixed`; the decimal keeps it, and it is the provenance record).
    pub weight_decimal: String,
    /// The terrestrial interval `[lo, hi]` (as raw decimals) for an interval element whose isotopic
    /// composition varies between natural materials; `None` for a single-composition element.
    pub interval: Option<(String, String)>,
    /// Whether the stored weight is a true abundance-averaged standard atomic weight. `false` for a
    /// radioactive-only element with no characteristic terrestrial composition, whose stored value is a
    /// single-isotope reference mass (CIAAW assigns it no standard atomic weight).
    pub has_standard_weight: bool,
    /// The citation and provenance for this row.
    pub provenance: String,
}

/// What can go wrong loading or reading the periodic table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeriodicError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// The data file could not be read.
    Io(String),
    /// A symbol appears twice.
    DuplicateSymbol(String),
    /// An atomic number appears twice.
    DuplicateZ(u8),
    /// A decimal value could not be parsed to fixed-point, or an interval is half-declared.
    BadValue {
        /// The element the value belongs to.
        symbol: String,
        /// What went wrong.
        detail: String,
    },
    /// An element carries no citation (every row must be real-with-source).
    MissingProvenance(String),
    /// A formula names a symbol that is not in the table.
    UnknownElement(String),
    /// A molar-mass accumulation overflowed fixed-point.
    Overflow(String),
}

impl fmt::Display for PeriodicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeriodicError::Parse(m) => write!(f, "periodic-table parse error: {m}"),
            PeriodicError::Io(m) => write!(f, "periodic-table read error: {m}"),
            PeriodicError::DuplicateSymbol(s) => write!(f, "duplicate element symbol '{s}'"),
            PeriodicError::DuplicateZ(z) => write!(f, "duplicate atomic number {z}"),
            PeriodicError::BadValue { symbol, detail } => {
                write!(f, "value in element '{symbol}' could not be read: {detail}")
            }
            PeriodicError::MissingProvenance(s) => {
                write!(
                    f,
                    "element '{s}' must declare a citation (real-with-source)"
                )
            }
            PeriodicError::UnknownElement(s) => {
                write!(
                    f,
                    "formula references unknown element '{s}' (not in the periodic table)"
                )
            }
            PeriodicError::Overflow(m) => write!(f, "molar-mass accumulation overflowed: {m}"),
        }
    }
}

impl std::error::Error for PeriodicError {}

/// The loaded periodic table: the elements keyed by symbol in a sorted map so any walk is in a fixed
/// canonical order (the determinism discipline), with a Z-to-symbol index for atomic-number lookup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeriodicTable {
    elements: BTreeMap<String, Element>,
    by_z: BTreeMap<u8, String>,
}

impl PeriodicTable {
    /// Parse and validate a table from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, PeriodicError> {
        let file: TableFile = toml::from_str(s).map_err(|e| PeriodicError::Parse(e.to_string()))?;
        let mut table = PeriodicTable::default();
        for e in file.element {
            let element = e.into_element()?;
            if table.elements.contains_key(&element.symbol) {
                return Err(PeriodicError::DuplicateSymbol(element.symbol));
            }
            if table.by_z.contains_key(&element.z) {
                return Err(PeriodicError::DuplicateZ(element.z));
            }
            table.by_z.insert(element.z, element.symbol.clone());
            table.elements.insert(element.symbol.clone(), element);
        }
        Ok(table)
    }

    /// Load and validate a table from a file path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PeriodicError> {
        let text = std::fs::read_to_string(path).map_err(|e| PeriodicError::Io(e.to_string()))?;
        Self::from_toml_str(&text)
    }

    /// The embedded standard table (CIAAW/IUPAC Standard Atomic Weights 2021, Mirror-calibrated to Earth),
    /// built from the crate's embedded data so a caller needs no filesystem path.
    pub fn standard() -> Result<Self, PeriodicError> {
        Self::from_toml_str(include_str!("../data/periodic_table.toml"))
    }

    /// An element by symbol.
    pub fn element(&self, symbol: &str) -> Option<&Element> {
        self.elements.get(symbol)
    }

    /// An element by atomic number Z.
    pub fn element_by_z(&self, z: u8) -> Option<&Element> {
        self.by_z.get(&z).and_then(|s| self.elements.get(s))
    }

    /// The elements, in sorted symbol order.
    pub fn elements(&self) -> impl Iterator<Item = &Element> + '_ {
        self.elements.values()
    }

    /// The number of elements.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// The molar-mass-from-formula derivation: given a molecule's atomic composition as a `{symbol: count}`
    /// map (water is `{H: 2, O: 1}`), return its molar mass `M = sum(count * A_r)` in g/mol, the value that
    /// RETIRES a per-substance authored molar mass. The sum walks the formula in sorted symbol order for a
    /// deterministic accumulation; fixed-point addition is exact at these magnitudes, so the result is
    /// worker-invariant. The numeric result in g/mol equals the abundance-weighted relative-atomic-mass sum
    /// because the molar mass constant `M_u = 1 g/mol` to within its CODATA uncertainty (`M_u` derives from
    /// `N_A` and the atomic mass constant, not authored here). An unknown symbol fails loud; a count of zero
    /// contributes nothing. Deriving through a radioactive-only element (one with `has_standard_weight ==
    /// false`) uses its single-isotope reference mass, correct as a mass but not a per-world abundance
    /// average, so a consumer that needs a true standard weight should check the flag.
    pub fn molar_mass(&self, formula: &BTreeMap<String, u32>) -> Result<Fixed, PeriodicError> {
        let mut total = Fixed::ZERO;
        for (symbol, count) in formula {
            let element = self
                .element(symbol)
                .ok_or_else(|| PeriodicError::UnknownElement(symbol.clone()))?;
            let contribution = element
                .standard_atomic_weight
                .checked_mul(Fixed::from_int(*count as i32))
                .ok_or_else(|| {
                    PeriodicError::Overflow(format!(
                        "{count} * A_r({symbol}) exceeds the fixed-point range"
                    ))
                })?;
            total = total.checked_add(contribution).ok_or_else(|| {
                PeriodicError::Overflow(format!(
                    "the running molar-mass sum exceeded the fixed-point range at '{symbol}'"
                ))
            })?;
        }
        Ok(total)
    }

    /// A convenience form of [`PeriodicTable::molar_mass`] over a slice of `(symbol, count)` pairs, so a
    /// caller need not build a map for a literal formula (`&[("H", 2), ("O", 1)]`). Duplicate symbols in
    /// the slice add, so an unnormalised formula is handled.
    pub fn molar_mass_of(&self, formula: &[(&str, u32)]) -> Result<Fixed, PeriodicError> {
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        for (symbol, count) in formula {
            *map.entry((*symbol).to_string()).or_insert(0) += count;
        }
        self.molar_mass(&map)
    }
}

// The TOML-facing schema. Values are decimal strings parsed to Fixed by integer arithmetic, so no floating
// point reaches canonical state. Kept separate from the typed forms above so Fixed never needs serde.

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct TableFile {
    #[serde(default)]
    element: Vec<ElementDef>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ElementDef {
    symbol: String,
    #[serde(default)]
    name: String,
    z: u8,
    standard_atomic_weight: String,
    /// The terrestrial interval lower bound, empty for a single-composition element.
    #[serde(default)]
    interval_lo: String,
    /// The terrestrial interval upper bound, empty for a single-composition element.
    #[serde(default)]
    interval_hi: String,
    /// Whether the weight is a true abundance-averaged standard atomic weight; defaults true, set false
    /// for a radioactive-only element whose value is a single-isotope reference mass.
    #[serde(default = "default_true")]
    has_standard_weight: bool,
    /// The citation (every element is real-with-source).
    #[serde(default)]
    real: String,
}

fn default_true() -> bool {
    true
}

impl ElementDef {
    fn into_element(self) -> Result<Element, PeriodicError> {
        let standard_atomic_weight = Fixed::from_decimal_str(&self.standard_atomic_weight)
            .map_err(|detail| PeriodicError::BadValue {
                symbol: self.symbol.clone(),
                detail,
            })?;
        let lo = self.interval_lo.trim();
        let hi = self.interval_hi.trim();
        let interval = match (lo.is_empty(), hi.is_empty()) {
            (true, true) => None,
            (false, false) => {
                // Validate both interval bounds parse, so a malformed interval fails loud at load.
                Fixed::from_decimal_str(lo).map_err(|detail| PeriodicError::BadValue {
                    symbol: self.symbol.clone(),
                    detail,
                })?;
                Fixed::from_decimal_str(hi).map_err(|detail| PeriodicError::BadValue {
                    symbol: self.symbol.clone(),
                    detail,
                })?;
                Some((lo.to_string(), hi.to_string()))
            }
            _ => {
                return Err(PeriodicError::BadValue {
                    symbol: self.symbol.clone(),
                    detail: "an interval element must declare both interval_lo and interval_hi, or neither"
                        .to_string(),
                });
            }
        };
        if self.real.trim().is_empty() {
            return Err(PeriodicError::MissingProvenance(self.symbol.clone()));
        }
        Ok(Element {
            symbol: self.symbol,
            name: self.name,
            z: self.z,
            standard_atomic_weight,
            weight_decimal: self.standard_atomic_weight.trim().to_string(),
            interval,
            has_standard_weight: self.has_standard_weight,
            provenance: self.real.trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the embedded standard periodic table loads")
    }

    // A test-only float readout to compare a derived molar mass against its hand-computed decimal within a
    // tolerance far below any chemical relevance. `to_f64_lossy` is used DELIBERATELY and ONLY here, in a
    // test, exactly as fundamentals.rs uses f64 to validate a recorded relation: no float touches the
    // crate's canonical integer path.
    fn close(a: Fixed, b: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < 1e-3
    }

    #[test]
    fn the_table_covers_hydrogen_through_uranium() {
        let t = table();
        assert_eq!(t.len(), 92, "the floor covers Z=1 (H) through Z=92 (U)");
        for z in 1..=92u8 {
            assert!(
                t.element_by_z(z).is_some(),
                "the periodic-table floor is missing atomic number {z}"
            );
        }
    }

    #[test]
    fn symbol_and_z_lookups_agree() {
        let t = table();
        let fe = t.element("Fe").expect("iron is in the table");
        assert_eq!(fe.z, 26);
        assert_eq!(t.element_by_z(26), Some(fe));
        assert_eq!(t.element("H").map(|e| e.z), Some(1));
        assert_eq!(t.element("U").map(|e| e.z), Some(92));
        assert!(t.element("Xx").is_none());
    }

    #[test]
    fn water_molar_mass_derives_from_the_table() {
        let t = table();
        // 2 * A_r(H) + A_r(O) = 2*1.008 + 15.999 = 18.015 g/mol.
        let m = t
            .molar_mass_of(&[("H", 2), ("O", 1)])
            .expect("water derives");
        assert!(
            close(m, 18.015),
            "water molar mass {} != ~18.015",
            m.to_f64_lossy()
        );
    }

    #[test]
    fn carbon_dioxide_and_glucose_derive() {
        let t = table();
        // CO2 = 12.011 + 2*15.999 = 44.009 g/mol.
        let co2 = t.molar_mass_of(&[("C", 1), ("O", 2)]).expect("CO2 derives");
        assert!(close(co2, 44.009), "CO2 molar mass {}", co2.to_f64_lossy());
        // Glucose C6H12O6 = 6*12.011 + 12*1.008 + 6*15.999 = 180.156 g/mol.
        let glucose = t
            .molar_mass_of(&[("C", 6), ("H", 12), ("O", 6)])
            .expect("glucose derives");
        assert!(
            close(glucose, 180.156),
            "glucose molar mass {} != ~180.156",
            glucose.to_f64_lossy()
        );
    }

    #[test]
    fn an_unknown_symbol_fails_loud() {
        let t = table();
        let err = t.molar_mass_of(&[("Zz", 1)]).unwrap_err();
        assert_eq!(err, PeriodicError::UnknownElement("Zz".to_string()));
    }

    #[test]
    fn the_fourteen_interval_elements_carry_their_interval() {
        let t = table();
        let interval_elements = [
            "H", "Li", "B", "C", "N", "O", "Mg", "Si", "S", "Cl", "Ar", "Br", "Tl", "Pb",
        ];
        for sym in interval_elements {
            let e = t
                .element(sym)
                .unwrap_or_else(|| panic!("{sym} is in the table"));
            assert!(
                e.interval.is_some(),
                "{sym} is an interval element and must carry its terrestrial interval"
            );
        }
        // A single-composition element carries no interval.
        assert!(t.element("Fe").unwrap().interval.is_none());
    }

    #[test]
    fn the_radioactive_only_elements_are_flagged_not_abundance_averages() {
        let t = table();
        let radioactive_only = ["Tc", "Pm", "Po", "At", "Rn", "Fr", "Ra", "Ac"];
        for sym in radioactive_only {
            let e = t
                .element(sym)
                .unwrap_or_else(|| panic!("{sym} is in the table"));
            assert!(
                !e.has_standard_weight,
                "{sym} has no standard atomic weight; its value is a reference-isotope mass"
            );
        }
        // A stable element's weight IS a true abundance-averaged standard atomic weight.
        assert!(t.element("Fe").unwrap().has_standard_weight);
        assert!(t.element("U").unwrap().has_standard_weight);
    }

    #[test]
    fn every_element_carries_a_citation() {
        let t = table();
        for e in t.elements() {
            assert!(
                !e.provenance.trim().is_empty(),
                "{} must carry a citation",
                e.symbol
            );
        }
    }

    #[test]
    fn an_empty_formula_has_zero_mass_and_a_count_of_zero_contributes_nothing() {
        let t = table();
        assert_eq!(t.molar_mass_of(&[]).unwrap(), Fixed::ZERO);
        let only_water = t.molar_mass_of(&[("H", 2), ("O", 1)]).unwrap();
        let padded = t.molar_mass_of(&[("H", 2), ("O", 1), ("Fe", 0)]).unwrap();
        assert_eq!(only_water, padded, "a zero count must not change the mass");
    }

    #[test]
    fn a_duplicate_symbol_in_a_literal_formula_adds() {
        let t = table();
        // Acetic acid written CH3COOH: carbons and oxygens repeated in the literal, they must add.
        let split = t
            .molar_mass_of(&[("C", 1), ("H", 3), ("C", 1), ("O", 1), ("O", 1), ("H", 1)])
            .unwrap();
        let combined = t.molar_mass_of(&[("C", 2), ("H", 4), ("O", 2)]).unwrap();
        assert_eq!(split, combined);
    }

    #[test]
    fn a_duplicate_symbol_row_fails_to_load() {
        let dup = r#"
[[element]]
symbol = "H"
z = 1
standard_atomic_weight = "1.008"
real = "test"

[[element]]
symbol = "H"
z = 2
standard_atomic_weight = "2.000"
real = "test"
"#;
        assert_eq!(
            PeriodicTable::from_toml_str(dup).unwrap_err(),
            PeriodicError::DuplicateSymbol("H".to_string())
        );
    }

    #[test]
    fn a_missing_citation_fails_to_load() {
        let no_cite = r#"
[[element]]
symbol = "H"
z = 1
standard_atomic_weight = "1.008"
"#;
        assert_eq!(
            PeriodicTable::from_toml_str(no_cite).unwrap_err(),
            PeriodicError::MissingProvenance("H".to_string())
        );
    }

    #[test]
    fn a_half_declared_interval_fails_to_load() {
        let half = r#"
[[element]]
symbol = "H"
z = 1
standard_atomic_weight = "1.008"
interval_lo = "1.00784"
real = "test"
"#;
        assert!(matches!(
            PeriodicTable::from_toml_str(half).unwrap_err(),
            PeriodicError::BadValue { .. }
        ));
    }
}
