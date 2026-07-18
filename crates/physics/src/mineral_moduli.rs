//! The cited mineral-moduli floor: measured single-crystal elastic moduli, one row per candidate phase, the
//! per-MINERAL sibling of the phase registry (`crate::petrology_data`). Each row carries a phase's isotropic
//! aggregate bulk modulus `K` and shear modulus `G` (GPa), each with a symmetric measurement band, measured in
//! the intact single-crystal frame at a stated pressure and temperature, real-with-source.
//!
//! WHY THIS IS A FLOOR AND NOT A DERIVATION (RUNBOOK section 14, the owner's doctrine). A mineral's measured
//! modulus is a FLOOR datum: it was measured in ignorance of every rock it will ever sit in, still less the
//! world, so it CANNOT fit an outcome. The rock's modulus is then DERIVED by aggregating these mineral moduli
//! over the world's own mineral census (`crate::materials_oracle::assemblage_bulk_shear_moduli`), never cited as
//! a rock. Above the floor derive is the vertical: a cited MINERAL row feeds the aggregation from below, and the
//! only cited value that ever appears on the input side is a mineral, never a rock. A rock name in this file
//! would be the defect, grep-findable.
//!
//! WHY A SEPARATE FILE AND NOT A COLUMN ON THE PHASE REGISTRY. The phase registry cites one internally
//! consistent THERMODYNAMIC dataset (Robie and Hemingway 1995) for its enthalpies and volumes; the elastic
//! moduli come from a different measurement (single-crystal elasticity, a different primary), so they carry
//! their own citation in their own cited block rather than being appended silently to a thermodynamic row where
//! their provenance would be laundered (the mixed-grade guard: a cited import never wears another block's
//! grade). The mechanism is fixed Rust; the mineral MEMBERSHIP is data and grows with the world (Principle 11):
//! an alien phase's measured moduli are a new row, not a rewrite, and a phase with no measured row is refused by
//! the aggregator, never defaulted.

use civsim_core::fixed::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the mineral-moduli file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuliError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every measured value is real-with-source).
    MissingSource(String),
    /// A modulus is non-positive, or a band is negative, or a band brackets zero stiffness (a soft edge at or
    /// below zero is not a physical modulus).
    NonPhysical(String),
    /// A phase name appears twice.
    Duplicate(String),
}

impl fmt::Display for ModuliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuliError::Parse(m) => write!(f, "mineral-moduli parse error: {m}"),
            ModuliError::BadValue(m) => write!(f, "mineral-moduli value error: {m}"),
            ModuliError::MissingSource(m) => write!(f, "mineral-moduli row without citation: {m}"),
            ModuliError::NonPhysical(m) => write!(f, "mineral-moduli non-physical value: {m}"),
            ModuliError::Duplicate(m) => write!(f, "duplicate mineral-moduli key: {m}"),
        }
    }
}

impl std::error::Error for ModuliError {}

/// The canonical mineral key for a phase name, bridging the two phase-naming conventions the engine uses: the
/// petrology kernel's plain registry names (`forsterite`, `enstatite`) and the JANAF condensation solver's
/// decorated names (`Mg2SiO4(cr,forsterite)`, `MgSiO3(cr,enstatite)`, `MgAl2O4(cr,spinel)`). A decorated name
/// carries its mineral inside a `(cr,NAME)` tag; this extracts NAME so a crust derived by the condensation solver
/// and a mantle derived by the petrology kernel both resolve to the one measured moduli row for that mineral. A
/// plain name (no `(cr,` tag), or a bare `(cr)` metal phase like `Fe(cr)`, passes through unchanged. This is a
/// pure string normalization, not a rename of either convention: each keeps its own names, and the moduli floor
/// is keyed once by the mineral identity they share.
pub fn canonical_phase_key(name: &str) -> &str {
    if let Some(start) = name.find("(cr,") {
        let rest = &name[start + 4..];
        if let Some(end) = rest.find(')') {
            return rest[..end].trim();
        }
    }
    name
}

