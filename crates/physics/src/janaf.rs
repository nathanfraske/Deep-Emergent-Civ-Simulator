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

//! The NIST-JANAF thermochemical tables (`crates/physics/data/janaf/`), the mu-standard total `[M]` top rung the
//! disk-condensation arc consumes: per species the temperature-tabulated standard chemical potential. Each species'
//! table is fetched verbatim from `janaf.nist.gov/tables/<code>.txt` (the artifact-native machine-readable form) and
//! header-cited to Chase 1998 (NIST-JANAF 4th ed., J. Phys. Chem. Ref. Data Monograph 9). The tabulated columns
//! give `mu-standard(T)` as a MEASURED total: the Gibbs energy function `-[G(T)-H(Tr)]/T` (J/mol/K), the standard
//! entropy `S(T)` (J/mol/K), the enthalpy increment `H(T)-H(Tr)` (kJ/mol), and the formation `delta-f H` and
//! `delta-f G` (kJ/mol). The standard Gibbs energy of formation `delta-f G(T)` is the species' standard chemical
//! potential at temperature; the function-plus-formation columns let a consumer reconstruct the absolute Gibbs
//! function when it needs it.
//!
//! This is a two-electron-quantum-hard measured quantity (a critically evaluated thermochemical compilation, not
//! derivable at the floor level), so it enters as cited `[M]` data, the tier the optical constants and the H- cross
//! section set the precedent for. The condensation sequence's whole thesis over an authored ladder is that a
//! carbon-rich or metal-poor disk is a different MEMBERSHIP over this same species library (which condensate wins is
//! a Gibbs-minimization over the loaded potentials), never a rewrite, and a species with no measured table is
//! handled later by an estimator from the material's own derived properties (an admit-the-alien slice), never a
//! missing-row hard-fail.
//!
//! BLOCK KIND `[[species]]`, the cited-data-column idiom (matching `optical_constants.rs`), NOT the reserved floor
//! `[[element]]`/`[[substance]]` kind: an immutable transcription of Chase 1998 does not participate in the floor's
//! real/fantasy authorship axis, and an authored species draws its potential from the estimator tier at runtime,
//! never written into this citation file (co-location laundering). PROVENANCE is the per-species header citation
//! (out of floor-Element policing). GRADE is single: every row is the JANAF 4th-ed critically-evaluated set.
//!
//! HONEST LIMITS: the tables are the NIST-JANAF 4th edition (1998), so an update to a species' evaluation would need
//! a re-fetch (the md5 receipt in the manifest pins the fetched bytes; the `janaf_provenance_test.py` battery
//! re-checks it). Solid water ice `H2O(cr)` and calcium-titanate perovskite `CaTiO3(cr)` are absent from NIST-JANAF
//! (the vendored water condensate is the liquid `H2O(l)`, and perovskite is an uncovered species-membership gap that
//! the condensation battery reads from another source or the estimator). Phase-transition rows in the condensate
//! tables (a lambda maximum, an `ALPHA <--> GAMMA` boundary) carry a text marker in place of the formation columns;
//! the loader keeps the row with its temperature and function columns and records the marker, with the formation
//! columns absent. No consumer is wired in any pinned run path (byte-neutral).

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the JANAF library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JanafError {
    /// The manifest could not be parsed as TOML.
    Parse(String),
    /// A species carries no citation (every table is cited to Chase 1998).
    MissingCitation(String),
    /// A manifest entry names a file with no embedded content.
    MissingFile(String),
    /// A species name appears twice.
    Duplicate(String),
    /// A table yielded no tabulated rows.
    Empty(String),
}

impl fmt::Display for JanafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JanafError::Parse(m) => write!(f, "janaf manifest parse error: {m}"),
            JanafError::MissingCitation(m) => write!(f, "janaf species without citation: {m}"),
            JanafError::MissingFile(m) => {
                write!(f, "janaf manifest entry with no embedded file: {m}")
            }
            JanafError::Duplicate(m) => write!(f, "duplicate janaf species: {m}"),
            JanafError::Empty(m) => write!(f, "janaf species with no rows: {m}"),
        }
    }
}

impl std::error::Error for JanafError {}

/// Parse one decimal cell to `Fixed` through the exact `BigRat` path, returning `None` for an empty cell, the `T=0`
/// Gibbs-energy-function sentinel `INFINITE`, a phase-transition marker word, or any other non-numeric text. The
/// same fixed-point conduit `optical_constants.rs` uses (no floating point reaches canonical state).
fn parse_cell(s: &str) -> Option<Fixed> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let br = BigRat::from_decimal_str(s).ok()?;
    let bits = br.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

