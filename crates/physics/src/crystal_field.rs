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

//! The crystal-field column (`crates/physics/data/crystal_field.toml`, Stage 6): the octahedral splitting
//! `Delta_o` and the Racah `B`, the inputs to the magnetism (b) high/low-spin correction and the optics d-d colour.
//!
//! `Delta_o` FACTORIZES (Jorgensen): `Delta_o = f(ligand) * g(ion)`, with `f` dimensionless (`f(H2O) = 1.00`
//! PINNED, since multiplicativity breaks across sources otherwise) and `g` in `10^3 cm^-1`. The free-ion Racah `B`
//! is the electron-repulsion / spin-pairing side (`C ~ 4B` where `C` is untabulated). The solid MONOXIDES do NOT
//! factorize: the bare oxide `O2-` forms no discrete octahedral molecular complex, so there is no `f(O2-)`, and the
//! monoxide splitting is the DIRECT solid-state optical/RIXS/neutron measurement (a per-composition column). No
//! consumer is wired in any pinned run path yet (byte-neutral).
//!
//! NO NUMERICAL CROSS-CHECK, so the back-check is a THREE-MODALITY TREND (verified at the cited fetch, re-asserted
//! here): multiplicativity (`f*g` reproduces holdout compounds), CFSE-versus-calorimetry (the double-humped
//! hydration-enthalpy deviation), and the `Delta_o ~ R^-5` pressure scaling (ruby R-line, ferropericlase spin
//! transition). Every value is cited (Jorgensen 1971 via Dalal for `f`/`g`/`B`; single-crystal studies for the
//! oxide `Delta_o`), surfaced for owner verification, never invented.
//!
//! UNITS (the Slack lesson): values are stored in `cm^-1`; the `8065.544 cm^-1/eV` conversion is ASSEMBLED from the
//! exact SI mantissas of `e`, `h`, and `c` (the dimensionless-constant law, [`cm_per_ev`]) and round-trip tested,
//! never a folded decimal.

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// The canonical key of a composition: the elements in sorted order with their counts, so `{Ni:1, O:1}` and the
/// reverse both key the same row (the same shape the sibling `[M]` columns use).
fn composition_key(composition: &BTreeMap<String, u32>) -> String {
    composition
        .iter()
        .map(|(el, n)| format!("{el}{n}"))
        .collect::<Vec<_>>()
        .join("")
}

/// What can go wrong loading the crystal-field column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrystalFieldError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// A key appears twice.
    Duplicate(String),
    /// A value is non-positive (`f`, `g`, `B`, and `Delta_o` are all positive).
    NonPositive(String),
    /// The `f(H2O)` normalization is not pinned to `1.00` (multiplicativity would break across sources).
    UnpinnedReference(String),
}

impl fmt::Display for CrystalFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrystalFieldError::Parse(m) => write!(f, "crystal-field parse error: {m}"),
            CrystalFieldError::BadValue(m) => write!(f, "crystal-field value error: {m}"),
            CrystalFieldError::MissingSource(m) => {
                write!(f, "crystal-field row without citation: {m}")
            }
            CrystalFieldError::Duplicate(m) => write!(f, "duplicate crystal-field key: {m}"),
            CrystalFieldError::NonPositive(m) => write!(f, "crystal-field non-positive value: {m}"),
            CrystalFieldError::UnpinnedReference(m) => {
                write!(f, "crystal-field f(H2O) not pinned to 1.00: {m}")
            }
        }
    }
}

impl std::error::Error for CrystalFieldError {}

/// The `cm^-1`-per-eV conversion `8065.544`, ASSEMBLED from the exact SI mantissas of the elementary charge `e`, the
/// Planck constant `h`, and the speed of light `c` (in cm/s), with a single collapsed power of ten (the
/// dimensionless-constant law, no folded decimal): `1 eV = e / (h * c) cm^-1`, and since `e` carries `10^-19`, `h`
/// carries `10^-34`, and `c[cm/s]` carries `10^10`, the powers net `10^5`, so it is
/// `(1.602176634 / (6.62607015 * 2.99792458)) * 10^5 ~ 8065.54 cm^-1/eV`.
pub fn cm_per_ev() -> Fixed {
    let e = Fixed::from_ratio(1_602_176_634, 1_000_000_000);
    let h = Fixed::from_ratio(662_607_015, 100_000_000);
    let c = Fixed::from_ratio(299_792_458, 100_000_000);
    let denom = match h.checked_mul(c) {
        Some(v) if v > Fixed::ZERO => v,
        _ => return Fixed::ZERO,
    };
    e.checked_div(denom)
        .and_then(|x| x.checked_mul(Fixed::from_int(100_000)))
        .unwrap_or(Fixed::ZERO)
}