/// One phase's measured elastic moduli in the intact single-crystal frame. The bulk and shear moduli are the
/// isotropic aggregate (Voigt-Reuss-Hill of the single-crystal elastic tensor) in GPa; each band is the
/// symmetric half-width of the measurement uncertainty. The pressure and temperature are the CHORD conditions
/// the measurement was taken at (carried so the aggregator can declare what it is dropping, see
/// [`crate::materials_oracle::assemblage_bulk_shear_moduli`]): the seed rows are ambient (298 K, 1 bar), and a
/// rock at depth sits at a different chord, an offset the aggregator names as a dropped term rather than
/// silently absorbing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MineralModulusRow {
    /// The isotropic aggregate bulk modulus `K` (GPa), measured, positive.
    pub bulk_gpa: Fixed,
    /// The symmetric half-width of the bulk-modulus measurement band (GPa), non-negative, not reaching the
    /// modulus (a soft edge stays positive).
    pub bulk_band_gpa: Fixed,
    /// The isotropic aggregate shear modulus `G` (GPa), measured, positive.
    pub shear_gpa: Fixed,
    /// The symmetric half-width of the shear-modulus measurement band (GPa), non-negative, not reaching the
    /// modulus.
    pub shear_band_gpa: Fixed,
    /// The temperature the moduli were measured at (K), the thermal chord condition.
    pub temperature_k: Fixed,
    /// The pressure the moduli were measured at (bar), the pressure chord condition.
    pub pressure_bar: Fixed,
    /// The measurement frame (for example "intact single-crystal VRH, crack-free"), carried as text so the
    /// dropped-term declaration (cracks, porosity, texture) reads against the frame the number holds for.
    pub frame: String,
    /// The precise citation (source, table, page). Required non-empty.
    pub source: String,
}

