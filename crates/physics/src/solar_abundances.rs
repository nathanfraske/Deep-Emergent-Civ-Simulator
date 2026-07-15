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

//! The AGSS09 solar abundance pattern (`crates/physics/data/solar_abundances_agss09.toml`), the disk-condensation
//! composition vector: per element the present-day solar photospheric and CI-chondrite (meteoritic) abundance on the
//! `log-epsilon(H) = 12` scale (Asplund, Grevesse, Sauval & Scott 2009, ARA&A 47, 481, Table 1). AGSS09 is the
//! project's pinned solar anchor (recommended photospheric `Z = 0.0134`). The composition vector fixes how much of
//! each element the cooling disk gas carries, so a Gibbs-minimization condensation sequence (see `janaf.rs` and
//! `condensation.rs`) reads it as the elemental inventory it partitions between gas and condensate.
//!
//! Cited [M], transcribed from the primary arXiv reprint PDF (the Lodders-figure protocol: transcribe the table and
//! fingerprint-check spot rows). The parse routes every abundance through the exact `BigRat` conduit to `Fixed` (no
//! floating point reaches canonical state). BLOCK KIND `[[abundance]]`, the cited-data-column idiom, NOT the reserved
//! floor `[[element]]` kind: an immutable transcription out of the floor's real/fantasy authorship axis. The
//! photospheric and meteoritic values live in DISTINCT columns (each a single grade: the AGSS09 3D-model photospheric
//! determination, and the Lodders-Palme-Gail 2009 CI compilation), so a consumer never reads one grade as the other.
//!
//! HONEST LIMITS: the scope is `Z = 1` (H) through `Z = 42` (Mo), gapless, the abundant-and-refractory span that
//! drives the condensation sequence (every rock-former, every abundant volatile, and the ultrarefractory Zr); the
//! heavier trace elements are condensation-negligible and are not vendored. The noble gases He, Ne, Ar, Kr carry an
//! INDIRECT photospheric estimate (`photosphere_indirect`), not a line-based measurement; their meteoritic entries
//! are the depleted residue and carry no sigma. As, Se, Br have no photospheric determination (meteoritic only). The
//! `source` flag records the preferred present-day solar value (photospheric where any photospheric determination
//! exists, since AGSS09's `Z` is computed from the photosphere; meteoritic only where none does); a condensation
//! consumer may still substitute the lower-sigma CI value for a rock-forming refractory, the two agreeing to a mean
//! `0.00 +/- 0.05` dex over the non-volatiles, but the volatile columns are not interchangeable (volatiles are
//! depleted in meteorites, Li in the photosphere). No consumer is wired in any pinned run path (byte-neutral).

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the solar abundance column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolarAbundanceError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// An abundance string could not be parsed to fixed-point.
    BadValue(String),
    /// A `source` flag was neither `photospheric` nor `meteoritic`.
    BadSource(String),
    /// An element symbol appears twice.
    Duplicate(String),
    /// A row's `source` names a column that carries no value (e.g. `source = photospheric` with no photospheric
    /// abundance), so the preferred value would be absent.
    SourceWithoutValue(String),
}

impl fmt::Display for SolarAbundanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolarAbundanceError::Parse(m) => write!(f, "solar abundance parse error: {m}"),
            SolarAbundanceError::BadValue(m) => write!(f, "solar abundance value error: {m}"),
            SolarAbundanceError::BadSource(m) => write!(f, "solar abundance source error: {m}"),
            SolarAbundanceError::Duplicate(m) => {
                write!(f, "duplicate solar abundance element: {m}")
            }
            SolarAbundanceError::SourceWithoutValue(m) => {
                write!(f, "solar abundance preferred source has no value: {m}")
            }
        }
    }
}

impl std::error::Error for SolarAbundanceError {}

/// Which column an element's preferred present-day solar abundance is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbundanceSource {
    /// The photospheric determination (AGSS09's `Z` is computed from the photosphere).
    Photospheric,
    /// The CI-chondrite (meteoritic) value, preferred only where no photospheric determination exists.
    Meteoritic,
}

