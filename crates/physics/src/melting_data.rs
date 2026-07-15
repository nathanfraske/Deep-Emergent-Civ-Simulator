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

//! The cited ENDMEMBER MELTING registry (`data/melting_endmembers.toml`): one row per silicate or oxide
//! endmember carrying the two-number (plus fusion-volume) melting signature the [`crate::melting`] rung reads,
//! `(T_m, dH_fus, dV_fus)`. This is a cited `[M]` data column (the melting point and the enthalpy of fusion
//! are MEASURED quantities), so it wears its OWN block kind (`[[endmember]]`, header-cited), out of the
//! floor-`[[element]]` real/fantasy policing: a citation file is an immutable transcription of its source,
//! never a home for the authorship axis (the citation-file-immutability doctrine).
//!
//! The mechanism (the loader and the melt rung it feeds) is fixed Rust; the membership is data and grows with
//! the world (Principle 11), a sibling of the phase registry ([`crate::petrology_data`]). An alien crust is a
//! new row, not a rewrite: the wiring keys off each phase's own `Endmember` signature (admit-the-alien).
//!
//! Every value is fixed-point ([`Fixed`]), parsed from a decimal string by integer arithmetic (no float on the
//! canonical path), and carries its own citation. Two honest edges are declared per row rather than hidden: an
//! INCONGRUENT melter (enstatite melts to forsterite plus liquid at a peritectic near 1830 K at 1 bar) carries
//! `congruent = false` and its peritectic temperature as the ideal pseudo-endmember signature, never a
//! fabricated congruent value; a fusion VOLUME with no clean primary source is left empty, which loads as zero
//! (the pressure-insensitive surface-only rung) with `fusion_volume_sourced = false`, flagged rather than
//! invented.

use crate::melting::Endmember;
use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// One endmember's cited melting row. The `endmember()` accessor projects it to the melt-rung [`Endmember`]
/// signature the liquidus, eutectic, and melt-fraction machinery reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeltingEndmember {
    /// The endmember's mineral name, for example `forsterite`.
    pub name: String,
    /// The chemical formula as written, for example `Mg2SiO4` (the lookup key and the verbatim provenance).
    pub formula: String,
    /// The pure-endmember (or, for an incongruent melter, the peritectic) melting temperature `T_m`, kelvin.
    pub melting_point_k: Fixed,
    /// The raw decimal string of the melting temperature, retained verbatim as provenance.
    pub melting_point_decimal: String,
    /// The molar enthalpy of fusion `dH_fus`, joules per mole.
    pub fusion_enthalpy_j_per_mol: Fixed,
    /// The raw decimal string of the fusion enthalpy.
    pub fusion_enthalpy_decimal: String,
    /// The molar volume change on fusion `dV_fus`, cubic centimetres per mole (the petrology convention),
    /// zero when unsourced (the pressure-insensitive surface-only rung).
    pub fusion_volume_cm3_per_mol: Fixed,
    /// The raw decimal string of the fusion volume; empty when unsourced.
    pub fusion_volume_decimal: String,
    /// Whether the phase melts CONGRUENTLY at one bar. `false` marks an incongruent (peritectic) melter, whose
    /// `melting_point_k` is the peritectic temperature used as the ideal pseudo-endmember signature (labelled).
    pub congruent: bool,
    /// Whether the fusion volume is cited. `false` marks a fusion volume with no primary source, set to zero
    /// (the surface-only rung) and flagged rather than fabricated.
    pub fusion_volume_sourced: bool,
    /// The citation for the melting point and fusion enthalpy (real-with-source, one row per primary source).
    pub source: String,
    /// The citation for the fusion volume when it is sourced; empty when the volume is unsourced (zeroed).
    pub fusion_volume_source: String,
}

impl MeltingEndmember {
    /// The melt-rung [`Endmember`] signature this row carries: the three numbers the liquidus, eutectic, and
    /// pressure-shift machinery reads, with nothing added.
    pub fn endmember(&self) -> Endmember {
        Endmember {
            melting_point_k: self.melting_point_k,
            fusion_enthalpy_j_per_mol: self.fusion_enthalpy_j_per_mol,
            fusion_volume_cm3_per_mol: self.fusion_volume_cm3_per_mol,
        }
    }
}

