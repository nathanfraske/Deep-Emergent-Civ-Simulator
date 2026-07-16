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

//! The AGSS09 solar abundance pattern (`crates/physics/data/solar_abundances_agss09.toml`), the `[M]`-tier SOLAR
//! CROSS-CHECKER: the Sun's measured elemental composition, the reference the solar (Mirror) case is VALIDATED
//! against. It is NOT the disk-composition input for every world. Per element the present-day solar photospheric and
//! CI-chondrite (meteoritic) abundance on the `log-epsilon(H) = 12` scale (Asplund, Grevesse, Sauval & Scott 2009,
//! ARA&A 47, 481, Table 1); AGSS09 is the project's pinned solar anchor (recommended photospheric `Z = 0.0134`).
//!
//! ROLE (owner ruling, 2026-07-15). The disk-condensation composition is a PER-WORLD INPUT, the star's own abundance
//! pattern (admit-the-alien: a carbon star with `C/O > 1` is a data row that condenses carbides and graphite, not
//! silicates, and the Gibbs-minimization condensation sequence in `janaf.rs` and `condensation.rs` reads THAT
//! per-world pattern). The AGSS09 values are the Sun's data row AND the cross-check that the solar case reconstructs
//! reality; they never author the composition of an alien world. A consumer that reads this table as the universal
//! disk composition for all worlds is the SOLAR-BIAS DEFECT to retire (the composition-as-per-world-input arc): the
//! reference validates, it is not the input. The viewer's derived-planet path is the current such consumer, flagged.
//!
//! Cited `[M]`, transcribed from the primary arXiv reprint PDF (the Lodders-figure protocol: transcribe the table and
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

    /// A COPY of this pattern with every METAL (atomic number `Z >= 3`, everything heavier than helium) shifted by
    /// `delta_dex` on the `log-epsilon` scale, hydrogen (`Z = 1`) and helium (`Z = 2`) held fixed. This is the
    /// first-order `[Fe/H]` metallicity scaling: `[Fe/H]` moves the AMOUNT of metals relative to hydrogen, so a draw of
    /// `[Fe/H] = +0.2` adds `+0.2` dex to every metal's abundance (a factor `10^0.2` more metal), and `[Fe/H] = -0.5`
    /// subtracts `0.5` dex (a metal-poor pattern). Both the photospheric and meteoritic columns shift together so the
    /// two grades stay consistent; the per-element `sigma` uncertainties are untouched (a shift of the value, not of its
    /// error). `delta_dex = 0` returns a byte-identical copy (adding the fixed-point zero is exact), which is what pins
    /// the solar/Mirror instance to the unshifted AGSS09 pattern through the same call.
    ///
    /// SCOPE. This is the UNIFORM amount-scaling the `[Fe/H]` link owns: it leaves every inter-metal RATIO (Fe/Si,
    /// Mg/Si, C/O) unchanged, so the KIND of world (its silicate family, its core fraction, its carbide-versus-silicate
    /// branch) is untouched here. Those are the later conditional-chain links (`[alpha/Fe]`, Mg/Si, C/O), which
    /// differentiate individual elements on top of this scaling. Keying on `Z` (not a fixed element list) means an
    /// alien pattern with unusual elements scales as data, never a rewrite (Prime Directive 7).
    pub fn scaled_metals_by_dex(&self, delta_dex: Fixed) -> SolarAbundances {
        let mut rows = self.rows.clone();
        for row in rows.values_mut() {
            // Metals are everything heavier than helium (Z >= 3); H (Z=1) and He (Z=2) are not scaled by [Fe/H].
            if row.z >= 3 {
                if let Some(v) = row.log_eps_photosphere {
                    row.log_eps_photosphere = Some(v + delta_dex);
                }
                if let Some(v) = row.log_eps_meteorite {
                    row.log_eps_meteorite = Some(v + delta_dex);
                }
            }
        }
        SolarAbundances {
            rows,
            scale: self.scale.clone(),
            x_mass_fraction: self.x_mass_fraction.clone(),
            y_mass_fraction: self.y_mass_fraction.clone(),
            z_mass_fraction: self.z_mass_fraction.clone(),
            z_over_x: self.z_over_x.clone(),
        }
    }

    /// A COPY of this pattern with the ALPHA-CAPTURE elements (the symbols in `alpha_symbols`) shifted by `delta_dex`
    /// on the `log-epsilon` scale, EVERY OTHER element (iron, the iron-peak, hydrogen, helium) held fixed. This is the
    /// `[alpha/Fe]` enhancement the alpha-knee link owns: `[alpha/Fe]` moves the alpha-process elements (O, Mg, Si, Ca,
    /// Ti) UP relative to iron, so a draw of `[alpha/Fe] = +0.3` adds `+0.3` dex to each alpha element (a factor
    /// `10^0.3` more alpha at fixed iron), lifting the rock-former-to-iron ratio and, through it, the
    /// silicate-mantle-to-metal-core mass balance the bulk density reads. Iron is NOT shifted (the ratio is
    /// alpha-OVER-iron), so this is the first KIND-changing link: it moves the core mass fraction and hence the derived
    /// DENSITY, where [`SolarAbundances::scaled_metals_by_dex`] (the `[Fe/H]` amount-scaling) left every inter-metal
    /// ratio, and so the density, fixed. Both the photospheric and meteoritic columns shift together so the two grades
    /// stay consistent; the per-element `sigma` uncertainties are untouched (a shift of the value, not of its error).
    /// `delta_dex = 0` returns a byte-identical copy (adding the fixed-point zero is exact), which pins the solar/Mirror
    /// instance (`[alpha/Fe] = 0` at the solar pin) to the unshifted pattern through the same call.
    ///
    /// KEYED ON ELEMENT IDENTITY (Prime Directive 7, admit the alien). The alpha membership is DATA (`alpha_symbols`,
    /// the nucleosynthetic alpha-element set the caller supplies), never a fixed rule on `Z`: the metals are every
    /// `Z >= 3`, but the alpha elements are a specific cited set of alpha-capture products, so the membership grows as
    /// data and an alien pattern that names other elements is a data row, not a code change. A symbol absent from this
    /// pattern is skipped (an alien pattern missing an element is not an error), and this holds Mg/Si (both alpha)
    /// fixed under a uniform lift, so the silicate-family (olivine-versus-pyroxene) channel is a later element-
    /// differential refinement, not this uniform link.
    pub fn scaled_alpha_by_dex(&self, delta_dex: Fixed, alpha_symbols: &[&str]) -> SolarAbundances {
        let mut rows = self.rows.clone();
        for symbol in alpha_symbols {
            if let Some(row) = rows.get_mut(*symbol) {
                if let Some(v) = row.log_eps_photosphere {
                    row.log_eps_photosphere = Some(v + delta_dex);
                }
                if let Some(v) = row.log_eps_meteorite {
                    row.log_eps_meteorite = Some(v + delta_dex);
                }
            }
        }
        SolarAbundances {
            rows,
            scale: self.scale.clone(),
            x_mass_fraction: self.x_mass_fraction.clone(),
            y_mass_fraction: self.y_mass_fraction.clone(),
            z_mass_fraction: self.z_mass_fraction.clone(),
            z_over_x: self.z_over_x.clone(),
        }
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

    #[test]
    fn scaling_metals_by_zero_dex_is_byte_identical() {
        // The [Fe/H] = 0 (solar/Mirror pin) scaling is a byte-identical copy: adding the fixed-point zero to every
        // metal is exact, and hydrogen and helium are never touched. This is what keeps the pinned globe and the run
        // pins bit-exact.
        let solar = table();
        let scaled = solar.scaled_metals_by_dex(Fixed::ZERO);
        for sym in solar.elements() {
            assert_eq!(
                scaled.log_eps_photosphere(sym).map(|f| f.to_bits()),
                solar.log_eps_photosphere(sym).map(|f| f.to_bits()),
                "photospheric byte-identical at 0 dex for {sym}"
            );
            assert_eq!(
                scaled.log_eps_meteorite(sym).map(|f| f.to_bits()),
                solar.log_eps_meteorite(sym).map(|f| f.to_bits()),
                "meteoritic byte-identical at 0 dex for {sym}"
            );
        }
    }

    #[test]
    fn scaling_metals_shifts_metals_and_holds_hydrogen_and_helium() {
        // A nonzero shift moves every metal (Z >= 3) by exactly delta on the log-eps scale and leaves H (Z=1) and He
        // (Z=2) fixed: the first-order [Fe/H] amount-scaling. Inter-metal ratios (here Fe - O) are preserved, so the
        // KIND of world is untouched (that is the later links' job).
        let solar = table();
        let delta = Fixed::from_ratio(3, 10); // +0.3 dex
        let scaled = solar.scaled_metals_by_dex(delta);
        // Hydrogen unchanged.
        assert_eq!(
            scaled.preferred("H").map(|f| f.to_bits()),
            solar.preferred("H").map(|f| f.to_bits()),
            "hydrogen is not a metal and is unchanged"
        );
        // Helium unchanged.
        assert_eq!(
            scaled.preferred("He").map(|f| f.to_bits()),
            solar.preferred("He").map(|f| f.to_bits()),
            "helium is not a metal and is unchanged"
        );
        // Iron shifted by exactly delta.
        let fe0 = solar.preferred("Fe").expect("Fe present");
        let fe1 = scaled.preferred("Fe").expect("Fe present scaled");
        assert_eq!(
            fe1.to_bits(),
            (fe0 + delta).to_bits(),
            "iron shifted by exactly +0.3 dex"
        );
        // The Fe - O contrast (an inter-metal ratio) is preserved: both shifted by the same delta.
        let o0 = solar.preferred("O").expect("O present");
        let o1 = scaled.preferred("O").expect("O present scaled");
        assert_eq!(
            (fe1 - o1).to_bits(),
            (fe0 - o0).to_bits(),
            "the Fe/O ratio (an inter-metal contrast) is preserved by uniform metal scaling"
        );
    }

    // The nucleosynthetic alpha-capture set the [alpha/Fe] enhancement lifts (mirrors the const the draw supplies).
    const ALPHA: &[&str] = &["O", "Mg", "Si", "Ca", "Ti"];

    #[test]
    fn scaling_alpha_by_zero_dex_is_byte_identical() {
        // The [alpha/Fe] = 0 (solar/Mirror pin) alpha scaling is a byte-identical copy: adding the fixed-point zero to
        // each alpha element is exact, and every other element is untouched. This is what keeps the pinned globe and
        // the run pins bit-exact once the alpha link lands.
        let solar = table();
        let scaled = solar.scaled_alpha_by_dex(Fixed::ZERO, ALPHA);
        for sym in solar.elements() {
            assert_eq!(
                scaled.log_eps_photosphere(sym).map(|f| f.to_bits()),
                solar.log_eps_photosphere(sym).map(|f| f.to_bits()),
                "photospheric byte-identical at 0 dex for {sym}"
            );
            assert_eq!(
                scaled.log_eps_meteorite(sym).map(|f| f.to_bits()),
                solar.log_eps_meteorite(sym).map(|f| f.to_bits()),
                "meteoritic byte-identical at 0 dex for {sym}"
            );
        }
    }

    #[test]
    fn scaling_alpha_lifts_the_alpha_elements_relative_to_iron_and_holds_mg_over_si() {
        // A nonzero [alpha/Fe] shift moves each alpha element (O, Mg, Si, Ca, Ti) UP by exactly delta and leaves iron
        // (the ratio's denominator), the iron-peak Ni, hydrogen, and helium fixed: the [alpha/Fe] enhancement. Unlike
        // the uniform [Fe/H] metal scaling, this CHANGES an inter-metal ratio, Mg/Fe, which is the KIND lever (the
        // silicate-mantle-to-metal-core balance). But Mg/Si (both alpha) is HELD, so the uniform lift does not touch the
        // olivine-versus-pyroxene family (a later element-differential refinement).
        let solar = table();
        let delta = Fixed::from_ratio(3, 10); // +0.3 dex, the fetched alpha plateau
        let scaled = solar.scaled_alpha_by_dex(delta, ALPHA);
        // Iron unchanged (the denominator of [alpha/Fe]).
        assert_eq!(
            scaled.preferred("Fe").map(|f| f.to_bits()),
            solar.preferred("Fe").map(|f| f.to_bits()),
            "iron is not an alpha element and is the ratio denominator, unchanged"
        );
        // The iron-peak Ni, hydrogen, and helium unchanged.
        for held in ["Ni", "H", "He"] {
            assert_eq!(
                scaled.preferred(held).map(|f| f.to_bits()),
                solar.preferred(held).map(|f| f.to_bits()),
                "{held} is not an alpha element and is unchanged"
            );
        }
        // Magnesium shifted by exactly delta.
        let mg0 = solar.preferred("Mg").expect("Mg present");
        let mg1 = scaled.preferred("Mg").expect("Mg present scaled");
        assert_eq!(
            mg1.to_bits(),
            (mg0 + delta).to_bits(),
            "magnesium lifted by exactly +0.3 dex (an alpha element)"
        );
        // Mg/Fe (the KIND lever) CHANGED: magnesium rose relative to iron by delta.
        let fe = scaled.preferred("Fe").expect("Fe present");
        let fe0 = solar.preferred("Fe").expect("Fe present unscaled");
        assert_eq!(
            (mg1 - fe).to_bits(),
            (mg0 - fe0 + delta).to_bits(),
            "the Mg/Fe ratio rose by the alpha enhancement (the density-moving KIND lever)"
        );
        // Mg/Si (both alpha) HELD: the silicate family is untouched by the uniform lift.
        let si1 = scaled.preferred("Si").expect("Si present scaled");
        let mg0si0 = mg0 - solar.preferred("Si").expect("Si present unscaled");
        assert_eq!(
            (mg1 - si1).to_bits(),
            mg0si0.to_bits(),
            "Mg/Si (both alpha) is preserved by a uniform alpha lift (olivine/pyroxene untouched)"
        );
    }
}
