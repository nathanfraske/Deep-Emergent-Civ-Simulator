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

//! The Robie & Hemingway (1995) CaTiO3 perovskite standard Gibbs energy of formation
//! (`crates/physics/data/perovskite_robie1995.toml`), the CaTiO3 condensation front. NIST-JANAF (see `janaf.rs`) has
//! no Ca-Ti-O species, so this fills the beyond-JANAF gap: perovskite is the high-temperature titanium condensate
//! (condenses ~1582 K at `1e-4` bar solar, Lodders Ti = 1582 K), the front a Gibbs-minimization over the loaded
//! potentials must reproduce, a calibration target rather than a run input.
//!
//! Cited `[M]`, owner-pinned to the public-domain primary: Robie, R.A. & Hemingway, B.S. (1995), USGS Bulletin 2131,
//! the PEROVSKITE table (p.232), formation from the elements. Transcribed from the primary USGS PDF (the Lodders
//! protocol: transcribe the table, fingerprint-check spot rows). The second witness is Barin (Thermochemical Data of
//! Pure Substances), giving `delta-f H(298) ~ -1660.6`, `S(298) ~ 93.64`, `delta-f G(298) ~ -1575.3`, essentially
//! identical to Robie. The GGchem open condensate file lists `CaTiO3[s]` with a Sharp & Huebner (1990) atom-referenced
//! fit, confirming perovskite as the Ti condensate (a compilation-of-compilations membership cross-check, not a
//! numeric `[M]` here). Source-ladder (owner ruling): for a measurable terrestrial mineral the alternative `[M]` (Robie)
//! precedes any estimator; a Born-Lande estimate is the alien rung, never used here.
//!
//! The parse routes every value through the exact `BigRat` conduit to `Fixed` (no floating point reaches canonical
//! state). BLOCK KIND `[[point]]`, the cited-data-column idiom, NOT a reserved floor kind: an immutable transcription
//! out of the floor's real/fantasy authorship axis. GRADE is single (one compilation, one phase progression).
//!
//! HONEST LIMITS: the reference is formation from the elements, so `delta-f H` steps at 1200 K (~12 kJ) as the
//! calcium reference state changes when Ca melts near 1115 K (a reference-element feature, not an error; `delta-f G`
//! stays smooth); perovskite is orthorhombic below 1530 K and cubic 1530-1800 K (a `Cp` discontinuity there); the
//! melting temperature is 2188 K. The load-bearing values are the tabulated `delta-f G(T)`. No consumer is wired in
//! any pinned run path (byte-neutral).

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the perovskite column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerovskiteError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A value string could not be parsed to fixed-point.
    BadValue(String),
    /// A temperature appears twice.
    Duplicate(String),
}

impl fmt::Display for PerovskiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PerovskiteError::Parse(m) => write!(f, "perovskite parse error: {m}"),
            PerovskiteError::BadValue(m) => write!(f, "perovskite value error: {m}"),
            PerovskiteError::Duplicate(m) => write!(f, "duplicate perovskite temperature: {m}"),
        }
    }
}

impl std::error::Error for PerovskiteError {}

/// One tabulated temperature row of the perovskite formation-from-elements table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerovskiteRow {
    /// Temperature (K).
    pub t_k: Fixed,
    /// The standard Gibbs energy of formation from the elements `delta-f G` (kJ/mol), the load-bearing value.
    pub delta_f_g_kj: Fixed,
    /// The standard enthalpy of formation from the elements `delta-f H` (kJ/mol).
    pub delta_f_h_kj: Fixed,
    /// The standard entropy `S` (J/mol/K); present only on the 298.15 K anchor row.
    pub s_j: Option<Fixed>,
    /// The heat capacity `Cp` (J/mol/K); present only on the 298.15 K anchor row.
    pub cp_j: Option<Fixed>,
}