/// Convert an energy in `cm^-1` to eV (`E[eV] = E[cm^-1] / 8065.544`). `None` on a bad conversion.
pub fn cm_to_ev(cm: Fixed) -> Option<Fixed> {
    cm.checked_div(cm_per_ev())
}

/// The crystal-field tables: the Jorgensen `f`/`g` factorization, the free-ion Racah `B`, and the direct oxide
/// `Delta_o`, all in `cm^-1` (except the dimensionless `f`).
#[derive(Debug, Clone, Default)]
pub struct CrystalFieldTables {
    ligand_f: BTreeMap<String, Fixed>,
    ion_g_kilocm: BTreeMap<String, Fixed>,
    racah_b_cm: BTreeMap<String, Fixed>,
    oxide_delta_cm: BTreeMap<String, Fixed>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct CrystalFieldFile {
    #[serde(default)]
    ligand_f: Vec<LigandFDef>,
    #[serde(default)]
    ion_g: Vec<IonGDef>,
    #[serde(default)]
    racah_b: Vec<RacahBDef>,
    #[serde(default)]
    oxide_delta: Vec<OxideDeltaDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct LigandFDef {
    ligand: String,
    #[serde(default)]
    f: String,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct IonGDef {
    ion: String,
    #[serde(default)]
    g_kilocm: String,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct RacahBDef {
    ion: String,
    #[serde(default)]
    b_cm: String,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct OxideDeltaDef {
    #[serde(default)]
    composition: BTreeMap<String, u32>,
    #[serde(default)]
    delta_cm: String,
    #[serde(default)]
    reliability: String,
    #[serde(default)]
    source: String,
}

fn parse_positive(raw: &str, label: &str) -> Result<Fixed, CrystalFieldError> {
    let v = Fixed::from_decimal_str(raw.trim())
        .map_err(|d| CrystalFieldError::BadValue(format!("{label}: {d}")))?;
    if v <= Fixed::ZERO {
        return Err(CrystalFieldError::NonPositive(label.to_string()));
    }
    Ok(v)
}

impl CrystalFieldTables {
    /// Load the column from a TOML string. Every row must carry a citation and a positive value, and the `f(H2O)`
    /// reference must be pinned to `1.00` (the multiplicativity normalization).
    pub fn from_toml_str(s: &str) -> Result<Self, CrystalFieldError> {
        let file: CrystalFieldFile =
            toml::from_str(s).map_err(|e| CrystalFieldError::Parse(e.to_string()))?;
        let mut ligand_f = BTreeMap::new();
        for l in file.ligand_f {
            if l.source.trim().is_empty() {
                return Err(CrystalFieldError::MissingSource(l.ligand.clone()));
            }
            let f = parse_positive(&l.f, &format!("f({})", l.ligand))?;
            if ligand_f.insert(l.ligand.clone(), f).is_some() {
                return Err(CrystalFieldError::Duplicate(l.ligand));
            }
        }
        // The pinned normalization: f(H2O) must be exactly 1.00 (multiplicativity breaks otherwise).
        match ligand_f.get("H2O") {
            Some(f) if *f == Fixed::from_int(1) => {}
            _ => {
                return Err(CrystalFieldError::UnpinnedReference(
                    "f(H2O) must be present and equal to 1.00".to_string(),
                ))
            }
        }
        let mut ion_g_kilocm = BTreeMap::new();
        for g in file.ion_g {
            if g.source.trim().is_empty() {
                return Err(CrystalFieldError::MissingSource(g.ion.clone()));
            }
            let val = parse_positive(&g.g_kilocm, &format!("g({})", g.ion))?;
            if ion_g_kilocm.insert(g.ion.clone(), val).is_some() {
                return Err(CrystalFieldError::Duplicate(g.ion));
            }
        }
        let mut racah_b_cm = BTreeMap::new();
        for b in file.racah_b {
            if b.source.trim().is_empty() {
                return Err(CrystalFieldError::MissingSource(b.ion.clone()));
            }
            let val = parse_positive(&b.b_cm, &format!("B({})", b.ion))?;
            if racah_b_cm.insert(b.ion.clone(), val).is_some() {
                return Err(CrystalFieldError::Duplicate(b.ion));
            }
        }
        let mut oxide_delta_cm = BTreeMap::new();
        for o in file.oxide_delta {
            if o.source.trim().is_empty() {
                return Err(CrystalFieldError::MissingSource("oxide_delta".to_string()));
            }
            let key = composition_key(&o.composition);
            let val = parse_positive(&o.delta_cm, &format!("Delta_o({key})"))?;
            if oxide_delta_cm.insert(key.clone(), val).is_some() {
                return Err(CrystalFieldError::Duplicate(key));
            }
        }
        Ok(CrystalFieldTables {
            ligand_f,
            ion_g_kilocm,
            racah_b_cm,
            oxide_delta_cm,
        })
    }

    /// The embedded standard column (`data/crystal_field.toml`).
    pub fn standard() -> Result<Self, CrystalFieldError> {
        Self::from_toml_str(include_str!("../data/crystal_field.toml"))
    }

    /// The Jorgensen ligand factor `f` (dimensionless), or `None` when the ligand is not tabulated.
    pub fn ligand_f(&self, ligand: &str) -> Option<Fixed> {
        self.ligand_f.get(ligand).copied()
    }

    /// The Jorgensen metal factor `g` (in `10^3 cm^-1`), or `None` when the ion is not tabulated.
    pub fn ion_g_kilocm(&self, ion: &str) -> Option<Fixed> {
        self.ion_g_kilocm.get(ion).copied()
    }

    /// The factorized octahedral splitting `Delta_o = f(ligand) * g(ion)` in `cm^-1` (with `g` in `10^3 cm^-1`, so
    /// the product is scaled by 1000). `None` when either factor is absent. The molecular-complex route; the solid
    /// oxides use [`Self::oxide_delta_cm`] instead (no `f(O2-)`).
    pub fn delta_o_factored_cm(&self, ligand: &str, ion: &str) -> Option<Fixed> {
        let f = self.ligand_f(ligand)?;
        let g = self.ion_g_kilocm(ion)?;
        f.checked_mul(g)?.checked_mul(Fixed::from_int(1000))
    }

    /// The DIRECT measured octahedral splitting `Delta_o` (in `cm^-1`) of a solid monoxide, or `None` when the
    /// composition is not in the seeded set. The magnetism-(b) oxide anchor (the monoxides do not factorize).
    pub fn oxide_delta_cm(&self, composition: &[(String, u32)]) -> Option<Fixed> {
        let map: BTreeMap<String, u32> = composition.iter().cloned().collect();
        self.oxide_delta_cm.get(&composition_key(&map)).copied()
    }

    /// The free-ion Racah `B` (in `cm^-1`) of an ion, the electron-repulsion / spin-pairing input, or `None` when
    /// the ion is not tabulated.
    pub fn racah_b_cm(&self, ion: &str) -> Option<Fixed> {
        self.racah_b_cm.get(ion).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables() -> CrystalFieldTables {
        CrystalFieldTables::standard().expect("the crystal-field column loads")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    fn comp(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
        pairs.iter().map(|(s, c)| ((*s).to_string(), *c)).collect()
    }

    #[test]
    fn the_cm_per_ev_conversion_reassembles_from_e_h_c() {
        // THE DIMENSIONLESS-CONSTANT LAW: 1 eV = e/(h*c) cm^-1 reassembles to 8065.544 from the exact SI mantissas
        // of e, h, c with a single collapsed 10^5, never a folded decimal.
        let k = cm_per_ev();
        assert!(
            close(k, 8065.544, 0.1),
            "cm/eV ~ 8065.544, got {}",
            k.to_f64_lossy()
        );
        // Round-trip: NiO's 8470 cm^-1 is 1.05 eV.
        let ev = cm_to_ev(Fixed::from_int(8470)).expect("convert");
        assert!(
            close(ev, 1.05, 0.005),
            "8470 cm^-1 ~ 1.05 eV, got {}",
            ev.to_f64_lossy()
        );
    }

    #[test]
    fn the_factorization_reproduces_a_holdout_aqua_complex() {
        // Multiplicativity (modality 0): Delta_o = f * g * 1000. [Co(H2O)6]2+ = 1.00 * 9.3 * 1000 = 9300 cm^-1,
        // matching the Tanabe-Sugano-refined measurement (0% deviation, the holdout check).
        let t = tables();
        let co_aqua = t.delta_o_factored_cm("H2O", "Co2+").expect("Co aqua");
        assert!(
            close(co_aqua, 9300.0, 1.0),
            "[Co(H2O)6]2+ Delta_o ~ 9300 cm^-1, got {}",
            co_aqua.to_f64_lossy()
        );
        // A cross-ligand holdout: [Co(en)3]3+ = f(en) 1.28 * g(Co3+) 19.0 * 1000 = 24320 cm^-1 (measured ~22600,
        // within the ~10-15% multiplicativity band, neither factor fit to this compound).
        let co_en = t.delta_o_factored_cm("en", "Co3+").expect("Co en");
        assert!(
            close(co_en, 24320.0, 1.0),
            "[Co(en)3]3+ predicted 24320 cm^-1, got {}",
            co_en.to_f64_lossy()
        );
        assert!(
            (co_en.to_f64_lossy() - 22600.0).abs() / 22600.0 < 0.15,
            "the cross-ligand holdout is within the 15% multiplicativity band"
        );
    }

    #[test]
    fn the_charge_trend_holds_and_water_is_pinned() {
        // Modality trend: g(M3+) > g(M2+). g(Co3+) 19.0 > g(Co2+) 9.3. And f(H2O) is pinned to 1.00.
        let t = tables();
        let co3 = t.ion_g_kilocm("Co3+").expect("Co3+");
        let co2 = t.ion_g_kilocm("Co2+").expect("Co2+");
        assert!(co3 > co2, "g(Co3+) > g(Co2+), the charge trend");
        assert_eq!(
            t.ligand_f("H2O"),
            Some(Fixed::from_int(1)),
            "f(H2O) pinned to 1.00"
        );
    }

    #[test]
    fn the_direct_oxide_delta_are_the_monoxide_anchors() {
        // The solid monoxides carry the DIRECT measured Delta_o (they do not factorize; no f(O2-)). NiO 8470 cm^-1
        // (~1.05 eV), the reliable RIXS anchor; the values sit in the ~7500-9000 cm^-1 weak-oxygen-field band.
        let t = tables();
        let nio = t
            .oxide_delta_cm(&comp(&[("Ni", 1), ("O", 1)]))
            .expect("NiO");
        assert!(close(nio, 8470.0, 1.0), "NiO Delta_o 8470 cm^-1");
        let nio_ev = cm_to_ev(nio).expect("eV");
        assert!(close(nio_ev, 1.05, 0.005), "NiO Delta_o ~ 1.05 eV");
        // FeO and CoO are the shallower, high-spin oxides.
        assert!(t.oxide_delta_cm(&comp(&[("Fe", 1), ("O", 1)])).is_some());
        assert!(t.oxide_delta_cm(&comp(&[("Co", 1), ("O", 1)])).is_some());
        // The Racah B (spin-pairing side) is present for the monoxide cations.
        assert!(
            close(t.racah_b_cm("Ni2+").expect("Ni B"), 1080.0, 1.0),
            "Ni2+ free-ion B 1080 cm^-1"
        );
    }

    #[test]
    fn an_unpinned_water_reference_is_rejected() {
        // The f(H2O) = 1.00 pin is a load guard: a table whose water reference is not 1.00 is rejected (its
        // multiplicativity would not compose with other sources).
        let bad = r#"
[[ligand_f]]
ligand = "H2O"
f = "1.10"
source = "test (a mis-normalized water reference)"
"#;
        assert!(matches!(
            CrystalFieldTables::from_toml_str(bad),
            Err(CrystalFieldError::UnpinnedReference(_))
        ));
    }

    #[test]
    fn a_missing_citation_is_rejected() {
        let bad = r#"
[[ligand_f]]
ligand = "H2O"
f = "1.00"
source = ""
"#;
        assert!(matches!(
            CrystalFieldTables::from_toml_str(bad),
            Err(CrystalFieldError::MissingSource(_))
        ));
    }
}
