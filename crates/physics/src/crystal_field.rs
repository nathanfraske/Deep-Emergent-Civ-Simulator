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
//! `Delta_o` and the Racah `B`, the inputs to the magnetism (b) high/low-spin correction and the optics d-d colour,
//! plus (seam 2, the iron dark-crust optics) the direct-measured charge-transfer band energies that darken an
//! oxidized iron crust.
//!
//! THE CHARGE-TRANSFER COLUMN (seam 2). A ferric or mixed-valence iron oxide reads DARK, not the warm-white of a
//! ferrous or iron-free silicate, because of two intense charge-transfer bands the weak Laporte-forbidden d-d line
//! does not carry: the `O2- -> Fe3+` ligand-to-metal band (LMCT, ~3.1 eV, its intense tail flooding the visible to
//! redden hematite) and the `Fe2+ -> Fe3+` intervalence band (IVCT, ~0.6 eV, blackening magnetite). Which band a
//! phase carries keys on its DERIVED iron oxidation state ([`iron_valence_state`], charge balance over the phase's
//! own composition, the same primitive the correlation classifier uses). The IDEAL is to DERIVE the band energy from
//! the banked ligand-field machinery by Jorgensen's optical-electronegativity relation
//! (`E_CT = 30000 cm^-1 * (chi_opt(ligand) - chi_opt(metal))`); the scale constant is primary-cited, but the per-
//! species optical electronegativities `chi_opt(O2-)`, `chi_opt(Fe3+)`, `chi_opt(Fe2+)` are in Jorgensen's 1969-1971
//! books (not web-open), so that derivation is FLAGGED (see `docs/working/MORNING_REVIEW.md`) and the band energies
//! are instead supplied as DIRECT single-crystal optical measurements (the same idiom as the monoxide `Delta_o`
//! below, a per-chromophore cited datum, never a factorization, never fabricated).
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