/// One tabulated temperature row of a JANAF table. The temperature is always present; the other columns are
/// `Option` because the `T=0` row's Gibbs-energy function is `INFINITE` (absent) and a phase-transition marker row
/// replaces the formation columns with a text label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JanafRow {
    /// Temperature (K).
    pub t_k: Fixed,
    /// Heat capacity `Cp` (J/mol/K).
    pub cp: Option<Fixed>,
    /// Standard entropy `S(T)` (J/mol/K).
    pub s: Option<Fixed>,
    /// The Gibbs energy function `-[G(T)-H(Tr)]/T` (J/mol/K); `None` at `T=0` where the table prints `INFINITE`.
    pub gef: Option<Fixed>,
    /// The enthalpy increment `H(T)-H(Tr)` (kJ/mol).
    pub h_htr_kj: Option<Fixed>,
    /// The standard enthalpy of formation `delta-f H` (kJ/mol); `None` on a phase-transition marker row.
    pub dfh_kj: Option<Fixed>,
    /// The standard Gibbs energy of formation `delta-f G` (kJ/mol), the species' standard chemical potential at
    /// temperature; `None` on a phase-transition marker row.
    pub dfg_kj: Option<Fixed>,
    /// The phase-transition label (a lambda maximum, an `ALPHA <--> GAMMA` boundary, `TRANSITION`), if this row is a
    /// marker row rather than a data row; the temperature and function columns are still populated.
    pub marker: Option<String>,
}

/// One species' cited JANAF table.
#[derive(Debug, Clone)]
pub struct JanafSpecies {
    /// The species key (formula plus phase, e.g. `H2O(g)`).
    pub name: String,
    /// The chemical formula.
    pub formula: String,
    /// The phase (`g`, `cr`, `l`, `ref`).
    pub phase: String,
    /// The JANAF table code (the immutable source id, e.g. `H-064`).
    pub janaf_code: String,
    /// The Chase 1998 citation, required non-empty.
    pub citation: String,
    /// The tabulated rows in temperature order.
    pub rows: Vec<JanafRow>,
}

impl JanafSpecies {
    /// Parse one JANAF `<code>.txt` table into a species. The file is two header lines (a title line and a
    /// tab-delimited column-header line) then tab-delimited data rows `T Cp S -[G-H(Tr)]/T H-H(Tr) delta-f H
    /// delta-f G log Kf`. A line whose first cell is not a temperature is skipped (this drops both header lines and
    /// any stray line). The formation tail is split on whitespace so a row that packs `delta-f H`, `delta-f G`, and
    /// `log Kf` into one space-separated field near a transition parses the same as the tab-separated form; a tail
    /// whose first token is non-numeric (`Cp LAMBDA MAXIMUM`, `ALPHA <--> GAMMA`) is recorded as a marker with the
    /// formation columns absent. `log Kf` (the last column) is not retained: it is `-delta-f G / (R T ln 10)`,
    /// derivable from the retained `delta-f G`. `None` if the citation is empty or the table yields no rows.
    pub fn from_janaf_txt(
        name: &str,
        formula: &str,
        phase: &str,
        janaf_code: &str,
        citation: &str,
        content: &str,
    ) -> Option<JanafSpecies> {
        if citation.trim().is_empty() {
            return None;
        }
        let mut rows: Vec<JanafRow> = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let cells: Vec<&str> = line.split('\t').collect();
            // The first cell must be a temperature; a header line ("T(K)", "Water (H2O)") is not, and is skipped.
            let t_k = match parse_cell(cells[0]) {
                Some(t) => t,
                None => continue,
            };
            let cp = cells.get(1).and_then(|c| parse_cell(c));
            let s = cells.get(2).and_then(|c| parse_cell(c));
            let gef = cells.get(3).and_then(|c| parse_cell(c));
            let h_htr_kj = cells.get(4).and_then(|c| parse_cell(c));
            let (dfh_kj, dfg_kj, marker) = if cells.len() >= 6 {
                let tail = cells[5..].join(" ");
                let toks: Vec<&str> = tail.split_whitespace().collect();
                match toks.first() {
                    Some(first) => match parse_cell(first) {
                        Some(h) => {
                            let g = toks.get(1).and_then(|t| parse_cell(t));
                            (Some(h), g, None)
                        }
                        // A non-numeric tail is a phase-transition marker row.
                        None => (None, None, Some(tail.trim().to_string())),
                    },
                    None => (None, None, None),
                }
            } else {
                (None, None, None)
            };
            rows.push(JanafRow {
                t_k,
                cp,
                s,
                gef,
                h_htr_kj,
                dfh_kj,
                dfg_kj,
                marker,
            });
        }
        if rows.is_empty() {
            return None;
        }
        Some(JanafSpecies {
            name: name.to_string(),
            formula: formula.to_string(),
            phase: phase.to_string(),
            janaf_code: janaf_code.to_string(),
            citation: citation.to_string(),
            rows,
        })
    }

    /// The `delta-f G` (standard chemical potential, kJ/mol) at an exactly tabulated temperature, or `None` if that
    /// temperature is not a tabulated row or the row is a transition marker.
    pub fn delta_f_g_at(&self, t_k: Fixed) -> Option<Fixed> {
        self.rows
            .iter()
            .find(|r| r.t_k == t_k)
            .and_then(|r| r.dfg_kj)
    }

    /// The `delta-f H` (kJ/mol) at an exactly tabulated temperature.
    pub fn delta_f_h_at(&self, t_k: Fixed) -> Option<Fixed> {
        self.rows
            .iter()
            .find(|r| r.t_k == t_k)
            .and_then(|r| r.dfh_kj)
    }

    /// The Gibbs energy function `-[G(T)-H(Tr)]/T` (J/mol/K) at an exactly tabulated temperature.
    pub fn gef_at(&self, t_k: Fixed) -> Option<Fixed> {
        self.rows.iter().find(|r| r.t_k == t_k).and_then(|r| r.gef)
    }
}

