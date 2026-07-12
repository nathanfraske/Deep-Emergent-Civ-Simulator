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

//! The IONIC bulk-modulus tier of the materials oracle (owner research, #182): the bulk modulus of a
//! prototype-mapped ionic phase DERIVED from lattice curvature on the ionic radius, `B = (n-1) A / (18 r0^4)`,
//! the principled route that supersedes the cohesive-energy-density screen tier ([`crate::materials_oracle`])
//! for the ionic-covalent oxides. This is the real foundation of the mechanical arc: the whole oracle's
//! stiffness stands on the Shannon radius column and the structure-prototype dispatch.
//!
//! The derivation is Born-Lande, and every input is measured `[M]`, a class constant `[M class]`, or an exact
//! structure constant, composed by the fixed Rust mechanism into a derived `[D]` output:
//!
//! - The INTERIONIC DISTANCE `r0` is the sum of the cation and anion Shannon radii at the phase's coordination
//!   (`crate::data::shannon_radii`, `[M]`, pm-grade, Shannon 1976). The bulk modulus rides it fourfold
//!   (`B ~ 1/r0^4`), so the radius is the load-bearing column.
//! - The CHARGE PRODUCT `z+ z-` is derived from the formula and the anion's valence by charge balance (the
//!   `valence` column already on the periodic table), so a divalent oxide (MgO) earns its fourfold Coulomb jump
//!   over a monovalent halide (NaCl) from data, never an author.
//! - The BORN EXPONENT `n` is the mean of the cation and anion values, each keyed to the noble-gas core the ion
//!   is isoelectronic with (`crate::data::born_exponents`, `[M class]`, Pauling's series). An ion with no clean
//!   noble-gas core (a d-electron transition-metal ion) has no value and its phase falls through.
//! - The MADELUNG CONSTANT `A` is the exact electrostatic lattice sum of the phase's structure prototype
//!   (`crate::data::prototypes`), read by the phase's prototype key. A phase with no key, an unseeded prototype,
//!   or a prototype whose Madelung constant is held absent (a non-1:1 structure whose reduced lattice sum is not
//!   yet grounded) falls through to the screen tier, an honest `None`, NEVER a fabricated constant.
//!
//! The two remaining constants are fundamental physical law constants (the physics floor, Principle 11): the
//! Coulomb energy `e^2 / 4 pi eps0 = 14.39964 eV.A` and the conversion `1 eV/A^3 = 160.2177 GPa`, both CODATA.
//! The arithmetic is fixed-point and deterministic (radii in angstroms, energies in eV, the modulus in GPa, all
//! well inside the Q32.32 range). The emitted value is the BULK modulus `B` only; the shear modulus `G` is the
//! class-dispatched debt named for its own follow-on slice (a central-force model obeys the Cauchy relation
//! `C12 = C44` as a theorem, so `G` needs one ingredient beyond `B`). Nothing reads the output yet; the pins
//! hold.
//!
//! HONEST LIMIT (surfaced, the point-charge systematic): the bare Born-Lande with full formal charges is
//! excellent for the monovalent alkali halides (NaCl derives to about 2 percent of the measured 24 to 25 GPa)
//! but OVERESTIMATES the divalent oxides, because covalency lowers the effective charge below the formal +/-2
//! and the point-charge model omits it (periclase derives near 266 GPa against a measured 160 to 165). The
//! partial-charge or measured-modulus refinement is the follow-on; the reserved correction factor is documented
//! on [`phase_bulk_modulus_ionic`], surfaced not baked.

use crate::materials_oracle::{PropertyEstimate, Provenance};
use crate::periodic::PeriodicTable;
use crate::petrology_data::Phase;
use civsim_core::fixed::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading one of the lattice-modulus data files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LatticeError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// A key appears twice.
    Duplicate(String),
}

impl fmt::Display for LatticeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatticeError::Parse(m) => write!(f, "lattice-data parse error: {m}"),
            LatticeError::BadValue(m) => write!(f, "lattice-data value error: {m}"),
            LatticeError::MissingSource(m) => write!(f, "lattice-data row without citation: {m}"),
            LatticeError::Duplicate(m) => write!(f, "duplicate lattice-data key: {m}"),
        }
    }
}

impl std::error::Error for LatticeError {}

// ----- The Shannon ionic radius registry -----