/// The cited mineral-moduli table, keyed by phase name (matching the phase-registry spelling exactly).
#[derive(Debug, Clone, Default)]
pub struct MineralModuli {
    rows: BTreeMap<String, MineralModulusRow>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ModuliFile {
    #[serde(default)]
    mineral: Vec<MineralDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct MineralDef {
    name: String,
    bulk_modulus_gpa: String,
    #[serde(default)]
    bulk_band_gpa: String,
    shear_modulus_gpa: String,
    #[serde(default)]
    shear_band_gpa: String,
    #[serde(default)]
    temperature_k: String,
    #[serde(default)]
    pressure_bar: String,
    #[serde(default)]
    frame: String,
    #[serde(default)]
    source: String,
}

impl MineralModuli {
    /// Load the mineral-moduli table from a TOML string. Every row must carry a citation and a positive bulk and
    /// shear modulus whose bands do not reach zero; a violated invariant is an error, never a silent default.
    pub fn from_toml_str(s: &str) -> Result<Self, ModuliError> {
        let file: ModuliFile = toml::from_str(s).map_err(|e| ModuliError::Parse(e.to_string()))?;
        let mut rows = BTreeMap::new();
        for def in file.mineral {
            if def.source.trim().is_empty() {
                return Err(ModuliError::MissingSource(format!("mineral {}", def.name)));
            }
            // An explicitly UNSOURCED row is a DECLARED-but-unmeasured phase: a fetch could not hold a primary for
            // its aggregate moduli (its `source` field carries the reason and the owner action). Skip it, so it has
            // no table row, and the aggregator refuses any assemblage containing it (the banded-refusal path)
            // rather than erroring the whole load or fabricating a number. The row stays in the data file, visible.
            if def
                .bulk_modulus_gpa
                .trim()
                .eq_ignore_ascii_case("UNSOURCED")
                || def
                    .shear_modulus_gpa
                    .trim()
                    .eq_ignore_ascii_case("UNSOURCED")
            {
                continue;
            }
            let parse = |field: &str, raw: &str| -> Result<Fixed, ModuliError> {
                Fixed::from_decimal_str(raw.trim())
                    .map_err(|d| ModuliError::BadValue(format!("{field} of {}: {d}", def.name)))
            };
            let bulk_gpa = parse("bulk_modulus_gpa", &def.bulk_modulus_gpa)?;
            let shear_gpa = parse("shear_modulus_gpa", &def.shear_modulus_gpa)?;
            // A blank band is an absent measurement band, read as zero (the aggregator then carries only the
            // texture gap for that phase); a present band must be non-negative.
            let bulk_band_gpa = if def.bulk_band_gpa.trim().is_empty() {
                Fixed::ZERO
            } else {
                parse("bulk_band_gpa", &def.bulk_band_gpa)?
            };
            let shear_band_gpa = if def.shear_band_gpa.trim().is_empty() {
                Fixed::ZERO
            } else {
                parse("shear_band_gpa", &def.shear_band_gpa)?
            };
            let temperature_k = if def.temperature_k.trim().is_empty() {
                Fixed::ZERO
            } else {
                parse("temperature_k", &def.temperature_k)?
            };
            let pressure_bar = if def.pressure_bar.trim().is_empty() {
                Fixed::ZERO
            } else {
                parse("pressure_bar", &def.pressure_bar)?
            };
            // Physical invariants: a modulus is positive, a band is non-negative, and a band does not reach the
            // modulus (the soft edge K - band or G - band stays strictly positive, so the harmonic Reuss mean
            // the aggregator takes over the soft edges never divides through zero or a negative stiffness).
            if bulk_gpa <= Fixed::ZERO || shear_gpa <= Fixed::ZERO {
                return Err(ModuliError::NonPhysical(format!(
                    "{}: a modulus is non-positive",
                    def.name
                )));
            }
            if bulk_band_gpa < Fixed::ZERO || shear_band_gpa < Fixed::ZERO {
                return Err(ModuliError::NonPhysical(format!(
                    "{}: a band is negative",
                    def.name
                )));
            }
            if bulk_band_gpa >= bulk_gpa || shear_band_gpa >= shear_gpa {
                return Err(ModuliError::NonPhysical(format!(
                    "{}: a band reaches or exceeds its modulus (soft edge non-positive)",
                    def.name
                )));
            }
            let row = MineralModulusRow {
                bulk_gpa,
                bulk_band_gpa,
                shear_gpa,
                shear_band_gpa,
                temperature_k,
                pressure_bar,
                frame: def.frame.trim().to_string(),
                source: def.source.trim().to_string(),
            };
            if rows.insert(def.name.clone(), row).is_some() {
                return Err(ModuliError::Duplicate(format!("mineral {}", def.name)));
            }
        }
        Ok(MineralModuli { rows })
    }

    /// The embedded standard mineral-moduli table (`data/mineral_moduli.toml`), the cited single-crystal `K`, `G`
    /// of the seed rock-forming phases (quartz, corundum, periclase, forsterite, fayalite, spinel; hematite is an
    /// UNSOURCED row, absent from the table, so the aggregator refuses a hematite-bearing assemblage until it is
    /// measured). Each row is real-with-source, its primary vendored under `data/` with a SHA256 receipt.
    pub fn standard() -> Result<Self, ModuliError> {
        Self::from_toml_str(include_str!("../data/mineral_moduli.toml"))
    }

    /// The measured moduli for a phase, or `None` if the phase has no measured row (an unmeasured member the
    /// aggregator routes to a banded refusal, never a silent drop). The lookup is by CANONICAL KEY
    /// ([`canonical_phase_key`]), so both phase-naming conventions the engine uses resolve to the same row: the
    /// petrology kernel's plain name (`forsterite`) and the JANAF condensation solver's decorated name
    /// (`Mg2SiO4(cr,forsterite)`, `MgSiO3(cr,enstatite)`).
    pub fn row(&self, name: &str) -> Option<&MineralModulusRow> {
        self.rows.get(canonical_phase_key(name))
    }

    /// The number of rows loaded.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_a_cited_row_and_rejects_an_uncited_one() {
        let ok = r#"
[[mineral]]
name = "periclase"
bulk_modulus_gpa = "162.5"
bulk_band_gpa = "1.0"
shear_modulus_gpa = "130.0"
shear_band_gpa = "1.0"
temperature_k = "298"
pressure_bar = "1"
frame = "intact single-crystal VRH"
source = "test fixture, not a real citation load"
"#;
        let table = MineralModuli::from_toml_str(ok).expect("a cited row loads");
        assert_eq!(table.len(), 1);
        let row = table.row("periclase").expect("periclase present");
        assert_eq!(row.bulk_gpa, Fixed::from_ratio(1625, 10));
        assert!(table.row("forsterite").is_none());

        let uncited = r#"
[[mineral]]
name = "periclase"
bulk_modulus_gpa = "162.5"
shear_modulus_gpa = "130.0"
"#;
        assert!(matches!(
            MineralModuli::from_toml_str(uncited),
            Err(ModuliError::MissingSource(_))
        ));
    }

    #[test]
    fn rejects_a_band_that_reaches_zero_stiffness() {
        // A band as wide as the modulus would put the soft Reuss edge at zero, and the harmonic mean would
        // divide through it. The loader refuses it at the door rather than letting the aggregator hit it.
        let bad = r#"
[[mineral]]
name = "quartz"
bulk_modulus_gpa = "37.8"
bulk_band_gpa = "40.0"
shear_modulus_gpa = "44.3"
shear_band_gpa = "1.0"
source = "test fixture"
"#;
        assert!(matches!(
            MineralModuli::from_toml_str(bad),
            Err(ModuliError::NonPhysical(_))
        ));
    }

    #[test]
    fn the_standard_table_loads_the_sourced_phases_and_skips_unsourced_hematite() {
        let table = MineralModuli::standard().expect("the embedded mineral-moduli table loads");
        // The six sourced seed phases are present, each with cited positive moduli.
        for name in [
            "quartz",
            "corundum",
            "periclase",
            "forsterite",
            "fayalite",
            "spinel",
        ] {
            let row = table
                .row(name)
                .unwrap_or_else(|| panic!("{name} is present in the standard table"));
            assert!(row.bulk_gpa > Fixed::ZERO && row.shear_gpa > Fixed::ZERO);
            assert!(!row.source.is_empty(), "{name} carries a citation");
        }
        // Hematite is an UNSOURCED row (a fetch could not hold a primary for its aggregate), so it is skipped, not
        // loaded: the aggregator then refuses any assemblage containing it, the banded-refusal path.
        assert!(
            table.row("hematite").is_none(),
            "the UNSOURCED hematite row is skipped, not loaded"
        );
        assert_eq!(table.len(), 6, "six sourced phases, hematite absent");
        // Spot-check a cited value: forsterite K_S = 128.8 GPa (Zha et al. 1996).
        let fo = table.row("forsterite").expect("forsterite present");
        assert!(
            (fo.bulk_gpa - Fixed::from_ratio(1288, 10)).abs() < Fixed::from_ratio(1, 100),
            "forsterite K_S is the cited 128.8 GPa, got {}",
            fo.bulk_gpa.to_f64_lossy()
        );
    }

    #[test]
    fn the_canonical_key_bridges_the_janaf_and_registry_names() {
        // The JANAF condensation solver decorates its phase names; the petrology kernel uses plain ones. Both
        // resolve to the same measured row through the canonical key.
        assert_eq!(canonical_phase_key("MgSiO3(cr,enstatite)"), "enstatite");
        assert_eq!(canonical_phase_key("Mg2SiO4(cr,forsterite)"), "forsterite");
        assert_eq!(canonical_phase_key("MgAl2O4(cr,spinel)"), "spinel");
        assert_eq!(canonical_phase_key("forsterite"), "forsterite"); // a plain name passes through
        assert_eq!(canonical_phase_key("Fe(cr)"), "Fe(cr)"); // a bare metal phase passes through
                                                             // A lookup by the decorated crust name lands on the registry-keyed row.
        let toml = r#"
[[mineral]]
name = "spinel"
bulk_modulus_gpa = "197.9"
shear_modulus_gpa = "108.5"
source = "test fixture, not a citation load"
"#;
        let table = MineralModuli::from_toml_str(toml).expect("the fixture loads");
        assert!(
            table.row("MgAl2O4(cr,spinel)").is_some(),
            "the JANAF crust name resolves to the spinel row"
        );
        assert!(
            table.row("spinel").is_some(),
            "and the plain name resolves too"
        );
    }
}