/// The loaded JANAF library, keyed by species name.
#[derive(Debug, Clone, Default)]
pub struct JanafTables {
    species: BTreeMap<String, JanafSpecies>,
}

#[derive(Debug, Default, Deserialize)]
struct RawManifest {
    #[serde(default)]
    species: Vec<RawEntry>,
}

#[derive(Debug, Default, Deserialize)]
struct RawEntry {
    name: String,
    formula: String,
    phase: String,
    janaf_code: String,
    #[serde(default)]
    citation: String,
}

impl JanafTables {
    /// Load the standard JANAF library: the vendored per-species `.txt` tables joined to the fixture manifest
    /// (`name`/`formula`/`phase`/`janaf_code`/`citation`/`md5`) by name. Fail-closed on a manifest parse failure, a
    /// manifest entry with no embedded file, a species with no citation, a table with no rows, or a duplicate name.
    pub fn standard() -> Result<Self, JanafError> {
        let manifest: RawManifest =
            toml::from_str(JANAF_MANIFEST).map_err(|e| JanafError::Parse(e.to_string()))?;
        let mut species = BTreeMap::new();
        for entry in &manifest.species {
            if entry.citation.trim().is_empty() {
                return Err(JanafError::MissingCitation(entry.name.clone()));
            }
            let content = JANAF_DAT
                .iter()
                .find(|(n, _)| *n == entry.name)
                .map(|(_, c)| *c)
                .ok_or_else(|| JanafError::MissingFile(entry.name.clone()))?;
            let sp = JanafSpecies::from_janaf_txt(
                &entry.name,
                &entry.formula,
                &entry.phase,
                &entry.janaf_code,
                &entry.citation,
                content,
            )
            .ok_or_else(|| JanafError::Empty(entry.name.clone()))?;
            if species.insert(entry.name.clone(), sp).is_some() {
                return Err(JanafError::Duplicate(entry.name.clone()));
            }
        }
        Ok(JanafTables { species })
    }

    /// The cited table for a species, or `None` if it is not in the library (the caller escalates to the estimator).
    pub fn species(&self, name: &str) -> Option<&JanafSpecies> {
        self.species.get(name)
    }

    /// The species names in the library, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> + '_ {
        self.species.keys().map(String::as_str)
    }
}

/// The JANAF fixture manifest (name / formula / phase / janaf_code / citation / md5).
const JANAF_MANIFEST: &str = include_str!("../data/janaf/manifest.toml");