/// What can go wrong loading or reading the melting registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeltingError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// The data file could not be read.
    Io(String),
    /// A formula appears twice (the lookup key must be unique).
    DuplicateFormula(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue {
        /// The endmember the value belongs to.
        name: String,
        /// What went wrong.
        detail: String,
    },
    /// An endmember carries no citation (every row is real-with-source).
    MissingSource(String),
    /// A physical-validity gate failed (a non-positive melting point, or a negative fusion enthalpy the melt
    /// rung's liquidus rejects).
    Unphysical {
        /// The endmember.
        name: String,
        /// What is wrong.
        detail: String,
    },
}

impl fmt::Display for MeltingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeltingError::Parse(m) => write!(f, "melting-registry parse error: {m}"),
            MeltingError::Io(m) => write!(f, "melting-registry read error: {m}"),
            MeltingError::DuplicateFormula(x) => write!(f, "duplicate endmember formula '{x}'"),
            MeltingError::BadValue { name, detail } => {
                write!(f, "value in endmember '{name}' could not be read: {detail}")
            }
            MeltingError::MissingSource(n) => {
                write!(
                    f,
                    "endmember '{n}' must declare a citation (real-with-source)"
                )
            }
            MeltingError::Unphysical { name, detail } => {
                write!(f, "endmember '{name}' is unphysical: {detail}")
            }
        }
    }
}

impl std::error::Error for MeltingError {}

/// The loaded melting registry: the endmembers keyed by FORMULA (the physically-identifying key, matching the
/// condensation species' formula) in a sorted map so any walk is in a fixed canonical order (determinism).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MeltingRegistry {
    by_formula: BTreeMap<String, MeltingEndmember>,
}

impl MeltingRegistry {
    /// Parse and validate a registry from TOML text. Each row passes the citation-required and the
    /// physical-validity gates (a non-positive melting point or a negative fusion enthalpy fails loud), so a
    /// hallucinated or corrupt value is caught at load rather than sorting silently into the melt solve.
    pub fn from_toml_str(s: &str) -> Result<Self, MeltingError> {
        let file: RegistryFile =
            toml::from_str(s).map_err(|e| MeltingError::Parse(e.to_string()))?;
        let mut registry = MeltingRegistry::default();
        for row in file.endmember {
            let em = row.into_endmember()?;
            if registry.by_formula.contains_key(&em.formula) {
                return Err(MeltingError::DuplicateFormula(em.formula));
            }
            registry.by_formula.insert(em.formula.clone(), em);
        }
        Ok(registry)
    }