/// The Shannon effective ionic radii, keyed by (symbol, charge, coordination), in picometres (measured `[M]`,
/// Shannon 1976). The load-bearing input to the ionic bulk modulus (`B ~ 1/r0^4`).
#[derive(Debug, Clone, Default)]
pub struct ShannonRadii {
    rows: BTreeMap<(String, i8, u8), Fixed>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct RadiiFile {
    #[serde(default)]
    ion: Vec<IonDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct IonDef {
    symbol: String,
    charge: i8,
    coordination: u8,
    radius_pm: String,
    #[serde(default)]
    source: String,
}

impl ShannonRadii {
    /// Load the radii from a TOML string, failing loud on a bad value or a missing citation.
    pub fn from_toml_str(s: &str) -> Result<Self, LatticeError> {
        let file: RadiiFile = toml::from_str(s).map_err(|e| LatticeError::Parse(e.to_string()))?;
        let mut rows = BTreeMap::new();
        for ion in file.ion {
            if ion.source.trim().is_empty() {
                return Err(LatticeError::MissingSource(format!(
                    "{}{:+}",
                    ion.symbol, ion.charge
                )));
            }
            let radius = Fixed::from_decimal_str(ion.radius_pm.trim())
                .map_err(|d| LatticeError::BadValue(format!("radius {}: {d}", ion.symbol)))?;
            let key = (ion.symbol.clone(), ion.charge, ion.coordination);
            if rows.insert(key, radius).is_some() {
                return Err(LatticeError::Duplicate(format!(
                    "{}{:+} CN{}",
                    ion.symbol, ion.charge, ion.coordination
                )));
            }
        }
        Ok(ShannonRadii { rows })
    }

    /// The embedded standard radii (`data/shannon_radii.toml`).
    pub fn standard() -> Result<Self, LatticeError> {
        Self::from_toml_str(include_str!("../data/shannon_radii.toml"))
    }

    /// The radius in picometres for an ion at a coordination, or `None` if the ion is not populated (a data gap,
    /// absent not zero: the phase falls through to the screen tier).
    pub fn radius_pm(&self, symbol: &str, charge: i8, coordination: u8) -> Option<Fixed> {
        self.rows
            .get(&(symbol.to_string(), charge, coordination))
            .copied()
    }
}

// ----- The Born exponent registry -----

/// The Born exponents keyed by the electron count of a noble-gas core (measured class constant `[M class]`,
/// Pauling's series). An ion is assigned the value of the noble gas it is isoelectronic with.
#[derive(Debug, Clone, Default)]
pub struct BornExponents {
    by_electrons: BTreeMap<u32, Fixed>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct BornFile {
    #[serde(default)]
    core: Vec<CoreDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct CoreDef {
    #[serde(default)]
    noble_gas: String,
    electrons: u32,
    exponent: String,
    #[serde(default)]
    source: String,
}

impl BornExponents {
    /// Load the Born exponents from a TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self, LatticeError> {
        let file: BornFile = toml::from_str(s).map_err(|e| LatticeError::Parse(e.to_string()))?;
        let mut by_electrons = BTreeMap::new();
        for core in file.core {
            if core.source.trim().is_empty() {
                return Err(LatticeError::MissingSource(format!(
                    "core {}",
                    core.noble_gas
                )));
            }
            let exponent = Fixed::from_decimal_str(core.exponent.trim())
                .map_err(|d| LatticeError::BadValue(format!("exponent {}: {d}", core.noble_gas)))?;
            if by_electrons.insert(core.electrons, exponent).is_some() {
                return Err(LatticeError::Duplicate(format!(
                    "core electrons {}",
                    core.electrons
                )));
            }
        }
        Ok(BornExponents { by_electrons })
    }

    /// The embedded standard exponents (`data/born_exponents.toml`).
    pub fn standard() -> Result<Self, LatticeError> {
        Self::from_toml_str(include_str!("../data/born_exponents.toml"))
    }

    /// The Born exponent for an ion with the given electron count, or `None` if that count is not a closed
    /// noble-gas shell (a d-electron ion has no clean core; its phase falls through).
    pub fn exponent_for_electrons(&self, electrons: u32) -> Option<Fixed> {
        self.by_electrons.get(&electrons).copied()
    }
}

// ----- The structure-prototype library -----

/// One structure prototype's constants: its Madelung constant (absent for a non-1:1 structure whose reduced
/// lattice sum is not yet grounded) and the cation and anion coordination numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prototype {
    /// The Madelung constant, `None` when held absent (the phase falls through to the screen tier).
    pub madelung: Option<Fixed>,
    /// The cation coordination number (picks the cation Shannon radius).
    pub cation_coordination: u8,
    /// The anion coordination number (picks the anion Shannon radius).
    pub anion_coordination: u8,
}

/// The structure-prototype (aristotype) library, keyed by prototype name.
#[derive(Debug, Clone, Default)]
pub struct PrototypeLibrary {
    rows: BTreeMap<String, Prototype>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct PrototypeFile {
    #[serde(default)]
    prototype: Vec<PrototypeDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct PrototypeDef {
    name: String,
    #[serde(default)]
    madelung_constant: String,
    cation_coordination: u8,
    anion_coordination: u8,
    #[serde(default)]
    source: String,
}

impl PrototypeLibrary {
    /// Load the prototype library from a TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self, LatticeError> {
        let file: PrototypeFile =
            toml::from_str(s).map_err(|e| LatticeError::Parse(e.to_string()))?;
        let mut rows = BTreeMap::new();
        for proto in file.prototype {
            if proto.source.trim().is_empty() {
                return Err(LatticeError::MissingSource(format!(
                    "prototype {}",
                    proto.name
                )));
            }
            let madelung_raw = proto.madelung_constant.trim();
            let madelung =
                if madelung_raw.is_empty() {
                    None
                } else {
                    Some(Fixed::from_decimal_str(madelung_raw).map_err(|d| {
                        LatticeError::BadValue(format!("madelung {}: {d}", proto.name))
                    })?)
                };
            let entry = Prototype {
                madelung,
                cation_coordination: proto.cation_coordination,
                anion_coordination: proto.anion_coordination,
            };
            if rows.insert(proto.name.clone(), entry).is_some() {
                return Err(LatticeError::Duplicate(format!("prototype {}", proto.name)));
            }
        }
        Ok(PrototypeLibrary { rows })
    }

    /// The embedded standard library (`data/prototypes.toml`).
    pub fn standard() -> Result<Self, LatticeError> {
        Self::from_toml_str(include_str!("../data/prototypes.toml"))
    }

    /// The prototype for a key, or `None` if unseeded.
    pub fn prototype(&self, name: &str) -> Option<&Prototype> {
        self.rows.get(name)
    }
}

// ----- The Born-Lande bulk-modulus derivation -----

/// The Coulomb energy `e^2 / (4 pi eps0)` at unit separation, in electron-volt-angstroms (CODATA 2018,
/// `14.399645 eV.A`). A fundamental physical law constant (the physics floor, Principle 11), the energy scale of
/// the Madelung sum.
fn coulomb_energy_ev_angstrom() -> Fixed {
    Fixed::from_decimal_str("14.39964").expect("the Coulomb energy constant is a valid decimal")
}

/// The conversion from `eV/A^3` to gigapascals (CODATA: `1 eV = 1.602177e-19 J`, `1 A^3 = 1e-30 m^3`, so
/// `1 eV/A^3 = 160.2177 GPa`). A fundamental unit-conversion law constant.
fn gpa_per_ev_per_angstrom_cubed() -> Fixed {
    Fixed::from_decimal_str("160.2177").expect("the eV/A^3 to GPa conversion is a valid decimal")
}

/// The identified ionic roles of a binary phase: which element is the cation and which the anion, their integer
/// charges (the anion negative, the cation positive), and their atomic numbers and counts.
struct IonicPair {
    cation_symbol: String,
    cation_z: u8,
    cation_charge: i8,
    anion_symbol: String,
    anion_z: u8,
    anion_charge: i8,
}

/// Identify the cation and anion of a binary ionic phase and derive their charges from the formula and the
/// anion's dominant valence by charge balance. Returns `None` for a phase that is not a clean binary ionic
/// (not two elements, no element with a negative valence, or a charge balance that is not a positive integer
/// cation charge). No charge is authored: the anion takes its dominant (first) negative valence and the cation
/// charge falls out of `x*z_cation + y*z_anion = 0`.
fn identify_ionic_pair(phase: &Phase, table: &PeriodicTable) -> Option<IonicPair> {
    if phase.composition.len() != 2 {
        return None;
    }
    // Split into the element with a negative valence (the anion) and the other (the cation).
    let mut cation: Option<(&str, u8, u32)> = None;
    let mut anion: Option<(&str, u8, u32, i8)> = None;
    for (symbol, count) in &phase.composition {
        let el = table.element(symbol)?;
        let dominant_negative = el.valence.iter().copied().find(|v| *v < 0);
        match dominant_negative {
            Some(charge) => {
                if anion.is_some() {
                    // Two anionic elements: not a simple binary cation-anion ionic phase.
                    return None;
                }
                anion = Some((symbol.as_str(), el.z, *count, charge));
            }
            None => {
                if cation.is_some() {
                    return None;
                }
                cation = Some((symbol.as_str(), el.z, *count));
            }
        }
    }
    let (cation_symbol, cation_z, cation_count) = cation?;
    let (anion_symbol, anion_z, anion_count, anion_charge) = anion?;
    // Charge balance: cation_count * z_cation + anion_count * z_anion = 0, requiring a positive integer cation
    // charge that divides evenly (else the phase is not a clean binary ionic at this anion charge).
    let anion_total = anion_count as i32 * anion_charge as i32; // negative
    if cation_count == 0 || anion_total % (cation_count as i32) != 0 {
        return None;
    }
    let cation_charge = -anion_total / (cation_count as i32);
    if cation_charge <= 0 || cation_charge > i8::MAX as i32 {
        return None;
    }
    Some(IonicPair {
        cation_symbol: cation_symbol.to_string(),
        cation_z,
        cation_charge: cation_charge as i8,
        anion_symbol: anion_symbol.to_string(),
        anion_z,
        anion_charge,
    })
}

/// The BULK MODULUS in GPa of a prototype-mapped ionic phase, DERIVED by Born-Lande lattice curvature,
/// `B = (n-1) A |z+ z-| (e^2/4pi eps0) / (18 r0^4)`, emitted as a `PropertyEstimate` `{value, band, provenance}`.
/// Returns `None` (a fall-through to the screen tier, never a fabricated value) when the phase has no prototype
/// key, an unseeded prototype, a prototype whose Madelung constant is held absent, is not a clean binary ionic,
/// or carries an ion absent from the Shannon table or with no noble-gas Born core. Provenance `[E]`: the route
/// is an exact form over measured inputs but the point-charge model is an approximation, so the honest tag is
/// the estimator one, not `[D]`.
///
/// The emitted `band` is the derived point-charge magnitude scaled by the RESERVED estimator fraction (surfaced,
/// not baked): the systematic deviation of the full-formal-charge point-charge Born-Lande from measured moduli,
/// small for monovalent halides and larger for divalent oxides (the covalency the model omits). Its basis is the
/// empirical `B_measured / B_pointcharge` ratio across the alkali-halide and oxide validation set, a fraction the
/// owner sets; until set the band is emitted as zero and the systematic is documented, never invented into the
/// value.
pub fn phase_bulk_modulus_ionic(
    phase: &Phase,
    table: &PeriodicTable,
    radii: &ShannonRadii,
    born: &BornExponents,
    prototypes: &PrototypeLibrary,
) -> Option<PropertyEstimate> {
    let prototype = prototypes.prototype(phase.prototype.as_deref()?)?;
    let madelung = prototype.madelung?;
    let pair = identify_ionic_pair(phase, table)?;

    // The interionic distance r0 = r_cation + r_anion, in angstroms (the pm radii over 100).
    let cation_radius_pm = radii.radius_pm(
        &pair.cation_symbol,
        pair.cation_charge,
        prototype.cation_coordination,
    )?;
    let anion_radius_pm = radii.radius_pm(
        &pair.anion_symbol,
        pair.anion_charge,
        prototype.anion_coordination,
    )?;
    let hundred = Fixed::from_int(100);
    let r0 = (cation_radius_pm + anion_radius_pm).checked_div(hundred)?;
    if r0 <= Fixed::ZERO {
        return None;
    }

    // The Born exponent n = mean of the cation and anion core values. The cation loses its charge in electrons,
    // the anion gains |charge| (its charge is negative, so subtracting adds).
    let cation_electrons = (pair.cation_z as i32 - pair.cation_charge as i32) as u32;
    let anion_electrons = (pair.anion_z as i32 - pair.anion_charge as i32) as u32;
    let n_cation = born.exponent_for_electrons(cation_electrons)?;
    let n_anion = born.exponent_for_electrons(anion_electrons)?;
    let n = (n_cation + n_anion).checked_div(Fixed::from_int(2))?;

    // The charge product |z+ z-| = cation_charge * -anion_charge (anion charge is negative), from charge balance.
    let charge_product = Fixed::from_int(pair.cation_charge as i32 * -(pair.anion_charge as i32));

    // B = (n-1) * A * |z+z-| * (e^2/4pi eps0) / (18 * r0^4), in eV/A^3, then to GPa.
    let r0_sq = r0.checked_mul(r0)?;
    let r0_fourth = r0_sq.checked_mul(r0_sq)?;
    let denominator = Fixed::from_int(18).checked_mul(r0_fourth)?;
    let numerator = (n - Fixed::ONE)
        .checked_mul(madelung)?
        .checked_mul(charge_product)?
        .checked_mul(coulomb_energy_ev_angstrom())?;
    let b_ev_per_angstrom_cubed = numerator.checked_div(denominator)?;
    let value = b_ev_per_angstrom_cubed.checked_mul(gpa_per_ev_per_angstrom_cubed())?;
    if value <= Fixed::ZERO {
        return None;
    }
    Some(PropertyEstimate {
        value,
        band: Fixed::ZERO,
        provenance: Provenance::Estimator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::petrology_data::PhaseRegistry;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the periodic table loads")
    }
    fn radii() -> ShannonRadii {
        ShannonRadii::standard().expect("the Shannon radii load")
    }
    fn born() -> BornExponents {
        BornExponents::standard().expect("the Born exponents load")
    }
    fn protos() -> PrototypeLibrary {
        PrototypeLibrary::standard().expect("the prototype library loads")
    }
    fn registry() -> PhaseRegistry {
        PhaseRegistry::standard().expect("the phase registry loads")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    // A test-only NaCl phase, the alkali-halide validation reference (NaCl is not a rock-forming phase, so it is
    // not in the registry; it is the clean monovalent case the Born-Lande model reproduces to ~2 percent).
    fn nacl() -> Phase {
        Phase {
            name: "halite".to_string(),
            formula: "NaCl".to_string(),
            composition: vec![("Na".to_string(), 1), ("Cl".to_string(), 1)],
            enthalpy_formation: Fixed::from_int(-411),
            enthalpy_decimal: "-411".to_string(),
            standard_entropy: Fixed::from_int(72),
            entropy_decimal: "72".to_string(),
            molar_volume: Fixed::from_int(27),
            volume_decimal: "27".to_string(),
            clapeyron_slope: None,
            clapeyron_decimal: None,
            prototype: Some("rock-salt".to_string()),
            source: "test-only NaCl validation reference".to_string(),
        }
    }

    #[test]
    fn nacl_bulk_modulus_matches_the_measured_alkali_halide() {
        // The clean monovalent validation: A=1.74756, z+z-=1, n=avg(Ne 7, Ar 9)=8, r0=(102+181)/100=2.83 A.
        // B = 7 * 1.74756 * 1 * 14.39964 / (18 * 2.83^4) * 160.2177 ~ 24.4 GPa, against a measured 24 to 25.
        let est = phase_bulk_modulus_ionic(&nacl(), &table(), &radii(), &born(), &protos())
            .expect("NaCl derives its bulk modulus");
        assert!(
            close(est.value, 24.4, 2.0),
            "NaCl bulk modulus should be ~24 GPa (measured 24-25), got {}",
            est.value.to_f64_lossy()
        );
        assert_eq!(est.provenance, Provenance::Estimator);
    }

    #[test]
    fn periclase_derives_with_the_documented_divalent_overestimate() {
        // MgO: A=1.74756, z+z-=4, n=7, r0=(72+140)/100=2.12 A. The point-charge model gives ~266 GPa against a
        // measured 160-165: the divalent-oxide overestimate, derived-but-approximate, the honest limit surfaced.
        let reg = registry();
        let periclase = reg
            .phase("periclase")
            .expect("periclase is in the registry");
        let est = phase_bulk_modulus_ionic(periclase, &table(), &radii(), &born(), &protos())
            .expect("periclase derives its bulk modulus (it is rock-salt, prototype-mapped)");
        assert!(
            close(est.value, 266.0, 6.0),
            "periclase point-charge B should be ~266 GPa, got {}",
            est.value.to_f64_lossy()
        );
        // The fourfold charge product lifts it well above NaCl's ~24 GPa: the Coulomb jump earned from data.
        assert!(
            est.value > Fixed::from_int(100),
            "the divalent charge product lifts MgO far above the monovalent halide"
        );
    }

    #[test]
    fn corundum_and_hematite_fall_through_on_the_absent_madelung() {
        // Both map to the corundum prototype, whose Madelung constant is held absent (A2B3 reduced lattice sum
        // not yet grounded), so the ionic route returns None: an honest fall-through, never a fabricated A.
        let reg = registry();
        for name in ["corundum", "hematite"] {
            let phase = reg.phase(name).expect("phase is in the registry");
            assert!(
                phase_bulk_modulus_ionic(phase, &table(), &radii(), &born(), &protos()).is_none(),
                "{name} falls through: its prototype's Madelung constant is absent"
            );
        }
    }

    #[test]
    fn hematite_fe3_also_lacks_a_noble_gas_born_core() {
        // Independent of the absent Madelung: Fe3+ has 23 electrons ([Ar]3d5), not a closed noble-gas shell, so
        // even with a Madelung constant it would fall through on the Born core. Confirm the core lookup is None.
        assert!(
            born().exponent_for_electrons(23).is_none(),
            "Fe3+ (23 electrons) has no clean noble-gas Born core"
        );
    }

    #[test]
    fn unkeyed_and_covalent_phases_fall_through_to_the_screen_tier() {
        // Quartz (no prototype key, covalent framework) and forsterite (no key, multi-site orthosilicate) both
        // return None from the ionic route: the class dispatch routes them to the screen tier, not a forced
        // rock-salt formula.
        let reg = registry();
        for name in ["quartz", "forsterite", "fayalite"] {
            let phase = reg.phase(name).expect("phase is in the registry");
            assert!(
                phase.prototype.is_none(),
                "{name} carries no ionic prototype key"
            );
            assert!(
                phase_bulk_modulus_ionic(phase, &table(), &radii(), &born(), &protos()).is_none(),
                "{name} falls through to the screen tier (no ionic prototype)"
            );
        }
    }

    #[test]
    fn the_charge_product_is_derived_from_the_formula_not_authored() {
        // MgO charge balance: O is -2 (dominant valence), so Mg is +2 by 1*z_Mg + 1*(-2) = 0. Al2O3: Al is +3
        // by 2*z_Al + 3*(-2) = 0. The cation charge is never read from a per-phase field.
        let reg = registry();
        let periclase = reg.phase("periclase").unwrap();
        let pair = identify_ionic_pair(periclase, &table()).expect("MgO is a clean binary ionic");
        assert_eq!(pair.cation_symbol, "Mg");
        assert_eq!(pair.cation_charge, 2);
        assert_eq!(pair.anion_charge, -2);
        let corundum = reg.phase("corundum").unwrap();
        let pair = identify_ionic_pair(corundum, &table()).expect("Al2O3 is a clean binary ionic");
        assert_eq!(pair.cation_symbol, "Al");
        assert_eq!(pair.cation_charge, 3);
    }

    #[test]
    fn the_data_files_load_and_carry_their_seed_rows() {
        assert!(
            radii().radius_pm("Mg", 2, 6).is_some(),
            "Mg2+ VI radius is seeded"
        );
        assert!(
            radii().radius_pm("O", -2, 6).is_some(),
            "O2- VI radius is seeded"
        );
        assert_eq!(
            born().exponent_for_electrons(10),
            Some(Fixed::from_int(7)),
            "Ne core = 7"
        );
        assert!(
            protos()
                .prototype("rock-salt")
                .and_then(|p| p.madelung)
                .is_some(),
            "the rock-salt Madelung constant is populated"
        );
        assert!(
            protos()
                .prototype("corundum")
                .and_then(|p| p.madelung)
                .is_none(),
            "the corundum Madelung constant is held absent"
        );
    }
}
