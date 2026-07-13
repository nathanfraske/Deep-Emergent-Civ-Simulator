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

//! The candidate-phase thermodynamic registry (`data/phase_registry.toml`): one entry per candidate solid
//! phase (mineral, oxide, or melt end-member) carrying the standard-state thermodynamic primitives a
//! free-energy-minimizing petrology kernel reads to decide the stable assemblage for a bulk composition at a
//! given pressure and temperature. This is the per-PHASE sibling of the per-ELEMENT periodic table: a pure
//! element's Gibbs energy of formation is zero by definition, so formation energies live here, keyed to a
//! phase, not to an element.
//!
//! The mechanism (the loader and, later, the Gibbs-minimization kernel) is fixed Rust; the phase MEMBERSHIP
//! is data and grows with the world (Principle 11). It is an extensible registry, a sibling of the fifteen
//! reference substances and the periodic table, NOT a closed enum of Earth minerals: an alien phase in an
//! alien chemistry is a new row, not a rewrite. The registry seeds the Mg-Fe-Si-Al-O oxide-and-silicate core
//! of a bulk-silicate world; the Stage-3 petrology kernel grows the membership to the phases a given world's
//! composition and pressure-temperature field reach.
//!
//! Every value is fixed-point ([`Fixed`]), parsed from a decimal string by integer arithmetic (no float in
//! canonical state), and carries its own citation (real-with-source, the same discipline as the atomic
//! weight and the standard molar entropy). A thermodynamic subtlety the calibration must honor: a
//! free-energy minimization is only correct when all competing phases come from ONE internally consistent
//! dataset (the relative energies decide the assemblage, so mixing datasets corrupts the comparison). The
//! seed cites a single reference set and the owner pins the production dataset before the kernel runs. No
//! consumer is wired to this registry yet; it is a pure addition.

use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// One candidate phase's thermodynamic row. The three standard-state primitives a Gibbs-energy read needs
/// are the enthalpy of formation, the standard molar entropy, and the molar volume; the pressure-temperature
/// dependence (heat-capacity coefficients, thermal expansion, bulk modulus, and a phase-transition Clapeyron
/// slope) grows onto the row as the kernel needs it, so those fields are optional and absent-not-zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase {
    /// The phase name, for example `quartz`.
    pub name: String,
    /// The chemical formula as written, for example `SiO2` (the verbatim provenance record).
    pub formula: String,
    /// The parsed composition as `(element symbol, count)` pairs in the order written, for the molar-mass
    /// cross-check and the stoichiometry the kernel reads.
    pub composition: Vec<(String, u32)>,
    /// The standard enthalpy of formation from the elements, in kJ/mol (negative for a stable phase).
    pub enthalpy_formation: Fixed,
    /// The raw decimal string of the enthalpy of formation, retained verbatim as provenance.
    pub enthalpy_decimal: String,
    /// The standard molar entropy S at 298.15 K, in J/(mol K).
    pub standard_entropy: Fixed,
    /// The raw decimal string of the standard molar entropy.
    pub entropy_decimal: String,
    /// The molar volume V at the standard state, in cm^3/mol.
    pub molar_volume: Fixed,
    /// The raw decimal string of the molar volume.
    pub volume_decimal: String,
    /// The Clapeyron slope dP/dT of the phase's defining transition, in MPa/K, when the phase is defined by a
    /// solid-solid transition; `None` for a phase carried without one yet (the field grows onto the row).
    pub clapeyron_slope: Option<Fixed>,
    /// The raw decimal string of the Clapeyron slope; `None` when absent.
    pub clapeyron_decimal: Option<String>,
    /// The STRUCTURE-PROTOTYPE key (the aristotype), naming the structure type the phase crystallizes in (for
    /// example `rock-salt` for periclase). This is the data-driven bonding-class dispatch (owner research,
    /// #182): a phase carrying a prototype key that maps to a seeded ionic prototype derives its bulk modulus
    /// from lattice curvature (the Madelung constant and coordination the prototype supplies); a phase without a
    /// key, or whose prototype is unseeded, falls through to the cohesive-energy-density screen tier, an honest
    /// fall-through never a forced ionic formula. `None` for a phase not yet keyed (the extensible registry).
    pub prototype: Option<String>,
    /// The citation for this phase's thermodynamic data (real-with-source; one internally consistent dataset).
    pub source: String,
}

/// What can go wrong loading or reading the phase registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// The data file could not be read.
    Io(String),
    /// A phase name appears twice.
    DuplicateName(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue {
        /// The phase the value belongs to.
        name: String,
        /// What went wrong.
        detail: String,
    },
    /// A formula could not be parsed into an element-and-count composition.
    BadFormula {
        /// The phase the formula belongs to.
        name: String,
        /// Why the formula did not parse.
        detail: String,
    },
    /// A phase carries no citation (every row must be real-with-source).
    MissingSource(String),
    /// A phase's formula names an element not in the periodic table (the cross-check).
    UnknownElement {
        /// The phase.
        name: String,
        /// The unknown element symbol.
        symbol: String,
    },
}

