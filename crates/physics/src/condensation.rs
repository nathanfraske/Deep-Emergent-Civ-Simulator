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

//! The Lodders (2003) condensation-temperature column (`crates/physics/data/condensation_lodders2003.toml`), the
//! pre-registered battery for the disk-condensation arc: per element the equilibrium 50% condensation temperature
//! for a solar-system composition gas at `1e-4` bar (Lodders 2003, ApJ 591, 1220, Table 8). The 50% temperature,
//! the point at which half of an element has condensed out of the gas as it cools, is the ground truth a
//! Gibbs-minimization over the JANAF standard potentials (see `janaf.rs`) must reproduce, so this column is a
//! calibration target rather than a run input.
//!
//! Cited [M], transcribed from the primary reprint PDF (the Pyykkö-figure protocol: transcribe the table and
//! fingerprint-check spot rows). The parse routes every temperature through the exact `BigRat` conduit to `Fixed`
//! (no floating point reaches canonical state). BLOCK KIND `[[condensation]]`, the cited-data-column idiom, NOT the
//! reserved floor `[[element]]` kind: an immutable transcription out of the floor's real/fantasy authorship axis.
//! GRADE is single (one table, one composition, one pressure). The phase and host strings are descriptive
//! annotation; the load-bearing values are the two temperatures. HONEST LIMITS: this is the solar-system
//! composition of Table 8, not the solar-photosphere variant of Table 9 (a few K apart); a trace element that only
//! dissolves into a host has no first-condensate temperature (`t_first_k` absent), and H and He never reach 50%
//! condensation in range (`t50_k` absent, water ice's onset carried as the `t_first_k = 182 K` of H and O). No
//! consumer is wired in any pinned run path (byte-neutral).

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the condensation column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CondensationError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A temperature string could not be parsed to fixed-point.
    BadValue(String),
    /// An element symbol appears twice.
    Duplicate(String),
}

impl fmt::Display for CondensationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CondensationError::Parse(m) => write!(f, "condensation parse error: {m}"),
            CondensationError::BadValue(m) => write!(f, "condensation value error: {m}"),
            CondensationError::Duplicate(m) => write!(f, "duplicate condensation element: {m}"),
        }
    }
}

impl std::error::Error for CondensationError {}

/// One element's condensation entry from Table 8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CondensationRow {
    /// The element symbol.
    pub element: String,
    /// The atomic number.
    pub z: u32,
    /// The temperature (K) of the element's first condensate (col 2 `TC`); `None` for a trace element that only
    /// dissolves into a host phase, or where the table lists no independent onset.
    pub t_first_k: Option<Fixed>,
    /// The first-condensate phase, or the `{dissolving species}` for a trace element (col 3), descriptive.
    pub initial_phase: String,
    /// The 50% condensation temperature (K) (col 4 `50% TC`), the load-bearing calibration value; `None` for H and
    /// He, which never reach 50% condensation in range.
    pub t50_k: Option<Fixed>,
    /// The major host phase(s) (col 5), descriptive.
    pub host: String,
}

/// The loaded condensation column, keyed by element symbol, plus the table's global pressure and composition.
#[derive(Debug, Clone, Default)]
pub struct CondensationTable {
    rows: BTreeMap<String, CondensationRow>,
    pressure_bar: String,
    composition: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawFile {
    #[serde(default)]
    pressure_bar: String,
    #[serde(default)]
    composition: String,
    // `[[condensation]]`, the cited-data-column block kind, NOT the reserved floor `[[element]]` kind.
    #[serde(default)]
    condensation: Vec<RawRow>,
}

#[derive(Debug, Default, Deserialize)]
struct RawRow {
    element: String,
    z: u32,
    #[serde(default)]
    t_first_k: Option<String>,
    #[serde(default)]
    initial_phase: String,
    #[serde(default)]
    t50_k: Option<String>,
    #[serde(default)]
    host: String,
}

/// Parse one decimal temperature string to `Fixed` through the exact `BigRat` path.
fn fixed_from_decimal(s: &str) -> Result<Fixed, CondensationError> {
    let br = BigRat::from_decimal_str(s).map_err(CondensationError::BadValue)?;
    let bits = br
        .round_to_scale(Fixed::FRAC_BITS)
        .ok_or_else(|| CondensationError::BadValue(format!("{s} out of range")))?;
    Fixed::from_bits_i128(bits)
        .ok_or_else(|| CondensationError::BadValue(format!("{s} out of range")))
}

impl CondensationTable {
    /// Parse and validate the column from a TOML string. Every row keys off a unique element symbol; the two
    /// temperatures are both optional (He carries neither, its onset being an unbounded `<3 K` and its 50% never
    /// reached), so an absent temperature is a valid state rather than a defect. A malformed temperature string
    /// fails closed.
    pub fn from_toml_str(s: &str) -> Result<Self, CondensationError> {
        let file: RawFile =
            toml::from_str(s).map_err(|e| CondensationError::Parse(e.to_string()))?;
        let mut rows = BTreeMap::new();
        for raw in file.condensation {
            let t_first_k = match &raw.t_first_k {
                Some(v) => Some(fixed_from_decimal(v)?),
                None => None,
            };
            let t50_k = match &raw.t50_k {
                Some(v) => Some(fixed_from_decimal(v)?),
                None => None,
            };
            let row = CondensationRow {
                element: raw.element.clone(),
                z: raw.z,
                t_first_k,
                initial_phase: raw.initial_phase,
                t50_k,
                host: raw.host,
            };
            if rows.insert(raw.element.clone(), row).is_some() {
                return Err(CondensationError::Duplicate(raw.element));
            }
        }
        Ok(CondensationTable {
            rows,
            pressure_bar: file.pressure_bar,
            composition: file.composition,
        })
    }