/// One element's abundance entry from Table 1, both columns carried verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbundanceRow {
    /// The element symbol.
    pub symbol: String,
    /// The atomic number.
    pub z: u32,
    /// The photospheric abundance `log-epsilon` (dex, H = 12); `None` for As, Se, Br.
    pub log_eps_photosphere: Option<Fixed>,
    /// The 1-sigma uncertainty on the photospheric value (dex); `None` where the source gives none (H's `12.00` is
    /// the scale definition).
    pub sigma_photosphere: Option<Fixed>,
    /// Whether the photospheric value is an INDIRECT estimate (the noble gases He, Ne, Ar, Kr; AGSS09 Sect. 3.9),
    /// not a line-based measurement.
    pub photosphere_indirect: bool,
    /// The CI-chondrite (meteoritic) abundance `log-epsilon` (dex); present for every vendored row.
    pub log_eps_meteorite: Option<Fixed>,
    /// The 1-sigma uncertainty on the meteoritic value (dex); `None` for the depleted noble-gas residues.
    pub sigma_meteorite: Option<Fixed>,
    /// Which column is the preferred present-day solar value.
    pub source: AbundanceSource,
}

impl AbundanceRow {
    /// The preferred present-day solar abundance (`log-epsilon`), drawn from the column the `source` flag names.
    pub fn preferred(&self) -> Option<Fixed> {
        match self.source {
            AbundanceSource::Photospheric => self.log_eps_photosphere,
            AbundanceSource::Meteoritic => self.log_eps_meteorite,
        }
    }
}

/// The loaded abundance column, keyed by element symbol, plus the table's scale and recommended mass fractions.
#[derive(Debug, Clone, Default)]
pub struct SolarAbundances {
    rows: BTreeMap<String, AbundanceRow>,
    scale: String,
    x_mass_fraction: String,
    y_mass_fraction: String,
    z_mass_fraction: String,
    z_over_x: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawFile {
    #[serde(default)]
    scale: String,
    #[serde(default)]
    x_mass_fraction: String,
    #[serde(default)]
    y_mass_fraction: String,
    #[serde(default)]
    z_mass_fraction: String,
    #[serde(default)]
    z_over_x: String,
    // `[[abundance]]`, the cited-data-column block kind, NOT the reserved floor `[[element]]` kind.
    #[serde(default)]
    abundance: Vec<RawRow>,
}

#[derive(Debug, Default, Deserialize)]
struct RawRow {
    symbol: String,
    z: u32,
    #[serde(default)]
    log_eps_photosphere: Option<String>,
    #[serde(default)]
    sigma_photosphere: Option<String>,
    #[serde(default)]
    photosphere_indirect: bool,
    #[serde(default)]
    log_eps_meteorite: Option<String>,
    #[serde(default)]
    sigma_meteorite: Option<String>,
    #[serde(default)]
    source: String,
}

/// Parse one decimal abundance string to `Fixed` through the exact `BigRat` path (a negative value, e.g. the
/// depleted meteoritic noble gases, parses the same as a positive one).
fn fixed_from_decimal(s: &str) -> Result<Fixed, SolarAbundanceError> {
    let br = BigRat::from_decimal_str(s).map_err(SolarAbundanceError::BadValue)?;
    let bits = br
        .round_to_scale(Fixed::FRAC_BITS)
        .ok_or_else(|| SolarAbundanceError::BadValue(format!("{s} out of range")))?;
    Fixed::from_bits_i128(bits)
        .ok_or_else(|| SolarAbundanceError::BadValue(format!("{s} out of range")))
}

fn opt_fixed(s: &Option<String>) -> Result<Option<Fixed>, SolarAbundanceError> {
    match s {
        Some(v) => Ok(Some(fixed_from_decimal(v)?)),
        None => Ok(None),
    }
}

impl SolarAbundances {
    /// Parse and validate the column from a TOML string. Every row keys off a unique element symbol; both columns are
    /// optional per row (As/Se/Br carry no photospheric value); a malformed abundance or an unknown `source` flag
    /// fails closed, and a `source` naming an absent column is rejected so the preferred value is never silently
    /// missing.
    pub fn from_toml_str(s: &str) -> Result<Self, SolarAbundanceError> {
        let file: RawFile =
            toml::from_str(s).map_err(|e| SolarAbundanceError::Parse(e.to_string()))?;
        let mut rows = BTreeMap::new();
        for raw in file.abundance {
            let source = match raw.source.trim() {
                "photospheric" => AbundanceSource::Photospheric,
                "meteoritic" => AbundanceSource::Meteoritic,
                other => {
                    return Err(SolarAbundanceError::BadSource(format!(
                        "{}: '{other}' (expected photospheric or meteoritic)",
                        raw.symbol
                    )));
                }
            };
            let log_eps_photosphere = opt_fixed(&raw.log_eps_photosphere)?;
            let log_eps_meteorite = opt_fixed(&raw.log_eps_meteorite)?;
            let has_preferred = match source {
                AbundanceSource::Photospheric => log_eps_photosphere.is_some(),
                AbundanceSource::Meteoritic => log_eps_meteorite.is_some(),
            };
            if !has_preferred {
                return Err(SolarAbundanceError::SourceWithoutValue(raw.symbol));
            }
            let row = AbundanceRow {
                symbol: raw.symbol.clone(),
                z: raw.z,
                log_eps_photosphere,
                sigma_photosphere: opt_fixed(&raw.sigma_photosphere)?,
                photosphere_indirect: raw.photosphere_indirect,
                log_eps_meteorite,
                sigma_meteorite: opt_fixed(&raw.sigma_meteorite)?,
                source,
            };
            if rows.insert(raw.symbol.clone(), row).is_some() {
                return Err(SolarAbundanceError::Duplicate(raw.symbol));
            }
        }
        Ok(SolarAbundances {
            rows,
            scale: file.scale,
            x_mass_fraction: file.x_mass_fraction,
            y_mass_fraction: file.y_mass_fraction,
            z_mass_fraction: file.z_mass_fraction,
            z_over_x: file.z_over_x,
        })
    }