impl fmt::Display for PhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseError::Parse(m) => write!(f, "phase-registry parse error: {m}"),
            PhaseError::Io(m) => write!(f, "phase-registry read error: {m}"),
            PhaseError::DuplicateName(n) => write!(f, "duplicate phase name '{n}'"),
            PhaseError::BadValue { name, detail } => {
                write!(f, "value in phase '{name}' could not be read: {detail}")
            }
            PhaseError::BadFormula { name, detail } => {
                write!(f, "formula of phase '{name}' could not be parsed: {detail}")
            }
            PhaseError::MissingSource(n) => {
                write!(f, "phase '{n}' must declare a citation (real-with-source)")
            }
            PhaseError::UnknownElement { name, symbol } => write!(
                f,
                "phase '{name}' names element '{symbol}', which is not in the periodic table"
            ),
        }
    }
}

impl std::error::Error for PhaseError {}

/// Parse a simple chemical formula (an element symbol is an uppercase letter and any following lowercase
/// letters, an optional count follows, a bare symbol is a count of one) into `(symbol, count)` pairs.
/// Parentheses and hydration dots are not yet supported and fail loud (the registry grows to them when a
/// phase needs them); an empty or malformed formula fails loud.
fn parse_formula(formula: &str) -> Result<Vec<(String, u32)>, String> {
    let chars: Vec<char> = formula.chars().collect();
    let mut out: Vec<(String, u32)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !c.is_ascii_uppercase() {
            return Err(format!(
                "expected an element symbol (an uppercase letter) at position {i} in '{formula}'"
            ));
        }
        let mut symbol = String::new();
        symbol.push(c);
        i += 1;
        while i < chars.len() && chars[i].is_ascii_lowercase() {
            symbol.push(chars[i]);
            i += 1;
        }
        let mut digits = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            digits.push(chars[i]);
            i += 1;
        }
        let count: u32 = if digits.is_empty() {
            1
        } else {
            digits
                .parse()
                .map_err(|_| format!("could not read the count '{digits}' in '{formula}'"))?
        };
        out.push((symbol, count));
    }
    if out.is_empty() {
        return Err(format!("the formula '{formula}' is empty"));
    }
    Ok(out)
}

/// The loaded phase registry: the phases keyed by name in a sorted map so any walk is in a fixed canonical
/// order (the determinism discipline).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhaseRegistry {
    phases: BTreeMap<String, Phase>,
}