    /// Load the standard Lodders 2003 Table 8 column from the checked-in data file.
    pub fn standard() -> Result<Self, CondensationError> {
        Self::from_toml_str(include_str!("../data/condensation_lodders2003.toml"))
    }

    /// The condensation entry for an element symbol, or `None` if the table does not cover it.
    pub fn element(&self, symbol: &str) -> Option<&CondensationRow> {
        self.rows.get(symbol)
    }

    /// The 50% condensation temperature (K) for an element, if the table gives one.
    pub fn t50_k(&self, symbol: &str) -> Option<Fixed> {
        self.rows.get(symbol).and_then(|r| r.t50_k)
    }

    /// The total pressure the table is computed at (a string, e.g. `1e-4` bar), for provenance.
    pub fn pressure_bar(&self) -> &str {
        &self.pressure_bar
    }

    /// The gas composition the table is computed for (e.g. `solar-system`).
    pub fn composition(&self) -> &str {
        &self.composition
    }

    /// The element symbols in the table, sorted.
    pub fn elements(&self) -> impl Iterator<Item = &str> + '_ {
        self.rows.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> CondensationTable {
        CondensationTable::standard().expect("the Lodders 2003 condensation table loads")
    }

    #[test]
    fn the_table_loads_all_83_elements_at_the_stated_conditions() {
        let t = table();
        assert_eq!(t.elements().count(), 83, "83 elements in Table 8");
        assert_eq!(t.pressure_bar(), "1e-4", "1e-4 bar total pressure");
        assert_eq!(t.composition(), "solar-system", "solar-system composition");
    }

    #[test]
    fn the_fingerprint_rows_reproduce_lodders_table_8() {
        // The owner's pre-registered transcription fingerprints (within ~1 K): the 50% condensation temperatures of
        // the load-bearing rock formers, plus the water-ice onset. If the transcription drifted, this fails.
        let t = table();
        let cases = [
            ("Al", 1653.0),
            ("Ca", 1517.0),
            ("Ti", 1582.0),
            ("Fe", 1334.0),
            ("Mg", 1336.0),
            ("Si", 1310.0),
            ("S", 664.0),
        ];
        for (sym, want) in cases {
            let got = t
                .t50_k(sym)
                .unwrap_or_else(|| panic!("{sym} has a 50% condensation temperature"));
            assert!(
                (got.to_f64_lossy() - want).abs() < 1.0,
                "{sym} 50% Tc is {want} K, got {}",
                got.to_f64_lossy()
            );
        }
        // Water ice's condensation onset is the first-condensate temperature of H (and O), 182 K, the water-ice line.
        let h = t.element("H").unwrap();
        let onset = h.t_first_k.expect("water ice has an onset temperature");
        assert!(
            (onset.to_f64_lossy() - 182.0).abs() < 1.0,
            "water ice onset is 182 K, got {}",
            onset.to_f64_lossy()
        );
        assert!(
            h.initial_phase.contains("H2O"),
            "H's first condensate is water ice, got {}",
            h.initial_phase
        );
        // H never reaches 50% condensation, so its 50% temperature is absent (not a fabricated number).
        assert_eq!(
            h.t50_k, None,
            "hydrogen has no 50% condensation temperature"
        );
    }

    #[test]
    fn the_refractory_to_volatile_order_is_preserved() {
        // A coarse sanity check on the transcription: the refractory sequence Al > Ti > Ca > Fe ~ Mg > Si > S holds,
        // and the volatiles sit far below (a scrambled column would break this monotonic spine).
        let t = table();
        let g = |s: &str| t.t50_k(s).unwrap().to_f64_lossy();
        assert!(g("Al") > g("Ti"), "Al condenses before Ti");
        assert!(g("Ti") > g("Ca"), "Ti condenses before Ca");
        assert!(g("Ca") > g("Fe"), "Ca condenses before Fe");
        assert!(g("Fe") > g("Si"), "Fe condenses before Si");
        assert!(g("Si") > g("S"), "Si condenses before S");
        assert!(
            g("Zr") > g("Al"),
            "Zr (ultrarefractory) condenses before Al"
        );
        assert!(g("Na") < g("Fe"), "Na (volatile) condenses well after Fe");
    }

    #[test]
    fn a_row_with_no_temperature_is_valid() {
        // Helium legitimately carries neither temperature (its onset is an unbounded <3 K, its 50% never reached),
        // so it loads as a row with both temperatures absent rather than failing.
        let t = table();
        let he = t.element("He").expect("He is a row");
        assert_eq!(he.t_first_k, None);
        assert_eq!(he.t50_k, None);
    }

    #[test]
    fn a_bad_temperature_fails_closed() {
        let bad = r#"
[[condensation]]
element = "X"
z = 200
t50_k = "not-a-number"
host = "nowhere"
"#;
        assert!(matches!(
            CondensationTable::from_toml_str(bad),
            Err(CondensationError::BadValue(_))
        ));
    }

    #[test]
    fn a_duplicate_element_fails_closed() {
        let bad = r#"
[[condensation]]
element = "Fe"
z = 26
t50_k = "1334"
host = "Fe alloy"

[[condensation]]
element = "Fe"
z = 26
t50_k = "1334"
host = "Fe alloy"
"#;
        assert!(matches!(
            CondensationTable::from_toml_str(bad),
            Err(CondensationError::Duplicate(_))
        ));
    }
}