/// The embedded per-species JANAF tables, joined to the manifest by species name.
const JANAF_DAT: &[(&str, &str)] = &[
    ("H2(ref)", include_str!("../data/janaf/H-050.txt")),
    ("H2O(g)", include_str!("../data/janaf/H-064.txt")),
    ("CO(g)", include_str!("../data/janaf/C-093.txt")),
    ("CO2(g)", include_str!("../data/janaf/C-095.txt")),
    ("CH4(g)", include_str!("../data/janaf/C-067.txt")),
    ("N2(ref)", include_str!("../data/janaf/N-023.txt")),
    ("NH3(g)", include_str!("../data/janaf/H-083.txt")),
    ("O2(ref)", include_str!("../data/janaf/O-029.txt")),
    ("SiO(g)", include_str!("../data/janaf/O-012.txt")),
    ("TiO(g)", include_str!("../data/janaf/O-022.txt")),
    ("S2(g)", include_str!("../data/janaf/S-012.txt")),
    ("H2S(g)", include_str!("../data/janaf/H-080.txt")),
    ("Mg(g)", include_str!("../data/janaf/Mg-005.txt")),
    ("Fe(g)", include_str!("../data/janaf/Fe-008.txt")),
    ("SiS(g)", include_str!("../data/janaf/S-009.txt")),
    ("Na(g)", include_str!("../data/janaf/Na-005.txt")),
    ("K(g)", include_str!("../data/janaf/K-005.txt")),
    (
        "Al2O3(cr,corundum)",
        include_str!("../data/janaf/Al-096.txt"),
    ),
    (
        "MgAl2O4(cr,spinel)",
        include_str!("../data/janaf/Al-089.txt"),
    ),
    (
        "Mg2SiO4(cr,forsterite)",
        include_str!("../data/janaf/Mg-028.txt"),
    ),
    (
        "MgSiO3(cr,enstatite)",
        include_str!("../data/janaf/Mg-012.txt"),
    ),
    ("Fe(cr)", include_str!("../data/janaf/Fe-004.txt")),
    ("FeS(cr,troilite)", include_str!("../data/janaf/Fe-023.txt")),
    ("H2O(l)", include_str!("../data/janaf/H-063.txt")),
    ("Ni(cr)", include_str!("../data/janaf/Ni-002.txt")),
    // The aluminium and calcium gas carriers (the refractory-condensate deepening: with Al and Ca gas-balanceable,
    // corundum and spinel condense, the CAI-first head of the condensation sequence). NIST-JANAF (Chase 1998).
    ("Al(g)", include_str!("../data/janaf/Al-005.txt")),
    ("AlOH(g)", include_str!("../data/janaf/Al-058.txt")),
    ("AlO(g)", include_str!("../data/janaf/Al-074.txt")),
    ("Al2O(g)", include_str!("../data/janaf/Al-092.txt")),
    ("AlH(g)", include_str!("../data/janaf/Al-056.txt")),
    ("Ca(g)", include_str!("../data/janaf/Ca-006.txt")),
    ("CaOH(g)", include_str!("../data/janaf/Ca-018.txt")),
    ("Ca(OH)2(g)", include_str!("../data/janaf/Ca-021.txt")),
    ("CaO(g)", include_str!("../data/janaf/Ca-030.txt")),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> JanafTables {
        JanafTables::standard().expect("the JANAF library loads")
    }

    /// The row whose temperature is closest to `target_k` (a test helper for the reference checks).
    fn nearest(sp: &JanafSpecies, target_k: f64) -> &JanafRow {
        sp.rows
            .iter()
            .min_by(|a, b| {
                let da = (a.t_k.to_f64_lossy() - target_k).abs();
                let db = (b.t_k.to_f64_lossy() - target_k).abs();
                da.partial_cmp(&db).unwrap()
            })
            .unwrap()
    }

    #[test]
    fn the_library_loads_all_cited_species() {
        let l = lib();
        // 25 original + the 9 Al/Ca gas carriers (the refractory-condensate deepening: Al lets corundum and spinel
        // condense, the CAI-first head of the sequence). A new cited species is one more manifest row.
        assert_eq!(l.names().count(), 34, "34 cited species");
        for name in l.names() {
            let sp = l.species(name).unwrap();
            assert!(!sp.citation.trim().is_empty(), "{name} carries a citation");
            assert!(sp.rows.len() > 5, "{name} has a real table");
            assert!(
                sp.citation.contains("Chase"),
                "{name} is cited to Chase 1998"
            );
        }
    }

    #[test]
    fn the_formation_enthalpies_re_verify_against_janaf() {
        // The standing re-verification gate (the optical-constants peak-gate pattern): the loaded delta-f H at
        // 298.15 K reproduce the JANAF 4th-ed values, so a corruption of a fetched table fails the build. Each value
        // is the species' table's own 298.15 K formation enthalpy (kJ/mol).
        let l = lib();
        let cases = [
            ("H2O(g)", -241.826),
            ("CO2(g)", -393.522),
            ("CO(g)", -110.527),
            ("CH4(g)", -74.873),
            ("NH3(g)", -45.898),
            ("H2S(g)", -20.502),
            ("SiO(g)", -100.416),
            ("Al2O3(cr,corundum)", -1675.692),
            ("MgAl2O4(cr,spinel)", -2299.108),
            ("Mg2SiO4(cr,forsterite)", -2176.935),
            ("MgSiO3(cr,enstatite)", -1548.917),
            ("FeS(cr,troilite)", -101.671),
            ("H2O(l)", -285.830),
        ];
        for (name, want) in cases {
            let sp = l.species(name).unwrap_or_else(|| panic!("{name} present"));
            let row = nearest(sp, 298.15);
            let got = row
                .dfh_kj
                .unwrap_or_else(|| panic!("{name} has a 298.15 K formation enthalpy"));
            assert!(
                (got.to_f64_lossy() - want).abs() < 1e-2,
                "{name} delta-f H(298.15) is {want} kJ/mol, got {}",
                got.to_f64_lossy()
            );
        }
    }

    #[test]
    fn the_elements_have_zero_formation_energy() {
        // A reference-state element has delta-f H = delta-f G = 0 at 298.15 K by definition (the honesty check that
        // the reference rows loaded as data, not as markers).
        let l = lib();
        for name in ["Fe(cr)", "Ni(cr)", "H2(ref)", "N2(ref)", "O2(ref)"] {
            let sp = l.species(name).unwrap();
            let row = nearest(sp, 298.15);
            assert_eq!(
                row.dfh_kj.map(|v| v.to_f64_lossy().abs() < 1e-6),
                Some(true),
                "{name} delta-f H(298.15) is 0"
            );
        }
    }

    #[test]
    fn the_t_zero_gibbs_function_is_absent() {
        // The T=0 row prints INFINITE for the Gibbs energy function; the loader stores that as an absent value
        // rather than a fabricated number.
        let l = lib();
        let w = l.species("H2O(g)").unwrap();
        let row0 = w.rows.iter().find(|r| r.t_k == Fixed::ZERO).unwrap();
        assert_eq!(
            row0.gef, None,
            "the T=0 Gibbs function (INFINITE) is absent"
        );
        // A finite-temperature row does carry the function.
        assert!(
            w.gef_at(Fixed::from_ratio(29815, 100)).is_some(),
            "the 298.15 K Gibbs function is present"
        );
    }

    #[test]
    fn a_phase_transition_row_is_a_marker_not_a_data_row() {
        // Iron has an ALPHA <--> GAMMA transition; that row carries a text marker in place of the formation columns,
        // so its temperature and function survive but its formation columns are absent (never parsed as a number).
        let l = lib();
        let fe = l.species("Fe(cr)").unwrap();
        let markers: Vec<&str> = fe.rows.iter().filter_map(|r| r.marker.as_deref()).collect();
        assert!(
            markers.iter().any(|m| m.contains("ALPHA")),
            "iron carries an ALPHA <--> GAMMA marker row, got {markers:?}"
        );
        for r in &fe.rows {
            if r.marker.is_some() {
                assert!(
                    r.dfh_kj.is_none() && r.dfg_kj.is_none(),
                    "a marker row has no formation columns"
                );
                assert!(r.cp.is_some(), "a marker row keeps its temperature columns");
            }
        }
    }

    #[test]
    fn delta_f_g_lookup_hits_the_tabulated_row() {
        // The standard chemical potential lookup returns the delta-f G at an exactly tabulated temperature.
        let l = lib();
        let w = l.species("H2O(g)").unwrap();
        let g = w.delta_f_g_at(Fixed::from_ratio(29815, 100)).unwrap();
        assert!(
            (g.to_f64_lossy() - -228.582).abs() < 1e-2,
            "H2O(g) delta-f G(298.15) is -228.582 kJ/mol, got {}",
            g.to_f64_lossy()
        );
        assert_eq!(
            w.delta_f_g_at(Fixed::from_int(12345)),
            None,
            "an untabulated temperature returns None"
        );
    }

    #[test]
    fn a_missing_citation_fails_closed() {
        let bad = r#"
[[species]]
name = "H2O(g)"
formula = "H2O"
phase = "g"
janaf_code = "H-064"
citation = ""
"#;
        let manifest: RawManifest = toml::from_str(bad).unwrap();
        assert!(manifest.species[0].citation.is_empty());
        // The loader rejects an empty citation.
        let sp = JanafSpecies::from_janaf_txt("H2O(g)", "H2O", "g", "H-064", "", "0\t0.\t0.");
        assert!(sp.is_none(), "an empty citation fails closed");
    }
}