impl PhaseRegistry {
    /// Parse and validate a registry from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, PhaseError> {
        let file: RegistryFile = toml::from_str(s).map_err(|e| PhaseError::Parse(e.to_string()))?;
        let mut registry = PhaseRegistry::default();
        for p in file.phase {
            let phase = p.into_phase()?;
            if registry.phases.contains_key(&phase.name) {
                return Err(PhaseError::DuplicateName(phase.name));
            }
            registry.phases.insert(phase.name.clone(), phase);
        }
        Ok(registry)
    }

    /// Load and validate a registry from a file path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PhaseError> {
        let text = std::fs::read_to_string(path).map_err(|e| PhaseError::Io(e.to_string()))?;
        Self::from_toml_str(&text)
    }

    /// The embedded standard registry, built from the crate's embedded data so a caller needs no filesystem
    /// path.
    pub fn standard() -> Result<Self, PhaseError> {
        Self::from_toml_str(include_str!("../data/phase_registry.toml"))
    }

    /// A phase by name.
    pub fn phase(&self, name: &str) -> Option<&Phase> {
        self.phases.get(name)
    }

    /// The phases, in sorted name order.
    pub fn phases(&self) -> impl Iterator<Item = &Phase> + '_ {
        self.phases.values()
    }

    /// The number of phases.
    pub fn len(&self) -> usize {
        self.phases.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.phases.is_empty()
    }

    /// Validate every phase's formula against a periodic table (every named element must exist), so a typo in
    /// a formula fails loud rather than silently dropping mass. Returns the first offending phase, or `Ok` if
    /// all resolve.
    pub fn validate_against(&self, table: &PeriodicTable) -> Result<(), PhaseError> {
        for phase in self.phases.values() {
            for (symbol, _count) in &phase.composition {
                if table.element(symbol).is_none() {
                    return Err(PhaseError::UnknownElement {
                        name: phase.name.clone(),
                        symbol: symbol.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    phase: Vec<PhaseDef>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PhaseDef {
    name: String,
    formula: String,
    enthalpy_formation: String,
    standard_entropy: String,
    molar_volume: String,
    /// The Clapeyron slope in MPa/K; empty when the phase carries none yet.
    #[serde(default)]
    clapeyron_slope: String,
    /// The structure-prototype (aristotype) key; empty when the phase carries none yet.
    #[serde(default)]
    prototype: String,
    /// The citation (every phase is real-with-source).
    #[serde(default)]
    source: String,
}

impl PhaseDef {
    fn into_phase(self) -> Result<Phase, PhaseError> {
        let read = |field: &str, raw: &str| -> Result<Fixed, PhaseError> {
            Fixed::from_decimal_str(raw.trim()).map_err(|detail| PhaseError::BadValue {
                name: self.name.clone(),
                detail: format!("{field}: {detail}"),
            })
        };
        let enthalpy_formation = read("enthalpy_formation", &self.enthalpy_formation)?;
        let standard_entropy = read("standard_entropy", &self.standard_entropy)?;
        let molar_volume = read("molar_volume", &self.molar_volume)?;
        let clapeyron_raw = self.clapeyron_slope.trim();
        let (clapeyron_slope, clapeyron_decimal) = if clapeyron_raw.is_empty() {
            (None, None)
        } else {
            (
                Some(read("clapeyron_slope", clapeyron_raw)?),
                Some(clapeyron_raw.to_string()),
            )
        };
        let prototype_raw = self.prototype.trim();
        let prototype = if prototype_raw.is_empty() {
            None
        } else {
            Some(prototype_raw.to_string())
        };
        if self.source.trim().is_empty() {
            return Err(PhaseError::MissingSource(self.name.clone()));
        }
        let composition =
            parse_formula(self.formula.trim()).map_err(|detail| PhaseError::BadFormula {
                name: self.name.clone(),
                detail,
            })?;
        Ok(Phase {
            name: self.name.clone(),
            formula: self.formula.trim().to_string(),
            composition,
            enthalpy_formation,
            enthalpy_decimal: self.enthalpy_formation.trim().to_string(),
            standard_entropy,
            entropy_decimal: self.standard_entropy.trim().to_string(),
            molar_volume,
            volume_decimal: self.molar_volume.trim().to_string(),
            clapeyron_slope,
            clapeyron_decimal,
            prototype,
            source: self.source.trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> PhaseRegistry {
        PhaseRegistry::standard().expect("the embedded phase registry loads")
    }

    fn close(a: Fixed, b: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < 1e-2
    }

    #[test]
    fn the_seed_phases_load_with_their_thermodynamics() {
        let r = registry();
        assert!(
            r.len() >= 5,
            "the seed covers the Mg-Fe-Si-Al-O oxide-silicate core"
        );
        let quartz = r.phase("quartz").expect("quartz is seeded");
        assert_eq!(quartz.formula, "SiO2");
        assert_eq!(
            quartz.composition,
            vec![("Si".to_string(), 1), ("O".to_string(), 2)]
        );
        assert!(
            close(quartz.standard_entropy, 41.46),
            "quartz S is ~41.46 J/mol/K"
        );
        assert!(
            close(quartz.molar_volume, 22.688),
            "quartz V is ~22.69 cm3/mol"
        );
        assert!(
            quartz.enthalpy_formation.to_f64_lossy() < 0.0,
            "a stable phase has a negative enthalpy of formation"
        );
        assert!(!quartz.source.is_empty(), "every phase carries a citation");
    }

    #[test]
    fn the_registry_resolves_against_the_periodic_table() {
        let r = registry();
        let t = PeriodicTable::standard().expect("the table loads");
        r.validate_against(&t)
            .expect("every seeded formula names elements in the table");
    }

    #[test]
    fn a_phase_without_its_citation_fails_to_load() {
        let no_src = r#"
[[phase]]
name = "quartz"
formula = "SiO2"
enthalpy_formation = "-910.70"
standard_entropy = "41.46"
molar_volume = "22.688"
"#;
        assert_eq!(
            PhaseRegistry::from_toml_str(no_src).unwrap_err(),
            PhaseError::MissingSource("quartz".to_string())
        );
    }

    #[test]
    fn a_duplicate_phase_name_fails_to_load() {
        let dup = r#"
[[phase]]
name = "quartz"
formula = "SiO2"
enthalpy_formation = "-910.70"
standard_entropy = "41.46"
molar_volume = "22.688"
source = "test"

[[phase]]
name = "quartz"
formula = "SiO2"
enthalpy_formation = "-910.70"
standard_entropy = "41.46"
molar_volume = "22.688"
source = "test"
"#;
        assert_eq!(
            PhaseRegistry::from_toml_str(dup).unwrap_err(),
            PhaseError::DuplicateName("quartz".to_string())
        );
    }

    #[test]
    fn the_formula_parser_reads_composition_and_defaults_a_bare_symbol_to_one() {
        assert_eq!(
            parse_formula("Mg2SiO4").unwrap(),
            vec![
                ("Mg".to_string(), 2),
                ("Si".to_string(), 1),
                ("O".to_string(), 4)
            ]
        );
        assert_eq!(
            parse_formula("MgO").unwrap(),
            vec![("Mg".to_string(), 1), ("O".to_string(), 1)]
        );
        assert!(parse_formula("").is_err(), "an empty formula fails loud");
        assert!(
            parse_formula("2SiO").is_err(),
            "a formula must start with a symbol"
        );
    }

    #[test]
    fn an_optional_clapeyron_slope_is_absent_by_default() {
        let r = registry();
        // The seed oxides carry no transition Clapeyron slope yet; the field is absent, not zero.
        let periclase = r.phase("periclase").expect("periclase is seeded");
        assert_eq!(periclase.clapeyron_slope, None);
        assert_eq!(periclase.clapeyron_decimal, None);
    }
}