/// The loaded perovskite column, keyed by an integer-milliKelvin temperature id for a deterministic exact lookup,
/// plus the substance metadata.
#[derive(Debug, Clone, Default)]
pub struct PerovskiteGibbs {
    rows: BTreeMap<i64, PerovskiteRow>,
    substance: String,
    mineral: String,
    phase: String,
    reference: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawFile {
    #[serde(default)]
    substance: String,
    #[serde(default)]
    mineral: String,
    #[serde(default)]
    phase: String,
    #[serde(default)]
    reference: String,
    // `[[point]]`, the cited-data-column block kind, NOT a reserved floor kind.
    #[serde(default)]
    point: Vec<RawPoint>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPoint {
    t_k: String,
    delta_f_g_kj: String,
    delta_f_h_kj: String,
    #[serde(default)]
    s_j: Option<String>,
    #[serde(default)]
    cp_j: Option<String>,
}

/// Parse one decimal string to `Fixed` through the exact `BigRat` path (a negative energy parses the same as a
/// positive one).
fn fixed_from_decimal(s: &str) -> Result<Fixed, PerovskiteError> {
    let br = BigRat::from_decimal_str(s).map_err(PerovskiteError::BadValue)?;
    let bits = br
        .round_to_scale(Fixed::FRAC_BITS)
        .ok_or_else(|| PerovskiteError::BadValue(format!("{s} out of range")))?;
    Fixed::from_bits_i128(bits)
        .ok_or_else(|| PerovskiteError::BadValue(format!("{s} out of range")))
}

fn opt_fixed(s: &Option<String>) -> Result<Option<Fixed>, PerovskiteError> {
    match s {
        Some(v) => Ok(Some(fixed_from_decimal(v)?)),
        None => Ok(None),
    }
}

/// The integer-milliKelvin key of a temperature, so `298.15 K` and the round hundreds each key uniquely and exactly.
fn temp_key(t_k: Fixed) -> i64 {
    (t_k.to_f64_lossy() * 1000.0).round() as i64
}

impl PerovskiteGibbs {
    /// Parse and validate the column from a TOML string. Every point keys off a unique temperature; a malformed
    /// value fails closed.
    pub fn from_toml_str(s: &str) -> Result<Self, PerovskiteError> {
        let file: RawFile = toml::from_str(s).map_err(|e| PerovskiteError::Parse(e.to_string()))?;
        let mut rows = BTreeMap::new();
        for raw in file.point {
            let t_k = fixed_from_decimal(&raw.t_k)?;
            let row = PerovskiteRow {
                t_k,
                delta_f_g_kj: fixed_from_decimal(&raw.delta_f_g_kj)?,
                delta_f_h_kj: fixed_from_decimal(&raw.delta_f_h_kj)?,
                s_j: opt_fixed(&raw.s_j)?,
                cp_j: opt_fixed(&raw.cp_j)?,
            };
            if rows.insert(temp_key(t_k), row).is_some() {
                return Err(PerovskiteError::Duplicate(raw.t_k));
            }
        }
        Ok(PerovskiteGibbs {
            rows,
            substance: file.substance,
            mineral: file.mineral,
            phase: file.phase,
            reference: file.reference,
        })
    }

    /// Load the standard Robie-Hemingway perovskite column from the checked-in data file.
    pub fn standard() -> Result<Self, PerovskiteError> {
        Self::from_toml_str(include_str!("../data/perovskite_robie1995.toml"))
    }

    /// The full row at an exactly tabulated temperature, or `None` if that temperature is not tabulated.
    pub fn row_at(&self, t_k: Fixed) -> Option<&PerovskiteRow> {
        self.rows.get(&temp_key(t_k))
    }

    /// The standard Gibbs energy of formation `delta-f G` (kJ/mol) at an exactly tabulated temperature.
    pub fn delta_f_g_at(&self, t_k: Fixed) -> Option<Fixed> {
        self.rows.get(&temp_key(t_k)).map(|r| r.delta_f_g_kj)
    }

    /// The standard enthalpy of formation `delta-f H` (kJ/mol) at an exactly tabulated temperature.
    pub fn delta_f_h_at(&self, t_k: Fixed) -> Option<Fixed> {
        self.rows.get(&temp_key(t_k)).map(|r| r.delta_f_h_kj)
    }

    /// The substance formula (e.g. `CaTiO3`), for provenance.
    pub fn substance(&self) -> &str {
        &self.substance
    }

    /// The mineral name (e.g. `perovskite`).
    pub fn mineral(&self) -> &str {
        &self.mineral
    }

    /// The phase label (e.g. `cr`).
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// The thermodynamic reference (e.g. `formation-from-elements`).
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// The rows in temperature order.
    pub fn rows(&self) -> impl Iterator<Item = &PerovskiteRow> + '_ {
        self.rows.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PerovskiteGibbs {
        PerovskiteGibbs::standard().expect("the Robie-Hemingway perovskite column loads")
    }

    fn at(k: i64) -> Fixed {
        Fixed::from_int(k as i32)
    }

    #[test]
    fn the_table_loads_the_anchor_and_six_high_temperatures() {
        let t = table();
        assert_eq!(t.rows().count(), 7, "the 298.15 K anchor plus 1200-1700 K");
        assert_eq!(t.substance(), "CaTiO3");
        assert_eq!(t.mineral(), "perovskite");
        assert_eq!(t.reference(), "formation-from-elements");
    }

    #[test]
    fn the_fingerprint_rows_reproduce_robie_hemingway() {
        // The owner's pre-registered fingerprint: perovskite is very refractory, so delta-f H(298) is the -1660
        // kJ/mol class, and delta-f G is strongly negative throughout. If the transcription drifted, this fails.
        let t = table();
        let anchor = Fixed::from_ratio(29815, 100);
        let dfh298 = t
            .delta_f_h_at(anchor)
            .expect("the 298.15 K anchor is present");
        assert!(
            (dfh298.to_f64_lossy() - -1660.6).abs() < 1.0,
            "delta-f H(298.15) is -1660.6 kJ/mol, got {}",
            dfh298.to_f64_lossy()
        );
        let dfg298 = t.delta_f_g_at(anchor).unwrap();
        assert!(
            (dfg298.to_f64_lossy() - -1574.8).abs() < 1.0,
            "delta-f G(298.15) is -1574.8 kJ/mol, got {}",
            dfg298.to_f64_lossy()
        );
        // The six requested high-temperature delta-f G values.
        let cases = [
            (1200, -1323.9),
            (1300, -1295.6),
            (1400, -1267.5),
            (1500, -1239.4),
            (1600, -1211.8),
            (1700, -1184.1),
        ];
        for (temp, want) in cases {
            let got = t
                .delta_f_g_at(at(temp))
                .unwrap_or_else(|| panic!("{temp} K is tabulated"));
            assert!(
                (got.to_f64_lossy() - want).abs() < 0.1,
                "delta-f G({temp}) is {want} kJ/mol, got {}",
                got.to_f64_lossy()
            );
            assert!(
                got.to_f64_lossy() < -1100.0,
                "{temp} K delta-f G stays strongly negative"
            );
        }
    }

    #[test]
    fn the_gibbs_energy_rises_monotonically_with_temperature() {
        // delta-f G = delta-f H - T*delta-f S; the reaction has a large negative delta-f S (three elements and O2
        // gas consumed into one solid), so -T*delta-f S is positive and grows, making delta-f G less negative as T
        // rises (a scrambled column would break this monotonic spine across 298-1700 K).
        let t = table();
        let g = |k: i64| {
            if k == 298 {
                t.delta_f_g_at(Fixed::from_ratio(29815, 100)).unwrap()
            } else {
                t.delta_f_g_at(at(k)).unwrap()
            }
            .to_f64_lossy()
        };
        let temps = [298i64, 1200, 1300, 1400, 1500, 1600, 1700];
        for w in temps.windows(2) {
            assert!(g(w[1]) > g(w[0]), "delta-f G at {} exceeds {}", w[1], w[0]);
        }
    }

    #[test]
    fn the_anchor_carries_entropy_and_heat_capacity_only() {
        // The 298.15 K anchor carries S and Cp; the high-temperature rows do not (not a fabricated value).
        let t = table();
        let anchor = t.row_at(Fixed::from_ratio(29815, 100)).unwrap();
        assert!(
            (anchor.s_j.unwrap().to_f64_lossy() - 93.64).abs() < 0.01,
            "S(298.15) is 93.64 J/mol/K"
        );
        assert!(
            (anchor.cp_j.unwrap().to_f64_lossy() - 97.65).abs() < 0.01,
            "Cp(298.15) is 97.65 J/mol/K"
        );
        let hot = t.row_at(at(1500)).unwrap();
        assert_eq!(hot.s_j, None, "a high-T row carries no anchor entropy");
        assert_eq!(
            hot.cp_j, None,
            "a high-T row carries no anchor heat capacity"
        );
    }

    #[test]
    fn an_untabulated_temperature_returns_none() {
        let t = table();
        assert_eq!(t.delta_f_g_at(at(1250)), None);
    }

    #[test]
    fn a_bad_value_fails_closed() {
        let bad = r#"
[[point]]
t_k = "1500"
delta_f_g_kj = "not-a-number"
delta_f_h_kj = "-1658.9"
"#;
        assert!(matches!(
            PerovskiteGibbs::from_toml_str(bad),
            Err(PerovskiteError::BadValue(_))
        ));
    }

    #[test]
    fn a_duplicate_temperature_fails_closed() {
        let bad = r#"
[[point]]
t_k = "1500"
delta_f_g_kj = "-1239.4"
delta_f_h_kj = "-1658.9"

[[point]]
t_k = "1500"
delta_f_g_kj = "-1239.4"
delta_f_h_kj = "-1658.9"
"#;
        assert!(matches!(
            PerovskiteGibbs::from_toml_str(bad),
            Err(PerovskiteError::Duplicate(_))
        ));
    }
}