    /// Load the standard AGSS09 Table 1 column from the checked-in data file.
    pub fn standard() -> Result<Self, SolarAbundanceError> {
        Self::from_toml_str(include_str!("../data/solar_abundances_agss09.toml"))
    }

    /// The abundance entry for an element symbol, or `None` if the table does not cover it.
    pub fn element(&self, symbol: &str) -> Option<&AbundanceRow> {
        self.rows.get(symbol)
    }

    /// The photospheric `log-epsilon` for an element, if the table gives one.
    pub fn log_eps_photosphere(&self, symbol: &str) -> Option<Fixed> {
        self.rows.get(symbol).and_then(|r| r.log_eps_photosphere)
    }

    /// The meteoritic (CI-chondrite) `log-epsilon` for an element, if the table gives one.
    pub fn log_eps_meteorite(&self, symbol: &str) -> Option<Fixed> {
        self.rows.get(symbol).and_then(|r| r.log_eps_meteorite)
    }

    /// The preferred present-day solar `log-epsilon` for an element (the column the `source` flag names).
    pub fn preferred(&self, symbol: &str) -> Option<Fixed> {
        self.rows.get(symbol).and_then(|r| r.preferred())
    }

    /// The abundance scale label (e.g. `log_eps_H12`), for provenance.
    pub fn scale(&self) -> &str {
        &self.scale
    }

    /// The recommended present-day photospheric hydrogen mass fraction `X` (AGSS09 Table 4), a string.
    pub fn x_mass_fraction(&self) -> &str {
        &self.x_mass_fraction
    }

    /// The recommended present-day photospheric helium mass fraction `Y`, a string.
    pub fn y_mass_fraction(&self) -> &str {
        &self.y_mass_fraction
    }

    /// The recommended present-day photospheric metal mass fraction `Z` (the pinned solar anchor), a string.
    pub fn z_mass_fraction(&self) -> &str {
        &self.z_mass_fraction
    }

    /// The recommended present-day photospheric metals-to-hydrogen ratio `Z/X`, a string.
    pub fn z_over_x(&self) -> &str {
        &self.z_over_x
    }

    /// The element symbols in the table, sorted.
    pub fn elements(&self) -> impl Iterator<Item = &str> + '_ {
        self.rows.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> SolarAbundances {
        SolarAbundances::standard().expect("the AGSS09 solar abundance table loads")
    }

    #[test]
    fn the_table_loads_all_42_elements_at_the_stated_scale() {
        let t = table();
        assert_eq!(
            t.elements().count(),
            42,
            "Z = 1 through Z = 42 (H through Mo)"
        );
        assert_eq!(t.scale(), "log_eps_H12", "the log-epsilon(H)=12 scale");
        assert_eq!(t.z_mass_fraction(), "0.0134", "the pinned solar anchor Z");
        assert_eq!(t.z_over_x(), "0.0181", "the recommended Z/X");
    }