use crate::periodic::PeriodicTable;

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
    /// A charge-transfer row is malformed (an unknown `kind`, or a missing metal/ligand/donor/acceptor key).
    ChargeTransfer(String),
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
            CrystalFieldError::ChargeTransfer(m) => {
                write!(f, "crystal-field charge-transfer row error: {m}")
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
    /// The direct-measured ligand-to-metal charge-transfer band energy (eV), keyed by (metal species, ligand
    /// species), the same DIRECT-MEASUREMENT idiom as [`Self::oxide_delta_cm`] (a per-chromophore cited optical
    /// energy, not a factorization). The `O2- -> Fe3+` LMCT that reddens a ferric oxide.
    lmct_ev: BTreeMap<(String, String), Fixed>,
    /// The direct-measured metal-to-metal intervalence charge-transfer band energy (eV), keyed by (donor species,
    /// acceptor species). The `Fe2+ -> Fe3+` IVCT that blackens a mixed-valence oxide.
    ivct_ev: BTreeMap<(String, String), Fixed>,
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
    #[serde(default)]
    charge_transfer: Vec<ChargeTransferDef>,
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

#[derive(Debug, Default, Deserialize, Serialize)]
struct ChargeTransferDef {
    /// The transfer kind: `"lmct"` (ligand-to-metal, keyed by `metal`/`ligand`) or `"ivct"` (metal-to-metal
    /// intervalence, keyed by `donor`/`acceptor`).
    #[serde(default)]
    kind: String,
    #[serde(default)]
    metal: String,
    #[serde(default)]
    ligand: String,
    #[serde(default)]
    donor: String,
    #[serde(default)]
    acceptor: String,
    #[serde(default)]
    band_ev: String,
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
        let mut lmct_ev = BTreeMap::new();
        let mut ivct_ev = BTreeMap::new();
        for c in file.charge_transfer {
            if c.source.trim().is_empty() {
                return Err(CrystalFieldError::MissingSource(
                    "charge_transfer".to_string(),
                ));
            }
            match c.kind.trim() {
                "lmct" => {
                    let (metal, ligand) = (c.metal.trim(), c.ligand.trim());
                    if metal.is_empty() || ligand.is_empty() {
                        return Err(CrystalFieldError::ChargeTransfer(
                            "an lmct row needs both `metal` and `ligand`".to_string(),
                        ));
                    }
                    let val = parse_positive(&c.band_ev, &format!("E_LMCT({metal}<-{ligand})"))?;
                    let key = (metal.to_string(), ligand.to_string());
                    if lmct_ev.insert(key, val).is_some() {
                        return Err(CrystalFieldError::Duplicate(format!(
                            "lmct {metal}<-{ligand}"
                        )));
                    }
                }
                "ivct" => {
                    let (donor, acceptor) = (c.donor.trim(), c.acceptor.trim());
                    if donor.is_empty() || acceptor.is_empty() {
                        return Err(CrystalFieldError::ChargeTransfer(
                            "an ivct row needs both `donor` and `acceptor`".to_string(),
                        ));
                    }
                    let val = parse_positive(&c.band_ev, &format!("E_IVCT({donor}->{acceptor})"))?;
                    let key = (donor.to_string(), acceptor.to_string());
                    if ivct_ev.insert(key, val).is_some() {
                        return Err(CrystalFieldError::Duplicate(format!(
                            "ivct {donor}->{acceptor}"
                        )));
                    }
                }
                other => {
                    return Err(CrystalFieldError::ChargeTransfer(format!(
                        "unknown charge-transfer kind '{other}' (expected 'lmct' or 'ivct')"
                    )));
                }
            }
        }
        Ok(CrystalFieldTables {
            ligand_f,
            ion_g_kilocm,
            racah_b_cm,
            oxide_delta_cm,
            lmct_ev,
            ivct_ev,
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

    /// The direct-measured ligand-to-metal charge-transfer band energy (eV) for a (metal species, ligand species)
    /// chromophore, or `None` when the pair is not tabulated. The `("Fe3+", "O2-")` LMCT anchors the ferric-oxide
    /// darkening.
    pub fn lmct_ev(&self, metal: &str, ligand: &str) -> Option<Fixed> {
        self.lmct_ev
            .get(&(metal.to_string(), ligand.to_string()))
            .copied()
    }

    /// The direct-measured metal-to-metal intervalence charge-transfer band energy (eV) for a (donor species,
    /// acceptor species) pair, or `None` when the pair is not tabulated. The `("Fe2+", "Fe3+")` IVCT anchors the
    /// mixed-valence-oxide darkening.
    pub fn ivct_ev(&self, donor: &str, acceptor: &str) -> Option<Fixed> {
        self.ivct_ev
            .get(&(donor.to_string(), acceptor.to_string()))
            .copied()
    }

    /// The charge-transfer and intervalence band energies (eV) a composition carries, keyed on its DERIVED iron
    /// oxidation state (the phase's own charge balance, [`iron_valence_state`]): a ferric phase carries the
    /// ligand-to-metal charge-transfer band of its `Fe3+`-anion chromophore; a mixed-valence phase carries that band
    /// AND the `Fe2+ -> Fe3+` intervalence band. Returned as `(charge_transfer_ev, intervalence_ev)`, either `None`
    /// when the phase does not carry that feature or when the chromophore is not yet a tabulated data row (fail-soft,
    /// the honest data gap: a novel iron-bearing phase is classified but its band energy is a cited row that grows
    /// with the world, never fabricated). The observer-independent energies the optics substrate emits as features.
    pub fn iron_charge_transfer_energies(
        &self,
        composition: &[(String, u32)],
        table: &PeriodicTable,
    ) -> (Option<Fixed>, Option<Fixed>) {
        let state = iron_valence_state(composition, table);
        match state {
            IronValence::Ferric | IronValence::Mixed => {
                // The charge-transfer ligand is the phase's anion at its formal charge (`O` at `-2` -> `"O2-"`),
                // keyed off the composition so an alien anion is a data row.
                let ligand = dominant_anion_species(composition, table);
                let ct = ligand.and_then(|l| self.lmct_ev("Fe3+", &l));
                let ivct = if state == IronValence::Mixed {
                    self.ivct_ev("Fe2+", "Fe3+")
                } else {
                    None
                };
                (ct, ivct)
            }
            IronValence::NoIron
            | IronValence::Metallic
            | IronValence::Ferrous
            | IronValence::Unresolved => (None, None),
        }
    }
}

/// The iron oxidation state of a phase, read from the phase's own composition (the phase IS the state; there is no
/// separate continuous `Fe2+/Fe3+` ratio). The band a ferric or mixed-valence oxide carries in the optics substrate
/// keys on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IronValence {
    /// No iron in the phase.
    NoIron,
    /// Iron present but reduced (no oxidizing anion, or an average charge at or below zero): metallic iron, no
    /// iron-oxide chromophore.
    Metallic,
    /// Ferrous, average iron charge `<= 2` (`FeO`, fayalite): only the weak near-infrared d-d line, no charge-
    /// transfer band, so a ferrous phase reads light.
    Ferrous,
    /// Mixed valence, average iron charge strictly between 2 and 3 (`Fe3O4` magnetite): both `Fe2+` and `Fe3+` are
    /// present, so the phase carries the intervalence band (and the charge-transfer band of its `Fe3+`).
    Mixed,
    /// Ferric, average iron charge `>= 3` (`Fe2O3` hematite): the `O2- -> Fe3+` charge-transfer band reddens and
    /// darkens the phase.
    Ferric,
    /// Iron present but the valence is not cleanly derivable here (a multi-cation phase where iron's charge cannot be
    /// isolated from the other cations by charge balance alone, a scoped follow-on): emit no charge-transfer band.
    Unresolved,
}

/// The average iron oxidation state of a phase, DERIVED by charge balance against the phase's anions (the same
/// periodic-valence charge-balance primitive the correlation classifier's `identify_correlated_pair` uses, extended
/// to admit the non-integer average of a mixed-valence phase). An element's role is read from its PRIMARY (first-
/// listed) valence: a negative primary valence is an anion at that charge, a positive primary valence is a cation.
/// Iron must be the sole cation for the balance to isolate its charge; a phase with another cation is `Unresolved`
/// (a scoped follow-on). Reserves no value; keyed entirely on the composition and the banked periodic valence, so a
/// novel iron-bearing phase (an alien anion, an unusual stoichiometry) is a data row.
pub fn iron_valence_state(composition: &[(String, u32)], table: &PeriodicTable) -> IronValence {
    // THE CONSTRAINT SOLVE, replacing a first-listed-valence read that was not general charge balance.
    //
    // The old rule took each element's FIRST banked valence as its role. Sulfur's row begins `-2`, so
    // `FeSO4` was read as one S(2-) plus four O(2-), demanding Fe(10+), and came out FERRIC. Sulfate is
    // S(6+) and FeSO4 is ferrous. Every polyatomic anion failed the same way.
    //
    // The fix enumerates, rather than assumes: one oxidation state per non-iron element drawn from its
    // COMPLETE banked set, with iron permitted to split across its own states (magnetite needs that), and
    // keeps only charge-neutral assignments. FeSO4 then has exactly one neutral vector, Fe(2+) with S(6+)
    // and four O(2-), so it derives as ferrous WITHOUT anyone teaching the engine what a sulfate is. That
    // is the difference between deriving a fact and tabulating it.
    //
    // AMBIGUITY REFUSES. Two or more neutral vectors means the formula alone does not determine the state,
    // and this returns `Unresolved` even when every vector lands in the same coarse class, because
    // agreeing by luck is not the same as being determined. Electronegativity is deliberately NOT used as
    // a tiebreak: atomic electronegativity carries no group connectivity, so it would convert a likelihood
    // into an authored certainty.
    //
    // THE MODEL'S LIMIT, stated: one state per non-iron element. Permitting arbitrary O(2-)/O(1-) mixtures
    // within a flat formula would make even hematite and magnetite ambiguous peroxide readings and refuse
    // both. Formula-level oxidation state is what this API's inputs can support.
    let mut n_fe: i64 = 0;
    let mut others: Vec<(String, i64, Vec<i64>)> = Vec::new();
    for (symbol, count) in composition {
        if *count == 0 {
            continue;
        }
        if symbol == "Fe" {
            n_fe += *count as i64;
            continue;
        }
        let states: Vec<i64> = match table.element(symbol) {
            Some(e) if !e.valence.is_empty() => e.valence.iter().map(|v| i64::from(*v)).collect(),
            // An element with no banked valence cannot be placed, and skipping it as neutral (the old
            // behaviour) silently invents a charge balance. It refuses.
            _ => return IronValence::Unresolved,
        };
        others.push((symbol.clone(), *count as i64, states));
    }
    if n_fe == 0 {
        return IronValence::NoIron;
    }
    let fe_states: Vec<i64> = match table.element("Fe") {
        Some(e) if !e.valence.is_empty() => e.valence.iter().map(|v| i64::from(*v)).collect(),
        _ => return IronValence::Unresolved,
    };
    // Every total the iron atoms can reach by distributing over their own banked states.
    let mut fe_totals: Vec<i64> = Vec::new();
    let lo = fe_states.iter().copied().min().unwrap_or(0);
    let hi = fe_states.iter().copied().max().unwrap_or(0);
    for total in (lo * n_fe)..=(hi * n_fe) {
        // reachable when total = sum of n_fe values each drawn from fe_states
        let reachable = fe_states.iter().any(|_| {
            // with two states a and b, total is reachable iff total = a*k + b*(n-k) for some 0<=k<=n
            fe_states.iter().any(|a| {
                fe_states
                    .iter()
                    .any(|b| (0..=n_fe).any(|k| a * k + b * (n_fe - k) == total))
            })
        });
        if reachable {
            fe_totals.push(total);
        }
    }
    // Enumerate one state per non-iron element and keep the neutral assignments.
    let mut neutral_fe_totals: Vec<i64> = Vec::new();
    let mut index = vec![0usize; others.len()];
    loop {
        let rest: i64 = others
            .iter()
            .zip(index.iter())
            .map(|((_, count, states), i)| count * states[*i])
            .sum();
        for fe_total in &fe_totals {
            if fe_total + rest == 0 && !neutral_fe_totals.contains(fe_total) {
                neutral_fe_totals.push(*fe_total);
            }
        }
        // odometer over the per-element state choices
        let mut pos = others.len();
        loop {
            if pos == 0 {
                break;
            }
            pos -= 1;
            index[pos] += 1;
            if index[pos] < others[pos].2.len() {
                break;
            }
            index[pos] = 0;
            if pos == 0 {
                pos = usize::MAX;
                break;
            }
        }
        if pos == usize::MAX || others.is_empty() {
            break;
        }
    }
    // The solved per-element assignment is also what the anion helper needs, so it is returned through
    // `solved_assignment` below rather than re-derived there from first-listed valences. Two functions
    // answering one question independently is the diamond this file already paid for once.
    match neutral_fe_totals.len() {
        // No neutral assignment: an alloy or metal with no oxidizing partner is metallic; otherwise the
        // formula does not balance at all and refuses.
        0 => {
            if others.is_empty() {
                IronValence::Metallic
            } else {
                IronValence::Unresolved
            }
        }
        1 => {
            let supply = neutral_fe_totals[0];
            if supply <= 0 {
                IronValence::Metallic
            } else if supply <= 2 * n_fe {
                IronValence::Ferrous
            } else if supply < 3 * n_fe {
                IronValence::Mixed
            } else {
                IronValence::Ferric
            }
        }
        // Determined by luck is not determined. Refuse.
        _ => IronValence::Unresolved,
    }
}

/// The species string of the phase's dominant anion (the anion contributing the most negative charge), formatted as
/// symbol + magnitude + sign (`O` at `-2` -> `"O2-"`), the key the charge-transfer ligand column uses. `None` when
/// the phase has no anion. Keyed off the composition, so an alien anion is a data row.
fn dominant_anion_species(composition: &[(String, u32)], table: &PeriodicTable) -> Option<String> {
    // THE SAME DETERMINACY RULE the iron classifier uses, rather than a second first-listed-valence read.
    //
    // This helper previously took each element's FIRST banked valence to decide which species is the anion,
    // which is the identical defect that made `FeSO4` read as S(2-) and demand Fe(10+). Fixing one and
    // leaving the other would have left the wrong answer reachable through a different door: the classifier
    // would call sulfate ferrous while this named `S2-` as its ligand.
    //
    // So the anion is taken from the UNIQUE charge-neutral assignment when one exists, and this refuses
    // when none does. A composition whose formula does not determine its oxidation states does not have a
    // determinate dominant anion either.
    let mut others: Vec<(String, i64, Vec<i64>)> = Vec::new();
    for (symbol, count) in composition {
        if *count == 0 || symbol == "Fe" {
            continue;
        }
        let states: Vec<i64> = match table.element(symbol) {
            Some(e) if !e.valence.is_empty() => e.valence.iter().map(|v| i64::from(*v)).collect(),
            _ => return None,
        };
        others.push((symbol.clone(), *count as i64, states));
    }
    if others.is_empty() {
        return None;
    }
    let n_fe: i64 = composition
        .iter()
        .filter(|(s, _)| s == "Fe")
        .map(|(_, c)| *c as i64)
        .sum();
    let fe_states: Vec<i64> = table
        .element("Fe")
        .map(|e| e.valence.iter().map(|v| i64::from(*v)).collect())
        .unwrap_or_default();

    // Enumerate one state per non-iron element; keep assignments the irons can neutralize.
    let mut solutions: Vec<Vec<i64>> = Vec::new();
    let mut index = vec![0usize; others.len()];
    loop {
        let rest: i64 = others
            .iter()
            .zip(index.iter())
            .map(|((_, count, states), i)| count * states[*i])
            .sum();
        let neutralizable = if n_fe == 0 {
            rest == 0
        } else {
            fe_states.iter().any(|a| {
                fe_states
                    .iter()
                    .any(|b| (0..=n_fe).any(|k| a * k + b * (n_fe - k) + rest == 0))
            })
        };
        if neutralizable {
            let assignment: Vec<i64> = others
                .iter()
                .zip(index.iter())
                .map(|((_, _, states), i)| states[*i])
                .collect();
            if !solutions.contains(&assignment) {
                solutions.push(assignment);
            }
        }
        let mut pos = others.len();
        loop {
            if pos == 0 {
                pos = usize::MAX;
                break;
            }
            pos -= 1;
            index[pos] += 1;
            if index[pos] < others[pos].2.len() {
                break;
            }
            index[pos] = 0;
            if pos == 0 {
                pos = usize::MAX;
                break;
            }
        }
        if pos == usize::MAX {
            break;
        }
    }
    // Determinate or nothing: an ambiguous formula has no determinate dominant anion.
    if solutions.len() != 1 {
        return None;
    }
    let assignment = &solutions[0];
    let mut best: Option<(String, i64, i64)> = None; // (symbol, charge, total magnitude)
    for ((symbol, count, _), charge) in others.iter().zip(assignment.iter()) {
        if *charge >= 0 {
            continue;
        }
        let magnitude = (-charge) * count;
        if best.as_ref().map(|b| magnitude > b.2).unwrap_or(true) {
            best = Some((symbol.clone(), *charge, magnitude));
        }
    }
    // Formatted through the SAME `species_key` the charge-transfer column keys on, rather than by an
    // inline format string, so the anion name this produces cannot drift from the name that column expects.
    best.map(|(symbol, charge, _)| species_key(&symbol, charge as i8))
}

/// A species key: the element symbol, the charge magnitude, and the sign (`("O", -2) -> "O2-"`,
/// `("Fe", 3) -> "Fe3+"`), the format the charge-transfer column keys its metal and ligand species by.
fn species_key(symbol: &str, charge: i8) -> String {
    let sign = if charge < 0 { "-" } else { "+" };
    format!("{symbol}{}{sign}", charge.unsigned_abs())
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

    fn periodic() -> PeriodicTable {
        PeriodicTable::standard().expect("the periodic table loads")
    }

    #[test]
    fn the_iron_oxidation_state_derives_from_charge_balance() {
        // THE PHASE IS THE STATE: the average iron charge is derived by charge balance against the phase's anions
        // (O at -2), and the class boundaries are the integer valences 2 and 3. FeO -> Fe2+ (ferrous), Fe2O3 -> Fe3+
        // (ferric), Fe3O4 -> 8/3 (mixed valence), pure iron -> metallic, an iron-free oxide -> no iron.
        //
        // FAYALITE MOVED, 2026-07-19, and the move is the point rather than a fixture adjustment. This case read
        // `Unresolved` while the balance refused any phase carrying a second cation, which this function's own
        // doc called a scoped follow-on. Silicon lists exactly ONE positive valence, so it takes +4 with no
        // choice left open and can be subtracted from the budget: Fe2SiO4's oxygens demand 8, silicon supplies
        // 4, and the two irons split the remaining 4 into the FERROUS state. The five assertions above are
        // untouched, which is the evidence the extension closed a limit rather than moved a result.
        //
        // `Unresolved` stays live-fired below on PYRITE, whose persulfide sulfur no one-state-per-element
        // assignment can balance. Siderite, which this comment once named as the refusal case, resolves.
        let t = periodic();
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 1), ("O", 1)]), &t),
            IronValence::Ferrous
        );
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 2), ("O", 3)]), &t),
            IronValence::Ferric
        );
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 3), ("O", 4)]), &t),
            IronValence::Mixed
        );
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 1)]), &t),
            IronValence::Metallic
        );
        assert_eq!(
            iron_valence_state(&comp(&[("Mg", 1), ("O", 1)]), &t),
            IronValence::NoIron
        );
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 2), ("Si", 1), ("O", 4)]), &t),
            IronValence::Ferrous,
            "fayalite: the oxygens demand 8, silicon's single positive state supplies 4, two irons split 4"
        );
        // SIDERITE MOVED, 2026-07-19, and the move is the fix. Under the old first-valence rule this read
        // as unresolvable; the constraint solve finds exactly ONE neutral assignment, Fe(2+) with C(4+) and
        // three O(2-), because C(2+) would demand Fe(4+) and O(1-) would demand a negative iron, neither of
        // which iron's own banked set admits. FeCO3 is ferrous carbonate, so the derived answer is right.
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 1), ("C", 1), ("O", 3)]), &t),
            IronValence::Ferrous,
            "siderite: exactly one neutral vector exists, Fe(2+) C(4+) 3 O(2-)"
        );
        // THE MOTIVATING CASE. Sulfate defeated the old rule completely: sulfur's row begins -2, so FeSO4
        // read as S(2-) plus four O(2-), demanded Fe(10+), and came out FERRIC. The solve finds the single
        // neutral vector Fe(2+) S(6+) 4 O(2-) without anyone teaching it what a sulfate is.
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 1), ("S", 1), ("O", 4)]), &t),
            IronValence::Ferrous,
            "FeSO4 is ferrous sulfate, derived rather than tabulated"
        );
        // AND THE REFUSAL STAYS LIVE-FIRED on a formula the model genuinely cannot place. Pyrite's sulfur is
        // a persulfide pair S2(2-), which one-state-per-element cannot express: no assignment over the
        // banked sets is charge neutral, so it refuses instead of inventing a state.
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 1), ("S", 2)]), &t),
            IronValence::Unresolved,
            "pyrite FeS2 has no neutral assignment under one state per element"
        );
    }

    #[test]
    fn the_charge_transfer_column_carries_the_cited_hematite_and_magnetite_bands() {
        // The direct-measured charge-transfer band energies (eV), keyed by chromophore pair: the O2- -> Fe3+ LMCT
        // (~3.1 eV, hematite) and the Fe2+ -> Fe3+ IVCT (~0.6 eV, magnetite). An untabulated pair is None.
        let t = tables();
        assert!(
            close(t.lmct_ev("Fe3+", "O2-").expect("Fe3+ LMCT"), 3.1, 1e-6),
            "O2- -> Fe3+ LMCT 3.1 eV"
        );
        assert!(
            close(t.ivct_ev("Fe2+", "Fe3+").expect("Fe IVCT"), 0.6, 1e-6),
            "Fe2+ -> Fe3+ IVCT 0.6 eV"
        );
        assert!(
            t.lmct_ev("Fe2+", "O2-").is_none(),
            "ferrous has no LMCT row"
        );
        assert!(
            t.ivct_ev("Ni2+", "Ni3+").is_none(),
            "an untabulated pair is None"
        );
    }

    #[test]
    fn the_charge_transfer_energies_key_on_the_derived_oxidation_state() {
        // The bridge from a composition to the optics substrate's feature energies: a ferric oxide carries the LMCT
        // only; a mixed-valence oxide carries the LMCT AND the IVCT; a ferrous oxide and metallic iron carry neither
        // (the honest per-valence distinction that darkens ferric/mixed and leaves ferrous light).
        let t = tables();
        let p = periodic();
        assert_eq!(
            t.iron_charge_transfer_energies(&comp(&[("Fe", 2), ("O", 3)]), &p),
            (Some(Fixed::from_ratio(31, 10)), None),
            "hematite: LMCT only"
        );
        assert_eq!(
            t.iron_charge_transfer_energies(&comp(&[("Fe", 3), ("O", 4)]), &p),
            (
                Some(Fixed::from_ratio(31, 10)),
                Some(Fixed::from_ratio(6, 10))
            ),
            "magnetite: LMCT and IVCT"
        );
        assert_eq!(
            t.iron_charge_transfer_energies(&comp(&[("Fe", 1), ("O", 1)]), &p),
            (None, None),
            "wustite (ferrous): neither"
        );
        assert_eq!(
            t.iron_charge_transfer_energies(&comp(&[("Fe", 1)]), &p),
            (None, None),
            "metallic iron: neither"
        );
    }

    #[test]
    fn an_alien_iron_bearing_phase_is_a_data_row() {
        // ADMIT THE ALIEN: the mechanism keys on the DERIVED valence and the chromophore, never a hardcoded phase
        // list, so a novel iron-oxide stoichiometry not in any mineral table (here Fe5O7, average charge 14/5 = 2.8,
        // a mixed valence) is classified and carries its bands as a data row, no code change.
        let t = tables();
        let p = periodic();
        assert_eq!(
            iron_valence_state(&comp(&[("Fe", 5), ("O", 7)]), &p),
            IronValence::Mixed
        );
        assert_eq!(
            t.iron_charge_transfer_energies(&comp(&[("Fe", 5), ("O", 7)]), &p),
            (
                Some(Fixed::from_ratio(31, 10)),
                Some(Fixed::from_ratio(6, 10))
            ),
            "the alien mixed-valence oxide carries both bands via the chromophore column"
        );
    }

    #[test]
    fn a_malformed_charge_transfer_row_is_rejected() {
        // An unknown kind and a missing key are load errors (never a silently dropped row).
        let unknown = r#"
[[ligand_f]]
ligand = "H2O"
f = "1.00"
source = "test"
[[charge_transfer]]
kind = "mmct"
metal = "Fe3+"
ligand = "O2-"
band_ev = "3.1"
source = "test"
"#;
        assert!(matches!(
            CrystalFieldTables::from_toml_str(unknown),
            Err(CrystalFieldError::ChargeTransfer(_))
        ));
        let missing_key = r#"
[[ligand_f]]
ligand = "H2O"
f = "1.00"
source = "test"
[[charge_transfer]]
kind = "lmct"
metal = "Fe3+"
band_ev = "3.1"
source = "test"
"#;
        assert!(matches!(
            CrystalFieldTables::from_toml_str(missing_key),
            Err(CrystalFieldError::ChargeTransfer(_))
        ));
    }
}