    /// Load and validate a registry from a file path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, MeltingError> {
        let text = std::fs::read_to_string(path).map_err(|e| MeltingError::Io(e.to_string()))?;
        Self::from_toml_str(&text)
    }

    /// The embedded standard registry, built from the crate's embedded data so a caller needs no filesystem
    /// path.
    pub fn standard() -> Result<Self, MeltingError> {
        Self::from_toml_str(include_str!("../data/melting_endmembers.toml"))
    }

    /// The cited melting row for a formula, for example `Mg2SiO4`.
    pub fn by_formula(&self, formula: &str) -> Option<&MeltingEndmember> {
        self.by_formula.get(formula)
    }

    /// The melt-rung [`Endmember`] signature for a formula, or `None` if the formula is not a carried row.
    pub fn endmember_for_formula(&self, formula: &str) -> Option<Endmember> {
        self.by_formula
            .get(formula)
            .map(MeltingEndmember::endmember)
    }

    /// The melt-rung [`Endmember`] for a condensation SPECIES NAME (the formula before its `(cr,...)` or `(l)`
    /// phase suffix), so a caller looks up straight from the differentiation's floating-phase name. `None` if
    /// the species name has no formula or the formula is not a carried row (the fail-soft buoyancy fallback the
    /// wiring names).
    pub fn endmember_for_species(&self, species_name: &str) -> Option<Endmember> {
        let formula = species_name.split('(').next()?;
        self.endmember_for_formula(formula)
    }

    /// The endmembers, in sorted formula order.
    pub fn endmembers(&self) -> impl Iterator<Item = &MeltingEndmember> + '_ {
        self.by_formula.values()
    }

    /// The number of endmembers.
    pub fn len(&self) -> usize {
        self.by_formula.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.by_formula.is_empty()
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    endmember: Vec<EndmemberDef>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct EndmemberDef {
    name: String,
    formula: String,
    melting_point_k: String,
    fusion_enthalpy_j_per_mol: String,
    /// The fusion volume, cm^3/mol; empty declares it UNSOURCED (zeroed, the surface-only rung), flagged.
    #[serde(default)]
    fusion_volume_cm3_per_mol: String,
    /// Whether the phase melts congruently at 1 bar; defaults to true, set false for an incongruent melter.
    #[serde(default = "default_true")]
    congruent: bool,
    /// The citation for the melting point and fusion enthalpy.
    #[serde(default)]
    source: String,
    /// The citation for the fusion volume; empty when the volume is unsourced.
    #[serde(default)]
    fusion_volume_source: String,
}

fn default_true() -> bool {
    true
}

impl EndmemberDef {
    fn into_endmember(self) -> Result<MeltingEndmember, MeltingError> {
        let read = |field: &str, raw: &str| -> Result<Fixed, MeltingError> {
            Fixed::from_decimal_str(raw.trim()).map_err(|detail| MeltingError::BadValue {
                name: self.name.clone(),
                detail: format!("{field}: {detail}"),
            })
        };
        if self.source.trim().is_empty() {
            return Err(MeltingError::MissingSource(self.name.clone()));
        }
        let melting_point_k = read("melting_point_k", &self.melting_point_k)?;
        let fusion_enthalpy_j_per_mol =
            read("fusion_enthalpy_j_per_mol", &self.fusion_enthalpy_j_per_mol)?;
        // The physical-validity gate: the melt rung's liquidus rejects a non-positive melting point or a
        // negative fusion enthalpy, so reject them here at load (fail-loud on a corrupt or hallucinated value).
        if melting_point_k <= Fixed::ZERO {
            return Err(MeltingError::Unphysical {
                name: self.name.clone(),
                detail: "the melting point must be positive".to_string(),
            });
        }
        if fusion_enthalpy_j_per_mol < Fixed::ZERO {
            return Err(MeltingError::Unphysical {
                name: self.name.clone(),
                detail: "the fusion enthalpy must be non-negative".to_string(),
            });
        }
        // The fusion volume: empty declares it unsourced, loaded as zero (the pressure-insensitive surface-only
        // rung), flagged rather than fabricated.
        let vol_raw = self.fusion_volume_cm3_per_mol.trim();
        let (fusion_volume_cm3_per_mol, fusion_volume_decimal, fusion_volume_sourced) =
            if vol_raw.is_empty() {
                (Fixed::ZERO, String::new(), false)
            } else {
                (
                    read("fusion_volume_cm3_per_mol", vol_raw)?,
                    vol_raw.to_string(),
                    true,
                )
            };
        Ok(MeltingEndmember {
            name: self.name.trim().to_string(),
            formula: self.formula.trim().to_string(),
            melting_point_k,
            melting_point_decimal: self.melting_point_k.trim().to_string(),
            fusion_enthalpy_j_per_mol,
            fusion_enthalpy_decimal: self.fusion_enthalpy_j_per_mol.trim().to_string(),
            fusion_volume_cm3_per_mol,
            fusion_volume_decimal,
            congruent: self.congruent,
            fusion_volume_sourced,
            source: self.source.trim().to_string(),
            fusion_volume_source: self.fusion_volume_source.trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> MeltingRegistry {
        MeltingRegistry::standard().expect("the embedded melting registry loads")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_seed_endmembers_load_with_their_signatures() {
        let r = registry();
        assert!(
            r.len() >= 4,
            "the seed covers at least the fertile-crust formers"
        );
        let fo = r.by_formula("Mg2SiO4").expect("forsterite is seeded");
        assert_eq!(fo.name, "forsterite");
        assert!(fo.congruent, "forsterite melts congruently");
        assert!(
            close(fo.melting_point_k, 2163.0, 5.0),
            "forsterite T_m ~ 2163 K, got {}",
            fo.melting_point_k.to_f64_lossy()
        );
        assert!(
            fo.fusion_enthalpy_j_per_mol.to_f64_lossy() > 0.0,
            "the fusion enthalpy is positive"
        );
        assert!(!fo.source.is_empty(), "every row carries a citation");
    }

    #[test]
    fn the_incongruent_enstatite_is_labelled() {
        let r = registry();
        let en = r.by_formula("MgSiO3").expect("enstatite is seeded");
        assert!(
            !en.congruent,
            "enstatite is an incongruent (peritectic) melter, labelled not fabricated congruent"
        );
    }

    #[test]
    fn the_species_lookup_strips_the_phase_suffix() {
        let r = registry();
        // The condensation names its phases FORMULA(cr,mineral); the lookup reads the formula before the suffix.
        let em = r
            .endmember_for_species("Mg2SiO4(cr,forsterite)")
            .expect("the forsterite species resolves to its endmember");
        assert!(close(em.melting_point_k, 2163.0, 5.0));
        assert!(
            r.endmember_for_species("Fe(cr)").is_none()
                || r.endmember_for_species("XyZ(cr)").is_none(),
            "a phase with no melting row resolves to None (the fail-soft buoyancy fallback)"
        );
    }

    #[test]
    fn a_row_without_a_citation_fails_to_load() {
        let no_src = r#"
[[endmember]]
name = "forsterite"
formula = "Mg2SiO4"
melting_point_k = "2163"
fusion_enthalpy_j_per_mol = "114000"
"#;
        assert_eq!(
            MeltingRegistry::from_toml_str(no_src).unwrap_err(),
            MeltingError::MissingSource("forsterite".to_string())
        );
    }

    #[test]
    fn a_negative_fusion_enthalpy_fails_the_physical_gate() {
        let bad = r#"
[[endmember]]
name = "forsterite"
formula = "Mg2SiO4"
melting_point_k = "2163"
fusion_enthalpy_j_per_mol = "-114000"
source = "test"
"#;
        assert!(matches!(
            MeltingRegistry::from_toml_str(bad).unwrap_err(),
            MeltingError::Unphysical { .. }
        ));
    }

    #[test]
    fn an_unsourced_fusion_volume_loads_as_zero_and_is_flagged() {
        let no_vol = r#"
[[endmember]]
name = "testmineral"
formula = "Xy2SiO4"
melting_point_k = "2000"
fusion_enthalpy_j_per_mol = "100000"
source = "test row for the unsourced-volume path"
"#;
        let r = MeltingRegistry::from_toml_str(no_vol).expect("loads");
        let em = r.by_formula("Xy2SiO4").expect("the test row is present");
        assert_eq!(em.fusion_volume_cm3_per_mol, Fixed::ZERO);
        assert!(
            !em.fusion_volume_sourced,
            "an unsourced fusion volume is flagged, zeroed (the surface-only rung), never fabricated"
        );
    }

    #[test]
    fn the_seeded_forsterite_reproduces_its_clapeyron_slope() {
        // A coherence cross-check: the loaded forsterite signature (T_m 2163, dH_fus 142000, dV_fus 3.84), fed
        // the melt rung's own Clapeyron law, reproduces the measured fusion-curve slope (~48-63 K/GPa; Davis &
        // England 1964 / Ghiorso 2004), so a corrupt T_m/dH/dV triple is caught by a physical consequence rather
        // than only by its citation. Forsterite is the clean check (its fusion volume is derived directly from
        // the fusion-curve slope); diopside's larger ~19 percent fusion volume gives a steeper ~150 K/GPa.
        let r = registry();
        let fo = r.by_formula("Mg2SiO4").expect("forsterite is seeded");
        let em = fo.endmember();
        let t0 = crate::melting::melting_point_at_pressure(em, Fixed::ZERO).unwrap();
        let t1 = crate::melting::melting_point_at_pressure(em, Fixed::from_int(10_000)).unwrap();
        let slope = t1.to_f64_lossy() - t0.to_f64_lossy();
        assert!(
            (40.0..=80.0).contains(&slope),
            "the loaded forsterite reproduces its ~58 K/GPa fusion-curve slope, got {slope} K/GPa"
        );
    }
}