    #[test]
    fn the_fingerprint_rows_reproduce_agss09_table_1() {
        // The owner's pre-registered transcription fingerprints (photospheric, within 0.05 dex): the load-bearing
        // rock-formers and abundant volatiles. If the transcription drifted, this fails.
        let t = table();
        let cases = [
            ("O", 8.69),
            ("C", 8.43),
            ("Fe", 7.50),
            ("Mg", 7.60),
            ("Si", 7.51),
            ("Ca", 6.34),
            ("Al", 6.45),
            ("Ti", 4.95),
            ("Na", 6.24),
            ("Ni", 6.22),
            ("S", 7.12),
        ];
        for (sym, want) in cases {
            let got = t
                .log_eps_photosphere(sym)
                .unwrap_or_else(|| panic!("{sym} has a photospheric abundance"));
            assert!(
                (got.to_f64_lossy() - want).abs() < 0.05,
                "{sym} photospheric log-eps is {want}, got {}",
                got.to_f64_lossy()
            );
        }
        // Hydrogen is 12.00 exactly, the scale definition (no fabricated uncertainty).
        let h = t.element("H").unwrap();
        assert!(
            (h.log_eps_photosphere.unwrap().to_f64_lossy() - 12.00).abs() < 1e-6,
            "H is 12.00 by definition"
        );
        assert_eq!(h.sigma_photosphere, None, "H's definition carries no sigma");
    }

    #[test]
    fn the_abundance_spine_is_monotonic() {
        // A coarse sanity check on the transcription: the abundant volatiles dominate, the rock-formers sit below
        // them, and the refractory trace elements far below (a scrambled column would break this spine).
        let t = table();
        let g = |s: &str| t.log_eps_photosphere(s).unwrap().to_f64_lossy();
        assert!(g("H") > g("O"), "H is the most abundant");
        assert!(g("O") > g("C"), "O exceeds C");
        assert!(g("C") > g("Fe"), "C exceeds Fe");
        assert!(g("Fe") > g("Na"), "Fe exceeds Na");
        assert!(g("Na") > g("Ti"), "Na exceeds Ti");
        assert!(g("Ti") > g("Zr"), "Ti exceeds the ultrarefractory Zr");
    }

    #[test]
    fn the_noble_gases_carry_indirect_photospheric_estimates() {
        // He, Ne, Ar, Kr are flagged indirect (AGSS09 Sect. 3.9); a line-measured element is not.
        let t = table();
        for sym in ["He", "Ne", "Ar", "Kr"] {
            assert!(
                t.element(sym).unwrap().photosphere_indirect,
                "{sym} carries an indirect photospheric estimate"
            );
        }
        assert!(
            !t.element("Fe").unwrap().photosphere_indirect,
            "Fe is line-measured, not indirect"
        );
    }

    #[test]
    fn a_meteoritic_only_element_prefers_the_meteoritic_value() {
        // As, Se, Br have no photospheric determination, so the preferred value is the CI-chondrite one.
        let t = table();
        for (sym, want) in [("As", 2.30), ("Se", 3.34), ("Br", 2.54)] {
            let row = t.element(sym).unwrap();
            assert_eq!(
                row.source,
                AbundanceSource::Meteoritic,
                "{sym} is meteoritic"
            );
            assert_eq!(
                row.log_eps_photosphere, None,
                "{sym} has no photospheric value"
            );
            assert!(
                (row.preferred().unwrap().to_f64_lossy() - want).abs() < 0.05,
                "{sym} preferred is the CI value {want}"
            );
        }
    }

    #[test]
    fn a_bad_source_flag_fails_closed() {
        let bad = r#"
[[abundance]]
symbol = "X"
z = 200
log_eps_photosphere = "5.0"
source = "guessed"
"#;
        assert!(matches!(
            SolarAbundances::from_toml_str(bad),
            Err(SolarAbundanceError::BadSource(_))
        ));
    }

    #[test]
    fn a_source_naming_an_absent_column_fails_closed() {
        let bad = r#"
[[abundance]]
symbol = "X"
z = 200
log_eps_meteorite = "5.0"
source = "photospheric"
"#;
        assert!(matches!(
            SolarAbundances::from_toml_str(bad),
            Err(SolarAbundanceError::SourceWithoutValue(_))
        ));
    }

    #[test]
    fn a_duplicate_element_fails_closed() {
        let bad = r#"
[[abundance]]
symbol = "Fe"
z = 26
log_eps_photosphere = "7.50"
source = "photospheric"

[[abundance]]
symbol = "Fe"
z = 26
log_eps_photosphere = "7.50"
source = "photospheric"
"#;
        assert!(matches!(
            SolarAbundances::from_toml_str(bad),
            Err(SolarAbundanceError::Duplicate(_))
        ));
    }
}
